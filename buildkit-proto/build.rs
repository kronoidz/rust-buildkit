const DEFS: &[&str] = &["proto/github.com/moby/buildkit/frontend/gateway/pb/gateway.proto"];
const PATHS: &[&str] = &["proto"];

fn main() {
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .compile_protos(DEFS, PATHS)
        .unwrap_or_else(|e| panic!("{e}"));
}
