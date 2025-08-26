use anyhow::Result;
use htx::mux::handshake_xk_responder;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    signal,
    task,
};
use std::fs;

/// Per‑connection handler: runs handshake, spawns Yamux driver, echoes substream data
async fn handle_connection(
    socket: TcpStream,
    server_static_priv: Vec<u8>,
    client_static_pub: Vec<u8>,
) -> Result<()> {
    // Complete Noise_XK handshake as responder
    let mux = handshake_xk_responder(socket, &server_static_priv, &client_static_pub).await?;
    let driver = mux.driver;
    tokio::spawn(driver);

    // Accept inbound Yamux substreams forever
    loop {
        let mut inbound = mux.control.accept_stream().await?;
        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];
            match inbound.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    println!(
                        "[server] Received: {}",
                        String::from_utf8_lossy(&buf[..n])
                    );
                    if let Err(e) = inbound.write_all(&buf[..n]).await {
                        eprintln!("[server] Write error: {e}");
                    }
                }
                _ => { /* stream closed or error */ }
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load server private key and expected client public key from disk
    let server_static_priv = fs::read("server.key")?;
    let client_static_pub = fs::read("client.pub")?;

    let listener = TcpListener::bind("0.0.0.0:443").await?;
    println!("[boot] Server listening on 0.0.0.0:443");

    // Spawn an accept loop that runs until shutdown signal
    let server_task = {
        let server_static_priv = server_static_priv.clone();
        let client_static_pub = client_static_pub.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, addr)) => {
                        println!("[server] New TCP connection from {addr}");
                        let priv_clone = server_static_priv.clone();
                        let pub_clone = client_static_pub.clone();
                        task::spawn(async move {
                            if let Err(e) =
                                handle_connection(socket, priv_clone, pub_clone).await
                            {
                                eprintln!("[server] Error with {addr}: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("[server] Accept error: {e}");
                        // Optionally: short sleep before retry to avoid spin on persistent error
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        })
    };

    // Wait for Ctrl‑C
    signal::ctrl_c().await?;
    println!("\n[server] Shutdown signal received. Closing listener…");

    // Drop listener and let accept loop end
    drop(listener);

    // Optionally: wait for accept loop to finish
    let _ = server_task.abort(); // or .await if you modify accept loop to exit cleanly

    println!("[server] Shutdown complete.");
    Ok(())
}
