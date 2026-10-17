#![cfg(feature = "control-plane")]

use canic::{
    Error,
    access::{AccessCheck, AccessContext, AccessError},
    api::canister::component::RootComponentMembershipApi,
    dto::component_registry::{RootMembershipRequest, RootMembershipResponse},
    ids::{ManagedCanisterBinding, TemplateChunkingMode, TemplateManifestState},
};

struct PublicAccessCheck;

impl AccessCheck for PublicAccessCheck {
    fn eval(&self, _context: &AccessContext) -> Result<(), AccessError> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "public_access_check"
    }
}

const fn require_access_check<T: AccessCheck>() {}

// Confirms the public `canic` facade exposes the full control-plane enum surface.
#[test]
fn control_plane_facade_reexports_template_manifest_enums() {
    let _ = TemplateChunkingMode::Chunked;
    let _ = TemplateManifestState::Approved;
}

#[test]
fn control_plane_facade_exposes_root_membership_and_custom_access_contracts() {
    let _: fn(candid::Principal) -> Result<ManagedCanisterBinding, Error> =
        RootComponentMembershipApi::active_member;
    let _: fn(RootMembershipRequest) -> Result<RootMembershipResponse, Error> =
        RootComponentMembershipApi::lookup;
    require_access_check::<PublicAccessCheck>();
}
