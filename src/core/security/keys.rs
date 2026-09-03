use ed25519_dalek::SigningKey;

pub fn generate_keypair() -> (String, String) {
    // (private_hex, public_hex)
    let mut secret = [0u8; 32];
    getrandom::fill(&mut secret).expect("getrandom failed");
    let sk = SigningKey::from_bytes(&secret);
    let vk = sk.verifying_key();
    (hex::encode(sk.to_bytes()), hex::encode(vk.to_bytes()))
}
pub fn public_from_private_hex(priv_hex: &str) -> Option<String> {
    let bytes = hex::decode(priv_hex).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let arr: [u8; 32] = bytes.try_into().ok()?;
    let sk = SigningKey::from_bytes(&arr);
    Some(hex::encode(sk.verifying_key().to_bytes()))
}
pub fn verify_write_credential(stored_pub_hex: &str, priv_hex: &str) -> bool {
    match public_from_private_hex(priv_hex) {
        Some(derived) => derived.eq_ignore_ascii_case(stored_pub_hex),
        None => false,
    }
}
