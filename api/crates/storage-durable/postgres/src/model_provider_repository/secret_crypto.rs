use super::*;

pub(super) fn encrypt_secret_json(secret_json: &Value, master_key: &str) -> Result<Value> {
    if master_key.is_empty() {
        bail!(ControlPlaneError::InvalidInput(
            "provider_secret_master_key"
        ));
    }

    let plaintext = serde_json::to_vec(secret_json)?;
    Ok(json!({
        "algorithm": "xor_v1",
        "ciphertext": xor_hex(&plaintext, master_key.as_bytes()),
    }))
}

pub(super) fn decrypt_secret_json(
    encrypted_secret_json: &Value,
    master_key: &str,
) -> Result<Value> {
    if master_key.is_empty() {
        bail!(ControlPlaneError::InvalidInput(
            "provider_secret_master_key"
        ));
    }

    let algorithm = encrypted_secret_json
        .get("algorithm")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing secret encryption algorithm"))?;
    if algorithm != "xor_v1" {
        bail!(anyhow!(
            "unsupported secret encryption algorithm: {algorithm}"
        ));
    }
    let ciphertext = encrypted_secret_json
        .get("ciphertext")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing secret ciphertext"))?;
    let plaintext = xor_hex_decode(ciphertext, master_key.as_bytes())?;
    Ok(serde_json::from_slice(&plaintext)?)
}

pub(super) fn xor_hex(bytes: &[u8], key: &[u8]) -> String {
    bytes
        .iter()
        .enumerate()
        .map(|(index, byte)| format!("{:02x}", byte ^ key[index % key.len()]))
        .collect::<String>()
}

pub(super) fn xor_hex_decode(ciphertext: &str, key: &[u8]) -> Result<Vec<u8>> {
    if !ciphertext.len().is_multiple_of(2) {
        bail!(anyhow!("invalid ciphertext length"));
    }

    let mut encrypted = Vec::with_capacity(ciphertext.len() / 2);
    let mut chars = ciphertext.as_bytes().chunks_exact(2);
    for chunk in &mut chars {
        let pair = std::str::from_utf8(chunk)?;
        encrypted.push(u8::from_str_radix(pair, 16)?);
    }

    Ok(encrypted
        .into_iter()
        .enumerate()
        .map(|(index, byte)| byte ^ key[index % key.len()])
        .collect())
}
