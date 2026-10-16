//! Published host-side qualification support for managed and standalone Canic Apps.
//!
//! This feature-gated module owns test construction only. It does not participate
//! in canister runtime state, admission decisions, lifecycle ownership, or Fleet
//! control-plane authority.

mod caller_authority;
mod managed_app;
mod managed_component_group;

use std::time::Duration;

use ic_testkit::pic::PocketIcStartupConfig;

pub use ic_testkit::pic::{
    CandidCallError, CandidCallExt, CanisterInstallExt, PocketIc, PocketIcBuilder,
};
pub use managed_app::{
    ManagedAppFixture, ManagedAppQualificationError, ManagedAppQualificationInput,
    StandaloneAppFixture, install_managed_app, install_standalone_app,
};
pub use managed_component_group::{
    ManagedApplicationInit, ManagedComponentGroupFixture, ManagedComponentGroupQualificationError,
    ManagedComponentGroupQualificationInput, ManagedComponentNode,
    ManagedRoleQualificationArtifact, install_managed_component_group,
};

/// Borrow the governed caller's server; never discover or launch a hidden child.
fn build_pocketic() -> Result<PocketIc, String> {
    let server_url = std::env::var("IC_TESTKIT_POCKET_IC_URL")
        .map_err(|error| format!("caller-owned PocketIC server URL is required: {error}"))?;
    PocketIcStartupConfig::connect(&server_url, Duration::from_secs(30))
        .build_instance(PocketIcBuilder::new().with_application_subnet())
        .map(|owned| owned.instance)
        .map_err(|error| error.to_string())
}

/// Finite reply budgets for framework qualification, including paged fleet views.
/// Transport and canister execution budgets remain independently caller-owned.
pub const CANDID_REPLY_LIMITS: ic_testkit_candid::DecodeLimits = ic_testkit_candid::DecodeLimits {
    reply_bytes: 4 * 1024 * 1024,
    decoding_quota: 100_000_000,
    skipping_quota: 10_000_000,
    type_entries: 10_000,
    header_bytes: 256 * 1024,
};
