# htx-crate-TCP-443-QUIC-443
a reusable library that lets any app open or accept encrypted connections that look like normal HTTPS on ports 443

# HTX: Encrypted Multiplexed Transport for Betanet

HTX is a Rust-based encrypted transport layer built for Betanet applications. It uses Noise XK for mutual authentication 
and ChaCha20-Poly1305 encryption, layered over Yamux for multiplexed stream handling. This crate provides a reusable 
dial/accept API for secure, bidirectional communication indistinguishable from HTTPS.

## 🔐 Features

- Noise XK handshake with static key authentication
- ChaCha20-Poly1305 encryption
- Yamux-based stream multiplexing
- Cross-platform compatibility (Linux, macOS, Windows)
- Simple client/server demo with interactive echo

## 📁 Folder Structure

htx/ 
├── src/ 
│ ├── main.rs # Entry point for client/server modes 
│ ├── mux.rs # Core transport logic: handshake, yamux, stream control 
├── client.key # Client's static private key (binary) 
├── client.pub # Client's static public key (binary) 
├── server.key # Server's static private key (binary) 
├── server.pub # Server's static public key (binary) 
├── Cargo.toml # Rust crate manifest



> Note: Keys are expected to be generated and placed manually. See below for instructions.

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+ recommended)
- Git

### 1. Clone the repository

```bash
git clone https://github.com/yourusername/htx.git
cd htx

2. Generate static keypairs
You can use noiseexplorer.com or any Noise-compatible tool to generate 25519 keypairs.

Save them as:

server.key — server's private key

server.pub — server's public key

client.key — client's private key

client.pub — client's public key

All files should be raw 32-byte binary.

3. Run the server
Linux / macOS
cargo run -- server

Windows (PowerShell)
cargo run -- server

The server will bind to 0.0.0.0:443 and accept multiple encrypted client connections.

4. Run the client
cargo run -- client
Type messages to send over a secure substream. Press Enter on an empty line to quit.

🧪 Testing Multi-Client
You can run multiple clients simultaneously. Each will establish its own Noise session and multiplexed Yamux connection with the server.

📜 License
MIT
