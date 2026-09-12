//! Module: ops::fixture_grant
//!
//! Responsibility: derive source access from the protected release and installation payload.
//! Does not own: Store effects, target installation or application readiness.
//! Boundary: a selected source is bound to one exact Root, role, release and installation.

use crate::{
    ops::{canister_pool::CanisterPoolClaimKey, fixture_content},
    view::{
        component_registry::{
            RootComponentAllocationProgressView, RootComponentChildAllocationProgressView,
        },
        fixture_store::{FixtureGrantSelection, FixtureInstallation},
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::error::InternalError,
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        fixture_provisioning::{
            FixtureAssignment, FixtureGrant, FixtureGrantRequest, FixtureTargetBinding,
        },
        fleet_subnet_root::FleetSubnetRootAuthority,
        root_store::RootStoreBootstrapResponse,
    },
    ids::{FleetRegistryAuthority, ManagedCanisterBinding, ReleaseBuildId, SubnetId},
};

/// Exact Root identity shared by the selected Store and immutable target binding.
#[derive(Eq, PartialEq)]
struct FixtureRootIdentity<'a> {
    authority: &'a FleetRegistryAuthority,
    placement_subnet: SubnetId,
    root: Principal,
    store: Principal,
    release: ReleaseBuildId,
}

/// Recheck the current Root installation against every retained fixture authority field.
pub fn validate_root(
    fixture: &FixtureInstallation,
    authority: &FleetSubnetRootAuthority,
) -> Result<(), InternalError> {
    let component = match &fixture.binding.target {
        ManagedCanisterBinding::Component(binding) => binding,
        ManagedCanisterBinding::ComponentChild(binding) => &binding.component,
    };
    let expected = FixtureRootIdentity {
        authority: &component.authority,
        placement_subnet: component.placement_subnet,
        root: component.fleet_subnet_root,
        store: fixture.store,
        release: fixture.binding.release_build_id,
    };
    let current = FixtureRootIdentity {
        authority: &authority.binding.authority,
        placement_subnet: authority.binding.placement_subnet,
        root: authority.binding.fleet_subnet_root,
        store: authority.wasm_store_authority.wasm_store,
        release: authority.initial_release_set.release_build_id,
    };
    if current != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

/// Project the manifest-selected role source into the exact immutable installation identity.
pub fn installation(
    store: &RootStoreBootstrapResponse,
    payload: &mut CanisterInitPayload,
) -> Result<Option<FixtureInstallation>, InternalError> {
    if payload.fixture.is_some() {
        return Err(InternalError::conflict());
    }
    let (root, target, role) = match &payload.authority {
        CanisterInitAuthority::Component { root, binding } => (
            root,
            ManagedCanisterBinding::Component(binding.clone()),
            &binding.role,
        ),
        CanisterInitAuthority::ComponentChild { root, binding } => (
            root,
            ManagedCanisterBinding::ComponentChild(binding.clone()),
            &binding.role,
        ),
    };
    if root.fleet_subnet_root != store.fleet_subnet_root
        || payload.release_build_id != store.release_set.release_build_id
    {
        return Err(InternalError::conflict());
    }
    let mut sources = store.fixtures.iter().filter(|source| &source.role == role);
    let Some(source) = sources.next() else {
        return Ok(None);
    };
    if sources.next().is_some()
        || fixture_content::content_id(&source.descriptor).ok() != Some(source.content_id)
    {
        return Err(InternalError::conflict());
    }
    let fixture = FixtureInstallation {
        store: store.wasm_store,
        revision: 1,
        binding: FixtureTargetBinding {
            target,
            installation: payload.install_id,
            release_build_id: payload.release_build_id,
            content_id: source.content_id,
        },
    };
    payload.fixture = Some(Box::new(FixtureAssignment {
        store: fixture.store,
        grant: FixtureGrant {
            revision: 1,
            binding: fixture.binding.clone(),
            enabled: true,
        },
        descriptor: source.descriptor.clone(),
    }));
    Ok(Some(fixture))
}

/// Recover the source selection from the existing installation effect.
pub const fn component_selection(
    progress: &RootComponentAllocationProgressView,
) -> Result<FixtureGrantSelection, InternalError> {
    match progress {
        RootComponentAllocationProgressView::Created { .. } => Ok(FixtureGrantSelection::Fresh),
        RootComponentAllocationProgressView::InstallIntent { installation, .. }
        | RootComponentAllocationProgressView::Installed { installation, .. }
        | RootComponentAllocationProgressView::Verified { installation, .. }
        | RootComponentAllocationProgressView::Committed { installation, .. }
        | RootComponentAllocationProgressView::Removed { installation, .. } => Ok(
            FixtureGrantSelection::Retained(installation.fixture_grant_revision),
        ),
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::CreationIntent(_) => Err(InternalError::conflict()),
    }
}

/// Recover the source selection from the existing installation effect.
pub const fn child_selection(
    progress: &RootComponentChildAllocationProgressView,
) -> Result<FixtureGrantSelection, InternalError> {
    match progress {
        RootComponentChildAllocationProgressView::Created { .. } => {
            Ok(FixtureGrantSelection::Fresh)
        }
        RootComponentChildAllocationProgressView::InstallIntent { installation, .. }
        | RootComponentChildAllocationProgressView::Installed { installation, .. }
        | RootComponentChildAllocationProgressView::Verified { installation, .. }
        | RootComponentChildAllocationProgressView::Committed { installation, .. } => Ok(
            FixtureGrantSelection::Retained(installation.fixture_grant_revision),
        ),
        RootComponentChildAllocationProgressView::Reserved
        | RootComponentChildAllocationProgressView::CreationIntent(_) => {
            Err(InternalError::conflict())
        }
    }
}

/// Root, release and physical target that must remain exact across source replacement.
#[derive(Eq, PartialEq)]
struct FixtureReplacementAuthority<'a> {
    authority: &'a FleetRegistryAuthority,
    placement_subnet: SubnetId,
    root: Principal,
    release: ReleaseBuildId,
    target: Principal,
}

impl<'a> From<&'a FixtureTargetBinding> for FixtureReplacementAuthority<'a> {
    fn from(binding: &'a FixtureTargetBinding) -> Self {
        let component = target_component(&binding.target);
        Self {
            authority: &component.authority,
            placement_subnet: component.placement_subnet,
            root: component.fleet_subnet_root,
            release: binding.release_build_id,
            target: target_principal(&binding.target),
        }
    }
}

/// Replace only disabled source access from this Root and release at the same Principal.
pub fn next_revision(
    binding: &FixtureTargetBinding,
    observed: Option<&FixtureGrant>,
) -> Result<u64, InternalError> {
    let Some(previous) = observed else {
        return Ok(1);
    };
    if FixtureReplacementAuthority::from(binding)
        != FixtureReplacementAuthority::from(&previous.binding)
        || binding.installation == previous.binding.installation
        || previous.enabled
        || previous.revision == 0
    {
        return Err(InternalError::conflict());
    }
    previous
        .revision
        .checked_add(1)
        .ok_or_else(InternalError::conflict)
}

const fn target_component(target: &ManagedCanisterBinding) -> &canic_core::ids::ComponentBinding {
    match target {
        ManagedCanisterBinding::Component(binding) => binding,
        ManagedCanisterBinding::ComponentChild(binding) => &binding.component,
    }
}

/// Exact physical target for pool-claim and Store lookup checks.
pub const fn target_principal(target: &ManagedCanisterBinding) -> Principal {
    match target {
        ManagedCanisterBinding::Component(binding) => binding.canister_id,
        ManagedCanisterBinding::ComponentChild(binding) => binding.canister_id,
    }
}

/// Bind the retained revision into the protected payload without another progress owner.
pub fn assign_revision(
    payload: &mut CanisterInitPayload,
    fixture: &mut FixtureInstallation,
    revision: u64,
) -> Result<(), InternalError> {
    let assignment = payload
        .fixture
        .as_mut()
        .ok_or_else(InternalError::conflict)?;
    if revision == 0
        || assignment.store != fixture.store
        || assignment.grant.binding != fixture.binding
    {
        return Err(InternalError::conflict());
    }
    fixture.revision = revision;
    assignment.grant.revision = revision;
    Ok(())
}

/// Issue only the revision selected by the retained installation intent.
pub fn issuance_request(
    binding: &FixtureTargetBinding,
    revision: u64,
    observed: Option<&FixtureGrant>,
) -> Result<Option<FixtureGrantRequest>, InternalError> {
    if revision == 0 {
        return Err(InternalError::conflict());
    }
    if observed.is_some_and(|grant| {
        grant.enabled && grant.revision == revision && &grant.binding == binding
    }) {
        return Ok(None);
    }
    if next_revision(binding, observed)? != revision {
        return Err(InternalError::conflict());
    }
    Ok(Some(FixtureGrantRequest {
        expected_revision: revision - 1,
        binding: binding.clone(),
        enabled: true,
    }))
}

/// Revoke only the grant belonging to the exact allocation being recycled.
/// Observing its revoked revision reconciles a lost reply without another effect.
pub fn revocation_request(
    canister_id: Principal,
    claim: &CanisterPoolClaimKey,
    observed: Option<&FixtureGrant>,
) -> Result<Option<FixtureGrantRequest>, InternalError> {
    let Some(grant) = observed else {
        return Ok(None);
    };
    let (target, component) = match &grant.binding.target {
        ManagedCanisterBinding::Component(binding) => (binding.canister_id, binding.component),
        ManagedCanisterBinding::ComponentChild(binding) => {
            (binding.canister_id, binding.component.component)
        }
    };
    let granted_claim = CanisterPoolClaimKey {
        component,
        operation_id: grant.binding.installation,
    };
    if target != canister_id || &granted_claim != claim || grant.revision == 0 {
        return Err(InternalError::conflict());
    }
    if !grant.enabled {
        return Ok(None);
    }
    grant
        .revision
        .checked_add(1)
        .ok_or_else(InternalError::conflict)?;
    Ok(Some(FixtureGrantRequest {
        expected_revision: grant.revision,
        binding: grant.binding.clone(),
        enabled: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::ids::{
        ComponentBinding, ComponentChildBinding, ComponentInstanceId, ReleaseBuildId,
        ReleaseBuildNonce, SubnetId,
    };

    fn binding() -> FixtureTargetBinding {
        FixtureTargetBinding {
            target: ManagedCanisterBinding::Component(ComponentBinding {
                authority: crate::test_support::root_funding_request_fixture(1)
                    .expected_registry
                    .authority,
                component: ComponentInstanceId::from_generated_bytes([1; 32]),
                component_spec: "fixture".parse().unwrap(),
                spec_hash: [2; 32],
                role: "fixture".into(),
                placement_subnet: SubnetId::from_principal(Principal::from_slice(&[3; 29])),
                fleet_subnet_root: Principal::from_slice(&[4; 29]),
                canister_id: Principal::from_slice(&[5; 29]),
            }),
            installation: [6; 32],
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [7; 32],
            )),
            content_id: [8; 32],
        }
    }

    #[test]
    fn initial_issuance_and_lost_reply_replay_keep_one_exact_grant() {
        let binding = binding();
        let request = issuance_request(&binding, 1, None).unwrap().unwrap();
        assert_eq!(request.expected_revision, 0);
        assert_eq!(request.binding, binding);
        assert!(request.enabled);
        let observed = FixtureGrant {
            revision: 1,
            binding: binding.clone(),
            enabled: true,
        };
        assert_eq!(
            issuance_request(&binding, 1, Some(&observed)).unwrap(),
            None
        );
    }

    #[test]
    fn revoked_and_conflicting_grants_never_authorize_initial_issuance() {
        let expected = binding();
        let original = FixtureGrant {
            revision: 1,
            binding: expected.clone(),
            enabled: true,
        };
        let mut observations = vec![
            FixtureGrant {
                revision: 2,
                enabled: false,
                ..original.clone()
            },
            FixtureGrant {
                revision: 0,
                ..original.clone()
            },
            FixtureGrant {
                revision: 3,
                ..original.clone()
            },
        ];
        let mut installation = original.clone();
        installation.binding.installation = [9; 32];
        observations.push(installation);
        let mut content = original.clone();
        content.binding.content_id = [9; 32];
        observations.push(content);
        let mut release = original.clone();
        release.binding.release_build_id =
            ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([9; 32]));
        observations.push(release);
        let ManagedCanisterBinding::Component(component) = expected.target.clone() else {
            unreachable!()
        };
        for child in [false, true] {
            let mut changed = original.clone();
            let mut component = component.clone();
            component.canister_id = Principal::from_slice(&[9; 29]);
            changed.binding.target = if child {
                ManagedCanisterBinding::ComponentChild(ComponentChildBinding {
                    parent_canister_id: component.canister_id,
                    role: "child".into(),
                    canister_id: Principal::from_slice(&[10; 29]),
                    component,
                })
            } else {
                ManagedCanisterBinding::Component(component)
            };
            observations.push(changed);
        }
        for observed in observations {
            assert_eq!(
                issuance_request(&expected, 1, Some(&observed))
                    .unwrap_err()
                    .code(),
                InternalError::conflict().code()
            );
        }
    }
    #[test]
    fn recycling_revokes_only_the_exact_claim_and_reconciles_a_lost_reply() {
        let binding = binding();
        let ManagedCanisterBinding::Component(target) = &binding.target else {
            unreachable!()
        };
        let canister = target.canister_id;
        let mut claim = CanisterPoolClaimKey {
            component: target.component,
            operation_id: binding.installation,
        };
        let mut grant = FixtureGrant {
            revision: 1,
            binding,
            enabled: true,
        };
        let request = revocation_request(canister, &claim, Some(&grant))
            .unwrap()
            .unwrap();
        assert_eq!(request.expected_revision, 1);
        assert_eq!(request.binding, grant.binding);
        assert!(!request.enabled);
        grant.revision = 2;
        grant.enabled = false;
        assert_eq!(
            revocation_request(canister, &claim, Some(&grant)).unwrap(),
            None
        );
        claim.operation_id = [9; 32];
        assert_eq!(
            revocation_request(canister, &claim, Some(&grant))
                .unwrap_err()
                .public_code(),
            InternalError::conflict().public_code()
        );
    }

    #[test]
    fn replacement_selection_and_replay_cannot_follow_a_newer_store_revision() {
        let mut next = binding();
        let mut previous = FixtureGrant {
            revision: 2,
            binding: next.clone(),
            enabled: false,
        };
        next.installation = [9; 32];
        assert_eq!(next_revision(&next, Some(&previous)).unwrap(), 3);
        let request = issuance_request(&next, 3, Some(&previous))
            .unwrap()
            .unwrap();
        assert_eq!(request.expected_revision, 2);
        assert_eq!(request.binding, next);
        previous = FixtureGrant {
            revision: 3,
            binding: next.clone(),
            enabled: true,
        };
        assert_eq!(issuance_request(&next, 3, Some(&previous)).unwrap(), None);
        assert_eq!(
            next_revision(&next, Some(&previous))
                .unwrap_err()
                .public_code(),
            InternalError::conflict().public_code()
        );
        previous.enabled = false;
        previous.revision = 4;
        assert_eq!(
            issuance_request(&next, 3, Some(&previous))
                .unwrap_err()
                .public_code(),
            InternalError::conflict().public_code()
        );
        previous.binding.installation = [10; 32];
        assert_eq!(
            issuance_request(&next, 3, Some(&previous))
                .unwrap_err()
                .public_code(),
            InternalError::conflict().public_code()
        );
        previous.revision = u64::MAX;
        assert_eq!(
            next_revision(&next, Some(&previous))
                .unwrap_err()
                .public_code(),
            InternalError::conflict().public_code()
        );
    }

    #[test]
    fn replacement_rejects_live_or_foreign_source_authority() {
        let mut next = binding();
        let previous = FixtureGrant {
            revision: 2,
            binding: next.clone(),
            enabled: false,
        };
        next.installation = [9; 32];
        let changes: &[fn(&mut FixtureGrant)] = &[
            |g| g.enabled = true,
            |g| g.revision = 0,
            |g| g.binding.installation = [9; 32],
            |g| {
                g.binding.release_build_id =
                    ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([9; 32]));
            },
            |g| {
                let ManagedCanisterBinding::Component(target) = &mut g.binding.target else {
                    unreachable!()
                };
                target.fleet_subnet_root = Principal::from_slice(&[9; 29]);
            },
            |g| {
                let ManagedCanisterBinding::Component(target) = &mut g.binding.target else {
                    unreachable!()
                };
                target.canister_id = Principal::from_slice(&[9; 29]);
            },
        ];
        for change in changes {
            let mut invalid = previous.clone();
            change(&mut invalid);
            assert_eq!(
                next_revision(&next, Some(&invalid))
                    .unwrap_err()
                    .public_code(),
                InternalError::conflict().public_code()
            );
        }
    }
}
