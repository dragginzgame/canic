use super::{LocalFleetError, model::*, ops, policy, workflow::LocalFleetSession};
use candid::Principal;
use std::{fs, net::TcpListener, path::PathBuf};

#[test]
fn binary_fingerprint_preserves_digest_size_admission_and_native_read_errors() {
    let root = crate::test_support::temp_dir("local-binary-fingerprint");
    fs::create_dir_all(&root).unwrap();
    let binary = root.join("binary");
    fs::write(&binary, b"abc").unwrap();
    assert_eq!(
        ops::binary_sha256(&binary).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    std::assert_matches!(ops::binary_sha256(&root), Err(LocalFleetError::UnsafePath));
    std::assert_matches!(
        ops::binary_sha256(&root.join("missing")),
        Err(LocalFleetError::Io(source)) if source.kind() == std::io::ErrorKind::NotFound
    );
    #[cfg(unix)]
    {
        let link = root.join("link");
        std::os::unix::fs::symlink(&binary, &link).unwrap();
        std::assert_matches!(
            ops::binary_sha256(&link),
            Err(LocalFleetError::Io(source))
                if source.raw_os_error() == Some(rustix::io::Errno::LOOP.raw_os_error())
        );
        let fifo = root.join("fifo");
        crate::test_support::create_fifo(&fifo);
        std::assert_matches!(ops::binary_sha256(&fifo), Err(LocalFleetError::UnsafePath));
    }
    fs::File::options()
        .write(true)
        .open(&binary)
        .unwrap()
        .set_len(512 * 1024 * 1024 + 1)
        .unwrap();
    std::assert_matches!(
        ops::binary_sha256(&binary),
        Err(LocalFleetError::UnsafePath)
    );
    fs::remove_dir_all(root).unwrap();
}

fn reset_fixture(label: &str) -> (PathBuf, PathBuf, LocalFleetRecord) {
    let root = crate::test_support::temp_dir(label);
    fs::create_dir_all(&root).unwrap();
    let config = configuration(12345);
    let (directory, lock) = ops::lock_directory(&root, &config.name).unwrap();
    let record = ops::initial_record(&config).unwrap();
    ops::write_record(&directory, &record).unwrap();
    drop(lock);
    (root, directory, record)
}

#[test]
fn reset_discards_over_depth_instance_and_replays_without_rewriting_receipt() {
    let (root, directory, record) = reset_fixture("canic-local-reset-depth");
    let instance = ops::instance_directory(&directory, &record.session_id).unwrap();
    let mut nested = instance.clone();
    for _ in 0..65 {
        nested.push("nested");
    }
    fs::create_dir_all(nested).unwrap();
    std::assert_matches!(
        ops::validate_tree(&directory),
        Err(LocalFleetError::Capacity)
    );
    LocalFleetSession::reset(&root, &record.configuration.name, &record.session_id).unwrap();
    assert!(!instance.exists());
    assert!(ops::read_record(&directory).unwrap().is_none());
    let receipt = fs::read(directory.join("reset.json")).unwrap();
    LocalFleetSession::reset(&root, &record.configuration.name, &record.session_id).unwrap();
    assert_eq!(fs::read(directory.join("reset.json")).unwrap(), receipt);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn reset_discards_interior_symlinks_without_touching_their_targets() {
    let (root, directory, record) = reset_fixture("canic-local-reset-interior-link");
    let instance = ops::instance_directory(&directory, &record.session_id).unwrap();
    fs::create_dir_all(&instance).unwrap();
    let outside = root.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"unrelated").unwrap();
    std::os::unix::fs::symlink(&outside, instance.join("link")).unwrap();
    std::assert_matches!(
        ops::validate_tree(&directory),
        Err(LocalFleetError::UnsafePath)
    );
    LocalFleetSession::reset(&root, &record.configuration.name, &record.session_id).unwrap();
    assert!(!instance.exists());
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"unrelated");
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn reset_rejects_a_symlinked_instance_and_recovers_after_its_removal() {
    let (root, directory, record) = reset_fixture("canic-local-reset-instance-link");
    let instance = ops::instance_directory(&directory, &record.session_id).unwrap();
    let outside = root.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"unrelated").unwrap();
    std::os::unix::fs::symlink(&outside, &instance).unwrap();
    std::assert_matches!(
        LocalFleetSession::reset(&root, &record.configuration.name, &record.session_id),
        Err(LocalFleetError::UnsafePath)
    );
    assert!(ops::read_record(&directory).unwrap().is_some());
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"unrelated");
    fs::remove_file(&instance).unwrap();
    LocalFleetSession::reset(&root, &record.configuration.name, &record.session_id).unwrap();
    assert!(ops::read_record(&directory).unwrap().is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn allocation_intent_preserves_timestamp_and_capacity_before_any_platform_call() {
    let root = crate::test_support::temp_dir("canic-local-allocation-intent");
    fs::create_dir_all(&root).unwrap();
    let config = configuration(12345);
    let (directory, _lock) = ops::lock_directory(&root, &config.name).unwrap();
    let mut record = ops::initial_record(&config).unwrap();
    record.application_subnets = vec![Principal::self_authenticating(b"local-subnet")];
    for index in 0..config.maximum_canisters {
        let input = LocalAllocationInput {
            name: format!("allocation-{index}"),
            role: "pool".into(),
            application_subnet: 0,
            controller: Principal::self_authenticating(b"local-operator"),
        };
        let (next, intent) = ops::reserve(&directory, &record, &input, 100).unwrap();
        let bytes = fs::read(directory.join("environment.json")).unwrap();
        let (replay, repeated) = ops::reserve(&directory, &next, &input, u64::MAX).unwrap();
        assert_eq!(intent, repeated);
        assert_eq!(next, replay);
        assert_eq!(bytes, fs::read(directory.join("environment.json")).unwrap());
        record = next;
    }
    let extra = LocalAllocationInput {
        name: "extra".into(),
        role: "pool".into(),
        application_subnet: 0,
        controller: Principal::self_authenticating(b"local-operator"),
    };
    assert!(matches!(
        ops::reserve(&directory, &record, &extra, 100),
        Err(LocalFleetError::Capacity)
    ));
    let mut changed = record.allocations[0].input.clone();
    changed.role = "root".into();
    assert!(matches!(
        ops::reserve(&directory, &record, &changed, 100),
        Err(LocalFleetError::Identity)
    ));
    assert!(matches!(
        ops::prepare::require_workspace(&directory, &root.join(".canic")),
        Err(LocalFleetError::Identity)
    ));
    fs::remove_dir_all(root).unwrap();
}

fn configuration(port: u16) -> LocalFleetConfig {
    LocalFleetConfig {
        schema_version: 1,
        name: "developer".into(),
        server_binary: PathBuf::from("/nonexistent/pocket-ic"),
        server_binary_sha256: "00".repeat(32),
        gateway_port: port,
        application_subnets: 2,
        maximum_canisters: 8,
        canister_memory_bytes: 64 * 1024 * 1024,
        allocation_debit_cycles: 100_000_000_000_000,
        request_timeout_secs: 30,
        server_lifetime_secs: 300,
    }
}

#[test]
fn bounds_and_exclusive_directory_reject_unsafe_ownership() {
    let root = crate::test_support::temp_dir("canic-local-ownership");
    fs::create_dir_all(&root).unwrap();
    let config = configuration(12345);
    policy::validate(&config).unwrap();
    let mut invalid = config.clone();
    invalid.application_subnets = 1;
    assert!(matches!(
        policy::validate(&invalid),
        Err(LocalFleetError::Configuration)
    ));
    let (_, lock) = ops::lock_directory(&root, &config.name).unwrap();
    assert!(matches!(
        ops::lock_directory(&root, &config.name),
        Err(LocalFleetError::Busy)
    ));
    drop(lock);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/tmp", root.join(".canic/local-fleets/escape")).unwrap();
        assert!(matches!(
            ops::lock_directory(&root, "escape"),
            Err(LocalFleetError::UnsafePath)
        ));
        assert!(matches!(
            ops::validate_tree(&root),
            Err(LocalFleetError::UnsafePath)
        ));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "owned persistent PocketIC process and loopback gateway"]
fn persistent_session_preserves_subnets_allocations_time_and_trust() {
    let root = crate::test_support::temp_dir("canic-local-persistent");
    fs::create_dir_all(&root).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut config = configuration(listener.local_addr().unwrap().port());
    drop(listener);
    config.server_binary = std::env::var_os("POCKET_IC_BIN")
        .expect("exact PocketIC binary")
        .into();
    config.server_binary_sha256 = ops::binary_sha256(&config.server_binary).unwrap();
    let mut session = LocalFleetSession::open(&root, &config).unwrap();
    let first = session.status().unwrap();
    assert!(matches!(
        session.shutdown("wrong-session"),
        Err(LocalFleetError::Session)
    ));
    assert_eq!(session.status().unwrap().session_id, first.session_id);
    assert_eq!(first.application_subnets.len(), 2);
    let inputs = (0..2)
        .map(|index| LocalAllocationInput {
            name: format!("root-{index}"),
            role: "root".into(),
            application_subnet: index,
            controller: Principal::self_authenticating(b"public-local-fleet-fixture"),
        })
        .collect::<Vec<_>>();
    let targets = inputs
        .iter()
        .map(|input| session.allocate(input).unwrap())
        .collect::<Vec<_>>();
    assert_ne!(targets[0].subnet_id, targets[1].subnet_id);
    assert_eq!(session.allocate(&inputs[0]).unwrap(), targets[0]);
    assert!(matches!(
        session.restart("different-session"),
        Err(LocalFleetError::Session)
    ));
    session.advance_time(&first.session_id, 120).unwrap();
    session.restart(&first.session_id).unwrap();
    let restarted = session.status().unwrap();
    assert_eq!(restarted.gateway, first.gateway);
    assert_eq!(restarted.root_key_der_hex, first.root_key_der_hex);
    assert!(
        restarted.simulated_time_ns.parse::<u128>().unwrap()
            >= first.simulated_time_ns.parse::<u128>().unwrap() + 120_000_000_000
    );
    assert!(restarted.canisters.iter().all(|canister| canister.exists));
    session.shutdown(&first.session_id).unwrap();
    // Retain the real Ledger effect but remove the host response, as after a lost reply.
    let directory = root.join(".canic/local-fleets/developer");
    let mut record = ops::read_record(&directory).unwrap().unwrap();
    record.allocations[1].canister_id = None;
    ops::write_record(&directory, &record).unwrap();
    let mut reopened = LocalFleetSession::open(&root, &config).unwrap();
    assert_eq!(reopened.allocate(&inputs[1]).unwrap(), targets[1]);
    assert!(matches!(
        LocalFleetSession::reset(&root, &config.name, &first.session_id),
        Err(LocalFleetError::Busy)
    ));
    reopened.shutdown(&first.session_id).unwrap();
    assert!(matches!(
        LocalFleetSession::reset(&root, &config.name, "different"),
        Err(LocalFleetError::Session)
    ));
    // Interrupt after the durable reset intent, then resume without touching a later session.
    assert!(ops::reset::begin(&directory, &first.session_id).unwrap());
    ops::reset::remove_instance(&directory).unwrap();
    assert!(matches!(
        LocalFleetSession::open(&root, &config),
        Err(LocalFleetError::Session)
    ));
    LocalFleetSession::reset(&root, &config.name, &first.session_id).unwrap();
    let receipt = fs::read(directory.join("reset.json")).unwrap();
    LocalFleetSession::reset(&root, &config.name, &first.session_id).unwrap();
    assert_eq!(fs::read(directory.join("reset.json")).unwrap(), receipt);
    // A late orphan may recreate only its discarded generation's simulator directory.
    let discarded = ops::instance_directory(&directory, &first.session_id).unwrap();
    fs::create_dir_all(&discarded).unwrap();
    fs::write(discarded.join("late-orphan-write"), b"old generation").unwrap();
    let mut replacement = LocalFleetSession::open(&root, &config).unwrap();
    let replacement_status = replacement.status().unwrap();
    assert_ne!(replacement_status.session_id, first.session_id);
    assert!(replacement_status.canisters.is_empty());
    assert_ne!(
        ops::instance_directory(&directory, &replacement_status.session_id).unwrap(),
        discarded
    );
    replacement
        .shutdown(&replacement_status.session_id)
        .unwrap();
    assert!(matches!(
        LocalFleetSession::reset(&root, &config.name, &first.session_id),
        Err(LocalFleetError::Session)
    ));
    assert_eq!(
        fs::read(discarded.join("late-orphan-write")).unwrap(),
        b"old generation"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_startup_is_resettable_and_old_reset_cannot_discard_a_new_session() {
    let root = crate::test_support::temp_dir("canic-local-startup-reset");
    fs::create_dir_all(&root).unwrap();
    let config = configuration(12345);
    assert!(LocalFleetSession::open(&root, &config).is_err());
    let directory = root.join(".canic/local-fleets/developer");
    let record = ops::read_record(&directory).unwrap().unwrap();
    assert_eq!(record.checkpoint, LocalCheckpoint::Running);
    assert!(matches!(
        LocalFleetSession::open(&root, &config),
        Err(LocalFleetError::UncleanCheckpoint)
    ));
    LocalFleetSession::reset(&root, &config.name, &record.session_id).unwrap();
    LocalFleetSession::reset(&root, &config.name, &record.session_id).unwrap();
    assert!(ops::read_record(&directory).unwrap().is_none());
    let mut value = serde_json::to_value(&record).unwrap();
    value.as_object_mut().unwrap().remove("release_build_id");
    assert!(serde_json::from_value::<LocalFleetRecord>(value).is_err());
    fs::remove_dir_all(root).unwrap();
}
