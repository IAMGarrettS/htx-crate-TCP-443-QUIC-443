// in htx/src/mux/handshake.rs
pub fn handshake_xk_initiator_builder(
    client_static_priv: &[u8],
    server_static_pub: &[u8],
) -> Result<snow::Builder, anyhow::Error> {
    let mut builder =
        snow::Builder::new("Noise_XK_25519_ChaChaPoly_BLAKE2s".parse()?);
    builder.local_private_key(client_static_priv);
    builder.remote_public_key(server_static_pub);
    Ok(builder)
}
