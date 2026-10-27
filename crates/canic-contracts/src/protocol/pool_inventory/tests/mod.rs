//! Native pool pages preserve full asset evidence and cursor identity.
use super::*;
use crate::{
    cycles::Cycles,
    dto::{pool as stored, pool_inventory as wire},
    ids::ComponentInstanceId,
};

#[test]
fn pool_cursor_preserves_identity_and_limit() {
    let request = wire::Request {
        start_after: Some(ic_platform::Principal::from_slice(&[7; 29])),
        limit: 256,
    };
    let bytes = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<wire::Request>(METHOD_ID, &bytes, REQUEST_LIMITS)
            .unwrap(),
        request
    );
    assert!(matches!(
        ic_wire::native::decode_request::<wire::Request>(
            crate::protocol::pool_census::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[test]
fn every_asset_state_retains_claims_receipts_cycles_and_handoff_authority() {
    use stored::CanisterPoolAssetStatus as S;
    let principal = ic_platform::principal::ExternalPrincipal::from_slice(&[9; 29]);
    let claim = stored::CanisterPoolClaim {
        component: ComponentInstanceId::from_generated_bytes([3; 32]),
        operation_id: [4; 32],
    };
    let statuses = [
        S::Store,
        S::StoreDeletionPending {
            operation_id: [2; 32],
        },
        S::PendingReset,
        S::Ready,
        S::Claimed {
            claim: claim.clone(),
        },
        S::Workload {
            claim: claim.clone(),
        },
        S::Recycling {
            claim,
            reset: stored::CanisterPoolRecycleReset::Failed {
                reason: "retained reset failure".into(),
            },
        },
        S::HandingOff {
            recipient: principal,
        },
        S::Failed {
            reason: "retained failure".into(),
        },
    ];
    for status in statuses {
        let original = stored::CanisterPoolAsset {
            canister_id: principal,
            cycles: Cycles::new(u128::MAX),
            origin: stored::CanisterPoolAssetOrigin::Imported,
            status,
            added_at_ns: 5,
            updated_at_ns: 6,
            creation_receipt: Some(stored::CanisterPoolCreationReceipt {
                block_index: u64::MAX,
                operation_id: [8; 32],
                cycles_ledger: principal,
                ledger_amount: Cycles::new(100),
                ledger_fee: Cycles::new(1),
                readiness_floor: Cycles::new(2),
                creation_execution_margin: Cycles::new(3),
                management_creation_fee: Cycles::new(4),
                first_observed_cycles: Some(Cycles::new(5)),
            }),
        };
        let native = wire::Asset::from(original.clone());
        let bytes = ic_wire::native::encode_response(METHOD_ID, &native, RESPONSE_LIMITS).unwrap();
        let decoded: wire::Asset =
            ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
        assert_eq!(stored::CanisterPoolAsset::from(decoded), original);
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn complete_pool_page_has_native_metadata() {
    ic_wire::typescript::native_schema::<wire::Request>().unwrap();
    ic_wire::typescript::native_schema::<Result<wire::Page, crate::dto::error::Error>>().unwrap();
}
