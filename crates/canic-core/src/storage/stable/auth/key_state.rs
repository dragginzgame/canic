use super::{AttestationPublicKeyRecord, DelegationStateRecord, ShardPublicKeyRecord};
use crate::storage::prelude::*;

// Resolve the current root public key, if one is stored.
pub(super) fn get_root_public_key(data: &DelegationStateRecord) -> Option<Vec<u8>> {
    data.root_public_key.clone()
}

// Persist the current root public key.
pub(super) fn set_root_public_key(data: &mut DelegationStateRecord, public_key_sec1: Vec<u8>) {
    data.root_public_key = Some(public_key_sec1);
}

// Resolve one shard public key by shard principal.
pub(super) fn get_shard_public_key(
    data: &DelegationStateRecord,
    shard_pid: Principal,
) -> Option<Vec<u8>> {
    data.shard_public_keys
        .iter()
        .find(|entry| entry.shard_pid == shard_pid)
        .map(|entry| entry.public_key_sec1.clone())
}

// Persist or replace one shard public key by shard principal.
pub(super) fn set_shard_public_key(
    data: &mut DelegationStateRecord,
    shard_pid: Principal,
    public_key_sec1: Vec<u8>,
) {
    upsert_shard_public_key_record(&mut data.shard_public_keys, shard_pid, public_key_sec1);
}

// Resolve one attestation public key by key id.
pub(super) fn get_attestation_public_key(
    data: &DelegationStateRecord,
    key_id: u32,
) -> Option<AttestationPublicKeyRecord> {
    data.attestation_public_keys
        .iter()
        .find(|entry| entry.key_id == key_id)
        .cloned()
}

// Resolve the full attestation public key set.
pub(super) fn get_attestation_public_keys(
    data: &DelegationStateRecord,
) -> Vec<AttestationPublicKeyRecord> {
    data.attestation_public_keys.clone()
}

// Replace the attestation public key set.
pub(super) fn set_attestation_public_keys(
    data: &mut DelegationStateRecord,
    keys: Vec<AttestationPublicKeyRecord>,
) {
    data.attestation_public_keys = keys;
}

// Upsert one attestation public key by key id.
pub(super) fn upsert_attestation_public_key(
    data: &mut DelegationStateRecord,
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

// Upsert one shard public key record by shard principal.
fn upsert_shard_public_key_record(
    entries: &mut Vec<ShardPublicKeyRecord>,
    shard_pid: Principal,
    public_key_sec1: Vec<u8>,
) {
    if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.shard_pid == shard_pid)
    {
        entry.public_key_sec1 = public_key_sec1;
    } else {
        entries.push(ShardPublicKeyRecord {
            shard_pid,
            public_key_sec1,
        });
    }
}
