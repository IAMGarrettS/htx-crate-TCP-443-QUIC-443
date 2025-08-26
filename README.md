# HTX (MVP)

Minimal Rust crate scaffolding for the HTX transport (TCP-443 over TLS 1.3). This is a starting point to iterate towards full Betanet-compliant HTX (HTTP/2/3 mimicry, QUIC-443, Noise XK inside, tickets, fallbacks, etc.).

## Features (MVP)
- TLS 1.3 server/client using `rustls` + `tokio-rustls`.
- SNI + WebPKI root verification on client.
- Echo example to validate the encrypted channel.

## Not Implemented Yet
- QUIC/HTTP3 (`quinn`).
- HTTP/2/3 browser-like fingerprints (ALPN tuning, frame timings, priorities, padding, PING cadence).
- Inner Noise XK handshake + stream framing.
- Access-tickets, replay binding, and rate limits.
- QUIC→TCP fallback with cover connections.

## Quickstart

1) Create a local testing certificate (dev only):

```bash
# using mkcert (recommended)
mkcert -install
mkcert localhost 127.0.0.1 ::1
# produces e.g. localhost+2.pem (cert) and localhost+2-key.pem (key)
```

2) Run the TLS echo server:

```bash
RUST_LOG=info cargo run --example echo_server -- \
  --listen 0.0.0.0:8443 \
  --cert localhost+2.pem \
  --key  localhost+2-key.pem
```

3) In another terminal, run the client:

```bash
cargo run --example echo_client -- \
  --server_name localhost \
  --addr 127.0.0.1:8443 \
  --msg "ping"
```

You should see `echoed: ping`.

## Next Steps (suggested commits)
- Add `quinn` transport and implement dual-mode (UDP-443 QUIC + TCP-443).
- Introduce an inner framing layer and wrap a Noise XK handshake using `snow`.
- Implement padding/timing logic and HTTP/2+3 behavioral mimicry.
- Add access-ticket encode/decode + rate limiting.
- Fallback logic with decoy cover connections when UDP is blocked.

## License
Apache-2.0 OR MIT
