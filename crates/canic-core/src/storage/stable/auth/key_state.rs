use super::{AttestationPublicKeyRecord, AuthStateRecord};
use sha2::{Digest, Sha256};

// Resolve one attestation public key by key id.
pub(super) fn get_attestation_public_key(
    data: &AuthStateRecord,
    key_id: u32,
    key_name: &str,
) -> Option<AttestationPublicKeyRecord> {
    data.attestation_public_keys
        .iter()
        .find(|entry| {
            entry.key_id == key_id
                && key_identity_matches(
                    &entry.public_key_sec1,
                    &entry.key_name,
                    entry.key_hash,
                    key_name,
                )
        })
        .cloned()
}

// Resolve the full attestation public key set.
pub(super) fn get_attestation_public_keys(
    data: &AuthStateRecord,
    key_name: &str,
) -> Vec<AttestationPublicKeyRecord> {
    data.attestation_public_keys
        .iter()
        .filter(|entry| {
            key_identity_matches(
                &entry.public_key_sec1,
                &entry.key_name,
                entry.key_hash,
                key_name,
            )
        })
        .cloned()
        .collect()
}

// Replace the attestation public key set.
pub(super) fn set_attestation_public_keys(
    data: &mut AuthStateRecord,
    keys: Vec<AttestationPublicKeyRecord>,
) {
    data.attestation_public_keys = keys;
}

// Upsert one attestation public key by key id.
pub(super) fn upsert_attestation_public_key(
    data: &mut AuthStateRecord,
    key: AttestationPublicKeyRecord,
) {
    if let Some(existing) = data
        .attestation_public_keys
        .iter_mut()
        .find(|entry| entry.key_id == key.key_id)
    {
        *existing = key;
    } else {
        data.attestation_public_keys.push(key);
    }
}

// Compute the stable identity hash for a cached SEC1 public key.
fn public_key_hash(public_key_sec1: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(public_key_sec1);
    hasher.finalize().into()
}

// Validate that a stored key belongs to the expected configured key identity.
fn key_identity_matches(
    public_key_sec1: &[u8],
    cached_key_name: &str,
    cached_key_hash: [u8; 32],
    expected_key_name: &str,
) -> bool {
    cached_key_name == expected_key_name && public_key_hash(public_key_sec1) == cached_key_hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stable::auth::AttestationKeyStatusRecord;

    #[test]
    fn attestation_public_key_cache_requires_matching_identity() {
        let mut data = AuthStateRecord::default();
        data.attestation_public_keys
            .push(AttestationPublicKeyRecord {
                key_id: 1,
                public_key_sec1: vec![10, 11, 12],
                key_name: "key_a".to_string(),
                key_hash: public_key_hash(&[10, 11, 12]),
                status: AttestationKeyStatusRecord::Current,
                valid_from: Some(10),
                valid_until: None,
            });

        assert!(get_attestation_public_key(&data, 1, "key_a").is_some());
        assert!(get_attestation_public_key(&data, 1, "key_b").is_none());

        data.attestation_public_keys[0].key_hash = [1; 32];
        assert!(get_attestation_public_key(&data, 1, "key_a").is_none());
    }
}
