//! Module: pic::fleet_registry::baseline::tests::funding_inventory
//!
//! Responsibility: qualify controller directory and funding census reads against live fixtures.
//! Boundary: query-only coverage retains member authorization and exact current-head checks.

use canic::{
    Error,
    dto::component_registry::{ComponentDirectoryHeadRequest, ComponentLifecycleStatus},
    ids::{ComponentBinding, ComponentChildBinding},
    protocol,
};
use canic_contracts::dto::{
    component_directory::{
        ComponentDirectoryHead, ComponentDirectoryPageRequest, ComponentDirectoryPageResponse,
    },
    root::RootFundingStatusResponse,
};
use ic_platform::pocket_ic::PocketIc;
use ic_platform::principal::ExternalPrincipal as Principal;

/// Observe the real Ledger/CMC completion through the controller-only release census.
pub(super) fn assert_release_funding(
    pic: &PocketIc,
    root: Principal,
    expected: &RootFundingStatusResponse,
) {
    let balance = pic.cycle_balance(root);
    let read = |caller, cursor: Option<u64>| {
        let bytes = pic
            .query_call(
                root,
                caller,
                protocol::funding_read::release::METHOD,
                ic_wire::native::encode_request(
                    protocol::funding_read::release::METHOD_ID,
                    &cursor,
                    protocol::funding_read::REQUEST_LIMITS,
                )
                .unwrap(),
            )
            .expect("funding release Wire transport");
        let response: Result<canic_contracts::dto::funding_read::ReleasePage, Error> =
            ic_wire::native::decode_response(
                protocol::funding_read::release::METHOD_ID,
                &bytes,
                protocol::funding_read::RESPONSE_LIMITS,
            )
            .expect("bounded funding release response");
        response.map(|page| {
            (
                canic_contracts::dto::root::RootFundingReleaseResponse::try_from(page)
                    .expect("canonical funding evidence"),
                bytes,
            )
        })
    };
    let (page, original_reply) = read(Principal::anonymous(), None).unwrap();
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
    let (_, replayed_reply) = read(Principal::anonymous(), None).unwrap();
    assert_eq!(original_reply, replayed_reply);
    let (end, _) = read(Principal::anonymous(), Some(entry.record_id)).unwrap();
    assert!(end.icp_refills.is_empty());
    assert_eq!(end.next_after, None);
    let denied = read(Principal::from_slice(&[99; 29]), None).expect_err("controller required");
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
    assert_public_children(pic, root, component, child);

    let before = pic.cycle_balance(root);
    let head: Result<ComponentDirectoryHead, Error> = ic_testkit::pic::query_wire(
        pic,
        root,
        controller,
        ic_testkit::pic::WireCallSpec {
            method: protocol::component_directory::HEAD,
            method_id: protocol::component_directory::HEAD_ID,
            request_limits: protocol::component_directory::REQUEST_LIMITS,
            response_limits: protocol::component_directory::RESPONSE_LIMITS,
        },
        &ComponentDirectoryHeadRequest {
            component: component.component,
        },
    )
    .unwrap();
    let head = head.unwrap();
    assert_eq!(head.provenance.component, component.clone().into());
    assert_eq!(head.descendant_count, 1);
    let request = ComponentDirectoryPageRequest {
        directory: head,
        parent_canister_id: None,
        role: None,
        status: None,
        cursor: None,
        limit: 1,
    };
    let read = |caller,
                member,
                request: &ComponentDirectoryPageRequest|
     -> Result<ComponentDirectoryPageResponse, Error> {
        let (method, method_id) = if member {
            (
                protocol::component_directory::MEMBER_PAGE,
                protocol::component_directory::MEMBER_PAGE_ID,
            )
        } else {
            (
                protocol::component_directory::PAGE,
                protocol::component_directory::PAGE_ID,
            )
        };
        ic_testkit::pic::query_wire(
            pic,
            root,
            caller,
            ic_testkit::pic::WireCallSpec {
                method,
                method_id,
                request_limits: protocol::component_directory::REQUEST_LIMITS,
                response_limits: protocol::component_directory::RESPONSE_LIMITS,
            },
            request,
        )
        .unwrap()
    };
    for (caller, member) in [(controller, false), (component.canister_id, true)] {
        let page = read(caller, member, &request).unwrap();
        assert_eq!(page.directory, request.directory);
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].binding, child.clone().into());
        assert_eq!(page.entries[0].status, ComponentLifecycleStatus::Active);
        assert_eq!(page.next_cursor, None);
        assert_eq!(page, read(caller, member, &request).unwrap());
    }
    for caller in [component.canister_id, Principal::from_slice(&[99; 29])] {
        assert!(matches!(read(caller, false, &request), Err(error)
            if error.code() == canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()));
    }
    assert!(matches!(read(controller, true, &request), Err(error)
        if error.code() == canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()));
    let mut stale = request.clone();
    stale.directory.provenance.component_registry_revision += 1;
    assert!(read(controller, false, &stale).is_err());
    let mut invalid = request;
    invalid.limit = 0;
    assert!(read(controller, false, &invalid).is_err());
    assert_eq!(pic.cycle_balance(root), before);
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
        let response: Result<canic::dto::inspection::Response, Error> =
            ic_testkit::pic::update_wire(
                pic,
                root,
                controller,
                ic_testkit::pic::WireCallSpec {
                    method: protocol::inspection::METHOD,
                    method_id: protocol::inspection::METHOD_ID,
                    request_limits: protocol::inspection::REQUEST_LIMITS,
                    response_limits: protocol::inspection::RESPONSE_LIMITS,
                },
                &canic::dto::inspection::Request {
                    canister_id: target.into(),
                },
            )
            .unwrap();
        let outcome =
            canic::dto::canister::CanisterInspectionOutcome::try_from(response.unwrap()).unwrap();
        let canic::dto::canister::CanisterInspectionOutcome::Status(status) = outcome else {
            panic!("funded inspection expected")
        };
        assert_eq!(status.settings.controllers, vec![root]);
        assert_eq!(
            status.status,
            canic::dto::canister::CanisterStatusType::Running
        );
        assert!(status.module_hash.is_some());
        assert!(status.cycles.0 > 0u8.into());
    }
    let response: Result<canic::dto::child_funding::ChildFundingUsage, Error> =
        ic_testkit::pic::update_wire(
            pic,
            root,
            controller,
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::root_observation::child_funding::METHOD,
                method_id: canic::protocol::root_observation::child_funding::METHOD_ID,
                request_limits: canic::protocol::root_observation::child_funding::REQUEST_LIMITS,
                response_limits: canic::protocol::root_observation::child_funding::RESPONSE_LIMITS,
            },
            &canic::dto::root_observation::Request {
                canister_id: component.canister_id.into(),
                input: ic_platform::Principal::from(child.canister_id),
            },
        )
        .unwrap();
    let usage: canic::dto::observability::ChildFundingUsage = response.unwrap().into();
    assert_eq!(usage.parent, component.canister_id);
    assert_eq!(usage.child, child.canister_id);
    assert_eq!(usage.pending_operations, 0);
    assert_eq!(
        usage.reserved_cycles.map(|cycles| cycles.to_u128()),
        Some(0)
    );
    assert!(usage.last_accounted_at_secs <= usage.observed_at_ns / 1_000_000_000);
}

fn assert_public_children(
    pic: &PocketIc,
    root: Principal,
    component: &ComponentBinding,
    child: &ComponentChildBinding,
) {
    use canic::dto::{
        canister::CanisterInfo,
        page::{Page, PageRequest},
    };
    let query = |canister, page: PageRequest| {
        let result: Result<Page<CanisterInfo>, Error> = ic_testkit::pic::query_wire(
            pic,
            canister,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: protocol::CANIC_PUBLIC_CHILDREN,
                method_id: protocol::public_children::METHOD_ID,
                request_limits: protocol::public_children::REQUEST_LIMITS,
                response_limits: protocol::public_children::RESPONSE_LIMITS,
            },
            &page,
        )
        .expect("child inventory Wire reply");
        result.expect("child inventory application reply")
    };
    let page = query(
        component.canister_id,
        PageRequest {
            limit: 1,
            offset: 0,
        },
    );
    assert_eq!(page.total, 1);
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].pid, child.canister_id.into());
    assert_eq!(
        page.entries[0].parent_pid,
        Some(component.canister_id.into())
    );
    assert_eq!(page.entries[0].role, child.role);
    let end = query(
        component.canister_id,
        PageRequest {
            limit: 1,
            offset: 1,
        },
    );
    assert_eq!(end.total, page.total);
    assert!(end.entries.is_empty());
    let root_page = query(
        root,
        PageRequest {
            limit: 1,
            offset: 0,
        },
    );
    assert!(root_page.entries.len() <= 1);
    let replay = query(
        root,
        PageRequest {
            limit: 1,
            offset: 0,
        },
    );
    assert_eq!(root_page.total, replay.total);
    assert_eq!(root_page.entries, replay.entries);
}
