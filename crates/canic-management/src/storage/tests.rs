//! Exercise the actual persistent codecs, including their untrusted record boundaries.
use super::*;

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

#[test]
fn only_the_current_journal_format_is_admitted() {
    let current = Journals::default().to_bytes().unwrap();
    for tag in [
        b"CUJ2", b"CUJ3", b"CUJ4", b"CUJ5", b"CUJ6", b"CUJ7", b"CUJ8", b"CUJ9", b"CUJA", b"CUJB",
        b"CUJC", b"CUJD", b"CUJE", b"CUJF", b"CUJG",
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
fn refuses_all_previous_artifact_tags() {
    let bytes = Artifacts::default().to_bytes().unwrap();
    for tag in [
        b"CUA2", b"CUA3", b"CUA4", b"CUA5", b"CUA6", b"CUA7", b"CUS1",
    ] {
        let mut old = bytes.clone();
        old[..4].copy_from_slice(tag);
        assert!(Artifacts::from_bytes(&old).is_err());
    }
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
        let bytes = encoded(ARTIFACT_TAG, &entries, ARTIFACT_LIMITS);
        assert!(Artifacts::from_bytes(&bytes).is_err());
    }
    let original = Artifacts::default().to_bytes().unwrap();
    for length in 0..original.len() {
        assert!(Artifacts::from_bytes(&original[..length]).is_err());
    }
    let mut trailing = original;
    trailing.push(0);
    assert!(Artifacts::from_bytes(&trailing).is_err());
}

#[test]
fn application_custody_reopens_with_its_exact_arguments() {
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
    assert_application_artifact_codec(&sources);
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
    let bytes = artifacts.to_bytes().unwrap();
    assert!(bytes.starts_with(ARTIFACT_TAG));
    assert_eq!(
        Artifacts::from_bytes(&bytes).unwrap().to_bytes().unwrap(),
        bytes
    );
    let mut old_tag = bytes;
    old_tag[..4].copy_from_slice(b"CUA4");
    assert!(Artifacts::from_bytes(&old_tag).is_err());
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
        let bytes = journals.to_bytes().unwrap();
        assert!(bytes.starts_with(JOURNAL_TAG));
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
    let mut bytes = journals.to_bytes().unwrap();
    assert!(bytes.starts_with(JOURNAL_TAG));
    assert_eq!(
        Journals::from_bytes(&bytes).unwrap().0.len(),
        ChildId::HISTORICAL_SERVICES.len() + 2
    );
    bytes[..4].copy_from_slice(b"CUJF");
    assert!(Journals::from_bytes(&bytes).is_err());

    let mut bytes = artifacts.to_bytes().unwrap();
    assert!(bytes.starts_with(ARTIFACT_TAG));
    assert_eq!(
        Artifacts::from_bytes(&bytes).unwrap().0.len(),
        ChildId::HISTORICAL_SERVICES.len() + 2
    );
    bytes[..4].copy_from_slice(b"CUA5");
    assert!(Artifacts::from_bytes(&bytes).is_err());

    for ids in [[0_u8, 1, 3, 2], [0, 1, 2, 2], [0, 1, 2, 4]] {
        let entries = ids.map(|id| (id, Artifact::default())).to_vec();
        let bytes = encoded(ARTIFACT_TAG, &entries, ARTIFACT_LIMITS);
        assert!(Artifacts::from_bytes(&bytes).is_err());
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
    assert!(encoded_journals.starts_with(JOURNAL_TAG));
    assert!(encoded_artifacts.starts_with(ARTIFACT_TAG));
    assert_eq!(
        Journals::from_bytes(&encoded_journals)
            .unwrap()
            .0
            .keys()
            .collect::<Vec<_>>(),
        journals.0.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        Artifacts::from_bytes(&encoded_artifacts)
            .unwrap()
            .0
            .keys()
            .collect::<Vec<_>>(),
        artifacts.0.keys().collect::<Vec<_>>()
    );
    for ids in [[0_u8, 1, 4, 4], [0, 1, 5, 4], [0, 1, 4, 16]] {
        let entries = ids.map(|id| (id, Artifact::default())).to_vec();
        let bytes = encoded(ARTIFACT_TAG, &entries, ARTIFACT_LIMITS);
        assert!(Artifacts::from_bytes(&bytes).is_err());
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
    let reopened = Artifacts::from_bytes(&artifacts.to_bytes().unwrap()).unwrap();
    assert!(loaded.0.keys().eq(reopened.0.keys()));
    for ordinal in ChildId::HISTORICAL_SERVICES.len()..=256 {
        let slot = ChildId::service(ordinal).unwrap();
        assert_eq!(
            loaded.0[&slot].0.worker.as_ref().unwrap(),
            &format!("service-{ordinal}")
        );
    }
}
