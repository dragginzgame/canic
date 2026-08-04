//! Module: install_root::fleet_install_session::tests
//!
//! Responsibility: prove exact retry and conflict behavior for durable Fleet install identity.
//! Does not own: Coordinator or Fleet Subnet Root journal coverage.
//! Boundary: every test uses one finalized release build and filesystem-local recovery state.

use crate::{
    install_root::fleet_install_session::{
        FleetInstallSessionError, PlanFleetInstallSessionRequest, plan_fleet_install_session,
        recover_fleet_install_session_release_build, session_path,
    },
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    test_support::temp_dir,
};
use std::fs;

use canic_core::ids::CanonicalNetworkId;

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
    };

    let first = plan_fleet_install_session(request()).expect("plan session");
    let repeated = plan_fleet_install_session(request()).expect("recover session");
    let recovered_release = recover_fleet_install_session_release_build(
        &root,
        network,
        &first.fleet_name,
        &first.fleet.app,
    )
    .expect("recover session release build")
    .expect("existing session release build");

    assert_eq!(repeated, first);
    assert_eq!(
        recovered_release.record.release_build_id,
        first.release_build_id
    );
    assert_ne!(first.operation_id, [0; 32]);
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
    };
    plan_fleet_install_session(first).expect("plan session");

    let changed_app = PlanFleetInstallSessionRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "primary".parse().expect("Fleet name"),
        app: "other".into(),
        finalized_release_build: &first_release,
    };
    assert!(matches!(
        plan_fleet_install_session(changed_app),
        Err(FleetInstallSessionError::ConflictingAuthority { .. })
    ));
    assert!(matches!(
        recover_fleet_install_session_release_build(
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
    };
    assert!(matches!(
        plan_fleet_install_session(changed_release),
        Err(FleetInstallSessionError::ConflictingAuthority { .. })
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
