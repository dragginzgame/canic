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
    ids::ComponentDeploymentConfigurationDigest,
};

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
    /// Compile and hash the complete semantic Component deployment configuration.
    pub fn compile_component_deployment_configuration_digest(
        &self,
    ) -> Result<ComponentDeploymentConfigurationDigest, ComponentDeploymentConfigurationDigestError>
    {
        let component_topology = self.compile_component_topology()?;
        let component_group_topology = self.compile_component_group_topology()?;
        let deployment_topology = ComponentGroupDeploymentTopology::compile_from_topologies(
            self,
            &component_group_topology,
            &component_topology,
        )?;
        let fleet_service_topology = FleetServiceTopology::compile_from_topologies(
            self,
            &deployment_topology,
            &component_topology,
        )?;
        derive_digest(
            &component_group_topology,
            &deployment_topology,
            &fleet_service_topology,
            &component_topology,
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
