use super::*;
use crate::{
    release_build::{
        ReleaseBuildPlanRecord, finalize_release_build_from_manifest, plan_release_build,
    },
    test_support::temp_dir,
};
use std::fs;

fn finalized_release(root: &Path, contents: &[u8]) -> FinalizedReleaseBuild {
    let plan = plan_release_build(root).expect("plan release build");
    let manifest = root.join("release-set.json");
    fs::write(&manifest, contents).expect("write release-set manifest");
    finalize_release_build_from_manifest(root, plan.record.release_build_id, &manifest)
        .expect("finalize release build")
}

fn request<'a>(
    root: &'a Path,
    finalized_release_build: &'a FinalizedReleaseBuild,
) -> PlanFleetInstallActivationRequest<'a> {
    PlanFleetInstallActivationRequest {
        root,
        canonical_network_id: CanonicalNetworkId::public_ic(),
        fleet_name: "toko-local".parse().expect("Fleet name"),
        app: AppId::from("toko"),
        finalized_release_build,
    }
}

#[test]
fn planned_journal_is_canonical_durable_and_bound_to_every_path_identity() {
    let root = temp_dir("fleet-install-activation-plan");
    let finalized = finalized_release(&root, b"{\"release\":\"exact\"}");
    let planned = plan_fleet_install_activation(request(&root, &finalized))
        .expect("plan Fleet install activation");
    let identity = &planned.journal.activation.identity;
    let expected_path = fleet_install_activation_journal_path(
        &root,
        identity.fleet.fleet.network,
        identity.fleet.fleet.fleet_id,
        identity.operation_id,
    );

    assert_eq!(planned.path, expected_path);
    assert_eq!(planned.journal.sequence, 0);
    assert_eq!(planned.journal.phase, FleetInstallActivationPhase::Planned);
    assert_eq!(planned.journal.fleet_name.as_str(), "toko-local");
    assert_eq!(identity.fleet.app.as_str(), "toko");
    assert_eq!(identity.release_build_id, finalized.record.release_build_id);
    assert_eq!(planned.journal.release_build_plan_hash, finalized.plan_hash);
    assert_eq!(
        planned.journal.release_set_manifest_digest,
        match finalized.record.state {
            ReleaseBuildPlanState::Finalized {
                release_set_manifest_digest,
            } => release_set_manifest_digest,
            ReleaseBuildPlanState::Planned => unreachable!("fixture is finalized"),
        }
    );
    assert_eq!(
        planned.journal_hash,
        fleet_install_activation_journal_hash(&planned.journal)
    );
    assert_eq!(
        load_fleet_install_activation_journal(
            &root,
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        )
        .expect("load journal"),
        planned.journal
    );
    assert_eq!(fs::read(&planned.path).expect("read journal")[0], 0x88);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn planning_requires_unchanged_finalized_release_build_authority() {
    let root = temp_dir("fleet-install-activation-release-authority");
    let finalized = finalized_release(&root, b"manifest");
    let forged = FinalizedReleaseBuild {
        plan_hash: [0xff; 32],
        ..finalized
    };

    std::assert_matches!(
        plan_fleet_install_activation(request(&root, &forged)),
        Err(FleetInstallActivationJournalError::FinalizedReleaseBuildMismatch)
    );

    let other_root = temp_dir("fleet-install-activation-planned-release");
    let planned = plan_release_build(&other_root).expect("plan unfinalized release build");
    let unfinalized = FinalizedReleaseBuild {
        record: ReleaseBuildPlanRecord {
            state: ReleaseBuildPlanState::Planned,
            ..planned.record
        },
        plan_hash: [0; 32],
        path: planned.path,
    };
    std::assert_matches!(
        plan_fleet_install_activation(request(&other_root, &unfinalized)),
        Err(FleetInstallActivationJournalError::ReleaseBuild(
            ReleaseBuildPlanError::InvalidDocument { .. }
        ))
    );

    fs::remove_dir_all(root).expect("remove temp root");
    fs::remove_dir_all(other_root).expect("remove temp root");
}

#[test]
fn duplicate_identity_cannot_replace_the_existing_journal() {
    let root = temp_dir("fleet-install-activation-create-new");
    let finalized = finalized_release(&root, b"manifest");
    let ReleaseBuildPlanState::Finalized {
        release_set_manifest_digest,
    } = finalized.record.state
    else {
        unreachable!("fixture is finalized");
    };
    let network = CanonicalNetworkId::public_ic();
    let fleet_id = FleetId::from_generated_bytes([7; 32]);
    let operation_id = [8; 32];
    let request = PlanFleetInstallActivationRequest {
        root: &root,
        canonical_network_id: network,
        fleet_name: "toko-local".parse().expect("Fleet name"),
        app: AppId::from("toko"),
        finalized_release_build: &finalized,
    };
    let first = plan_fleet_install_activation_with_ids(
        &request,
        &finalized,
        release_set_manifest_digest,
        fleet_id,
        operation_id,
    )
    .expect("first plan");
    let original = fs::read(&first.path).expect("read first journal");

    std::assert_matches!(
        plan_fleet_install_activation_with_ids(
            &PlanFleetInstallActivationRequest {
                fleet_name: "other".parse().expect("Fleet name"),
                app: AppId::from("other"),
                ..request
            },
            &finalized,
            release_set_manifest_digest,
            fleet_id,
            operation_id,
        ),
        Err(FleetInstallActivationJournalError::Io { source, .. })
            if source.kind() == io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read(&first.path).expect("reread journal"), original);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn corrupt_noncanonical_and_path_mismatched_journals_fail_closed() {
    let root = temp_dir("fleet-install-activation-reject");
    let finalized = finalized_release(&root, b"manifest");
    let planned = plan_fleet_install_activation(request(&root, &finalized))
        .expect("plan Fleet install activation");
    let identity = &planned.journal.activation.identity;
    let canonical = fs::read(&planned.path).expect("read journal");

    fs::write(&planned.path, b"not-cbor").expect("corrupt journal");
    std::assert_matches!(
        load_fleet_install_activation_journal(
            &root,
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ),
        Err(FleetInstallActivationJournalError::InvalidDocument { .. })
    );

    fs::write(&planned.path, &canonical).expect("restore journal");
    let other_network = "11".repeat(32).parse().expect("canonical network");
    let other_fleet = FleetId::from_generated_bytes([0xee; 32]);
    let other_operation = [0xdd; 32];
    for (network, fleet, operation) in [
        (
            other_network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ),
        (
            identity.fleet.fleet.network,
            other_fleet,
            identity.operation_id,
        ),
        (
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            other_operation,
        ),
    ] {
        let other_path = fleet_install_activation_journal_path(&root, network, fleet, operation);
        fs::create_dir_all(other_path.parent().expect("journal parent"))
            .expect("create other parent");
        fs::copy(&planned.path, &other_path).expect("copy journal under wrong identity");
        std::assert_matches!(
            load_fleet_install_activation_journal(&root, network, fleet, operation),
            Err(FleetInstallActivationJournalError::InvalidDocument { .. })
        );
    }

    let mut noncanonical = canonical;
    noncanonical.splice(1..2, [0x98, 0x08]);
    fs::write(&planned.path, noncanonical).expect("write noncanonical journal");
    std::assert_matches!(
        load_fleet_install_activation_journal(
            &root,
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ),
        Err(FleetInstallActivationJournalError::InvalidDocument { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn planned_phase_rejects_post_mutation_evidence() {
    let root = temp_dir("fleet-install-activation-phase");
    let finalized = finalized_release(&root, b"manifest");
    let planned = plan_fleet_install_activation(request(&root, &finalized))
        .expect("plan Fleet install activation");
    let mut invalid = planned.journal;
    invalid.root_install_receipt_hash = Some([1; 32]);

    std::assert_matches!(
        encode_journal(&invalid),
        Err(FleetInstallActivationJournalError::InvalidDocument { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn journal_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("fleet-install-activation-symlink");
    let finalized = finalized_release(&root, b"manifest");
    let planned = plan_fleet_install_activation(request(&root, &finalized))
        .expect("plan Fleet install activation");
    let identity = &planned.journal.activation.identity;
    let real = root.join("real-journal.cbor");
    fs::rename(&planned.path, &real).expect("move journal");
    symlink(&real, &planned.path).expect("link journal");

    std::assert_matches!(
        load_fleet_install_activation_journal(
            &root,
            identity.fleet.fleet.network,
            identity.fleet.fleet.fleet_id,
            identity.operation_id,
        ),
        Err(FleetInstallActivationJournalError::UnsafeFile { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}
