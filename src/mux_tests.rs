// in mux.rs or in a new file tests/mux_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn mux_sends_and_receives() -> Result<(), Box<dyn std::error::Error>> {
        // Create an in-memory duplex stream (two connected endpoints)
        let (stream_a, stream_b) = duplex(1024);

        // Spawn Mux on each side
        let mux_a = Mux::new(stream_a);
        let mux_b = Mux::new(stream_b);

        // Spawn tasks to run them
        tokio::spawn(mux_a.run());
        tokio::spawn(mux_b.run());

        // Open a logical channel on side A
        let mut chan_a = mux_a.open_stream().await?;

        // Get a channel opened from A's side on B's mux
        // NOTE: In a real impl, you'd need accept_stream() or similar on mux_b.
        // For test simplicity we assume chan_id=0 on both sides matches.
        let mut chan_b = mux_b.channels.lock().unwrap().get(&0).unwrap().clone();

        // Send message from A → B
        chan_a.send(b"hello world").await?;

        // Receive message on B
        let msg = chan_b.recv().await?;
        assert_eq!(msg, b"hello world");

        Ok(())
    }
}
