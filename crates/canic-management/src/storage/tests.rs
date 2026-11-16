//! Exercise the actual persistent codecs, including their untrusted record boundaries.
use super::*;
use ic_storage::{
    AllocationGrants, GenericAllocationPolicy, MemoryAuthority, MemoryRequest, MemoryRuntime,
    SealedDeclarationSnapshot, StableKey, StorageConfig, VectorMemory, adapter::StableCell,
};

fn encoded(tag: &[u8], value: &impl Serialize, limits: ic_wire::Limits) -> Vec<u8> {
    let mut bytes = tag.to_vec();
    bytes.extend(ic_wire::encode(value, ic_wire::Encoding::Payload, limits).unwrap());
    bytes
}

fn journal(worker: &str) -> Journal {
    Journal(
        Status {
            worker: Some(worker.into()),
            ..Default::default()
        },
        Default::default(),
        vec![],
    )
}

fn positional_artifacts(first: &Artifact, second: &Artifact) -> Vec<u8> {
    let first_bytes = previous_artifact(first);
    let mut bytes = b"CUA3".to_vec();
    bytes.extend_from_slice(&(first_bytes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&first_bytes);
    bytes.extend_from_slice(&previous_artifact(second));
    bytes
}

fn previous_artifact(artifact: &Artifact) -> Vec<u8> {
    match artifact {
        Artifact::Inline(value) => {
            let mut bytes = b"CUA2".to_vec();
            bytes.extend_from_slice(&(value.0.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&value.0);
            bytes.extend_from_slice(&value.1);
            bytes
        }
        Artifact::Stored(value) => {
            encoded(b"CUS1", value, canic_management_protocol::stored::LIMITS)
        }
    }
}

#[test]
fn positional_journals_migrate_to_identified_records_without_reassignment() {
    let previous = [journal("first"), journal("second")];
    let bytes = encoded(
        b"CUJ7",
        &(
            &previous,
            None::<store_maintenance::Status>,
            Vec::<store_maintenance::Status>::new(),
        ),
        JOURNAL_LIMITS,
    );
    let loaded = Journals::from_bytes(Cow::Owned(bytes)).unwrap();
    let bytes = loaded.to_bytes().unwrap();
    assert!(bytes.starts_with(b"CUJ9"));
    let reopened = Journals::from_bytes(bytes).unwrap();
    for slot in ChildId::ALL {
        assert_eq!(reopened.0[&slot].0, previous[slot.index()].0);
    }
    let mut incomplete = Journals::default();
    incomplete.0.remove(&ChildId::SECOND);
    assert!(incomplete.to_bytes().is_err());
}

#[test]
fn identified_journal_refuses_missing_duplicate_reordered_and_unknown_ids() {
    for ids in [
        vec![],
        vec![0_u8],
        vec![0, 0],
        vec![1, 0],
        vec![0, 2],
        vec![0, 1, 2],
    ] {
        let entries: Vec<_> = ids.into_iter().map(|id| (id, Journal::default())).collect();
        let bytes = encoded(
            b"CUJ8",
            &(
                entries,
                None::<store_maintenance::Status>,
                Vec::<store_maintenance::Status>::new(),
            ),
            JOURNAL_LIMITS,
        );
        assert!(Journals::from_bytes(Cow::Owned(bytes)).is_err());
    }
    let journal = Journal(
        Default::default(),
        Default::default(),
        vec![upgrade::Status::default(); upgrade::MAX_UPGRADES],
    );
    for tag in [b"CUJ7", b"CUJ8"] {
        let bytes = if tag == b"CUJ7" {
            encoded(
                tag,
                &(
                    [&journal, &journal],
                    None::<store_maintenance::Status>,
                    Vec::<store_maintenance::Status>::new(),
                ),
                JOURNAL_LIMITS,
            )
        } else {
            encoded(
                tag,
                &(
                    vec![(0_u8, &journal), (1, &journal)],
                    None::<store_maintenance::Status>,
                    Vec::<store_maintenance::Status>::new(),
                ),
                JOURNAL_LIMITS,
            )
        };
        assert!(Journals::from_bytes(Cow::Owned(bytes)).is_err());
    }
}

#[test]
fn artifact_collection_preserves_legacy_bytes_and_the_full_inline_budget() {
    let first = Artifact::Inline(Inline(vec![255; MAX_WASM], vec![]));
    let second = Artifact::Inline(Inline(
        vec![],
        vec![254; canic_management_protocol::children::MAX_ARTIFACT_BYTES - MAX_WASM],
    ));
    let old = positional_artifacts(&first, &second);
    let loaded = Artifacts::from_bytes(Cow::Owned(old)).unwrap();
    let bytes = loaded.to_bytes().unwrap();
    assert!(bytes.starts_with(b"CUA4"));
    assert!(bytes.len() <= ARTIFACT_BYTES);
    let mut reopened = Artifacts::from_bytes(bytes).unwrap();
    assert_eq!(reopened.0[&ChildId::FIRST].wasm(false), first.wasm(false));
    assert_eq!(reopened.0[&ChildId::SECOND].wasm(true), second.wasm(true));
    assert_eq!(reopened.0[&ChildId::FIRST].hash(false), first.hash(false));
    assert!(reopened.0[&ChildId::SECOND].sources().is_none());
    let Artifact::Inline(value) = reopened.0.get_mut(&ChildId::SECOND).unwrap() else {
        panic!()
    };
    value.1.push(254);
    assert!(!reopened.fits());
    assert!(reopened.to_bytes().is_err());
    reopened.0.remove(&ChildId::SECOND);
    assert!(reopened.to_bytes().is_err());
    assert!(Artifacts::default().fits());
}

#[test]
fn artifact_collection_refuses_partial_ambiguous_and_truncated_records() {
    let payload = Artifact::default();
    for ids in [
        vec![],
        vec![0_u8],
        vec![0, 0],
        vec![1, 0],
        vec![0, 2],
        vec![0, 1, 2],
    ] {
        let entries: Vec<_> = ids.into_iter().map(|id| (id, &payload)).collect();
        let bytes = encoded(b"CUA4", &entries, ARTIFACT_LIMITS);
        assert!(Artifacts::from_bytes(Cow::Owned(bytes)).is_err());
    }
    let original = Artifacts::default().to_bytes().unwrap().into_owned();
    // Empty is the valid legacy empty artifact; test identified-format truncation only.
    for length in 4..original.len() {
        assert!(Artifacts::from_bytes(Cow::Borrowed(&original[..length])).is_err());
    }
    let mut trailing = original;
    trailing.push(0);
    assert!(Artifacts::from_bytes(Cow::Owned(trailing)).is_err());
}

// Frozen opaque CUA3 input is written through the same governed Cell contract.
// It grants no production raw-backing path and cannot encode newer records.
struct PreviousArtifacts(Vec<u8>);
impl Storable for PreviousArtifacts {
    const BOUND: Bound = Bound::Bounded {
        max_size: (2 * MAX_WASM + 24) as u32,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        Ok(Cow::Borrowed(&self.0))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        Ok(Self(bytes.into_owned()))
    }
}

#[test]
fn full_mixed_predecessor_migrates_without_growing_either_checked_allocation() {
    use canic_management_protocol::stored;
    let sources = stored::Sources {
        creation: stored::Delivery {
            reference: stored::Reference {
                store: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
                manifest: ic_wasm_store_protocol::Manifest {
                    release: ic_wasm_store_protocol::Release {
                        name: "worker".into(),
                        version: "1".into(),
                    },
                    hash: [7; 32],
                    bytes: 1,
                    chunks: vec![[7; 32]],
                },
            },
            operation: [9; 32],
            state: stored::PinState::Retained,
            cache_deadline: Some(123),
        },
        upgrades: vec![],
    };
    let second = Artifact::Stored(sources.clone());
    let original_ceiling = 2 * MAX_WASM - ic_storage::STABLE_CELL_HEADER_SIZE;
    let inline_bytes = original_ceiling - 8 - 8 - previous_artifact(&second).len();
    assert!(inline_bytes <= canic_management_protocol::children::MAX_ARTIFACT_BYTES);
    let first = Artifact::Inline(Inline(
        vec![255; MAX_WASM],
        vec![254; inline_bytes - MAX_WASM],
    ));
    let old = positional_artifacts(&first, &second);
    assert_eq!(old.len(), original_ceiling);

    let mut runtime = MemoryRuntime::new_with_config(
        VectorMemory::default(),
        StorageConfig {
            allocations: 2,
            extents: 3,
            pages: 64,
        },
    )
    .unwrap();
    runtime
        .bootstrap(
            &SealedDeclarationSnapshot::new(vec![
                MemoryRequest::new("manager", "manager.journal.v1").unwrap(),
                MemoryRequest::new("manager", "manager.artifact.v1").unwrap(),
            ])
            .unwrap(),
            &AllocationGrants::new(vec![MemoryAuthority::new("manager", "manager.").unwrap()])
                .unwrap(),
            &GenericAllocationPolicy,
        )
        .unwrap();
    let journal = StableCell::create(
        runtime
            .open_memory(&StableKey::parse("manager.journal.v1").unwrap())
            .unwrap(),
        Journals::default(),
    )
    .unwrap();
    let key = StableKey::parse("manager.artifact.v1").unwrap();
    let previous =
        StableCell::create(runtime.open_memory(&key).unwrap(), PreviousArtifacts(old)).unwrap();
    let before = runtime.diagnostics().unwrap();
    drop(previous);
    let mut migrated: StableCell<Artifacts> =
        StableCell::load(runtime.open_memory(&key).unwrap(), ARTIFACT_BYTES).unwrap();
    assert!(migrated.get().to_bytes().unwrap().len() < original_ceiling);
    migrated.set(migrated.get().clone());
    assert_eq!(runtime.diagnostics().unwrap(), before);
    drop(migrated);
    let reopened: StableCell<Artifacts> =
        StableCell::load(runtime.open_memory(&key).unwrap(), ARTIFACT_BYTES).unwrap();
    assert_eq!(
        reopened.get().0[&ChildId::FIRST].wasm(false),
        first.wasm(false)
    );
    assert_eq!(
        reopened.get().0[&ChildId::FIRST].wasm(true),
        first.wasm(true)
    );
    assert_eq!(reopened.get().0[&ChildId::SECOND].sources(), Some(&sources));
    let overflow = original_ceiling - reopened.get().to_bytes().unwrap().len() + 1;
    let mut too_large = reopened.get().clone();
    let Artifact::Inline(value) = too_large.0.get_mut(&ChildId::FIRST).unwrap() else {
        panic!()
    };
    value.1.extend(vec![254; overflow]);
    assert!(!too_large.fits(), "refuse before a Cell growth attempt");
    assert!(too_large.to_bytes().is_err());
    assert_eq!(runtime.diagnostics().unwrap(), before);
    assert!(
        journal
            .get()
            .0
            .values()
            .all(|entry| entry.0.approval.is_none())
    );
}

#[test]
fn scan_contract_requires_its_journal_version_and_survives_reopen() {
    use store_maintenance::{Approval, Phase, Scan};
    let record = store_maintenance::Status {
        approval: Approval {
            store: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
            previous_module: [1; 32],
            module_hash: [2; 32],
            attempt: ic_wasm_store_protocol::MaintenanceAttempt {
                sequence: 1,
                operation: [3; 32],
            },
            expires_at_ns: 100,
        },
        operator: "r7inp-6aaaa-aaaaa-aaabq-cai".into(),
        phase: Phase::Reserved,
        stop_request: None,
        failure: None,
        preservation: Scan::Entries(None),
        installation: None,
    };
    let mut journals = Journals(Journals::default().0, Some(record), vec![]);
    let bytes = journals.to_bytes().unwrap().into_owned();
    let loaded = Journals::from_bytes(Cow::Borrowed(&bytes)).unwrap();
    assert_eq!(loaded.1, journals.1);
    let mut mislabeled = bytes;
    mislabeled[..4].copy_from_slice(b"CUJ8");
    assert!(Journals::from_bytes(Cow::Owned(mislabeled)).is_err());
    journals.1.as_mut().unwrap().preservation = Scan::Legacy(None);
    let mut historical = journals.to_bytes().unwrap().into_owned();
    historical[..4].copy_from_slice(b"CUJ8");
    let loaded = Journals::from_bytes(Cow::Owned(historical)).unwrap();
    assert_eq!(loaded.1, journals.1);
    assert!(loaded.to_bytes().unwrap().starts_with(b"CUJ9"));
}
