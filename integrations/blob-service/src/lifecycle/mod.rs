//! Module: blob_service::lifecycle
//!
//! Responsibility: bound the nested service configuration after Canic admits the envelope.
//! Does not own: framework restoration, memory bootstrap or deferred service work.
//! Boundary: synchronous lifecycle participants delegate installation and restoration.

use canic::{
    api::public_status::{ApplicationMetricsSampler, PublicStatusApi},
    endpoint::ArgumentLimits,
};
use ic_blob_storage_contracts::dto::configuration::ServiceInstallationInput;

pub const REQUEST_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 4096,
    decoding_quota: 2_000_000,
    skipping_quota: 1024,
    max_type_len: 64,
    max_header_len: 4096,
};
pub const MANIFEST_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 131_072,
    ..REQUEST_LIMITS
};
pub const CONFIGURATION_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 16_384,
    ..REQUEST_LIMITS
};
pub const ENVELOPE_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 65_536,
    max_type_len: 512,
    max_header_len: 16_384,
    ..REQUEST_LIMITS
};

/// Install the blob owner after Canic restores its invariants and memory runtime.
///
/// The host must bound and decode any external input before calling this function.
/// The configured service must equal the actual canister Principal. Failure traps
/// the enclosing installation, including preceding application state changes.
/// This does not replace the host's application metrics sampler.
pub fn install(input: &ServiceInstallationInput) {
    crate::workflow::install(input);
}

/// Restore blob state synchronously, retaining the upstream mutation fence.
///
/// Call from the host's post-upgrade participant before scheduling async work.
/// This neither replaces the host sampler nor installs missing state.
pub fn restore() {
    crate::workflow::restore();
}

/// Dedicated-canister argument adapter used by [`crate::canister!`].
#[doc(hidden)]
pub fn install_from_args() {
    // Canic has already admitted this envelope before restoring its invariants.
    // The participant has no argument parameter: re-read only under the same
    // raw-byte/decoder bounds, then decode the nested service bytes once.
    let (_, application): (canic::dto::abi::v1::CanisterInitPayload, Option<Vec<u8>>) =
        ENVELOPE_LIMITS.read().expect("admitted lifecycle envelope");
    let (input,): (ServiceInstallationInput,) = CONFIGURATION_LIMITS
        .decode(
            application
                .as_deref()
                .expect("service installation arguments"),
        )
        .expect("bounded service configuration");
    install(&input);
    register_blob_sampler();
}

/// Dedicated-canister sampler registration; embedded hosts compose their own.
#[doc(hidden)]
pub fn register_blob_sampler() {
    PublicStatusApi::set_application_sampler(Some(ApplicationMetricsSampler::new(
        crate::metrics::sample,
    )));
}
