use crate::{cycles::Cycles, dto::prelude::*};

pub use crate::values::icp_refill::{IcpRefillErrorCode, IcpRefillStatus, IcpRefillTrigger};

/// Exact input shared by the Root's preview and durable refill commands.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CycleRefillInput {
    pub operation_id: [u8; 32],
    pub source_subaccount: Option<[u8; 32]>,
    pub amount_e8s: u64,
}

///
/// IcpRefillRequest
///

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct IcpRefillRequest {
    pub operation_id: [u8; 32],
    pub source_subaccount: Option<[u8; 32]>,
    pub amount_e8s: u64,
    pub dry_run: bool,
}

///
/// IcpRefillResponse
///

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct IcpRefillDryRun {
    pub operation_id: [u8; 32],
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub xdr_permyriad_per_icp: Option<u64>,
    pub estimated_cycles: Option<Cycles>,
}

///
/// IcpRefillEndpointResponse
///

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[remain::sorted]
pub enum IcpRefillEndpointResponse {
    DryRun(IcpRefillDryRun),
    Refill(IcpRefillResponse),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_status_and_error_code_roundtrip_through_wire() {
        let response = IcpRefillResponse {
            operation_id: [7; 32],
            status: crate::values::icp_refill::IcpRefillStatus::Failed,
            ledger_block_index: Some(42),
            cycles_sent: None,
            error_code: Some(crate::values::icp_refill::IcpRefillErrorCode::NotifyFailed),
            error_message: Some("bounded failure summary".to_string()),
        };

        let bytes =
            crate::protocol::authority::encode(&response).expect("encode ICP refill response");
        let decoded: IcpRefillResponse =
            crate::protocol::authority::decode(&bytes).expect("decode ICP refill response");

        let dto_status: IcpRefillStatus = crate::values::icp_refill::IcpRefillStatus::Failed;
        let dto_error_code: IcpRefillErrorCode =
            crate::values::icp_refill::IcpRefillErrorCode::NotifyFailed;

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
    fn refill_input_roundtrips_through_native_wire() {
        let input = CycleRefillInput {
            operation_id: [9; 32],
            source_subaccount: None,
            amount_e8s: 10_000,
        };
        let bytes = ic_wire::native::encode_request(
            crate::protocol::cycle_refill::execute::METHOD_ID,
            &input,
            crate::protocol::cycle_refill::REQUEST_LIMITS,
        )
        .expect("encode cycle refill input");
        assert_eq!(
            ic_wire::native::decode_request::<CycleRefillInput>(
                crate::protocol::cycle_refill::execute::METHOD_ID,
                &bytes,
                crate::protocol::cycle_refill::REQUEST_LIMITS
            )
            .expect("decode cycle refill input"),
            input,
        );

        let request = IcpRefillRequest {
            operation_id: [9; 32],
            source_subaccount: None,
            amount_e8s: 10_000,
            dry_run: true,
        };

        let bytes =
            crate::protocol::authority::encode(&request).expect("encode ICP refill request");
        let decoded: IcpRefillRequest =
            crate::protocol::authority::decode(&bytes).expect("decode ICP refill request");

        assert_eq!(decoded.operation_id, [9; 32]);
        assert_eq!(decoded.source_subaccount, None);
        assert_eq!(decoded.amount_e8s, 10_000);
        assert!(decoded.dry_run);
    }
}
