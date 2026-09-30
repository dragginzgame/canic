//! Reopen a durably issued but unsent, expired ingress through real certified request status.

use super::*;
use canic_host::fleet_ensure::{
    model::capacity_import::CapacityImportHandoffRequestRecord,
    ops::capacity_import::observation::{
        CapacityImportObserver, PreparedCapacityImportObservation,
    },
};
use std::time::SystemTime;

#[derive(CandidType)]
struct UpdateControllers {
    canister_id: Principal,
    settings: Controllers,
    sender_canister_version: Option<u64>,
}

#[derive(CandidType)]
struct Controllers {
    controllers: Option<Vec<Principal>>,
}

pub(super) fn retain_expired_intent(
    store: &CapacityImportJournalStore,
    icp: &IcpCli,
    runtime: &tokio::runtime::Runtime,
    transport: &CapacityImportTransport,
) -> [u8; 32] {
    let staged = store.read().unwrap().unwrap();
    let plan = &staged.plan;
    let source = plan.sources[0].binding.canister_id;
    let mut reader = CapacityImportLiveObserver::from_icp(icp).unwrap();
    let prepared = runtime.block_on(reader.prepare_destination(plan)).unwrap();
    let charged = publication::reserve_inspection(&staged, plan.authority.root).unwrap();
    store.save(&charged).unwrap();
    let destination = runtime.block_on(prepared.observe()).unwrap();
    let prepared = runtime
        .block_on(reader.prepare_source(plan, source))
        .unwrap();
    let charged = publication::reserve_inspection(&charged, source).unwrap();
    store.save(&charged).unwrap();
    let observed = runtime.block_on(prepared.observe()).unwrap();
    let approved = journal::approve(
        &charged,
        plan.plan_sha256,
        &destination,
        std::slice::from_ref(&observed),
    )
    .unwrap();
    store.save(&approved).unwrap();
    let prepared = runtime
        .block_on(transport.prepare_reserve_root(&approved))
        .unwrap();
    let charged = publication::reserve_submission(&approved, "reserve").unwrap();
    store.save(&charged).unwrap();
    let status = runtime.block_on(prepared.submit()).unwrap();
    let reserved =
        journal::reserve(&charged, reservation_evidence(plan, &status).unwrap()).unwrap();
    store.save(&reserved).unwrap();
    let agent = icp.authenticated_agent().unwrap();
    let signed = agent
        .update(&Principal::management_canister(), "update_settings")
        .with_effective_canister_id(source)
        .with_arg(
            candid::encode_one(UpdateControllers {
                canister_id: source,
                settings: Controllers {
                    controllers: Some(plan.transitional_controllers.clone()),
                },
                sender_canister_version: None,
            })
            .unwrap(),
        )
        .expire_at(SystemTime::now() - Duration::from_secs(30))
        .sign()
        .unwrap();
    let request_id = *signed.request_id;
    let intent = journal::prepare_handoff(
        &reserved,
        &observed,
        CapacityImportHandoffRequestRecord {
            request_id,
            ingress_expiry: signed.ingress_expiry,
            signed_envelope_hex: hex_bytes(signed.signed_update),
        },
    )
    .unwrap();
    store.save(&intent).unwrap();
    let charged = publication::reserve_submission(&intent, "0:handoff").unwrap();
    let issued = journal::issue_handoff(&charged, source).unwrap();
    store.save(&issued).unwrap();
    // Simulate process death after intent publication but before the HTTP submission.
    assert!(matches!(
        runtime
            .block_on(transport.completion(&issued, source))
            .unwrap(),
        HandoffOutcome::Retired(_)
    ));
    assert_eq!(store.read().unwrap().unwrap(), issued);
    request_id
}
