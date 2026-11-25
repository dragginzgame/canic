//! Exercise the actual persistent codecs, including their untrusted record boundaries.
use super::*;
use ic_storage::VectorMemory;

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
        Artifact::Application(..) => panic!("historical format has no application arguments"),
    }
}

#[test]
fn only_the_current_journal_format_is_admitted() {
    let current = Journals::default().to_bytes().unwrap().into_owned();
    for tag in [
        b"CUJ2", b"CUJ3", b"CUJ4", b"CUJ5", b"CUJ6", b"CUJ7", b"CUJ8", b"CUJ9", b"CUJA", b"CUJB",
        b"CUJC", b"CUJD", b"CUJE", b"CUJF",
    ] {
        let mut bytes = current.clone();
        bytes[..4].copy_from_slice(tag);
        assert_eq!(
            Journals::from_bytes(&bytes).err(),
            Some("unsupported Root journal format")
        );
    }
    let untagged = encoded(b"", &Status::default(), LIMITS);
    assert_eq!(
        Journals::from_bytes(&untagged).err(),
        Some("unsupported Root journal format")
    );
    for length in 0..current.len() {
        assert!(Journals::from_bytes(&current[..length]).is_err());
    }
}

#[test]
fn identified_journal_refuses_missing_duplicate_reordered_and_unknown_ids() {
    for ids in [
        vec![],
        vec![0_u32],
        vec![0, 0],
        vec![1, 0],
        vec![0, 2],
        vec![0, 1, 2],
        vec![0, 1, 16],
    ] {
        let entries: Vec<_> = ids.into_iter().map(|id| (id, Journal::default())).collect();
        let bytes = encoded(
            JOURNAL_TAG,
            &(
                entries,
                None::<store_maintenance::Status>,
                Vec::<store_maintenance::Status>::new(),
                None::<canic_management_protocol::stored::RootBootstrap>,
            ),
            JOURNAL_LIMITS,
        );
        assert!(Journals::from_bytes(&bytes).is_err());
    }
    let journal = Journal(
        Default::default(),
        Default::default(),
        vec![upgrade::Status::default(); upgrade::MAX_UPGRADES],
    );
    let bytes = encoded(
        JOURNAL_TAG,
        &(
            vec![(0_u32, &journal), (1, &journal)],
            None::<store_maintenance::Status>,
            Vec::<store_maintenance::Status>::new(),
            None::<canic_management_protocol::stored::RootBootstrap>,
        ),
        JOURNAL_LIMITS,
    );
    assert!(Journals::from_bytes(&bytes).is_err());
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
        vec![0_u32],
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

#[test]
fn logical_predecessor_payload_reopens_without_growing_records() {
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
    assert_application_artifact_codec(&sources);
    let original_ceiling = 2 * MAX_WASM - 24;
    let inline_bytes = original_ceiling - 8 - 8 - previous_artifact(&second).len();
    assert!(inline_bytes <= canic_management_protocol::children::MAX_ARTIFACT_BYTES);
    let first = Artifact::Inline(Inline(
        vec![255; MAX_WASM],
        vec![254; inline_bytes - MAX_WASM],
    ));
    let old = positional_artifacts(&first, &second);
    assert_eq!(old.len(), original_ceiling);

    let backing = VectorMemory::default();
    let runtime = canic::storage::bootstrap(backing.clone(), true, "manager", KEYS, 512);
    let mut cell = canic::storage::cell(&runtime, ARTIFACT_KEY, ARTIFACT_BYTES, Some(old));
    let migrated = Artifacts::from_bytes(cell.get().as_slice().into()).unwrap();
    assert!(migrated.to_bytes().unwrap().len() < original_ceiling);
    cell.set(migrated.to_bytes().unwrap().into_owned());
    drop(cell);
    drop(runtime);
    let runtime = canic::storage::bootstrap(backing, false, "manager", KEYS, 512);
    let cell = canic::storage::cell(&runtime, ARTIFACT_KEY, ARTIFACT_BYTES, None);
    let reopened = Artifacts::from_bytes(cell.get().as_slice().into()).unwrap();
    assert_eq!(reopened.0[&ChildId::FIRST].wasm(false), first.wasm(false));
    assert_eq!(reopened.0[&ChildId::FIRST].wasm(true), first.wasm(true));
    assert_eq!(reopened.0[&ChildId::SECOND].sources(), Some(&sources));
    let overflow = original_ceiling - reopened.to_bytes().unwrap().len() + 1;
    let mut too_large = reopened;
    let Artifact::Inline(value) = too_large.0.get_mut(&ChildId::FIRST).unwrap() else {
        panic!()
    };
    value.1.extend(vec![254; overflow]);
    assert!(!too_large.fits(), "refuse before a persistent write");
    assert!(too_large.to_bytes().is_err());
}

fn assert_application_artifact_codec(sources: &canic_management_protocol::stored::Sources) {
    use canic_management_protocol::application;
    let mut artifacts = Artifacts::default();
    artifacts.0.insert(
        ChildId::FIRST,
        Artifact::Application(
            sources.clone(),
            application::Lifecycle {
                creation: application::Arguments::new(vec![255; application::MAX_ARGUMENT_BYTES])
                    .unwrap(),
                upgrades: vec![],
            },
        ),
    );
    let bytes = artifacts.to_bytes().unwrap().into_owned();
    assert!(bytes.starts_with(b"CUA7"));
    assert_eq!(
        Artifacts::from_bytes(Cow::Borrowed(&bytes))
            .unwrap()
            .to_bytes()
            .unwrap(),
        bytes
    );
    let mut old_tag = bytes;
    old_tag[..4].copy_from_slice(b"CUA4");
    assert!(Artifacts::from_bytes(Cow::Owned(old_tag)).is_err());
    let Artifact::Application(_, lifecycle) = artifacts.0.get_mut(&ChildId::FIRST).unwrap() else {
        unreachable!()
    };
    lifecycle.upgrades.push(application::Maintenance {
        operator: "2vxsx-fae".into(),
        arguments: application::Arguments::new(vec![1]).unwrap(),
        attempt: 1,
        phase: application::MaintenancePhase::Scheduled,
        requests: [None; 4],
        before: None,
        after: None,
        activator: None,
    });
    assert!(
        !artifacts.fits(),
        "arguments cannot outlive their effect/source history"
    );
}

#[test]
fn growth_preserves_child_and_store_evidence_in_one_format() {
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
        phase: Phase::Installed,
        stop_request: Some([4; 32]),
        failure: None,
        preservation: Scan::Entries(None),
        installation: Some(store_maintenance::Installation {
            upload_request: [5; 32],
            request: Some([6; 32]),
            sender_version: Some(17),
            start_request: None,
            verification: Scan::Entries(None),
            activation_request: None,
        }),
    };
    let mut journals = Journals(Journals::default().0, Some(record.clone()), vec![], None);
    journals.0.insert(ChildId::FIRST, journal("first"));
    for additions in [
        &[][..],
        &ChildId::PROJECT[..],
        &[ChildId::SHARDS[6], ChildId::INSTANCES[7]],
    ] {
        for slot in additions {
            journals.0.insert(*slot, journal("added"));
        }
        let bytes = journals.to_bytes().unwrap().into_owned();
        assert!(bytes.starts_with(b"CUJG"));
        journals = Journals::from_bytes(&bytes).unwrap();
        assert_eq!(
            journals.0[&ChildId::FIRST].0.worker.as_deref(),
            Some("first")
        );
        assert_eq!(journals.1.as_ref(), Some(&record));
        assert_eq!(&*journals.to_bytes().unwrap(), bytes.as_slice());
    }
}

#[test]
fn project_custody_requires_paired_ordered_records() {
    let mut journals = Journals::default();
    let mut artifacts = Artifacts::default();
    for slot in ChildId::PROJECT {
        journals.0.insert(slot, journal("project"));
        artifacts.0.insert(slot, Artifact::default());
    }
    let mut bytes = journals.to_bytes().unwrap().into_owned();
    assert!(bytes.starts_with(b"CUJG"));
    assert_eq!(
        Journals::from_bytes(&bytes).unwrap().0.len(),
        ChildId::HISTORICAL_SERVICES.len() + 2
    );
    bytes[..4].copy_from_slice(b"CUJF");
    assert!(Journals::from_bytes(&bytes).is_err());

    let mut bytes = artifacts.to_bytes().unwrap().into_owned();
    assert!(bytes.starts_with(b"CUA7"));
    assert_eq!(
        Artifacts::from_bytes(bytes.as_slice().into())
            .unwrap()
            .0
            .len(),
        ChildId::HISTORICAL_SERVICES.len() + 2
    );
    bytes[..4].copy_from_slice(b"CUA5");
    assert!(Artifacts::from_bytes(bytes.into()).is_err());

    for ids in [[0_u8, 1, 3, 2], [0, 1, 2, 2], [0, 1, 2, 4]] {
        let entries = ids.map(|id| (id, Artifact::default())).to_vec();
        let bytes = encoded(b"CUA6", &entries, ARTIFACT_LIMITS);
        assert!(Artifacts::from_bytes(bytes.into()).is_err());
    }
    journals.0.remove(&ChildId::PROJECT[0]);
    artifacts.0.remove(&ChildId::PROJECT[1]);
    assert!(journals.to_bytes().is_err());
    assert!(artifacts.to_bytes().is_err());
}

#[test]
fn extended_custody_round_trips_sparse_placement_identities() {
    let mut journals = Journals::default();
    let mut artifacts = Artifacts::default();
    let slot = ChildId::SHARDS[6];
    journals.0.insert(slot, journal("shard"));
    artifacts.0.insert(slot, Artifact::default());
    let encoded_journals = journals.to_bytes().unwrap();
    let encoded_artifacts = artifacts.to_bytes().unwrap();
    assert!(encoded_journals.starts_with(b"CUJG"));
    assert!(encoded_artifacts.starts_with(b"CUA7"));
    assert_eq!(
        Journals::from_bytes(&encoded_journals)
            .unwrap()
            .0
            .keys()
            .collect::<Vec<_>>(),
        journals.0.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        Artifacts::from_bytes(encoded_artifacts)
            .unwrap()
            .0
            .keys()
            .collect::<Vec<_>>(),
        artifacts.0.keys().collect::<Vec<_>>()
    );
    for ids in [[0_u8, 1, 4, 4], [0, 1, 5, 4], [0, 1, 4, 16]] {
        let entries = ids.map(|id| (id, Artifact::default())).to_vec();
        let bytes = encoded(b"CUA7", &entries, ARTIFACT_LIMITS);
        assert!(Artifacts::from_bytes(bytes.into()).is_err());
    }
}

#[test]
fn service_identities_beyond_one_byte_reopen_without_aliasing() {
    let mut journals = Journals::default();
    let mut artifacts = Artifacts::default();
    for ordinal in ChildId::HISTORICAL_SERVICES.len()..=256 {
        let slot = ChildId::service(ordinal).unwrap();
        journals
            .0
            .insert(slot, journal(&format!("service-{ordinal}")));
        artifacts.0.insert(slot, Artifact::default());
    }
    let loaded = Journals::from_bytes(&journals.to_bytes().unwrap()).unwrap();
    let reopened = Artifacts::from_bytes(artifacts.to_bytes().unwrap()).unwrap();
    assert!(loaded.0.keys().eq(reopened.0.keys()));
    for ordinal in ChildId::HISTORICAL_SERVICES.len()..=256 {
        let slot = ChildId::service(ordinal).unwrap();
        assert_eq!(
            loaded.0[&slot].0.worker.as_ref().unwrap(),
            &format!("service-{ordinal}")
        );
    }
}
