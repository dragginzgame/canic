use crate::storage::prelude::*;

///
/// ShardPublicKeyRecord
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShardPublicKeyRecord {
    pub shard_pid: Principal,
    pub public_key_sec1: Vec<u8>,
    pub key_name: String,
    pub key_hash: [u8; 32],
}

///
/// DelegatedSessionRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedSessionRecord {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
    pub bootstrap_token_fingerprint: Option<[u8; 32]>,
}

///
/// DelegatedSessionBootstrapBindingRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedSessionBootstrapBindingRecord {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    pub token_fingerprint: [u8; 32],
    pub bound_at: u64,
    pub expires_at: u64,
}

///
/// AttestationKeyStatusRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AttestationKeyStatusRecord {
    Current,
    Previous,
}

///
/// AttestationPublicKeyRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AttestationPublicKeyRecord {
    pub key_id: u32,
    pub public_key_sec1: Vec<u8>,
    pub key_name: String,
    pub key_hash: [u8; 32],
    pub status: AttestationKeyStatusRecord,
    pub valid_from: Option<u64>,
    pub valid_until: Option<u64>,
}

///
/// DelegationStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DelegationStateRecord {
    pub shard_public_keys: Vec<ShardPublicKeyRecord>,

    pub delegated_sessions: Vec<DelegatedSessionRecord>,

    pub delegated_session_bootstrap_bindings: Vec<DelegatedSessionBootstrapBindingRecord>,

    pub attestation_public_keys: Vec<AttestationPublicKeyRecord>,
}
