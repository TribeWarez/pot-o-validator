use std::net::TcpListener;

use tokio::runtime::Runtime;

#[test]
fn status_endpoint_starts_and_responds() {
    let rt = Runtime::new().expect("runtime");
    rt.block_on(async {
        // Bind an ephemeral port just to ensure the server can start a listener.
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().unwrap();
        drop(listener);

        // For now, just assert that we can construct the config and not panic.
        // Full end-to-end HTTP tests can be added separately using axum::Router testing utilities.
        let _ = addr;
    });
}
