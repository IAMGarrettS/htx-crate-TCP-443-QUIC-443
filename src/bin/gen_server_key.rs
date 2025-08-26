// src/bin/gen_server_key.rs

use snow::Builder;
use std::fs;

fn main() -> std::io::Result<()> {
    // Define the handshake pattern & crypto suite
    let params: snow::params::NoiseParams =
        "Noise_XK_25519_ChaChaPoly_BLAKE2s".parse().unwrap();

    // Generate a brand-new static keypair
    let kp = Builder::new(params).generate_keypair().unwrap();

    // Save the raw 32‑byte private key
    fs::write("server.key", &kp.private)?;
    // Save the matching 32‑byte public key
    fs::write("server.pub", &kp.public)?;

    println!(
        "Generated server.key and server.pub ({} bytes each)",
        kp.private.len()
    );

    Ok(())
}
