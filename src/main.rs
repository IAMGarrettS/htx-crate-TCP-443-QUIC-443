use anyhow::Result;
use htx::mux::{accept, dial, spawn_driver, client_interactive};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_CONN_ID: AtomicUsize = AtomicUsize::new(1);


#[tokio::main]
async fn main() -> Result<()> {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "server".into());

    match mode.as_str() {
        "server" => {
            let server_priv = fs::read("server.key")?;
            println!("[server] listening on 0.0.0.0:8443");

            loop {
                let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);
                let conn = accept("0.0.0.0:8443", &server_priv).await?;
                println!("[server][session #{conn_id}] accepted");
                let (driver, _ctrl) = spawn_driver(conn);
                tokio::spawn(async move {
                    if let Err(e) = driver.await {
                        eprintln!("[server][session #{conn_id}] connection error: {e:?}");
                    }
                });
            }
        }

        "client" => {
            let client_priv = fs::read("client.key")?;
            let server_pub  = fs::read("server.pub")?;
            let conn = dial("0.0.0.0:8443", &client_priv, &server_pub).await?;
            let (driver, ctrl) = spawn_driver(conn);
            client_interactive(&ctrl).await?;
            driver.abort();
        }
        _ => eprintln!("Usage: cargo run -- [server|client]"),
    }

    Ok(())
}
