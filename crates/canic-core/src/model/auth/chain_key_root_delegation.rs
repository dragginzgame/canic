//! Module: model::auth::chain_key_root_delegation
//!
//! Responsibility: own internal chain-key root delegation install failure states.
//! Does not own: issuer calls, retry orchestration, or stable-record conversion.
//! Boundary: workflow classifies failures; ops persists their stable diagnostic labels.

/// Failure recorded when one issuer does not install a prepared root proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainKeyRootDelegationInstallFailure {
    CallFailed,
    ExpiredOrSuperseded,
    ProofMismatch,
    RejectedBySigner,
}
