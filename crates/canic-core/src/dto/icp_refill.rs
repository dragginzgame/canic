use crate::{cdk::types::Cycles, dto::prelude::*};

pub use crate::domain::icp_refill::{IcpRefillErrorCode, IcpRefillMode, IcpRefillStatus};

///
/// IcpRefillRequest
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct IcpRefillRequest {
    pub operation_id: [u8; 32],
    pub source_canister: Principal,
    pub source_subaccount: Option<[u8; 32]>,
    pub target_canister: Principal,
    pub amount_e8s: u64,
    pub dry_run: bool,
    pub mode: IcpRefillMode,
}

///
/// IcpRefillResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct IcpRefillResponse {
    pub operation_id: [u8; 32],
    pub status: IcpRefillStatus,
    pub ledger_block_index: Option<u64>,
    pub cycles_sent: Option<Nat>,
    pub error_code: Option<IcpRefillErrorCode>,
    pub error_message: Option<String>,
}

///
/// IcpRefillDryRun
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct IcpRefillDryRun {
    pub operation_id: [u8; 32],
    pub mode: IcpRefillMode,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub xdr_permyriad_per_icp: Option<u64>,
    pub estimated_cycles: Option<Cycles>,
    pub message: Option<String>,
}

///
/// IcpRefillEndpointResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
#[remain::sorted]
pub enum IcpRefillEndpointResponse {
    DryRun(IcpRefillDryRun),
    Refill(IcpRefillResponse),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_status_and_error_code_roundtrip_through_candid() {
        let response = IcpRefillResponse {
            operation_id: [7; 32],
            status: crate::domain::icp_refill::IcpRefillStatus::Failed,
            ledger_block_index: Some(42),
            cycles_sent: None,
            error_code: Some(crate::domain::icp_refill::IcpRefillErrorCode::NotifyFailed),
            error_message: Some("bounded failure summary".to_string()),
        };

        let bytes = candid::encode_one(&response).expect("encode ICP refill response");
        let decoded: IcpRefillResponse =
            candid::decode_one(&bytes).expect("decode ICP refill response");

        let dto_status: IcpRefillStatus = crate::domain::icp_refill::IcpRefillStatus::Failed;
        let dto_error_code: IcpRefillErrorCode =
            crate::domain::icp_refill::IcpRefillErrorCode::NotifyFailed;

        assert_eq!(decoded.operation_id, [7; 32]);
        assert_eq!(decoded.status, dto_status);
        assert_eq!(decoded.ledger_block_index, Some(42));
        assert_eq!(decoded.cycles_sent, None);
        assert_eq!(decoded.error_code, Some(dto_error_code));
        assert_eq!(
            decoded.error_message,
            Some("bounded failure summary".to_string())
        );
    }

    #[test]
    fn reexported_mode_roundtrips_through_candid() {
        let request = IcpRefillRequest {
            operation_id: [9; 32],
            source_canister: Principal::from_slice(&[1; 29]),
            source_subaccount: None,
            target_canister: Principal::from_slice(&[2; 29]),
            amount_e8s: 10_000,
            dry_run: true,
            mode: crate::domain::icp_refill::IcpRefillMode::Fabricate,
        };

        let bytes = candid::encode_one(&request).expect("encode ICP refill request");
        let decoded: IcpRefillRequest =
            candid::decode_one(&bytes).expect("decode ICP refill request");

        let dto_mode: IcpRefillMode = crate::domain::icp_refill::IcpRefillMode::Fabricate;

        assert_eq!(decoded.operation_id, [9; 32]);
        assert_eq!(decoded.source_canister, Principal::from_slice(&[1; 29]));
        assert_eq!(decoded.target_canister, Principal::from_slice(&[2; 29]));
        assert_eq!(decoded.amount_e8s, 10_000);
        assert!(decoded.dry_run);
        assert_eq!(decoded.mode, dto_mode);
    }
}
