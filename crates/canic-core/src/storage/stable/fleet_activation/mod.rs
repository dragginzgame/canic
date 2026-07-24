//! Module: storage::stable::fleet_activation
//!
//! Responsibility: persist the sole protected Fleet activation record at memory ID 21.
//! Does not own: install admission, state transitions, Candid DTOs, or lifecycle scheduling.
//! Boundary: ops validates and converts complete records before this single-record store mutates.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    ids::{FleetBinding, ReleaseBuildId},
    role_contract::allocation::memory::activation::FLEET_ACTIVATION_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

/// Maximum canonical bytes admitted for the complete protected activation record.
pub const MAX_FLEET_ACTIVATION_RECORD_BYTES: u32 = 2_097_152;

const FLEET_ACTIVATION_RECORD_KEY: u8 = 0;

eager_static! {
    static FLEET_ACTIVATION: RefCell<
        StableBtreeMap<u8, FleetActivationRecord, VirtualMemory<DefaultMemoryImpl>>,
    > = RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(
        authority = CANIC_CORE_MEMORY_AUTHORITY,
        key = "canic.core.fleet_activation.v1",
        ty = FleetActivation,
        id = FLEET_ACTIVATION_ID,
    )));
}

///
/// FleetActivationIdentityRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetActivationIdentityRecord {
    pub fleet: FleetBinding,
    pub operation_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

///
/// FleetCascadeActivationEvidenceRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetCascadeActivationEvidenceRecord {
    Source {
        cascade_manifest_hash: [u8; 32],
    },
    Applied {
        state_snapshot_hash: [u8; 32],
        topology_snapshot_hash: [u8; 32],
    },
}

///
/// FleetCredentialGenerationRefRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetCredentialGenerationRefRecord {
    pub generation: u64,
    pub manifest_hash: [u8; 32],
}

///
/// FleetActivationEvidenceRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetActivationEvidenceRecord {
    pub cascade: Option<FleetCascadeActivationEvidenceRecord>,
    pub credential: Option<FleetCredentialGenerationRefRecord>,
}

///
/// FleetActivationStateRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetActivationStateRecord {
    Prepared {
        identity: FleetActivationIdentityRecord,
        evidence: FleetActivationEvidenceRecord,
    },
    Active {
        identity: FleetActivationIdentityRecord,
        evidence: FleetActivationEvidenceRecord,
        activated_at_ns: u64,
    },
}

///
/// FleetCascadeManifestEntryRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetCascadeManifestEntryRecord {
    pub principal: Principal,
    pub state_snapshot_hash: [u8; 32],
    pub topology_snapshot_hash: [u8; 32],
}

///
/// FleetCredentialManifestEntryRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetCredentialManifestEntryRecord {
    pub root_issuer: Principal,
    pub subject_canister: Principal,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub key_identity_hash: [u8; 32],
    pub cert_hash: [u8; 32],
    pub proof_hash: [u8; 32],
    pub bundle_hash: [u8; 32],
}

///
/// FleetCredentialManifestRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetCredentialManifestRecord {
    pub fleet: crate::ids::FleetKey,
    pub activation_id: [u8; 32],
    pub generation: u64,
    pub root_policy_set_hash: [u8; 32],
    pub renewal_template_set_hash: [u8; 32],
    pub entries: Vec<FleetCredentialManifestEntryRecord>,
}

///
/// FleetActivationRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetActivationRecord {
    pub state: FleetActivationStateRecord,
    pub cascade_manifest: Option<Vec<FleetCascadeManifestEntryRecord>>,
    pub credential_manifests: Vec<FleetCredentialManifestRecord>,
}

impl FleetActivationRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetActivationRecord";
}

impl_storable_bounded!(
    FleetActivationRecord,
    MAX_FLEET_ACTIVATION_RECORD_BYTES,
    false
);

///
/// FleetActivationData
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FleetActivationData {
    pub record: Option<FleetActivationRecord>,
}

impl FleetActivationData {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetActivationData";
}

///
/// FleetActivation
///

pub struct FleetActivation;

impl FleetActivation {
    #[must_use]
    pub(crate) fn get() -> Option<FleetActivationRecord> {
        FLEET_ACTIVATION.with_borrow(|store| store.get(&FLEET_ACTIVATION_RECORD_KEY))
    }

    pub(crate) fn initialize(record: FleetActivationRecord) -> bool {
        FLEET_ACTIVATION.with_borrow_mut(|store| {
            if store.get(&FLEET_ACTIVATION_RECORD_KEY).is_some() {
                return false;
            }
            let previous = store.insert(FLEET_ACTIVATION_RECORD_KEY, record);
            debug_assert!(previous.is_none());
            true
        })
    }

    #[must_use]
    pub(crate) fn export() -> FleetActivationData {
        FleetActivationData {
            record: Self::get(),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: FleetActivationData) {
        FLEET_ACTIVATION.with_borrow_mut(|store| {
            store.clear_new();
            if let Some(record) = data.record {
                store.insert(FLEET_ACTIVATION_RECORD_KEY, record);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::structures::storable::Storable,
        ids::{AppId, CanonicalNetworkId, FleetId, FleetKey, ReleaseBuildId, ReleaseBuildNonce},
    };

    fn record() -> FleetActivationRecord {
        FleetActivationRecord {
            state: FleetActivationStateRecord::Prepared {
                identity: FleetActivationIdentityRecord {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            network: CanonicalNetworkId::public_ic(),
                            fleet_id: FleetId::from_generated_bytes([1; 32]),
                        },
                        app: AppId::from("toko"),
                    },
                    operation_id: [2; 32],
                    release_build_id: ReleaseBuildId::from_nonce(
                        ReleaseBuildNonce::from_random_bytes([3; 32]),
                    ),
                },
                evidence: FleetActivationEvidenceRecord {
                    cascade: None,
                    credential: None,
                },
            },
            cascade_manifest: None,
            credential_manifests: Vec::new(),
        }
    }

    #[test]
    fn prepared_record_roundtrips_through_stable_encoding() {
        let record = record();
        let bytes = record.to_bytes();
        let decoded = FleetActivationRecord::from_bytes(bytes);

        assert_eq!(decoded, record);
    }

    #[test]
    fn store_initializes_once_without_an_unbound_record() {
        FleetActivation::import(FleetActivationData::default());
        let record = record();

        assert_eq!(FleetActivation::get(), None);
        assert!(FleetActivation::initialize(record.clone()));
        assert_eq!(FleetActivation::get(), Some(record.clone()));
        assert!(!FleetActivation::initialize(record));

        FleetActivation::import(FleetActivationData::default());
    }
}
