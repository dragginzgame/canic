//! Fresh row layout, retained record validation and exact child custody.
use super::*;

#[test]
fn row_update_reopens_under_its_original_key() {
    check_journal_rows(257);
}

#[test]
#[ignore = "fleet-scale storage qualification"]
fn eighty_thousand_child_journals_reopen() {
    check_journal_rows(80_001);
}

fn check_journal_rows(count: usize) {
    let memory = ic_storage::DefaultMemoryImpl::default();
    let open = |fresh| canic::storage::bootstrap(memory.clone(), fresh, "manager", KEYS, 4096);
    let runtime = open(true);
    let mut journals = ChildRows::<Journal>::open(&runtime, JOURNAL_KEY, true);
    for ordinal in 0..count {
        let slot = ChildId::service(ordinal).unwrap();
        let mut journal = Journal::default();
        journal.0.worker = Some(format!("child-{ordinal}"));
        journals.set(slot, journal).unwrap();
    }
    let original = journals
        .stable
        .get(&(ChildId::FIRST.index() as u32, 0))
        .unwrap();
    let mut replacement = Journal::default();
    replacement.0.worker = Some("changed".into());
    journals.set(ChildId::SECOND, replacement).unwrap();
    assert_eq!(
        journals
            .stable
            .get(&(ChildId::FIRST.index() as u32, 0))
            .unwrap(),
        original
    );
    drop(journals);
    drop(runtime);
    let reopened = ChildRows::<Journal>::open(&open(false), JOURNAL_KEY, false);
    assert_eq!(reopened.rows.len(), count);
    assert_eq!(
        reopened.rows[&ChildId::SECOND].0.worker.as_deref(),
        Some("changed")
    );
    let last = ChildId::service(count - 1).unwrap();
    assert_eq!(
        reopened.rows[&last].0.worker.as_deref(),
        Some(format!("child-{}", count - 1).as_str())
    );
}

#[test]
fn rows_reject_old_tags_and_truncation() {
    let mut bytes = Metadata::default().encode().unwrap();
    bytes[..4].copy_from_slice(b"CUJG");
    assert_eq!(
        Metadata::decode(&bytes).err(),
        Some("unsupported management record format")
    );
    let mut bytes = Journal::default().encode().unwrap();
    for end in 0..bytes.len() {
        assert!(Journal::decode(&bytes[..end]).is_err());
    }
    bytes[..4].copy_from_slice(b"CJID");
    assert!(Journal::decode(&bytes).is_err());
    let mut bytes = Artifact::Empty.encode().unwrap();
    bytes[..4].copy_from_slice(b"CAID");
    assert!(Artifact::decode(&bytes).is_err());
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
    let mut artifact = Artifact::Application(
        sources.clone(),
        application::Lifecycle {
            creation: application::Arguments::new(vec![255; application::MAX_ARGUMENT_BYTES])
                .unwrap(),
            upgrades: vec![],
        },
    );
    let bytes = artifact.encode().unwrap();
    assert!(bytes.starts_with(Artifact::TAG));
    assert_eq!(Artifact::decode(&bytes).unwrap().encode().unwrap(), bytes);
    let mut old_tag = bytes;
    old_tag[..4].copy_from_slice(b"CUA4");
    assert!(Artifact::decode(&old_tag).is_err());
    let Artifact::Application(_, lifecycle) = &mut artifact else {
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
        artifact.encode().is_err(),
        "arguments cannot outlive their effect/source history"
    );
}
