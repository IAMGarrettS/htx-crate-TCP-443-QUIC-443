use anyhow::Result;
use htx::mux::{dial, spawn_driver, client_send_echo_once};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Load client private key and server public key from disk
    let client_static_priv = std::fs::read("client.key")?;
    let server_static_pub = std::fs::read("server.pub")?;

    println!("[boot] connecting to 0.0.0.0:8443");

    // Complete TCP connect, Noise_XK handshake, and Yamux upgrade
    let yamux_conn =
        dial("0.0.0.0:8443", &client_static_priv, &server_static_pub).await?;

    println!("[conn] handshake + yamux ready");

    // Spawn the driver and get a Control handle for opening streams
    let (driver_handle, control) = spawn_driver(yamux_conn);

    // --- Example: open several independent substreams in sequence ---
    for i in 0..3 {
        let ctl = control.clone();
        tokio::spawn(async move {
            if let Err(e) = client_send_echo_once(&ctl).await {
                eprintln!("[stream {i}] error: {e}");
            }
        });
        sleep(Duration::from_millis(200)).await;
    }

    // Wait for the driver to finish (e.g. connection closes)
    // In a long‑running client, you might just `driver_handle.await?;`
    driver_handle.await??;

    Ok(())
}
