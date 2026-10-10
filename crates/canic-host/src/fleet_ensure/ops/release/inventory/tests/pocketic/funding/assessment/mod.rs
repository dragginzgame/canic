//! Signed-query evidence flows through the release assessment without losing account custody facts.

use super::*;
use crate::fleet_ensure::{
    view::release::funding::ReleaseRefillDisposition, workflow::release::assess_funding,
};
use canic_contracts::{
    cycles::Cycles,
    dto::{
        fleet_funding::{
            FleetRootFundingAcceptanceReceipt, FleetRootFundingAcceptanceRequest,
            FleetRootFundingRequest,
        },
        fleet_registry::FleetRegistryVersion,
        icp_refill::{IcpRefillErrorCode, IcpRefillResponse, IcpRefillStatus, IcpRefillTrigger},
        root::RootIcpRefillReleaseEvidence,
    },
};

fn refill(root: Principal, id: u8) -> RootIcpRefillReleaseEvidence {
    RootIcpRefillReleaseEvidence {
        transfer_uncertain: true,
        record_id: u64::from(id),
        trigger: IcpRefillTrigger::Manual,
        policy_hash: [3; 32],
        source_canister: root,
        source_subaccount: Some([4; 32]),
        target_canister: root,
        ledger_canister_id: Principal::from_slice(&[5]),
        cmc_canister_id: Principal::from_slice(&[6]),
        cmc_to_account_owner: Principal::from_slice(&[6]),
        cmc_to_account_subaccount: Some([7; 32]),
        amount_e8s: 100,
        fee_e8s: 10,
        budget_window_start_secs: 20,
        budget_reserved: false,
        memo: vec![8],
        created_at_time_ns: 30,
        notify_attempts: 10,
        response: IcpRefillResponse {
            operation_id: [id; 32],
            status: IcpRefillStatus::Failed,
            ledger_block_index: None,
            cycles_sent: None,
            error_code: Some(IcpRefillErrorCode::TransferWindowStale),
            error_message: None,
        },
        refund_block_index: None,
        transaction_too_old_min_block_index: None,
    }
}

pub(super) fn assert_assessment(
    pic: &PocketIc,
    agent: &Agent,
    runtime: &tokio::runtime::Runtime,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
    mut page: RootFundingReleaseResponse,
) {
    let root = page.fleet_subnet_root;
    let version = FleetRegistryVersion {
        authority: registry.authority.clone(),
        revision: registry.revision,
        content_hash: [4; 32],
    };
    page.current_request = Some(FleetRootFundingRequest {
        operation_id: [9; 32],
        operation_sequence: 1,
        expected_registry: version.clone(),
        observed_balance: Cycles::new(100),
        requested_cycles: Cycles::new(200),
        policy_hash: page.policy_hash,
    });
    page.accepted_grant = Some(FleetRootFundingAcceptanceReceipt {
        request: FleetRootFundingAcceptanceRequest {
            operation_id: [9; 32],
            operation_sequence: 1,
            expected_registry: version,
            observed_balance: Cycles::new(100),
            granted_cycles: Cycles::new(200),
            policy_hash: page.policy_hash,
        },
        fleet_subnet_root: root,
        coordinator: review.authority.coordinator,
        accepted_at_ns: 123,
    });
    let mut coordinator = funding::tests::coordinator_status(registry);
    coordinator.roots[0].current_operation = page.current_request.clone();
    coordinator.roots[0].window.reserved_cycles = Cycles::new(200);
    let coordinator_bytes = candid::encode_one(Ok::<_, Error>(CoordinatorReply::Funding(
        Box::new(coordinator.clone()),
    )))
    .unwrap();
    pic.update_call(
        review.authority.coordinator,
        review.authority.operator,
        "replace_funding",
        coordinator_bytes,
    )
    .unwrap();
    let mut completed = refill(root, 1);
    completed.transfer_uncertain = false;
    completed.response.status = IcpRefillStatus::Completed;
    completed.response.error_code = None;
    completed.response.ledger_block_index = Some(42);
    completed.response.cycles_sent = Some(candid::Nat::from(500_u64));
    let unresolved = refill(root, 2);
    let mut refused = refill(root, 3);
    refused.transfer_uncertain = false;
    page.icp_refills = vec![completed, unresolved, refused];
    let bytes =
        candid::encode_one(Ok::<_, Error>(RootReply::FundingRelease(Box::new(page)))).unwrap();
    pic.update_call(root, review.authority.operator, "replace", bytes.clone())
        .unwrap();
    let evidence = runtime
        .block_on(funding::collect_with_agent(agent, review, registry))
        .unwrap();
    let report = assess_funding(evidence).unwrap();
    assert_eq!(report.evidence.coordinator, coordinator);
    assert_eq!(report.roots[0].coordinator_operations, [[9; 32]]);
    assert_eq!(
        report.roots[0].refills[0].disposition,
        ReleaseRefillDisposition::RecordedConversion {
            ledger_block: 42,
            cycles: 500
        }
    );
    assert_eq!(
        report.roots[0].refills[1].disposition,
        ReleaseRefillDisposition::LedgerReconciliation
    );
    assert_eq!(
        report.roots[0].refills[2].disposition,
        ReleaseRefillDisposition::NoLedgerTransfer
    );
    let retained = report.evidence.roots[0].pages[0].clone();
    assert_eq!(retained.icp_refills[0].source_subaccount, Some([4; 32]));
    assert_eq!(
        candid::encode_one(Ok::<_, Error>(RootReply::FundingRelease(Box::new(
            retained
        ))))
        .unwrap(),
        bytes
    );
}
