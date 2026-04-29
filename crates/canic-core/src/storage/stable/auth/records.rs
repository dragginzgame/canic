use crate::storage::prelude::*;

///
/// ShardPublicKeyRecord
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShardPublicKeyRecord {
    pub shard_pid: Principal,
    pub public_key_sec1: Vec<u8>,
}

///
/// DelegatedSessionRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedSessionRecord {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    #[serde(default)]
    pub issued_at: u64,
    #[serde(default)]
    pub expires_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default)]
    pub bound_at: u64,
    #[serde(default)]
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
    pub status: AttestationKeyStatusRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<u64>,
}

///
/// DelegationStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DelegationStateRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_public_key: Option<Vec<u8>>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shard_public_keys: Vec<ShardPublicKeyRecord>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delegated_sessions: Vec<DelegatedSessionRecord>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delegated_session_bootstrap_bindings: Vec<DelegatedSessionBootstrapBindingRecord>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attestation_public_keys: Vec<AttestationPublicKeyRecord>,
}
