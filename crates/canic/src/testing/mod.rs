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

pub use ic_testkit::pic::{CandidCallError, CanisterInstallExt, PocketIc, PocketIcBuilder};
pub use managed_app::{
    ManagedAppFixture, ManagedAppQualificationError, ManagedAppQualificationInput,
    StandaloneAppFixture, install_managed_app, install_standalone_app,
};
pub use managed_component_group::{
    ManagedApplicationInit, ManagedComponentGroupFixture, ManagedComponentGroupQualificationError,
    ManagedComponentGroupQualificationInput, ManagedComponentNode,
    ManagedRoleQualificationArtifact, install_managed_component_group,
};

/// Qualification-only reply ceilings, independent of production canister policy.
pub const CANDID_REPLY_LIMITS: ic_testkit_candid::DecodeLimits = ic_testkit_candid::DecodeLimits {
    reply_bytes: 16 * 1024 * 1024,
    decoding_quota: 100_000_000,
    skipping_quota: 1_000_000,
    type_entries: 4096,
    header_bytes: 256 * 1024,
};

/// Borrow the governed caller's server; never discover or launch a hidden child.
fn build_pocketic() -> Result<PocketIc, String> {
    let server_url = std::env::var("IC_TESTKIT_POCKET_IC_URL")
        .map_err(|error| format!("caller-owned PocketIC server URL is required: {error}"))?;
    PocketIcStartupConfig::Connect {
        server_url,
        timeout: Duration::from_secs(30),
    }
    .build_instance(PocketIcBuilder::new().with_application_subnet())
    .map(|owned| owned.instance)
    .map_err(|error| error.to_string())
}
