// src/mux.rs

use anyhow::Result;
use futures_util::io::{AsyncReadExt as FuturesReadExt, AsyncWriteExt as FuturesWriteExt};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt as TokioReadExt, AsyncWriteExt as TokioWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use yamux::{Config as YamuxConfig, Connection, Mode};

use snow::{params::NoiseParams, Builder};

const PROLOGUE: &[u8] = b"betanet-noise-xk";
const NOISE_PATTERN: &str = "Noise_XK_25519_ChaChaPoly_BLAKE2s";
const MAX_NOISE_MSG: usize = 65535;

// --- Length-delimited framing (u16 big-endian) used only during handshakes ---

async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut len_bytes = [0u8; 2];
    stream.read_exact(&mut len_bytes).await?;
    let len = u16::from_be_bytes(len_bytes) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
    let len = u16::try_from(data.len())?;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

// ----- Noise XK: async responder handshake over framed TCP -----

pub async fn handshake_xk_responder(mut tcp: TcpStream, static_priv: &[u8]) -> Result<TcpStream> {
    let params: NoiseParams = NOISE_PATTERN.parse()?;
    let builder = Builder::new(params)
        .local_private_key(static_priv)?
        .prologue(PROLOGUE)?;
    let mut hs = builder.build_responder()?;

    // <- e
    let msg1 = read_frame(&mut tcp).await?;
    let mut tmp = vec![0u8; MAX_NOISE_MSG];
    hs.read_message(&msg1, &mut tmp)?;

    // -> e, ee, s, es
    let mut out = vec![0u8; MAX_NOISE_MSG];
    let n = hs.write_message(&[], &mut out)?;
    write_frame(&mut tcp, &out[..n]).await?;

    // <- s, se
    let msg3 = read_frame(&mut tcp).await?;
    let mut tmp2 = vec![0u8; MAX_NOISE_MSG];
    hs.read_message(&msg3, &mut tmp2)?;

    // Transport is established (not used directly here — yamux runs over raw TCP).
    let _transport = hs.into_transport_mode()?;

    Ok(tcp)
}

// ----- Noise XK: async initiator handshake over framed TCP -----

pub async fn handshake_xk_initiator(
    mut tcp: TcpStream,
    static_priv: &[u8],
    remote_static_pub: &[u8],
) -> Result<TcpStream> {
    let params: NoiseParams = NOISE_PATTERN.parse()?;
    let builder = Builder::new(params)
        .local_private_key(static_priv)?
        .remote_public_key(remote_static_pub)?
        .prologue(PROLOGUE)?;
    let mut hs = builder.build_initiator()?;

    // -> e
    let mut out = vec![0u8; MAX_NOISE_MSG];
    let n = hs.write_message(&[], &mut out)?;
    write_frame(&mut tcp, &out[..n]).await?;

    // <- e, ee, s, es
    let msg2 = read_frame(&mut tcp).await?;
    let mut tmp = vec![0u8; MAX_NOISE_MSG];
    hs.read_message(&msg2, &mut tmp)?;

    // -> s, se
    let n3 = hs.write_message(&[], &mut out)?;
    write_frame(&mut tcp, &out[..n3]).await?;

    let _transport = hs.into_transport_mode()?;

    Ok(tcp)
}

// Convert a Tokio TcpStream (or your secured transport) into futures-io via compat.
fn to_futures_io(tcp: TcpStream) -> Compat<TcpStream> {
    tcp.compat()
}

// ----- Pure upgrade helpers: given a connected TcpStream, perform Noise + wrap with Yamux -----

pub async fn server_upgrade(
    tcp: TcpStream,
    static_priv: &[u8],
) -> Result<Connection<Compat<TcpStream>>> {
    let secured = handshake_xk_responder(tcp, static_priv).await?;
    let io = to_futures_io(secured);
    let cfg = YamuxConfig::default();
    Ok(Connection::new(io, cfg, Mode::Server))
}

pub async fn client_upgrade(
    tcp: TcpStream,
    static_priv: &[u8],
    remote_static_pub: &[u8],
) -> Result<Connection<Compat<TcpStream>>> {
    let secured = handshake_xk_initiator(tcp, static_priv, remote_static_pub).await?;
    let io = to_futures_io(secured);
    let cfg = YamuxConfig::default();
    Ok(Connection::new(io, cfg, Mode::Client))
}

// Optional convenience for clients (does not bind or listen).
pub async fn dial(
    addr: &str,
    static_priv: &[u8],
    remote_static_pub: &[u8],
) -> Result<Connection<Compat<TcpStream>>> {
    let tcp = TcpStream::connect(addr).await?;
    println!("[boot] Connected to {}", addr);
    client_upgrade(tcp, static_priv, remote_static_pub).await
}

// ----- Driver with a simple "control" channel to open outbound streams -----

#[derive(Clone)]
pub struct Control {
    tx: mpsc::Sender<oneshot::Sender<Result<yamux::Stream, yamux::ConnectionError>>>,
}

impl Control {
    pub async fn open_stream(&self) -> Result<yamux::Stream, yamux::ConnectionError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(reply_tx)
            .await
            .map_err(|_| yamux::ConnectionError::Closed)?;
        reply_rx.await.map_err(|_| yamux::ConnectionError::Closed)?
    }
}

// Async helpers to await yamux poll-style APIs.
async fn next_inbound_stream<T>(
    conn: &mut Connection<T>,
) -> Option<Result<yamux::Stream, yamux::ConnectionError>>
where
    T: futures_io::AsyncRead + futures_io::AsyncWrite + Unpin,
{
    futures_util::future::poll_fn(|cx| conn.poll_next_inbound(cx)).await
}

async fn open_outbound_stream<T>(
    conn: &mut Connection<T>,
) -> Result<yamux::Stream, yamux::ConnectionError>
where
    T: futures_io::AsyncRead + futures_io::AsyncWrite + Unpin,
{
    futures_util::future::poll_fn(|cx| conn.poll_new_outbound(cx)).await
}

// Spawn a task that owns the yamux::Connection, services inbound streams,
// and also handles outbound open requests via Control.
pub fn spawn_driver<T>(
    mut conn: Connection<T>,
) -> (tokio::task::JoinHandle<Result<()>>, Control)
where
    T: Send + 'static + futures_io::AsyncRead + futures_io::AsyncWrite + Unpin,
{
    let (tx, mut rx) =
        mpsc::channel::<oneshot::Sender<Result<yamux::Stream, yamux::ConnectionError>>>(32);
    let control = Control { tx };

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Accept inbound substreams and echo them.
                inbound = next_inbound_stream(&mut conn) => {
                    match inbound {
                        Some(Ok(mut s)) => {
                            tokio::spawn(async move {
                                let mut buf = vec![0u8; 16 * 1024];
                                loop {
                                    match FuturesReadExt::read(&mut s, &mut buf).await {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            if FuturesWriteExt::write_all(&mut s, &buf[..n]).await.is_err() {
                                                break;
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            });
                        }
                        Some(Err(e)) => {
                            eprintln!("[driver] inbound error: {e}");
                        }
                        None => {
                            // Underlying connection closed.
                            break;
                        }
                    }
                }

                // Open outbound streams on request.
                maybe_req = rx.recv() => {
                    match maybe_req {
                        Some(reply_tx) => {
                            let res = timeout(Duration::from_secs(10), open_outbound_stream(&mut conn)).await
                                .map_err(|_| yamux::ConnectionError::Closed)
                                .and_then(|inner| inner);
                            let _ = reply_tx.send(res);
                        }
                        None => {
                            // All senders dropped; continue until connection closes.
                        }
                    }
                }
            }
        }
        Ok(())
    });

    (handle, control)
}

// ----- Client demo: interactive echo over new substreams -----

pub async fn client_interactive(control: &Control) -> Result<()> {
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() { break; }
        let mut s = control.open_stream().await?;
        FuturesWriteExt::write_all(&mut s, line.as_bytes()).await?;
        let mut buf = vec![0u8; 1024];
        let n = FuturesReadExt::read(&mut s, &mut buf).await?;
        println!("[client] echo: {}", String::from_utf8_lossy(&buf[..n]));
    }

    Ok(())
}

use tokio::net::TcpListener;

/// Bind and wait for a single inbound connection, then Noise+Yamux it.
/// This is a convenience so `main.rs` can still call `mux::accept()`.
pub async fn accept(
    bind_addr: &str,
    static_priv: &[u8],
) -> anyhow::Result<yamux::Connection<tokio_util::compat::Compat<TcpStream>>> {
    let listener = TcpListener::bind(bind_addr).await?;
    println!("[server] listening on {}", bind_addr);

    // Accept exactly one connection for this call
    let (socket, addr) = listener.accept().await?;
    println!("[server] accepted from {}", addr);

    // Perform Noise handshake + wrap in Yamux Connection
    server_upgrade(socket, static_priv).await
}
