//! Passive component initialization transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::component_registry::RootComponentInitializationRequest;
use candid::CandidType;

/// Bounded RootCommand wire projection for component initialization.

#[derive(CandidType)]
pub enum RootCommand<'a> {
    BindComponentInitialization(&'a RootComponentInitializationRequest),
}
