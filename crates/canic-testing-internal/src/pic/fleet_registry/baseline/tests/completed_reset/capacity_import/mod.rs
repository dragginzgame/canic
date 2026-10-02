//! Exercise the operator review/apply entrypoint on a genuinely completed current Fleet.

use super::*;
use canic_core::{
    control_plane_support::{error::InternalError, policy::pool_import},
    dto::pool_import::{
        PoolImportCommand, PoolImportIdentity, PoolImportPhase, PoolImportSourceProgress,
    },
};
use canic_host::{
    fleet_ensure::{
        dto::capacity_import::CapacityImportReviewRequest,
        model::capacity_import::CapacityImportJournalRecord,
        ops::capacity_import::{
            admission::{
                CapacityImportDeclaration, CapacityImportDeclarations,
                CapacityImportDispositionKind,
            },
            publication,
        },
        workflow::capacity_import::review,
    },
    icp::IcpCli,
};

/// Retain a real unfinished Root import with cleared, stopped and untouched sources.
pub(super) fn pause_reset(input: &ReinstallJourney<'_>, journal: &CapacityImportJournalRecord) {
    use canic_host::fleet_ensure::ops::capacity_import::{
        journal as persistence, reservation_evidence, root_reservation,
    };
    let operator = journal.plan.authority.operator;
    let owner = CapacityImportJournalStore::open(&paths(input)).unwrap();
    let mut approved = journal.clone();
    approved.approved = true;
    owner.save(&approved).unwrap();
    let reservation = root_reservation(&approved.plan).unwrap();
    let identity = PoolImportIdentity {
        sequence: reservation.sequence,
        plan_sha256: reservation.plan_sha256,
    };
    let reserved = super::super::capacity_import::command(
        input.pic,
        input.root,
        operator,
        PoolImportCommand::Reserve(Box::new(reservation)),
    )
    .unwrap();
    approved = persistence::reserve(
        &approved,
        reservation_evidence(&approved.plan, &reserved).unwrap(),
    )
    .unwrap();
    owner.save(&approved).unwrap();
    let running = approved
        .plan
        .sources
        .iter()
        .enumerate()
        .filter(|(_, source)| source.binding.module_sha256.is_some() && !source.binding.stopped)
        .map(|(index, source)| (index, source.binding.canister_id))
        .collect::<Vec<_>>();
    assert!(running.len() >= 2);
    let (cleared_index, cleared) = running[0];
    for _ in 0..8 {
        let status = super::super::capacity_import::command(
            input.pic,
            input.root,
            operator,
            PoolImportCommand::Advance {
                identity,
                canister_id: cleared,
            },
        )
        .unwrap();
        if matches!(
            status.progress[cleared_index],
            PoolImportSourceProgress::Ready(_)
        ) {
            break;
        }
    }
    let (stopped_index, stopped) = running[1];
    let status = super::super::capacity_import::command(
        input.pic,
        input.root,
        operator,
        PoolImportCommand::Advance {
            identity,
            canister_id: stopped,
        },
    )
    .unwrap();
    assert!(matches!(
        status.progress[cleared_index],
        PoolImportSourceProgress::Ready(_)
    ));
    assert_eq!(
        status.progress[stopped_index],
        PoolImportSourceProgress::StopIssued
    );
    assert!(
        status
            .progress
            .iter()
            .any(|entry| matches!(entry, PoolImportSourceProgress::AwaitingHandoff))
    );
    assert!(status.root_receipt.is_none());
    assert!(
        input
            .pic
            .canister_status(cleared, Some(input.root))
            .unwrap()
            .module_hash
            .is_none()
    );
    let stopped = input
        .pic
        .canister_status(stopped, Some(input.root))
        .unwrap();
    assert!(stopped.module_hash.is_some());
    let status: canic_core::dto::canister::CanisterStatusType =
        candid::decode_one(&encode_one(stopped.status).unwrap()).unwrap();
    assert_eq!(
        status,
        canic_core::dto::canister::CanisterStatusType::Stopped
    );
    assert!(approved.handoffs.iter().all(|entry| entry.effect.is_none()));
}

pub(super) fn qualify_reset_review(
    input: &ReinstallJourney<'_>,
    journal: &CapacityImportJournalRecord,
) {
    let plan = &journal.plan;
    let (workloads, ready) = expected_counts(input);
    assert_eq!(plan.sources.len(), workloads + ready);
    for source in &plan.sources {
        let status = input
            .pic
            .canister_status(source.binding.canister_id, Some(input.root))
            .unwrap();
        assert_eq!(
            source.binding.module_sha256.map(Vec::from),
            status.module_hash
        );
    }
    let operator = plan.authority.operator;
    let context = super::super::capacity_import::context(input.pic, input.root, operator);
    assert_eq!(
        plan.root_budget.maximum_paid_calls,
        pool_import::recommended_calls(plan.sources.len()).unwrap()
    );
    assert!(
        plan.root_budget.maximum_debit_cycles
            >= pool_import::required_debit(
                context.maximum_call_debit_cycles,
                plan.root_budget.maximum_paid_calls
            )
            .unwrap()
    );
    let mut insufficient =
        canic_host::fleet_ensure::ops::capacity_import::root_reservation(plan).unwrap();
    insufficient.maximum_root_debit_cycles = pool_import::required_debit(
        context.maximum_call_debit_cycles,
        insufficient.maximum_paid_calls,
    )
    .unwrap()
        - 1;
    let before = input
        .pools
        .iter()
        .map(|id| input.pic.canister_status(*id, Some(input.root)).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        super::super::capacity_import::command(
            input.pic,
            input.root,
            operator,
            PoolImportCommand::Reserve(Box::new(insufficient))
        )
        .err(),
        Some(InternalError::resource_exhausted().into())
    );
    assert_eq!(
        super::super::capacity_import::context(input.pic, input.root, operator).active_import,
        None
    );
    for (id, original) in input.pools.iter().zip(before) {
        let after = input.pic.canister_status(*id, Some(input.root)).unwrap();
        assert_eq!(after.version, original.version);
        assert_eq!(after.status, original.status);
        assert_eq!(after.module_hash, original.module_hash);
    }
}

pub(super) fn qualify_reset_completion(
    input: &ReinstallJourney<'_>,
    journal: &CapacityImportJournalRecord,
) {
    let plan = &journal.plan;
    let status = super::super::capacity_import::status(
        input.pic,
        input.root,
        plan.authority.operator,
        PoolImportIdentity {
            sequence: plan.authority.import_sequence,
            plan_sha256: plan.plan_sha256,
        },
    );
    assert!(matches!(status.phase, PoolImportPhase::Released { .. }));
    assert_eq!(status.progress.len(), input.pools.len());
    assert!(
        status
            .progress
            .iter()
            .all(|progress| matches!(progress, PoolImportSourceProgress::Ready(_)))
    );
    assert!(status.paid_calls <= plan.root_budget.maximum_paid_calls);
    let receipt = status.root_receipt.unwrap();
    assert!(receipt.observed_debit_cycles <= plan.root_budget.maximum_debit_cycles);
    assert_eq!(
        receipt.retained_cycles + receipt.retained_reserved_cycles + receipt.observed_debit_cycles,
        plan.root_budget.observed_cycles + plan.root_budget.observed_reserved_cycles
    );
    eprintln!(
        "Capacity import qualified: {} sources, {}/{} paid calls, {}/{} observed/maximum Root debit cycles",
        status.progress.len(),
        status.paid_calls,
        plan.root_budget.maximum_paid_calls,
        receipt.observed_debit_cycles,
        plan.root_budget.maximum_debit_cycles
    );
    assert_eq!(
        CapacityImportJournalStore::open(&paths(input))
            .unwrap()
            .read()
            .unwrap()
            .unwrap()
            .plan,
        *plan
    );
}

pub(super) fn qualify(input: &ReinstallJourney<'_>, desired: &DesiredFleet, icp: &IcpCli) {
    let span = Span::start("completed_fleet_capacity_import");
    let pic = input.pic;
    let operator = Principal::from_text(&desired.operator).unwrap();
    let source = pic.create_canister_on_subnet(None, None, pic.get_subnet(input.root).unwrap());
    pic.add_cycles(source, 20_000_000_000_000);
    pic.set_controllers(source, None, vec![operator]).unwrap();
    let before = pic.canister_status(source, Some(operator)).unwrap();
    let declarations = CapacityImportDeclarations {
        schema_version: 1,
        operator: operator.to_text(),
        network_root_key_sha256: sha256_hex(&pic.root_key().unwrap()),
        canisters: vec![CapacityImportDeclaration {
            canister: source.to_text(),
            subnet: pic.get_subnet(source).unwrap().to_text(),
            controllers: vec![operator.to_text()],
            module_sha256: "empty".into(),
            canister_version: before.version,
            disposition: CapacityImportDispositionKind::Absence,
            no_external_obligations: true,
            no_other_fleet_ownership: true,
            evidence: "New empty test canister, funded explicitly and owned only by the operator"
                .into(),
        }],
    };
    std::fs::write(
        input.adapter_root.join("capacity-import.toml"),
        toml::to_string(&declarations).unwrap(),
    )
    .unwrap();
    let request = CapacityImportReviewRequest {
        funding_credits: Vec::new(),
        environment: "local".into(),
        fleet: desired.fleet.clone(),
        canisters: vec![source],
        root: Some(input.root),
        declarations: "capacity-import.toml".into(),
        policy: "fleet-policy.toml".into(),
        seed: "fleet-seed.toml".into(),
        maximum_source_debit_cycles: 1_000_000_000_000,
        maximum_root_debit_cycles: super::super::capacity_import::context(
            pic, input.root, operator,
        )
        .maximum_call_debit_cycles
        .checked_mul(64)
        .unwrap(),
        maximum_root_paid_calls: 64,
    };
    let planned = review::plan(input.adapter_root, &request, icp)
        .expect("review supplied capacity from the completed current Fleet");
    assert!(!planned.approved);
    assert_eq!(
        pic.canister_status(source, Some(operator)).unwrap().version,
        before.version
    );
    let repeated =
        review::plan(input.adapter_root, &request, icp).expect("reopen the original saved survey");
    assert_eq!(
        repeated, planned,
        "review repeat must preserve balances and approval identity"
    );
    let digest = planned.operation.as_ref().unwrap().review.review_sha256;
    let completed = review::apply(input.adapter_root, "local", &desired.fleet, digest, icp)
        .expect("apply the exact CLI-owned review");
    assert!(publication::completed(&completed));
    let after = pic.canister_status(source, Some(input.root)).unwrap();
    assert!(after.module_hash.is_none());
    assert_eq!(after.settings.controllers, planned.plan.final_controllers);
    let paths =
        canic_host::fleet_ensure::EnsurePaths::under(input.adapter_root, "local", &desired.fleet);
    let before_journal = std::fs::read(paths.plan.with_file_name("capacity-import.json")).unwrap();
    let unavailable = IcpCli::new("/no/such/import-icp", Some("local".into()));
    assert_eq!(
        review::apply(
            input.adapter_root,
            "local",
            &desired.fleet,
            digest,
            &unavailable
        )
        .unwrap(),
        completed
    );
    assert_eq!(
        std::fs::read(paths.plan.with_file_name("capacity-import.json")).unwrap(),
        before_journal
    );
    assert_eq!(
        pic.canister_status(source, Some(input.root))
            .unwrap()
            .version,
        after.version
    );
    span.finish();
}
