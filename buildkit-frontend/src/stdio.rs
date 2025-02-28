use hyper::{rt::ReadBufCursor, Uri};
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, Stdin, Stdout};
use std::{io::Result, task::Poll};

#[pin_project]
pub struct StdioSocket {
    #[pin]
    stdin: Stdin,
    #[pin]
    stdout: Stdout,
}

pub async fn stdio_connector(_: Uri) -> Result<StdioSocket> {
    StdioSocket::try_new()
}

impl StdioSocket {
    pub fn try_new() -> Result<Self> {
        Ok(StdioSocket {
            stdin: tokio::io::stdin(),
            stdout: tokio::io::stdout(),
        })
    }
}

impl hyper::rt::Read for StdioSocket {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> std::task::Poll<Result<()>> {
        let mut tbuf = tokio::io::ReadBuf::uninit(unsafe { buf.as_mut() });
        let n = match self.project().stdin.poll_read(cx, &mut tbuf) {
            Poll::Ready(Ok(())) => tbuf.filled().len(),
            other => return other,
        };
        unsafe { buf.advance(n); }
        Poll::Ready(Ok(()))
    }
}

impl hyper::rt::Write for StdioSocket {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        self.project().stdout.poll_write(cx, buf)
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<std::result::Result<(), std::io::Error>> {
        self.project().stdout.poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        self.project().stdout.poll_shutdown(cx)
    }
}
