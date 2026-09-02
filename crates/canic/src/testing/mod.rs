//! Published host-side qualification support for managed and standalone Canic Apps.
//!
//! This feature-gated module owns test construction only. It does not participate
//! in canister runtime state, admission decisions, lifecycle ownership, or Fleet
//! control-plane authority.

mod managed_app;
mod managed_component_group;

pub use ic_testkit::pic::{
    CandidCallError, CandidCallExt, CanisterInstallExt, PocketIc, PocketIcBuilder,
};
pub use managed_app::{
    ManagedAppFixture, ManagedAppQualificationError, ManagedAppQualificationInput,
    StandaloneAppFixture, install_managed_app, install_standalone_app,
};
pub use managed_component_group::{
    ManagedComponentGroupFixture, ManagedComponentGroupQualificationError,
    ManagedComponentGroupQualificationInput, ManagedComponentNode,
    ManagedRoleQualificationArtifact, install_managed_component_group,
};
