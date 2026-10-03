//! Module: view::provisioning_release
//!
//! Responsibility: carry read-only pages from the existing provisioning operation journal.
//! Does not own: admission, persistence or settlement decisions.
//! Boundary: retain storage identities for projection by the release observation owner.

use crate::storage::stable::component_provisioning::{
    RootComponentOperationKey, RootComponentOperationRecord,
};

/// One existing stable row and whether another key follows it.
pub struct RootComponentOperationReleasePageView {
    pub entry: Option<RootComponentOperationEntryView>,
    pub has_more: bool,
}

/// Exact key and value observed together from one persisted operation row.
pub struct RootComponentOperationEntryView {
    pub key: RootComponentOperationKey,
    pub record: RootComponentOperationRecord,
}
