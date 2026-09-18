//! Module: fleet_ensure::model::operator_mint
//!
//! Responsibility: own exact operator conversion intent and receipt data.
//! Boundary: records neither authenticate remote evidence nor authorize payment.

#[cfg(test)]
mod tests;

use candid::Principal;
use serde::{Deserialize, Serialize};

///
/// InitialOperatorFundingRequiredRecord
///
/// Model-owned operator shortfall bound to an original retained withdrawal.
/// Used by recovery without introducing a Root-native or estate funding pause.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InitialOperatorFundingRequiredRecord {
    pub action_sha256: String,
    pub cycles_ledger: String,
    pub operator: String,
    pub target: String,
    pub target_principal: String,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub available_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub required_debit_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub shortfall_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub ledger_fee_cycles: u128,
}

///
/// OperatorMintAuthority
///
/// Model-owned operation and network bindings shared by mint intent and receipt.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorMintAuthority {
    pub operation_id: [u8; 32],
    pub plan_sha256: [u8; 32],
    pub funding_review_sha256: [u8; 32],
    pub network_identity_sha256: [u8; 32],
    pub operator: Principal,
    pub icp_ledger: Principal,
    pub cmc: Principal,
    pub cycles_ledger: Principal,
}

///
/// OperatorMintIntentRecord
///
/// Model-owned default-account transfer identity retained before payment effects.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorMintIntentRecord {
    pub authority: OperatorMintAuthority,
    pub amount_e8s: u64,
    pub transfer_fee_e8s: u64,
    pub created_at_time_ns: u64,
    pub deposit_memo: [u8; 32],
}

///
/// OperatorMintReviewRecord
///
/// Model-owned conversion review inside an existing funding review. A retained
/// transfer argument is the durable approval boundary, preceding any submission.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorMintReviewRecord {
    pub intent: OperatorMintIntentRecord,
    pub review_sha256: String,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub transfer_argument: Option<Vec<u8>>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub transfer_outcome: Option<OperatorMintTransferOutcomeRecord>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub notification: Option<OperatorMintNotificationRecord>,
    /// Retained only after both Ledger transactions have been authenticated.
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub receipt: Option<OperatorMintReceiptRecord>,
}

///
/// OperatorMintNotificationRecord
///
/// Model-owned exact notification intent and optional reply. Neither is deposit proof.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorMintNotificationRecord {
    pub icp_block_index: u64,
    pub argument: Vec<u8>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub outcome: Option<OperatorMintNotificationOutcomeRecord>,
}

///
/// OperatorMintReceiptRecord
///
/// Model-owned transfer and deposit facts for funding admission. The transport
/// must authenticate both transactions before policy checks these bindings.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorMintReceiptRecord {
    pub intent: OperatorMintIntentRecord,
    pub icp_block_index: u64,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub deposit_block_index: u128,
    pub destination_owner: Principal,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub destination_subaccount: Option<[u8; 32]>,
    pub deposit_memo: [u8; 32],
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub gross_minted_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub deposit_fee_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub net_credit_cycles: u128,
}

///
/// OperatorMintTransferOutcomeRecord
///
/// Model-owned ICP Ledger reply retained for exact transfer reconciliation.
/// An accepted or duplicate block still requires transaction verification.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperatorMintTransferOutcomeRecord {
    Accepted { block_index: u64 },

    BadFee { expected_fee_e8s: u64 },

    CreatedInFuture,

    Duplicate { block_index: u64 },

    InsufficientFunds { balance_e8s: u64 },

    TooOld { allowed_window_ns: u64 },
}

///
/// OperatorMintNotificationOutcomeRecord
///
/// Model-owned CMC reply evidence. A success locates a deposit, and a refund
/// locates optional ICP evidence; neither admits a credit by itself.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperatorMintNotificationOutcomeRecord {
    InvalidTransaction {
        reason: String,
    },

    Minted {
        #[serde(with = "crate::fleet_ensure::model::u128_text")]
        deposit_block_index: u128,
        #[serde(with = "crate::fleet_ensure::model::u128_text")]
        gross_minted_cycles: u128,
        #[serde(with = "crate::fleet_ensure::model::u128_text")]
        historical_balance_cycles: u128,
    },

    Other {
        code: u64,
        message: String,
    },

    Processing,

    Refunded {
        #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
        refund_block_index: Option<u64>,
        reason: String,
    },

    TransactionTooOld {
        oldest_block_index: u64,
    },
}
