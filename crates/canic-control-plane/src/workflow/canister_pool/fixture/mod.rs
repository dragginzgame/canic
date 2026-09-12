//! Module: workflow::canister_pool::fixture
//!
//! Responsibility: revoke a recycling allocation's source access before physical reset.
//! Does not own: pool claims, target installation, grant replacement or source retirement.
//! Boundary: the existing pending recycling claim fences every Store await.

use crate::{
    ops::{
        canister_pool::CanisterPoolOps, fixture_grant,
        storage::state::root_wasm_store::RootWasmStoreStateOps,
    },
    view::fixture_store::FixtureInstallation,
    workflow::{
        root_authority::validated_root_authority, runtime::template::WasmStoreInternalClient,
    },
};
use canic_core::{
    cdk::types::Principal, control_plane_support::error::InternalError,
    dto::fixture_provisioning::FixtureGrant,
};

pub(super) async fn revoke_before_reset(canister: Principal) -> Result<(), InternalError> {
    let Some(claim) = CanisterPoolOps::pending_recycling_claim(canister)? else {
        return Ok(());
    };
    let Some(store) = RootWasmStoreStateOps::fixture_delivery_store() else {
        return Ok(());
    };
    let (authority, _) = validated_root_authority()?;
    if authority.wasm_store_authority.wasm_store != store {
        return Err(InternalError::conflict());
    }
    let validate = || {
        CanisterPoolOps::require_pending_recycling_claim(canister, &claim)?;
        if RootWasmStoreStateOps::fixture_delivery_store() != Some(store)
            || validated_root_authority()?.0 != authority
        {
            return Err(InternalError::conflict());
        }
        Ok(())
    };
    validate()?;
    let client = WasmStoreInternalClient::new(store);
    let observed = client.fixture_grant(canister).await?;
    validate()?;
    if let Some(grant) = &observed {
        fixture_grant::validate_root(
            &FixtureInstallation {
                store,
                binding: grant.binding.clone(),
                revision: grant.revision,
            },
            &authority,
        )?;
    }
    let Some(request) = fixture_grant::revocation_request(canister, &claim, observed.as_ref())?
    else {
        return Ok(());
    };
    let expected = FixtureGrant {
        revision: request.expected_revision + 1,
        binding: request.binding.clone(),
        enabled: false,
    };
    let result = client.set_fixture_grant(request).await?;
    validate()?;
    if result.ok().as_ref() != Some(&expected) {
        return Err(InternalError::conflict());
    }
    Ok(())
}
