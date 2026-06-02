use crate::{cdk::types::Cycles, dto::prelude::*};

///
/// IcpRefillMode
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum IcpRefillMode {
    Canister,
    Fabricate,
}

///
/// IcpRefillStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum IcpRefillStatus {
    Completed,
    Failed,
    InvalidTransaction,
    NotifyProcessing,
    Refunded,
    Requested,
    TransactionTooOld,
    Transferred,
}

///
/// IcpRefillErrorCode
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum IcpRefillErrorCode {
    BadFee,
    Duplicate,
    FabricationUnavailable,
    InvalidLedgerBlockIndex,
    InvalidTransaction,
    LedgerTransferFailed,
    NotifyFailed,
    NotifyMaxAttempts,
    Processing,
    RateGateDenied,
    Refunded,
    RequestDenied,
    TransactionTooOld,
    TransferWindowStale,
}

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
