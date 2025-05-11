use crate::TC;
use sha2::{Digest, Sha256};

// format_tc
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn format_cycles(cycles: u128) -> String {
    format!("{:.6} TC", cycles as f64 / TC as f64)
}

// get_wasm_hash
#[must_use]
pub fn get_wasm_hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);

    hasher.finalize().to_vec()
}
