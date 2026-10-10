//! Passive capability rpc transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::capability::{
    NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1, RootCapabilityEnvelopeV1,
    RootCapabilityResponseV1,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded RootCommandFragment wire projection for capability rpc.

#[derive(CandidType, Deserialize)]
pub enum RootCommandFragment {
    RespondCapability(RootCapabilityEnvelopeV1),
}

/// Bounded RootCommandResponseFragment wire projection for capability rpc.
#[derive(CandidType, Deserialize)]
pub enum RootCommandResponseFragment {
    RespondCapability(RootCapabilityResponseV1),
}

/// Bounded CanisterCommandFragment wire projection for capability rpc.
#[derive(CandidType)]
pub enum CanisterCommandFragment {
    RespondCapability(NonrootCyclesCapabilityEnvelopeV1),
}

/// Bounded CanisterCommandResponseFragment wire projection for capability rpc.
#[derive(CandidType, Deserialize)]
pub enum CanisterCommandResponseFragment {
    RespondCapability(NonrootCyclesCapabilityResponseV1),
}
