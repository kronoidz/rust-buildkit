use anyhow::{anyhow, Result};
use hyper::{rt::ReadBufCursor, Uri};
use std::io::Write;
use std::{io::{ErrorKind, Read, Stdin, Stdout}, os::fd::AsRawFd, pin::Pin, task::{Context, Poll}};
use tokio::io::{unix::AsyncFd, Interest, ReadBuf};

pub struct StdioSocket {
    stdin: AsyncFd<Stdin>,
    stdout: AsyncFd<Stdout>,
}

pub async fn stdio_connector(_: Uri) -> Result<StdioSocket> {
    StdioSocket::try_new()
}

fn set_non_blocking_flag(stream: &impl AsRawFd) -> Result<()> {
    let flags = unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_GETFL, 0) };

    if flags < 0 {
        return Err(anyhow!(std::io::Error::last_os_error()));
    }

    if unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } != 0 {
        return Err(anyhow!(std::io::Error::last_os_error()));
    }

    Ok(())
}

impl StdioSocket {
    fn try_new() -> Result<StdioSocket> {
        let stdin = std::io::stdin();
        set_non_blocking_flag(&stdin)?;

        let stdout = std::io::stdout();
        set_non_blocking_flag(&stdout)?;

        Ok(StdioSocket {
            stdin: AsyncFd::with_interest(stdin, Interest::READABLE)?,
            stdout: AsyncFd::with_interest(stdout, Interest::WRITABLE)?,
        })
    }
}

impl hyper::rt::Read for StdioSocket {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut read_buf = ReadBuf::uninit(unsafe { buf.as_mut() });

        // try reading and retry on ErrorKind::Interrupted
        loop {
            match self.stdin.get_mut().read(read_buf.initialize_unfilled()) {
                Ok(n) => {
                    read_buf.advance(n);
                    unsafe { buf.advance(n) };
                    return Poll::Ready(Ok(()));
                },
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // ErrorKind::WouldBlock reached: poll AsyncFd
                    loop {
                        match self.stdin.poll_read_ready(cx) {
                            Poll::Ready(Ok(_)) => break,
                            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                            Poll::Pending => return Poll::Pending,
                        }
                    }
                },
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }
}

impl hyper::rt::Write for StdioSocket {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        // try writing and retry on ErrorKind::Interrupted
        loop {
            match self.stdout.get_mut().write(buf) {
                Ok(n) => return Poll::Ready(Ok(n)),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // ErrorKind::WouldBlock reached: poll AsyncFd
                    loop {
                        match self.stdout.poll_write_ready(cx) {
                            Poll::Ready(Ok(_)) => break,
                            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                            Poll::Pending => return Poll::Pending,
                        }
                    }
                },
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(self.stdout.get_mut().flush())
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(self.stdout.get_mut().flush())
    }
}
