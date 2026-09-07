//! Host persistence and review regressions for an underfunded retained operation.
//! The platform models host evidence; real Ledger effects are qualified in PocketIC.

use super::{
    tests::{estate_funding_observation, estate_funding_plan, retained_evidence},
    *,
};
use crate::fleet_ensure::{
    model::*,
    ops::{EffectObservation, EffectOutcome},
};
use std::{io, path::PathBuf};

const ROOT: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";

struct Fixture {
    root: PathBuf,
    paths: EnsurePaths,
    desired: DesiredFleet,
    plan: FleetEnsurePlan,
    platform: FundingPlatform,
}

impl Fixture {
    fn new() -> Self {
        let root = crate::test_support::temp_dir("funding-review");
        let desired: DesiredFleet = serde_json::from_value(serde_json::json!({
            "canisters": [{"controllers": [ROOT], "initial_cycles": "0B", "minimum_cycles": "0B",
                "kind": "auxiliary", "name": "root", "presence": "present", "principal": ROOT,
                "replace": false, "subnet": ROOT}],
            "cycles_ledger": "um5iw-rqaaa-aaaaq-qaaba-cai", "environment": "local", "fleet": "fleet",
            "ledger_fee_cycles": "0.000000005B", "management_creation_fee_cycles": "0B",
            "material_cycle_threshold": "1B", "maximum_observation_burn_cycles": "1B",
            "maximum_stalled_observations": 2, "maximum_update_burn_cycles": "2B",
            "operator": ROOT, "schema_version": 1, "treasury": "root"
        })).unwrap();
        let paths = EnsurePaths::under(&root, "local", "fleet");
        let mut plan = estate_funding_plan();
        plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(&desired)));
        plan.canisters.push(CanisterPlan {
            actions: Vec::new(),
            disposition: CanisterDisposition::Reuse,
            name: "root".into(),
            observed_cycles: 10,
            principal: Some(ROOT.into()),
        });
        plan.conservation.estate_funding_domains[0].maximum_funding_cycles = 0;
        plan.plan_sha256 = expected_plan_sha256(&plan);
        let (mut state, mut journal) = retained_evidence();
        state.completed_reinstall_action_sha256.clear();
        state.completed_reinstalls.clear();
        state.completed_reinstall_operation_id = None;
        journal.completion = FleetEnsureCompletion::InProgress;
        journal.effects.clear();
        journal.plan_sha256.clone_from(&plan.plan_sha256);
        journal.initial_controlled_cycles = 50;
        journal.initial_operator_cycles = 1000;
        journal
            .initial_estate_funding_cycles_by_root
            .insert("root".into(), 40);
        journal.estate_funding_required = estate_funding_requirement::<io::Error>(
            &plan,
            &state,
            &estate_funding_observation(Some(40)),
        )
        .unwrap();
        write_plan(&paths, &plan).unwrap();
        write_state(&paths, &state).unwrap();
        write_journal(&paths, &journal).unwrap();
        Self {
            root,
            paths,
            desired,
            plan,
            platform: FundingPlatform {
                balance: 40,
                operator: 1000,
                transfers: BTreeSet::new(),
                lose_reply: false,
                drift_fee: false,
                drift_controller: false,
            },
        }
    }

    fn review(&mut self) -> Result<FleetEnsureReport, EnsureWorkflowError<io::Error>> {
        plan(
            &self.root,
            &self.desired,
            "desired",
            "fleet",
            99,
            &mut self.platform,
        )
    }

    fn apply(&mut self, digest: &str) -> Result<FleetEnsureReport, EnsureWorkflowError<io::Error>> {
        apply(
            &self.root,
            &self.desired,
            "desired",
            "fleet",
            digest,
            &mut self.platform,
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

struct FundingPlatform {
    drift_controller: bool,
    balance: u128,
    operator: u128,
    transfers: BTreeSet<String>,
    lose_reply: bool,
    drift_fee: bool,
}

impl EnsurePlatform for FundingPlatform {
    type Error = io::Error;
    fn bind_reviewed_desired(&mut self, _: &DesiredFleet) -> Result<(), Self::Error> {
        Ok(())
    }
    fn observe(
        &mut self,
        _: &str,
        _: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error> {
        let mut observation = estate_funding_observation(Some(self.balance));
        observation.ledger_fee_cycles = if self.drift_fee { 6 } else { 5 };
        observation.operator_cycles = self.operator;
        observation.canisters.insert(
            "root".into(),
            Some(LiveCanister {
                canister_version: Some(1),
                controllers: if self.drift_controller {
                    vec!["aaaaa-aa".into()]
                } else {
                    vec![ROOT.into()]
                },
                cycles: 10,
                module_sha256: None,
                principal: ROOT.into(),
                reinstall_required: false,
                root_owned_lifecycle: None,
                status: CanisterRuntimeStatus::Running,
            }),
        );
        observation
            .estate_funding_domains
            .get_mut("root")
            .unwrap()
            .pool = Some(EstatePoolInventoryObservation {
            assets: Vec::new(),
            maximum_size: 2,
            minimum_size: 0,
            pending_creation: None,
            readiness_floor_cycles: 35,
            creation_execution_margin_cycles: 5,
        });
        Ok(observation)
    }
    fn observe_effect(
        &mut self,
        _: &str,
        _: &EnsureAction,
        _: &EffectRecord,
        _: &FleetEnsureStateRecord,
    ) -> Result<EffectObservation, Self::Error> {
        unreachable!("no ordinary effects in this host fixture")
    }
    fn action_cycles(
        &mut self,
        _: &EnsureAction,
        _: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        Ok(Some(self.operator))
    }
    fn action_destination_cycles(
        &mut self,
        _: &EnsureAction,
        _: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        Ok(Some(self.balance))
    }
    fn pace_effect_observation(&mut self, _: &EnsureAction, _: u32) {}
    fn apply(
        &mut self,
        _: &str,
        action: &EnsureAction,
        _: &EffectRecord,
        _: &FleetEnsureStateRecord,
    ) -> Result<EffectOutcome, Self::Error> {
        let EnsureAction::FundEstate {
            amount,
            ledger_fee_cycles,
            ..
        } = action
        else {
            unreachable!()
        };
        if self.transfers.insert(action_sha256(action)) {
            self.balance += amount;
            self.operator -= amount + ledger_fee_cycles;
        }
        if std::mem::take(&mut self.lose_reply) {
            return Err(io::Error::other("lost fixture reply"));
        }
        Ok(EffectOutcome {
            created_principal: None,
            post_cycles: None,
            receipt: Some("7".into()),
        })
    }
}

#[test]
fn funding_review_preserves_operation_and_requires_its_exact_digest() {
    let mut fixture = Fixture::new();
    let original = read_journal(&fixture.paths).unwrap().unwrap();
    let report = fixture.review().unwrap();
    assert_eq!(report.plan, fixture.plan);
    let review = report.funding_review.unwrap();
    assert_eq!(review.pause.shortfall_cycles, 60);
    assert!(review.effect.is_none());
    assert_eq!(fixture.review().unwrap().funding_review, Some(review));
    let digest = fixture.plan.plan_sha256.clone();
    assert!(matches!(
        fixture.apply(&digest),
        Err(EnsureWorkflowError::EstateFundingRequired(_))
    ));
    assert!(matches!(
        fixture.apply(&"ab".repeat(32)),
        Err(EnsureWorkflowError::PlanDigestMismatch { .. })
    ));
    assert!(fixture.platform.transfers.is_empty());
    let retained = read_journal(&fixture.paths).unwrap().unwrap();
    assert_eq!(retained.effects, original.effects);
    assert_eq!(retained.operation_id, original.operation_id);
    assert_eq!(retained.plan_sha256, original.plan_sha256);
}

#[test]
fn funding_review_recovers_lost_reply_and_replays_without_another_debit() {
    let mut fixture = Fixture::new();
    let review = fixture.review().unwrap().funding_review.unwrap();
    fixture.platform.lose_reply = true;
    assert!(matches!(
        fixture.apply(&review.review_sha256),
        Err(EnsureWorkflowError::Platform(_))
    ));
    let issued = read_journal(&fixture.paths).unwrap().unwrap();
    assert_eq!(
        issued.funding_reviews[0].effect.as_ref().unwrap().state,
        EffectState::Intent
    );
    assert_eq!(fixture.platform.transfers.len(), 1);
    assert_eq!(
        fixture.review().unwrap().funding_review.unwrap().action,
        review.action
    );
    // This host-only fixture deliberately has no Coordinator/runtime topology.
    // Reaching its policy boundary proves funding resumed; PocketIC owns Fleet readiness.
    assert!(matches!(
        fixture.apply(&review.review_sha256),
        Err(EnsureWorkflowError::Policy(_))
    ));
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    let state = read_state(&fixture.paths, "fleet").unwrap();
    let observation = fixture
        .platform
        .observe(&fixture.plan.operation_id, &state)
        .unwrap();
    let actual =
        verify_terminal_conservation::<io::Error>(&fixture.plan, &journal, &state, &observation)
            .unwrap();
    assert_eq!(
        (
            actual.estate_funding_cycles,
            actual.operator_debit_cycles,
            actual.exact_unavoidable_fee_cycles
        ),
        (60, 65, 5)
    );
    let original_digest = fixture.plan.plan_sha256.clone();
    funding::resume(
        &fixture.paths,
        &fixture.plan,
        &mut journal,
        &state,
        &original_digest,
        &mut fixture.platform,
    )
    .unwrap();
    funding::resume(
        &fixture.paths,
        &fixture.plan,
        &mut journal,
        &state,
        &review.review_sha256,
        &mut fixture.platform,
    )
    .unwrap();
    assert_eq!(fixture.platform.transfers.len(), 1);
}

#[test]
fn funding_review_rejects_fee_balance_and_authority_drift_before_debit() {
    let mut fixture = Fixture::new();
    let review = fixture.review().unwrap().funding_review.unwrap();
    fixture.platform.drift_fee = true;
    assert!(matches!(
        fixture.apply(&review.review_sha256),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
    fixture.platform.drift_fee = false;
    fixture.platform.balance = 39;
    assert!(matches!(
        fixture.apply(&review.review_sha256),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
    fixture.platform.balance = 40;
    fixture.platform.drift_controller = true;
    assert!(matches!(
        fixture.apply(&review.review_sha256),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
    fixture.platform.drift_controller = false;
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    journal.funding_reviews[0].pause.shortfall_cycles += 1;
    write_journal(&fixture.paths, &journal).unwrap();
    assert!(matches!(
        fixture.apply(&review.review_sha256),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
    assert!(fixture.platform.transfers.is_empty());
}

#[test]
fn completed_funding_review_cannot_authorize_a_later_plan() {
    let mut fixture = Fixture::new();
    let review = fixture.review().unwrap().funding_review.unwrap();
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    let state = read_state(&fixture.paths, "fleet").unwrap();
    funding::resume(
        &fixture.paths,
        &fixture.plan,
        &mut journal,
        &state,
        &review.review_sha256,
        &mut fixture.platform,
    )
    .unwrap();
    journal.estate_funding_required = None;
    journal.completion = FleetEnsureCompletion::Converged;
    write_journal(&fixture.paths, &journal).unwrap();
    let mut later = fixture.plan.clone();
    later.planned_at_time += 1;
    later.plan_sha256 = expected_plan_sha256(&later);
    write_plan(&fixture.paths, &later).unwrap();
    assert!(matches!(
        fixture.apply(&review.review_sha256),
        Err(EnsureWorkflowError::PlanDigestMismatch { .. })
    ));
    assert_eq!(fixture.platform.transfers.len(), 1);
}
