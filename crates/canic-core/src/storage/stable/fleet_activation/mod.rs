//! Module: storage::stable::fleet_activation
//!
//! Responsibility: persist the sole protected Fleet activation record at memory ID 38.
//! Does not own: install admission, state transitions, Candid DTOs, or lifecycle scheduling.
//! Boundary: ops validates and converts complete records before this single-record store mutates.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory},
    ids::{
        ComponentBinding, FleetBinding, FleetRegistryAuthority, FleetSubnetRootBinding,
        FleetSubnetRootReleaseSet, ManagedCanisterBinding, ReleaseBuildId, SubnetId,
    },
    role_contract::allocation::memory::activation::FLEET_ACTIVATION_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

/// Maximum canonical bytes admitted for the complete protected activation record.
pub const MAX_FLEET_ACTIVATION_RECORD_BYTES: u32 = 2_097_152;
/// Maximum credential generations retained while a Fleet is being activated.
pub const MAX_RETAINED_PREPARED_CREDENTIAL_GENERATIONS: usize = 2;

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
        application_init_args: Option<Vec<u8>>,
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
/// FleetSubnetRootAuthorityRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootAuthorityRecord {
    pub binding: FleetSubnetRootBinding,
    pub initial_release_set: FleetSubnetRootReleaseSet,
    pub expected_module_hash: [u8; 32],
}

///
/// ComponentRuntimeRecord
///
/// Protected Component-tree identity, Directory authority and activation receipt for one non-root.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeRecord {
    pub binding: ManagedCanisterBinding,
    pub directory: Option<ComponentRuntimeDirectoryRecord>,
    pub activation: Option<ComponentRuntimeActivationRecord>,
}

///
/// ComponentRuntimeDirectoryAuthorityRecord
///
/// Persisted Fleet and Component Directory authority for one managed Component-tree node.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeDirectoryAuthorityRecord {
    pub fleet: FleetDirectorySnapshotRecord,
    pub component: ComponentDirectoryHeadRecord,
}

///
/// FleetDirectorySnapshotRecord
///
/// Persisted root-local Fleet Directory projection retained by one Component runtime.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetDirectorySnapshotRecord {
    pub provenance: FleetDirectoryProvenanceRecord,
    pub fleet_subnet_roots: Vec<FleetSubnetRootDirectoryEntryRecord>,
}

///
/// FleetDirectoryProvenanceRecord
///
/// Persisted Registry authority and source root for one Fleet Directory projection.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetDirectoryProvenanceRecord {
    pub registry: FleetRegistryVersionRecord,
    pub source_fleet_subnet_root: Principal,
}

///
/// FleetRegistryVersionRecord
///
/// Persisted immutable identity of the Registry behind one Fleet Directory projection.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRegistryVersionRecord {
    pub authority: FleetRegistryAuthority,
    pub revision: u64,
    pub content_hash: [u8; 32],
}

///
/// FleetSubnetRootDirectoryEntryRecord
///
/// Persisted placement and lifecycle status of one Fleet Subnet Root Directory row.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDirectoryEntryRecord {
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub status: FleetSubnetRootStatusRecord,
}

///
/// FleetSubnetRootStatusRecord
///
/// Persisted lifecycle state of one Fleet Subnet Root Directory row.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetSubnetRootStatusRecord {
    Joining,
    Active,
    Draining,
    Removed,
}

///
/// ComponentDirectoryHeadRecord
///
/// Persisted independently versioned discovery head for one Component tree.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentDirectoryHeadRecord {
    pub provenance: ComponentDirectoryProvenanceRecord,
    pub descendant_count: u32,
}

///
/// ComponentDirectoryProvenanceRecord
///
/// Persisted Component Registry authority from which one Component Directory is derived.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentDirectoryProvenanceRecord {
    pub component: ComponentBinding,
    pub source_fleet_subnet_root: Principal,
    pub component_registry_revision: u64,
    pub component_registry_content_hash: [u8; 32],
    pub synchronized_at_ns: u64,
}

///
/// ComponentRuntimeDirectoryRecord
///
/// Exact target-local Directory authority committed before runtime activation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeDirectoryRecord {
    pub authority: ComponentRuntimeDirectoryAuthorityRecord,
    pub authority_hash: [u8; 32],
}

///
/// ComponentRuntimeActivationRecord
///
/// Exact target-local runtime activation receipt retained beside its Directory authority.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentRuntimeActivationRecord {
    pub directory: ComponentRuntimeDirectoryRecord,
    pub activated_at_ns: u64,
}

///
/// FleetActivationRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetActivationRecord {
    pub state: FleetActivationStateRecord,
    pub root_authority: Option<FleetSubnetRootAuthorityRecord>,
    pub prepared_state_snapshot_hash: Option<[u8; 32]>,
    pub prepared_topology_snapshot_hash: Option<[u8; 32]>,
    pub cascade_manifest: Option<Vec<FleetCascadeManifestEntryRecord>>,
    pub credential_manifests: Vec<FleetCredentialManifestRecord>,
    pub component_runtime: Option<ComponentRuntimeRecord>,
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

    pub(crate) fn replace(record: FleetActivationRecord) -> bool {
        FLEET_ACTIVATION.with_borrow_mut(|store| {
            if store.get(&FLEET_ACTIVATION_RECORD_KEY).is_none() {
                return false;
            }
            store.insert(FLEET_ACTIVATION_RECORD_KEY, record);
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
                            canonical_network_id: CanonicalNetworkId::public_ic(),
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
                application_init_args: Some(vec![4, 5, 6]),
            },
            root_authority: None,
            prepared_state_snapshot_hash: None,
            prepared_topology_snapshot_hash: None,
            cascade_manifest: None,
            credential_manifests: Vec::new(),
            component_runtime: None,
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
