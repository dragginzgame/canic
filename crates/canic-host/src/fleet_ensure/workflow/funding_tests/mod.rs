//! Host persistence and review regressions for an underfunded retained operation.
//! The platform models host evidence; real Ledger effects are qualified in PocketIC.

mod operator_mint_reply_tests;
mod operator_mint_tests;

use super::{
    tests::{estate_funding_observation, estate_funding_plan, retained_evidence},
    *,
};
use crate::fleet_ensure::{
    model::*,
    ops::{EffectObservation, EffectOutcome, read_journal},
};
use std::{io, path::PathBuf};

const ROOT: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";
// Native cases keep a 90-cycle deficit below the real deployment floor while
// retaining the small exact fee/burn deltas used by the accounting assertions.
const NATIVE_BASE: u128 =
    canic_core::control_plane_support::policy::deployment::MINIMUM_DEPLOYMENT_RESERVE_CYCLES - 100;

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
                controllers: vec![ROOT.into()],
                cycles_ledger: "ledger".into(),
                balance: 40,
                native_balance: 10,
                operator: 1000,
                transfers: BTreeSet::new(),
                fault: FundingTransportFault::None,
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

#[derive(Eq, PartialEq)]
enum FundingTransportFault {
    None,
    LostReply,
    LostPostObservation,
}

struct FundingPlatform {
    controllers: Vec<String>,
    cycles_ledger: String,
    drift_controller: bool,
    balance: u128,
    native_balance: u128,
    operator: u128,
    transfers: BTreeSet<String>,
    fault: FundingTransportFault,
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
        observation
            .estate_funding_domains
            .get_mut("root")
            .unwrap()
            .cycles_ledger
            .clone_from(&self.cycles_ledger);
        observation.ledger_fee_cycles = if self.drift_fee { 6 } else { 5 };
        observation.operator_cycles = self.operator;
        observation.canisters.insert(
            "root".into(),
            Some(LiveCanister {
                canister_version: Some(1),
                controllers: if self.drift_controller {
                    vec!["aaaaa-aa".into()]
                } else {
                    self.controllers.clone()
                },
                cycles: self.native_balance,
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
    fn observe_native_funding(
        &mut self,
        _: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<NativeFundingObservation>, Self::Error> {
        if self.fault == FundingTransportFault::LostPostObservation && !self.transfers.is_empty() {
            self.fault = FundingTransportFault::None;
            return Err(io::Error::other("post-withdrawal observation unavailable"));
        }
        let observed = self.observe("operation", state)?;
        Ok(Some(NativeFundingObservation {
            cycles_ledger: "um5iw-rqaaa-aaaaq-qaaba-cai".into(),
            ledger_fee_cycles: observed.ledger_fee_cycles,
            live: observed.canisters["root"].clone().unwrap(),
            operator_cycles: self.operator,
        }))
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
        let (amount, fee, balance) = match action {
            EnsureAction::FundEstate {
                amount,
                ledger_fee_cycles,
                ..
            } => (*amount, *ledger_fee_cycles, &mut self.balance),
            EnsureAction::Fund { amount, .. } => (*amount, 5, &mut self.native_balance),
            _ => unreachable!(),
        };
        if self.transfers.insert(action_sha256(action)) {
            *balance += amount;
            self.operator -= amount + fee;
        }
        if self.fault == FundingTransportFault::LostReply {
            self.fault = FundingTransportFault::None;
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
fn mixed_root_creation_fees_reject_before_funding_and_preserve_evidence() {
    let mut fixture = Fixture::new();
    let mut second_root = fixture.desired.canisters[0].clone();
    second_root.name = "second-root".into();
    second_root.subnet = candid::Principal::from_slice(&[77]).to_text();
    fixture.desired.canisters.push(second_root);
    let mut second_domain = fixture.plan.conservation.estate_funding_domains[0].clone();
    second_domain.root = "second-root".into();
    fixture
        .plan
        .conservation
        .estate_funding_domains
        .push(second_domain);
    fixture.plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(
        &fixture.desired,
    )));
    fixture.plan.plan_sha256 = expected_plan_sha256(&fixture.plan);
    write_plan(&fixture.paths, &fixture.plan).unwrap();
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    journal.plan_sha256.clone_from(&fixture.plan.plan_sha256);
    journal
        .estate_funding_required
        .as_mut()
        .unwrap()
        .plan_sha256
        .clone_from(&fixture.plan.plan_sha256);
    journal
        .initial_estate_funding_cycles_by_root
        .insert("second-root".into(), 40);
    write_journal(&fixture.paths, &journal).unwrap();
    let digest = fixture.plan.plan_sha256.clone();
    assert!(matches!(
        fixture.apply(&digest),
        Err(EnsureWorkflowError::Policy(
            EnsurePolicyError::MixedSubnetCreationFees { .. }
        ))
    ));
    assert!(fixture.platform.transfers.is_empty());
    assert_eq!(read_journal(&fixture.paths).unwrap().unwrap(), journal);
    assert_eq!(read_plan(&fixture.paths).unwrap().unwrap(), fixture.plan);
}

#[test]
fn creation_fee_scope_includes_pending_roots_and_excludes_idle_roots() {
    let mut fixture = Fixture::new();
    let mut second_root = fixture.desired.canisters[0].clone();
    second_root.name = "second-root".into();
    second_root.subnet = candid::Principal::from_slice(&[77]).to_text();
    fixture.desired.canisters.push(second_root);
    let mut second_domain = fixture.plan.conservation.estate_funding_domains[0].clone();
    second_domain.root = "second-root".into();
    second_domain.required_creation_count = 0;
    fixture
        .plan
        .conservation
        .estate_funding_domains
        .push(second_domain);
    assert_eq!(
        validate_creation_fee_scope(
            &fixture.desired,
            &fixture.plan.canisters,
            &fixture.plan.conservation.estate_funding_domains,
        ),
        Ok(())
    );
    fixture.plan.conservation.estate_funding_domains[1].pending_creation_count = 1;
    assert!(matches!(
        validate_creation_fee_scope(
            &fixture.desired,
            &fixture.plan.canisters,
            &fixture.plan.conservation.estate_funding_domains,
        ),
        Err(EnsurePolicyError::MixedSubnetCreationFees { .. })
    ));
}

#[test]
fn funding_review_preserves_operation_and_requires_its_exact_digest() {
    let mut fixture = Fixture::new();
    let original = read_journal(&fixture.paths).unwrap().unwrap();
    let report = fixture.review().unwrap();
    assert_eq!(report.plan, fixture.plan);
    let review = report.funding_review.unwrap();
    assert_eq!(review.pause.shortfall_cycles(), 60);
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
    fixture.platform.fault = FundingTransportFault::LostReply;
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
    let FundingPauseRecord::Estate(pause) = &mut journal.funding_reviews[0].pause else {
        panic!("estate review");
    };
    pause.shortfall_cycles += 1;
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

impl Fixture {
    fn native() -> Self {
        let mut fixture = Self::new();
        fixture.platform.native_balance += NATIVE_BASE;
        fixture.plan.canisters[0].observed_cycles += NATIVE_BASE;
        fixture.desired.canisters[0].kind = DesiredCanisterKind::Root;
        fixture.desired.canisters[0].minimum_cycles = "0.0000001B".into();
        fixture.desired.maximum_observation_burn_cycles = "0.000000002B".into();
        fixture.desired.maximum_update_burn_cycles = "0.000000003B".into();
        fixture.plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(
            &fixture.desired,
        )));
        let mut action = crate::fleet_ensure::tests::typed_protocol_action(&"12".repeat(32));
        let EnsureAction::FleetProtocol {
            action: protocol, ..
        } = &mut action
        else {
            unreachable!()
        };
        let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = protocol.as_mut()
        else {
            unreachable!()
        };
        request
            .plan
            .directory_confirmation_roots
            .push(ROOT.parse().unwrap());
        fixture.plan.protocol_actions.push(action.clone());
        fixture.plan.conservation.maximum_execution_burn_cycles = 40;
        fixture.plan.plan_sha256 = expected_plan_sha256(&fixture.plan);
        let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
        journal.estate_funding_required = None;
        journal.initial_controlled_cycles += NATIVE_BASE;
        journal.plan_sha256.clone_from(&fixture.plan.plan_sha256);
        let mut effect = crate::fleet_ensure::ops::funding::intent(&action, 10 + NATIVE_BASE, 10);
        effect.state = EffectState::Issued;
        effect.receipt = Some("retained-provisioning".into());
        journal.effects.push(effect);
        write_plan(&fixture.paths, &fixture.plan).unwrap();
        write_journal(&fixture.paths, &journal).unwrap();
        fixture
    }

    fn native_review(
        &mut self,
    ) -> Result<Option<FundingReviewRecord>, EnsureWorkflowError<io::Error>> {
        let mut journal = read_journal(&self.paths).unwrap().unwrap();
        let state = read_state(&self.paths, "fleet").unwrap();
        funding::prepare(
            &self.paths,
            &self.plan,
            &mut journal,
            &state,
            99,
            &mut self.platform,
        )
    }

    fn native_resume(&mut self, digest: &str) -> Result<(), EnsureWorkflowError<io::Error>> {
        let mut journal = read_journal(&self.paths).unwrap().unwrap();
        let state = read_state(&self.paths, "fleet").unwrap();
        funding::verify(&self.plan, &journal, &state)?;
        funding::resume(
            &self.paths,
            &self.plan,
            &mut journal,
            &state,
            digest,
            &mut self.platform,
        )
    }
}

#[test]
fn native_funding_review_preserves_issued_operation_and_separate_approval() {
    let mut fixture = Fixture::native();
    let before = read_journal(&fixture.paths).unwrap().unwrap();
    let review = fixture.review().unwrap().funding_review.unwrap();
    assert_eq!(review.pause.shortfall_cycles(), 95);
    assert!(matches!(review.pause, FundingPauseRecord::Native(_)));
    assert!(review.effect.is_none());
    assert_eq!(fixture.native_review().unwrap(), Some(review.clone()));
    assert!(matches!(
        fixture.native_resume(&fixture.plan.plan_sha256.clone()),
        Err(EnsureWorkflowError::NativeFundingRequired { .. })
    ));
    assert!(fixture.platform.transfers.is_empty());
    fixture.native_resume(&review.review_sha256).unwrap();
    let after = read_journal(&fixture.paths).unwrap().unwrap();
    assert_eq!(after.effects, before.effects);
    assert_eq!(after.operation_id, before.operation_id);
    assert_eq!(after.plan_sha256, before.plan_sha256);
    assert_eq!(
        after.initial_operator_cycles,
        before.initial_operator_cycles
    );
    assert_eq!(
        after.initial_controlled_cycles,
        before.initial_controlled_cycles
    );
    assert_eq!(
        after.initial_estate_funding_cycles_by_root,
        before.initial_estate_funding_cycles_by_root
    );
    assert_eq!(
        (
            fixture.platform.operator,
            fixture.platform.native_balance,
            fixture.platform.balance
        ),
        (900, 105 + NATIVE_BASE, 40)
    );
    fixture.native_resume(&review.review_sha256).unwrap();
    fixture.platform.native_balance = 10 + NATIVE_BASE;
    assert!(fixture.native_review().unwrap().is_none());
    assert_eq!(fixture.platform.transfers.len(), 1);
}

#[test]
fn operator_mint_can_be_reviewed_before_supplementary_funds_are_available() {
    for native in [false, true] {
        let mut fixture = if native {
            Fixture::native()
        } else {
            Fixture::new()
        };
        let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
        journal.initial_operator_cycles = 0;
        write_journal(&fixture.paths, &journal).unwrap();
        fixture.platform.operator = 0;
        let review = fixture.native_review().unwrap().unwrap();
        assert!(review.effect.is_none());
        assert!(matches!(
            fixture.native_resume(&review.review_sha256),
            Err(EnsureWorkflowError::OperatorFundingRequired { .. })
        ));
        assert!(fixture.platform.transfers.is_empty());
        assert!(
            read_journal(&fixture.paths)
                .unwrap()
                .unwrap()
                .funding_reviews[0]
                .effect
                .is_none()
        );
    }
}

#[test]
fn native_funding_review_recovers_lost_withdrawal_and_conserves_exact_credit() {
    let mut fixture = Fixture::native();
    let review = fixture.native_review().unwrap().unwrap();
    fixture.platform.fault = FundingTransportFault::LostReply;
    assert!(matches!(
        fixture.native_resume(&review.review_sha256),
        Err(EnsureWorkflowError::Platform(_))
    ));
    let issued = read_journal(&fixture.paths).unwrap().unwrap();
    assert_eq!(
        issued.funding_reviews[0].effect.as_ref().unwrap().state,
        EffectState::Intent
    );
    for unexplained in [995, 899, 1001] {
        fixture.platform.operator = unexplained;
        assert!(matches!(
            fixture.native_resume(&review.review_sha256),
            Err(EnsureWorkflowError::DriftedBeforeApply)
        ));
        assert_eq!(fixture.platform.transfers.len(), 1);
    }
    fixture.platform.operator = 900;
    fixture.platform.native_balance -= 35;
    fixture.native_resume(&review.review_sha256).unwrap();
    assert_eq!(fixture.platform.transfers.len(), 1);
    let journal = read_journal(&fixture.paths).unwrap().unwrap();
    let state = read_state(&fixture.paths, "fleet").unwrap();
    funding::verify::<io::Error>(&fixture.plan, &journal, &state).unwrap();
    let terminal = fixture.platform.observe("operation", &state).unwrap();
    let actual =
        verify_terminal_conservation::<io::Error>(&fixture.plan, &journal, &state, &terminal)
            .unwrap();
    assert_eq!(actual.operator_debit_cycles, 100);
    assert_eq!(actual.received_new_funding_cycles, 95);
    assert_eq!(actual.exact_unavoidable_fee_cycles, 5);
    assert_eq!(actual.estate_funding_cycles, 0);
    assert_eq!(actual.measured_execution_burn_cycles, 35);
    assert_eq!(actual.observed_starting_cycles, 50 + NATIVE_BASE);
    assert_eq!(actual.final_controlled_cycles, 110 + NATIVE_BASE);
}

#[test]
fn native_funding_review_rejects_drift_and_unreceipted_operator_credit() {
    let mut fixture = Fixture::native();
    fixture.platform.operator += 1;
    assert!(matches!(
        fixture.native_review(),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
    fixture.platform.operator -= 1;
    let review = fixture.native_review().unwrap().unwrap();
    for drift in 0..3 {
        fixture.platform.drift_fee = drift == 0;
        fixture.platform.drift_controller = drift == 1;
        fixture.platform.native_balance = NATIVE_BASE + if drift == 2 { 4 } else { 10 };
        assert!(matches!(
            fixture.native_resume(&review.review_sha256),
            Err(EnsureWorkflowError::DriftedBeforeApply)
        ));
    }
    fixture.platform.drift_controller = false;
    fixture.platform.native_balance = 10 + NATIVE_BASE;
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    let FundingPauseRecord::Native(pause) = &mut journal.funding_reviews[0].pause else {
        unreachable!()
    };
    pause.shortfall_cycles += 1;
    write_journal(&fixture.paths, &journal).unwrap();
    assert!(matches!(
        fixture.native_resume(&review.review_sha256),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
    assert!(fixture.platform.transfers.is_empty());
}

#[test]
fn native_funding_review_requires_issued_provisioning_and_rejects_duplicate_authority() {
    let mut fixture = Fixture::native();
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    journal.effects[0].state = EffectState::Intent;
    write_journal(&fixture.paths, &journal).unwrap();
    assert!(fixture.native_review().unwrap().is_none());
    journal.effects[0].state = EffectState::Issued;
    write_journal(&fixture.paths, &journal).unwrap();
    let review = fixture.native_review().unwrap().unwrap();
    fixture.native_resume(&review.review_sha256).unwrap();
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    journal
        .funding_reviews
        .push(journal.funding_reviews[0].clone());
    let state = read_state(&fixture.paths, "fleet").unwrap();
    assert!(matches!(
        funding::verify::<io::Error>(&fixture.plan, &journal, &state),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
    journal.funding_reviews.pop();
    journal.funding_reviews[0]
        .effect
        .as_mut()
        .unwrap()
        .post_cycles = Some(901);
    assert!(matches!(
        funding::verify::<io::Error>(&fixture.plan, &journal, &state),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
}

#[test]
fn native_funding_review_retains_receipt_when_post_observation_is_lost() {
    let mut fixture = Fixture::native();
    let review = fixture.native_review().unwrap().unwrap();
    fixture.platform.fault = FundingTransportFault::LostPostObservation;
    assert!(matches!(
        fixture.native_resume(&review.review_sha256),
        Err(EnsureWorkflowError::Platform(_))
    ));
    let journal = read_journal(&fixture.paths).unwrap().unwrap();
    let effect = journal.funding_reviews[0].effect.as_ref().unwrap();
    assert_eq!(effect.state, EffectState::Issued);
    assert_eq!(effect.receipt.as_deref(), Some("7"));
    assert!(effect.post_cycles.is_none());
    fixture.native_resume(&review.review_sha256).unwrap();
    assert_eq!(fixture.platform.transfers.len(), 1);
    assert_eq!(fixture.platform.operator, 900);
}

#[test]
fn native_funding_review_does_not_credit_a_root_outside_provisioning_scope() {
    let mut fixture = Fixture::native();
    let EnsureAction::FleetProtocol { action, .. } = &mut fixture.plan.protocol_actions[0] else {
        unreachable!()
    };
    let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = action.as_mut() else {
        unreachable!()
    };
    request.plan.directory_confirmation_roots.clear();
    fixture.plan.plan_sha256 = expected_plan_sha256(&fixture.plan);
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    journal.plan_sha256.clone_from(&fixture.plan.plan_sha256);
    journal.effects[0].action_sha256 = action_sha256(&fixture.plan.protocol_actions[0]);
    write_plan(&fixture.paths, &fixture.plan).unwrap();
    write_journal(&fixture.paths, &journal).unwrap();
    assert!(fixture.native_review().unwrap().is_none());
    assert_eq!(read_journal(&fixture.paths).unwrap().unwrap(), journal);
    assert!(fixture.platform.transfers.is_empty());
}

#[test]
fn native_funding_review_refreshes_only_unapproved_expired_quotes() {
    let mut fixture = Fixture::native();
    let first = fixture.native_review().unwrap().unwrap();
    fixture.platform.native_balance = 4 + NATIVE_BASE;
    let refreshed = fixture.native_review().unwrap().unwrap();
    assert_ne!(refreshed.review_sha256, first.review_sha256);
    assert_eq!(refreshed.pause.shortfall_cycles(), 101);
    assert!(matches!(
        fixture.native_resume(&first.review_sha256),
        Err(EnsureWorkflowError::NativeFundingRequired { .. })
    ));
    assert!(fixture.platform.transfers.is_empty());
    fixture.platform.native_balance = 101 + NATIVE_BASE;
    assert!(fixture.native_review().unwrap().is_none());
    assert!(
        read_journal(&fixture.paths)
            .unwrap()
            .unwrap()
            .funding_reviews
            .is_empty()
    );
    fixture.platform.native_balance = 4 + NATIVE_BASE;
    let approved = fixture.native_review().unwrap().unwrap();
    fixture.platform.fault = FundingTransportFault::LostReply;
    assert!(matches!(
        fixture.native_resume(&approved.review_sha256),
        Err(EnsureWorkflowError::Platform(_))
    ));
    fixture.platform.native_balance = 1 + NATIVE_BASE;
    let retained = fixture.native_review().unwrap().unwrap();
    assert_eq!(retained.review_sha256, approved.review_sha256);
    assert_eq!(retained.action, approved.action);
    assert!(retained.effect.is_some());
    assert_eq!(fixture.platform.transfers.len(), 1);
}
