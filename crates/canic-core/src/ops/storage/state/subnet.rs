use crate::{
    dto::state::{SubnetStateInput, SubnetStateResponse},
    ops::storage::state::mapper::SubnetStateMapper,
    storage::stable::state::subnet::{RootPublicKeyRecord, SubnetState, SubnetStateRecord},
};
use sha2::{Digest, Sha256};

///
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    /// Export the current subnet state as a DTO snapshot.
    #[must_use]
    pub fn snapshot_input() -> SubnetStateInput {
        SubnetStateMapper::record_to_input(SubnetState::export())
    }

    /// Export the current subnet state as a response snapshot.
    #[must_use]
    pub fn snapshot_response() -> SubnetStateResponse {
        SubnetStateMapper::record_to_response(SubnetState::export())
    }

    // Import sanitized subnet state from an operational snapshot.
    fn import_record(data: SubnetStateRecord) {
        SubnetState::import(sanitized_subnet_state(data));
    }

    /// Import subnet state from a DTO snapshot.
    pub fn import_input(view: SubnetStateInput) {
        let record = SubnetStateMapper::input_to_record(view);
        Self::import_record(record);
    }

    /// Resolve the delegated root public key from subnet state.
    #[must_use]
    pub fn delegated_root_public_key(key_name: &str) -> Option<Vec<u8>> {
        SubnetState::export()
            .auth
            .delegated_root_public_key
            .filter(|record| root_key_identity_matches(record, key_name))
            .map(|record| record.public_key_sec1)
    }

    /// Publish delegated root public key material into subnet state.
    pub fn set_delegated_root_public_key(key_name: String, public_key_sec1: Vec<u8>) {
        let mut record = SubnetState::export();
        record.auth.delegated_root_public_key = Some(RootPublicKeyRecord {
            key_hash: public_key_hash(&public_key_sec1),
            key_name,
            public_key_sec1,
        });
        SubnetState::import(record);
    }
}

// Drop invalid delegated root key material before it enters local subnet state.
fn sanitized_subnet_state(mut data: SubnetStateRecord) -> SubnetStateRecord {
    if let Some(root_key) = data.auth.delegated_root_public_key.as_ref()
        && !root_key_identity_matches(root_key, &root_key.key_name)
    {
        data.auth.delegated_root_public_key = None;
    }

    data
}

// Hash one SEC1 public key for identity validation.
fn public_key_hash(public_key_sec1: &[u8]) -> [u8; 32] {
    Sha256::digest(public_key_sec1).into()
}

// Validate one delegated root key record against its declared identity.
fn root_key_identity_matches(record: &RootPublicKeyRecord, expected_key_name: &str) -> bool {
    record.key_name == expected_key_name
        && public_key_hash(&record.public_key_sec1) == record.key_hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::state::{SubnetAuthStateInput, SubnetRootPublicKeyInput};

    #[test]
    fn subnet_state_import_preserves_valid_delegated_root_key() {
        let public_key_sec1 = vec![4, 5, 6];
        let key_name = "subnet_state_import_key".to_string();

        SubnetStateOps::import_input(SubnetStateInput {
            auth: SubnetAuthStateInput {
                delegated_root_public_key: Some(SubnetRootPublicKeyInput {
                    key_hash: public_key_hash(&public_key_sec1),
                    key_name: key_name.clone(),
                    public_key_sec1: public_key_sec1.clone(),
                }),
            },
        });

        assert_eq!(
            SubnetStateOps::delegated_root_public_key(&key_name),
            Some(public_key_sec1)
        );
    }

    #[test]
    fn subnet_state_import_rejects_root_key_hash_drift() {
        let key_name = "subnet_state_bad_key".to_string();

        SubnetStateOps::import_input(SubnetStateInput {
            auth: SubnetAuthStateInput {
                delegated_root_public_key: Some(SubnetRootPublicKeyInput {
                    key_hash: [9; 32],
                    key_name: key_name.clone(),
                    public_key_sec1: vec![7, 8, 9],
                }),
            },
        });

        assert_eq!(SubnetStateOps::delegated_root_public_key(&key_name), None);
    }
}
