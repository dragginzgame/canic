//! Lossless runtime reporting projections preserve all retained authority and original deadlines.
use crate::dto::runtime_reporting as native;

impl From<crate::dto::component_registry::ComponentRuntimeStatusResponse>
    for native::ComponentRuntimeStatusResponse
{
    fn from(value: crate::dto::component_registry::ComponentRuntimeStatusResponse) -> Self {
        Self {
            fixture: value.fixture.map(|item| Box::new((*item).into())),
            operation_id: value.operation_id,
            binding: value.binding.into(),
            deployment: Box::new((*value.deployment).into()),
            phase: value.phase,
            authority: value.authority.map(Into::into),
            authority_hash: value.authority_hash,
            direct_children_hash: value.direct_children_hash,
            activation: value.activation,
        }
    }
}

impl From<native::ComponentRuntimeStatusResponse>
    for crate::dto::component_registry::ComponentRuntimeStatusResponse
{
    fn from(value: native::ComponentRuntimeStatusResponse) -> Self {
        Self {
            fixture: value.fixture.map(|item| Box::new((*item).into())),
            operation_id: value.operation_id,
            binding: value.binding.into(),
            deployment: Box::new((*value.deployment).into()),
            phase: value.phase,
            authority: value.authority.map(Into::into),
            authority_hash: value.authority_hash,
            direct_children_hash: value.direct_children_hash,
            activation: value.activation,
        }
    }
}

impl From<crate::dto::role::ComponentRuntimeOperationStatus>
    for native::ComponentRuntimeOperationStatus
{
    fn from(value: crate::dto::role::ComponentRuntimeOperationStatus) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_activation: value.fleet_activation.into(),
            runtime: value.runtime.into(),
        }
    }
}

impl From<native::ComponentRuntimeOperationStatus>
    for crate::dto::role::ComponentRuntimeOperationStatus
{
    fn from(value: native::ComponentRuntimeOperationStatus) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_activation: value.fleet_activation.into(),
            runtime: value.runtime.into(),
        }
    }
}

impl From<crate::dto::fleet_activation::FleetCascadeManifestEntry>
    for native::FleetCascadeManifestEntry
{
    fn from(value: crate::dto::fleet_activation::FleetCascadeManifestEntry) -> Self {
        Self {
            principal: value.principal.into(),
            state_snapshot_hash: value.state_snapshot_hash,
            topology_snapshot_hash: value.topology_snapshot_hash,
        }
    }
}

impl From<native::FleetCascadeManifestEntry>
    for crate::dto::fleet_activation::FleetCascadeManifestEntry
{
    fn from(value: native::FleetCascadeManifestEntry) -> Self {
        Self {
            principal: value.principal.into(),
            state_snapshot_hash: value.state_snapshot_hash,
            topology_snapshot_hash: value.topology_snapshot_hash,
        }
    }
}

impl From<crate::dto::fleet_activation::FleetCredentialManifestEntry>
    for native::FleetCredentialManifestEntry
{
    fn from(value: crate::dto::fleet_activation::FleetCredentialManifestEntry) -> Self {
        Self {
            root_issuer: value.root_issuer.into(),
            subject_canister: value.subject_canister.into(),
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            key_identity_hash: value.key_identity_hash,
            cert_hash: value.cert_hash,
            proof_hash: value.proof_hash,
            bundle_hash: value.bundle_hash,
        }
    }
}

impl From<native::FleetCredentialManifestEntry>
    for crate::dto::fleet_activation::FleetCredentialManifestEntry
{
    fn from(value: native::FleetCredentialManifestEntry) -> Self {
        Self {
            root_issuer: value.root_issuer.into(),
            subject_canister: value.subject_canister.into(),
            not_before_ns: value.not_before_ns,
            expires_at_ns: value.expires_at_ns,
            key_identity_hash: value.key_identity_hash,
            cert_hash: value.cert_hash,
            proof_hash: value.proof_hash,
            bundle_hash: value.bundle_hash,
        }
    }
}

impl From<crate::dto::fleet_activation::FleetCredentialManifest>
    for native::FleetCredentialManifest
{
    fn from(value: crate::dto::fleet_activation::FleetCredentialManifest) -> Self {
        Self {
            fleet: value.fleet,
            activation_id: value.activation_id,
            generation: value.generation,
            root_policy_set_hash: value.root_policy_set_hash,
            renewal_template_set_hash: value.renewal_template_set_hash,
            entries: value.entries.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::FleetCredentialManifest>
    for crate::dto::fleet_activation::FleetCredentialManifest
{
    fn from(value: native::FleetCredentialManifest) -> Self {
        Self {
            fleet: value.fleet,
            activation_id: value.activation_id,
            generation: value.generation,
            root_policy_set_hash: value.root_policy_set_hash,
            renewal_template_set_hash: value.renewal_template_set_hash,
            entries: value.entries.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::dto::fleet_activation::FleetActivationStatusResponse>
    for native::FleetActivationStatusResponse
{
    fn from(value: crate::dto::fleet_activation::FleetActivationStatusResponse) -> Self {
        Self {
            phase: value.phase,
            identity: value.identity,
            cascade: value.cascade,
            cascade_manifest: value
                .cascade_manifest
                .map(|item| item.into_iter().map(Into::into).collect()),
            credential: value.credential,
            credential_manifest: value.credential_manifest.map(Into::into),
            activated_at_ns: value.activated_at_ns,
        }
    }
}

impl From<native::FleetActivationStatusResponse>
    for crate::dto::fleet_activation::FleetActivationStatusResponse
{
    fn from(value: native::FleetActivationStatusResponse) -> Self {
        Self {
            phase: value.phase,
            identity: value.identity,
            cascade: value.cascade,
            cascade_manifest: value
                .cascade_manifest
                .map(|item| item.into_iter().map(Into::into).collect()),
            credential: value.credential,
            credential_manifest: value.credential_manifest.map(Into::into),
            activated_at_ns: value.activated_at_ns,
        }
    }
}

impl From<crate::dto::fixture_provisioning::FixtureTargetBinding> for native::FixtureTargetBinding {
    fn from(value: crate::dto::fixture_provisioning::FixtureTargetBinding) -> Self {
        Self {
            target: value.target.into(),
            installation: value.installation,
            release_build_id: value.release_build_id,
            content_id: value.content_id,
        }
    }
}

impl From<native::FixtureTargetBinding> for crate::dto::fixture_provisioning::FixtureTargetBinding {
    fn from(value: native::FixtureTargetBinding) -> Self {
        Self {
            target: value.target.into(),
            installation: value.installation,
            release_build_id: value.release_build_id,
            content_id: value.content_id,
        }
    }
}

impl From<crate::dto::fixture_provisioning::FixtureGrant> for native::FixtureGrant {
    fn from(value: crate::dto::fixture_provisioning::FixtureGrant) -> Self {
        Self {
            revision: value.revision,
            binding: value.binding.into(),
            enabled: value.enabled,
        }
    }
}

impl From<native::FixtureGrant> for crate::dto::fixture_provisioning::FixtureGrant {
    fn from(value: native::FixtureGrant) -> Self {
        Self {
            revision: value.revision,
            binding: value.binding.into(),
            enabled: value.enabled,
        }
    }
}

impl From<crate::dto::fixture_provisioning::FixtureAssignment> for native::FixtureAssignment {
    fn from(value: crate::dto::fixture_provisioning::FixtureAssignment) -> Self {
        Self {
            store: value.store.into(),
            grant: value.grant.into(),
            descriptor: value.descriptor,
        }
    }
}

impl From<native::FixtureAssignment> for crate::dto::fixture_provisioning::FixtureAssignment {
    fn from(value: native::FixtureAssignment) -> Self {
        Self {
            store: value.store.into(),
            grant: value.grant.into(),
            descriptor: value.descriptor,
        }
    }
}

impl From<crate::dto::component_deployment::ProtectedComponentDeployment>
    for native::ProtectedComponentDeployment
{
    fn from(value: crate::dto::component_deployment::ProtectedComponentDeployment) -> Self {
        match value {
            crate::dto::component_deployment::ProtectedComponentDeployment::UngroupedOrdinary {
                binding,
            } => Self::UngroupedOrdinary {
                binding: binding.into(),
            },
            crate::dto::component_deployment::ProtectedComponentDeployment::GroupMember {
                binding,
                configuration_digest,
                group_placement,
                component_group,
                member_path,
                purpose,
                labels,
                limits,
            } => Self::GroupMember {
                binding: binding.into(),
                configuration_digest,
                group_placement,
                component_group,
                member_path,
                purpose,
                labels,
                limits,
            },
        }
    }
}

impl From<native::ProtectedComponentDeployment>
    for crate::dto::component_deployment::ProtectedComponentDeployment
{
    fn from(value: native::ProtectedComponentDeployment) -> Self {
        match value {
            native::ProtectedComponentDeployment::UngroupedOrdinary { binding } => {
                Self::UngroupedOrdinary {
                    binding: binding.into(),
                }
            }
            native::ProtectedComponentDeployment::GroupMember {
                binding,
                configuration_digest,
                group_placement,
                component_group,
                member_path,
                purpose,
                labels,
                limits,
            } => Self::GroupMember {
                binding: binding.into(),
                configuration_digest,
                group_placement,
                component_group,
                member_path,
                purpose,
                labels,
                limits,
            },
        }
    }
}
