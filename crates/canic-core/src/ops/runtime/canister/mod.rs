//! Canister operations.
//!
//! This module contains **mechanical helpers** for mutating canisters
//! (create, install, uninstall, delete).
//!
//! These functions:
//! - wrap raw IC management canister calls
//! - encode system-specific conventions (e.g. init argument shapes)
//! - are reusable across multiple workflows
//!
//! They do **not**:
//! - decide *when* operations occur (workflow concern)
//! - enforce *who* may call them (policy concern)
//! - encode topology or lifecycle semantics

use crate::{
    Error, cdk::mgmt::CanisterInstallMode, dto::abi::v1::CanisterInitPayload, ops::ic::install_code,
};
use candid::Principal;

/// Install or reinstall a *Canic-style* canister using the standard
/// `(CanisterInitPayload, Option<Vec<u8>>)` argument convention.
///
/// This is a **mechanical helper**, not a lifecycle or policy decision.
///
/// Responsibilities:
/// - Wraps the raw IC `install_code` call
/// - Encodes the canonical init argument shape used by Canic canisters
/// - Performs no authority checks
/// - Makes no assumptions about *when* or *why* installation occurs
///
/// Callers are responsible for:
/// - Ensuring the caller is authorized (e.g. root vs non-root)
/// - Ensuring the target canister role is compatible with this installer
/// - Handling rollback and lifecycle semantics on failure
///
/// This function is intentionally reusable by:
/// - provisioning flows
/// - repair / reinstallation flows
/// - future migration or upgrade workflows
pub async fn install_code_with_extra_arg(
    mode: CanisterInstallMode,
    canister_pid: Principal,
    wasm: &[u8],
    payload: CanisterInitPayload,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    install_code(mode, canister_pid, wasm, (payload, extra_arg)).await
}
