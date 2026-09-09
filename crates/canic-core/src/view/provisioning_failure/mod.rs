//! Module: view::provisioning_failure
//!
//! Responsibility: carry one exact originating failure through internal orchestration.
//! Does not own: public error envelopes, persistence, or retry scheduling.
//! Boundary: the first failing owner supplies the target and operation identity.

use crate::domain::provisioning_failure::{ProvisioningFailureStage, ProvisioningRetryCategory};
use candid::Principal;

/// Bounded context attached to an internal failure without exposing it publicly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProvisioningFailureView {
    /// Set when the originating owner records the failure durably.
    pub recorded_at_ns: Option<u64>,
    pub stage: ProvisioningFailureStage,
    pub target: Principal,
    pub operation_id: [u8; 32],
    pub diagnostic_code: u16,
    pub retry_category: ProvisioningRetryCategory,
}
