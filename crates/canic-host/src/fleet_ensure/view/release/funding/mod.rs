//! Funding facts and assessments retain receipt meaning without granting reset authority.

use crate::fleet_ensure::view::release::FleetReleaseFundingView;
use candid::Principal;
use canic_core::shared_support::icp_refill::{IcpRefillErrorCode, IcpRefillStatus};

/// An exact cycles receipt or an unrepresentable observation; overflow is never zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordedRefillCycles {
    Missing,
    Recorded(u128),
    Overflow,
}

/// Pure facts projected from one retained ICP refill, independent of retry counters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseRefillFacts {
    pub transfer_uncertain: bool,
    pub record_id: u64,
    pub operation_id: [u8; 32],
    pub status: IcpRefillStatus,
    pub error_code: Option<IcpRefillErrorCode>,
    pub ledger_block_index: Option<u64>,
    pub cycles: RecordedRefillCycles,
    pub refund_block_index: Option<u64>,
    pub expired_before_block: Option<u64>,
}

/// A historical receipt is distinct from a required reconciliation or reviewed residual.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseRefillDisposition {
    NoLedgerTransfer,
    RecordedConversion {
        ledger_block: u64,
        cycles: u128,
    },
    RecordedRefund {
        ledger_block: u64,
        refund_block: u64,
    },
    RefundResidualReview {
        ledger_block: u64,
    },
    LedgerReconciliation,
    CmcReconciliation {
        ledger_block: u64,
    },
    InvalidReceipt,
}

/// One exact refill and the remaining evidence it needs, without issuing retry authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseRefillAssessment {
    pub record_id: u64,
    pub operation_id: [u8; 32],
    pub disposition: ReleaseRefillDisposition,
}

/// Current Root/Coordinator work is kept separate from historical ICP receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseRootFundingFacts {
    pub root: Principal,
    pub coordinator_operations: Vec<[u8; 32]>,
    pub rotation_operation: Option<[u8; 32]>,
    pub refills: Vec<ReleaseRefillFacts>,
}

/// Root-local assessment; a pending rotation alone is not a paid-effect classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseRootFundingAssessment {
    pub root: Principal,
    pub coordinator_operations: Vec<[u8; 32]>,
    pub rotation_operation: Option<[u8; 32]>,
    pub refills: Vec<ReleaseRefillAssessment>,
}

/// Keep all observed accounts and transfer identities alongside the assessment.
#[derive(Clone, Debug)]
pub struct FleetReleaseFundingAssessment {
    pub coordinator_rotation_operation: Option<[u8; 32]>,
    pub evidence: FleetReleaseFundingView,
    pub roots: Vec<ReleaseRootFundingAssessment>,
}
