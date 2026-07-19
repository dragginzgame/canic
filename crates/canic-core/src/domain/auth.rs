use sha2::{Digest, Sha256};

///
/// IC_ROOT_PUBLIC_KEY_RAW_LENGTH
///
pub const IC_ROOT_PUBLIC_KEY_RAW_LENGTH: usize = 96;

const CHAIN_KEY_DERIVATION_PATH_DOMAIN: &[u8] =
    b"CANIC_ROOT_DELEGATION_CHAIN_KEY_DERIVATION_PATH_V1";

///
/// MAINNET_IC_ROOT_PUBLIC_KEY_RAW
///
/// Raw 96-byte IC mainnet BLS root public key, not DER encoded.
pub const MAINNET_IC_ROOT_PUBLIC_KEY_RAW: [u8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH] = [
    0x81, 0x4c, 0x0e, 0x6e, 0xc7, 0x1f, 0xab, 0x58, 0x3b, 0x08, 0xbd, 0x81, 0x37, 0x3c, 0x25, 0x5c,
    0x3c, 0x37, 0x1b, 0x2e, 0x84, 0x86, 0x3c, 0x98, 0xa4, 0xf1, 0xe0, 0x8b, 0x74, 0x23, 0x5d, 0x14,
    0xfb, 0x5d, 0x9c, 0x0c, 0xd5, 0x46, 0xd9, 0x68, 0x5f, 0x91, 0x3a, 0x0c, 0x0b, 0x2c, 0xc5, 0x34,
    0x15, 0x83, 0xbf, 0x4b, 0x43, 0x92, 0xe4, 0x67, 0xdb, 0x96, 0xd6, 0x5b, 0x9b, 0xb4, 0xcb, 0x71,
    0x71, 0x12, 0xf8, 0x47, 0x2e, 0x0d, 0x5a, 0x4d, 0x14, 0x50, 0x5f, 0xfd, 0x74, 0x84, 0xb0, 0x12,
    0x91, 0x09, 0x1c, 0x5f, 0x87, 0xb9, 0x88, 0x83, 0x46, 0x3f, 0x98, 0x09, 0x1a, 0x0b, 0xaa, 0xae,
];

#[cfg(any(target_arch = "wasm32", test))]
const IC_ROOT_PK_DER_PREFIX: &[u8; 37] = b"\x30\x81\x82\x30\x1d\x06\x0d\x2b\x06\x01\x04\x01\x82\xdc\x7c\x05\x03\x01\x02\x01\x06\x0c\x2b\x06\x01\x04\x01\x82\xdc\x7c\x05\x03\x02\x01\x03\x61\x00";

pub fn is_mainnet_ic_root_public_key_raw(root_key: &[u8]) -> bool {
    root_key == MAINNET_IC_ROOT_PUBLIC_KEY_RAW
}

pub fn chain_key_derivation_path_hash(derivation_path: &[Vec<u8>]) -> [u8; 32] {
    let mut out = Vec::with_capacity(CHAIN_KEY_DERIVATION_PATH_DOMAIN.len() + 32);
    out.extend_from_slice(CHAIN_KEY_DERIVATION_PATH_DOMAIN);
    encode_len(&mut out, derivation_path.len());
    for component in derivation_path {
        encode_bytes(&mut out, component);
    }
    Sha256::digest(out).into()
}

fn encode_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    encode_len(out, bytes.len());
    out.extend_from_slice(bytes);
}

fn encode_len(out: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("chain-key derivation path length exceeds u32");
    out.extend_from_slice(&len.to_be_bytes());
}

#[cfg(any(target_arch = "wasm32", test))]
pub fn ic_root_public_key_raw_from_der_or_raw(root_key: &[u8]) -> Result<Vec<u8>, String> {
    if root_key.len() == IC_ROOT_PUBLIC_KEY_RAW_LENGTH {
        return Ok(root_key.to_vec());
    }

    let expected_length = IC_ROOT_PK_DER_PREFIX.len() + IC_ROOT_PUBLIC_KEY_RAW_LENGTH;
    if root_key.len() != expected_length {
        return Err("invalid IC root public key length".to_string());
    }
    if &root_key[..IC_ROOT_PK_DER_PREFIX.len()] != IC_ROOT_PK_DER_PREFIX {
        return Err("invalid IC root public key DER prefix".to_string());
    }
    Ok(root_key[IC_ROOT_PK_DER_PREFIX.len()..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_raw_ic_root_key_from_der_or_raw() {
        let mut der = IC_ROOT_PK_DER_PREFIX.to_vec();
        der.extend_from_slice(&[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]);

        assert_eq!(
            ic_root_public_key_raw_from_der_or_raw(&der).unwrap(),
            vec![9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]
        );
        assert_eq!(
            ic_root_public_key_raw_from_der_or_raw(&[8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]).unwrap(),
            vec![8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]
        );
    }
}
