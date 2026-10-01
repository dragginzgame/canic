//! Supplementary approvals retain their exact original observation across reopen and tampering.

use super::*;
use crate::fleet_ensure::{
    dto::capacity_import::CapacityImportFundingCreditRequest,
    ops::capacity_import::{prepare_review, with_funding},
    policy::capacity_import::tests::{plan, principal},
};

#[test]
fn credited_review_requires_its_original_survey_after_reopen() {
    let directory = crate::test_support::temp_dir("capacity-import-funding-origin");
    let paths = EnsurePaths::under(&directory, "local", "funding");
    let owner = CapacityImportJournalStore::open(&paths).unwrap();
    let mut plan = plan();
    let source = &plan.sources[0];
    let id = source.binding.canister_id;
    let ids = [id, plan.authority.root];
    let before = CapacityImportSampleRecord {
        binding: source.binding.clone(),
        cycles: source.observed_cycles,
        reserved_cycles: source.observed_reserved_cycles,
    };
    let digest = [2; 32];
    {
        let mut survey = CapacityImportSurveyStore::open(&owner, &paths, digest, &ids).unwrap();
        survey.reserve(id).unwrap();
        survey.retain(before.clone()).unwrap();
    }
    let original_path = paths
        .plan
        .with_file_name("capacity-import-surveys")
        .join(format!("{}.json", "02".repeat(32)));
    let original_bytes = std::fs::read(&original_path).unwrap();
    let baseline = survey_baseline(&owner, &paths, digest, &ids, id).unwrap();
    let mut observed = before;
    observed.cycles += 80;
    let funded = recognize(
        baseline,
        observed,
        100,
        plan.authority.root,
        source.maximum_debit_cycles,
        source.minimum_ready_cycles,
    )
    .unwrap();
    plan.sources[0].observed_cycles = funded.sample.cycles;
    let plan = with_funding(
        prepare_review(plan.authority, plan.sources, plan.root_budget).unwrap(),
        vec![funded.credit],
    )
    .unwrap();
    verify_origins(&owner, &paths, &plan).unwrap();
    assert_eq!(std::fs::read(&original_path).unwrap(), original_bytes);
    drop(owner);
    let owner = CapacityImportJournalStore::open(&paths).unwrap();
    verify_origins(&owner, &paths, &plan).unwrap();
    let mut changed = plan.clone();
    changed.funding_credits[0].before.cycles -= 1;
    assert!(matches!(
        verify_origins(&owner, &paths, &changed),
        Err(CapacityImportJournalError::Integrity)
    ));
    std::fs::remove_file(original_path).unwrap();
    assert!(matches!(verify_origins(&owner, &paths, &plan),
        Err(CapacityImportJournalError::FundingBaselineMissing { canister }) if canister == id));
    drop(owner);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn credit_selection_rejects_zero_unknown_and_duplicate_sources() {
    let root = principal(1);
    let mut request = CapacityImportReviewRequest {
        funding_credits: vec![CapacityImportFundingCreditRequest {
            canister: principal(9),
            cycles: 100,
        }],
        environment: "local".into(),
        fleet: "funding".into(),
        canisters: vec![principal(9)],
        root: Some(root),
        declarations: "import.toml".into(),
        policy: "policy.toml".into(),
        seed: "seed.toml".into(),
        maximum_source_debit_cycles: 200,
        maximum_root_debit_cycles: 100,
        maximum_root_paid_calls: 100,
    };
    assert_eq!(
        requested(&request, root).unwrap(),
        BTreeMap::from([(principal(9), 100)])
    );
    request.funding_credits[0].cycles = 0;
    assert!(matches!(
        requested(&request, root),
        Err(CapacityImportJournalError::Policy(
            CapacityImportPolicyError::InvalidSources
        ))
    ));
    request.funding_credits[0].cycles = 100;
    request.funding_credits[0].canister = principal(99);
    assert!(matches!(
        requested(&request, root),
        Err(CapacityImportJournalError::Policy(
            CapacityImportPolicyError::InvalidSources
        ))
    ));
    request.funding_credits[0].canister = root;
    requested(&request, root).unwrap();
    request
        .funding_credits
        .push(request.funding_credits[0].clone());
    assert!(matches!(
        requested(&request, root),
        Err(CapacityImportJournalError::Policy(
            CapacityImportPolicyError::InvalidSources
        ))
    ));
}
