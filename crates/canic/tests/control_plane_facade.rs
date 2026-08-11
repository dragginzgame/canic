#![cfg(feature = "control-plane")]

use canic::{
    Error,
    access::{AccessContext, AccessError, AsyncAccessPredicate, async_trait},
    api::canister::component::RootComponentMembershipApi,
    ids::{ManagedCanisterBinding, TemplateChunkingMode, TemplateManifestState},
};

struct PublicAsyncPredicate;

#[async_trait]
impl AsyncAccessPredicate for PublicAsyncPredicate {
    async fn eval(&self, _context: &AccessContext) -> Result<(), AccessError> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "public_async_predicate"
    }
}

fn require_async_predicate<T: AsyncAccessPredicate>() {}

// Confirms the public `canic` facade exposes the full control-plane enum surface.
#[test]
fn control_plane_facade_reexports_template_manifest_enums() {
    let _ = TemplateChunkingMode::Chunked;
    let _ = TemplateManifestState::Approved;
}

#[test]
fn control_plane_facade_exposes_root_membership_and_custom_async_access_contracts() {
    let _: fn(candid::Principal) -> Result<ManagedCanisterBinding, Error> =
        RootComponentMembershipApi::active_member;
    require_async_predicate::<PublicAsyncPredicate>();
}
