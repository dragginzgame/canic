use super::*;
use crate::{
    fleet_ensure::{
        generate::preflight,
        ops::{clean_reinstall, write_journal, write_plan},
        view::{
            operator_mint::OperatorMintRateQuote,
            readiness::{InfrastructureFundingUnavailable, ReadinessUnresolved},
        },
    },
    test_support::temp_dir,
};
use std::fs;

/// Exercise paired readiness against an actual compiled, persisted unpaid bootstrap review.
pub(in crate::fleet_ensure) fn qualify_unpaid_infrastructure_review(
    plan: &crate::fleet_ensure::model::FleetEnsurePlan,
    source: &Path,
    seed: &Path,
) {
    let root = temp_dir("readiness-unpaid-bootstrap");
    fs::create_dir_all(&root).unwrap();
    let policy = root.join("policy.toml");
    let inventory = root.join("seed.toml");
    fs::copy(source, &policy).unwrap();
    fs::copy(seed, &inventory).unwrap();
    let paths = EnsurePaths::under(&root, &plan.environment, &plan.fleet);
    let desired = plan.reviewed_desired.as_ref().unwrap().desired();
    clean_reinstall::bind(&paths, desired, &policy, &inventory).unwrap();
    write_plan(&paths, plan).unwrap();
    assert!(!paths.journal.exists());
    let before = directory_bytes(&root);
    let request = FleetReadinessRequest {
        environment: &plan.environment,
        fleet: &plan.fleet,
        operator: Principal::from_text(&desired.operator).unwrap(),
        cycles_ledger: Principal::from_text(&desired.cycles_ledger).unwrap(),
        generation_inputs: Some(ReadinessGenerationInputs {
            source: &policy,
            seed: &inventory,
        }),
        ..request(&root)
    };
    crate::fleet_ensure::ops::retained_contract::check(&root, request.environment, request.fleet)
        .unwrap();
    preflight::validate_generation_inputs(
        &preflight::FleetGenerationInputsRequest {
            root: &root,
            environment: request.environment,
            fleet: request.fleet,
            source: &policy,
            seed: &inventory,
        },
        request.operator,
        request.cycles_ledger,
    )
    .unwrap();
    let forecast = reset_forecast(
        &request,
        request.generation_inputs.as_ref().unwrap(),
        &IcpCli::new("must-not-run", None),
    )
    .unwrap();
    assert!(
        forecast
            .as_ref()
            .is_some_and(|forecast| !forecast.targets.is_empty())
    );
    let mut report = report(
        &request,
        "network".into(),
        0,
        retained(&paths, &request).unwrap(),
    );
    record_reset_forecast(&paths, &mut report, forecast).unwrap();
    assert!(report.generation_inputs_checked);
    // An unavailable quote must not prevent building inputs needed for a new review.
    assert!(report.blockers.is_empty());
    assert!(
        report
            .funding
            .clean_reinstall_infrastructure_unavailable
            .is_none()
    );
    assert!(report.estimated_required_cycles.is_none());
    assert!(report.estimated_shortfall_cycles.is_none());
    assert!(report.funding.conversion.is_none());
    assert_eq!(directory_bytes(&root), before);
    crate::fleet_ensure::workflow::clean_reinstall::cancel_review(
        &root,
        request.environment,
        request.fleet,
        &plan.plan_sha256,
    )
    .unwrap();
    assert!(
        crate::fleet_ensure::workflow::clean_reinstall::retained_desired(
            &root,
            request.environment,
            request.fleet,
            true
        )
        .unwrap()
        .is_none()
    );
    let targets = preflight::bootstrap_funding_targets(
        &preflight::FleetGenerationInputsRequest {
            root: &root,
            environment: request.environment,
            fleet: request.fleet,
            source: &policy,
            seed: &inventory,
        },
        request.operator,
        request.cycles_ledger,
    )
    .unwrap();
    assert!(!targets.is_empty());
    fs::remove_dir_all(root).unwrap();
}

fn directory_bytes(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut bytes = std::collections::BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            bytes.extend(directory_bytes(&path));
        } else {
            bytes.insert(path.clone(), fs::read(path).unwrap());
        }
    }
    bytes
}

fn request(root: &Path) -> FleetReadinessRequest<'_> {
    FleetReadinessRequest {
        workspace: root,
        environment: "local",
        fleet: "fleet",
        icp_executable: "must-not-run",
        signing_identity: None,
        operator: Principal::management_canister(),
        cycles_ledger: Principal::management_canister(),
        estimated_required_cycles: None,
        desired: None,
        generation_inputs: None,
        conversion: None,
    }
}

#[test]
fn unknown_funding_is_not_claimed_sufficient_and_estimates_never_grant_authority() {
    let root = temp_dir("readiness-estimate");
    let mut request = request(&root);
    let unknown = report(&request, "network".into(), 10, None);
    assert!(unknown.estimated_shortfall_cycles.is_none());
    assert_eq!(
        unknown.funding.clean_reinstall_infrastructure_unavailable,
        Some(InfrastructureFundingUnavailable::GenerationInputsNotSupplied)
    );
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
fn reset_funding_shortfall_is_explicit_without_duplicate_blockers() {
    use crate::fleet_ensure::view::readiness::InfrastructureFundingReadiness;
    let root = temp_dir("readiness-reset-shortfall");
    let request = request(&root);
    for (funding, blocked) in [(None, false), (Some(10), false), (Some(11), true)] {
        let mut report = report(&request, "network".into(), 10, None);
        for _ in 0..2 {
            record_reset_forecast(
                &EnsurePaths::under(&root, "local", "fleet"),
                &mut report,
                Some(InfrastructureFundingReadiness {
                    targets: Vec::new(),
                    maximum_funding_cycles: funding,
                    maximum_ledger_transfers: 1,
                }),
            )
            .unwrap();
        }
        assert_eq!(
            report
                .blockers
                .contains(&ReadinessBlocker::EstimatedFundingShortfall),
            blocked
        );
        assert_eq!(report.blockers.len(), usize::from(blocked));
        assert!(report.estimated_required_cycles.is_none());
        assert!(
            report
                .funding
                .clean_reinstall_infrastructure_unavailable
                .is_none()
        );
    }
}

#[test]
fn retained_work_is_reported_without_rewriting_evidence_or_opening_a_lock() {
    let mut fixture = crate::fleet_ensure::tests::protocol_tranche_fixture(Vec::new());
    fixture.desired.fleet = "fleet".into();
    let root = fixture.root;
    let request = request(&root);
    let paths = EnsurePaths::under(&root, "local", "fleet");
    let mut plan = super::super::tests::estate_funding_plan();
    plan.operation_id = "ab".repeat(32);
    plan.reviewed_desired = Some(Box::new(
        crate::fleet_ensure::model::ReviewedDesiredFleetRecord::capture(&fixture.desired),
    ));
    plan.plan_sha256 = crate::fleet_ensure::policy::expected_plan_sha256(&plan);
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
fn completed_readiness_ignores_retired_execution_payloads() {
    let root = temp_dir("readiness-completed-contract");
    let paths = EnsurePaths::under(&root, "local", "fleet");
    let operation = "ab".repeat(32);
    let digest = "cd".repeat(32);
    fs::create_dir_all(paths.plan.parent().unwrap()).unwrap();
    fs::write(
        &paths.plan,
        serde_json::to_vec(&serde_json::json!({
            "environment": "local", "fleet": "fleet", "operation_id": operation,
            "plan_sha256": digest, "reviewed_desired": {"release": "0.110.38"}
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &paths.journal,
        serde_json::to_vec(&serde_json::json!({
            "fleet": "fleet", "operation_id": operation, "plan_sha256": digest,
            "completion": "converged", "effects": [{"state": "applied"}],
            "successor_phases": [{"unreadable_as_current_contract": true}]
        }))
        .unwrap(),
    )
    .unwrap();
    let selected = retained(&paths, &request(&root)).unwrap();
    let report = report(&request(&root), "network".into(), 1000, selected);
    assert!(
        !report
            .blockers
            .contains(&ReadinessBlocker::RetainedOperation)
    );
    assert!(!paths.lock.exists());
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
        Err(FleetReadinessError::RetainedContract(
            crate::fleet_ensure::ops::retained_contract::RetainedContractError::State(
                EnsureStateError::Decode { path, .. }
            )
        )) if path == paths.journal
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

#[test]
fn conversion_keeps_mint_amount_fees_and_unknown_requirements_separate() {
    let root = temp_dir("readiness-conversion");
    let mut request = request(&root);
    let selected = ReadinessConversionRequest {
        icp_ledger: request.cycles_ledger,
        cmc: request.cycles_ledger,
    };
    let rate = OperatorMintRateQuote {
        rate_timestamp_seconds: 10,
        xdr_permyriad_per_icp: 100,
        transfer_fee_e8s: 3,
        estimated_deposit_fee_cycles: 20,
    };
    let mut unknown = report(&request, "network".into(), 100, None);
    apply_quote(&mut unknown, selected, Some(rate.clone()), 11_000);
    assert!(
        unknown
            .funding
            .conversion
            .unwrap()
            .estimated_mint_e8s
            .is_none()
    );
    assert!(
        unknown
            .funding
            .unresolved
            .contains(&ReadinessUnresolved::ConversionAmountUnavailable)
    );
    request.estimated_required_cycles = Some(250);
    let mut quote = report(&request, "network".into(), 100, None);
    apply_quote(&mut quote, selected, Some(rate.clone()), 11_000);
    let conversion = quote.funding.conversion.unwrap();
    assert_eq!(conversion.estimated_mint_e8s, Some(2));
    assert_eq!(conversion.estimated_total_icp_debit_e8s, Some(5));
    assert_eq!(conversion.rate.estimated_deposit_fee_cycles, 20);
    let mut future = report(&request, "network".into(), 100, None);
    apply_quote(&mut future, selected, Some(rate), 9_000);
    assert!(future.funding.conversion.is_none());
    assert!(
        future
            .funding
            .unresolved
            .contains(&ReadinessUnresolved::ConversionObservationFailed)
    );
    assert!(!root.exists());
}
