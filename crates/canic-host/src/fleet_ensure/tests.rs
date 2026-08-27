use crate::{
    fleet_ensure::{
        model::{
            CanisterRuntimeStatus, CurrentFleetProtocolAction, DesiredCanister,
            DesiredCanisterKind, DesiredFleet, DesiredFleetProtocol, DesiredPresence,
            DesiredProtocolStep, DrainAuthority, EffectRecord, EnsureAction,
            FLEET_ENSURE_SCHEMA_VERSION, FleetEnsureStateRecord, FleetObservation, LiveCanister,
        },
        ops::{EffectObservation, EffectOutcome, EnsurePlatform, TerminalFleetInventory},
        workflow,
    },
    registry::RegistryEntry,
    test_support::temp_dir,
};
use candid::Principal;
use canic_core::{
    cdk::utils::hash::sha256_hex,
    dto::{
        component_provisioning::{
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
            FleetComponentProvisioningPrepareRequest,
        },
        fleet_registry::{FleetRegistry, FleetRegistryVersion},
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentDeploymentConfigurationDigest, FleetAdmissionPolicy,
        FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, SubnetId,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
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
    create_shortfalls: BTreeMap<String, u128>,
    desired: DesiredFleet,
    fail_once: BTreeSet<String>,
    failed: BTreeSet<String>,
    live: BTreeMap<String, LiveCanister>,
    ledger_fee_cycles: u128,
    mutations: BTreeMap<String, u32>,
    operator_cycles: u128,
    protocol_command_only: bool,
    protocol_ready: BTreeSet<String>,
    typed_protocol: bool,
    skip_transfer_credit: bool,
    stall_before_mutation: BTreeMap<String, u32>,
    terminal_inventory: TerminalFleetInventory,
}

impl MockPlatform {
    fn new(desired: DesiredFleet, live: impl IntoIterator<Item = LiveCanister>) -> Self {
        let ledger_fee_cycles = desired
            .ledger_fee_cycles
            .parse()
            .expect("fixture ledger fee");
        Self {
            completed: BTreeMap::new(),
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
            protocol_ready: BTreeSet::new(),
            typed_protocol: false,
            skip_transfer_credit: false,
            stall_before_mutation: BTreeMap::new(),
            terminal_inventory: TerminalFleetInventory::default(),
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

    #[expect(
        clippy::too_many_lines,
        reason = "the deterministic adapter keeps every effect mutation in one exhaustive match"
    )]
    fn mutate(&mut self, action: &EnsureAction, state: &FleetEnsureStateRecord) -> EffectOutcome {
        let principal = Self::principal(state, action).map(str::to_string);
        match action {
            EnsureAction::Create {
                controllers,
                name,
                requested_initial_cycles,
                ..
            } => {
                let creation_fee = self
                    .desired
                    .management_creation_fee_cycles
                    .parse::<u128>()
                    .expect("creation fee");
                let ledger_fee = self
                    .desired
                    .ledger_fee_cycles
                    .parse::<u128>()
                    .expect("ledger fee");
                self.operator_cycles -= requested_initial_cycles + creation_fee + ledger_fee;
                let principal = format!("created-{name}");
                let retained_cycles = requested_initial_cycles
                    .checked_sub(self.create_shortfalls.get(name).copied().unwrap_or(0))
                    .expect("fixture creation shortfall is bounded");
                self.live.insert(
                    principal.clone(),
                    LiveCanister {
                        controllers: controllers.clone(),
                        cycles: retained_cycles,
                        module_sha256: None,
                        principal: principal.clone(),
                        root_owned_lifecycle: None,
                        status: CanisterRuntimeStatus::Running,
                    },
                );
                EffectOutcome {
                    created_principal: Some(principal),
                    post_cycles: Some(retained_cycles),
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
                    .parse::<u128>()
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
                self.live
                    .get_mut(principal.as_deref().expect("install principal"))
                    .expect("install target")
                    .module_sha256 = Some(wasm_sha256.clone());
                empty_outcome()
            }
            EnsureAction::FleetProtocol { name, .. } | EnsureAction::Protocol { name, .. } => {
                if !self.protocol_command_only {
                    self.protocol_ready.insert(name.clone());
                }
                EffectOutcome {
                    created_principal: None,
                    post_cycles: principal
                        .as_deref()
                        .and_then(|value| self.live.get(value))
                        .map(|live| live.cycles),
                    receipt: Some("protocol-receipt".to_string()),
                }
            }
            EnsureAction::SetControllers { controllers, .. } => {
                self.live
                    .get_mut(principal.as_deref().expect("settings principal"))
                    .expect("settings target")
                    .controllers = controllers.clone();
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
                        .get_mut(destination)
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
        if !self.typed_protocol || self.protocol_ready.contains("fleet-component-provisioning") {
            return Ok(Vec::new());
        }
        Ok(vec![typed_protocol_action(operation_id)])
    }

    fn terminal_inventory(
        &mut self,
        _operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<TerminalFleetInventory, Self::Error> {
        Ok(self.terminal_inventory.clone())
    }

    fn observe_effect(
        &mut self,
        _operation_id: &str,
        action: &EnsureAction,
        record: &EffectRecord,
        state: &FleetEnsureStateRecord,
    ) -> Result<EffectObservation, Self::Error> {
        if matches!(action, EnsureAction::Create { .. }) {
            return Ok(EffectObservation {
                applied: record.created_principal.is_some(),
                progress_identity: format!("created:{:?}", record.created_principal),
            });
        }
        if matches!(action, EnsureAction::Fund { .. }) {
            return Ok(EffectObservation {
                applied: record.receipt.is_some(),
                progress_identity: format!("fund:{:?}", record.receipt),
            });
        }
        let principal = Self::principal(state, action);
        let applied = match action {
            EnsureAction::Delete { .. } => {
                principal.is_none_or(|value| !self.live.contains_key(value))
            }
            EnsureAction::Install { wasm_sha256, .. } => {
                principal
                    .and_then(|value| self.live.get(value))
                    .and_then(|live| live.module_sha256.as_deref())
                    == Some(wasm_sha256)
            }
            EnsureAction::FleetProtocol { name, .. } | EnsureAction::Protocol { name, .. } => {
                self.protocol_ready.contains(name)
            }
            EnsureAction::SetControllers { controllers, .. } => principal
                .and_then(|value| self.live.get(value))
                .is_some_and(|live| live.controllers == *controllers),
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
            progress_identity: format!("mock:{action:?}:{applied}"),
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
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        let EnsureAction::Transfer { destination, .. } = action else {
            return Ok(None);
        };
        Ok(self.live.get(destination).map(|live| live.cycles))
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
        fleet: "inventory-test".to_string(),
        pending_principals: BTreeMap::new(),
        principals: BTreeMap::new(),
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

    let second = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut platform,
    )
    .expect("compile effect-free successor");
    assert!(second.plan.protocol_actions.is_empty());
    assert_eq!(
        second.plan.conservation.observed_controlled_cycles,
        platform
            .live
            .values()
            .map(|canister| canister.cycles)
            .sum::<u128>()
            + 25
    );
    workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &second.plan.plan_sha256,
        &mut platform,
    )
    .expect("apply effect-free successor");

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
    .expect_err("short creation receipt must not be called converged");
    assert!(matches!(
        error,
        workflow::EnsureWorkflowError::ConvergenceDrift
    ));
    let creation_hash = planned
        .plan
        .canisters
        .iter()
        .find(|canister| canister.name == "created")
        .and_then(|canister| canister.actions.first())
        .map(crate::fleet_ensure::ops::action_sha256)
        .expect("created action identity");
    assert_eq!(platform.mutations.get(&creation_hash), Some(&1));

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
    assert!(matches!(
        crate::fleet_ensure::read_current_fleet_inventory(
            &fixture.root,
            &fixture.desired.environment,
            &fixture.desired.fleet,
        ),
        Err(crate::fleet_ensure::CurrentFleetInventoryError::NotConverged { .. })
    ));
    let state = crate::fleet_ensure::ops::read_state(&paths, "test-fleet").expect("read state");
    assert!(!state.principals.contains_key("created"));
    assert!(state.topology.is_empty());

    let successor = workflow::plan(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        1_800_000_000_000_000_100,
        &mut platform,
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
                .any(|action| matches!(action, EnsureAction::Fund { amount: 1, .. }))
    }));
    let terminal = workflow::apply(
        &fixture.root,
        &fixture.desired,
        &source,
        "test-fleet",
        &successor.plan.plan_sha256,
        &mut platform,
    )
    .expect("successor plan converges the retained created canister");
    assert!(terminal.terminal);
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
#[expect(
    clippy::too_many_lines,
    reason = "one governed journey keeps the inconsistent estate and second-run proof together"
)]
fn pocketic_generic_toko_shaped_estate_converges_then_has_zero_effects() {
    use pocket_ic::{CreateCanisterParams, PocketIcBuilder};

    struct PocketPlatform {
        desired: DesiredFleet,
        known: BTreeSet<String>,
        operator_cycles: u128,
        pic: pocket_ic::PocketIc,
        protocol_ready: BTreeSet<String>,
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
                    .parse()
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
            if matches!(action, EnsureAction::Create { .. }) {
                return Ok(EffectObservation {
                    applied: record.created_principal.is_some(),
                    progress_identity: format!("created:{:?}", record.created_principal),
                });
            }
            if matches!(action, EnsureAction::Fund { .. }) {
                return Ok(EffectObservation {
                    applied: record.receipt.is_some(),
                    progress_identity: format!("fund:{:?}", record.receipt),
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
                EnsureAction::Install { wasm_sha256, .. } => {
                    principal
                        .and_then(|value| self.live(value))
                        .and_then(|live| live.module_sha256)
                        .as_deref()
                        == Some(wasm_sha256)
                }
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
                progress_identity: format!("pocketic:{action:?}:{applied}"),
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
    let old_wasm = b"\0asm\x01\0\0\0\0\x01\0".to_vec();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
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
        old_wasm,
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
    // minimal Wasm, not the typed Canic control plane. The production typed
    // journey owns Coordinator/Root/Store initialization separately.
    root_desired.kind = DesiredCanisterKind::Auxiliary;
    root_desired.parent = None;
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
        treasury: treasury.to_string(),
    };
    let mut platform = PocketPlatform {
        desired: desired.clone(),
        known: BTreeSet::from([treasury.to_string(), root_canister.to_string()]),
        operator_cycles: 10_000_000_000_000,
        pic,
        protocol_ready: BTreeSet::new(),
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
        treasury: TREASURY.to_string(),
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
        destination: TREASURY.to_string(),
        maximum_execution_burn_cycles: "2".to_string(),
        method: "canic_cycle_drain".to_string(),
    }
}

fn protocol_step(root: &Path) -> DesiredProtocolStep {
    protocol_step_for(root, "fleet-catalog-terminal", "app")
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
        controllers: controllers
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        cycles,
        module_sha256: module_sha256.map(str::to_string),
        principal: principal.to_string(),
        root_owned_lifecycle: None,
        status: if running {
            CanisterRuntimeStatus::Running
        } else {
            CanisterRuntimeStatus::Stopped
        },
    }
}

fn empty_outcome() -> EffectOutcome {
    EffectOutcome {
        created_principal: None,
        post_cycles: None,
        receipt: None,
    }
}
