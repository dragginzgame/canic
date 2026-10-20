//! Passive release provisioning transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::root::{RootProvisioningReleaseKey as Key, RootProvisioningReleaseResponse};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded Request wire projection for release provisioning.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum Request {
    ProvisioningRelease(Option<Key>),
}

/// Bounded Response wire projection for release provisioning.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum Response {
    ProvisioningRelease(RootProvisioningReleaseResponse),
}
