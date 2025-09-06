use crate::{interface::prelude::*, types::Subaccount, utils::time::now_secs};
use sha2::{Digest, Sha256};

/// derive_subaccount
/// use current time and a string salt.
#[must_use]
pub fn derive_subaccount(principal: &Principal, salt: &str) -> Subaccount {
    derive_subaccount_with(principal, now_secs(), salt.as_bytes())
}

/// derive_subaccount_with
/// derive a subaccount from principal + timestamp + arbitrary salt bytes.
pub fn derive_subaccount_with(
    principal: &Principal,
    timestamp: u64,
    salt: impl AsRef<[u8]>,
) -> Subaccount {
    let mut hasher = Sha256::new();
    hasher.update(principal.as_slice());
    hasher.update(timestamp.to_be_bytes());
    hasher.update(salt.as_ref());

    let hash = hasher.finalize();
    let mut sub = [0u8; 32];
    sub.copy_from_slice(&hash[..32]);

    Subaccount(sub)
}
