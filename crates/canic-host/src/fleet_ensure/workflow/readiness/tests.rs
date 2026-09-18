use super::*;
use crate::{
    fleet_ensure::ops::{write_journal, write_plan},
    test_support::temp_dir,
};
use std::fs;

fn request(root: &Path) -> FleetReadinessRequest<'_> {
    FleetReadinessRequest {
        workspace: root,
        environment: "local",
        fleet: "fleet",
        icp_executable: "must-not-run",
        operator: Principal::management_canister(),
        cycles_ledger: Principal::management_canister(),
        estimated_required_cycles: None,
    }
}

#[test]
fn unknown_funding_is_not_claimed_sufficient_and_estimates_never_grant_authority() {
    let root = temp_dir("readiness-estimate");
    let mut request = request(&root);
    let unknown = report(&request, "network".into(), 10, None);
    assert!(unknown.estimated_shortfall_cycles.is_none());
    request.estimated_required_cycles = Some(11);
    let estimate = report(&request, "network".into(), 10, None);
    assert_eq!(estimate.estimated_shortfall_cycles, Some(1));
    assert_eq!(
        estimate.blockers,
        vec![ReadinessBlocker::EstimatedFundingShortfall]
    );
    assert!(!root.exists());
}

#[test]
fn retained_work_is_reported_without_rewriting_evidence_or_opening_a_lock() {
    let root = temp_dir("readiness-retained");
    let request = request(&root);
    let paths = EnsurePaths::under(&root, "local", "fleet");
    let plan = super::super::tests::estate_funding_plan();
    let (_, mut journal) = super::super::tests::retained_evidence();
    journal.plan_sha256.clone_from(&plan.plan_sha256);
    journal.operation_id.clone_from(&plan.operation_id);
    for completion in [
        FleetEnsureCompletion::InProgress,
        FleetEnsureCompletion::Prepared,
        FleetEnsureCompletion::ReplanRequired,
        FleetEnsureCompletion::Converged,
    ] {
        journal.completion = completion;
        write_plan(&paths, &plan).unwrap();
        write_journal(&paths, &journal).unwrap();
        let before = fs::read(&paths.journal).unwrap();
        let operation = retained(&paths, &request).unwrap();
        let report = report(&request, "network".into(), 1000, operation);
        assert_eq!(
            report
                .blockers
                .contains(&ReadinessBlocker::RetainedOperation),
            completion != FleetEnsureCompletion::Converged
        );
        assert_eq!(fs::read(&paths.journal).unwrap(), before);
        assert!(!paths.lock.exists());
    }
    fs::write(&paths.journal, b"incomplete").unwrap();
    assert!(matches!(
        retained(&paths, &request),
        Err(FleetReadinessError::State(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unsafe_paths_and_unreadable_retained_evidence_fail_before_signer_or_network_access() {
    let root = temp_dir("readiness-invalid");
    let mut request = request(&root);
    request.fleet = "../outside";
    assert!(matches!(
        inspect(&request),
        Err(FleetReadinessError::Policy(_))
    ));
    assert!(!root.exists());
    request.fleet = "fleet";
    let paths = EnsurePaths::under(&root, "local", "fleet");
    fs::create_dir_all(paths.journal.parent().unwrap()).unwrap();
    fs::write(&paths.journal, b"incomplete").unwrap();
    assert!(matches!(
        inspect(&request),
        Err(FleetReadinessError::State(_))
    ));
    assert_eq!(fs::read(&paths.journal).unwrap(), b"incomplete");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn readiness_requires_exact_signer_and_network_without_anonymous_authority() {
    let mainnet = CanonicalNetworkId::ic_mainnet();
    let local = "00".repeat(32).parse::<CanonicalNetworkId>().unwrap();
    let signer = Principal::self_authenticating(b"operator");
    verify_authority(signer, signer, &mainnet, &mainnet).unwrap();
    for (expected, actual, network) in [
        (signer, Principal::anonymous(), &mainnet),
        (Principal::anonymous(), Principal::anonymous(), &mainnet),
        (signer, signer, &local),
    ] {
        assert!(matches!(
            verify_authority(expected, actual, &mainnet, network),
            Err(FleetReadinessError::AuthorityMismatch)
        ));
    }
}
