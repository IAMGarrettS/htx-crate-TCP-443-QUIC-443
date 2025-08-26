use snow::Builder;
use std::fs;

fn main() -> std::io::Result<()> {
    let params: snow::params::NoiseParams = "Noise_XK_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
    let builder = Builder::new(params);
    let keypair = builder.generate_keypair().unwrap();
    fs::write("client.key", &keypair.private)?;
    fs::write("client.pub", &keypair.public)?;
    Ok(())
}
