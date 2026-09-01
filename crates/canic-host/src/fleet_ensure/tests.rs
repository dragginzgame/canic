use crate::{
    fleet_ensure::{
        model::{
            CanisterDisposition, CanisterRuntimeStatus, CurrentFleetProtocolAction,
            CycleConservation, DesiredCanister, DesiredCanisterInit, DesiredCanisterKind,
            DesiredFleet, DesiredFleetProtocol, DesiredPresence, DesiredProtocolStep,
            DrainAuthority, EffectRecord, EffectState, EnsureAction, FLEET_ENSURE_SCHEMA_VERSION,
            FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan,
            FleetEnsureStateRecord, FleetObservation, LiveCanister,
        },
        ops::{
            EffectObservation, EffectOutcome, EffectRetry, EnsurePlatform, TerminalFleetInventory,
        },
        workflow,
    },
    registry::RegistryEntry,
    test_support::{start_pocket_ic, temp_dir},
};
use candid::Principal;
use canic_control_plane::{
    dto::template::{TemplateChunkInput, TemplateChunkSetPrepareInput},
    ids::{TemplateId, TemplateVersion},
};
use canic_core::{
    cdk::{types::Cycles, utils::hash::sha256_hex},
    dto::{
        component_provisioning::{
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
            FleetComponentProvisioningPrepareRequest,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationRequest, FleetRegistryVersion,
            FleetSubnetRootEntry, FleetSubnetRootJoinRequest, FleetSubnetRootStatus,
        },
        fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest,
        pool::{PoolLedgerRecoveryArtifact, PoolLedgerRecoveryRequest},
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentDeploymentConfigurationDigest, ComponentTopologyDigest,
        CyclesFundingBudget, FleetAdmissionPolicy, FleetBinding, FleetCoordinatorBinding, FleetId,
        FleetKey, FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, FleetSubnetWasmStoreAuthority, ReleaseBuildId,
        ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const TREASURY: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";
const OLD_APP: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
const RETIRED: &str = "r7inp-6aaaa-aaaaa-aaabq-cai";
const OLD_REPLACED: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const SUBNET: &str = "rwlgt-iiaaa-aaaaa-aaaaa-cai";
const CONTROLLER: &str = "rdmx6-jaaaa-aaaaa-aaadq-cai";
const LEDGER: &str = "um5iw-rqaaa-aaaaq-qaaba-cai";

#[derive(Debug, ThisError)]
#[error("simulated lost response")]
struct MockError;

struct MockPlatform {
    completed: BTreeMap<String, EffectOutcome>,
    duplicate_create_responses: BTreeMap<String, u32>,
    create_shortfalls: BTreeMap<String, u128>,
    desired: DesiredFleet,
    fail_once: BTreeSet<String>,
    failed: BTreeSet<String>,
    live: BTreeMap<String, LiveCanister>,
    ledger_fee_cycles: u128,
    mutations: BTreeMap<String, u32>,
    operator_cycles: u128,
    protocol_command_only: bool,
    protocol_action: Option<EnsureAction>,
    protocol_ready: BTreeSet<String>,
    protocol_retry: EffectRetry,
    typed_protocol: bool,
    typed_protocol_burns: Vec<u128>,
    skip_transfer_credit: bool,
    stall_before_mutation: BTreeMap<String, u32>,
    terminal_inventory: TerminalFleetInventory,
    terminal_inventory_expected_operation_id: Option<String>,
    terminal_inventory_operation_ids: Vec<String>,
    version_observation_failures: u8,
}

impl MockPlatform {
    fn new(desired: DesiredFleet, live: impl IntoIterator<Item = LiveCanister>) -> Self {
        let ledger_fee_cycles = desired
            .ledger_fee_cycles
            .parse::<Cycles>()
            .map(|cycles| cycles.to_u128())
            .expect("fixture ledger fee");
        Self {
            completed: BTreeMap::new(),
            duplicate_create_responses: BTreeMap::new(),
            create_shortfalls: BTreeMap::new(),
            desired,
            fail_once: BTreeSet::new(),
            failed: BTreeSet::new(),
            live: live
                .into_iter()
                .map(|canister| (canister.principal.clone(), canister))
                .collect(),
            ledger_fee_cycles,
            mutations: BTreeMap::new(),
            operator_cycles: 100_000,
            protocol_command_only: false,
            protocol_action: None,
            protocol_ready: BTreeSet::new(),
            protocol_retry: EffectRetry::None,
            typed_protocol: false,
            typed_protocol_burns: Vec::new(),
            skip_transfer_credit: false,
            stall_before_mutation: BTreeMap::new(),
            terminal_inventory: TerminalFleetInventory::default(),
            terminal_inventory_expected_operation_id: None,
            terminal_inventory_operation_ids: Vec::new(),
            version_observation_failures: 0,
        }
    }

    fn principal<'a>(
        state: &'a FleetEnsureStateRecord,
        action: &'a EnsureAction,
    ) -> Option<&'a str> {
        let principal = match action {
            EnsureAction::Create { .. } => return None,
            EnsureAction::Delete { principal, .. }
            | EnsureAction::FleetProtocol { principal, .. }
            | EnsureAction::Fund { principal, .. }
            | EnsureAction::Install { principal, .. }
            | EnsureAction::Protocol { principal, .. }
            | EnsureAction::SetControllers { principal, .. }
            | EnsureAction::Start { principal, .. }
            | EnsureAction::Stop { principal, .. }
            | EnsureAction::Transfer { principal, .. } => principal,
        };
        principal.strip_prefix("created:").map_or_else(
            || Some(principal.as_str()),
            |name| state.pending_principals.get(name).map(String::as_str),
        )
    }

    fn canister_principal<'a>(&'a self, state: &'a FleetEnsureStateRecord, name: &str) -> &'a str {
        state
            .pending_principals
            .get(name)
            .or_else(|| state.principals.get(name))
            .or_else(|| {
                self.desired
                    .canisters
                    .iter()
                    .find(|canister| canister.name == name)
                    .and_then(|canister| canister.principal.as_ref())
            })
            .map(String::as_str)
            .expect("fixture canister Principal")
    }

    fn controllers(
        &self,
        state: &FleetEnsureStateRecord,
        controllers: &[String],
        controller_canisters: &[String],
    ) -> Vec<String> {
        let mut resolved = controllers.to_vec();
        resolved.extend(
            controller_canisters
                .iter()
                .map(|name| self.canister_principal(state, name).to_string()),
        );
        resolved.sort();
        resolved.dedup();
        resolved
    }

    fn create_observation(
        &self,
        action: &EnsureAction,
        record: &EffectRecord,
    ) -> Option<EffectObservation> {
        let EnsureAction::Create {
            requested_initial_cycles,
            ..
        } = action
        else {
            return None;
        };
        let post_cycles = record
            .created_principal
            .as_deref()
            .and_then(|principal| self.live.get(principal))
            .map(|live| live.cycles);
        let applied = post_cycles == Some(*requested_initial_cycles);
        Some(EffectObservation {
            applied,
            post_cycles,
            progress_identity: format!("created:{:?}", record.created_principal),
            retry: if post_cycles.is_some() && !applied {
                EffectRetry::ReplanRequiredAfterCreateBalanceDrift
            } else {
                EffectRetry::None
            },
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the deterministic adapter keeps every effect mutation in one exhaustive match"
    )]
    fn mutate(&mut self, action: &EnsureAction, state: &FleetEnsureStateRecord) -> EffectOutcome {
        let principal = Self::principal(state, action).map(str::to_string);
        match action {
            EnsureAction::Create {
                controller_canisters,
                controllers,
                name,
                requested_initial_cycles,
                ..
            } => {
                let creation_fee = self
                    .desired
                    .management_creation_fee_cycles
                    .parse::<Cycles>()
                    .map(|cycles| cycles.to_u128())
                    .expect("creation fee");
                let ledger_fee = self
                    .desired
                    .ledger_fee_cycles
                    .parse::<Cycles>()
                    .map(|cycles| cycles.to_u128())
                    .expect("ledger fee");
                self.operator_cycles -= requested_initial_cycles + creation_fee + ledger_fee;
                let principal = format!("created-{name}");
                let retained_cycles = requested_initial_cycles
                    .checked_sub(self.create_shortfalls.get(name).copied().unwrap_or(0))
                    .expect("fixture creation shortfall is bounded");
                self.live.insert(
                    principal.clone(),
                    LiveCanister {
                        canister_version: Some(1),
                        controllers: self.controllers(state, controllers, controller_canisters),
                        cycles: retained_cycles,
                        module_sha256: None,
                        principal: principal.clone(),
                        reinstall_required: false,
                        root_owned_lifecycle: None,
                        status: CanisterRuntimeStatus::Running,
                    },
                );
                EffectOutcome {
                    created_principal: Some(principal),
                    // The production Cycles Ledger response binds the reviewed
                    // creation amount; live observation independently exposes
                    // any post-effect balance drift.
                    post_cycles: Some(*requested_initial_cycles),
                    receipt: Some("create-block".to_string()),
                }
            }
            EnsureAction::Delete { .. } => {
                self.live
                    .remove(principal.as_deref().expect("delete principal"));
                empty_outcome()
            }
            EnsureAction::Fund { amount, .. } => {
                let ledger_fee = self
                    .desired
                    .ledger_fee_cycles
                    .parse::<Cycles>()
                    .map(|cycles| cycles.to_u128())
                    .expect("ledger fee");
                self.operator_cycles -= amount + ledger_fee;
                let live = self
                    .live
                    .get_mut(principal.as_deref().expect("fund principal"))
                    .expect("fund target");
                live.cycles += amount;
                EffectOutcome {
                    created_principal: None,
                    post_cycles: Some(live.cycles),
                    receipt: Some("withdraw-block".to_string()),
                }
            }
            EnsureAction::Install { wasm_sha256, .. } => {
                let live = self
                    .live
                    .get_mut(principal.as_deref().expect("install principal"))
                    .expect("install target");
                live.module_sha256 = Some(wasm_sha256.clone());
                live.canister_version = Some(
                    live.canister_version
                        .unwrap_or_default()
                        .checked_add(1)
                        .expect("fixture canister version"),
                );
                live.reinstall_required = false;
                empty_outcome()
            }
            EnsureAction::FleetProtocol { name, .. } | EnsureAction::Protocol { name, .. } => {
                if !self.protocol_command_only {
                    self.protocol_ready.insert(name.clone());
                }
                self.protocol_retry = EffectRetry::None;
                EffectOutcome {
                    created_principal: None,
                    post_cycles: None,
                    receipt: Some("protocol-receipt".to_string()),
                }
            }
            EnsureAction::SetControllers {
                controller_canisters,
                controllers,
                ..
            } => {
                let controllers = self.controllers(state, controllers, controller_canisters);
                self.live
                    .get_mut(principal.as_deref().expect("settings principal"))
                    .expect("settings target")
                    .controllers = controllers;
                empty_outcome()
            }
            EnsureAction::Start { .. } => {
                self.live
                    .get_mut(principal.as_deref().expect("start principal"))
                    .expect("start target")
                    .status = CanisterRuntimeStatus::Running;
                empty_outcome()
            }
            EnsureAction::Stop { .. } => {
                self.live
                    .get_mut(principal.as_deref().expect("stop principal"))
                    .expect("stop target")
                    .status = CanisterRuntimeStatus::Stopped;
                empty_outcome()
            }
            EnsureAction::Transfer {
                amount,
                destination,
                maximum_execution_burn_cycles,
                ..
            } => {
                let destination = self.canister_principal(state, destination).to_string();
                let post_cycles = {
                    let source = self
                        .live
                        .get_mut(principal.as_deref().expect("transfer principal"))
                        .expect("transfer source");
                    source.cycles -= amount + maximum_execution_burn_cycles;
                    source.cycles
                };
                if !self.skip_transfer_credit {
                    self.live
                        .get_mut(&destination)
                        .expect("transfer destination")
                        .cycles += amount;
                }
                EffectOutcome {
                    created_principal: None,
                    post_cycles: Some(post_cycles),
                    receipt: Some("drain-receipt".to_string()),
                }
            }
        }
    }
}

impl EnsurePlatform for MockPlatform {
    type Error = MockError;

    fn bind_reviewed_desired(&mut self, desired: &DesiredFleet) -> Result<(), Self::Error> {
        self.desired = desired.clone();
        Ok(())
    }

    fn observe(
        &mut self,
        _operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error> {
        let canisters = self
            .desired
            .canisters
            .iter()
            .map(|configured| {
                let principal = state
                    .pending_principals
                    .get(&configured.name)
                    .or_else(|| state.principals.get(&configured.name))
                    .map(String::as_str)
                    .or(configured.principal.as_deref());
                (
                    configured.name.clone(),
                    principal.and_then(|principal| self.live.get(principal).cloned()),
                )
            })
            .collect();
        Ok(FleetObservation {
            additional_controlled_cycles: BTreeMap::new(),
            canisters,
            ledger_fee_cycles: self.ledger_fee_cycles,
            operator_cycles: self.operator_cycles,
            protocol_ready: self
                .desired
                .protocol_steps
                .iter()
                .map(|step| (step.name.clone(), self.protocol_ready.contains(&step.name)))
                .collect(),
        })
    }

    fn protocol_actions(
        &mut self,
        operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Vec<EnsureAction>, Self::Error> {
        if let Some(action) = self.protocol_action.clone() {
            return Ok((!self.protocol_ready.contains(action.name()))
                .then_some(action)
                .into_iter()
                .collect());
        }
        if !self.typed_protocol_burns.is_empty() {
            return Ok(self
                .typed_protocol_burns
                .iter()
                .enumerate()
                .filter_map(|(index, burn)| {
                    let mut action = typed_protocol_action(operation_id);
                    let EnsureAction::FleetProtocol {
                        maximum_execution_burn_cycles,
                        name,
                        ..
                    } = &mut action
                    else {
                        unreachable!("typed fixture action is FleetProtocol");
                    };
                    *maximum_execution_burn_cycles = *burn;
                    *name = format!("fleet-component-provisioning-{index}");
                    (!self.protocol_ready.contains(name.as_str())).then_some(action)
                })
                .collect());
        }
        if !self.typed_protocol || self.protocol_ready.contains("fleet-component-provisioning") {
            return Ok(Vec::new());
        }
        Ok(vec![typed_protocol_action(operation_id)])
    }

    fn terminal_inventory(
        &mut self,
        operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<TerminalFleetInventory, Self::Error> {
        self.terminal_inventory_operation_ids
            .push(operation_id.to_string());
        if self
            .terminal_inventory_expected_operation_id
            .as_deref()
            .is_some_and(|expected| expected != operation_id)
        {
            return Err(MockError);
        }
        Ok(self.terminal_inventory.clone())
    }

    fn observe_effect(
        &mut self,
        _operation_id: &str,
        action: &EnsureAction,
        record: &EffectRecord,
        state: &FleetEnsureStateRecord,
    ) -> Result<EffectObservation, Self::Error> {
        if let Some(observation) = self.create_observation(action, record) {
            return Ok(observation);
        }
        if matches!(action, EnsureAction::Fund { .. }) {
            return Ok(EffectObservation {
                applied: record.receipt.is_some(),
                post_cycles: Self::principal(state, action)
                    .and_then(|principal| self.live.get(principal))
                    .map(|live| live.cycles),
                progress_identity: format!("fund:{:?}", record.receipt),
                retry: EffectRetry::None,
            });
        }
        let principal = Self::principal(state, action);
        let applied = match action {
            EnsureAction::Delete { .. } => {
                principal.is_none_or(|value| !self.live.contains_key(value))
            }
            EnsureAction::Install {
                mode, wasm_sha256, ..
            } => principal
                .and_then(|value| self.live.get(value))
                .is_some_and(|live| {
                    crate::fleet_ensure::ops::install_effect_applied(
                        *mode,
                        wasm_sha256,
                        live.module_sha256.as_deref(),
                        record.pre_canister_version,
                        live.canister_version,
                    )
                }),
            EnsureAction::FleetProtocol { name, .. } | EnsureAction::Protocol { name, .. } => {
                self.protocol_ready.contains(name)
            }
            EnsureAction::SetControllers {
                controller_canisters,
                controllers,
                ..
            } => {
                let controllers = self.controllers(state, controllers, controller_canisters);
                principal
                    .and_then(|value| self.live.get(value))
                    .is_some_and(|live| live.controllers == controllers)
            }
            EnsureAction::Start { .. } => principal
                .and_then(|value| self.live.get(value))
                .is_some_and(|live| live.status == CanisterRuntimeStatus::Running),
            EnsureAction::Stop { .. } => principal
                .and_then(|value| self.live.get(value))
                .is_some_and(|live| live.status == CanisterRuntimeStatus::Stopped),
            EnsureAction::Transfer {
                amount,
                maximum_execution_burn_cycles,
                destination,
                ..
            } => {
                let destination = self.canister_principal(state, destination);
                let source = principal
                    .and_then(|value| self.live.get(value))
                    .map(|live| live.cycles)
                    .expect("transfer source");
                let destination = self
                    .live
                    .get(destination)
                    .map(|live| live.cycles)
                    .expect("transfer destination");
                let source_debit = record.pre_cycles.expect("source pre") - source;
                let destination_credit =
                    destination - record.destination_pre_cycles.expect("destination pre");
                source_debit >= *amount
                    && source_debit <= amount + maximum_execution_burn_cycles
                    && destination_credit == *amount
            }
            EnsureAction::Create { .. } | EnsureAction::Fund { .. } => false,
        };
        Ok(EffectObservation {
            applied,
            post_cycles: None,
            progress_identity: format!("mock:{action:?}:{applied}"),
            retry: if !applied
                && self.protocol_retry == EffectRetry::ReplayExactIssuedCommand
                && matches!(
                    action,
                    EnsureAction::FleetProtocol { action, .. }
                        if matches!(
                            action.as_ref(),
                            CurrentFleetProtocolAction::ProvisionComponents { .. }
                        )
                ) {
                EffectRetry::ReplayExactIssuedCommand
            } else {
                EffectRetry::None
            },
        })
    }

    fn action_cycles(
        &mut self,
        action: &EnsureAction,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        Ok(Self::principal(state, action)
            .and_then(|principal| self.live.get(principal))
            .map(|live| live.cycles))
    }

    fn action_destination_cycles(
        &mut self,
        action: &EnsureAction,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        let EnsureAction::Transfer { destination, .. } = action else {
            return Ok(None);
        };
        let destination = self.canister_principal(state, destination);
        Ok(self.live.get(destination).map(|live| live.cycles))
    }

    fn action_canister_version(
        &mut self,
        action: &EnsureAction,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<u64>, Self::Error> {
        if matches!(action, EnsureAction::Install { .. }) && self.version_observation_failures > 0 {
            self.version_observation_failures -= 1;
            return Err(MockError);
        }
        Ok(Self::principal(state, action)
            .and_then(|principal| self.live.get(principal))
            .and_then(|live| live.canister_version))
    }

    fn apply(
        &mut self,
        _operation_id: &str,
        action: &EnsureAction,
        _record: &EffectRecord,
        state: &FleetEnsureStateRecord,
    ) -> Result<EffectOutcome, Self::Error> {
        let hash = crate::fleet_ensure::ops::action_sha256(action);
        if let Some(outcome) = self.completed.get(&hash) {
            if matches!(action, EnsureAction::Create { .. }) {
                *self.duplicate_create_responses.entry(hash).or_default() += 1;
            }
            return Ok(outcome.clone());
        }
        if let Some(remaining) = self.stall_before_mutation.get_mut(&hash)
            && *remaining > 0
        {
            *remaining -= 1;
            return Err(MockError);
        }
        let outcome = self.mutate(action, state);
        *self.mutations.entry(hash.clone()).or_default() += 1;
        self.completed.insert(hash.clone(), outcome.clone());
        if self.fail_once.contains(&hash) && self.failed.insert(hash) {
            return Err(MockError);
        }
        Ok(outcome)
    }
}

#[test]
fn successful_create_response_retains_exact_applied_evidence_and_replays_without_effect() {
    assert_create_response_journey(false);
}

#[test]
fn duplicate_create_response_with_principal_retains_exact_applied_evidence_and_replays_without_effect()
 {
    assert_create_response_journey(true);
}

fn assert_create_response_journey(response_is_duplicate: bool) {
    let mut fixture = fixture();
    let desired_sha256 = "102".repeat(21) + "1";
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("plan production-shaped Create journey");
    let create = workflow::ordered_actions(&planned.plan)
        .into_iter()
        .find(|action| matches!(action, EnsureAction::Create { name, .. } if name == "created"))
        .expect("reviewed Create action");
    let create_hash = crate::fleet_ensure::ops::action_sha256(create);
    let EnsureAction::Create {
        requested_initial_cycles,
        ..
    } = create
    else {
        unreachable!("selected action is Create");
    };
    let requested_initial_cycles = *requested_initial_cycles;
    if response_is_duplicate {
        fixture.platform.fail_once.insert(create_hash.clone());
        let error = workflow::apply(
            &fixture.root,
            &fixture.desired,
            &desired_sha256,
            "test-fleet",
            &planned.plan.plan_sha256,
            &mut fixture.platform,
        )
        .expect_err("first Create response is lost after the Ledger effect");
        assert!(matches!(error, workflow::EnsureWorkflowError::Platform(_)));
    }

    let applied = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("apply successful or duplicate-with-Principal Create response");
    assert!(applied.terminal);
    assert_eq!(fixture.platform.mutations.get(&create_hash), Some(&1));
    assert_eq!(
        fixture
            .platform
            .duplicate_create_responses
            .get(&create_hash)
            .copied()
            .unwrap_or_default(),
        u32::from(response_is_duplicate)
    );

    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    let journal = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read Create journal")
        .expect("retained Create journal");
    let effect = journal
        .effects
        .iter()
        .find(|effect| effect.action_sha256 == create_hash)
        .expect("retained Create effect");
    assert_eq!(effect.state, EffectState::Applied);
    assert_eq!(effect.created_principal.as_deref(), Some("created-created"));
    assert_eq!(effect.receipt.as_deref(), Some("create-block"));
    assert_eq!(effect.post_cycles, Some(requested_initial_cycles));
    assert_retained_create_balance(&paths, requested_initial_cycles);

    let mutations = fixture.platform.mutations.clone();
    let replay_plan = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut fixture.platform,
    )
    .expect("plan terminal Create replay");
    assert!(workflow::ordered_actions(&replay_plan.plan).is_empty());
    let replay = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &replay_plan.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("apply effect-free Create replay");
    assert!(replay.terminal);
    assert_eq!(replay.effects_applied, 0);
    assert_eq!(fixture.platform.mutations, mutations);

    fs::remove_dir_all(fixture.root).expect("remove test directory");
}

#[test]
fn retained_0_109_32_create_balance_recovers_only_from_the_exact_duplicate_response() {
    let (mut fixture, desired_sha256, plan, create_hash, requested_initial_cycles) =
        completed_create_journey();
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    let mut journal = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read completed Create journal")
        .expect("completed Create journal");
    journal.completion = FleetEnsureCompletion::InProgress;
    journal
        .effects
        .iter_mut()
        .find(|effect| effect.action_sha256 == create_hash)
        .expect("retained Create effect")
        .post_cycles = None;
    crate::fleet_ensure::ops::write_journal(&paths, &journal)
        .expect("retain exact 0.109.32 missing-balance shape");
    let mutations = fixture.platform.mutations.clone();

    let recovered = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("recover exact retained Create evidence");
    assert!(recovered.terminal);
    assert_eq!(fixture.platform.mutations, mutations);
    assert_eq!(
        fixture
            .platform
            .duplicate_create_responses
            .get(&create_hash),
        Some(&1)
    );
    let journal = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read recovered Create journal")
        .expect("recovered Create journal");
    let effect = journal
        .effects
        .iter()
        .find(|effect| effect.action_sha256 == create_hash)
        .expect("recovered Create effect");
    assert_eq!(effect.state, EffectState::Applied);
    assert_eq!(effect.post_cycles, Some(requested_initial_cycles));
    assert_retained_create_balance(&paths, requested_initial_cycles);

    fs::remove_dir_all(fixture.root).expect("remove test directory");
}

#[test]
fn terminal_create_publication_rejects_a_conflicting_retained_balance() {
    let (mut fixture, desired_sha256, plan, create_hash, requested_initial_cycles) =
        completed_create_journey();
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    let mut journal = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read completed Create journal")
        .expect("completed Create journal");
    journal.completion = FleetEnsureCompletion::InProgress;
    crate::fleet_ensure::ops::write_journal(&paths, &journal)
        .expect("reopen exact Create operation");
    let mut state = crate::fleet_ensure::ops::read_state(&paths, "test-fleet")
        .expect("read completed Create state");
    state
        .retained_cycles_by_principal
        .insert("created-created".to_string(), requested_initial_cycles - 1);
    crate::fleet_ensure::ops::write_state(&paths, &state)
        .expect("retain conflicting Create balance");
    let mutations = fixture.platform.mutations.clone();

    let error = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect_err("conflicting retained Create balance must fail closed");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::JournalIntegrity
    ));
    assert_eq!(fixture.platform.mutations, mutations);
    let retained_state = crate::fleet_ensure::ops::read_state(&paths, "test-fleet")
        .expect("read rejected Create state");
    assert_eq!(retained_state, state);
    let retained_journal = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read rejected Create journal")
        .expect("rejected Create journal");
    assert_eq!(retained_journal, journal);
    assert!(
        retained_journal
            .effects
            .iter()
            .any(|effect| effect.action_sha256 == create_hash)
    );

    fs::remove_dir_all(fixture.root).expect("remove test directory");
}

fn assert_retained_create_balance(paths: &crate::fleet_ensure::ops::EnsurePaths, expected: u128) {
    let state = crate::fleet_ensure::ops::read_state(paths, "test-fleet")
        .expect("read terminal Create state");
    assert_eq!(
        state.retained_cycles_by_principal.get("created-created"),
        Some(&expected)
    );
}

#[test]
fn retained_0_109_32_create_balance_recovery_rejects_every_authority_mismatch() {
    #[derive(Clone, Copy, Debug)]
    enum Mismatch {
        Action,
        Operation,
        Plan,
        Principal,
        Receipt,
        RequestedBalance,
    }

    for mismatch in [
        Mismatch::Action,
        Mismatch::Operation,
        Mismatch::Plan,
        Mismatch::Principal,
        Mismatch::Receipt,
        Mismatch::RequestedBalance,
    ] {
        let (mut fixture, desired_sha256, plan, create_hash, requested_initial_cycles) =
            completed_create_journey();
        let paths = crate::fleet_ensure::ops::EnsurePaths::under(
            &fixture.root,
            &fixture.desired.environment,
            "test-fleet",
        );
        let mut journal = crate::fleet_ensure::ops::read_journal(&paths)
            .expect("read completed Create journal")
            .expect("completed Create journal");
        journal.completion = FleetEnsureCompletion::InProgress;
        let effect = journal
            .effects
            .iter_mut()
            .find(|effect| effect.action_sha256 == create_hash)
            .expect("retained Create effect");
        effect.post_cycles = None;
        match mismatch {
            Mismatch::Action => effect.action_sha256 = "wrong-action".to_string(),
            Mismatch::Operation => journal.operation_id = "wrong-operation".to_string(),
            Mismatch::Plan => journal.plan_sha256 = "wrong-plan".to_string(),
            Mismatch::Principal => {
                fixture
                    .platform
                    .completed
                    .get_mut(&create_hash)
                    .expect("retained Create response")
                    .created_principal = Some("different-principal".to_string());
            }
            Mismatch::Receipt => {
                fixture
                    .platform
                    .completed
                    .get_mut(&create_hash)
                    .expect("retained Create response")
                    .receipt = Some("different-receipt".to_string());
            }
            Mismatch::RequestedBalance => {
                fixture
                    .platform
                    .completed
                    .get_mut(&create_hash)
                    .expect("retained Create response")
                    .post_cycles = Some(requested_initial_cycles - 1);
            }
        }
        crate::fleet_ensure::ops::write_journal(&paths, &journal)
            .expect("retain mismatched Create recovery shape");
        let retained_journal = fs::read(&paths.journal).expect("read mismatched journal bytes");
        let mutations = fixture.platform.mutations.clone();

        let error = workflow::apply(
            &fixture.root,
            &fixture.desired,
            &desired_sha256,
            "test-fleet",
            &plan.plan_sha256,
            &mut fixture.platform,
        )
        .expect_err("mismatched Create recovery authority must reject");
        assert!(
            matches!(
                error,
                workflow::EnsureWorkflowError::JournalIntegrity
                    | workflow::EnsureWorkflowError::DriftedBeforeApply
            ),
            "unexpected {mismatch:?} error: {error:?}"
        );
        assert_eq!(fixture.platform.mutations, mutations);
        assert_eq!(
            fs::read(&paths.journal).expect("reread mismatched journal bytes"),
            retained_journal
        );
        fs::remove_dir_all(fixture.root).expect("remove test directory");
    }
}

fn completed_create_journey() -> (Fixture, String, FleetEnsurePlan, String, u128) {
    let mut fixture = fixture();
    let desired_sha256 = "102".repeat(21) + "2";
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("plan retained Create journey");
    let create = workflow::ordered_actions(&planned.plan)
        .into_iter()
        .find(|action| matches!(action, EnsureAction::Create { name, .. } if name == "created"))
        .expect("reviewed Create action");
    let create_hash = crate::fleet_ensure::ops::action_sha256(create);
    let EnsureAction::Create {
        requested_initial_cycles,
        ..
    } = create
    else {
        unreachable!("selected action is Create");
    };
    let requested_initial_cycles = *requested_initial_cycles;
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("complete retained Create journey");
    (
        fixture,
        desired_sha256,
        planned.plan,
        create_hash,
        requested_initial_cycles,
    )
}

#[test]
fn interruption_at_every_effect_converges_once_and_second_run_has_zero_effects() {
    let mut fixture = fixture();
    fixture
        .desired
        .protocol_steps
        .push(protocol_step(&fixture.root));
    fixture.platform.desired = fixture.desired.clone();
    let root = fixture.root.clone();
    let desired_sha256 = "a".repeat(64);
    let mut platform = fixture.platform;
    let planned = workflow::plan(
        &root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile reviewed plan");
    let actions = planned
        .plan
        .canisters
        .iter()
        .flat_map(|canister| canister.actions.iter())
        .chain(planned.plan.protocol_actions.iter())
        .collect::<Vec<_>>();
    assert_eq!(actions.len(), 15);
    platform.fail_once = actions
        .iter()
        .map(|action| crate::fleet_ensure::ops::action_sha256(action))
        .collect();

    let report = loop {
        match workflow::apply(
            &root,
            &fixture.desired,
            &desired_sha256,
            "test-fleet",
            &planned.plan.plan_sha256,
            &mut platform,
        ) {
            Ok(report) => break report,
            Err(workflow::EnsureWorkflowError::Platform(MockError)) => {}
            Err(error) => panic!("unexpected resume result: {error}"),
        }
    };
    assert!(report.terminal);
    let inventory = crate::fleet_ensure::read_current_fleet_inventory(
        &root,
        &fixture.desired.environment,
        "test-fleet",
    )
    .expect("terminal ensure inventory");
    assert_eq!(inventory.entries.len(), 4);
    let conservation = report
        .actual_conservation
        .expect("terminal conservation proof");
    assert_eq!(
        conservation.observed_starting_cycles + conservation.received_new_funding_cycles
            - conservation.measured_execution_burn_cycles,
        conservation.final_controlled_cycles
    );
    assert!(platform.mutations.values().all(|count| *count == 1));
    let mutation_count = platform.mutations.values().sum::<u32>();

    let no_effect_plan = workflow::plan(
        &root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut platform,
    )
    .expect("plan converged Fleet");
    assert!(
        no_effect_plan
            .plan
            .canisters
            .iter()
            .all(|canister| canister.actions.is_empty())
    );
    assert!(no_effect_plan.plan.protocol_actions.is_empty());
    let terminal = workflow::apply(
        &root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &no_effect_plan.plan.plan_sha256,
        &mut platform,
    )
    .expect("effect-free terminal replay");
    assert!(terminal.terminal);
    assert_eq!(platform.mutations.values().sum::<u32>(), mutation_count);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one proof keeps the retained thirteen-applied/one-issued journal and replay assertions visible"
)]
fn retryable_provisioning_failure_replays_only_the_exact_retained_issued_command() {
    let mut fixture = fixture();
    fixture.desired.protocol = Some(DesiredFleetProtocol {
        app_config: "canic.toml".to_string(),
        component_group_placements: Vec::new(),
        coordinator_candid: "coordinator.did".to_string(),
        root_candid: "root.did".to_string(),
        store_candid: "store.did".to_string(),
    });
    fixture
        .desired
        .canisters
        .iter_mut()
        .find(|canister| canister.kind == DesiredCanisterKind::Coordinator)
        .expect("fixture Coordinator")
        .wasm = None;
    fixture.platform.desired = fixture.desired.clone();
    let desired_sha256 = "45".repeat(32);
    let initial = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("initial plan");
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &initial.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("initial convergence");

    fixture.platform.completed.clear();
    fixture.platform.mutations.clear();
    fixture.platform.typed_protocol = true;
    let pending = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut fixture.platform,
    )
    .expect("typed provisioning plan");
    assert!(
        pending
            .plan
            .canisters
            .iter()
            .all(|canister| canister.actions.is_empty())
    );
    let [issued] = pending.plan.protocol_actions.as_slice() else {
        panic!("expected one typed provisioning action");
    };
    let issued = issued.clone();
    let mut retained = pending.plan;
    retained.protocol_actions.clear();
    for index in 0..13 {
        let mut applied = issued.clone();
        let EnsureAction::FleetProtocol { name, .. } = &mut applied else {
            panic!("typed provisioning action");
        };
        *name = format!("applied-{index:02}");
        retained.protocol_actions.push(applied);
    }
    retained.protocol_actions.push(issued.clone());
    retained.plan_sha256 = crate::fleet_ensure::policy::expected_plan_sha256(&retained);

    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    crate::fleet_ensure::ops::write_plan(&paths, &retained).expect("write retained plan");
    let effects = retained
        .protocol_actions
        .iter()
        .enumerate()
        .map(|(index, action)| EffectRecord {
            action_sha256: crate::fleet_ensure::ops::action_sha256(action),
            created_principal: None,
            destination_post_cycles: None,
            destination_pre_cycles: None,
            post_cycles: None,
            pre_cycles: None,
            pre_canister_version: None,
            progress_identity: Some(format!("retained-{index}")),
            receipt: Some("protocol-receipt".to_string()),
            state: if index == 13 {
                EffectState::Issued
            } else {
                EffectState::Applied
            },
        })
        .collect();
    crate::fleet_ensure::ops::write_journal(
        &paths,
        &FleetEnsureJournalRecord {
            completion: FleetEnsureCompletion::InProgress,
            effects,
            fleet: "test-fleet".to_string(),
            initial_controlled_cycles: retained.conservation.observed_controlled_cycles,
            initial_operator_cycles: fixture.platform.operator_cycles,
            operation_id: retained.operation_id.clone(),
            plan_sha256: retained.plan_sha256.clone(),
            schema_version: FLEET_ENSURE_SCHEMA_VERSION,
            stalled_observations: 0,
        },
    )
    .expect("write retained issued journal");
    fixture.platform.protocol_retry = EffectRetry::ReplayExactIssuedCommand;
    let identities = fixture
        .platform
        .live
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let cycles = fixture
        .platform
        .live
        .iter()
        .map(|(principal, live)| (principal.clone(), live.cycles))
        .collect::<BTreeMap<_, _>>();
    let operator_cycles = fixture.platform.operator_cycles;

    let report = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &retained.plan_sha256,
        &mut fixture.platform,
    )
    .expect("resume retained issued provisioning");

    assert!(report.terminal);
    let issued_hash = crate::fleet_ensure::ops::action_sha256(&issued);
    assert_eq!(fixture.platform.mutations.get(&issued_hash), Some(&1));
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 1);
    assert_eq!(
        fixture
            .platform
            .live
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        identities
    );
    assert_eq!(
        fixture
            .platform
            .live
            .iter()
            .map(|(principal, live)| (principal.clone(), live.cycles))
            .collect::<BTreeMap<_, _>>(),
        cycles
    );
    assert_eq!(fixture.platform.operator_cycles, operator_cycles);
    let journal = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read terminal journal")
        .expect("terminal journal");
    assert_eq!(journal.effects.len(), 14);
    assert!(
        journal
            .effects
            .iter()
            .all(|effect| effect.state == EffectState::Applied)
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one proof keeps same-ID Start, cycle conservation, and effect-free replay together"
)]
fn stopped_retained_coordinator_starts_same_id_then_replays_without_effect() {
    let mut fixture = fixture();
    let desired_sha256 = "46".repeat(32);
    let initial = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("initial plan");
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &initial.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("initial convergence");
    fixture.platform.completed.clear();
    fixture.platform.mutations.clear();
    fixture
        .platform
        .live
        .get_mut(TREASURY)
        .expect("retained Coordinator")
        .status = CanisterRuntimeStatus::Stopped;
    let identities = fixture
        .platform
        .live
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let cycles = fixture
        .platform
        .live
        .iter()
        .map(|(principal, live)| (principal.clone(), live.cycles))
        .collect::<BTreeMap<_, _>>();

    let start = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut fixture.platform,
    )
    .expect("same-ID Start plan");
    let actions = workflow::ordered_actions(&start.plan);
    let [action] = actions.as_slice() else {
        panic!("expected one same-ID Start action");
    };
    assert!(matches!(
        action,
        EnsureAction::Start { principal, .. } if principal == TREASURY
    ));
    let action_hash = crate::fleet_ensure::ops::action_sha256(action);
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &start.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("same-ID Start convergence");
    assert_eq!(fixture.platform.mutations.get(&action_hash), Some(&1));
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 1);
    assert_eq!(
        fixture
            .platform
            .live
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        identities
    );
    assert_eq!(
        fixture
            .platform
            .live
            .iter()
            .map(|(principal, live)| (principal.clone(), live.cycles))
            .collect::<BTreeMap<_, _>>(),
        cycles
    );

    let replay = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_200,
        &mut fixture.platform,
    )
    .expect("effect-free replay plan");
    assert!(workflow::ordered_actions(&replay.plan).is_empty());
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &replay.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("effect-free replay");
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 1);
}

#[test]
fn conservation_equation_accounts_for_funding_fees_and_burn_separately() {
    let fixture = fixture();
    let mut platform = fixture.platform;
    let report = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"b".repeat(64),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile plan");
    let proof = report.plan.conservation;
    assert_eq!(
        proof.observed_controlled_cycles + proof.maximum_operator_debit_cycles
            - proof.maximum_unavoidable_fee_cycles
            - proof.maximum_execution_burn_cycles,
        proof.expected_post_operation_cycles
    );
    assert!(proof.scheduled_transfer_cycles > 0);
    assert!(proof.retained_in_reused_canisters_cycles > 0);
    assert!(proof.maximum_new_funding_cycles > 0);
}

#[test]
fn pool_ledger_recovery_counts_controlled_ledger_cycles_and_fee_before_conversion() {
    let mut baseline = fixture();
    baseline.desired.protocol = Some(DesiredFleetProtocol {
        app_config: "canic.toml".to_string(),
        component_group_placements: Vec::new(),
        coordinator_candid: "coordinator.did".to_string(),
        root_candid: "root.did".to_string(),
        store_candid: "store.did".to_string(),
    });
    let root_canister = baseline
        .desired
        .canisters
        .iter_mut()
        .find(|canister| canister.name == "app")
        .expect("fixture Root");
    root_canister.kind = DesiredCanisterKind::Root;
    root_canister.parent = Some("treasury".to_string());
    for canister in &mut baseline.desired.canisters {
        if matches!(
            canister.kind,
            DesiredCanisterKind::Coordinator | DesiredCanisterKind::Root
        ) {
            canister.wasm = None;
        }
    }
    baseline.platform.desired = baseline.desired.clone();
    let baseline_plan = workflow::plan(
        &baseline.root,
        &baseline.desired,
        &"74".repeat(32),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut baseline.platform,
    )
    .expect("compile baseline plan")
    .plan;

    let mut recovery = fixture();
    recovery.desired = baseline.desired;
    recovery.platform.desired = recovery.desired.clone();
    recovery.platform.protocol_action = Some(pool_ledger_recovery_action());
    let recovery_plan = workflow::plan(
        &recovery.root,
        &recovery.desired,
        &"75".repeat(32),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut recovery.platform,
    )
    .expect("compile pool Ledger recovery plan")
    .plan;

    assert_eq!(
        recovery_plan.conservation.observed_controlled_cycles,
        baseline_plan.conservation.observed_controlled_cycles + 30
    );
    assert_eq!(
        recovery_plan.conservation.scheduled_transfer_cycles,
        baseline_plan.conservation.scheduled_transfer_cycles + 20
    );
    assert_eq!(
        recovery_plan.conservation.maximum_execution_burn_cycles,
        baseline_plan.conservation.maximum_execution_burn_cycles + 11
    );
    assert_eq!(
        recovery_plan.conservation.expected_post_operation_cycles,
        baseline_plan.conservation.expected_post_operation_cycles + 19
    );
    assert_eq!(
        recovery_plan.conservation.maximum_operator_debit_cycles,
        baseline_plan.conservation.maximum_operator_debit_cycles
    );
}

#[test]
fn infrastructure_install_order_keeps_store_before_root_initialization() {
    let coordinator = install_action(
        "coordinator",
        DesiredCanisterInit::Coordinator,
        crate::fleet_ensure::model::InstallMode::Reinstall,
    );
    let store = install_action(
        "store",
        DesiredCanisterInit::Store {
            root: "root".to_string(),
        },
        crate::fleet_ensure::model::InstallMode::Reinstall,
    );
    let root = install_action(
        "root",
        DesiredCanisterInit::Root {
            root: "root".to_string(),
        },
        crate::fleet_ensure::model::InstallMode::Reinstall,
    );

    assert!(workflow::action_order(&coordinator) < workflow::action_order(&store));
    assert!(workflow::action_order(&store) < workflow::action_order(&root));
}

#[test]
fn active_registry_is_retired_only_after_every_infrastructure_reinstall_is_applied() {
    let mut fixture = fixture();
    let mut plan = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"70".repeat(32),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("compile fixture plan")
    .plan;
    fixture.desired.protocol = Some(DesiredFleetProtocol {
        app_config: "canic.toml".to_string(),
        component_group_placements: Vec::new(),
        coordinator_candid: "coordinator.did".to_string(),
        root_candid: "root.did".to_string(),
        store_candid: "store.did".to_string(),
    });
    let roles = [
        (
            "treasury",
            DesiredCanisterKind::Coordinator,
            DesiredCanisterInit::Coordinator,
        ),
        (
            "app",
            DesiredCanisterKind::Root,
            DesiredCanisterInit::Root {
                root: "app".to_string(),
            },
        ),
        (
            "created",
            DesiredCanisterKind::Store,
            DesiredCanisterInit::Store {
                root: "app".to_string(),
            },
        ),
    ];
    let mut planned = Vec::new();
    let mut effects = Vec::new();
    for (name, kind, init) in roles {
        fixture
            .desired
            .canisters
            .iter_mut()
            .find(|canister| canister.name == name)
            .expect("fixture infrastructure role")
            .kind = kind;
        let action = install_action(
            name,
            init,
            crate::fleet_ensure::model::InstallMode::Reinstall,
        );
        effects.push(EffectRecord {
            action_sha256: crate::fleet_ensure::ops::action_sha256(&action),
            created_principal: None,
            destination_post_cycles: None,
            destination_pre_cycles: None,
            post_cycles: None,
            pre_cycles: Some(1),
            pre_canister_version: Some(1),
            progress_identity: Some(format!("applied-{name}")),
            receipt: None,
            state: EffectState::Applied,
        });
        planned.push(crate::fleet_ensure::model::CanisterPlan {
            actions: vec![action],
            disposition: CanisterDisposition::Reinstall,
            name: name.to_string(),
            observed_cycles: 1,
            principal: Some(Principal::from_slice(&[99; 29]).to_text()),
        });
    }
    plan.canisters = planned;
    plan.protocol_actions.clear();
    let mut journal = FleetEnsureJournalRecord {
        completion: FleetEnsureCompletion::InProgress,
        effects,
        fleet: plan.fleet.clone(),
        initial_controlled_cycles: 3,
        initial_operator_cycles: 0,
        operation_id: plan.operation_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        stalled_observations: 0,
    };

    assert!(workflow::completed_infrastructure_reinstall(
        &fixture.desired,
        &plan,
        &journal
    ));
    journal.effects[2].state = EffectState::Issued;
    assert!(!workflow::completed_infrastructure_reinstall(
        &fixture.desired,
        &plan,
        &journal
    ));
}

#[test]
fn same_module_reinstall_requires_a_newer_canister_version() {
    use crate::fleet_ensure::{model::InstallMode, ops::install_effect_applied};

    assert!(!install_effect_applied(
        InstallMode::Reinstall,
        "same",
        Some("same"),
        Some(7),
        Some(7),
    ));
    assert!(install_effect_applied(
        InstallMode::Reinstall,
        "same",
        Some("same"),
        Some(7),
        Some(8),
    ));
    assert!(!install_effect_applied(
        InstallMode::Reinstall,
        "same",
        Some("different"),
        Some(7),
        Some(8),
    ));
}

#[test]
fn ledger_withdraw_completion_uses_the_reviewed_burn_aware_lower_bound() {
    use crate::fleet_ensure::ops::{NativeFundingObservation, native_funding_applied};

    let observation = NativeFundingObservation {
        amount: 1_000_000_310_113,
        expected_post_cycles: 2_900_000_000_000,
        funding_deficit_cycles: 310_113,
        funding_margin_cycles: 1_000_000_000_000,
        live_cycles: Some(2_898_749_313_788),
        pre_cycles: Some(1_899_999_689_887),
    };
    assert!(native_funding_applied(observation));
    assert!(!native_funding_applied(NativeFundingObservation {
        live_cycles: Some(1_899_999_689_887),
        ..observation
    }));
    assert!(!native_funding_applied(NativeFundingObservation {
        live_cycles: None,
        ..observation
    }));
    assert!(!native_funding_applied(NativeFundingObservation {
        expected_post_cycles: 2_900_000_000_001,
        ..observation
    }));
}

#[test]
fn issued_funding_reconciles_burn_and_replays_without_a_second_withdrawal() {
    let mut fixture = fixture();
    let desired_sha256 = "116".repeat(21) + "1";
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("plan funding reconciliation journey");
    let funding = workflow::ordered_actions(&planned.plan)
        .into_iter()
        .find(|action| matches!(action, EnsureAction::Fund { name, .. } if name == "app"))
        .expect("reviewed App funding action");
    let funding_hash = crate::fleet_ensure::ops::action_sha256(funding);
    let EnsureAction::Fund {
        expected_post_cycles,
        funding_margin_cycles,
        principal,
        ..
    } = funding
    else {
        unreachable!("selected action is Fund");
    };
    assert!(*funding_margin_cycles > 1);
    fixture.platform.fail_once.insert(funding_hash.clone());

    let first = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect_err("lose the first Ledger response after its exact effect");
    assert!(matches!(first, workflow::EnsureWorkflowError::Platform(_)));
    let live = fixture
        .platform
        .live
        .get_mut(principal)
        .expect("funded App remains controlled");
    live.cycles = expected_post_cycles - 1;

    let resumed = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("adopt exact duplicate receipt and burn-aware live balance");
    assert!(resumed.terminal);
    assert_eq!(fixture.platform.mutations.get(&funding_hash), Some(&1));

    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    let journal = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read funding journal")
        .expect("retained funding journal");
    let effect = journal
        .effects
        .iter()
        .find(|effect| effect.action_sha256 == funding_hash)
        .expect("retained funding effect");
    assert_eq!(effect.state, EffectState::Applied);
    assert_eq!(effect.receipt.as_deref(), Some("withdraw-block"));
    assert_eq!(effect.post_cycles, Some(expected_post_cycles - 1));
    let state =
        crate::fleet_ensure::ops::read_state(&paths, "test-fleet").expect("read funding state");
    assert_eq!(
        state.retained_cycles_by_principal.get(principal),
        Some(&(expected_post_cycles - 1))
    );

    let mutations = fixture.platform.mutations.clone();
    let replay_plan = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut fixture.platform,
    )
    .expect("plan effect-free funding replay");
    assert!(workflow::ordered_actions(&replay_plan.plan).is_empty());
    let replay = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &replay_plan.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("terminal funding replay");
    assert!(replay.terminal);
    assert_eq!(fixture.platform.mutations, mutations);

    fs::remove_dir_all(fixture.root).expect("remove test directory");
}

#[test]
fn same_module_reinstall_runs_once_and_replay_is_effect_free() {
    let mut fixture = fixture();
    fixture.desired.canisters.truncate(1);
    fixture.platform.desired = fixture.desired.clone();
    let treasury = fixture
        .platform
        .live
        .get_mut(TREASURY)
        .expect("live Coordinator");
    treasury.reinstall_required = true;
    let before = treasury.canister_version;
    let desired_sha256 = "7".repeat(64);

    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("plan same-module reinstall");
    assert!(matches!(
        workflow::ordered_actions(&planned.plan).as_slice(),
        [EnsureAction::Install {
            mode: crate::fleet_ensure::model::InstallMode::Reinstall,
            ..
        }]
    ));
    fixture.platform.version_observation_failures = 1;
    assert!(matches!(
        workflow::apply(
            &fixture.root,
            &fixture.desired,
            &desired_sha256,
            "test-fleet",
            &planned.plan.plan_sha256,
            &mut fixture.platform,
        ),
        Err(workflow::EnsureWorkflowError::Platform(MockError))
    ));
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    let interrupted = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read interrupted reinstall journal")
        .expect("interrupted reinstall journal");
    assert_eq!(interrupted.plan_sha256, planned.plan.plan_sha256);
    assert!(interrupted.effects.is_empty());
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 0);

    let applied = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("apply same-module reinstall");
    assert!(applied.terminal);
    assert_eq!(applied.effects_applied, 1);
    let completed = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read completed reinstall journal")
        .expect("completed reinstall journal");
    assert_eq!(completed.effects.len(), 1);
    assert_eq!(completed.effects[0].pre_canister_version, before);
    assert_eq!(completed.effects[0].state, EffectState::Applied);
    assert!(
        fixture
            .platform
            .live
            .get(TREASURY)
            .and_then(|live| live.canister_version)
            > before
    );

    let replay_plan = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_001,
        &mut fixture.platform,
    )
    .expect("plan terminal replay");
    assert!(workflow::ordered_actions(&replay_plan.plan).is_empty());
    let replay = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &replay_plan.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("apply terminal replay");
    assert!(replay.terminal);
    assert_eq!(replay.effects_applied, 0);
}

#[test]
fn reviewed_plan_accepts_bounded_bidirectional_balance_movement_with_truthful_start() {
    let mut fixture = fixture();
    let desired_sha256 = "76".repeat(32);
    let initial = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("plan initial convergence");
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &initial.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("apply initial convergence");

    fixture.desired.maximum_observation_burn_cycles = "10".to_string();
    fixture.platform.desired = fixture.desired.clone();
    fixture.platform.mutations.clear();
    let reviewed = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut fixture.platform,
    )
    .expect("plan effect-free replay");
    assert!(workflow::ordered_actions(&reviewed.plan).is_empty());
    let reviewed_cycles = reviewed.plan.conservation.observed_controlled_cycles;
    fixture
        .platform
        .live
        .get_mut(TREASURY)
        .expect("retained Coordinator")
        .cycles -= 4;
    fixture
        .platform
        .live
        .get_mut(OLD_APP)
        .expect("retained App")
        .cycles += 3;

    let applied = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &reviewed.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("accept bounded decrease and refund");
    assert!(applied.terminal);
    assert_eq!(applied.effects_applied, 0);
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 0);
    assert_eq!(
        applied
            .actual_conservation
            .expect("terminal conservation")
            .observed_starting_cycles,
        reviewed_cycles - 1
    );

    let replay = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_200,
        &mut fixture.platform,
    )
    .expect("replan terminal state");
    assert!(workflow::ordered_actions(&replay.plan).is_empty());
}

#[test]
fn reviewed_plan_rejects_balance_movement_beyond_its_bound_before_effects() {
    let mut fixture = fixture();
    let desired_sha256 = "77".repeat(32);
    let initial = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("plan initial convergence");
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &initial.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("apply initial convergence");
    fixture.desired.maximum_observation_burn_cycles = "10".to_string();
    fixture.platform.desired = fixture.desired.clone();
    fixture.platform.mutations.clear();
    let reviewed = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut fixture.platform,
    )
    .expect("plan bounded estate");
    fixture
        .platform
        .live
        .get_mut(TREASURY)
        .expect("retained Coordinator")
        .cycles += 11;

    let error = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        "test-fleet",
        &reviewed.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect_err("reject movement beyond reviewed bound");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::DriftedBeforeApply
    ));
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 0);
}

#[test]
fn funding_margin_is_bounded_by_the_target_observation_only() {
    let mut fixture = fixture();
    fixture.desired.maximum_observation_burn_cycles = "10".to_string();
    fixture.platform.desired = fixture.desired.clone();

    let report = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"3".repeat(64),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("compile target-bounded funding plan");
    let funding = report
        .plan
        .canisters
        .iter()
        .find(|canister| canister.name == "app")
        .and_then(|canister| {
            canister.actions.iter().find_map(|action| match action {
                EnsureAction::Fund {
                    amount,
                    expected_post_cycles,
                    funding_deficit_cycles,
                    funding_margin_cycles,
                    ..
                } => Some((
                    *amount,
                    *expected_post_cycles,
                    *funding_deficit_cycles,
                    *funding_margin_cycles,
                )),
                _ => None,
            })
        })
        .expect("underfunded App has one funding action");

    assert_eq!(funding, (28, 33, 15, 13));
}

#[test]
fn live_ledger_fee_drift_rejects_before_intent_or_effect() {
    let mut fixture = fixture();
    fixture.platform.ledger_fee_cycles += 1;
    let error = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"9".repeat(64),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect_err("changed live Ledger fee requires a new desired plan");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Policy(
            crate::fleet_ensure::policy::EnsurePolicyError::LedgerFeeDrift { .. }
        )
    ));
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 0);
}

#[test]
fn missing_exact_retained_identity_rejects_instead_of_creating_replacement() {
    let mut fixture = fixture();
    fixture.platform.live.remove(OLD_APP);
    let error = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"8".repeat(64),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect_err("missing seeded identity must fail closed");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Policy(
            crate::fleet_ensure::policy::EnsurePolicyError::MissingObservation { name }
        ) if name == "app"
    ));
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 0);
}

#[test]
fn anonymous_operator_and_treasury_replacement_reject_before_effects() {
    let mut anonymous = fixture();
    anonymous.desired.operator = Principal::anonymous().to_text();
    anonymous.platform.desired = anonymous.desired.clone();
    let error = workflow::plan(
        &anonymous.root,
        &anonymous.desired,
        &"7".repeat(64),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut anonymous.platform,
    )
    .expect_err("anonymous operator rejects");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Policy(
            crate::fleet_ensure::policy::EnsurePolicyError::InvalidPrincipal { .. }
        )
    ));

    let mut treasury = fixture();
    treasury
        .desired
        .canisters
        .iter_mut()
        .find(|canister| canister.name == "treasury")
        .expect("treasury config")
        .replace = true;
    treasury.platform.desired = treasury.desired.clone();
    let error = workflow::plan(
        &treasury.root,
        &treasury.desired,
        &"6".repeat(64),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut treasury.platform,
    )
    .expect_err("treasury replacement rejects");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Policy(
            crate::fleet_ensure::policy::EnsurePolicyError::TreasuryReplacement { .. }
        )
    ));
    assert_eq!(anonymous.platform.mutations.values().sum::<u32>(), 0);
    assert_eq!(treasury.platform.mutations.values().sum::<u32>(), 0);

    let mut unsafe_name = fixture();
    unsafe_name.desired.fleet = "../escape".to_string();
    unsafe_name.platform.desired = unsafe_name.desired.clone();
    let error = workflow::plan(
        &unsafe_name.root,
        &unsafe_name.desired,
        &"5".repeat(64),
        "../escape",
        1_800_000_000_000_000_000,
        &mut unsafe_name.platform,
    )
    .expect_err("unsafe Fleet path label rejects before state access");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Policy(
            crate::fleet_ensure::policy::EnsurePolicyError::UnsafePathLabel { .. }
        )
    ));
    assert!(!unsafe_name.root.join(".canic").exists());
}

#[test]
fn managed_topology_without_typed_protocol_intent_rejects_before_effects() {
    let mut fixture = fixture();
    let coordinator = fixture
        .desired
        .canisters
        .iter()
        .find(|canister| canister.kind == DesiredCanisterKind::Coordinator)
        .expect("fixture Coordinator")
        .name
        .clone();
    let mut root = desired_canister(
        "managed-root",
        None,
        false,
        &fixture.root.join("app.wasm"),
        None,
    );
    root.kind = DesiredCanisterKind::Root;
    root.parent = Some(coordinator);
    fixture.desired.canisters.push(root);
    fixture.platform.desired = fixture.desired.clone();

    let error = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"f".repeat(64),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect_err("managed topology cannot omit typed protocol intent");

    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Policy(
            crate::fleet_ensure::policy::EnsurePolicyError::InvalidTopology {
                reason: "managed Fleet roles require complete typed protocol intent",
                ..
            }
        )
    ));
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 0);
}

#[test]
fn terminal_inventory_rejects_missing_and_duplicate_principals() {
    let mut state = FleetEnsureStateRecord {
        active_registry: None,
        completed_reinstall_action_sha256: BTreeMap::new(),
        completed_reinstall_operation_id: None,
        completed_reinstalls: BTreeMap::new(),
        fleet: "inventory-test".to_string(),
        pending_principals: BTreeMap::new(),
        principals: BTreeMap::new(),
        retained_cycles_by_principal: BTreeMap::new(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        topology: BTreeMap::from([(
            "coordinator".to_string(),
            crate::fleet_ensure::model::FleetEnsureTopologyRecord {
                kind: DesiredCanisterKind::Coordinator,
                module_hash: None,
                parent: None,
                protocol_binding: None,
                role: None,
            },
        )]),
    };
    assert!(matches!(
        crate::fleet_ensure::inventory::project_current_fleet_inventory(&state),
        Err(crate::fleet_ensure::CurrentFleetInventoryError::MissingPrincipal(name))
            if name == "coordinator"
    ));

    state
        .principals
        .insert("coordinator".to_string(), TREASURY.to_string());
    state.topology.insert(
        "duplicate".to_string(),
        crate::fleet_ensure::model::FleetEnsureTopologyRecord {
            kind: DesiredCanisterKind::Auxiliary,
            module_hash: None,
            parent: None,
            protocol_binding: None,
            role: None,
        },
    );
    state
        .principals
        .insert("duplicate".to_string(), TREASURY.to_string());
    assert!(matches!(
        crate::fleet_ensure::inventory::project_current_fleet_inventory(&state),
        Err(crate::fleet_ensure::CurrentFleetInventoryError::DuplicatePrincipal)
    ));
}

#[test]
fn terminal_protocol_inventory_survives_an_effect_free_successor_plan() {
    let fixture = fixture();
    let mut platform = fixture.platform;
    let component = Principal::from_slice(&[29; 29]).to_text();
    let module_hash = "ab".repeat(32);
    let registry = empty_active_registry();
    platform.terminal_inventory = TerminalFleetInventory {
        active_registry: Some(registry.clone()),
        controlled_cycles_by_principal: BTreeMap::from([(component.clone(), 0)]),
        entries: vec![RegistryEntry {
            module_hash: Some(module_hash.clone()),
            parent_pid: Some(TREASURY.to_string()),
            pid: component.clone(),
            protocol_binding: None,
            role: Some("managed_component".to_string()),
        }],
    };
    let source = "4".repeat(64);
    let first = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile first plan");
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &first.plan.plan_sha256,
        &mut platform,
    )
    .expect("converge first plan");

    platform
        .live
        .get_mut(TREASURY)
        .expect("terminal treasury")
        .cycles -= 25;
    platform
        .terminal_inventory
        .controlled_cycles_by_principal
        .insert(component.clone(), 25);

    platform.terminal_inventory_expected_operation_id = Some(first.plan.operation_id.clone());
    let second = apply_effect_free_successor(
        &fixture.root,
        &fixture.desired,
        &mut platform,
        &source,
        1_800_000_000_000_000_100,
        &first.plan.operation_id,
    );
    assert_eq!(
        second.conservation.observed_controlled_cycles,
        platform
            .live
            .values()
            .map(|canister| canister.cycles)
            .sum::<u128>()
            + 25
    );
    apply_effect_free_successor(
        &fixture.root,
        &fixture.desired,
        &mut platform,
        &source,
        1_800_000_000_000_000_200,
        &first.plan.operation_id,
    );

    let current = crate::fleet_ensure::resolve_current_fleet(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    )
    .expect("resolve terminal current Fleet");
    assert_eq!(
        current
            .initial_active_registry("test-fleet")
            .expect("terminal Registry authority"),
        &registry
    );
    assert!(current.registry.entries.iter().any(|entry| {
        entry.pid == component
            && entry.parent_pid.as_deref() == Some(TREASURY)
            && entry.module_hash.as_deref() == Some(module_hash.as_str())
    }));
}

fn apply_effect_free_successor(
    root: &Path,
    desired: &DesiredFleet,
    platform: &mut MockPlatform,
    source: &str,
    planned_at_time: u64,
    terminal_operation_id: &str,
) -> FleetEnsurePlan {
    let report = workflow::plan(
        root,
        desired,
        source,
        "test-fleet",
        planned_at_time,
        platform,
    )
    .expect("compile effect-free successor");
    assert!(report.plan.protocol_actions.is_empty());
    assert_eq!(
        report.plan.terminal_inventory_operation_id.as_deref(),
        Some(terminal_operation_id)
    );
    assert_eq!(
        platform
            .terminal_inventory_operation_ids
            .last()
            .map(String::as_str),
        Some(terminal_operation_id)
    );
    workflow::apply(
        root,
        desired,
        source,
        "test-fleet",
        &report.plan.plan_sha256,
        platform,
    )
    .expect("apply effect-free successor");
    report.plan
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the regression proves retained identity and conservation across the complete short-result replay"
)]
fn post_effect_balance_drift_preserves_the_nonterminal_journal_and_inventory() {
    let fixture = fixture();
    let mut platform = fixture.platform;
    platform.create_shortfalls.insert("created".to_string(), 1);
    let source = "8".repeat(64);
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile reviewed plan");
    let error = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect_err("short creation balance must stop before later effects");
    assert!(
        matches!(
            error,
            workflow::EnsureWorkflowError::ReplanRequiredAfterCreateBalanceDrift {
                actual_cycles: 19,
                configured_fee_cycles: 50,
                deficit_cycles: 1,
                requested_cycles: 20,
                ..
            }
        ),
        "unexpected short-create error: {error:?}"
    );
    let creation_hash = planned
        .plan
        .canisters
        .iter()
        .find(|canister| canister.name == "created")
        .and_then(|canister| canister.actions.first())
        .map(crate::fleet_ensure::ops::action_sha256)
        .expect("created action identity");
    assert_eq!(platform.mutations.get(&creation_hash), Some(&1));
    assert_eq!(platform.mutations.values().sum::<u32>(), 1);

    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    let journal = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("read journal")
        .expect("retained journal");
    assert_eq!(
        journal.completion,
        crate::fleet_ensure::model::FleetEnsureCompletion::ReplanRequired
    );
    assert_eq!(journal.effects.len(), 1);
    assert_eq!(journal.effects[0].state, EffectState::Applied);
    assert!(matches!(
        crate::fleet_ensure::read_current_fleet_inventory(
            &fixture.root,
            &fixture.desired.environment,
            &fixture.desired.fleet,
        ),
        Err(crate::fleet_ensure::CurrentFleetInventoryError::NotConverged { .. })
    ));
    let state = crate::fleet_ensure::ops::read_state(&paths, "test-fleet").expect("read state");
    assert_eq!(
        state.principals.get("created").map(String::as_str),
        Some("created-created")
    );
    let retained_topology = state
        .topology
        .get("created")
        .expect("created topology retained before replan");
    let reviewed_created = fixture
        .desired
        .canisters
        .iter()
        .find(|canister| canister.name == "created")
        .expect("reviewed created canister");
    assert_eq!(retained_topology.kind, reviewed_created.kind);
    assert_eq!(retained_topology.parent, reviewed_created.parent);
    assert_eq!(
        retained_topology.protocol_binding,
        reviewed_created.protocol_binding
    );
    assert_eq!(
        state.retained_cycles_by_principal.get("created-created"),
        Some(&19)
    );

    let mut successor_platform =
        MockPlatform::new(fixture.desired.clone(), platform.live.values().cloned());
    successor_platform.operator_cycles = platform.operator_cycles;
    successor_platform.terminal_inventory = platform.terminal_inventory.clone();
    let successor = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut successor_platform,
    )
    .expect("a closed nonconverged operation permits a freshly reviewed plan");
    assert!(successor.plan.canisters.iter().all(|canister| {
        canister.name != "created"
            || canister
                .actions
                .iter()
                .all(|action| !matches!(action, EnsureAction::Create { .. }))
    }));
    assert!(successor.plan.canisters.iter().any(|canister| {
        canister.name == "created"
            && canister
                .actions
                .iter()
                .any(|action| matches!(action, EnsureAction::Fund { .. }))
    }));
    let terminal = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &successor.plan.plan_sha256,
        &mut successor_platform,
    )
    .expect("successor plan converges the retained created canister");
    assert!(terminal.terminal);
    assert!(
        !successor_platform.mutations.contains_key(&creation_hash),
        "a fresh host process must not repeat the retained creation"
    );
    let state = crate::fleet_ensure::ops::read_state(&paths, "test-fleet").expect("read state");
    assert!(state.pending_principals.is_empty());
    assert_eq!(
        state.principals.get("created").map(String::as_str),
        Some("created-created")
    );
    let inventory = crate::fleet_ensure::resolve_current_fleet(
        &fixture.root,
        &fixture.desired.environment,
        &fixture.desired.fleet,
    )
    .expect("terminal current inventory is available to operator commands");
    assert_eq!(inventory.topology.coordinator_canister_id, TREASURY);
    assert_eq!(inventory.plan.plan_sha256, successor.plan.plan_sha256);
    assert_eq!(inventory.plan.operation_id, successor.plan.operation_id);
}

#[test]
fn consecutive_stalls_are_bounded_and_real_progress_resets_the_budget() {
    let fixture = fixture();
    let mut platform = fixture.platform;
    let source = "f".repeat(64);
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile reviewed plan");
    let first_action = planned
        .plan
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .next()
        .expect("fixture has an action");
    let first_hash = crate::fleet_ensure::ops::action_sha256(first_action);
    platform.stall_before_mutation.insert(first_hash.clone(), 2);

    let first = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect_err("first stalled observation remains retryable");
    assert!(matches!(
        first,
        workflow::EnsureWorkflowError::Platform(MockError)
    ));

    let second = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect_err("second consecutive stall reaches the configured bound");
    assert!(matches!(
        second,
        workflow::EnsureWorkflowError::Stalled { observations: 2 }
    ));
    assert_eq!(platform.mutations.values().sum::<u32>(), 0);

    let terminal = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect("genuine progress resumes the retained operation");
    assert!(terminal.terminal);
    assert_eq!(platform.mutations.get(&first_hash), Some(&1));
}

#[test]
fn retirement_without_exact_drain_authority_leaves_material_cycles_untouched() {
    let mut fixture = fixture();
    fixture
        .desired
        .canisters
        .iter_mut()
        .find(|canister| canister.name == "retired")
        .expect("retired config")
        .drain = None;
    fixture.platform.desired = fixture.desired.clone();
    let error = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"c".repeat(64),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect_err("material cycles require drain");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Policy(
            crate::fleet_ensure::policy::EnsurePolicyError::NoSafeDrain { .. }
        )
    ));
    assert_eq!(fixture.platform.mutations.values().sum::<u32>(), 0);
}

#[test]
fn retirement_never_stops_or_deletes_until_treasury_credit_is_observed() {
    let fixture = fixture();
    let mut platform = fixture.platform;
    platform.skip_transfer_credit = true;
    let source = "7".repeat(64);
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile reviewed plan");
    let retired = planned
        .plan
        .canisters
        .iter()
        .find(|canister| canister.name == "retired")
        .expect("retired canister plan");
    let transfer = retired
        .actions
        .iter()
        .find(|action| matches!(action, EnsureAction::Transfer { .. }))
        .map(crate::fleet_ensure::ops::action_sha256)
        .expect("transfer action");
    let stop = retired
        .actions
        .iter()
        .find(|action| matches!(action, EnsureAction::Stop { .. }))
        .map(crate::fleet_ensure::ops::action_sha256)
        .expect("stop action");
    let delete = retired
        .actions
        .iter()
        .find(|action| matches!(action, EnsureAction::Delete { .. }))
        .map(crate::fleet_ensure::ops::action_sha256)
        .expect("delete action");

    let error = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect_err("missing treasury credit blocks retirement");

    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Stalled { .. }
    ));
    assert_eq!(platform.mutations.get(&transfer), Some(&1));
    assert_eq!(platform.mutations.get(&stop), None);
    assert_eq!(platform.mutations.get(&delete), None);
    assert!(platform.live.contains_key(RETIRED));
}

#[test]
fn protocol_response_is_only_issuance_and_terminal_status_gates_later_actions() {
    let mut fixture = fixture();
    fixture
        .desired
        .protocol_steps
        .push(protocol_step(&fixture.root));
    fixture.platform.desired = fixture.desired.clone();
    fixture.platform.protocol_command_only = true;
    let source = "8".repeat(64);
    let mut platform = fixture.platform;
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile reviewed plan");
    let protocol = planned
        .plan
        .protocol_actions
        .first()
        .map(crate::fleet_ensure::ops::action_sha256)
        .expect("protocol action");
    let later_transfer = planned
        .plan
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .find(|action| matches!(action, EnsureAction::Transfer { .. }))
        .map(crate::fleet_ensure::ops::action_sha256)
        .expect("later transfer");

    let error = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect_err("issued protocol remains nonterminal");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Stalled { .. }
    ));
    assert_eq!(platform.mutations.get(&protocol), Some(&1));
    assert_eq!(platform.mutations.get(&later_transfer), None);

    platform
        .protocol_ready
        .insert("fleet-catalog-terminal".to_string());
    let terminal = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect("terminal observation resumes without reissuing protocol command");
    assert!(terminal.terminal);
    assert_eq!(platform.mutations.get(&protocol), Some(&1));
}

#[test]
fn in_progress_operation_resumes_reviewed_desired_before_newer_input() {
    let mut fixture = fixture();
    fixture.desired.protocol = Some(DesiredFleetProtocol {
        app_config: "canic.toml".to_string(),
        component_group_placements: Vec::new(),
        coordinator_candid: "coordinator.did".to_string(),
        root_candid: "root.did".to_string(),
        store_candid: "store.did".to_string(),
    });
    fixture
        .desired
        .canisters
        .iter_mut()
        .find(|canister| canister.kind == DesiredCanisterKind::Coordinator)
        .expect("fixture Coordinator")
        .wasm = None;
    fixture.platform.desired = fixture.desired.clone();
    fixture.platform.protocol_command_only = true;
    fixture.platform.typed_protocol = true;
    let reviewed_sha256 = "31".repeat(32);
    let mut platform = fixture.platform;
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &reviewed_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile reviewed operation");
    let protocol = planned
        .plan
        .protocol_actions
        .first()
        .map(crate::fleet_ensure::ops::action_sha256)
        .expect("protocol action");

    let error = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &reviewed_sha256,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect_err("issued protocol remains nonterminal");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Stalled { .. }
    ));
    assert_eq!(platform.mutations.get(&protocol), Some(&1));

    let mut newer = fixture.desired.clone();
    newer.maximum_stalled_observations = 17;
    let newer_sha256 = "32".repeat(32);
    platform.desired = newer.clone();
    let retained = workflow::plan(
        &fixture.root,
        &newer,
        &newer_sha256,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut platform,
    )
    .expect("return the immutable in-progress operation");
    assert_eq!(retained.plan.plan_sha256, planned.plan.plan_sha256);
    assert_eq!(retained.plan.desired_sha256, reviewed_sha256);
    assert_eq!(reviewed_desired(&retained.plan), Some(&fixture.desired));

    platform
        .protocol_ready
        .insert("fleet-component-provisioning".to_string());
    let terminal = workflow::apply(
        &fixture.root,
        &newer,
        &newer_sha256,
        "test-fleet",
        &retained.plan.plan_sha256,
        &mut platform,
    )
    .expect("resume the retained desired operation");
    assert!(terminal.terminal);
    assert_eq!(platform.mutations.get(&protocol), Some(&1));
    assert_eq!(platform.desired, fixture.desired);

    platform.desired = newer.clone();
    let successor = workflow::plan(
        &fixture.root,
        &newer,
        &newer_sha256,
        "test-fleet",
        1_800_000_000_000_000_200,
        &mut platform,
    )
    .expect("consider newer desired only after terminal operation");
    assert_eq!(successor.plan.desired_sha256, newer_sha256);
    assert_eq!(reviewed_desired(&successor.plan), Some(&newer));
}

#[test]
fn pre_snapshot_zero_debit_final_observation_resumes_without_reissuing() {
    let mut fixture = fixture();
    fixture
        .desired
        .canisters
        .retain(|canister| canister.name == "treasury");
    fixture.desired.protocol = Some(DesiredFleetProtocol {
        app_config: "canic.toml".to_string(),
        component_group_placements: Vec::new(),
        coordinator_candid: "coordinator.did".to_string(),
        root_candid: "root.did".to_string(),
        store_candid: "store.did".to_string(),
    });
    fixture
        .desired
        .canisters
        .iter_mut()
        .find(|canister| canister.kind == DesiredCanisterKind::Coordinator)
        .expect("fixture Coordinator")
        .wasm = None;
    fixture.platform.desired = fixture.desired.clone();
    fixture.platform.protocol_command_only = true;
    fixture.platform.typed_protocol = true;
    let reviewed_sha256 = "33".repeat(32);
    let mut platform = fixture.platform;
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &reviewed_sha256,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile reviewed operation");
    let mut retained = planned.plan;
    let action_hash = retained
        .protocol_actions
        .first()
        .map(crate::fleet_ensure::ops::action_sha256)
        .expect("terminal protocol action");
    retained.reviewed_desired = None;
    retained.plan_sha256 = crate::fleet_ensure::policy::expected_plan_sha256(&retained);
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    crate::fleet_ensure::ops::write_plan(&paths, &retained).expect("retain pre-snapshot plan");
    fs::remove_file(&paths.state).expect("remove disposable pre-effect identity projection");

    let error = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &reviewed_sha256,
        "test-fleet",
        &retained.plan_sha256,
        &mut platform,
    )
    .expect_err("issued protocol remains nonterminal");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Stalled { .. }
    ));
    assert_eq!(platform.mutations.get(&action_hash), Some(&1));

    let mut newer = fixture.desired.clone();
    newer.maximum_stalled_observations = 17;
    let newer_sha256 = "34".repeat(32);
    let mut mismatched = newer.clone();
    mismatched.canisters[0].principal = Some(OLD_APP.to_string());
    let error = workflow::apply(
        &fixture.root,
        &mismatched,
        &newer_sha256,
        "test-fleet",
        &retained.plan_sha256,
        &mut platform,
    )
    .expect_err("changed retained Principal cannot supply observation authority");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::RetainedDesiredUnavailable {
            actual,
            expected,
        } if actual == newer_sha256 && expected == reviewed_sha256
    ));

    platform
        .protocol_ready
        .insert("fleet-component-provisioning".to_string());
    let terminal = workflow::apply(
        &fixture.root,
        &newer,
        &newer_sha256,
        "test-fleet",
        &retained.plan_sha256,
        &mut platform,
    )
    .expect("newer input may observe the exact retained zero-debit terminal action");
    assert!(terminal.terminal);
    assert_eq!(platform.mutations.get(&action_hash), Some(&1));
}

#[test]
fn typed_fleet_protocol_is_issued_once_and_requires_terminal_status() {
    let mut fixture = fixture();
    fixture.desired.protocol = Some(DesiredFleetProtocol {
        app_config: "canic.toml".to_string(),
        component_group_placements: Vec::new(),
        coordinator_candid: "coordinator.did".to_string(),
        root_candid: "root.did".to_string(),
        store_candid: "store.did".to_string(),
    });
    fixture
        .desired
        .canisters
        .iter_mut()
        .find(|canister| canister.kind == DesiredCanisterKind::Coordinator)
        .expect("fixture Coordinator")
        .wasm = None;
    fixture.platform.desired = fixture.desired.clone();
    fixture.platform.protocol_command_only = true;
    fixture.platform.typed_protocol = true;
    let source = "a".repeat(64);
    let mut platform = fixture.platform;
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile reviewed typed protocol plan");
    let action = planned
        .plan
        .protocol_actions
        .first()
        .expect("typed protocol action");
    assert!(matches!(action, EnsureAction::FleetProtocol { .. }));
    let action_hash = crate::fleet_ensure::ops::action_sha256(action);

    let error = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect_err("issued typed protocol remains nonterminal");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Stalled { .. }
    ));
    assert_eq!(platform.mutations.get(&action_hash), Some(&1));

    platform
        .protocol_ready
        .insert("fleet-component-provisioning".to_string());
    let terminal = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect("terminal typed status resumes without a second command");
    assert!(terminal.terminal);
    assert_eq!(platform.mutations.get(&action_hash), Some(&1));

    let mutation_count = platform.mutations.values().sum::<u32>();
    let successor = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut platform,
    )
    .expect("plan immediate terminal replay");
    let replay = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &successor.plan.plan_sha256,
        &mut platform,
    )
    .expect("apply immediate terminal replay");
    assert!(replay.terminal);
    assert_eq!(platform.mutations.get(&action_hash), Some(&1));
    assert_eq!(platform.mutations.values().sum::<u32>(), mutation_count);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one focused regression keeps the exact Registry cycle projection and identity proof together"
)]
fn current_plan_round_trips_registry_actions_with_bounded_decimal_cycles() {
    let root = temp_dir("canic-fleet-ensure-json-round-trip");
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(&root, "staging", "json-round-trip");
    let base_registry = empty_active_registry();
    let base_version = FleetRegistryVersion {
        authority: base_registry.authority.clone(),
        revision: base_registry.revision,
        content_hash: [12; 32],
    };
    let root_entry = FleetSubnetRootEntry {
        placement_subnet: SubnetId::from_principal(
            SUBNET.parse().expect("fixture Subnet Principal"),
        ),
        fleet_subnet_root: OLD_APP.parse().expect("fixture Root Principal"),
        component_admissions: Vec::new(),
        component_topology_digest: ComponentTopologyDigest::from_bytes([13; 32]),
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [14; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([15; 32]),
        },
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 1,
            maximum_registry_bytes: 16_777_216,
            maximum_wasm_store_bytes: 40_000_000,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 2,
                canister_cycles: Cycles::new(u128::MAX),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(u128::MAX),
            },
            maximum_group_placements: 1,
        },
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
        status: FleetSubnetRootStatus::Joining,
    };
    let mut joined_registry = base_registry;
    joined_registry.revision = 2;
    joined_registry.fleet_subnet_roots.push(root_entry.clone());
    let joined_version = FleetRegistryVersion {
        authority: joined_registry.authority.clone(),
        revision: joined_registry.revision,
        content_hash: [16; 32],
    };
    let actions = vec![
        fleet_protocol_action(
            "registry-join",
            CurrentFleetProtocolAction::JoinRoot {
                expected_registry: joined_registry.clone(),
                expected_version: joined_version.clone(),
                request: FleetSubnetRootJoinRequest {
                    expected_registry: base_version,
                    entry: root_entry,
                },
            },
        ),
        fleet_protocol_action(
            "registry-activate",
            CurrentFleetProtocolAction::ActivateRegistry {
                expected_registry: joined_registry,
                expected_version: joined_version.clone(),
                request: FleetRegistryActivationRequest {
                    expected_registry: joined_version,
                },
            },
        ),
        fleet_protocol_action(
            "pool-ledger-recovery",
            CurrentFleetProtocolAction::RecoverPoolLedger {
                request: PoolLedgerRecoveryRequest {
                    artifact: PoolLedgerRecoveryArtifact {
                        candid_sha256: [19; 32],
                        payload_hash: [20; 32],
                        payload_size_bytes: 1,
                        raw_module_hash: [21; 32],
                        release_build_id: ReleaseBuildId::from_nonce(
                            ReleaseBuildNonce::from_random_bytes([22; 32]),
                        ),
                    },
                    canister_id: OLD_APP.parse().expect("fixture pool Principal"),
                    created_at_time_ns: 1,
                    cycles_ledger: TREASURY.parse().expect("fixture Ledger Principal"),
                    ledger_balance: Cycles::new(u128::MAX),
                    ledger_fee: Cycles::new(1),
                    maximum_execution_burn_cycles: Cycles::new(1),
                    operation_id: [23; 32],
                    withdrawal_amount: Cycles::new(u128::MAX - 1),
                },
            },
        ),
    ];
    let action_hashes = actions
        .iter()
        .map(crate::fleet_ensure::ops::action_sha256)
        .collect::<Vec<_>>();
    let mut plan = FleetEnsurePlan {
        canisters: Vec::new(),
        conservation: CycleConservation {
            expected_post_operation_cycles: 0,
            maximum_execution_burn_cycles: 0,
            maximum_new_funding_cycles: 0,
            maximum_operator_debit_cycles: 0,
            maximum_unavoidable_fee_cycles: 0,
            observed_controlled_cycles: 0,
            retained_in_reused_canisters_cycles: 0,
            scheduled_transfer_cycles: 0,
        },
        desired_sha256: "17".repeat(32),
        environment: "staging".to_string(),
        fleet: "json-round-trip".to_string(),
        operation_id: "18".repeat(32),
        plan_sha256: String::new(),
        planned_at_time: 1,
        protocol_actions: actions,
        root_start_authority: None,
        reviewed_desired: None,
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        scope: crate::fleet_ensure::model::FleetEnsurePlanScope::Full,
        terminal_inventory_operation_id: None,
    };
    plan.plan_sha256 = crate::fleet_ensure::policy::expected_plan_sha256(&plan);

    crate::fleet_ensure::ops::write_plan(&paths, &plan).expect("write current plan");
    let encoded = fs::read_to_string(&paths.plan).expect("read current plan JSON");
    assert!(encoded.contains(&format!("\"canister_cycles\": \"{}\"", u128::MAX)));
    assert!(encoded.contains(&format!("\"ledger_balance\": \"{}\"", u128::MAX)));
    assert!(encoded.contains(&format!("\"withdrawal_amount\": \"{}\"", u128::MAX - 1)));
    let reopened = crate::fleet_ensure::ops::read_plan(&paths)
        .expect("read current plan")
        .expect("retained current plan");

    assert_eq!(reopened, plan);
    assert_eq!(
        crate::fleet_ensure::policy::expected_plan_sha256(&reopened),
        plan.plan_sha256
    );
    assert_eq!(
        reopened
            .protocol_actions
            .iter()
            .map(crate::fleet_ensure::ops::action_sha256)
            .collect::<Vec<_>>(),
        action_hashes
    );

    fs::remove_dir_all(root).expect("remove Fleet ensure JSON fixture");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one focused proof keeps hash retention, partial progress, and content drift together"
)]
fn current_plan_retains_store_chunks_by_hash_instead_of_inline_bytes() {
    let root = temp_dir("canic-fleet-ensure-content-addressed-plan");
    let paths =
        crate::fleet_ensure::ops::EnsurePaths::under(&root, "staging", "content-addressed-plan");
    let template_id = TemplateId::owned("component:app".to_string());
    let version = TemplateVersion::owned("22".repeat(32));
    let bytes = vec![42; 4_096];
    let chunk_hash = canic_core::cdk::utils::hash::wasm_hash(&bytes);
    let actions = vec![
        fleet_protocol_action(
            "prepare-store-chunks",
            CurrentFleetProtocolAction::PrepareStoreChunkSet {
                request: TemplateChunkSetPrepareInput {
                    template_id: template_id.clone(),
                    version: version.clone(),
                    payload_hash: chunk_hash.clone(),
                    payload_size_bytes: bytes.len() as u64,
                    chunk_hashes: vec![chunk_hash.clone()],
                },
            },
        ),
        fleet_protocol_action(
            "publish-store-chunk",
            CurrentFleetProtocolAction::PublishStoreChunk {
                request: TemplateChunkInput {
                    template_id,
                    version,
                    chunk_index: 0,
                    bytes: bytes.clone(),
                },
            },
        ),
    ];
    let mut plan = FleetEnsurePlan {
        canisters: Vec::new(),
        conservation: CycleConservation {
            expected_post_operation_cycles: 0,
            maximum_execution_burn_cycles: 0,
            maximum_new_funding_cycles: 0,
            maximum_operator_debit_cycles: 0,
            maximum_unavoidable_fee_cycles: 0,
            observed_controlled_cycles: 0,
            retained_in_reused_canisters_cycles: 0,
            scheduled_transfer_cycles: 0,
        },
        desired_sha256: "23".repeat(32),
        environment: "staging".to_string(),
        fleet: "content-addressed-plan".to_string(),
        operation_id: "24".repeat(32),
        plan_sha256: String::new(),
        planned_at_time: 1,
        protocol_actions: actions,
        root_start_authority: None,
        reviewed_desired: None,
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        scope: crate::fleet_ensure::model::FleetEnsurePlanScope::Full,
        terminal_inventory_operation_id: None,
    };
    plan.plan_sha256 = crate::fleet_ensure::policy::expected_plan_sha256(&plan);

    fs::create_dir_all(paths.plan.parent().expect("plan state directory"))
        .expect("create inline-plan directory");
    fs::write(
        &paths.plan,
        crate::fleet_ensure::json::to_vec(&plan).expect("encode former inline plan"),
    )
    .expect("retain former inline plan");
    let inline_size = fs::metadata(&paths.plan)
        .expect("inspect former inline plan")
        .len();
    let inline = crate::fleet_ensure::ops::read_plan(&paths)
        .expect("read former inline current plan")
        .expect("former inline current plan");
    assert_eq!(inline, plan);
    assert!(
        crate::fleet_ensure::ops::compact_inline_plan(&paths, &inline)
            .expect("compact former inline current plan")
    );
    let encoded = fs::read_to_string(&paths.plan).expect("read hash-only current plan");
    assert!(encoded.len() as u64 * 2 < inline_size);
    assert!(!encoded.contains("\"bytes\""));
    assert!(encoded.contains("\"bytes_sha256\""));
    assert!(encoded.contains("\"chunk_hashes\""));
    let object = paths
        .content
        .join(canic_core::cdk::utils::hash::hex_bytes(&chunk_hash));
    assert_eq!(
        fs::read(&object).expect("read retained chunk object"),
        bytes
    );

    let reopened = crate::fleet_ensure::ops::read_plan(&paths)
        .expect("read hash-only current plan")
        .expect("retained hash-only current plan");
    assert_eq!(reopened, plan);
    assert_eq!(
        crate::fleet_ensure::policy::expected_plan_sha256(&reopened),
        plan.plan_sha256
    );
    assert!(
        !crate::fleet_ensure::ops::compact_inline_plan(&paths, &reopened)
            .expect("leave canonical current plan unchanged")
    );

    let partial_paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &root,
        "staging",
        "content-addressed-partial-plan",
    );
    let mut partial = plan;
    partial.fleet = "content-addressed-partial-plan".to_string();
    partial.protocol_actions.remove(0);
    partial.plan_sha256 = crate::fleet_ensure::policy::expected_plan_sha256(&partial);
    crate::fleet_ensure::ops::write_plan(&partial_paths, &partial)
        .expect("write publish-only partial-progress plan");
    assert_eq!(
        crate::fleet_ensure::ops::read_plan(&partial_paths)
            .expect("read publish-only partial-progress plan")
            .expect("retained publish-only partial-progress plan"),
        partial
    );

    fs::remove_file(&object).expect("remove isolated chunk object");
    assert!(matches!(
        crate::fleet_ensure::ops::read_plan(&paths),
        Err(crate::fleet_ensure::ops::EnsureStateError::StoreChunkUnavailable { path })
            if path == object
    ));
    fs::write(&object, [9]).expect("tamper isolated chunk object");
    assert!(matches!(
        crate::fleet_ensure::ops::read_plan(&paths),
        Err(crate::fleet_ensure::ops::EnsureStateError::StoreChunkMismatch { path })
            if path == object
    ));
    fs::remove_dir_all(root).expect("remove content-addressed plan fixture");
}

#[test]
#[ignore = "requires an explicit read-only current Fleet Ensure evidence directory"]
#[expect(
    clippy::too_many_lines,
    reason = "one evidence regression keeps isolation, all action variants, and immutable identity checks together"
)]
fn retained_current_plan_and_issued_journal_round_trip_from_an_isolated_copy() {
    let source = PathBuf::from(
        env::var_os("CANIC_CURRENT_FLEET_ENSURE_EVIDENCE")
            .expect("set CANIC_CURRENT_FLEET_ENSURE_EVIDENCE"),
    );
    let source_plan = source.join("plan.json");
    let source_journal = source.join("journal.json");
    let original_plan = fs::read(&source_plan).expect("read retained plan evidence");
    let original_journal = fs::read(&source_journal).expect("read retained journal evidence");
    let scratch = temp_dir("canic-retained-fleet-ensure-json");
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(&scratch, "evidence", "retained");
    fs::create_dir_all(
        paths
            .plan
            .parent()
            .expect("Fleet Ensure evidence directory"),
    )
    .expect("create isolated evidence directory");
    fs::write(&paths.plan, &original_plan).expect("copy retained plan evidence");
    fs::write(&paths.journal, &original_journal).expect("copy retained journal evidence");

    let mut plan = crate::fleet_ensure::ops::read_plan(&paths)
        .expect("decode retained current plan")
        .expect("retained current plan");
    let journal = crate::fleet_ensure::ops::read_journal(&paths)
        .expect("decode retained current journal")
        .expect("retained current journal");
    assert_eq!(
        crate::fleet_ensure::policy::expected_plan_sha256(&plan),
        plan.plan_sha256
    );
    assert_eq!(journal.plan_sha256, plan.plan_sha256);
    assert_eq!(journal.operation_id, plan.operation_id);
    assert!(matches!(
        journal.effects.last().map(|effect| &effect.state),
        Some(crate::fleet_ensure::model::EffectState::Issued)
    ));
    let retained_action_hashes = crate::fleet_ensure::workflow::ordered_actions(&plan)
        .into_iter()
        .take(journal.effects.len())
        .map(crate::fleet_ensure::ops::action_sha256)
        .collect::<Vec<_>>();
    assert_eq!(
        retained_action_hashes,
        journal
            .effects
            .iter()
            .map(|effect| effect.action_sha256.clone())
            .collect::<Vec<_>>()
    );
    assert!(
        crate::fleet_ensure::ops::compact_inline_plan(&paths, &plan)
            .expect("compact isolated retained plan before resumed effects")
    );
    let compacted_retained = fs::read(&paths.plan).expect("read compacted retained plan");
    assert!(compacted_retained.len() < original_plan.len() / 10);
    assert_eq!(
        crate::fleet_ensure::ops::read_plan(&paths)
            .expect("reopen compacted retained plan")
            .expect("compacted retained plan"),
        plan
    );
    assert_eq!(
        fs::read(&paths.journal).expect("reread isolated retained journal"),
        original_journal
    );

    let (registry_authority, root_entry) = plan
        .protocol_actions
        .iter()
        .find_map(|action| match action {
            EnsureAction::FleetProtocol { action, .. } => match action.as_ref() {
                CurrentFleetProtocolAction::JoinRoot {
                    expected_registry,
                    request,
                    ..
                } => Some((expected_registry.authority.clone(), request.entry.clone())),
                _ => None,
            },
            _ => None,
        })
        .expect("retained current plan Registry join");
    let store = plan
        .protocol_actions
        .iter()
        .find_map(|action| match action {
            EnsureAction::FleetProtocol { action, .. } => match action.as_ref() {
                CurrentFleetProtocolAction::BootstrapStore { expected, .. } => {
                    Some(expected.clone())
                }
                _ => None,
            },
            _ => None,
        })
        .expect("retained current plan Store bootstrap");
    plan.protocol_actions.push(fleet_protocol_action(
        "retained-evidence-adopt-store",
        CurrentFleetProtocolAction::AdoptStore {
            request: FleetSubnetWasmStoreAdoptionRequest {
                operation_id: [20; 32],
                authority: FleetSubnetWasmStoreAuthority {
                    authority: registry_authority,
                    placement_subnet: root_entry.placement_subnet,
                    fleet_subnet_root: root_entry.fleet_subnet_root,
                    wasm_store: store.wasm_store,
                    installation_controller: Principal::anonymous(),
                    release_build_id: store.release_set.release_build_id,
                    wasm_module_hash: [21; 32],
                },
            },
        },
    ));
    plan.plan_sha256 = crate::fleet_ensure::policy::expected_plan_sha256(&plan);
    let action_hashes = crate::fleet_ensure::workflow::ordered_actions(&plan)
        .into_iter()
        .map(crate::fleet_ensure::ops::action_sha256)
        .collect::<Vec<_>>();
    let expected_variants = BTreeSet::from([
        "activate_registry",
        "activate_registry_mirror",
        "adopt_store",
        "bootstrap_store",
        "join_root",
        "prepare_component_registry",
        "prepare_store_chunk_set",
        "provision_components",
        "publish_store_chunk",
        "stage_store_manifest",
        "synchronize_registry",
    ]);
    assert_eq!(current_protocol_variants(&plan), expected_variants);

    crate::fleet_ensure::ops::write_plan(&paths, &plan).expect("write isolated current plan");
    let reopened = crate::fleet_ensure::ops::read_plan(&paths)
        .expect("reopen isolated current plan")
        .expect("isolated current plan");
    assert_eq!(reopened, plan);
    assert_eq!(
        crate::fleet_ensure::policy::expected_plan_sha256(&reopened),
        plan.plan_sha256
    );
    assert_eq!(
        crate::fleet_ensure::workflow::ordered_actions(&reopened)
            .into_iter()
            .map(crate::fleet_ensure::ops::action_sha256)
            .collect::<Vec<_>>(),
        action_hashes
    );
    let canonical = fs::read(&paths.plan).expect("read canonical current plan");
    assert!(canonical.len() < original_plan.len() / 10);
    assert!(
        !canonical
            .windows(b"\"bytes\"".len())
            .any(|window| window == b"\"bytes\"")
    );
    let second = crate::fleet_ensure::ops::EnsurePaths::under(&scratch, "evidence", "second");
    crate::fleet_ensure::ops::write_plan(&second, &reopened)
        .expect("write second canonical current plan");
    assert_eq!(
        fs::read(&second.plan).expect("read second canonical current plan"),
        canonical
    );

    assert_eq!(
        fs::read(source_plan).expect("reread source plan"),
        original_plan
    );
    assert_eq!(
        fs::read(source_journal).expect("reread source journal"),
        original_journal
    );
    fs::remove_dir_all(scratch).expect("remove isolated retained evidence");
}

#[test]
fn tampered_reviewed_plan_fails_before_any_effect() {
    let fixture = fixture();
    let mut platform = fixture.platform;
    let source = "e".repeat(64);
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("compile reviewed plan");
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    let mut retained = serde_json::from_slice::<crate::fleet_ensure::model::FleetEnsurePlan>(
        &fs::read(&paths.plan).expect("read retained plan"),
    )
    .expect("decode retained plan");
    retained.conservation.maximum_new_funding_cycles += 1;
    fs::write(
        &paths.plan,
        serde_json::to_vec_pretty(&retained).expect("encode tampered plan"),
    )
    .expect("tamper retained plan");

    let error = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect_err("tampered retained plan rejects");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::PlanIntegrity
    ));
    assert_eq!(platform.mutations.values().sum::<u32>(), 0);
}

#[test]
#[ignore = "the workspace runner supplies one shared PocketIC server and serial execution"]
#[expect(
    clippy::too_many_lines,
    reason = "one governed journey keeps the inconsistent estate and second-run proof together"
)]
fn governed_pocketic_toko_shaped_estate_converges_then_has_zero_effects() {
    use pocket_ic::{CreateCanisterParams, PocketIcBuilder};

    struct PocketPlatform {
        desired: DesiredFleet,
        known: BTreeSet<String>,
        operator_cycles: u128,
        pic: pocket_ic::PocketIc,
        protocol_ready: BTreeSet<String>,
        reinstall_required: BTreeSet<String>,
    }

    impl PocketPlatform {
        fn principal<'a>(state: &'a FleetEnsureStateRecord, principal: &'a str) -> Option<&'a str> {
            principal
                .strip_prefix("created:")
                .map_or(Some(principal), |name| {
                    state.pending_principals.get(name).map(String::as_str)
                })
        }

        fn live(&self, principal: &str) -> Option<LiveCanister> {
            if !self.known.contains(principal) {
                return None;
            }
            let id = principal.parse().expect("PocketIC Principal");
            let status = self
                .pic
                .canister_status(id, Some(CONTROLLER.parse().expect("controller Principal")))
                .expect("PocketIC status");
            Some(LiveCanister {
                canister_version: Some(status.version),
                controllers: status
                    .settings
                    .controllers
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                cycles: self.pic.cycle_balance(id),
                module_sha256: status
                    .module_hash
                    .map(canic_core::cdk::utils::hash::hex_bytes),
                principal: principal.to_string(),
                reinstall_required: self.reinstall_required.contains(principal),
                root_owned_lifecycle: None,
                status: match format!("{:?}", status.status).as_str() {
                    "Running" => CanisterRuntimeStatus::Running,
                    "Stopped" => CanisterRuntimeStatus::Stopped,
                    _ => CanisterRuntimeStatus::Stopping,
                },
            })
        }
    }

    impl EnsurePlatform for PocketPlatform {
        type Error = std::io::Error;

        fn bind_reviewed_desired(&mut self, desired: &DesiredFleet) -> Result<(), Self::Error> {
            self.desired = desired.clone();
            Ok(())
        }

        fn observe(
            &mut self,
            _operation_id: &str,
            state: &FleetEnsureStateRecord,
        ) -> Result<FleetObservation, Self::Error> {
            let canisters = self
                .desired
                .canisters
                .iter()
                .map(|configured| {
                    let principal = state
                        .principals
                        .get(&configured.name)
                        .map(String::as_str)
                        .or(configured.principal.as_deref());
                    (
                        configured.name.clone(),
                        principal.and_then(|principal| self.live(principal)),
                    )
                })
                .collect();
            Ok(FleetObservation {
                additional_controlled_cycles: BTreeMap::new(),
                canisters,
                ledger_fee_cycles: self
                    .desired
                    .ledger_fee_cycles
                    .parse::<Cycles>()
                    .map(|cycles| cycles.to_u128())
                    .expect("PocketIC fixture ledger fee"),
                operator_cycles: self.operator_cycles,
                protocol_ready: self
                    .desired
                    .protocol_steps
                    .iter()
                    .map(|step| (step.name.clone(), self.protocol_ready.contains(&step.name)))
                    .collect(),
            })
        }

        fn observe_effect(
            &mut self,
            _operation_id: &str,
            action: &EnsureAction,
            record: &EffectRecord,
            state: &FleetEnsureStateRecord,
        ) -> Result<EffectObservation, Self::Error> {
            if let EnsureAction::Create {
                requested_initial_cycles,
                ..
            } = action
            {
                let post_cycles = record
                    .created_principal
                    .as_deref()
                    .and_then(|principal| self.live(principal))
                    .map(|live| live.cycles);
                let applied = post_cycles == Some(*requested_initial_cycles);
                return Ok(EffectObservation {
                    applied,
                    post_cycles,
                    progress_identity: format!("created:{:?}", record.created_principal),
                    retry: if post_cycles.is_some() && !applied {
                        EffectRetry::ReplanRequiredAfterCreateBalanceDrift
                    } else {
                        EffectRetry::None
                    },
                });
            }
            if matches!(action, EnsureAction::Fund { .. }) {
                return Ok(EffectObservation {
                    applied: record.receipt.is_some(),
                    post_cycles: None,
                    progress_identity: format!("fund:{:?}", record.receipt),
                    retry: EffectRetry::None,
                });
            }
            let principal = match action {
                EnsureAction::Delete { principal, .. }
                | EnsureAction::FleetProtocol { principal, .. }
                | EnsureAction::Install { principal, .. }
                | EnsureAction::Protocol { principal, .. }
                | EnsureAction::SetControllers { principal, .. }
                | EnsureAction::Start { principal, .. }
                | EnsureAction::Stop { principal, .. }
                | EnsureAction::Transfer { principal, .. } => Self::principal(state, principal),
                EnsureAction::Create { .. } | EnsureAction::Fund { .. } => None,
            };
            let applied = match action {
                EnsureAction::Install {
                    mode, wasm_sha256, ..
                } => principal
                    .and_then(|value| self.live(value))
                    .is_some_and(|live| {
                        crate::fleet_ensure::ops::install_effect_applied(
                            *mode,
                            wasm_sha256,
                            live.module_sha256.as_deref(),
                            record.pre_canister_version,
                            live.canister_version,
                        )
                    }),
                EnsureAction::Start { .. } => principal
                    .and_then(|value| self.live(value))
                    .is_some_and(|live| live.status == CanisterRuntimeStatus::Running),
                EnsureAction::SetControllers { controllers, .. } => principal
                    .and_then(|value| self.live(value))
                    .is_some_and(|live| live.controllers == *controllers),
                EnsureAction::Delete { .. } => {
                    principal.is_none_or(|value| self.live(value).is_none())
                }
                EnsureAction::Stop { .. } => principal
                    .and_then(|value| self.live(value))
                    .is_some_and(|live| live.status == CanisterRuntimeStatus::Stopped),
                EnsureAction::FleetProtocol { name, .. } | EnsureAction::Protocol { name, .. } => {
                    self.protocol_ready.contains(name)
                }
                EnsureAction::Create { .. }
                | EnsureAction::Fund { .. }
                | EnsureAction::Transfer { .. } => false,
            };
            Ok(EffectObservation {
                applied,
                post_cycles: None,
                progress_identity: format!("pocketic:{action:?}:{applied}"),
                retry: EffectRetry::None,
            })
        }

        fn action_cycles(
            &mut self,
            action: &EnsureAction,
            state: &FleetEnsureStateRecord,
        ) -> Result<Option<u128>, Self::Error> {
            let principal = match action {
                EnsureAction::Create { .. } => return Ok(None),
                EnsureAction::Delete { principal, .. }
                | EnsureAction::FleetProtocol { principal, .. }
                | EnsureAction::Fund { principal, .. }
                | EnsureAction::Install { principal, .. }
                | EnsureAction::Protocol { principal, .. }
                | EnsureAction::SetControllers { principal, .. }
                | EnsureAction::Start { principal, .. }
                | EnsureAction::Stop { principal, .. }
                | EnsureAction::Transfer { principal, .. } => Self::principal(state, principal),
            };
            Ok(principal.and_then(|value| self.live(value).map(|live| live.cycles)))
        }

        fn action_destination_cycles(
            &mut self,
            _action: &EnsureAction,
            _state: &FleetEnsureStateRecord,
        ) -> Result<Option<u128>, Self::Error> {
            Ok(None)
        }

        fn action_canister_version(
            &mut self,
            action: &EnsureAction,
            state: &FleetEnsureStateRecord,
        ) -> Result<Option<u64>, Self::Error> {
            let principal = match action {
                EnsureAction::Install { principal, .. } => Self::principal(state, principal),
                _ => None,
            };
            Ok(principal
                .and_then(|value| self.live(value))
                .and_then(|live| live.canister_version))
        }

        fn apply(
            &mut self,
            _operation_id: &str,
            action: &EnsureAction,
            _record: &EffectRecord,
            state: &FleetEnsureStateRecord,
        ) -> Result<EffectOutcome, Self::Error> {
            let principal = |value: &str| {
                Self::principal(state, value)
                    .expect("created Principal exists")
                    .parse()
                    .expect("PocketIC Principal")
            };
            match action {
                EnsureAction::Create {
                    requested_initial_cycles,
                    ..
                } => {
                    let id = self
                        .pic
                        .create_canister_with_params(
                            None,
                            CreateCanisterParams {
                                cycles: Some(*requested_initial_cycles),
                                settings: Some(pocket_ic::CanisterSettings {
                                    controllers: Some(vec![
                                        CONTROLLER.parse().expect("controller Principal"),
                                    ]),
                                    ..pocket_ic::CanisterSettings::default()
                                }),
                                ..CreateCanisterParams::default()
                            },
                        )
                        .map_err(std::io::Error::other)?;
                    self.operator_cycles -= requested_initial_cycles;
                    self.known.insert(id.to_string());
                    Ok(EffectOutcome {
                        created_principal: Some(id.to_string()),
                        post_cycles: Some(*requested_initial_cycles),
                        receipt: Some(format!("pocketic-create-{id}")),
                    })
                }
                EnsureAction::Fund {
                    amount,
                    principal: target,
                    ..
                } => {
                    let id = principal(target);
                    self.pic.add_cycles(id, *amount);
                    self.operator_cycles -= amount;
                    Ok(EffectOutcome {
                        created_principal: None,
                        post_cycles: Some(self.pic.cycle_balance(id)),
                        receipt: Some(format!("pocketic-fund-{id}")),
                    })
                }
                EnsureAction::Install {
                    mode,
                    principal: target,
                    wasm,
                    ..
                } => {
                    let id = principal(target);
                    let bytes = fs::read(wasm).expect("read PocketIC Wasm");
                    match mode {
                        crate::fleet_ensure::model::InstallMode::Install => {
                            self.pic.install_canister(
                                id,
                                bytes,
                                Vec::new(),
                                Some(CONTROLLER.parse().expect("controller Principal")),
                            );
                        }
                        crate::fleet_ensure::model::InstallMode::Reinstall => {
                            self.pic
                                .reinstall_canister(
                                    id,
                                    bytes,
                                    Vec::new(),
                                    Some(CONTROLLER.parse().expect("controller Principal")),
                                )
                                .expect("reinstall PocketIC canister");
                            self.reinstall_required.remove(&id.to_string());
                        }
                    }
                    Ok(empty_outcome())
                }
                EnsureAction::FleetProtocol {
                    name,
                    principal: target,
                    ..
                }
                | EnsureAction::Protocol {
                    name,
                    principal: target,
                    ..
                } => {
                    self.protocol_ready.insert(name.clone());
                    Ok(EffectOutcome {
                        created_principal: None,
                        post_cycles: Some(self.pic.cycle_balance(principal(target))),
                        receipt: Some("pocketic-protocol".to_string()),
                    })
                }
                EnsureAction::SetControllers {
                    controllers,
                    principal: target,
                    ..
                } => {
                    self.pic
                        .set_controllers(
                            principal(target),
                            Some(CONTROLLER.parse().expect("controller Principal")),
                            controllers
                                .iter()
                                .map(|value| value.parse().expect("controller Principal"))
                                .collect(),
                        )
                        .expect("set PocketIC controllers");
                    Ok(empty_outcome())
                }
                EnsureAction::Start {
                    principal: target, ..
                } => {
                    self.pic
                        .start_canister(
                            principal(target),
                            Some(CONTROLLER.parse().expect("controller Principal")),
                        )
                        .expect("start PocketIC canister");
                    Ok(empty_outcome())
                }
                EnsureAction::Delete { .. }
                | EnsureAction::Stop { .. }
                | EnsureAction::Transfer { .. } => Err(std::io::Error::other(
                    "governed current-state journey does not retire canisters",
                )),
            }
        }
    }

    let root = temp_dir("canic-fleet-ensure-pocketic");
    fs::create_dir_all(&root).expect("create PocketIC fixture root");
    let wasm = root.join("current.wasm");
    fs::write(&wasm, b"\0asm\x01\0\0\0").expect("write minimal Wasm");
    let pic = start_pocket_ic(PocketIcBuilder::new().with_application_subnet());
    let treasury = pic
        .create_canister_with_params(
            None,
            CreateCanisterParams {
                cycles: Some(1_000_000_000_000),
                settings: Some(pocket_ic::CanisterSettings {
                    controllers: Some(vec![CONTROLLER.parse().expect("controller Principal")]),
                    ..pocket_ic::CanisterSettings::default()
                }),
                ..CreateCanisterParams::default()
            },
        )
        .expect("create treasury");
    let root_canister = pic
        .create_canister_with_params(
            None,
            CreateCanisterParams {
                cycles: Some(500_000_000_000),
                settings: Some(pocket_ic::CanisterSettings {
                    controllers: Some(vec![CONTROLLER.parse().expect("controller Principal")]),
                    ..pocket_ic::CanisterSettings::default()
                }),
                ..CreateCanisterParams::default()
            },
        )
        .expect("create partial Root");
    pic.install_canister(
        root_canister,
        fs::read(&wasm).expect("read current Root Wasm"),
        Vec::new(),
        Some(CONTROLLER.parse().expect("controller Principal")),
    );
    pic.stop_canister(
        root_canister,
        Some(CONTROLLER.parse().expect("controller Principal")),
    )
    .expect("stop partial Root");

    let mut canisters = vec![DesiredCanister {
        canic_init: None,
        controller_canisters: Vec::new(),
        controllers: vec![CONTROLLER.to_string()],
        drain: None,
        initial_cycles: "0".to_string(),
        init_arg: None,
        init_candid: None,
        kind: DesiredCanisterKind::Auxiliary,
        minimum_cycles: "0".to_string(),
        name: "treasury".to_string(),
        parent: None,
        presence: DesiredPresence::Present,
        principal: Some(treasury.to_string()),
        protocol_binding: None,
        replace: false,
        subnet: SUBNET.to_string(),
        wasm: None,
    }];
    let mut root_desired =
        desired_canister("root", Some(&root_canister.to_string()), false, &wasm, None);
    // This fixture exercises the generic effect/conservation engine with
    // minimal Wasm while retaining the exact Coordinator -> Root topology
    // required for a governed same-module Root reset.
    root_desired.kind = DesiredCanisterKind::Root;
    root_desired.parent = Some("coordinator".to_string());
    root_desired.initial_cycles = "1000000000000".to_string();
    root_desired.minimum_cycles = "1000000000000".to_string();
    canisters.push(root_desired);
    for role in [
        "coordinator",
        "store",
        "discovery",
        "projects",
        "project_registry",
        "market",
        "users",
    ] {
        let mut canister = desired_canister(role, None, false, &wasm, None);
        canister.kind = if role == "coordinator" {
            DesiredCanisterKind::Coordinator
        } else {
            DesiredCanisterKind::Auxiliary
        };
        canister.parent = None;
        canister.initial_cycles = "1000000000000".to_string();
        canister.minimum_cycles = "500000000000".to_string();
        canisters.push(canister);
    }
    let desired = DesiredFleet {
        bootstrap: None,
        canisters,
        cycles_ledger: LEDGER.to_string(),
        environment: "local".to_string(),
        fleet: "toko-shaped".to_string(),
        ledger_fee_cycles: "0".to_string(),
        management_creation_fee_cycles: "0".to_string(),
        material_cycle_threshold: "1000000".to_string(),
        maximum_observation_burn_cycles: "10000000".to_string(),
        maximum_stalled_observations: 8,
        maximum_update_burn_cycles: "100000000000".to_string(),
        operator: CONTROLLER.to_string(),
        protocol: None,
        protocol_steps: [
            ("store-bootstrap", "store"),
            ("registry-join", "coordinator"),
            ("registry-activation", "coordinator"),
            ("root-registry-sync", "root"),
            ("admission-projection", "root"),
            ("activate-discovery", "discovery"),
            ("activate-projects", "projects"),
            ("activate-project-registry", "project_registry"),
            ("activate-market", "market"),
            ("activate-users", "users"),
            ("fleet-catalog-publication", "coordinator"),
        ]
        .into_iter()
        .map(|(name, canister)| protocol_step_for(&root, name, canister))
        .collect(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        treasury: "treasury".to_string(),
    };
    let mut platform = PocketPlatform {
        desired: desired.clone(),
        known: BTreeSet::from([treasury.to_string(), root_canister.to_string()]),
        operator_cycles: 10_000_000_000_000,
        pic,
        protocol_ready: BTreeSet::new(),
        reinstall_required: BTreeSet::from([root_canister.to_string()]),
    };
    let source = "d".repeat(64);
    let planned = workflow::plan(
        &root,
        &desired,
        &source,
        "toko-shaped",
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("plan inconsistent PocketIC estate");
    let applied = workflow::apply(
        &root,
        &desired,
        &source,
        "toko-shaped",
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect("converge PocketIC estate");
    assert!(applied.terminal);
    assert!(applied.actual_conservation.is_some());

    let second = workflow::plan(
        &root,
        &desired,
        &source,
        "toko-shaped",
        1_800_000_000_000_000_100,
        &mut platform,
    )
    .expect("plan converged PocketIC estate");
    assert!(
        second
            .plan
            .canisters
            .iter()
            .all(|canister| canister.actions.is_empty())
    );
    let replay = workflow::apply(
        &root,
        &desired,
        &source,
        "toko-shaped",
        &second.plan.plan_sha256,
        &mut platform,
    )
    .expect("effect-free PocketIC replay");
    assert!(replay.terminal);
    assert_eq!(replay.effects_applied, 0);
}

struct Fixture {
    desired: DesiredFleet,
    platform: MockPlatform,
    root: PathBuf,
}

#[test]
fn fresh_logical_controller_and_treasury_roles_create_and_replay_without_effect() {
    let mut fixture = fixture();
    let wasm = fixture.root.join("app.wasm");
    let mut coordinator = desired_canister("treasury", None, false, &wasm, None);
    coordinator.kind = DesiredCanisterKind::Coordinator;
    let mut child = desired_canister("created", None, false, &wasm, None);
    child.controller_canisters = vec!["treasury".to_string()];
    fixture.desired.canisters = vec![coordinator, child];
    fixture.desired.treasury = "treasury".to_string();
    fixture.desired.ledger_fee_cycles = "10B".to_string();
    fixture.desired.management_creation_fee_cycles = "50B".to_string();
    for canister in &mut fixture.desired.canisters {
        canister.initial_cycles = "20B".to_string();
        canister.minimum_cycles = "20B".to_string();
    }
    fixture.platform = MockPlatform::new(fixture.desired.clone(), Vec::new());
    fixture.platform.operator_cycles = 1_000_000_000_000_000;

    let desired_sha256 = "73".repeat(32);
    let planned = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        &fixture.desired.fleet,
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("plan fresh logical authority");
    assert_eq!(
        planned
            .plan
            .canisters
            .iter()
            .filter(|canister| canister.disposition == CanisterDisposition::Create)
            .count(),
        2
    );
    let applied = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        &fixture.desired.fleet,
        &planned.plan.plan_sha256,
        &mut fixture.platform,
    )
    .expect("apply fresh logical authority");
    assert!(applied.terminal);

    let second = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &desired_sha256,
        &fixture.desired.fleet,
        1_800_000_000_000_000_100,
        &mut fixture.platform,
    )
    .expect("replan converged logical authority");
    assert!(
        second
            .plan
            .canisters
            .iter()
            .all(|canister| canister.actions.is_empty())
    );

    fs::remove_dir_all(fixture.root).expect("remove test directory");
}

#[test]
fn protocol_planning_selects_the_largest_ordered_prefix_with_cycle_headroom() {
    let mut fixture = protocol_tranche_fixture(vec![499, 2]);

    let report = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"12".repeat(32),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect("compile the largest affordable protocol prefix");

    assert_eq!(report.plan.protocol_actions.len(), 1);
    assert_eq!(
        report.plan.protocol_actions[0].name(),
        "fleet-component-provisioning-0"
    );
    assert_eq!(report.plan.conservation.maximum_execution_burn_cycles, 499);
    assert_eq!(report.plan.conservation.observed_controlled_cycles, 500);
    assert_eq!(report.plan.conservation.expected_post_operation_cycles, 1);
    assert!(fixture.platform.mutations.is_empty());

    let first_action_sha256 = crate::fleet_ensure::ops::action_sha256(
        report
            .plan
            .protocol_actions
            .first()
            .expect("first protocol tranche"),
    );
    let first = apply_fixture_plan(&mut fixture, &"12".repeat(32), &report.plan)
        .expect_err("the completed prefix requires one reviewed successor plan");
    assert!(matches!(
        first,
        workflow::EnsureWorkflowError::ConvergenceDrift
    ));
    assert_eq!(
        fixture.platform.mutations.get(&first_action_sha256),
        Some(&1)
    );

    let successor = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"12".repeat(32),
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut fixture.platform,
    )
    .expect("compile the remaining affordable protocol tranche");
    assert_eq!(successor.plan.protocol_actions.len(), 1);
    assert_eq!(
        successor.plan.protocol_actions[0].name(),
        "fleet-component-provisioning-1"
    );
    assert_eq!(successor.plan.conservation.maximum_execution_burn_cycles, 2);
    let successor_action_sha256 = crate::fleet_ensure::ops::action_sha256(
        successor
            .plan
            .protocol_actions
            .first()
            .expect("successor protocol tranche"),
    );
    let terminal = apply_fixture_plan(&mut fixture, &"12".repeat(32), &successor.plan)
        .expect("apply the terminal protocol tranche");
    assert!(terminal.terminal);
    assert_eq!(
        fixture.platform.mutations.get(&successor_action_sha256),
        Some(&1)
    );

    let mutations = fixture.platform.mutations.clone();
    let replay_plan = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"12".repeat(32),
        "test-fleet",
        1_800_000_000_000_000_200,
        &mut fixture.platform,
    )
    .expect("compile effect-free protocol replay");
    assert!(workflow::ordered_actions(&replay_plan.plan).is_empty());
    let replay = apply_fixture_plan(&mut fixture, &"12".repeat(32), &replay_plan.plan)
        .expect("apply effect-free protocol replay");
    assert!(replay.terminal);
    assert_eq!(fixture.platform.mutations, mutations);

    fs::remove_dir_all(fixture.root).expect("remove test directory");
}

fn apply_fixture_plan(
    fixture: &mut Fixture,
    desired_sha256: &str,
    plan: &FleetEnsurePlan,
) -> Result<crate::fleet_ensure::model::FleetEnsureReport, workflow::EnsureWorkflowError<MockError>>
{
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        desired_sha256,
        "test-fleet",
        &plan.plan_sha256,
        &mut fixture.platform,
    )
}

#[test]
fn first_unaffordable_protocol_action_rejects_with_exact_cycle_guidance() {
    let mut fixture = protocol_tranche_fixture(vec![501, 1]);

    let error = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &"13".repeat(32),
        "test-fleet",
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .expect_err("the first indivisible protocol action cannot fit");

    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::Policy(
            crate::fleet_ensure::policy::EnsurePolicyError::InsufficientCycleConservation {
                action_count: 1,
                available: 500,
                required: 501,
                shortfall: 1,
            }
        )
    ));
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        &fixture.root,
        &fixture.desired.environment,
        "test-fleet",
    );
    assert!(
        crate::fleet_ensure::ops::read_plan(&paths)
            .expect("read absent rejected plan")
            .is_none()
    );
    assert!(fixture.platform.mutations.is_empty());

    fs::remove_dir_all(fixture.root).expect("remove test directory");
}

fn protocol_tranche_fixture(burns: Vec<u128>) -> Fixture {
    let mut fixture = fixture();
    fixture
        .desired
        .canisters
        .retain(|canister| canister.name == "treasury");
    fixture.desired.canisters[0].wasm = None;
    fixture.desired.protocol = Some(DesiredFleetProtocol {
        app_config: "canic.toml".to_string(),
        component_group_placements: Vec::new(),
        coordinator_candid: "coordinator.did".to_string(),
        root_candid: "root.did".to_string(),
        store_candid: "store.did".to_string(),
    });
    let current_hash = sha256_hex(b"current-wasm");
    fixture.platform = MockPlatform::new(
        fixture.desired.clone(),
        vec![live(
            TREASURY,
            500,
            Some(&current_hash),
            true,
            &[CONTROLLER],
        )],
    );
    fixture.platform.typed_protocol_burns = burns;
    fixture
}

fn fixture() -> Fixture {
    let root = temp_dir("canic-fleet-ensure");
    fs::create_dir_all(&root).expect("create fixture root");
    let wasm = root.join("app.wasm");
    let candid = root.join("drain.did");
    fs::write(&wasm, b"current-wasm").expect("write Wasm");
    fs::write(&candid, b"service : {};").expect("write Candid");
    let desired = DesiredFleet {
        bootstrap: None,
        canisters: vec![
            desired_canister("treasury", Some(TREASURY), false, &wasm, None),
            desired_canister("created", None, false, &wasm, None),
            desired_canister("app", Some(OLD_APP), false, &wasm, None),
            DesiredCanister {
                canic_init: None,
                controller_canisters: Vec::new(),
                controllers: vec![CONTROLLER.to_string()],
                drain: Some(drain(&candid)),
                initial_cycles: "0".to_string(),
                init_arg: None,
                init_candid: None,
                kind: DesiredCanisterKind::Auxiliary,
                minimum_cycles: "0".to_string(),
                name: "retired".to_string(),
                parent: None,
                presence: DesiredPresence::Absent,
                principal: Some(RETIRED.to_string()),
                protocol_binding: None,
                replace: false,
                subnet: SUBNET.to_string(),
                wasm: None,
            },
            desired_canister("replaced", Some(OLD_REPLACED), true, &wasm, Some(&candid)),
        ],
        cycles_ledger: LEDGER.to_string(),
        environment: "local".to_string(),
        fleet: "test-fleet".to_string(),
        ledger_fee_cycles: "10".to_string(),
        management_creation_fee_cycles: "50".to_string(),
        material_cycle_threshold: "5".to_string(),
        maximum_observation_burn_cycles: "0".to_string(),
        maximum_stalled_observations: 2,
        maximum_update_burn_cycles: "1".to_string(),
        operator: CONTROLLER.to_string(),
        protocol: None,
        protocol_steps: Vec::new(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        treasury: "treasury".to_string(),
    };
    let current_hash = sha256_hex(b"current-wasm");
    let live = vec![
        live(TREASURY, 500, Some(&current_hash), true, &[CONTROLLER]),
        live(OLD_APP, 5, Some("old"), false, &[TREASURY]),
        live(RETIRED, 100, Some("old"), true, &[CONTROLLER]),
        live(OLD_REPLACED, 100, Some("old"), true, &[CONTROLLER]),
    ];
    let platform = MockPlatform::new(desired.clone(), live);
    Fixture {
        desired,
        platform,
        root,
    }
}

fn desired_canister(
    name: &str,
    principal: Option<&str>,
    replace: bool,
    wasm: &Path,
    drain_candid: Option<&Path>,
) -> DesiredCanister {
    DesiredCanister {
        canic_init: None,
        controller_canisters: Vec::new(),
        controllers: vec![CONTROLLER.to_string()],
        drain: drain_candid.map(drain),
        initial_cycles: "20".to_string(),
        init_arg: None,
        init_candid: None,
        kind: if name == "treasury" {
            DesiredCanisterKind::Coordinator
        } else {
            DesiredCanisterKind::Auxiliary
        },
        minimum_cycles: "20".to_string(),
        name: name.to_string(),
        parent: None,
        presence: DesiredPresence::Present,
        principal: principal.map(str::to_string),
        protocol_binding: None,
        replace,
        subnet: SUBNET.to_string(),
        wasm: Some(wasm.display().to_string()),
    }
}

fn drain(candid: &Path) -> DrainAuthority {
    DrainAuthority {
        candid: candid.display().to_string(),
        destination: "treasury".to_string(),
        maximum_execution_burn_cycles: "2".to_string(),
        method: "canic_cycle_drain".to_string(),
    }
}

fn protocol_step(root: &Path) -> DesiredProtocolStep {
    protocol_step_for(root, "fleet-catalog-terminal", "app")
}

fn fleet_protocol_action(name: &str, action: CurrentFleetProtocolAction) -> EnsureAction {
    EnsureAction::FleetProtocol {
        action: Box::new(action),
        candid: "coordinator.did".to_string(),
        candid_sha256: "19".repeat(32),
        maximum_execution_burn_cycles: 1,
        name: name.to_string(),
        principal: TREASURY.to_string(),
    }
}

fn current_protocol_variants(plan: &FleetEnsurePlan) -> BTreeSet<&'static str> {
    plan.protocol_actions
        .iter()
        .filter_map(|action| match action {
            EnsureAction::FleetProtocol { action, .. } => Some(match action.as_ref() {
                CurrentFleetProtocolAction::ActivateRegistry { .. } => "activate_registry",
                CurrentFleetProtocolAction::ActivateRegistryMirror { .. } => {
                    "activate_registry_mirror"
                }
                CurrentFleetProtocolAction::AdoptStore { .. } => "adopt_store",
                CurrentFleetProtocolAction::BootstrapStore { .. } => "bootstrap_store",
                CurrentFleetProtocolAction::JoinRoot { .. } => "join_root",
                CurrentFleetProtocolAction::PrepareStoreChunkSet { .. } => {
                    "prepare_store_chunk_set"
                }
                CurrentFleetProtocolAction::PrepareComponentRegistry { .. } => {
                    "prepare_component_registry"
                }
                CurrentFleetProtocolAction::ProvisionComponents { .. } => "provision_components",
                CurrentFleetProtocolAction::RecoverPoolLedger { .. } => "recover_pool_ledger",
                CurrentFleetProtocolAction::PublishStoreChunk { .. } => "publish_store_chunk",
                CurrentFleetProtocolAction::StageStoreManifest { .. } => "stage_store_manifest",
                CurrentFleetProtocolAction::SynchronizeRegistry { .. } => "synchronize_registry",
            }),
            _ => None,
        })
        .collect()
}

fn reviewed_desired(plan: &FleetEnsurePlan) -> Option<&DesiredFleet> {
    plan.reviewed_desired
        .as_deref()
        .map(crate::fleet_ensure::model::ReviewedDesiredFleetRecord::desired)
}

fn typed_protocol_action(operation_id: &str) -> EnsureAction {
    let operation_id = canic_core::cdk::utils::hash::decode_hex(operation_id)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .expect("Fleet ensure operation ID");
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([7; 32]),
        },
        app: AppId::from("ensure_test"),
    };
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet.clone(),
            coordinator_subnet: SubnetId::from_principal(
                SUBNET.parse().expect("fixture Subnet Principal"),
            ),
            coordinator: TREASURY.parse().expect("fixture Coordinator Principal"),
        },
        epoch: 1,
    };
    EnsureAction::FleetProtocol {
        action: Box::new(CurrentFleetProtocolAction::ProvisionComponents {
            request: FleetComponentProvisioningPrepareRequest {
                operation_id,
                plan: FleetComponentProvisioningPlan {
                    fleet,
                    fleet_registry: FleetRegistryVersion {
                        authority,
                        revision: 1,
                        content_hash: [8; 32],
                    },
                    configuration_digest: ComponentDeploymentConfigurationDigest::from_bytes(
                        [9; 32],
                    ),
                    operation: FleetComponentProvisioningOperation::FreshInstall,
                    directory_confirmation_roots: Vec::new(),
                    batches: Vec::new(),
                },
            },
            plan_hash: [10; 32],
        }),
        candid: "coordinator.did".to_string(),
        candid_sha256: "11".repeat(32),
        maximum_execution_burn_cycles: 1,
        name: "fleet-component-provisioning".to_string(),
        principal: TREASURY.to_string(),
    }
}

fn pool_ledger_recovery_action() -> EnsureAction {
    EnsureAction::FleetProtocol {
        action: Box::new(CurrentFleetProtocolAction::RecoverPoolLedger {
            request: PoolLedgerRecoveryRequest {
                artifact: PoolLedgerRecoveryArtifact {
                    candid_sha256: [1; 32],
                    payload_hash: [2; 32],
                    payload_size_bytes: 3,
                    raw_module_hash: [4; 32],
                    release_build_id: ReleaseBuildId::from_nonce(
                        ReleaseBuildNonce::from_random_bytes([5; 32]),
                    ),
                },
                canister_id: OLD_APP.parse().expect("fixture pool Principal"),
                created_at_time_ns: 6,
                cycles_ledger: LEDGER.parse().expect("fixture Cycles Ledger Principal"),
                ledger_balance: Cycles::new(30),
                ledger_fee: Cycles::new(10),
                maximum_execution_burn_cycles: Cycles::new(1),
                operation_id: [7; 32],
                withdrawal_amount: Cycles::new(20),
            },
        }),
        candid: "root.did".to_string(),
        candid_sha256: "8".repeat(64),
        maximum_execution_burn_cycles: 1,
        name: "pool-ledger-recovery:app".to_string(),
        principal: OLD_APP.to_string(),
    }
}

fn empty_active_registry() -> FleetRegistry {
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([7; 32]),
        },
        app: AppId::from("ensure_test"),
    };
    FleetRegistry {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: fleet.clone(),
                coordinator_subnet: SubnetId::from_principal(
                    SUBNET.parse().expect("fixture Subnet Principal"),
                ),
                coordinator: TREASURY.parse().expect("fixture Coordinator Principal"),
            },
            epoch: 1,
        },
        revision: 1,
        admission: FleetAdmissionPolicy {
            schema_version: 1,
            fleet,
            generation: 1,
            fleet_principals: Vec::new(),
            rules: Vec::new(),
            policy_digest: [0; 32],
        },
        component_specs: Vec::new(),
        fleet_subnet_roots: Vec::new(),
        services: Vec::new(),
    }
}

fn protocol_step_for(root: &Path, name: &str, canister: &str) -> DesiredProtocolStep {
    let candid = root.join("protocol.did");
    let command_args = root.join("protocol-command.args");
    let status_args = root.join("protocol-status.args");
    let expected_status = root.join("protocol-expected.args");
    fs::write(
        &candid,
        b"service : { apply : (text) -> (); status : (text) -> (bool) query; };",
    )
    .expect("write protocol Candid");
    fs::write(&command_args, b"({{operation_id}})").expect("write protocol command");
    fs::write(&status_args, b"({{operation_id}})").expect("write protocol status");
    fs::write(&expected_status, b"(true)").expect("write expected protocol status");
    DesiredProtocolStep {
        canister: canister.to_string(),
        candid: candid.display().to_string(),
        command_args: command_args.display().to_string(),
        command_method: "apply".to_string(),
        expected_status: expected_status.display().to_string(),
        maximum_execution_burn_cycles: "1".to_string(),
        name: name.to_string(),
        status_args: status_args.display().to_string(),
        status_method: "status".to_string(),
    }
}

fn live(
    principal: &str,
    cycles: u128,
    module_sha256: Option<&str>,
    running: bool,
    controllers: &[&str],
) -> LiveCanister {
    LiveCanister {
        canister_version: Some(1),
        controllers: controllers
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        cycles,
        module_sha256: module_sha256.map(str::to_string),
        principal: principal.to_string(),
        reinstall_required: false,
        root_owned_lifecycle: None,
        status: if running {
            CanisterRuntimeStatus::Running
        } else {
            CanisterRuntimeStatus::Stopped
        },
    }
}

fn install_action(
    name: &str,
    canic_init: DesiredCanisterInit,
    mode: crate::fleet_ensure::model::InstallMode,
) -> EnsureAction {
    EnsureAction::Install {
        canic_init: Some(canic_init),
        init_arg: None,
        init_arg_sha256: None,
        init_candid: None,
        init_candid_sha256: None,
        mode,
        name: name.to_string(),
        principal: Principal::from_slice(&[99; 29]).to_text(),
        wasm: "artifact.wasm".to_string(),
        wasm_sha256: "ab".repeat(32),
    }
}

fn empty_outcome() -> EffectOutcome {
    EffectOutcome {
        created_principal: None,
        post_cycles: None,
        receipt: None,
    }
}
