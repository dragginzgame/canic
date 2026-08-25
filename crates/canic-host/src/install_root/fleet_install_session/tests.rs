//! Module: install_root::fleet_install_session::tests
//!
//! Responsibility: prove exact retry and conflict behavior for durable Fleet install identity.
//! Does not own: Coordinator or Fleet Subnet Root journal coverage.
//! Boundary: every test uses one finalized release build and filesystem-local recovery state.

use crate::{
    install_root::fleet_component_provisioning_journal::FleetComponentProvisioningTerminalEvidence,
    install_root::fleet_install_session::{
        CloseFleetInstallSessionRequest, FleetInstallSessionError, PlanFleetInstallSessionRequest,
        close_fleet_install_session, plan_fleet_install_session,
        recover_fleet_install_session_authority, session_path,
    },
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    test_support::temp_dir,
};
use std::fs;

use canic_core::ids::CanonicalNetworkId;

const PLAN_DIGEST: &str = "abababababababababababababababababababababababababababababababab";

#[test]
fn exact_retry_recovers_one_immutable_fleet_and_operation_identity() {
    let root = temp_dir("fleet-install-session-retry");
    let finalized = finalized_release(&root, [7; 32]);
    let network = CanonicalNetworkId::ic_mainnet();
    let request = || PlanFleetInstallSessionRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "primary".parse().expect("Fleet name"),
        app: "toko".into(),
        finalized_release_build: &finalized,
        decision_release_build_id: None,
        fresh_fleet_plan_digest: PLAN_DIGEST,
    };

    let first = plan_fleet_install_session(request()).expect("plan session");
    let repeated = plan_fleet_install_session(request()).expect("recover session");
    let recovered_release = recover_fleet_install_session_authority(
        &root,
        network,
        &first.fleet_name,
        &first.fleet.app,
    )
    .expect("recover session release build")
    .expect("existing session release build");

    assert_eq!(repeated, first);
    assert_eq!(
        recovered_release
            .finalized_release_build
            .record
            .release_build_id,
        first.release_build_id
    );
    assert_ne!(first.operation_id, [0; 32]);
    assert_eq!(first.fresh_fleet_plan_digest, PLAN_DIGEST);
    assert_eq!(first.decision_release_build_id, None);
    assert_eq!(
        first.fleet.fleet.canonical_network_id,
        CanonicalNetworkId::ic_mainnet()
    );
    let path = session_path(&root, CanonicalNetworkId::ic_mainnet(), &first.fleet_name);
    let bytes = fs::read(&path).expect("read session");
    assert_eq!(bytes.last(), Some(&b'\n'));
}

#[test]
fn retry_rejects_changed_app_or_release_authority() {
    let root = temp_dir("fleet-install-session-conflict");
    let first_release = finalized_release(&root, [8; 32]);
    let network = CanonicalNetworkId::ic_mainnet();
    let first = PlanFleetInstallSessionRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "primary".parse().expect("Fleet name"),
        app: "toko".into(),
        finalized_release_build: &first_release,
        decision_release_build_id: None,
        fresh_fleet_plan_digest: PLAN_DIGEST,
    };
    plan_fleet_install_session(first).expect("plan session");

    let changed_app = PlanFleetInstallSessionRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "primary".parse().expect("Fleet name"),
        app: "other".into(),
        finalized_release_build: &first_release,
        decision_release_build_id: None,
        fresh_fleet_plan_digest: PLAN_DIGEST,
    };
    assert!(matches!(
        plan_fleet_install_session(changed_app),
        Err(FleetInstallSessionError::ConflictingAuthority { .. })
    ));
    assert!(matches!(
        recover_fleet_install_session_authority(
            &root,
            network,
            &"primary".parse().expect("Fleet name"),
            &"other".into(),
        ),
        Err(FleetInstallSessionError::ConflictingAuthority { .. })
    ));

    let second_release = finalized_release(&root, [9; 32]);
    let changed_release = PlanFleetInstallSessionRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "primary".parse().expect("Fleet name"),
        app: "toko".into(),
        finalized_release_build: &second_release,
        decision_release_build_id: None,
        fresh_fleet_plan_digest: PLAN_DIGEST,
    };
    assert!(matches!(
        plan_fleet_install_session(changed_release),
        Err(FleetInstallSessionError::ConflictingAuthority { .. })
    ));

    let changed_digest = PlanFleetInstallSessionRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "primary".parse().expect("Fleet name"),
        app: "toko".into(),
        finalized_release_build: &first_release,
        decision_release_build_id: None,
        fresh_fleet_plan_digest: "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
    };
    assert!(matches!(
        plan_fleet_install_session(changed_digest),
        Err(FleetInstallSessionError::ConflictingAuthority { .. })
    ));
}

#[test]
fn recovery_retains_original_finalized_decision_source_and_digest() {
    let root = temp_dir("fleet-install-session-plan-authority");
    let finalized = finalized_release(&root, [11; 32]);
    let network = CanonicalNetworkId::ic_mainnet();
    let release_build_id = finalized.record.release_build_id;
    let session = plan_fleet_install_session(PlanFleetInstallSessionRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "primary".parse().expect("Fleet name"),
        app: "toko".into(),
        finalized_release_build: &finalized,
        decision_release_build_id: Some(release_build_id),
        fresh_fleet_plan_digest: PLAN_DIGEST,
    })
    .expect("plan finalized-source session");

    let recovered = recover_fleet_install_session_authority(
        &root,
        network,
        &session.fleet_name,
        &session.fleet.app,
    )
    .expect("recover plan authority")
    .expect("existing session");

    assert_eq!(recovered.decision_release_build_id, Some(release_build_id));
    assert_eq!(recovered.fresh_fleet_plan_digest, PLAN_DIGEST);
    assert_eq!(
        recovered.finalized_release_build.record.release_build_id,
        release_build_id
    );
}

#[test]
fn direct_terminal_one_advance_and_multi_advance_close_fresh_install_recovery() {
    for (path, sequence) in [
        ("fleet-install-session-direct-terminal", 5),
        ("fleet-install-session-one-advance", 7),
        ("fleet-install-session-multi-advance", 9),
        ("fleet-install-session-later-terminal", 13),
    ] {
        assert_terminal_journal_closes_session(path, sequence);
    }
}

fn assert_terminal_journal_closes_session(path: &str, sequence: u64) {
    let root = temp_dir(path);
    let finalized = finalized_release(&root, [12; 32]);
    let network = CanonicalNetworkId::ic_mainnet();
    let request = || PlanFleetInstallSessionRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "primary".parse().expect("Fleet name"),
        app: "toko".into(),
        finalized_release_build: &finalized,
        decision_release_build_id: None,
        fresh_fleet_plan_digest: PLAN_DIGEST,
    };
    let session = plan_fleet_install_session(request()).expect("plan session");
    let entry = crate::fleet_catalog::FleetCatalogEntryV1 {
        canonical_network_id: network,
        fleet_id: session.fleet.fleet.fleet_id,
        fleet_name: session.fleet_name.clone(),
        app: session.fleet.app.clone(),
        environment: "staging".to_string(),
        deployed_at_unix_secs: 1,
        release_build_id: session.release_build_id,
        coordinator_principal: "aaaaa-aa".to_string(),
    };
    let terminal_journal = FleetComponentProvisioningTerminalEvidence {
        schema_version: 1,
        sequence,
        journal_digest: [6; 32],
        fleet_install_plan_digest: [1; 32],
        operation_id: [2; 32],
        plan_hash: [3; 32],
        catalog_entry: entry,
        catalog_hash: [4; 32],
    };
    let close = || CloseFleetInstallSessionRequest {
        root: &root,
        session: &session,
        component_journal: &terminal_journal,
    };

    close_fleet_install_session(close()).expect("close session");
    close_fleet_install_session(close()).expect("idempotently close session");
    assert!(matches!(
        close_fleet_install_session(CloseFleetInstallSessionRequest {
            component_journal: &FleetComponentProvisioningTerminalEvidence {
                catalog_hash: [5; 32],
                ..terminal_journal.clone()
            },
            root: &root,
            session: &session,
        }),
        Err(FleetInstallSessionError::ConflictingAuthority { .. })
    ));
    assert!(matches!(
        recover_fleet_install_session_authority(
            &root,
            network,
            &session.fleet_name,
            &session.fleet.app,
        ),
        Err(FleetInstallSessionError::Completed { .. })
    ));
    assert!(matches!(
        plan_fleet_install_session(request()),
        Err(FleetInstallSessionError::Completed { .. })
    ));
}

#[cfg(unix)]
#[test]
fn session_loader_rejects_noncanonical_and_symlinked_files() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("fleet-install-session-files");
    let finalized = finalized_release(&root, [10; 32]);
    let network = CanonicalNetworkId::ic_mainnet();
    let request = || PlanFleetInstallSessionRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "primary".parse().expect("Fleet name"),
        app: "toko".into(),
        finalized_release_build: &finalized,
        decision_release_build_id: None,
        fresh_fleet_plan_digest: PLAN_DIGEST,
    };
    let session = plan_fleet_install_session(request()).expect("plan session");
    let path = session_path(&root, network, &session.fleet_name);
    let canonical = fs::read(&path).expect("read canonical session");

    fs::write(&path, serde_json::to_vec(&session).expect("encode JSON"))
        .expect("write noncanonical session");
    assert!(matches!(
        plan_fleet_install_session(request()),
        Err(FleetInstallSessionError::InvalidDocument { .. })
    ));

    fs::write(&path, &canonical).expect("restore session");
    let real = path.with_file_name("real-session.json");
    fs::rename(&path, &real).expect("move session");
    symlink(&real, &path).expect("symlink session");
    assert!(matches!(
        plan_fleet_install_session(request()),
        Err(FleetInstallSessionError::UnsafeFile { .. })
    ));
}

fn finalized_release(
    root: &std::path::Path,
    manifest_digest: [u8; 32],
) -> crate::release_build::FinalizedReleaseBuild {
    let planned = plan_release_build(root).expect("plan release");
    let manifest_path = root.join(format!(
        "release-set-{}.json",
        planned.record.release_build_id
    ));
    fs::write(&manifest_path, manifest_digest).expect("write release-set authority");
    finalize_release_build_from_manifest(root, planned.record.release_build_id, &manifest_path)
        .expect("finalize release")
}
