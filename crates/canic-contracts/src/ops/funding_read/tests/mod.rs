//! Complete funding evidence must survive transport without projected-away receipts.
use crate::{
    cycles::Cycles,
    dto::{
        funding_control::RotationReceipt,
        funding_read::{RefillResult, RefillStatus, RootStatus},
        root::RootFundingStatusResponse,
    },
    ids::{CyclesFundingBudget, FleetFundingProfile, FleetSubnetRootFundingPolicy},
    protocol::funding_read as protocol,
    values::icp_refill::{IcpRefillStatus, IcpRefillTrigger},
};

fn status() -> RootStatus {
    let root = ic_platform::Principal::from_slice(&[1]);
    let rotation = RotationReceipt {
        operation_id: [2; 32],
        plan_digest: [3; 32],
        fleet_subnet_root: root,
        predecessor_generation: 4,
        successor_generation: 5,
        prepared: true,
        activated: false,
        recorded_at_ns: 6,
    };
    RootStatus {
        fleet_subnet_root: root,
        lifecycle_status: crate::dto::fleet_registry::FleetSubnetRootStatus::Active,
        funding_eligible: true,
        cycles_funding_enabled: false,
        current_cycles: Cycles::new(u128::MAX),
        policy_generation: 4,
        funding_profile: FleetFundingProfile::SingleSubnet,
        policy_hash: [7; 32],
        root_policy: FleetSubnetRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            request_threshold: Cycles::new(8),
            target_balance: Cycles::new(9),
            cooldown_secs: 10,
            budget: CyclesFundingBudget {
                window_secs: 11,
                maximum_cycles: Cycles::new(12),
            },
            maximum_automatic_grants: 13,
            maximum_automatic_cycles: Cycles::new(14),
        },
        current_operation: None,
        last_result: None,
        historical_automatic_grants: 15,
        historical_automatic_cycles: Cycles::new(16),
        automatic_grants: 17,
        automatic_cycles: Cycles::new(18),
        rotation_current: Some(rotation.clone()),
        rotation_last: Some(RotationReceipt {
            activated: true,
            ..rotation
        }),
        icp_refill_policy: None,
        icp_window_start_secs: Some(19),
        icp_window_reserved_e8s: 20,
        automatic_icp_refills: 21,
        automatic_icp_refill_e8s: 22,
        latest_icp_refill: Some(RefillStatus {
            trigger: IcpRefillTrigger::Manual,
            amount_e8s: 23,
            fee_e8s: 24,
            budget_window_start_secs: 25,
            resumable: true,
            response: RefillResult {
                operation_id: [26; 32],
                status: IcpRefillStatus::Completed,
                ledger_block_index: Some(27),
                cycles_sent: Some(ic_wire::Bytes::new(vec![1; 17]).unwrap()),
                error_code: None,
                error_message: None,
            },
        }),
    }
}

#[test]
fn root_status_preserves_full_evidence_and_unbounded_external_cycles() {
    let original = status();
    let expected = ic_wire::native::encode_response(
        protocol::root_status::METHOD_ID,
        &original,
        protocol::RESPONSE_LIMITS,
    )
    .unwrap();
    let canonical = RootFundingStatusResponse::try_from(original).unwrap();
    assert!(
        canonical
            .latest_icp_refill
            .as_ref()
            .unwrap()
            .response
            .cycles_sent
            .as_ref()
            .unwrap()
            > &ic_platform::candid::contract::Nat::from(u128::MAX)
    );
    assert!(canonical.rotation_current.as_ref().unwrap().prepared);
    assert!(canonical.rotation_last.as_ref().unwrap().activated);
    let native = RootStatus::try_from(canonical).unwrap();
    assert_eq!(
        ic_wire::native::encode_response(
            protocol::root_status::METHOD_ID,
            &native,
            protocol::RESPONSE_LIMITS
        )
        .unwrap(),
        expected
    );
    let decoded: RootStatus = ic_wire::native::decode_response(
        protocol::root_status::METHOD_ID,
        &expected,
        protocol::RESPONSE_LIMITS,
    )
    .unwrap();
    RootFundingStatusResponse::try_from(decoded).unwrap();
}

#[test]
fn refill_status_preserves_missing_and_zero_and_rejects_nonminimal_cycles() {
    for value in [None, Some(Vec::new())] {
        let mut original = status();
        original
            .latest_icp_refill
            .as_mut()
            .unwrap()
            .response
            .cycles_sent = value
            .clone()
            .map(|bytes| ic_wire::Bytes::new(bytes).unwrap());
        let canonical = RootFundingStatusResponse::try_from(original).unwrap();
        assert_eq!(
            canonical
                .latest_icp_refill
                .as_ref()
                .unwrap()
                .response
                .cycles_sent,
            value
                .as_ref()
                .map(|_| ic_platform::candid::contract::Nat::from(0_u8))
        );
        let roundtrip = RootStatus::try_from(canonical).unwrap();
        assert_eq!(
            roundtrip
                .latest_icp_refill
                .unwrap()
                .response
                .cycles_sent
                .map(|bytes| bytes.as_slice().to_vec()),
            value
        );
    }
    let mut invalid = status();
    invalid
        .latest_icp_refill
        .as_mut()
        .unwrap()
        .response
        .cycles_sent = Some(ic_wire::Bytes::new(vec![0, 1]).unwrap());
    assert!(matches!(
        RootFundingStatusResponse::try_from(invalid),
        Err(ic_wire::Error::Malformed)
    ));
}
