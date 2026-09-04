//! Module: model::fleet_activation
//!
//! Responsibility: validate the immutable identity established by fresh Fleet activation.
//! Does not own: Candid decoding, stable-record conversion, storage access, or lifecycle traps.
//! Boundary: workflows supply the embedded build identity before ops persists `Prepared`.

pub mod endpoint_mode;

use crate::{
    config::ComponentTopology,
    ids::{
        AppId, CanonicalNetworkId, FleetBinding, FleetSubnetRootBinding, FleetSubnetRootReleaseSet,
        FleetSubnetWasmStoreActivationAuthority, FleetSubnetWasmStoreAuthority, ReleaseBuildId,
    },
    model::fleet_funding_policy::{
        FleetFundingPolicyValidationError, validate_fleet_subnet_root_funding_authority,
    },
};
use candid::Principal;
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

///
/// PreparedFleetActivation
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedFleetActivation {
    pub identity: FleetActivationIdentity,
    pub root_authority: Option<PreparedFleetSubnetRootAuthority>,
    pub wasm_store_authority: Option<FleetSubnetWasmStoreAuthority>,
}

///
/// PreparedFleetSubnetRootAuthority
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedFleetSubnetRootAuthority {
    pub binding: FleetSubnetRootBinding,
    pub initial_release_set: FleetSubnetRootReleaseSet,
    pub expected_module_hash: [u8; 32],
    pub wasm_store_activation: FleetSubnetWasmStoreActivationAuthority,
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
    pub wasm_store_authority: FleetSubnetWasmStoreAuthority,
    pub wasm_store_activation: FleetSubnetWasmStoreActivationAuthority,
}

///
/// WasmStoreInstallIdentity
///
/// Host-materialized identity supplied to one independently installed sibling Store.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmStoreInstallIdentity {
    pub authority: FleetSubnetWasmStoreAuthority,
    pub install_id: [u8; 32],
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

    #[error("sibling Wasm Store authority does not match its Fleet Subnet Root")]
    WasmStoreAuthorityMismatch,

    #[error("sibling Wasm Store authority contains an anonymous or conflicting principal")]
    WasmStorePrincipalInvalid,

    #[error("sibling Wasm Store binding principal {bound} does not match this Canister {observed}")]
    WasmStorePrincipalMismatch {
        bound: Principal,
        observed: Principal,
    },

    #[error("sibling Wasm Store authority has a zero module hash")]
    WasmStoreModuleHashZero,

    #[error("sibling Wasm Store installation identity is missing or conflicts with Root")]
    WasmStoreInstallIdInvalid,

    #[error("Fleet Subnet Root funding authority is invalid: {0}")]
    FundingPolicy(#[from] FleetFundingPolicyValidationError),

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
    validate_fleet_subnet_root_funding_authority(
        &input.binding.funding,
        input
            .binding
            .authority
            .binding
            .fleet
            .fleet
            .canonical_network_id
            == CanonicalNetworkId::ic_mainnet(),
    )?;
    component_topology.validate_root_binding(&input.binding)?;
    validate_wasm_store_authority(&input.wasm_store_authority)?;
    let root_authority = (
        input.binding.authority.clone(),
        input.binding.placement_subnet,
        input.binding.fleet_subnet_root,
        input.initial_release_set.release_build_id,
    );
    let store_authority = (
        input.wasm_store_authority.authority.clone(),
        input.wasm_store_authority.placement_subnet,
        input.wasm_store_authority.fleet_subnet_root,
        input.wasm_store_authority.release_build_id,
    );
    if store_authority != root_authority {
        return Err(PrepareFleetActivationError::WasmStoreAuthorityMismatch);
    }
    validate_wasm_store_activation_authority(&input)?;

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
            wasm_store_activation: input.wasm_store_activation,
        }),
        wasm_store_authority: Some(input.wasm_store_authority),
    })
}

/// Validate and normalize one independently installed sibling Store identity.
pub fn prepare_wasm_store_install(
    input: WasmStoreInstallIdentity,
    embedded_release_build_id: ReleaseBuildId,
    wasm_store_canister: Principal,
) -> Result<PreparedFleetActivation, PrepareFleetActivationError> {
    require_release_build_match(input.authority.release_build_id, embedded_release_build_id)?;
    if input.install_id == [0; 32] {
        return Err(PrepareFleetActivationError::WasmStoreInstallIdInvalid);
    }
    validate_wasm_store_authority(&input.authority)?;
    if input.authority.wasm_store != wasm_store_canister {
        return Err(PrepareFleetActivationError::WasmStorePrincipalMismatch {
            bound: input.authority.wasm_store,
            observed: wasm_store_canister,
        });
    }
    Ok(PreparedFleetActivation {
        identity: FleetActivationIdentity {
            fleet: input.authority.authority.binding.fleet.clone(),
            operation_id: input.install_id,
            release_build_id: input.authority.release_build_id,
        },
        root_authority: None,
        wasm_store_authority: Some(input.authority),
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
        wasm_store_authority: None,
    })
}

fn validate_wasm_store_authority(
    authority: &FleetSubnetWasmStoreAuthority,
) -> Result<(), PrepareFleetActivationError> {
    if authority.authority.epoch != 1 {
        return Err(PrepareFleetActivationError::WasmStorePrincipalInvalid);
    }
    let required_principals = [
        *authority.placement_subnet.as_principal(),
        authority.fleet_subnet_root,
        authority.wasm_store,
        authority.installation_controller,
    ];
    if required_principals.contains(&Principal::anonymous()) {
        return Err(PrepareFleetActivationError::WasmStorePrincipalInvalid);
    }
    let controlled_canisters = [
        authority.authority.binding.coordinator,
        authority.fleet_subnet_root,
        authority.wasm_store,
        authority.installation_controller,
    ];
    if controlled_canisters
        .into_iter()
        .collect::<BTreeSet<_>>()
        .len()
        != controlled_canisters.len()
    {
        return Err(PrepareFleetActivationError::WasmStorePrincipalInvalid);
    }
    if authority.wasm_module_hash == [0; 32] {
        return Err(PrepareFleetActivationError::WasmStoreModuleHashZero);
    }
    Ok(())
}

fn validate_wasm_store_activation_authority(
    input: &RootInstallIdentity,
) -> Result<(), PrepareFleetActivationError> {
    let child = &input.wasm_store_activation;
    let expected_controllers = expected_wasm_store_controllers(
        input.binding.fleet_subnet_root,
        input.wasm_store_authority.installation_controller,
    );
    let exact_binding = child.fleet == input.binding.authority.binding.fleet
        && child.fleet_subnet_root == input.binding.fleet_subnet_root
        && child.wasm_store == input.wasm_store_authority.wasm_store
        && child.release_build_id == input.initial_release_set.release_build_id
        && child.component_topology_digest == input.binding.component_topology_digest
        && child.manifest_digest == input.initial_release_set.manifest_digest;
    let exact_identity = child.operation_id != [0; 32]
        && child.operation_id != input.install_id
        && child.wasm_store != Principal::anonymous()
        && child.fleet_subnet_root != Principal::anonymous();
    let exact_controllers = child.controllers == expected_controllers;
    if !exact_binding || !exact_identity || !exact_controllers {
        return Err(PrepareFleetActivationError::WasmStoreInstallIdInvalid);
    }
    Ok(())
}

pub fn expected_wasm_store_controllers(
    fleet_subnet_root: Principal,
    installation_controller: Principal,
) -> Vec<Principal> {
    let mut controllers = vec![fleet_subnet_root, installation_controller];
    controllers.sort();
    controllers.dedup();
    controllers
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
        let authority = FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([2; 32]),
                    },
                    app: AppId::from("toko"),
                },
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[4; 29])),
                coordinator: Principal::from_slice(&[5; 29]),
            },
            epoch: 1,
        };
        let placement_subnet = SubnetId::from_principal(Principal::from_slice(&[6; 29]));
        let fleet_subnet_root = Principal::from_slice(&[7; 29]);
        let binding = FleetSubnetRootBinding {
            authority: authority.clone(),
            placement_subnet,
            fleet_subnet_root,
            component_admissions: vec![ComponentSpecAdmission {
                component_spec,
                spec_hash: [8; 32],
                maximum_root_instances: 2,
            }],
            component_topology_digest: ComponentTopologyDigest::from_bytes([9; 32]),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 10,
                maximum_registry_bytes: 4_194_304,
                maximum_wasm_store_bytes: 40_000_000,
                maximum_group_placements: 16,
                canister_pool: crate::ids::FleetSubnetCanisterPoolConfig {
                    minimum_size: 1,
                    maximum_size: 10,
                    canister_cycles: Cycles::new(5_000_000_000_000),
                    creation_execution_margin: Cycles::new(1_000_000_000_000),
                },
                cycles_funding: funding(),
            },
            funding: crate::test::support::fleet_subnet_root_funding_authority(),
        };
        let initial_release_set = FleetSubnetRootReleaseSet {
            release_build_id,
            manifest_digest: ReleaseSetDigest::from_bytes([10; 32]),
        };
        let wasm_store = Principal::from_slice(&[12; 29]);
        let installation_controller = Principal::from_slice(&[13; 29]);
        let wasm_store_activation = FleetSubnetWasmStoreActivationAuthority {
            fleet: binding.authority.binding.fleet.clone(),
            operation_id: [15; 32],
            fleet_subnet_root,
            wasm_store,
            release_build_id,
            component_topology_digest: binding.component_topology_digest,
            controllers: expected_wasm_store_controllers(
                fleet_subnet_root,
                installation_controller,
            ),
            manifest_digest: initial_release_set.manifest_digest,
        };
        RootInstallIdentity {
            binding,
            initial_release_set,
            install_id: [3; 32],
            expected_module_hash: [11; 32],
            wasm_store_authority: FleetSubnetWasmStoreAuthority {
                authority,
                placement_subnet,
                fleet_subnet_root,
                wasm_store,
                installation_controller,
                release_build_id,
                wasm_module_hash: [14; 32],
            },
            wasm_store_activation,
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
        input.wasm_store_activation.component_topology_digest =
            input.binding.component_topology_digest;
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
        let input = input(release_build_id);
        let store_authority = input.wasm_store_authority.clone();
        let prepared = prepare_root(input, release_build_id).expect("prepare");

        assert_eq!(prepared.identity.operation_id, [3; 32]);
        assert_eq!(prepared.identity.release_build_id, release_build_id);
        assert_eq!(prepared.wasm_store_authority, Some(store_authority));
        assert_eq!(
            prepared
                .root_authority
                .expect("root authority")
                .wasm_store_activation
                .operation_id,
            [15; 32]
        );
    }

    #[test]
    fn root_install_rejects_missing_foreign_and_conflicting_store_child_authority() {
        let release_build_id = release_build(16);
        let mut missing = input(release_build_id);
        missing.wasm_store_activation.operation_id = [0; 32];
        assert_eq!(
            prepare_root(missing, release_build_id),
            Err(PrepareFleetActivationError::WasmStoreInstallIdInvalid)
        );

        let mut conflicting = input(release_build_id);
        conflicting.wasm_store_activation.operation_id = conflicting.install_id;
        assert_eq!(
            prepare_root(conflicting, release_build_id),
            Err(PrepareFleetActivationError::WasmStoreInstallIdInvalid)
        );

        let mut foreign = input(release_build_id);
        foreign.wasm_store_activation.wasm_store = Principal::from_slice(&[99; 29]);
        assert_eq!(
            prepare_root(foreign, release_build_id),
            Err(PrepareFleetActivationError::WasmStoreInstallIdInvalid)
        );

        let mut widened = input(release_build_id);
        widened
            .wasm_store_activation
            .controllers
            .push(Principal::from_slice(&[98; 29]));
        widened.wasm_store_activation.controllers.sort();
        assert_eq!(
            prepare_root(widened, release_build_id),
            Err(PrepareFleetActivationError::WasmStoreInstallIdInvalid)
        );
    }

    #[test]
    fn wasm_store_install_requires_its_exact_principal_and_release() {
        let release_build_id = release_build(15);
        let root = input(release_build_id);
        let authority = root.wasm_store_authority;
        let store = authority.wasm_store;
        let prepared = prepare_wasm_store_install(
            WasmStoreInstallIdentity {
                authority: authority.clone(),
                install_id: root.install_id,
            },
            release_build_id,
            store,
        )
        .expect("prepare Store");

        assert_eq!(prepared.root_authority, None);
        assert_eq!(prepared.wasm_store_authority, Some(authority.clone()));
        let observed = Principal::from_slice(&[99; 29]);
        assert_eq!(
            prepare_wasm_store_install(
                WasmStoreInstallIdentity {
                    authority,
                    install_id: root.install_id,
                },
                release_build_id,
                observed,
            ),
            Err(PrepareFleetActivationError::WasmStorePrincipalMismatch {
                bound: store,
                observed,
            })
        );
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
    fn root_install_rejects_invalid_protected_funding_authority() {
        let release_build_id = release_build(8);
        let mut invalid = input(release_build_id);
        invalid.binding.funding.root_funding.request_threshold = Cycles::new(1);

        assert_eq!(
            prepare_root(invalid, release_build_id),
            Err(PrepareFleetActivationError::FundingPolicy(
                FleetFundingPolicyValidationError::RootRequestThresholdBelowFloor
            ))
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
