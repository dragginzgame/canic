use crate::storage::prelude::*;

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
/// AuthStateRecord
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AuthStateRecord {
    pub delegated_sessions: Vec<DelegatedSessionRecord>,

    pub delegated_session_bootstrap_bindings: Vec<DelegatedSessionBootstrapBindingRecord>,

    pub attestation_public_keys: Vec<AttestationPublicKeyRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::{serialize::deserialize, serialize::serialize};
    use serde::ser::SerializeMap;

    #[test]
    fn auth_state_decode_drops_legacy_delegated_token_use_markers() {
        let legacy = LegacyAuthStateRecord {
            delegated_sessions: vec![DelegatedSessionRecord {
                wallet_pid: p(1),
                delegated_pid: p(2),
                issued_at: 10,
                expires_at: 20,
                bootstrap_token_fingerprint: None,
            }],
            delegated_session_bootstrap_bindings: Vec::new(),
            legacy_uses: vec![LegacyUseRecord {
                issuer_shard_pid: p(3),
                subject: p(4),
                cert_hash: [5; 32],
                nonce: [6; 16],
                used_at: 11,
                expires_at: 20,
            }],
            attestation_public_keys: Vec::new(),
        };
        let bytes = serialize(&legacy).expect("legacy auth state serializes");

        let decoded: AuthStateRecord =
            deserialize(&bytes).expect("new auth state ignores removed field");

        assert_eq!(decoded.delegated_sessions, legacy.delegated_sessions);
        assert!(decoded.delegated_session_bootstrap_bindings.is_empty());
        assert!(decoded.attestation_public_keys.is_empty());
    }

    ///
    /// LegacyUseRecord
    ///
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
    struct LegacyUseRecord {
        issuer_shard_pid: Principal,
        subject: Principal,
        cert_hash: [u8; 32],
        nonce: [u8; 16],
        used_at: u64,
        expires_at: u64,
    }

    ///
    /// LegacyAuthStateRecord
    ///
    #[derive(Clone, Debug)]
    struct LegacyAuthStateRecord {
        delegated_sessions: Vec<DelegatedSessionRecord>,
        delegated_session_bootstrap_bindings: Vec<DelegatedSessionBootstrapBindingRecord>,
        legacy_uses: Vec<LegacyUseRecord>,
        attestation_public_keys: Vec<AttestationPublicKeyRecord>,
    }

    impl Serialize for LegacyAuthStateRecord {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut map = serializer.serialize_map(Some(4))?;
            map.serialize_entry("delegated_sessions", &self.delegated_sessions)?;
            map.serialize_entry(
                "delegated_session_bootstrap_bindings",
                &self.delegated_session_bootstrap_bindings,
            )?;
            let legacy_key = ["delegated", "_token", "_uses"].concat();
            map.serialize_entry(&legacy_key, &self.legacy_uses)?;
            map.serialize_entry("attestation_public_keys", &self.attestation_public_keys)?;
            map.end()
        }
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }
}
