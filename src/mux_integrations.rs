// tests/mux_integration.rs
use std::sync::Arc;
use tokio::io::duplex;
use htx::mux::Mux;
use htx::frame::Frame;
use tokio::task;

#[tokio::test]
async fn mux_parallel_streams() {
    // Create a bidirectional in-memory duplex stream (simulates a network connection)
    let (a, b) = duplex(64 * 1024);

    // Spawn Mux instances on both ends
    let mux_a = Mux::new(a);
    let mux_b = Mux::new(b);

    // Number of parallel logical streams
    let num_streams = 5;

    // Spawn tasks that open streams from mux_a -> mux_b
    let mut handles = Vec::new();
    for i in 0..num_streams {
        let mux_a = mux_a.clone();
        let mux_b = mux_b.clone();

        handles.push(task::spawn(async move {
            // Open a stream from mux_a
            let mut stream_a = mux_a.open_stream();

            // Wait for mux_b to accept the new incoming stream
            let mut stream_b = mux_b.accept_stream().await.expect("stream accepted");

            // Send a message from A -> B
            let msg = format!("hello stream {}", i).into_bytes();
            stream_a.send(msg.clone()).await.expect("send failed");

            // Receive the message on B
            let received = stream_b.recv().await.expect("recv failed");
            assert_eq!(received, msg);

            // Echo back B -> A
            stream_b.send(b"ack".to_vec()).await.expect("send ack failed");

            // Receive ack on A
            let ack = stream_a.recv().await.expect("recv ack failed");
            assert_eq!(ack, b"ack");
        }));
    }

    // Wait for all streams to finish
    for h in handles {
        h.await.unwrap();
    }
}
