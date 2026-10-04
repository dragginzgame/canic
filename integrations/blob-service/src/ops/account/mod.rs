//! Module: blob_service::ops::account
//!
//! Responsibility: adapt read-only service borrowing and bound account observations.
//! Does not own: account policy, funding or provider responses.
//! Boundary: the shared account workflow owns authorization and correlation.

use crate::ops;

use ic_blob_storage::ops::service::{
    account::{AccountInspectionAccess, AccountInspectionLimits},
    operator::OperatorStores,
};
pub(crate) struct AccountHost;
impl AccountInspectionAccess for AccountHost {
    type Memory = ops::memory::Memory;
    fn with_account_stores<R>(&self, f: impl FnOnce(OperatorStores<'_, Self::Memory>) -> R) -> R {
        ops::read(|stores| f(OperatorStores::from(stores)))
    }
}
pub(crate) fn limits() -> AccountInspectionLimits {
    AccountInspectionLimits {
        max_bytes: 4096.try_into().unwrap(),
        decoding_quota: 500_000.try_into().unwrap(),
        skipping_quota: 1000.try_into().unwrap(),
        max_type_entries: 64.try_into().unwrap(),
    }
}
