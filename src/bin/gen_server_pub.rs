use snow::Builder;
use std::fs;

fn main() -> std::io::Result<()> {
    // Read the existing static private key from file
    let priv_key = fs::read("server.key")?;

    // Create a Noise builder so we can generate a keypair structure
    let params: snow::params::NoiseParams =
        "Noise_XK_25519_ChaChaPoly_BLAKE2s".parse().unwrap();

    // This will discard the random private key it would have generated,
    // and replace it with the one you already have, so we can access the public half.
    let keypair = Builder::new(params)
        .local_private_key(&priv_key)
        .unwrap()
        .generate_keypair()
        .unwrap();

    // Save the public part to server.pub
    fs::write("server.pub", &keypair.public)?;

    println!("server.pub written ({} bytes)", keypair.public.len());
    Ok(())
}
