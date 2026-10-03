//! Module: pic::fleet_registry::baseline::tests::funding_inventory
//!
//! Responsibility: qualify controller directory and funding census reads against live fixtures.
//! Boundary: query-only coverage retains member authorization and exact current-head checks.

use candid::{CandidType, Deserialize, Principal, decode_one, encode_one};
use canic::{
    Error,
    dto::component_registry::{
        ComponentDirectoryHead, ComponentDirectoryHeadRequest, ComponentDirectoryPageRequest,
        ComponentDirectoryPageResponse, ComponentLifecycleStatus,
    },
    ids::{ComponentBinding, ComponentChildBinding},
    protocol,
};
use canic_control_plane::dto::root::{RootFundingReleaseResponse, RootFundingStatusResponse};
use ic_testkit::pic::PocketIc;

#[derive(CandidType)]
enum Request {
    ComponentDirectoryHead(ComponentDirectoryHeadRequest),
    ComponentDirectoryPage(Box<ComponentDirectoryPageRequest>),
    FundingRelease(Option<u64>),
}

#[derive(CandidType, Deserialize)]
enum Response {
    ComponentDirectoryHead(ComponentDirectoryHead),
    ComponentDirectoryPage(ComponentDirectoryPageResponse),
    FundingRelease(Box<RootFundingReleaseResponse>),
}

/// Observe the real Ledger/CMC completion through the controller-only release census.
pub(super) fn assert_release_funding(
    pic: &PocketIc,
    root: Principal,
    expected: &RootFundingStatusResponse,
) {
    let balance = pic.cycle_balance(root);
    let read = |caller, cursor| {
        query(
            pic,
            root,
            caller,
            protocol::CANIC_ROOT_STATUS,
            Request::FundingRelease(cursor),
        )
    };
    let Response::FundingRelease(page) = read(Principal::anonymous(), None).unwrap() else {
        panic!("expected release funding census");
    };
    assert_eq!(page.fleet_subnet_root, root);
    assert_eq!(page.policy_hash, expected.policy_hash);
    assert_eq!(page.policy_generation, expected.policy_generation);
    assert_eq!(page.icp_refill_policy, expected.icp_refill_policy);
    assert_eq!(page.current_request, expected.current_operation);
    assert_eq!(page.accepted_grant, None);
    assert_eq!(page.rotation_current, None);
    assert_eq!(page.icp_refills.len(), 1);
    assert_eq!(page.next_after, None);
    let entry = &page.icp_refills[0];
    assert!(!entry.transfer_uncertain);
    let refill = expected
        .latest_icp_refill
        .as_ref()
        .expect("real completed refill");
    assert_eq!(entry.response.operation_id, refill.response.operation_id);
    assert_eq!(entry.response.status, refill.response.status);
    assert_eq!(
        entry.response.ledger_block_index,
        refill.response.ledger_block_index
    );
    assert_eq!(entry.response.cycles_sent, refill.response.cycles_sent);
    assert_eq!(entry.amount_e8s, refill.amount_e8s);
    assert_eq!(entry.source_canister, root);
    assert_eq!(entry.source_subaccount, None);
    assert_eq!(entry.target_canister, root);
    assert_eq!(
        entry.ledger_canister_id,
        Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap()
    );
    assert_eq!(
        entry.cmc_canister_id,
        Principal::from_text("rkp4c-7iaaa-aaaaa-aaaca-cai").unwrap()
    );
    assert_eq!(entry.refund_block_index, None);
    let Response::FundingRelease(replayed) = read(Principal::anonymous(), None).unwrap() else {
        panic!("expected release funding replay");
    };
    assert_eq!(encode_one(&page).unwrap(), encode_one(&replayed).unwrap());
    let Response::FundingRelease(end) =
        read(Principal::anonymous(), Some(entry.record_id)).unwrap()
    else {
        panic!("expected final release funding page");
    };
    assert!(end.icp_refills.is_empty());
    assert_eq!(end.next_after, None);
    let denied = read(Principal::from_slice(&[99; 29]), None)
        .err()
        .expect("controller required");
    assert_eq!(
        denied.code(),
        canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
    );
    assert_eq!(pic.cycle_balance(root), balance);
}

pub(super) fn assert_controller_directory(
    pic: &PocketIc,
    root: Principal,
    controller: Principal,
    component: &ComponentBinding,
    child: &ComponentChildBinding,
) {
    let before = pic.cycle_balance(root);
    let Response::ComponentDirectoryHead(head) = query(
        pic,
        root,
        controller,
        protocol::CANIC_ROOT_STATUS,
        Request::ComponentDirectoryHead(ComponentDirectoryHeadRequest {
            component: component.component,
        }),
    )
    .unwrap() else {
        panic!("expected directory head")
    };
    assert_eq!(head.provenance.component, *component);
    assert_eq!(head.descendant_count, 1);
    let request = ComponentDirectoryPageRequest {
        directory: head,
        parent_canister_id: None,
        role: None,
        status: None,
        cursor: None,
        limit: 1,
    };
    for (caller, method) in [
        (controller, protocol::CANIC_ROOT_STATUS),
        (component.canister_id, protocol::CANIC_PUBLIC_STATUS),
    ] {
        let Response::ComponentDirectoryPage(page) = query(
            pic,
            root,
            caller,
            method,
            Request::ComponentDirectoryPage(Box::new(request.clone())),
        )
        .unwrap() else {
            panic!("expected directory page")
        };
        assert_eq!(page.directory, request.directory);
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].binding, *child);
        assert_eq!(page.entries[0].status, ComponentLifecycleStatus::Active);
        assert_eq!(page.next_cursor, None);
    }
    for caller in [component.canister_id, Principal::from_slice(&[99; 29])] {
        assert!(
            matches!(query(pic, root, caller, protocol::CANIC_ROOT_STATUS,
            Request::ComponentDirectoryPage(Box::new(request.clone()))), Err(error)
            if error.code() == canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code())
        );
    }
    assert!(
        matches!(query(pic, root, controller, protocol::CANIC_PUBLIC_STATUS,
        Request::ComponentDirectoryPage(Box::new(request.clone()))), Err(error)
        if error.code() == canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code())
    );
    let mut stale = request.clone();
    stale.directory.provenance.component_registry_revision += 1;
    assert!(
        query(
            pic,
            root,
            controller,
            protocol::CANIC_ROOT_STATUS,
            Request::ComponentDirectoryPage(Box::new(stale))
        )
        .is_err()
    );
    let mut invalid = request;
    invalid.limit = 0;
    assert!(
        query(
            pic,
            root,
            controller,
            protocol::CANIC_ROOT_STATUS,
            Request::ComponentDirectoryPage(Box::new(invalid))
        )
        .is_err()
    );
    assert_eq!(pic.cycle_balance(root), before);
}

fn query(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    method: &str,
    request: Request,
) -> Result<Response, Error> {
    decode_one(
        &pic.query_call(root, caller, method, encode_one(request).unwrap())
            .unwrap(),
    )
    .unwrap()
}

#[derive(CandidType)]
enum Command {
    InspectCanister(canic::dto::canister::CanisterInspectionRequest),
    ObserveCanister(canic::dto::observability::FleetCanisterObservabilityRequest),
}

#[derive(CandidType, Deserialize)]
enum CommandResponse {
    InspectCanister(Box<canic::dto::canister::CanisterStatusResponse>),
    ObserveCanister(canic::dto::observability::CanisterObservabilityResponse),
}

/// Exercise the two production observation commands against installed parent and child code.
pub(super) fn assert_recovery_observations(
    pic: &PocketIc,
    root: Principal,
    controller: Principal,
    component: &ComponentBinding,
    child: &ComponentChildBinding,
) {
    for target in [component.canister_id, child.canister_id] {
        let response: Result<CommandResponse, Error> = decode_one(
            &pic.update_call(
                root,
                controller,
                protocol::CANIC_ROOT_COMMAND,
                encode_one(Command::InspectCanister(
                    canic::dto::canister::CanisterInspectionRequest {
                        canister_id: target,
                    },
                ))
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let CommandResponse::InspectCanister(status) = response.unwrap() else {
            panic!("unexpected inspection response");
        };
        assert_eq!(status.settings.controllers, vec![root]);
        assert_eq!(
            status.status,
            canic::dto::canister::CanisterStatusType::Running
        );
        assert!(status.module_hash.is_some());
        assert!(status.cycles.0 > 0u8.into());
    }
    let response: Result<CommandResponse, Error> = decode_one(
        &pic.update_call(
            root,
            controller,
            protocol::CANIC_ROOT_COMMAND,
            encode_one(Command::ObserveCanister(
                canic::dto::observability::FleetCanisterObservabilityRequest {
                    canister_id: component.canister_id,
                    request: canic::dto::observability::CanisterObservabilityRequest::ChildFunding(
                        child.canister_id,
                    ),
                },
            ))
            .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let CommandResponse::ObserveCanister(
        canic::dto::observability::CanisterObservabilityResponse::ChildFunding(usage),
    ) = response.unwrap()
    else {
        panic!("unexpected funding ledger response");
    };
    assert_eq!(usage.parent, component.canister_id);
    assert_eq!(usage.child, child.canister_id);
    assert_eq!(usage.pending_operations, 0);
    assert_eq!(
        usage.reserved_cycles.map(|cycles| cycles.to_u128()),
        Some(0)
    );
    assert!(usage.last_accounted_at_secs <= usage.observed_at_ns / 1_000_000_000);
}
