//! Exercise normal completed-Fleet reset with current custody, recovery and local replay.

use super::*;
use canic_host::{
    fleet_ensure::{
        model::FleetEnsurePlanScope,
        workflow::clean_reinstall::{self, CleanReinstallReport},
    },
    icp::IcpCli,
};

#[expect(
    clippy::too_many_lines,
    reason = "one real reset retains review, interruption, conservation and replay evidence"
)]
pub(super) fn assert_journey(input: ReinstallJourney<'_>, previous_operation: &str) -> String {
    let span = Span::start("current_custody_clean_reinstall");
    let root = input.adapter_root;
    let desired = input.desired;
    let operator = Principal::from_text(&desired.operator).unwrap();
    let platform = || {
        literal_zero_journey_platform(
            desired,
            input.icp_wrapper,
            root,
            input.local_replica.clone(),
            true,
        )
    };
    let icp = IcpCli::new(
        input.icp_wrapper.to_str().unwrap(),
        Some(desired.environment.clone()),
    )
    .with_cwd(root)
    .with_local_replica(Some(input.local_replica.clone()));
    let ledger = Principal::from_text(&desired.cycles_ledger).unwrap();
    let before_operator = ledger_account_balance(input.pic, ledger, operator);
    let before_root = ledger_account_balance(input.pic, ledger, input.root);
    let controlled = [input.coordinator, input.root, input.store]
        .into_iter()
        .chain(input.pools.iter().copied())
        .collect::<Vec<_>>();
    let before_native = controlled
        .iter()
        .map(|id| input.pic.cycle_balance(*id))
        .sum::<u128>();
    let before_pool = root_pool_status_as(input.pic, input.root, operator);
    let mutation_path = root.join("reinstall-mutations.log");
    let before_mutations = std::fs::read_to_string(&mutation_path).unwrap_or_default();
    let paths = canic_host::fleet_ensure::ops::EnsurePaths::under(
        root,
        &desired.environment,
        &desired.fleet,
    );
    let original_plan = std::fs::read(&paths.plan).unwrap();
    assert!(
        clean_reinstall::selected(root, &desired.environment, &desired.fleet, true, false).unwrap()
    );
    assert!(
        !clean_reinstall::selected(root, &desired.environment, &desired.fleet, false, false)
            .unwrap()
    );
    #[expect(
        clippy::result_large_err,
        reason = "assert the public workflow error without changing its type"
    )]
    let review = || {
        clean_reinstall::review(
            root,
            desired,
            &root.join("fleet-policy.toml"),
            &root.join("fleet-seed.toml"),
            1_800_000_000_000_000_200,
            &mut platform(),
            &icp,
        )
    };
    let CleanReinstallReport::Infrastructure(infrastructure) =
        review().expect("review current custody")
    else {
        panic!("infrastructure must be reviewed first")
    };
    assert_eq!(
        infrastructure.plan.scope,
        FleetEnsurePlanScope::InfrastructureBootstrap
    );
    assert_ne!(infrastructure.plan.operation_id, previous_operation);
    let history = root
        .join(".canic/fleet-ensure/history")
        .join(&desired.environment)
        .join(&desired.fleet);
    assert_eq!(
        std::fs::read(
            history
                .join("objects")
                .join(canic_core::cdk::utils::hash::sha256_hex(&original_plan))
        )
        .unwrap(),
        original_plan
    );
    assert_ne!(std::fs::read(&paths.plan).unwrap(), original_plan);
    let install_count = planned_actions(&infrastructure.plan)
        .into_iter()
        .filter(|action| matches!(action, EnsureAction::Install { .. }))
        .count();
    assert_eq!(install_count, 3);
    let lost_marker = root.join("lost-install-response");
    if lost_marker.exists() {
        std::fs::remove_file(&lost_marker).unwrap();
    }
    std::fs::write(root.join("lose-install-response"), []).unwrap();
    #[expect(
        clippy::result_large_err,
        reason = "assert the public workflow error without changing its type"
    )]
    let apply = |digest: &str| {
        clean_reinstall::apply(
            root,
            &desired.environment,
            &desired.fleet,
            digest,
            &mut platform(),
            &icp,
        )
    };
    if infrastructure
        .plan
        .conservation
        .maximum_operator_debit_cycles
        > 0
    {
        operator_shortfall::assert_fresh_reinstall_rejection(&input, &infrastructure.plan);
        let withdrawals: u64 = input
            .pic
            .query_candid(ledger, "withdrawal_count", ())
            .unwrap();
        let funding_lost = root.join("lost-funding-response");
        if funding_lost.exists() {
            std::fs::remove_file(&funding_lost).unwrap();
        }
        std::fs::write(root.join("lose-funding-response"), []).unwrap();
        let lost_funding = apply(&infrastructure.plan.plan_sha256);
        assert!(
            matches!(lost_funding, Err(EnsureWorkflowError::Platform(_))),
            "lost funding reply: {lost_funding:?}"
        );
        assert!(funding_lost.is_file());
        assert_eq!(
            input
                .pic
                .query_candid::<u64, _>(ledger, "withdrawal_count", ())
                .unwrap(),
            withdrawals + 1
        );
    }
    let lost = apply(&infrastructure.plan.plan_sha256);
    assert!(
        matches!(lost, Err(EnsureWorkflowError::Platform(_))),
        "lost install: {lost:?}"
    );
    assert!(lost_marker.is_file());
    let CleanReinstallReport::Infrastructure(initialized) =
        apply(&infrastructure.plan.plan_sha256).expect("resume the same infrastructure effects")
    else {
        panic!("infrastructure result")
    };
    assert!(initialized.terminal);
    let mut debit = infrastructure
        .plan
        .conservation
        .maximum_operator_debit_cycles;
    let mut maximum_burn = infrastructure
        .plan
        .conservation
        .maximum_execution_burn_cycles;
    let CleanReinstallReport::Import(import) =
        review().expect("review child clearing through fresh Root")
    else {
        panic!("import must follow initialization")
    };
    let import_digest = canic_core::cdk::utils::hash::hex_bytes(
        import.operation.as_ref().unwrap().review.review_sha256,
    );
    let CleanReinstallReport::Import(imported) =
        apply(&import_digest).expect("clear and import exact children")
    else {
        panic!("import result")
    };
    assert!(imported.operation.as_ref().unwrap().publication_complete);
    for child in input.pools {
        let status = input.pic.canister_status(*child, Some(input.root)).unwrap();
        assert!(status.module_hash.is_none());
        assert_eq!(status.memory_metrics.stable_memory_size, Nat::from(0_u8));
    }

    maximum_burn +=
        4_000_000_000_000 + u128::try_from(input.pools.len()).unwrap() * 100_000_000_000;
    let CleanReinstallReport::Fleet(fleet) = review().expect("review fresh workload convergence")
    else {
        panic!("Fleet convergence must follow import")
    };
    assert_eq!(fleet.plan.scope, FleetEnsurePlanScope::Full);
    let mut completed = None;
    for attempt in 0..=64 {
        match apply(&fleet.plan.plan_sha256) {
            Ok(CleanReinstallReport::Fleet(report)) => {
                completed = Some(report);
                break;
            }
            Err(EnsureWorkflowError::ProvisioningRetryPending { .. }) if attempt < 64 => {
                std::thread::sleep(Duration::from_secs(1));
            }
            other => panic!("fresh Fleet convergence: {other:?}"),
        }
    }
    let complete = completed.unwrap();
    assert!(complete.terminal);
    assert_ne!(complete.plan.operation_id, previous_operation);
    let actual = complete.actual_conservation.as_ref().unwrap();
    debit += actual.operator_debit_cycles;
    maximum_burn += fleet.plan.conservation.maximum_execution_burn_cycles;
    assert_eq!(
        ledger_account_balance(input.pic, ledger, operator),
        before_operator - Nat::from(debit)
    );
    assert_eq!(
        ledger_account_balance(input.pic, ledger, input.root),
        before_root
    );
    let after_native = controlled
        .iter()
        .map(|id| input.pic.cycle_balance(*id))
        .sum::<u128>();
    assert!(after_native <= before_native + debit);
    assert!(before_native + debit - after_native <= maximum_burn);
    let after_pool = root_pool_status_as(input.pic, input.root, operator);
    assert_eq!(
        (
            after_pool.workload,
            after_pool.ready,
            after_pool.pending_reset
        ),
        (before_pool.workload, before_pool.ready, 0)
    );
    let mutations = std::fs::read_to_string(&mutation_path).unwrap();
    assert_eq!(
        mutations.lines().count(),
        before_mutations.lines().count() + install_count
    );
    // Model interruption after terminal receipt persistence but before publishing completion.
    let mut interrupted = canic_host::fleet_ensure::ops::read_journal(&paths)
        .unwrap()
        .unwrap();
    interrupted.completion = canic_host::fleet_ensure::model::FleetEnsureCompletion::InProgress;
    canic_host::fleet_ensure::ops::write_journal(&paths, &interrupted).unwrap();
    let unavailable = IcpCli::new(
        "/nonexistent/clean-reinstall-icp",
        Some(desired.environment.clone()),
    );
    let mut offline =
        IcpEnsurePlatform::new(desired.clone(), "/nonexistent/clean-reinstall-icp", root);
    let CleanReinstallReport::Fleet(replay) = clean_reinstall::apply(
        root,
        &desired.environment,
        &desired.fleet,
        &complete.plan.plan_sha256,
        &mut offline,
        &unavailable,
    )
    .expect("terminal replay must not resolve signer or contact the IC") else {
        panic!("Fleet replay")
    };
    assert_eq!(replay.effects_applied, 0);
    assert_eq!(
        canic_host::fleet_ensure::ops::read_journal(&paths)
            .unwrap()
            .unwrap()
            .completion,
        canic_host::fleet_ensure::model::FleetEnsureCompletion::Converged
    );

    assert_eq!(replay.actual_conservation, complete.actual_conservation);
    assert_eq!(std::fs::read_to_string(&mutation_path).unwrap(), mutations);
    span.finish();
    complete.plan.operation_id.clone()
}
