//! Module: ops::storage::fleet_activation::codec
//!
//! Responsibility: select concrete activation codecs and reconstruct the read cache.
//! Does not own: activation policy, lifecycle scheduling or a second durable authority.
//! Boundary: only role lifecycle entrypoints select codecs; cache changes follow stable writes.

use crate::{
    cdk::serialize::serialize,
    ops::storage::fleet_activation::FleetActivationOpsError,
    storage::stable::fleet_activation::{
        FleetActivation as DurableActivation, FleetActivationRecord,
        MAX_FLEET_ACTIVATION_RECORD_BYTES, OrdinaryActivationRecord, RootActivationRecord,
        WasmStoreActivationRecord,
    },
    view::fleet_activation::FleetActivationView,
};
use serde::{Serialize, de::DeserializeOwned};
use std::cell::RefCell;

type Encode = fn(&FleetActivationView) -> Result<FleetActivationRecord, FleetActivationOpsError>;
type Decode = fn(&[u8]) -> Result<FleetActivationView, FleetActivationOpsError>;

#[derive(Clone, Copy)]
struct Codec {
    tag: u8,
    encode: Encode,
    decode: Decode,
}
struct SelectedActivation {
    codec: Option<Codec>,
    loaded: bool,
    cached: Option<FleetActivationView>,
}
thread_local! {
    static SELECTED: RefCell<SelectedActivation> = const { RefCell::new(SelectedActivation {
        codec: None,
        loaded: false,
        cached: None,
    }) };
}

fn select(codec: Codec) {
    SELECTED.with_borrow_mut(|selected| {
        assert!(
            selected.cached.is_none()
                || selected
                    .codec
                    .is_some_and(|current| current.tag == codec.tag),
            "activation codec cannot change after protected state is loaded",
        );
        selected.codec = Some(codec);
        selected.loaded = false;
        selected.cached = None;
    });
}

pub(super) fn select_ordinary() {
    select(Codec {
        tag: 1,
        encode: encode_ordinary,
        decode: decode_ordinary,
    });
}

pub(super) fn select_root() {
    select(Codec {
        tag: 2,
        encode: encode_root,
        decode: decode_root,
    });
}

pub(super) fn select_wasm_store() {
    select(Codec {
        tag: 3,
        encode: encode_wasm_store,
        decode: decode_wasm_store,
    });
}

/// Single-record access through the selected codec; the projection is never persisted.
pub(super) struct FleetActivation;

impl FleetActivation {
    pub(super) fn get() -> Option<FleetActivationView> {
        Self::with(Clone::clone)
    }

    pub(super) fn with<T>(read: impl FnOnce(&Option<FleetActivationView>) -> T) -> T {
        SELECTED.with_borrow_mut(|selected| {
            if !selected.loaded {
                selected.cached = DurableActivation::get().map(|record| {
                    (selected
                        .codec
                        .expect("activation lifecycle must select a codec")
                        .decode)(&record.bytes)
                    .expect("protected activation bytes must match the lifecycle-selected codec")
                });
                selected.loaded = true;
            }
            read(&selected.cached)
        })
    }

    pub(super) fn initialize(record: FleetActivationView) -> bool {
        let encoded = encode(&record).expect("activation encoding must pass preflight");
        if !DurableActivation::initialize(encoded) {
            return false;
        }
        retain(record);
        true
    }

    pub(super) fn replace(record: FleetActivationView) -> bool {
        let encoded = encode(&record).expect("activation encoding must pass preflight");
        if !DurableActivation::replace(encoded) {
            return false;
        }
        retain(record);
        true
    }

    #[cfg(test)]
    pub(super) fn export() -> FleetActivationData {
        FleetActivationData {
            record: Self::get(),
        }
    }

    #[cfg(test)]
    pub(super) fn import(data: FleetActivationData) {
        let encoded = data.record.as_ref().map(|record| encode(record).unwrap());
        DurableActivation::import(
            crate::storage::stable::fleet_activation::FleetActivationData { record: encoded },
        );
        SELECTED.with_borrow_mut(|selected| {
            selected.cached = None;
            selected.loaded = false;
            if data.record.is_none() {
                selected.codec = None;
            }
        });
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct FleetActivationData {
    pub record: Option<FleetActivationView>,
}

fn retain(record: FleetActivationView) {
    SELECTED.with_borrow_mut(|selected| {
        selected.cached = Some(record);
        selected.loaded = true;
    });
}

pub(super) fn encode(
    record: &FleetActivationView,
) -> Result<FleetActivationRecord, FleetActivationOpsError> {
    SELECTED.with_borrow(|selected| {
        (selected
            .codec
            .expect("activation lifecycle must select a codec")
            .encode)(record)
    })
}

#[cfg(test)]
pub(super) fn forget_projection() {
    SELECTED.with_borrow_mut(|selected| {
        selected.loaded = false;
        selected.cached = None;
    });
}

fn encode_concrete<T: Serialize>(
    tag: u8,
    record: &T,
) -> Result<FleetActivationRecord, FleetActivationOpsError> {
    let mut bytes = vec![b'C', b'A', 1, tag];
    bytes.extend(
        serialize(record).map_err(|error| FleetActivationOpsError::Encode(error.to_string()))?,
    );
    let maximum = MAX_FLEET_ACTIVATION_RECORD_BYTES as usize;
    if bytes.len() > maximum {
        return Err(FleetActivationOpsError::RecordTooLarge {
            maximum,
            observed: bytes.len(),
        });
    }
    Ok(FleetActivationRecord { bytes })
}

fn decode_concrete<T: DeserializeOwned>(
    tag: u8,
    bytes: &[u8],
) -> Result<T, FleetActivationOpsError> {
    if bytes.len() > MAX_FLEET_ACTIVATION_RECORD_BYTES as usize
        || !bytes.starts_with(&[b'C', b'A', 1, tag])
    {
        return Err(invalid(
            "activation encoding does not match the selected role",
        ));
    }
    let mut reader = std::io::Cursor::new(&bytes[4..]);
    let record = ciborium::de::from_reader(&mut reader)
        .map_err(|_| invalid("invalid activation encoding"))?;
    if usize::try_from(reader.position()).ok() != Some(bytes.len() - 4) {
        return Err(invalid("activation encoding contains trailing bytes"));
    }
    Ok(record)
}

fn invalid(reason: &str) -> FleetActivationOpsError {
    FleetActivationOpsError::InvalidRecord {
        reason: reason.to_string(),
    }
}

fn encode_ordinary(
    record: &FleetActivationView,
) -> Result<FleetActivationRecord, FleetActivationOpsError> {
    if record.root_authority.is_some()
        || record.wasm_store_authority.is_some()
        || record.cascade_manifest.is_some()
        || !record.credential_manifests.is_empty()
    {
        return Err(invalid(
            "ordinary activation contains infrastructure authority",
        ));
    }
    encode_concrete(
        1,
        &OrdinaryActivationRecord {
            state: record.state.clone(),
            prepared_state_snapshot_hash: record.prepared_state_snapshot_hash,
            prepared_topology_snapshot_hash: record.prepared_topology_snapshot_hash,
            component_runtime: record.component_runtime.clone(),
        },
    )
}

pub(super) fn decode_ordinary(
    bytes: &[u8],
) -> Result<FleetActivationView, FleetActivationOpsError> {
    let record: OrdinaryActivationRecord = decode_concrete(1, bytes)?;
    Ok(FleetActivationView {
        state: record.state,
        prepared_state_snapshot_hash: record.prepared_state_snapshot_hash,
        prepared_topology_snapshot_hash: record.prepared_topology_snapshot_hash,
        component_runtime: record.component_runtime,
        root_authority: None,
        wasm_store_authority: None,
        cascade_manifest: None,
        credential_manifests: Vec::new(),
    })
}

fn encode_root(
    record: &FleetActivationView,
) -> Result<FleetActivationRecord, FleetActivationOpsError> {
    if record.component_runtime.is_some() {
        return Err(invalid("Root activation contains Component runtime state"));
    }
    encode_concrete(
        2,
        &RootActivationRecord {
            state: record.state.clone(),
            prepared_state_snapshot_hash: record.prepared_state_snapshot_hash,
            prepared_topology_snapshot_hash: record.prepared_topology_snapshot_hash,
            root_authority: record
                .root_authority
                .clone()
                .ok_or_else(|| invalid("Root authority is absent"))?,
            wasm_store_authority: record
                .wasm_store_authority
                .clone()
                .ok_or_else(|| invalid("Store authority is absent"))?,
            cascade_manifest: record.cascade_manifest.clone(),
            credential_manifests: record.credential_manifests.clone(),
        },
    )
}

fn decode_root(bytes: &[u8]) -> Result<FleetActivationView, FleetActivationOpsError> {
    let record: RootActivationRecord = decode_concrete(2, bytes)?;
    Ok(FleetActivationView {
        state: record.state,
        prepared_state_snapshot_hash: record.prepared_state_snapshot_hash,
        prepared_topology_snapshot_hash: record.prepared_topology_snapshot_hash,
        root_authority: Some(record.root_authority),
        wasm_store_authority: Some(record.wasm_store_authority),
        cascade_manifest: record.cascade_manifest,
        credential_manifests: record.credential_manifests,
        component_runtime: None,
    })
}

fn encode_wasm_store(
    record: &FleetActivationView,
) -> Result<FleetActivationRecord, FleetActivationOpsError> {
    if record.component_runtime.is_some()
        || record.root_authority.is_some()
        || record.cascade_manifest.is_some()
        || !record.credential_manifests.is_empty()
    {
        return Err(invalid(
            "Store activation contains unrelated role authority",
        ));
    }
    encode_concrete(
        3,
        &WasmStoreActivationRecord {
            state: record.state.clone(),
            prepared_state_snapshot_hash: record.prepared_state_snapshot_hash,
            prepared_topology_snapshot_hash: record.prepared_topology_snapshot_hash,
            wasm_store_authority: record
                .wasm_store_authority
                .clone()
                .ok_or_else(|| invalid("Store authority is absent"))?,
        },
    )
}

fn decode_wasm_store(bytes: &[u8]) -> Result<FleetActivationView, FleetActivationOpsError> {
    let record: WasmStoreActivationRecord = decode_concrete(3, bytes)?;
    Ok(FleetActivationView {
        state: record.state,
        prepared_state_snapshot_hash: record.prepared_state_snapshot_hash,
        prepared_topology_snapshot_hash: record.prepared_topology_snapshot_hash,
        wasm_store_authority: Some(record.wasm_store_authority),
        root_authority: None,
        component_runtime: None,
        cascade_manifest: None,
        credential_manifests: Vec::new(),
    })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::{
            AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildId,
            ReleaseBuildNonce,
        },
        storage::stable::fleet_activation::{
            FleetActivationEvidenceRecord, FleetActivationIdentityRecord,
            FleetActivationStateRecord,
        },
    };

    fn ordinary() -> FleetActivationView {
        FleetActivationView {
            state: FleetActivationStateRecord::Prepared {
                identity: FleetActivationIdentityRecord {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([1; 32]),
                        },
                        app: AppId::from("audit"),
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
                application_init_args: Some(vec![4, 5]),
            },
            root_authority: None,
            wasm_store_authority: None,
            prepared_state_snapshot_hash: Some([6; 32]),
            prepared_topology_snapshot_hash: Some([7; 32]),
            cascade_manifest: None,
            credential_manifests: Vec::new(),
            component_runtime: None,
        }
    }

    #[test]
    fn ordinary_roundtrip_rejects_other_roles_trailing_bytes_and_missing_fields() {
        let view = ordinary();
        let record = encode_ordinary(&view).unwrap();
        assert_eq!(decode_ordinary(&record.bytes).unwrap(), view);
        for tag in [0, 2, 3, 255] {
            let mut wrong = record.bytes.clone();
            wrong[3] = tag;
            assert!(matches!(
                decode_ordinary(&wrong),
                Err(FleetActivationOpsError::InvalidRecord { .. })
            ));
        }
        let mut trailing = record.bytes.clone();
        trailing.push(0);
        assert!(matches!(
            decode_ordinary(&trailing),
            Err(FleetActivationOpsError::InvalidRecord { .. })
        ));
        let mut value: ciborium::Value =
            crate::cdk::serialize::deserialize(&record.bytes[4..]).unwrap();
        let ciborium::Value::Map(ref mut fields) = value else {
            panic!("record must be a map")
        };
        fields.retain(|(key, _)| key.as_text() != Some("component_runtime"));
        let mut omitted = vec![b'C', b'A', 1, 1];
        omitted.extend(serialize(&value).unwrap());
        assert!(matches!(
            decode_ordinary(&omitted),
            Err(FleetActivationOpsError::InvalidRecord { .. })
        ));
    }

    #[test]
    fn restart_reconstructs_projection_and_oversize_admission_preserves_durable_state() {
        FleetActivation::import(FleetActivationData::default());
        select_ordinary();
        let view = ordinary();
        assert!(FleetActivation::initialize(view.clone()));
        assert!(!FleetActivation::initialize(view.clone()));
        forget_projection();
        assert_eq!(FleetActivation::get(), Some(view.clone()));
        let before = DurableActivation::get();
        let mut oversized = view;
        let FleetActivationStateRecord::Prepared {
            application_init_args,
            ..
        } = &mut oversized.state
        else {
            unreachable!()
        };
        *application_init_args = Some(vec![0; MAX_FLEET_ACTIVATION_RECORD_BYTES as usize]);
        assert!(matches!(
            encode(&oversized),
            Err(FleetActivationOpsError::RecordTooLarge { .. })
        ));
        assert_eq!(DurableActivation::get(), before);
        FleetActivation::import(FleetActivationData::default());
    }
}
