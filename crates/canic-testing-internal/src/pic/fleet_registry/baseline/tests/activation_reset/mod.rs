//! Module: pic::fleet_registry::baseline::tests::activation_reset
//!
//! Responsibility: qualify retained activation evidence through preparation and Root reset.
//! Boundary: faulty fixture initialization creates the conflict; production recovery owns effects.

use super::*;
use canic_host::fleet_ensure::{
    model::{FleetEnsurePlanScope, FleetEnsureTopologyRecord},
    ops::{EnsurePaths, read_journal, read_state, resolve_desired_artifacts, write_state},
};

#[test]
pub(super) fn source_bound_activation_reset_recovers_and_replays() {
    assert_literal_zero_host_journey(FundingJourney::ActivationReset, 1);
}

fn platform(input: &ReinstallJourney<'_>, desired: &DesiredFleet) -> IcpEnsurePlatform {
    literal_zero_journey_platform(
        desired,
        input.icp_wrapper,
        input.adapter_root,
        input.local_replica.clone(),
        true,
    )
}

/// Install the Store under the retained provisioning operation, leaving the Root's
/// original initialization unchanged. Both canisters run sealed production Wasms.
fn conflict_store_identity(input: &ReinstallJourney<'_>, plan: &FleetEnsurePlan) {
    let store = input
        .desired
        .canisters
        .iter()
        .find(|c| c.name == "store")
        .unwrap();
    let artifacts = resolve_desired_artifacts(input.adapter_root, input.desired).unwrap();
    let action = EnsureAction::Install {
        canic_init: store.canic_init.clone(),
        reinstall_witness: None,
        init_arg: None,
        init_arg_sha256: None,
        init_candid: None,
        init_candid_sha256: None,
        mode: canic_host::fleet_ensure::model::InstallMode::Reinstall,
        name: store.name.clone(),
        principal: store.principal.clone().unwrap(),
        wasm: store.wasm.clone().unwrap(),
        wasm_sha256: artifacts.wasm_sha256_by_canister[&store.name].clone(),
    };
    let paths = EnsurePaths::under(input.adapter_root, &plan.environment, &plan.fleet);
    let state = retain_infrastructure(input, &paths);
    let operator = Principal::from_text(&input.desired.operator).unwrap();
    input
        .pic
        .stop_canister(input.store, Some(operator))
        .unwrap();
    platform(input, input.desired)
        .apply(
            &plan.operation_id,
            &action,
            &fixture_effect_intent(&action),
            &state,
        )
        .expect("install the conflicting Store through the production initializer");
    input
        .pic
        .start_canister(input.store, Some(operator))
        .unwrap();
}

fn lose_preparation_stop_reply(input: &ReinstallJourney<'_>) {
    let script = std::fs::read_to_string(input.icp_wrapper).unwrap();
    let branch = r#"case " $* " in
  *" canister stop "*)
    icp "$@"
    result=$?
    [ "$result" -eq 0 ] || exit "$result"
    printf '%s\n' "$*" >> "$wrapper_root/preparation-stops.log"
    if [ ! -e "$wrapper_root/lost-preparation-stop" ]; then
      : > "$wrapper_root/lost-preparation-stop"
      exit 78
    fi
    exit 0
    ;;
esac
"#;
    assert!(script.contains("case \" $* \" in"));
    std::fs::write(
        input.icp_wrapper,
        script.replacen("case \" $* \" in", &format!("{branch}case \" $* \" in"), 1),
    )
    .unwrap();
}

/// Capture physical observations, without manufacturing a completed operation journal.
fn retain_infrastructure(
    input: &ReinstallJourney<'_>,
    paths: &EnsurePaths,
) -> FleetEnsureStateRecord {
    let mut state = read_state(paths, &input.desired.fleet).unwrap();
    let operator = Principal::from_text(&input.desired.operator).unwrap();
    for configured in &input.desired.canisters {
        let principal = Principal::from_text(configured.principal.as_deref().unwrap()).unwrap();
        state
            .principals
            .insert(configured.name.clone(), principal.to_text());
        if configured.kind == canic_host::fleet_ensure::model::DesiredCanisterKind::Pool {
            continue;
        }
        let live = input
            .pic
            .canister_status(principal, Some(operator))
            .unwrap();
        state.topology.insert(
            configured.name.clone(),
            FleetEnsureTopologyRecord {
                kind: configured.kind,
                module_hash: live.module_hash.map(hex_bytes),
                parent: configured.parent.clone(),
                protocol_binding: None,
                role: None,
            },
        );
    }
    write_state(paths, &state).unwrap();
    state
}

fn retain_source(input: &ReinstallJourney<'_>) -> FleetEnsurePlan {
    let desired = input.desired;
    let paths = EnsurePaths::under(input.adapter_root, &desired.environment, &desired.fleet);
    let mut source_platform = platform(input, desired);
    let mut source = fleet_ensure_workflow::plan(
        input.adapter_root,
        desired,
        &desired_sha256(desired),
        &desired.fleet,
        1_800_000_000_000_000_041,
        &mut source_platform,
    )
    .unwrap()
    .plan;
    assert!(
        source
            .canisters
            .iter()
            .all(|canister| canister.actions.is_empty())
    );
    conflict_store_identity(input, &source);
    let state = retain_infrastructure(input, &paths);
    let operator = Principal::from_text(&desired.operator).unwrap();
    let ledger = Principal::from_text(&desired.cycles_ledger).unwrap();
    let funding = AutonomousFundingJourney {
        adapter_root: input.adapter_root,
        icp_wrapper: input.icp_wrapper,
        local_replica: input.local_replica,
        pic: input.pic,
        desired,
        root: input.root,
        operator,
        cycles_ledger: ledger,
        assets: &[],
        imported: input.pools,
        repair_failed_reserve: false,
        funding_pause: false,
        native_pause: None,
        readiness_floor: 0,
        operator_after_initial_creation: ledger_account_balance(input.pic, ledger, operator)
            .0
            .try_into()
            .unwrap(),
    };
    retain_issued_underfunded_fixture(&funding, &mut source, &state, &mut source_platform);
    let operation_id = source
        .protocol_actions
        .iter()
        .find_map(|action| {
            if let EnsureAction::FleetProtocol { action, .. } = action
                && let CurrentFleetProtocolAction::ProvisionComponents { request, .. } =
                    action.as_ref()
            {
                Some(request.operation_id)
            } else {
                None
            }
        })
        .unwrap();
    for attempt in 0..240 {
        let response: Result<RootStatusResponseFragment, Error> = input
            .pic
            .query_candid_as(
                input.root,
                operator,
                canic::protocol::CANIC_ROOT_OPERATION_STATUS,
                (RootStatusRequestFragment::Operation(
                    OperationStatusRequest { operation_id },
                ),),
            )
            .unwrap();
        if let Ok(RootStatusResponseFragment::Operation(
            RootOperationStatusResponse::ProvisionComponents(status),
        )) = response
            && status.component_count > 0
            && status.activated_component_count == status.component_count
        {
            assert!(!status.root_runtime_active);
            assert_eq!(
                status.phase,
                canic_core::dto::component_provisioning::RootComponentProvisioningPhase::Published
            );
            let journal = read_journal(&paths).unwrap().unwrap();
            assert_eq!(journal.effects.last().unwrap().state, EffectState::Issued);
            return source;
        }
        assert!(attempt < 239, "published Components with an inactive Root");
        input.pic.advance_time(Duration::from_secs(1));
        input.pic.tick();
    }
    unreachable!()
}

#[expect(
    clippy::too_many_lines,
    reason = "one IC recovery journey retains exact source, preparation and reset replay evidence"
)]
pub(super) fn assert_journey(input: ReinstallJourney<'_>) {
    let source = retain_source(&input);
    let paths = EnsurePaths::under(
        input.adapter_root,
        &input.desired.environment,
        &input.desired.fleet,
    );
    let original = [
        std::fs::read(&paths.plan).unwrap(),
        std::fs::read(&paths.journal).unwrap(),
        std::fs::read(&paths.state).unwrap(),
    ];
    let replacement = selected_reinstall_artifacts(&input);
    let desired = select_reinstall_build(input.desired, &replacement);
    let digest = desired_sha256(&desired);
    let review = fleet_ensure_workflow::plan_reinstall(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        1_800_000_000_000_000_051,
        &mut platform(&input, &desired),
    )
    .expect("review exact retained activation evidence");
    assert_eq!(
        review.plan.scope,
        FleetEnsurePlanScope::ReinstallPreparation
    );
    let retained = &review
        .plan
        .reinstall
        .as_ref()
        .unwrap()
        .activation_reset
        .as_ref()
        .unwrap()
        .source;
    assert_eq!(retained.operation_id, source.operation_id);
    assert_eq!(
        review.plan.reinstall.as_ref().unwrap().assets.len(),
        input.pools.len()
    );
    assert_eq!(
        [
            std::fs::read(&paths.plan).unwrap(),
            std::fs::read(&paths.journal).unwrap(),
            std::fs::read(&paths.state).unwrap()
        ],
        original
    );
    lose_preparation_stop_reply(&input);
    let rejected = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &"f".repeat(64),
        &mut platform(&input, &desired),
    );
    assert!(matches!(
        rejected,
        Err(EnsureWorkflowError::PlanDigestMismatch { .. })
    ));
    assert!(!input.adapter_root.join("lost-preparation-stop").exists());
    let lost = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &review.plan.plan_sha256,
        &mut platform(&input, &desired),
    );
    assert!(
        matches!(lost, Err(EnsureWorkflowError::Platform(_))),
        "lost preparation Stop: {lost:?}"
    );
    assert!(input.adapter_root.join("lost-preparation-stop").is_file());
    let interrupted = read_journal(&paths).unwrap().unwrap();
    // A controlled fixture credit exercises positive Stop settlement without
    // attributing Toko's observed credit to a particular IC refund mechanism.
    input.pic.add_cycles(input.coordinator, 10_000_000_000);
    let prepared = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &review.plan.plan_sha256,
        &mut platform(&input, &desired),
    )
    .expect("source-bound preparation conserves cycles");
    assert!(prepared.terminal);
    let conservation = prepared.actual_conservation.as_ref().unwrap();
    assert!(
        conservation.observed_net_cycle_debit_cycles == 0
            || conservation.observed_net_cycle_credit_cycles == 0
    );
    assert_eq!(
        conservation.observed_starting_cycles
            + conservation.received_new_funding_cycles
            + conservation.observed_net_cycle_credit_cycles,
        conservation.final_controlled_cycles
            + conservation.exact_estate_creation_fee_cycles
            + conservation.observed_net_cycle_debit_cycles
    );
    assert_eq!(conservation.operator_debit_cycles, 0);
    assert_eq!(conservation.received_new_funding_cycles, 0);
    let completed = read_journal(&paths).unwrap().unwrap();
    assert_eq!(
        completed.initial_controlled_cycles,
        interrupted.initial_controlled_cycles
    );
    assert_eq!(
        completed.initial_operator_cycles,
        interrupted.initial_operator_cycles
    );
    for (digest, bytes) in [
        &retained.plan_document_sha256,
        &retained.journal_document_sha256,
        &retained.state_document_sha256,
    ]
    .into_iter()
    .zip(&original)
    {
        assert_eq!(
            std::fs::read(
                paths
                    .plan
                    .with_file_name("activation-reset-evidence")
                    .join(digest)
            )
            .unwrap(),
            *bytes
        );
    }
    let replay = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &review.plan.plan_sha256,
        &mut platform(&input, &desired),
    )
    .unwrap();
    assert_eq!(replay.effects_applied, 0);
    let replay_conservation = replay.actual_conservation.as_ref().unwrap();
    assert!(
        replay_conservation.observed_net_cycle_debit_cycles == 0
            || replay_conservation.observed_net_cycle_credit_cycles == 0
    );
    assert_eq!(
        replay_conservation.observed_starting_cycles
            + replay_conservation.received_new_funding_cycles
            + replay_conservation.observed_net_cycle_credit_cycles,
        replay_conservation.final_controlled_cycles
            + replay_conservation.exact_estate_creation_fee_cycles
            + replay_conservation.observed_net_cycle_debit_cycles
    );
    assert_eq!(
        replay_conservation.observed_starting_cycles,
        conservation.observed_starting_cycles
    );
    assert_eq!(
        read_journal(&paths).unwrap().unwrap().effects,
        completed.effects
    );
    let reset = fleet_ensure_workflow::plan(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        1_800_000_000_000_000_061,
        &mut platform(&input, &desired),
    )
    .expect("review source-bound Root reset");
    assert_eq!(
        reset.plan.scope,
        FleetEnsurePlanScope::RootReinstallPrerequisite
    );
    assert_eq!(reset.plan.operation_id, review.plan.operation_id);
    let operator = Principal::from_text(&desired.operator).unwrap();
    let ledger = Principal::from_text(&desired.cycles_ledger).unwrap();
    let operator_balance = ledger_account_balance(input.pic, ledger, operator);
    let root_balance = ledger_account_balance(input.pic, ledger, input.root);
    std::fs::write(input.adapter_root.join("lose-install-response"), b"once").unwrap();
    let lost = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &reset.plan.plan_sha256,
        &mut platform(&input, &desired),
    );
    assert!(
        matches!(lost, Err(EnsureWorkflowError::Platform(_))),
        "lost Root install response: {lost:?}"
    );
    assert!(input.adapter_root.join("lost-install-response").is_file());
    let reset_complete = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &reset.plan.plan_sha256,
        &mut platform(&input, &desired),
    )
    .expect("recover the same issued Root reset");
    assert!(reset_complete.terminal);
    assert!(reset_complete.actual_conservation.is_some());
    let replay = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &reset.plan.plan_sha256,
        &mut platform(&input, &desired),
    )
    .unwrap();
    assert_eq!(replay.effects_applied, 0);
    assert_eq!(
        std::fs::read_to_string(input.adapter_root.join("reinstall-mutations.log"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert_eq!(
        ledger_account_balance(input.pic, ledger, operator),
        operator_balance
    );
    assert_eq!(
        ledger_account_balance(input.pic, ledger, input.root),
        root_balance
    );
}
