//! Module: model::fleet_activation
//!
//! Responsibility: validate the immutable identity established by fresh Fleet activation.
//! Does not own: Candid decoding, stable-record conversion, storage access, or lifecycle traps.
//! Boundary: workflows supply the embedded build identity before ops persists `Prepared`.

pub mod endpoint_mode;

use crate::{
    config::ComponentTopology,
    ids::{AppId, FleetBinding, FleetSubnetRootBinding, FleetSubnetRootReleaseSet, ReleaseBuildId},
};
use candid::Principal;
use thiserror::Error as ThisError;

///
/// PreparedFleetActivation
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedFleetActivation {
    pub identity: FleetActivationIdentity,
    pub root_authority: Option<PreparedFleetSubnetRootAuthority>,
}

///
/// PreparedFleetSubnetRootAuthority
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedFleetSubnetRootAuthority {
    pub binding: FleetSubnetRootBinding,
    pub initial_release_set: FleetSubnetRootReleaseSet,
    pub expected_module_hash: [u8; 32],
}

///
/// FleetActivationIdentity
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationIdentity {
    pub fleet: FleetBinding,
    pub operation_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

///
/// RootInstallIdentity
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootInstallIdentity {
    pub binding: FleetSubnetRootBinding,
    pub initial_release_set: FleetSubnetRootReleaseSet,
    pub install_id: [u8; 32],
    pub expected_module_hash: [u8; 32],
}

///
/// NonrootInstallIdentity
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonrootInstallIdentity {
    pub fleet: FleetBinding,
    pub install_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

///
/// PrepareFleetActivationError
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum PrepareFleetActivationError {
    #[error(
        "install release-build identity {supplied} does not match embedded Wasm identity {embedded}"
    )]
    ReleaseBuildMismatch {
        supplied: ReleaseBuildId,
        embedded: ReleaseBuildId,
    },

    #[error("Fleet Subnet Root authority uses epoch {observed}, expected fresh epoch one")]
    AuthorityEpoch { observed: u64 },

    #[error(
        "Fleet Subnet Root authority App '{authority}' does not match configured App '{configured}'"
    )]
    AppMismatch { configured: AppId, authority: AppId },

    #[error("Fleet Subnet Root binding principal {bound} does not match this Canister {observed}")]
    RootPrincipalMismatch {
        bound: Principal,
        observed: Principal,
    },

    #[error(transparent)]
    Topology(#[from] crate::config::ComponentTopologyError),
}

/// Validate and normalize fresh root input into the sole internal activation identity.
pub fn prepare_root_install(
    input: RootInstallIdentity,
    embedded_release_build_id: ReleaseBuildId,
    configured_app: &AppId,
    component_topology: &ComponentTopology,
    root_canister: Principal,
) -> Result<PreparedFleetActivation, PrepareFleetActivationError> {
    require_release_build_match(
        input.initial_release_set.release_build_id,
        embedded_release_build_id,
    )?;
    if input.binding.authority.epoch != 1 {
        return Err(PrepareFleetActivationError::AuthorityEpoch {
            observed: input.binding.authority.epoch,
        });
    }
    if &input.binding.authority.binding.fleet.app != configured_app {
        return Err(PrepareFleetActivationError::AppMismatch {
            configured: configured_app.clone(),
            authority: input.binding.authority.binding.fleet.app,
        });
    }
    if input.binding.fleet_subnet_root != root_canister {
        return Err(PrepareFleetActivationError::RootPrincipalMismatch {
            bound: input.binding.fleet_subnet_root,
            observed: root_canister,
        });
    }
    component_topology.validate_root_binding(&input.binding)?;

    Ok(PreparedFleetActivation {
        identity: FleetActivationIdentity {
            fleet: input.binding.authority.binding.fleet.clone(),
            operation_id: input.install_id,
            release_build_id: input.initial_release_set.release_build_id,
        },
        root_authority: Some(PreparedFleetSubnetRootAuthority {
            binding: input.binding,
            initial_release_set: input.initial_release_set,
            expected_module_hash: input.expected_module_hash,
        }),
    })
}

/// Validate and normalize fresh non-root input into the sole internal activation identity.
pub fn prepare_nonroot_install(
    input: NonrootInstallIdentity,
    embedded_release_build_id: ReleaseBuildId,
) -> Result<PreparedFleetActivation, PrepareFleetActivationError> {
    require_release_build_match(input.release_build_id, embedded_release_build_id)?;

    Ok(PreparedFleetActivation {
        identity: FleetActivationIdentity {
            fleet: input.fleet,
            operation_id: input.install_id,
            release_build_id: input.release_build_id,
        },
        root_authority: None,
    })
}

fn require_release_build_match(
    supplied: ReleaseBuildId,
    embedded: ReleaseBuildId,
) -> Result<(), PrepareFleetActivationError> {
    if supplied != embedded {
        return Err(PrepareFleetActivationError::ReleaseBuildMismatch { supplied, embedded });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Cycles,
        config::{ComponentLimits, ComponentSpec},
        ids::{
            CanisterRole, CanonicalNetworkId, ComponentSpecAdmission, ComponentSpecId,
            ComponentTopologyDigest, CyclesFundingBudget, FleetCoordinatorBinding, FleetId,
            FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits, ReleaseBuildNonce,
            ReleaseSetDigest, SubnetId,
        },
    };

    fn release_build(byte: u8) -> ReleaseBuildId {
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([byte; 32]))
    }

    fn input(release_build_id: ReleaseBuildId) -> RootInstallIdentity {
        let component_spec: ComponentSpecId = "projects".parse().expect("Component Spec");
        RootInstallIdentity {
            binding: FleetSubnetRootBinding {
                authority: FleetRegistryAuthority {
                    binding: FleetCoordinatorBinding {
                        fleet: FleetBinding {
                            fleet: FleetKey {
                                canonical_network_id: CanonicalNetworkId::public_ic(),
                                fleet_id: FleetId::from_generated_bytes([2; 32]),
                            },
                            app: AppId::from("toko"),
                        },
                        coordinator_subnet: SubnetId::from_principal(Principal::from_slice(
                            &[4; 29],
                        )),
                        coordinator: Principal::from_slice(&[5; 29]),
                    },
                    epoch: 1,
                },
                placement_subnet: SubnetId::from_principal(Principal::from_slice(&[6; 29])),
                fleet_subnet_root: Principal::from_slice(&[7; 29]),
                component_admissions: vec![ComponentSpecAdmission {
                    component_spec,
                    spec_hash: [8; 32],
                    maximum_root_instances: 2,
                }],
                component_topology_digest: ComponentTopologyDigest::from_bytes([9; 32]),
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 10,
                    maximum_managed_canisters: 1_000,
                    maximum_registry_bytes: 4_194_304,
                    maximum_wasm_store_bytes: 40_000_000,
                    cycles_funding: funding(),
                },
            },
            initial_release_set: FleetSubnetRootReleaseSet {
                release_build_id,
                manifest_digest: ReleaseSetDigest::from_bytes([10; 32]),
            },
            install_id: [3; 32],
            expected_module_hash: [11; 32],
        }
    }

    fn nonroot_input(release_build_id: ReleaseBuildId) -> NonrootInstallIdentity {
        let root = input(release_build_id);
        NonrootInstallIdentity {
            fleet: root.binding.authority.binding.fleet,
            install_id: root.install_id,
            release_build_id: root.initial_release_set.release_build_id,
        }
    }

    fn funding() -> CyclesFundingBudget {
        CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(1_000_000_000_000),
        }
    }

    fn topology() -> ComponentTopology {
        ComponentTopology {
            component_specs: vec![ComponentSpec {
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [8; 32],
                component_role: CanisterRole::from("project_hub"),
                maximum_fleet_instances: 10,
                limits: ComponentLimits {
                    maximum_descendants: 100,
                    maximum_registry_bytes: 1_048_576,
                    cycles_funding: funding(),
                },
                children: Vec::new(),
                spawn_grants: Vec::new(),
            }],
            provisioning_grants: Vec::new(),
        }
    }

    fn prepare_root(
        input: RootInstallIdentity,
        embedded: ReleaseBuildId,
    ) -> Result<PreparedFleetActivation, PrepareFleetActivationError> {
        let root_canister = input.binding.fleet_subnet_root;
        let mut input = input;
        input.binding.component_topology_digest = topology()
            .project_for_admissions(&input.binding.component_admissions)
            .expect("projection")
            .digest()
            .expect("digest");
        prepare_root_install(
            input,
            embedded,
            &AppId::from("toko"),
            &topology(),
            root_canister,
        )
    }

    #[test]
    fn root_install_normalizes_install_identity_only_after_build_match() {
        let release_build_id = release_build(5);
        let prepared = prepare_root(input(release_build_id), release_build_id).expect("prepare");

        assert_eq!(prepared.identity.operation_id, [3; 32]);
        assert_eq!(prepared.identity.release_build_id, release_build_id);
    }

    #[test]
    fn root_install_rejects_release_build_mismatch() {
        let supplied = release_build(6);
        let embedded = release_build(7);

        assert_eq!(
            prepare_root(input(supplied), embedded),
            Err(PrepareFleetActivationError::ReleaseBuildMismatch { supplied, embedded })
        );
    }

    #[test]
    fn root_install_rejects_wrong_epoch_app_and_root_principal() {
        let release_build_id = release_build(11);
        let mut wrong_epoch = input(release_build_id);
        wrong_epoch.binding.authority.epoch = 2;
        assert_eq!(
            prepare_root(wrong_epoch, release_build_id),
            Err(PrepareFleetActivationError::AuthorityEpoch { observed: 2 })
        );

        let mut wrong_app = input(release_build_id);
        wrong_app.binding.authority.binding.fleet.app = AppId::from("other");
        assert_eq!(
            prepare_root(wrong_app, release_build_id),
            Err(PrepareFleetActivationError::AppMismatch {
                configured: AppId::from("toko"),
                authority: AppId::from("other"),
            })
        );

        let mut wrong_root = input(release_build_id);
        wrong_root.binding.component_topology_digest = topology()
            .project_for_admissions(&wrong_root.binding.component_admissions)
            .expect("projection")
            .digest()
            .expect("digest");
        let bound = wrong_root.binding.fleet_subnet_root;
        let observed = Principal::from_slice(&[99; 29]);
        assert_eq!(
            prepare_root_install(
                wrong_root,
                release_build_id,
                &AppId::from("toko"),
                &topology(),
                observed,
            ),
            Err(PrepareFleetActivationError::RootPrincipalMismatch { bound, observed })
        );
    }

    #[test]
    fn nonroot_install_normalizes_the_same_exact_identity() {
        let release_build_id = release_build(8);
        let prepared = prepare_nonroot_install(nonroot_input(release_build_id), release_build_id)
            .expect("prepare");

        assert_eq!(prepared.identity.operation_id, [3; 32]);
        assert_eq!(prepared.identity.release_build_id, release_build_id);
    }

    #[test]
    fn nonroot_install_rejects_release_build_mismatch() {
        let supplied = release_build(9);
        let embedded = release_build(10);

        assert_eq!(
            prepare_nonroot_install(nonroot_input(supplied), embedded),
            Err(PrepareFleetActivationError::ReleaseBuildMismatch { supplied, embedded })
        );
    }
}
