//! Local wire and completion classification; real IC effects are qualified in PocketIC.

use super::*;
use crate::fleet_ensure::{
    ops::capacity_import::prepare_review, policy::capacity_import::tests::plan,
};
use ic_agent::agent::{Envelope, ReplyResponse};
use std::borrow::Cow;

pub fn with_agent(agent: Agent) -> CapacityImportTransport {
    CapacityImportTransport { agent }
}

pub fn root_context() -> canic_core::dto::pool_import::PoolImportContext {
    crate::fleet_ensure::ops::capacity_import::destination::tests::fixture().1
}

pub fn awaiting_handoff_status(
    plan: &CapacityImportPlanRecord,
) -> canic_core::dto::pool_import::PoolImportStatus {
    use canic_core::dto::pool_import::{PoolImportPhase, PoolImportSourceProgress};
    let mut status = crate::fleet_ensure::ops::capacity_import::evidence::tests::settled(plan);
    status.progress = vec![PoolImportSourceProgress::AwaitingHandoff; plan.sources.len()];
    status.phase = PoolImportPhase::Reserved;
    status.paid_calls = 0;
    status.reserved_debit_cycles = 0;
    status.last_root_cycles = plan.root_budget.observed_cycles;
    status.root_receipt = None;
    status
}

pub fn request(
    plan: &CapacityImportPlanRecord,
    canister_id: Principal,
) -> CapacityImportHandoffRequestRecord {
    request_with_expiry(plan, canister_id, 1)
}

pub fn request_with_expiry(
    plan: &CapacityImportPlanRecord,
    canister_id: Principal,
    ingress_expiry: u64,
) -> CapacityImportHandoffRequestRecord {
    let content = EnvelopeContent::Call {
        nonce: Some(plan.plan_sha256.to_vec()),
        ingress_expiry,
        sender: plan.authority.operator,
        canister_id: Principal::management_canister(),
        method_name: "update_settings".into(),
        arg: argument(plan, canister_id).unwrap(),
        sender_info: None,
    };
    let request_id = *ic_agent::to_request_id(&content).unwrap();
    let envelope = Envelope {
        content: Cow::Owned(content),
        sender_pubkey: None,
        sender_sig: None,
        sender_delegation: None,
    };
    CapacityImportHandoffRequestRecord {
        request_id,
        ingress_expiry,
        signed_envelope_hex: hex_bytes(envelope.encode_bytes()),
    }
}

pub fn completion(
    plan: &CapacityImportPlanRecord,
    canister_id: Principal,
    request: &CapacityImportHandoffRequestRecord,
) -> CompletedHandoff {
    completed(
        plan,
        canister_id,
        request,
        RequestStatusResponse::Replied(ReplyResponse {
            arg: candid::encode_args(()).unwrap(),
        }),
    )
    .unwrap()
}

#[test]
fn capacity_import_handoff_rejects_substituted_ingress_and_nonterminal_status() {
    let plan = plan();
    let id = plan.sources[0].binding.canister_id;
    let original = request(&plan, id);
    validate_request(&plan, id, &original).unwrap();
    let mut wrong = original.clone();
    wrong.request_id[0] ^= 1;
    assert!(matches!(
        validate_request(&plan, id, &wrong),
        Err(CapacityImportJournalError::RequestInvalid)
    ));
    wrong = original.clone();
    wrong.ingress_expiry += 1;
    assert!(matches!(
        validate_request(&plan, id, &wrong),
        Err(CapacityImportJournalError::RequestInvalid)
    ));
    for status in [
        RequestStatusResponse::Unknown,
        RequestStatusResponse::Received,
        RequestStatusResponse::Processing,
        RequestStatusResponse::Done,
    ] {
        assert!(matches!(
            completed(&plan, id, &original, status),
            Err(CapacityImportJournalError::Unresolved)
        ));
    }
    let completion = completion(&plan, id, &original);
    assert!(completion.matches(&plan, id, &original));
    wrong.request_id[0] ^= 1;
    assert!(!completion.matches(&plan, id, &wrong));
}

#[test]
fn capacity_import_handoff_signs_only_with_the_reviewed_network_and_operator() {
    let agent = Agent::builder()
        .with_url("http://127.0.0.1:1")
        .with_identity(ic_agent::identity::BasicIdentity::from_raw_key(&[7; 32]))
        .build()
        .unwrap();
    let initial = plan();
    let mut authority = initial.authority;
    let previous_operator = authority.operator;
    authority.operator = agent.get_principal().unwrap();
    authority.network_root_key_sha256 = Sha256::digest(agent.read_root_key()).into();
    let mut sources = initial.sources;
    for controller in &mut sources[0].binding.controllers {
        if *controller == previous_operator {
            *controller = authority.operator;
        }
    }
    let plan = prepare_review(authority, sources, initial.root_budget).unwrap();
    let transport = CapacityImportTransport { agent };
    let id = plan.sources[0].binding.canister_id;
    let signed = transport.prepare(&plan, id).unwrap();
    validate_request(&plan, id, &signed).unwrap();
    let mut changed = plan.authority.clone();
    changed.network_root_key_sha256 = [1; 32];
    let changed = prepare_review(changed, plan.sources, plan.root_budget).unwrap();
    assert!(matches!(
        transport.prepare(&changed, id),
        Err(CapacityImportJournalError::ReaderMismatch)
    ));
}

// Classify wire data for pure journal tests; production authentication stays in Agent.
pub fn rejected(
    plan: &CapacityImportPlanRecord,
    canister_id: Principal,
    request: &CapacityImportHandoffRequestRecord,
) -> RetiredHandoff {
    let certificate = ic_agent::Certificate {
        tree: ic_certification::empty(),
        signature: vec![1],
        delegation: None,
    };
    let status = RequestStatusResponse::Rejected(ic_agent::agent::RejectResponse {
        reject_code: ic_agent::agent::RejectCode::SysTransient,
        reject_message: "transient failure".into(),
        error_code: None,
    });
    match retirement::outcome(plan, canister_id, request, status, &certificate).unwrap() {
        HandoffOutcome::Retired(witness) => witness,
        HandoffOutcome::Completed(_) => panic!("rejected response classified as success"),
    }
}

pub fn retired(
    plan: &CapacityImportPlanRecord,
    canister_id: Principal,
    request: &CapacityImportHandoffRequestRecord,
    status: RequestStatusResponse,
    certified_at_ns: u64,
) -> RetiredHandoff {
    match retirement::outcome(
        plan,
        canister_id,
        request,
        status,
        &timed_certificate(certified_at_ns),
    )
    .unwrap()
    {
        HandoffOutcome::Retired(witness) => witness,
        HandoffOutcome::Completed(_) => panic!("expected terminal evidence without a reply"),
    }
}

fn timed_certificate(mut time: u64) -> ic_agent::Certificate {
    let mut bytes = Vec::new();
    loop {
        let byte = (time & 0x7f) as u8;
        time >>= 7;
        bytes.push(byte | if time == 0 { 0 } else { 0x80 });
        if time == 0 {
            break;
        }
    }
    ic_agent::Certificate {
        tree: ic_certification::labeled(b"time".to_vec(), ic_certification::leaf(bytes)),
        signature: vec![1],
        delegation: None,
    }
}

#[test]
fn capacity_import_retirement_requires_certified_absence_after_expiry() {
    let plan = plan();
    let id = plan.sources[0].binding.canister_id;
    for expiry in [1, 127, 128, u64::MAX - 1] {
        let request = request_with_expiry(&plan, id, expiry);
        for time in [expiry - 1, expiry] {
            assert!(matches!(
                retirement::outcome(
                    &plan,
                    id,
                    &request,
                    RequestStatusResponse::Unknown,
                    &timed_certificate(time)
                ),
                Err(CapacityImportJournalError::Unresolved)
            ));
        }
        let witness = retired(
            &plan,
            id,
            &request,
            RequestStatusResponse::Unknown,
            expiry + 1,
        );
        assert!(witness.matches(&plan, id, &request));
        for status in [
            RequestStatusResponse::Received,
            RequestStatusResponse::Processing,
        ] {
            assert!(matches!(
                retirement::outcome(&plan, id, &request, status, &timed_certificate(expiry + 1)),
                Err(CapacityImportJournalError::Unresolved)
            ));
        }
    }
    let request = request(&plan, id);
    let expired = request.ingress_expiry;
    assert!(matches!(
        retirement::outcome(
            &plan,
            id,
            &request,
            RequestStatusResponse::Unknown,
            &timed_certificate(expired + 300_000_000_000)
        ),
        Ok(HandoffOutcome::Retired(_))
    ));
    assert!(matches!(
        retirement::outcome(
            &plan,
            id,
            &request,
            RequestStatusResponse::Unknown,
            &timed_certificate(expired + 300_000_000_001)
        ),
        Err(CapacityImportJournalError::Unresolved)
    ));
    let witness = retired(&plan, id, &request, RequestStatusResponse::Done, 0);
    assert!(witness.matches(&plan, id, &request));
    for bytes in [
        vec![],
        vec![0x80],
        vec![0xff; 10],
        vec![0x80; 11],
        vec![2, 0],
    ] {
        let mut certificate = timed_certificate(2);
        certificate.tree =
            ic_certification::labeled(b"time".to_vec(), ic_certification::leaf(bytes));
        assert!(matches!(
            retirement::outcome(
                &plan,
                id,
                &request,
                RequestStatusResponse::Unknown,
                &certificate
            ),
            Err(CapacityImportJournalError::Unresolved)
        ));
    }
    let mut incomplete = timed_certificate(2);
    incomplete.tree = ic_certification::empty();
    assert!(matches!(
        retirement::outcome(
            &plan,
            id,
            &request,
            RequestStatusResponse::Unknown,
            &incomplete
        ),
        Err(CapacityImportJournalError::Unresolved)
    ));
}
