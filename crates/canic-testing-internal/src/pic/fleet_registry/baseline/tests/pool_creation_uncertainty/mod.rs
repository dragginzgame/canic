//! Real Root retries preserve a debited creation until the Ledger returns its exact principal.

use super::*;
use canic_core::dto::pool::{CanisterPoolCreation, CanisterPoolCreationProgress};

/// Typed faults at the test Ledger's reply boundary; no production fault path is added.
#[derive(CandidType, Clone, Copy, Debug)]
pub(super) enum CreationRetryRefusal {
    InsufficientFunds,
    CreatedInFuture,
    TemporarilyUnavailable,
    FailedToCreate,
    GenericError,
}

pub(super) fn set_refusal(
    pic: &PocketIc,
    ledger: Principal,
    refusal: Option<CreationRetryRefusal>,
) {
    let (): () = pic
        .update_candid(ledger, "set_creation_retry_refusal", (refusal,))
        .unwrap();
}

fn uncertain(fixture: &MainnetRefillFixture) -> CanisterPoolCreation {
    let creation = root_pool_status(&fixture.pic, fixture.root)
        .pending_creation
        .unwrap();
    assert_eq!(
        creation.progress,
        CanisterPoolCreationProgress::Intent {
            uncertain_result: true
        }
    );
    creation
}

#[test]
pub(super) fn uncertain_creation_retains_custody_across_retry_refusals() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let fixture = build_mainnet_refill_fixture(MainnetRefillScenario {
        first_response_pending: true,
        hold_creation_retries: true,
        required_ready_assets: 1,
    });
    for _ in 0..4 {
        if root_pool_status(&fixture.pic, fixture.root)
            .pending_creation
            .is_some_and(|creation| {
                matches!(
                    creation.progress,
                    CanisterPoolCreationProgress::Intent {
                        uncertain_result: true
                    }
                )
            })
        {
            break;
        }
        root_command(
            &fixture.pic,
            fixture.root,
            RootCommandFragment::MaintainPool,
        )
        .unwrap();
    }
    let original = uncertain(&fixture);
    for refusal in [
        CreationRetryRefusal::InsufficientFunds,
        CreationRetryRefusal::CreatedInFuture,
        CreationRetryRefusal::TemporarilyUnavailable,
        CreationRetryRefusal::FailedToCreate,
        CreationRetryRefusal::GenericError,
    ] {
        set_refusal(&fixture.pic, fixture.cycles_ledger, Some(refusal));
        let response = root_command(
            &fixture.pic,
            fixture.root,
            RootCommandFragment::MaintainPool,
        )
        .unwrap();
        assert!(
            matches!(response, RootCommandResponseFragment::MaintainPool(PoolMaintenanceResponse::RefillPending { operation_id, uncertain_result: true }) if operation_id == original.operation_id),
            "retry refusal {refusal:?} must retain the uncertain creation"
        );
        let retained = uncertain(&fixture);
        assert_eq!(retained.operation_id, original.operation_id);
        assert_eq!(retained.created_at_time_ns, original.created_at_time_ns);
        assert_eq!(retained.ledger_amount, original.ledger_amount);
        assert_eq!(retained.ledger_fee, original.ledger_fee);
        assert_eq!(retained.root, original.root);
        assert_eq!(retained.placement_subnet, original.placement_subnet);
        let amounts: Vec<Nat> = fixture
            .pic
            .query_candid(fixture.cycles_ledger, "requested_amounts", ())
            .unwrap();
        assert_eq!(amounts, vec![Nat::from(original.ledger_amount.to_u128())]);
    }
    set_refusal(&fixture.pic, fixture.cycles_ledger, None);
    converge_mainnet_refill(&fixture, 1);
    assert!(
        root_pool_status(&fixture.pic, fixture.root)
            .pending_creation
            .is_none()
    );
    let requests: u64 = fixture
        .pic
        .query_candid(fixture.cycles_ledger, "request_count", ())
        .unwrap();
    assert_mainnet_refill_result(&fixture, 1, requests);
}
