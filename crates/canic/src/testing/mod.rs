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

pub use managed_app::{
    ManagedAppFixture, ManagedAppQualificationError, ManagedAppQualificationInput,
    StandaloneAppFixture, install_managed_app, install_standalone_app,
};
pub use managed_component_group::{
    ManagedApplicationInit, ManagedComponentGroupFixture, ManagedComponentGroupQualificationError,
    ManagedComponentGroupQualificationInput, ManagedComponentNode,
    ManagedRoleQualificationArtifact, install_managed_component_group,
};
pub use {
    ic_platform::pocket_ic::{PocketIc, PocketIcBuilder},
    ic_testkit::pic::CanisterInstallExt,
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
    .map(|owned| {
        // Connect borrows the caller-owned server; no server custody is transferred.
        let (instance, server) = owned.into_parts();
        assert!(server.is_none(), "Connect must not acquire server custody");
        instance
    })
    .map_err(|error| error.to_string())
}

/// Finite reply budgets for framework qualification, including paged fleet views.
/// Transport and canister execution budgets remain independently caller-owned.
pub const CANDID_REPLY_LIMITS: ic_platform::candid::DecodeLimits =
    ic_platform::candid::DecodeLimits {
        reply_bytes: 4 * 1024 * 1024,
        decoding_quota: 100_000_000,
        skipping_quota: 10_000_000,
        type_entries: 10_000,
        header_bytes: 256 * 1024,
    };
