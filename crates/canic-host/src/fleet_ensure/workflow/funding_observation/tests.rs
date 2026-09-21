use super::*;
use crate::fleet_ensure::{
    model::{
        CanisterDisposition, CanisterPlan, DesiredFleet, EffectState, EnsureAction,
        ReviewedDesiredFleetRecord, funding_observation::FundingComponentHeadRecord,
    },
    policy::expected_plan_sha256,
    view::startup_funding::StartupNativeBalance,
    workflow::tests::{estate_funding_plan, retained_evidence},
};
use std::{collections::BTreeMap, io};

#[derive(PartialEq)]
enum FailureStep {
    Inspection,
    Ledger,
}

struct Platform {
    paths: EnsurePaths,
    root: StartupRootFunding,
    authority: FundingObservationAuthorityRecord,
    calls: usize,
    snapshots: usize,
    fail: bool,
    failure_step: FailureStep,
    ledger_calls: usize,
    crash: bool,
    drift_after_call: bool,
}

impl FundingObservationPlatform for Platform {
    type Error = io::Error;

    fn snapshot(
        &mut self,
        _: &FleetEnsurePlan,
        _: &DesiredFleet,
        _: &str,
    ) -> Result<FundingObservationSnapshot, Self::Error> {
        self.snapshots += 1;
        let mut authority = self.authority.clone();
        if self.drift_after_call && self.calls > 0 {
            authority.revision += 1;
        }
        let configuration_source =
            crate::fleet_ensure::policy::startup_funding::hub_source().to_owned();
        Ok(FundingObservationSnapshot {
            configuration_source,
            root: self.root.clone(),
            authority,
        })
    }

    fn child_balance(
        &mut self,
        _: &FleetEnsurePlan,
        request: &FundingObservationRequestRecord,
    ) -> Result<u128, Self::Error> {
        let journal = ops::read_journal(&self.paths).unwrap().unwrap();
        let review = &journal.funding_observations[&self.root.root];
        assert!(review.approved);
        let attempt = review.attempts.last().unwrap();
        assert_eq!(review.body.requests[attempt.request_index], *request);
        assert_eq!(attempt.outcome, None);
        self.calls += 1;
        assert!(
            !self.crash || self.failure_step == FailureStep::Ledger,
            "simulated process interruption after durable intent"
        );
        if self.fail && self.failure_step != FailureStep::Ledger {
            return Err(io::Error::other("simulated lost response"));
        }
        Ok(4_000_000_000_000)
    }

    fn child_usage(
        &mut self,
        _: &FleetEnsurePlan,
        request: &FundingObservationRequestRecord,
        native_cycles: u128,
    ) -> Result<FundingChildAccountingRecord, Self::Error> {
        self.ledger_calls += 1;
        assert!(
            !self.crash,
            "simulated interruption after the second paid call"
        );
        if self.fail {
            return Err(io::Error::other("simulated lost ledger response"));
        }
        Ok(FundingChildAccountingRecord {
            native_cycles,
            parent: request.parent,
            child: request.child,
            observed_at_ns: 10_000_000_000,
            accounted_cycles: 7,
            last_accounted_at_secs: 9,
            pending_operations: 0,
            reserved_cycles: Some(0),
        })
    }
}

fn fixture(desired: &DesiredFleet, root: &StartupRootFunding) -> (FleetEnsurePlan, Platform) {
    let directory = crate::test_support::temp_dir("funding-observation-journal");
    let paths = EnsurePaths::under(&directory, &desired.environment, &desired.fleet);
    let mut plan = estate_funding_plan();
    plan.fleet.clone_from(&desired.fleet);
    plan.environment.clone_from(&desired.environment);
    plan.conservation.estate_funding_domains.clear();
    plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(desired)));
    plan.plan_sha256 = expected_plan_sha256(&plan);
    let (mut state, mut journal) = retained_evidence();
    state.fleet.clone_from(&desired.fleet);
    journal.fleet.clone_from(&desired.fleet);
    journal.plan_sha256.clone_from(&plan.plan_sha256);
    journal.effects.clear();
    journal.completion = FleetEnsureCompletion::InProgress;
    let bindings = root
        .child_usage
        .iter()
        .filter_map(|child| child.binding.as_ref())
        .collect::<Vec<_>>();
    let authority = FundingObservationAuthorityRecord {
        registry: bindings[0].component.authority.clone(),
        revision: 2,
        content_hash: [2; 32],
        components: bindings
            .iter()
            .map(|binding| {
                (
                    binding.component.component,
                    FundingComponentHeadRecord {
                        revision: 3,
                        content_hash: [3; 32],
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    };
    ops::write_plan(&paths, &plan).unwrap();
    ops::write_state(&paths, &state).unwrap();
    ops::write_journal(&paths, &journal).unwrap();
    let mut root = root.clone();
    root.balance = StartupNativeBalance::Observed(u128::MAX / 2);
    (
        plan,
        Platform {
            paths,
            root,
            authority,
            calls: 0,
            snapshots: 0,
            fail: false,
            failure_step: FailureStep::Inspection,
            ledger_calls: 0,
            crash: false,
            drift_after_call: false,
        },
    )
}

/// Use the generated estate's compiled authority to exercise real durable writes and restoration.
pub(in crate::fleet_ensure) fn qualify(desired: &DesiredFleet, root: &StartupRootFunding) {
    replay_and_conservation(desired, root);
    failures_remain_consumed(desired, root);
    admission_and_drift(desired, root);
}

fn replay_and_conservation(desired: &DesiredFleet, root: &StartupRootFunding) {
    let (plan, mut platform) = fixture(desired, root);
    let paths = EnsurePaths::under(
        &platform.paths.workspace,
        &desired.environment,
        &desired.fleet,
    );
    let retained = review(&paths, &root.root, &mut platform).unwrap();
    assert!(!retained.approved);
    assert_eq!(platform.calls, 0);
    assert!(matches!(
        collect(&paths, &root.root, "wrong", &mut platform),
        Err(EnsureWorkflowError::FundingObservation(
            FundingObservationError::ApprovalMismatch
        ))
    ));
    let before = std::fs::read(&paths.journal).unwrap();
    assert_eq!(review(&paths, &root.root, &mut platform).unwrap(), retained);
    assert_eq!(std::fs::read(&paths.journal).unwrap(), before);
    let complete = collect(&paths, &root.root, &retained.review_sha256, &mut platform).unwrap();
    assert_eq!(platform.calls, retained.body.requests.len());
    assert!(complete.attempts.iter().all(|attempt| matches!(
        attempt.outcome,
        Some(FundingObservationOutcomeRecord::Observed(_))
    )));
    let journal = ops::read_journal(&paths).unwrap().unwrap();
    let funded = crate::fleet_ensure::workflow::funding_plan::<io::Error>(&plan, &journal).unwrap();
    assert_eq!(
        funded.conservation.maximum_execution_burn_cycles,
        plan.conservation.maximum_execution_burn_cycles + retained.body.maximum_cycles
    );
    assert_eq!(
        funded.conservation.maximum_operator_debit_cycles,
        plan.conservation.maximum_operator_debit_cycles
    );
    crate::fleet_ensure::workflow::funding::qualify_observation_quotes(&plan, &journal, &complete);
    let demand = status(&paths, &root.root).unwrap().recovery_demand.unwrap();
    assert_eq!(demand.root_grants_cycles, 30_000_000_000_000);
    assert_eq!(
        validation::source_allowance(
            desired,
            &plan.operation_id,
            &plan.plan_sha256,
            &journal.funding_observations
        )
        .unwrap(),
        retained.body.maximum_cycles
    );
    assert!(
        validation::source_allowance(
            desired,
            &"ff".repeat(32),
            &plan.plan_sha256,
            &journal.funding_observations
        )
        .is_err()
    );
    let snapshots = platform.snapshots;
    let bytes = std::fs::read(&paths.journal).unwrap();
    assert_eq!(
        collect(&paths, &root.root, &retained.review_sha256, &mut platform).unwrap(),
        complete
    );
    assert_eq!(platform.snapshots, snapshots);
    assert_eq!(platform.calls, retained.body.requests.len());
    assert_eq!(std::fs::read(&paths.journal).unwrap(), bytes);
    let mut corrupt = journal;
    corrupt
        .funding_observations
        .get_mut(&root.root)
        .unwrap()
        .approved = false;
    assert!(validation::verify(&plan, &corrupt).is_err());
    std::fs::remove_dir_all(&paths.workspace).unwrap();
}

fn failures_remain_consumed(desired: &DesiredFleet, root: &StartupRootFunding) {
    for (crash, fail_in_ledger) in [(false, false), (true, false), (false, true), (true, true)] {
        let (plan, mut platform) = fixture(desired, root);
        let paths = EnsurePaths::under(
            &platform.paths.workspace,
            &desired.environment,
            &desired.fleet,
        );
        let retained = review(&paths, &root.root, &mut platform).unwrap();
        platform.crash = crash;
        platform.failure_step = if fail_in_ledger {
            FailureStep::Ledger
        } else {
            FailureStep::Inspection
        };
        platform.fail = !crash;
        let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            collect(&paths, &root.root, &retained.review_sha256, &mut platform)
        }));
        assert_eq!(first.is_err(), crash);
        assert_eq!(platform.ledger_calls, usize::from(fail_in_ledger));
        let journal = ops::read_journal(&paths).unwrap().unwrap();
        assert_eq!(journal.funding_observations[&root.root].attempts.len(), 1);
        assert_eq!(
            validation::total(&journal).unwrap(),
            retained.body.per_attempt_cycles
        );
        validation::verify(&plan, &journal).unwrap();
        platform.crash = false;
        platform.fail = false;
        if crash {
            let resumed =
                collect(&paths, &root.root, &retained.review_sha256, &mut platform).unwrap();
            assert_eq!(platform.calls, 1);
            assert_eq!(
                resumed.attempts[0].outcome,
                Some(FundingObservationOutcomeRecord::Interrupted)
            );
        }
        let complete = collect(&paths, &root.root, &retained.review_sha256, &mut platform).unwrap();
        assert_eq!(platform.calls, retained.body.requests.len());
        assert_eq!(complete.attempts.len(), retained.body.requests.len());
        assert_eq!(review(&paths, &root.root, &mut platform).unwrap(), complete);
        std::fs::remove_dir_all(&paths.workspace).unwrap();
    }
}

fn admission_and_drift(desired: &DesiredFleet, root: &StartupRootFunding) {
    let (_, mut platform) = fixture(desired, root);
    let paths = EnsurePaths::under(
        &platform.paths.workspace,
        &desired.environment,
        &desired.fleet,
    );
    let retained = review(&paths, &root.root, &mut platform).unwrap();
    platform.root.balance = StartupNativeBalance::Observed(retained.body.recovery_floor_cycles);
    assert!(matches!(
        collect(&paths, &root.root, &retained.review_sha256, &mut platform),
        Err(EnsureWorkflowError::FundingObservation(
            FundingObservationError::Underfunded
        ))
    ));
    assert_eq!(platform.calls, 0);
    assert!(
        ops::read_journal(&paths)
            .unwrap()
            .unwrap()
            .funding_observations[&root.root]
            .attempts
            .is_empty()
    );
    platform.root.balance = StartupNativeBalance::Observed(u128::MAX / 2);
    platform.authority.revision += 1;
    assert!(matches!(
        collect(&paths, &root.root, &retained.review_sha256, &mut platform),
        Err(EnsureWorkflowError::FundingObservation(
            FundingObservationError::AuthorityMismatch
        ))
    ));
    assert_eq!(platform.calls, 0);
    platform.authority.revision -= 1;
    platform.drift_after_call = true;
    let failed = collect(&paths, &root.root, &retained.review_sha256, &mut platform).unwrap();
    assert_eq!(platform.calls, 1);
    assert_eq!(
        failed.attempts[0].outcome,
        Some(FundingObservationOutcomeRecord::AuthorityChanged)
    );
    std::fs::remove_dir_all(&paths.workspace).unwrap();
}

#[test]
fn funding_observation_resolves_only_applied_creation_receipts() {
    let mut plan = estate_funding_plan();
    let mut desired: DesiredFleet = serde_json::from_value(serde_json::json!({
        "canisters": [{"controllers": ["2vxsx-fae"], "initial_cycles": "1T", "minimum_cycles": "1T",
            "kind": "root", "name": "root", "presence": "present", "principal": null,
            "replace": false, "subnet": "aaaaa-aa"}],
        "cycles_ledger": "um5iw-rqaaa-aaaaq-qaaba-cai", "environment": "local", "fleet": "fleet",
        "ledger_fee_cycles": "0B", "management_creation_fee_cycles": "0B",
        "material_cycle_threshold": "1B", "maximum_observation_burn_cycles": "1B",
        "maximum_stalled_observations": 2, "maximum_update_burn_cycles": "2B",
        "operator": "2vxsx-fae", "schema_version": 1, "treasury": "root"
    }))
    .unwrap();
    plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(&desired)));
    let action = EnsureAction::Create {
        controller_canisters: vec![],
        controllers: vec![desired.operator.clone()],
        created_at_time: 1,
        ledger: desired.cycles_ledger.clone(),
        name: "root".into(),
        requested_initial_cycles: 1_000_000_000_000,
        subnet: "aaaaa-aa".into(),
    };
    plan.canisters.push(CanisterPlan {
        actions: vec![action.clone()],
        disposition: CanisterDisposition::Create,
        name: "root".into(),
        observed_cycles: 0,
        principal: None,
    });
    let (_, mut journal) = retained_evidence();
    journal.effects.clear();
    let mut effect = crate::fleet_ensure::ops::funding::intent(&action, 0, 0);
    effect.created_principal = Some("rrkah-fqaaa-aaaaa-aaaaq-cai".into());
    journal.effects.push(effect);
    for state in [EffectState::Intent, EffectState::Issued] {
        journal.effects[0].state = state;
        assert_eq!(
            records::resolved(&plan, &journal).unwrap().canisters[0].principal,
            None
        );
    }
    journal.effects[0].state = EffectState::Applied;
    assert_eq!(
        records::resolved(&plan, &journal).unwrap().canisters[0].principal,
        journal.effects[0].created_principal
    );
    journal.effects[0].action_sha256 = "ff".repeat(32);
    assert_eq!(
        records::resolved(&plan, &journal).unwrap().canisters[0].principal,
        None
    );
    desired.canisters[0].principal = Some("rrkah-fqaaa-aaaaa-aaaaq-cai".into());
    plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(&desired)));
    assert_eq!(
        records::resolved(&plan, &journal).unwrap().canisters[0].principal,
        desired.canisters[0].principal
    );
}
