//! Module: config::component_deployment_configuration
//!
//! Responsibility: derive one semantic digest over protected Component deployment configuration.
//! Does not own: source parsing, topology semantics, placement planning, persistence, or effects.
//! Boundary: three validated canonical sections become one schema-v1 SHA-256 authority identity.

#[cfg(test)]
mod tests;

use crate::{
    config::{
        ComponentGroupDeploymentTopology, ComponentGroupDeploymentTopologyError,
        ComponentGroupTopology, ComponentGroupTopologyError, ComponentTopology,
        ComponentTopologyError, FleetServiceTopology, FleetServiceTopologyError,
        canonical::CanonicalEncoder, schema::ConfigModel,
    },
    dto::component_deployment::ProtectedComponentDeployment,
    ids::{
        ComponentBinding, ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
        ComponentGroupMemberPath, ComponentGroupSpecId, ComponentSpecId,
    },
};

use candid::CandidType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

const COMPONENT_DEPLOYMENT_CONFIGURATION_DOMAIN: &[u8] =
    b"canic/component-deployment-configuration/v1";
const COMPONENT_DEPLOYMENT_CONFIGURATION_SCHEMA_VERSION: u32 = 1;
const COMPONENT_GROUP_GRAPH_SECTION: &str = "component_group_graph";
const FLATTENED_DEPLOYMENT_SECTION: &str = "flattened_deployments";
const FLEET_SERVICE_TARGET_SECTION: &str = "fleet_service_targets";

/// Maximum canonical bytes for the complete protected deployment configuration.
pub const MAX_COMPONENT_DEPLOYMENT_CONFIGURATION_CANONICAL_BYTES: usize = 8_388_608;

impl ConfigModel {
    /// Compile the complete protected Component deployment authority.
    pub fn compile_component_deployment_configuration(
        &self,
    ) -> Result<ComponentDeploymentConfiguration, ComponentDeploymentConfigurationDigestError> {
        ComponentDeploymentConfiguration::compile(self)
    }

    /// Compile and hash the complete semantic Component deployment configuration.
    pub fn compile_component_deployment_configuration_digest(
        &self,
    ) -> Result<ComponentDeploymentConfigurationDigest, ComponentDeploymentConfigurationDigestError>
    {
        self.compile_component_deployment_configuration()?.digest()
    }

    /// Validate one runtime context against the complete compiled deployment configuration.
    pub(crate) fn validate_protected_component_deployment(
        &self,
        context: &ProtectedComponentDeployment,
        owning_component: &ComponentBinding,
    ) -> Result<(), ProtectedComponentDeploymentError> {
        validate_protected_component_deployment(self, context, owning_component)
    }
}

/// Canonical compiled App authority required to validate provisioning without source TOML.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeploymentConfiguration {
    pub component_topology: ComponentTopology,
    pub component_group_topology: ComponentGroupTopology,
    pub deployment_topology: ComponentGroupDeploymentTopology,
    pub fleet_service_topology: FleetServiceTopology,
}

impl ComponentDeploymentConfiguration {
    /// Compile all four canonical projections from one checked-in App model.
    pub fn compile(
        config: &ConfigModel,
    ) -> Result<Self, ComponentDeploymentConfigurationDigestError> {
        let component_topology = config.compile_component_topology()?;
        let component_group_topology = config.compile_component_group_topology()?;
        let deployment_topology = ComponentGroupDeploymentTopology::compile_from_topologies(
            config,
            &component_group_topology,
            &component_topology,
        )?;
        let fleet_service_topology = FleetServiceTopology::compile_from_topologies(
            config,
            &deployment_topology,
            &component_topology,
        )?;
        let compiled = Self {
            component_topology,
            component_group_topology,
            deployment_topology,
            fleet_service_topology,
        };
        compiled.digest()?;
        Ok(compiled)
    }

    /// Revalidate all decoded projections and derive their protected semantic identity.
    pub fn digest(
        &self,
    ) -> Result<ComponentDeploymentConfigurationDigest, ComponentDeploymentConfigurationDigestError>
    {
        derive_digest(
            &self.component_group_topology,
            &self.deployment_topology,
            &self.fleet_service_topology,
            &self.component_topology,
        )
    }
}

/// Typed rejection while deriving protected Component deployment configuration identity.
#[derive(Debug, ThisError)]
pub enum ComponentDeploymentConfigurationDigestError {
    #[error(transparent)]
    ComponentGroupDeploymentTopology(#[from] ComponentGroupDeploymentTopologyError),

    #[error(transparent)]
    ComponentGroupTopology(#[from] ComponentGroupTopologyError),

    #[error(transparent)]
    ComponentTopology(#[from] ComponentTopologyError),

    #[error(transparent)]
    FleetServiceTopology(#[from] FleetServiceTopologyError),

    #[error("canonical Component deployment configuration bytes {actual} exceed bound {maximum}")]
    CanonicalBytesBoundExceeded { actual: usize, maximum: usize },
}

/// Typed rejection for a runtime deployment context that differs from compiled authority.
#[derive(Debug, ThisError)]
pub enum ProtectedComponentDeploymentError {
    #[error(transparent)]
    Configuration(Box<ComponentDeploymentConfigurationDigestError>),

    #[error("protected deployment binding differs from the managed owning Component")]
    BindingMismatch,

    #[error("protected deployment configuration digest differs from compiled configuration")]
    ConfigurationDigestMismatch,

    #[error("protected deployment references unknown deployment '{deployment}'")]
    UnknownDeployment {
        deployment: ComponentGroupDeploymentId,
    },

    #[error(
        "protected deployment '{deployment}' references Component Group '{actual}', expected '{expected}'"
    )]
    ComponentGroupMismatch {
        deployment: ComponentGroupDeploymentId,
        actual: ComponentGroupSpecId,
        expected: ComponentGroupSpecId,
    },

    #[error("protected deployment '{deployment}' references unknown member path '{member:?}'")]
    UnknownMember {
        deployment: ComponentGroupDeploymentId,
        member: ComponentGroupMemberPath,
    },

    #[error(
        "protected deployment member '{member:?}' binds Component Spec '{actual}', expected '{expected}'"
    )]
    ComponentSpecMismatch {
        member: ComponentGroupMemberPath,
        actual: ComponentSpecId,
        expected: ComponentSpecId,
    },

    #[error("protected deployment member '{member:?}' has the wrong Component Spec hash")]
    ComponentSpecHashMismatch { member: ComponentGroupMemberPath },

    #[error("protected deployment member '{member:?}' has the wrong typed purpose")]
    PurposeMismatch { member: ComponentGroupMemberPath },

    #[error("protected deployment member '{member:?}' has the wrong effective labels")]
    LabelsMismatch { member: ComponentGroupMemberPath },

    #[error("protected deployment member '{member:?}' has the wrong effective limits")]
    LimitsMismatch { member: ComponentGroupMemberPath },
}

fn validate_protected_component_deployment(
    config: &ConfigModel,
    context: &ProtectedComponentDeployment,
    owning_component: &ComponentBinding,
) -> Result<(), ProtectedComponentDeploymentError> {
    if let ProtectedComponentDeployment::UngroupedOrdinary { binding } = context {
        return (binding == owning_component)
            .then_some(())
            .ok_or(ProtectedComponentDeploymentError::BindingMismatch);
    }
    let ProtectedComponentDeployment::GroupMember {
        binding,
        configuration_digest,
        group_placement,
        component_group,
        member_path,
        purpose,
        labels,
        limits,
    } = context
    else {
        unreachable!("ungrouped deployment returned before grouped validation")
    };
    if binding != owning_component {
        return Err(ProtectedComponentDeploymentError::BindingMismatch);
    }

    let expected_digest = config
        .compile_component_deployment_configuration_digest()
        .map_err(|error| ProtectedComponentDeploymentError::Configuration(Box::new(error)))?;
    if configuration_digest != &expected_digest {
        return Err(ProtectedComponentDeploymentError::ConfigurationDigestMismatch);
    }

    let deployment_topology = config
        .compile_component_group_deployment_topology()
        .map_err(ComponentDeploymentConfigurationDigestError::ComponentGroupDeploymentTopology)
        .map_err(|error| ProtectedComponentDeploymentError::Configuration(Box::new(error)))?;
    let deployment = deployment_topology
        .get(&group_placement.deployment)
        .ok_or_else(|| ProtectedComponentDeploymentError::UnknownDeployment {
            deployment: group_placement.deployment.clone(),
        })?;
    if component_group != &deployment.component_group {
        return Err(ProtectedComponentDeploymentError::ComponentGroupMismatch {
            deployment: group_placement.deployment.clone(),
            actual: component_group.clone(),
            expected: deployment.component_group.clone(),
        });
    }
    let member = deployment
        .members
        .binary_search_by(|candidate| candidate.member_path.cmp(member_path))
        .ok()
        .map(|index| &deployment.members[index])
        .ok_or_else(|| ProtectedComponentDeploymentError::UnknownMember {
            deployment: group_placement.deployment.clone(),
            member: member_path.clone(),
        })?;
    if owning_component.component_spec != member.component_spec {
        return Err(ProtectedComponentDeploymentError::ComponentSpecMismatch {
            member: member_path.clone(),
            actual: owning_component.component_spec.clone(),
            expected: member.component_spec.clone(),
        });
    }
    if owning_component.spec_hash != member.component_spec_hash {
        return Err(
            ProtectedComponentDeploymentError::ComponentSpecHashMismatch {
                member: member_path.clone(),
            },
        );
    }
    if purpose != &member.purpose {
        return Err(ProtectedComponentDeploymentError::PurposeMismatch {
            member: member_path.clone(),
        });
    }
    if labels != &member.labels {
        return Err(ProtectedComponentDeploymentError::LabelsMismatch {
            member: member_path.clone(),
        });
    }
    if limits != &member.limits {
        return Err(ProtectedComponentDeploymentError::LimitsMismatch {
            member: member_path.clone(),
        });
    }
    Ok(())
}

fn derive_digest(
    component_group_topology: &ComponentGroupTopology,
    deployment_topology: &ComponentGroupDeploymentTopology,
    fleet_service_topology: &FleetServiceTopology,
    component_topology: &ComponentTopology,
) -> Result<ComponentDeploymentConfigurationDigest, ComponentDeploymentConfigurationDigestError> {
    let bytes = canonical_bytes(
        component_group_topology,
        deployment_topology,
        fleet_service_topology,
        component_topology,
    )?;
    Ok(ComponentDeploymentConfigurationDigest::from_bytes(
        Sha256::digest(bytes).into(),
    ))
}

fn canonical_bytes(
    component_group_topology: &ComponentGroupTopology,
    deployment_topology: &ComponentGroupDeploymentTopology,
    fleet_service_topology: &FleetServiceTopology,
    component_topology: &ComponentTopology,
) -> Result<Vec<u8>, ComponentDeploymentConfigurationDigestError> {
    let group_bytes = component_group_topology.canonical_bytes()?;
    let deployment_bytes =
        deployment_topology.canonical_bytes(component_group_topology, component_topology)?;
    let service_bytes =
        fleet_service_topology.canonical_bytes(deployment_topology, component_topology)?;
    let mut encoder = CanonicalEncoder::new(
        COMPONENT_DEPLOYMENT_CONFIGURATION_DOMAIN,
        COMPONENT_DEPLOYMENT_CONFIGURATION_SCHEMA_VERSION,
    );
    encode_section(&mut encoder, COMPONENT_GROUP_GRAPH_SECTION, &group_bytes);
    encode_section(
        &mut encoder,
        FLATTENED_DEPLOYMENT_SECTION,
        &deployment_bytes,
    );
    encode_section(&mut encoder, FLEET_SERVICE_TARGET_SECTION, &service_bytes);
    let bytes = encoder.finish();
    if bytes.len() > MAX_COMPONENT_DEPLOYMENT_CONFIGURATION_CANONICAL_BYTES {
        return Err(
            ComponentDeploymentConfigurationDigestError::CanonicalBytesBoundExceeded {
                actual: bytes.len(),
                maximum: MAX_COMPONENT_DEPLOYMENT_CONFIGURATION_CANONICAL_BYTES,
            },
        );
    }
    Ok(bytes)
}

fn encode_section(encoder: &mut CanonicalEncoder, name: &str, bytes: &[u8]) {
    encoder.string(name);
    encoder.bytes(bytes);
}

impl From<ComponentDeploymentConfigurationDigestError> for crate::config::ConfigError {
    fn from(error: ComponentDeploymentConfigurationDigestError) -> Self {
        match error {
            ComponentDeploymentConfigurationDigestError::ComponentGroupDeploymentTopology(
                error,
            ) => Self::ComponentGroupDeploymentTopology(error),
            ComponentDeploymentConfigurationDigestError::ComponentGroupTopology(error) => {
                Self::ComponentGroupTopology(error)
            }
            ComponentDeploymentConfigurationDigestError::ComponentTopology(error) => {
                Self::ComponentTopology(error)
            }
            ComponentDeploymentConfigurationDigestError::FleetServiceTopology(error) => {
                Self::FleetServiceTopology(error)
            }
            ComponentDeploymentConfigurationDigestError::CanonicalBytesBoundExceeded {
                actual,
                maximum,
            } => Self::ComponentDeploymentConfigurationCanonicalBytesBoundExceeded {
                actual,
                maximum,
            },
        }
    }
}
