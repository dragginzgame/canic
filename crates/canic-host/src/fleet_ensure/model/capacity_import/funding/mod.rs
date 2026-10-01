//! Exact additional funding evidence retained alongside the original capacity observations.
//!
//! These records authorize accounting for a reviewed credit, never a funding transfer.

use crate::fleet_ensure::model::capacity_import::survey::CapacityImportSampleRecord;
use serde::{Deserialize, Serialize};

/// Immutable observation owner from which the import's original debit allowance starts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum CapacityImportFundingOrigin {
    BootstrapSource { plan_sha256: String },
    BootstrapTerminal { plan_sha256: String },
    Survey { request_sha256: [u8; 32] },
}

/// Original custody, exact credited amount and the separately sampled current balance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportFundingCreditRecord {
    pub origin: CapacityImportFundingOrigin,
    pub before: CapacityImportSampleRecord,
    pub observed: CapacityImportSampleRecord,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub credited_cycles: u128,
}
