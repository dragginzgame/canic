use crate::{
    dto::{component_deployment::ProtectedComponentDeployment, prelude::*},
    ids::{
        ComponentBinding, ComponentChildBinding, FleetAdmissionProjection, FleetSubnetRootBinding,
        ReleaseBuildId,
    },
};

///
/// CanisterInitAuthority
///
/// Exact authority from which one managed non-root initializes its immutable identity.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CanisterInitAuthority {
    Component {
        root: FleetSubnetRootBinding,
        binding: ComponentBinding,
    },
    ComponentChild {
        root: FleetSubnetRootBinding,
        binding: ComponentChildBinding,
    },
}

//
// CanisterInitPayload
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CanisterInitPayload {
    pub install_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
    pub authority: CanisterInitAuthority,
    pub component_deployment: Box<ProtectedComponentDeployment>,
    pub admission: Option<FleetAdmissionProjection>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::types::Cycles;
    use crate::ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentInstanceId, ComponentSpecId,
        ComponentTopologyDigest, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits, ReleaseBuildNonce,
        SubnetId,
    };

    #[test]
    fn managed_nonroot_init_payload_roundtrips_the_exact_component_authority() {
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([1; 32]),
            },
            app: AppId::from("toko"),
        };
        let release_build_id =
            ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([2; 32]));
        let principal = Principal::from_slice(&[3; 29]);
        let authority = FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet,
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[4; 29])),
                coordinator: Principal::from_slice(&[5; 29]),
            },
            epoch: 1,
        };
        let root = FleetSubnetRootBinding {
            authority: authority.clone(),
            placement_subnet: SubnetId::from_principal(Principal::from_slice(&[6; 29])),
            fleet_subnet_root: Principal::from_slice(&[7; 29]),
            component_admissions: Vec::new(),
            component_topology_digest: ComponentTopologyDigest::from_bytes([8; 32]),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 1,
                maximum_registry_bytes: 16 * 1_024 * 1_024,
                maximum_wasm_store_bytes: 16 * 1_024 * 1_024,
                maximum_group_placements: 16,
                canister_pool: crate::ids::FleetSubnetCanisterPoolConfig {
                    minimum_size: 1,
                    maximum_size: 10,
                    canister_cycles: Cycles::new(5_000_000_000_000),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(1_000_000_000_000),
                },
            },
            funding: crate::test::support::fleet_subnet_root_funding_authority(),
        };
        let binding = ComponentBinding {
            authority,
            component: ComponentInstanceId::from_generated_bytes([9; 32]),
            component_spec: ComponentSpecId::try_from(String::from("default"))
                .expect("default Component Spec ID"),
            spec_hash: [10; 32],
            role: CanisterRole::new("app"),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: principal,
        };
        let payload = CanisterInitPayload {
            install_id: [11; 32],
            release_build_id,
            component_deployment: Box::new(ProtectedComponentDeployment::UngroupedOrdinary {
                binding: binding.clone(),
            }),
            authority: CanisterInitAuthority::Component {
                root,
                binding: binding.clone(),
            },
            admission: Some(crate::test::support::fleet_admission_projection(
                crate::ids::ManagedCanisterBinding::Component(binding.clone()),
            )),
        };

        let bytes = candid::encode_one(&payload).expect("encode managed non-root init payload");
        let decoded: CanisterInitPayload =
            candid::decode_one(&bytes).expect("decode managed non-root init payload");

        assert_eq!(decoded.install_id, [11; 32]);
        assert_eq!(decoded.release_build_id, release_build_id);
        assert_eq!(decoded.admission, payload.admission);
        assert_eq!(decoded.component_deployment, payload.component_deployment);
        assert_eq!(
            decoded.authority,
            CanisterInitAuthority::Component {
                root: match &payload.authority {
                    CanisterInitAuthority::Component { root, .. } => root.clone(),
                    CanisterInitAuthority::ComponentChild { .. } => unreachable!(),
                },
                binding,
            }
        );
    }
}
