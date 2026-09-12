//! Module: workflow::component_registry::component_installation::fixture
//!
//! Responsibility: issue exact source grants after independently verified installation.
//! Does not own: source publication, grant retirement or application import progress.
//! Boundary: re-read installed allocation and Root authority after every Store await.

use crate::{
    ops::{
        canister_pool::{CanisterPoolClaimKey, CanisterPoolOps},
        component_registry::ComponentRegistryOps,
        fixture_grant,
    },
    view::{
        component_registry::{
            RootComponentAllocationProgressView, RootComponentChildAllocationProgressView,
        },
        fixture_store::{FixtureGrantSelection, FixtureInstallation},
    },
    workflow::{
        component_registry::{
            component_installation::{
                ComponentChildInstallPlan, ComponentInstallPlan,
                validate_installed_component_status,
            },
            query_component_runtime_status,
        },
        root_authority::validated_root_authority,
        runtime::template::WasmStoreInternalClient,
    },
};
use canic_core::{
    control_plane_support::error::InternalError,
    dto::{
        abi::v1::CanisterInitPayload, fixture_provisioning::FixtureGrant,
        root_store::RootStoreBootstrapResponse,
    },
    ids::ManagedCanisterBinding,
};

/// Freeze source access before installation; retries read the existing install intent.
pub(super) async fn select(
    store: &RootStoreBootstrapResponse,
    payload: &mut CanisterInitPayload,
    selection: FixtureGrantSelection,
) -> Result<Option<FixtureInstallation>, InternalError> {
    let Some(mut fixture) = fixture_grant::installation(store, payload)? else {
        return match selection {
            FixtureGrantSelection::Fresh | FixtureGrantSelection::Retained(None) => Ok(None),
            FixtureGrantSelection::Retained(Some(_)) => Err(InternalError::conflict()),
        };
    };
    let revision = match selection {
        FixtureGrantSelection::Fresh => {
            require_current_allocation(&fixture)?;
            let client = WasmStoreInternalClient::new(fixture.store);
            let observed = client
                .fixture_grant(fixture_grant::target_principal(&fixture.binding.target))
                .await?;
            require_current_allocation(&fixture)?;
            fixture_grant::next_revision(&fixture.binding, observed.as_ref())?
        }
        FixtureGrantSelection::Retained(Some(revision)) => revision,
        FixtureGrantSelection::Retained(None) => return Err(InternalError::conflict()),
    };
    fixture_grant::assign_revision(payload, &mut fixture, revision)?;
    Ok(Some(fixture))
}

fn require_current_allocation(fixture: &FixtureInstallation) -> Result<(), InternalError> {
    let component = match &fixture.binding.target {
        ManagedCanisterBinding::Component(binding) => binding.component,
        ManagedCanisterBinding::ComponentChild(binding) => binding.component.component,
    };
    CanisterPoolOps::require_workload_claim(
        fixture_grant::target_principal(&fixture.binding.target),
        &CanisterPoolClaimKey {
            component,
            operation_id: fixture.binding.installation,
        },
    )?;
    ComponentRegistryOps::require_root_store_admin_open()?;
    let (authority, _) = validated_root_authority()?;
    fixture_grant::validate_root(fixture, &authority)
}

pub(super) async fn ensure_component(plan: &ComponentInstallPlan) -> Result<(), InternalError> {
    let Some(fixture) = &plan.fixture else {
        return Ok(());
    };
    ensure(fixture, || {
        let allocation = ComponentRegistryOps::allocation(plan.payload.install_id)
            .ok_or_else(InternalError::conflict)?;
        let RootComponentAllocationProgressView::Installed { installation, .. } =
            allocation.progress
        else {
            return Err(InternalError::conflict());
        };
        if !plan.durable.matches_effect(&installation)
            || allocation.release_set.release_build_id != fixture.binding.release_build_id
        {
            return Err(InternalError::conflict());
        }
        Ok(())
    })
    .await
}

pub(super) async fn ensure_child(plan: &ComponentChildInstallPlan) -> Result<(), InternalError> {
    let Some(fixture) = &plan.fixture else {
        return Ok(());
    };
    // Child binding alone cannot distinguish two installs of the same module at one Principal.
    let status = query_component_runtime_status(plan.canister, plan.payload.install_id).await?;
    if status.fixture != plan.payload.fixture {
        return Err(InternalError::conflict());
    }
    validate_installed_component_status(
        &status,
        plan.payload.install_id,
        &fixture.binding.target,
        &plan.deployment,
    )?;
    ensure(fixture, || {
        let allocation = ComponentRegistryOps::child_allocation(
            plan.durable.binding.component.component,
            plan.payload.install_id,
        )?
        .ok_or_else(InternalError::conflict)?;
        let RootComponentChildAllocationProgressView::Installed { installation, .. } =
            allocation.progress
        else {
            return Err(InternalError::conflict());
        };
        if !plan.durable.matches_effect(&installation)
            || allocation.release_set.release_build_id != fixture.binding.release_build_id
        {
            return Err(InternalError::conflict());
        }
        Ok(())
    })
    .await
}

async fn ensure(
    fixture: &FixtureInstallation,
    validate_allocation: impl Fn() -> Result<(), InternalError> + Send + Sync,
) -> Result<(), InternalError> {
    let validate = || {
        validate_allocation()?;
        require_current_allocation(fixture)
    };
    validate()?;
    let target = match &fixture.binding.target {
        ManagedCanisterBinding::Component(binding) => binding.canister_id,
        ManagedCanisterBinding::ComponentChild(binding) => binding.canister_id,
    };
    let client = WasmStoreInternalClient::new(fixture.store);
    let observed = client.fixture_grant(target).await?;
    validate()?;
    let Some(request) =
        fixture_grant::issuance_request(&fixture.binding, fixture.revision, observed.as_ref())?
    else {
        return Ok(());
    };
    let expected = FixtureGrant {
        revision: fixture.revision,
        binding: request.binding.clone(),
        enabled: true,
    };
    let granted = client.set_fixture_grant(request).await?;
    validate()?;
    if granted.ok().as_ref() != Some(&expected) {
        return Err(InternalError::conflict());
    }
    Ok(())
}
