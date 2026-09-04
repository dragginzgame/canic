//! Module: testing::managed_component_group
//!
//! Responsibility: construct one bounded managed Component tree in PocketIC.
//! Does not own: application assertions, production placement, or runtime authority.
//! Boundary: config and exact role Wasms are caller input; Canic derives protected payloads.

mod model;
#[cfg(test)]
mod tests;

pub use model::{
    ManagedComponentGroupQualificationInput, ManagedComponentNode, ManagedRoleQualificationArtifact,
};

use crate::{
    Error,
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        component_deployment::ProtectedComponentDeployment,
        component_provisioning::{
            ComponentGroupDirectory, ComponentGroupDirectoryMember,
            ComponentGroupDirectoryProvenance,
        },
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryProvenance, ComponentRuntimeDirectChild,
            ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryPreparationRequest,
        },
        fleet_admission::{
            FleetAdmissionPrepareTargetRequest, FleetAdmissionProjectionStatusResponse,
            FleetAdmissionTargetReceipt,
        },
        fleet_registry::{
            FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistryVersion,
            FleetSubnetRootDirectoryEntry, FleetSubnetRootStatus,
        },
        page::PageRequest,
        role::{
            ComponentRuntimeOperationStatus, OperationReceipt, OperationStatusRequest,
            RoleOverviewResponse,
        },
        runtime::CanicRuntimeStatus,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentChildBinding,
        ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
        ComponentGroupPlacementId, ComponentInstanceId, ComponentSpecAdmission, ComponentSpecId,
        CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
        FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
        FleetSubnetRootLimits, ManagedCanisterBinding, SubnetId,
    },
    protocol::{CANIC_COMMAND, CANIC_STATUS},
};
use candid::{CandidType, Deserialize, Principal, encode_args, encode_one};
use canic_core::{
    bootstrap::{
        compiled::{
            ComponentGroupDeploymentSpec, ComponentTopology, ConfigModel,
            FlattenedComponentGroupDeploymentMember,
        },
        parse_config_model,
    },
    cdk::{types::Cycles, utils::hash::sha256_bytes},
    ids::{
        FleetAdmissionPolicy, FleetFundingProfile, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy, ReleaseBuildId,
    },
    role_contract::ProtocolProfileDigest,
    shared_support::fleet_admission_policy::{
        compile_fleet_admission_projection, compile_installed_fleet_admission_policy,
    },
};
use ic_testkit::pic::{
    CandidCallError, CandidCallExt, CanisterInstallExt, PocketIc, PocketIcBuilder,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    time::Duration,
};

const DEFAULT_INSTALL_CYCLES: u128 = 10_000_000_000_000;
const DEFAULT_STATUS_PAGE_LIMIT: u64 = 256;
const INITIAL_DIRECTORY_REVISION: u64 = 1;
const SETTLEMENT_QUIET_OBSERVATIONS: usize = 3;
const SETUP_TICKS: usize = 96;
const TEST_ROOT_WASM: &[u8] = include_bytes!("fixture/sharding_root_stub.wasm");

/// Installed managed Component Group and every currently materialized descendant.
pub struct ManagedComponentGroupFixture {
    components: Vec<ComponentState>,
    nodes: Vec<NodeState>,
    pic: PocketIc,
    policy: FleetAdmissionPolicy,
    role_artifacts: BTreeMap<CanisterRole, ManagedRoleQualificationArtifact>,
    root: FleetSubnetRootBinding,
    topology: ComponentTopology,
}

impl ManagedComponentGroupFixture {
    /// Borrow the caller-owned PocketIC for application-specific assertions.
    #[must_use]
    pub const fn pic(&self) -> &PocketIc {
        &self.pic
    }

    /// Exact synthetic Root Principal serving the shared placement protocol.
    #[must_use]
    pub const fn root(&self) -> Principal {
        self.root.fleet_subnet_root
    }

    /// Return every installed top-level and child node in canonical Principal order.
    #[must_use]
    pub fn nodes(&self) -> Vec<ManagedComponentNode> {
        let mut nodes = self
            .nodes
            .iter()
            .map(|node| node.public.clone())
            .collect::<Vec<_>>();
        nodes.sort_by_key(|node| node.canister_id);
        nodes
    }

    /// Resolve a role only when exactly one installed node has that role.
    pub fn unique_role(
        &self,
        role: &CanisterRole,
    ) -> Result<Principal, ManagedComponentGroupQualificationError> {
        let mut matching = self
            .nodes
            .iter()
            .filter(|node| &node.public.role == role)
            .map(|node| node.public.canister_id);
        let one = matching.next().ok_or_else(|| {
            ManagedComponentGroupQualificationError::Config(format!(
                "managed Component Group has no installed role {role}"
            ))
        })?;
        if matching.next().is_some() {
            return Err(ManagedComponentGroupQualificationError::Config(format!(
                "managed Component Group has more than one installed role {role}"
            )));
        }
        Ok(one)
    }

    /// Install allocations already requested through sharding, scaling, or index placement.
    ///
    /// Call this after an application request that may create an on-demand index or scaling
    /// child. Configured initial sharding and scaling children are settled during installation.
    pub fn settle_requested_children(
        &mut self,
        maximum_ticks: usize,
    ) -> Result<usize, ManagedComponentGroupQualificationError> {
        let before = self.nodes.len();
        let mut last_allocation_count = None;
        let mut quiet_observations = 0usize;
        for _ in 0..maximum_ticks {
            self.pic.tick();
            let allocation_count = self.install_unsettled_allocations()?;
            let installed_child_count = self
                .nodes
                .iter()
                .filter(|node| node.public.parent_canister_id.is_some())
                .count();
            if self.all_nodes_ready()? && allocation_count == installed_child_count {
                if last_allocation_count == Some(allocation_count) {
                    quiet_observations = quiet_observations.saturating_add(1);
                } else {
                    quiet_observations = 1;
                }
                if quiet_observations >= SETTLEMENT_QUIET_OBSERVATIONS {
                    return Ok(self.nodes.len().saturating_sub(before));
                }
            } else {
                quiet_observations = 0;
            }
            last_allocation_count = Some(allocation_count);
            if quiet_observations < SETTLEMENT_QUIET_OBSERVATIONS {
                self.pic.advance_time(Duration::from_secs(1));
            }
        }
        Err(ManagedComponentGroupQualificationError::ProgressLimit {
            operation: "managed Component child settlement",
            maximum_ticks,
        })
    }

    /// Read one target's exact local Fleet-admission projection as the owning Root.
    pub fn admission_status(
        &self,
        target: Principal,
    ) -> Result<FleetAdmissionProjectionStatusResponse, ManagedComponentGroupQualificationError>
    {
        let response: Result<ManagedStatusResponse, Error> = self.pic.query_candid_as(
            target,
            self.root(),
            CANIC_STATUS,
            (ManagedStatusRequest::Admission(PageRequest {
                limit: DEFAULT_STATUS_PAGE_LIMIT,
                offset: 0,
            }),),
        )?;
        let response = response.map_err(ManagedComponentGroupQualificationError::Canic)?;
        let ManagedStatusResponse::Admission(status) = response else {
            return Err(ManagedComponentGroupQualificationError::UnexpectedResponse(
                "managed admission status",
            ));
        };
        Ok(status)
    }

    /// Prepare one exact successor projection for a node and leave it fenced.
    ///
    /// This uses the same target-local transition as production for top-level Components
    /// and for children created through sharding, scaling, or index placement.
    pub fn prepare_admission_successor(
        &self,
        target: Principal,
        operation_id: [u8; 32],
        admitted_principals: Vec<Principal>,
    ) -> Result<FleetAdmissionTargetReceipt, ManagedComponentGroupQualificationError> {
        let current = self.admission_status(target)?;
        let successor_policy = compile_installed_fleet_admission_policy(
            current.authority.fleet.clone(),
            current.generation.checked_add(1).ok_or_else(|| {
                ManagedComponentGroupQualificationError::Authority(
                    "managed admission generation is exhausted".to_string(),
                )
            })?,
            admitted_principals,
            Vec::new(),
        )
        .map_err(|error| ManagedComponentGroupQualificationError::Authority(error.to_string()))?;
        let successor = compile_fleet_admission_projection(&successor_policy, current.target)
            .map_err(|error| {
                ManagedComponentGroupQualificationError::Authority(error.to_string())
            })?;
        let request = FleetAdmissionPrepareTargetRequest {
            operation_id,
            expected_generation: current.generation,
            expected_policy_digest: current.policy_digest,
            successor,
        };
        let response: Result<ManagedCommandResponse, Error> = self.pic.update_candid_as(
            target,
            self.root(),
            CANIC_COMMAND,
            (ManagedCommand::PrepareFleetAdmission(Box::new(request)),),
        )?;
        let response = response.map_err(ManagedComponentGroupQualificationError::Canic)?;
        let ManagedCommandResponse::PrepareFleetAdmission(receipt) = response else {
            return Err(ManagedComponentGroupQualificationError::UnexpectedResponse(
                "managed admission preparation",
            ));
        };
        Ok(*receipt)
    }

    /// Read the exact managed binding restored by one installed node.
    pub fn binding(
        &self,
        target: Principal,
    ) -> Result<ManagedCanisterBinding, ManagedComponentGroupQualificationError> {
        let response: Result<ManagedStatusResponse, Error> = self.pic.query_candid_as(
            target,
            self.root(),
            CANIC_STATUS,
            (ManagedStatusRequest::Binding,),
        )?;
        let response = response.map_err(ManagedComponentGroupQualificationError::Canic)?;
        let ManagedStatusResponse::Binding(binding) = response else {
            return Err(ManagedComponentGroupQualificationError::UnexpectedResponse(
                "managed binding status",
            ));
        };
        Ok(binding)
    }

    /// Read the exact retained runtime operation and Directory authority for one node.
    pub fn runtime_status(
        &self,
        target: Principal,
    ) -> Result<ComponentRuntimeOperationStatus, ManagedComponentGroupQualificationError> {
        let node = self.node(target)?;
        let response: Result<ManagedStatusResponse, Error> = self.pic.query_candid_as(
            target,
            self.root(),
            CANIC_STATUS,
            (ManagedStatusRequest::Operation(OperationStatusRequest {
                operation_id: node.directory.operation_id,
            }),),
        )?;
        let response = response.map_err(ManagedComponentGroupQualificationError::Canic)?;
        let ManagedStatusResponse::Operation(status) = response else {
            return Err(ManagedComponentGroupQualificationError::UnexpectedResponse(
                "managed runtime operation status",
            ));
        };
        let ManagedOperationStatusResponse::ConfigureRuntime(status) = *status;
        Ok(status)
    }

    /// Read the target's controller-only runtime and timer diagnostics.
    pub fn diagnostic_status(
        &self,
        target: Principal,
    ) -> Result<CanicRuntimeStatus, ManagedComponentGroupQualificationError> {
        let response: Result<ManagedStatusResponse, Error> = self.pic.query_candid_as(
            target,
            self.root(),
            CANIC_STATUS,
            (ManagedStatusRequest::Runtime,),
        )?;
        let response = response.map_err(ManagedComponentGroupQualificationError::Canic)?;
        let ManagedStatusResponse::Runtime(status) = response else {
            return Err(ManagedComponentGroupQualificationError::UnexpectedResponse(
                "managed runtime diagnostics",
            ));
        };
        Ok(status)
    }

    /// Upgrade one installed node to its exact same-release Wasm.
    pub fn upgrade_same_release(
        &self,
        target: Principal,
        install_code_cooldown: Duration,
    ) -> Result<(), ManagedComponentGroupQualificationError> {
        let node = self.node(target)?;
        let artifact = self.role_artifacts.get(&node.public.role).ok_or_else(|| {
            ManagedComponentGroupQualificationError::Authority(
                "installed node has no retained role artifact".to_string(),
            )
        })?;
        self.pic
            .wait_out_install_code_rate_limit(install_code_cooldown);
        self.pic
            .upgrade_canister(
                target,
                artifact.wasm.clone(),
                encode_one(()).map_err(|error| {
                    ManagedComponentGroupQualificationError::Candid(error.to_string())
                })?,
                Some(self.root()),
            )
            .map_err(|error| ManagedComponentGroupQualificationError::Install(error.to_string()))
    }

    fn install_unsettled_allocations(
        &mut self,
    ) -> Result<usize, ManagedComponentGroupQualificationError> {
        let allocations: Vec<FixtureChildAllocation> =
            self.pic
                .query_candid(self.root(), "testing_component_child_allocations", ())?;
        let allocation_count = allocations.len();
        for allocation in allocations {
            if self
                .nodes
                .iter()
                .any(|node| node.public.canister_id == allocation.child)
            {
                continue;
            }
            self.install_child(allocation)?;
        }
        Ok(allocation_count)
    }

    fn install_child(
        &mut self,
        allocation: FixtureChildAllocation,
    ) -> Result<(), ManagedComponentGroupQualificationError> {
        let parent = self.node(allocation.parent)?.clone();
        let component_index = self
            .components
            .iter()
            .position(|component| component.binding.component == parent.component)
            .ok_or_else(|| {
                ManagedComponentGroupQualificationError::Authority(
                    "managed child parent is not bound to a fixture Component".to_string(),
                )
            })?;
        let component = &self.components[component_index];
        let component_spec = self
            .topology
            .get(&component.binding.component_spec)
            .ok_or_else(|| {
                ManagedComponentGroupQualificationError::Authority(
                    "managed child Component Spec is absent from compiled topology".to_string(),
                )
            })?;
        if component_spec
            .spawn_grant(&parent.public.role, &allocation.canister_role)
            .is_none()
        {
            return Err(ManagedComponentGroupQualificationError::Authority(format!(
                "role {} cannot create child role {} in Component Spec {}",
                parent.public.role, allocation.canister_role, component.binding.component_spec
            )));
        }
        let artifact = self
            .role_artifacts
            .get(&allocation.canister_role)
            .ok_or_else(|| {
                ManagedComponentGroupQualificationError::Config(format!(
                    "no qualification artifact was supplied for child role {}",
                    allocation.canister_role
                ))
            })?;
        self.pic
            .add_cycles(allocation.child, artifact.install_cycles);
        let binding = ComponentChildBinding {
            component: component.binding.clone(),
            parent_canister_id: allocation.parent,
            role: allocation.canister_role.clone(),
            canister_id: allocation.child,
        };
        self.topology
            .validate_component_child_binding(&self.root, &binding)
            .map_err(|error| {
                ManagedComponentGroupQualificationError::Authority(error.to_string())
            })?;
        let managed_binding = ManagedCanisterBinding::ComponentChild(binding.clone());
        let admission = if self.role_uses_admission(&allocation.canister_role)? {
            Some(
                compile_fleet_admission_projection(&self.policy, managed_binding.clone()).map_err(
                    |error| ManagedComponentGroupQualificationError::Authority(error.to_string()),
                )?,
            )
        } else {
            None
        };
        let install_id = allocation.request_id;
        let payload = CanisterInitPayload {
            admission,
            authority: CanisterInitAuthority::ComponentChild {
                root: self.root.clone(),
                binding,
            },
            component_deployment: Box::new(component.deployment.clone()),
            install_id,
            release_build_id: component.release_build_id,
        };
        let application_init_args = allocation
            .extra_arg
            .clone()
            .or_else(|| artifact.application_init_args.clone());
        let init_args = encode_args((payload, application_init_args))
            .map_err(|error| ManagedComponentGroupQualificationError::Candid(error.to_string()))?;
        self.pic.install_canister(
            allocation.child,
            artifact.wasm.clone(),
            init_args,
            Some(self.root()),
        );

        let public = ManagedComponentNode {
            binding: managed_binding,
            canister_id: allocation.child,
            component_group_member: component.member.member_path.clone(),
            component_spec: component.binding.component_spec.clone(),
            parent_canister_id: Some(allocation.parent),
            role: allocation.canister_role,
        };
        let directory = self.directory_for_new_node(component_index, install_id, &public)?;
        configure_runtime(&self.pic, self.root(), allocation.child, &directory)?;
        self.nodes.push(NodeState {
            component: component.binding.component,
            directory,
            public,
        });
        self.synchronize_component_tree(component_index)?;
        Ok(())
    }

    fn directory_for_new_node(
        &self,
        component_index: usize,
        operation_id: [u8; 32],
        node: &ManagedComponentNode,
    ) -> Result<ComponentRuntimeDirectoryPreparationRequest, ManagedComponentGroupQualificationError>
    {
        let component = &self.components[component_index];
        let descendant_count = self
            .nodes
            .iter()
            .filter(|candidate| candidate.component == component.binding.component)
            .filter(|candidate| candidate.public.parent_canister_id.is_some())
            .count()
            .checked_add(1)
            .and_then(|count| u32::try_from(count).ok())
            .ok_or_else(|| {
                ManagedComponentGroupQualificationError::Authority(
                    "managed Component descendant count exceeds u32".to_string(),
                )
            })?;
        let revision = u64::from(descendant_count)
            .checked_add(INITIAL_DIRECTORY_REVISION)
            .ok_or_else(|| {
                ManagedComponentGroupQualificationError::Authority(
                    "managed Component directory revision is exhausted".to_string(),
                )
            })?;
        Ok(ComponentRuntimeDirectoryPreparationRequest {
            authority: component.directory_authority(
                descendant_count,
                revision,
                component_content_hash(component.binding.component, revision, &self.nodes, node),
            ),
            direct_children: Vec::new(),
            operation_id,
        })
    }

    fn synchronize_component_tree(
        &mut self,
        component_index: usize,
    ) -> Result<(), ManagedComponentGroupQualificationError> {
        let component = &self.components[component_index];
        let mut component_nodes = self
            .nodes
            .iter()
            .filter(|node| node.component == component.binding.component)
            .map(|node| node.public.clone())
            .collect::<Vec<_>>();
        component_nodes.sort_by_key(|node| node.canister_id);
        let descendant_count =
            u32::try_from(component_nodes.len().saturating_sub(1)).map_err(|_| {
                ManagedComponentGroupQualificationError::Authority(
                    "managed Component descendant count exceeds u32".to_string(),
                )
            })?;
        let revision = u64::from(descendant_count)
            .checked_add(INITIAL_DIRECTORY_REVISION)
            .ok_or_else(|| {
                ManagedComponentGroupQualificationError::Authority(
                    "managed Component directory revision is exhausted".to_string(),
                )
            })?;
        let content_hash = component_content_hash_from_catalogue(
            component.binding.component,
            revision,
            &component_nodes,
        );
        let authority = component.directory_authority(descendant_count, revision, content_hash);
        let mut updates = Vec::new();
        for node in self
            .nodes
            .iter()
            .filter(|node| node.component == component.binding.component)
        {
            let mut direct_children = self
                .nodes
                .iter()
                .filter(|candidate| {
                    candidate.public.parent_canister_id == Some(node.public.canister_id)
                })
                .map(|candidate| self.direct_child(candidate))
                .collect::<Result<Vec<_>, _>>()?;
            direct_children.sort();
            let mut request = node.directory.clone();
            request.authority = authority.clone();
            request.direct_children = direct_children;
            updates.push((node.public.canister_id, request));
        }
        for (canister, request) in &updates {
            configure_runtime(&self.pic, self.root(), *canister, request)?;
        }
        for node in &mut self.nodes {
            if node.component == component.binding.component {
                node.directory = updates
                    .iter()
                    .find(|(canister, _)| *canister == node.public.canister_id)
                    .expect("every selected fixture node has one directory update")
                    .1
                    .clone();
            }
        }
        Ok(())
    }

    fn direct_child(
        &self,
        node: &NodeState,
    ) -> Result<ComponentRuntimeDirectChild, ManagedComponentGroupQualificationError> {
        Ok(ComponentRuntimeDirectChild {
            canister_id: node.public.canister_id,
            role: node.public.role.clone(),
            protocol_profile_digest: ProtocolProfileDigest::from_bytes(
                overview(&self.pic, node.public.canister_id)?.protocol_profile_digest,
            ),
        })
    }

    fn all_nodes_ready(&self) -> Result<bool, ManagedComponentGroupQualificationError> {
        self.nodes.iter().try_fold(true, |ready, node| {
            overview(&self.pic, node.public.canister_id)
                .map(|status| ready && status.bootstrap.ready)
        })
    }

    fn node(
        &self,
        canister: Principal,
    ) -> Result<&NodeState, ManagedComponentGroupQualificationError> {
        self.nodes
            .iter()
            .find(|node| node.public.canister_id == canister)
            .ok_or_else(|| {
                ManagedComponentGroupQualificationError::Config(format!(
                    "Principal {canister} is not an installed fixture node"
                ))
            })
    }

    fn role_uses_admission(
        &self,
        role: &CanisterRole,
    ) -> Result<bool, ManagedComponentGroupQualificationError> {
        let component = self
            .components
            .iter()
            .find(|component| {
                component.binding.role == *role
                    || self
                        .topology
                        .get(&component.binding.component_spec)
                        .is_some_and(|spec| spec.child(role).is_some())
            })
            .ok_or_else(|| {
                ManagedComponentGroupQualificationError::Config(format!(
                    "role {role} is outside the selected managed Component Group"
                ))
            })?;
        Ok(component
            .admitted_roles
            .iter()
            .any(|candidate| candidate == role))
    }
}

/// Install one exact Component Group and its configured initial placement children.
///
/// # Panics
///
/// Panics only if PocketIC rejects primitive canister creation or installation before
/// Canic can return typed fixture evidence.
pub fn install_managed_component_group(
    input: ManagedComponentGroupQualificationInput<'_>,
) -> Result<ManagedComponentGroupFixture, ManagedComponentGroupQualificationError> {
    let config = parse_config_model(input.app_config_source)
        .map_err(|error| ManagedComponentGroupQualificationError::Config(error.to_string()))?;
    let topology = config
        .compile_component_topology()
        .map_err(|error| ManagedComponentGroupQualificationError::Config(error.to_string()))?;
    let deployment_id = input
        .component_group_deployment
        .parse::<ComponentGroupDeploymentId>()
        .map_err(|error| ManagedComponentGroupQualificationError::Config(error.to_string()))?;
    let deployments = config
        .compile_component_group_deployment_topology()
        .map_err(|error| ManagedComponentGroupQualificationError::Config(error.to_string()))?;
    let deployment = deployments.get(&deployment_id).ok_or_else(|| {
        ManagedComponentGroupQualificationError::Config(format!(
            "Component Group deployment {deployment_id} is not declared"
        ))
    })?;
    if input.admitted_principals.is_empty() {
        return Err(ManagedComponentGroupQualificationError::Authority(
            "managed Component Group qualification requires at least one admitted Principal"
                .to_string(),
        ));
    }
    let (pic, root_principal, top_level_principals) =
        create_fixture_canisters(deployment.members.len())?;
    let seed = qualification_seed(&input, root_principal, &top_level_principals);
    let role_artifacts = validate_role_artifacts(&topology, deployment, input.role_artifacts)?;
    let release_build_id = input
        .release_build_id
        .parse::<ReleaseBuildId>()
        .map_err(|error| ManagedComponentGroupQualificationError::Authority(error.to_string()))?;
    let root = compile_root(&config, &topology, deployment, root_principal, &seed)?;
    let policy = compile_installed_fleet_admission_policy(
        root.authority.binding.fleet.clone(),
        1,
        input.admitted_principals,
        Vec::new(),
    )
    .map_err(|error| ManagedComponentGroupQualificationError::Authority(error.to_string()))?;
    let group_directory = compile_group_directory(
        &config,
        deployment,
        input.component_group_ordinal,
        &root,
        &top_level_principals,
        &topology,
        &seed,
    )?;
    let group_placement = ComponentGroupPlacementId {
        deployment: deployment.deployment.clone(),
        ordinal: input.component_group_ordinal,
    };
    let configuration_digest = config
        .compile_component_deployment_configuration_digest()
        .map_err(|error| ManagedComponentGroupQualificationError::Config(error.to_string()))?;
    let install_context = TopLevelInstallContext {
        config: &config,
        configuration_digest,
        deployment,
        fleet_directory: compile_fleet_directory(&root, &seed),
        group_directory: &group_directory,
        group_placement: &group_placement,
        pic: &pic,
        policy: &policy,
        release_build_id,
        role_artifacts: &role_artifacts,
        root: &root,
        seed: &seed,
        topology: &topology,
    };
    let (components, nodes) = install_context.install_all(&top_level_principals)?;
    let mut fixture = ManagedComponentGroupFixture {
        components,
        nodes,
        pic,
        policy,
        role_artifacts,
        root,
        topology,
    };
    for node in &fixture.nodes {
        configure_runtime(
            &fixture.pic,
            fixture.root(),
            node.public.canister_id,
            &node.directory,
        )?;
    }
    fixture.settle_requested_children(SETUP_TICKS)?;
    Ok(fixture)
}

fn create_fixture_canisters(
    component_count: usize,
) -> Result<(PocketIc, Principal, Vec<Principal>), ManagedComponentGroupQualificationError> {
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let root = pic.create_canister();
    pic.add_cycles(root, 100_000_000_000_000);
    let init_args = encode_one(())
        .map_err(|error| ManagedComponentGroupQualificationError::Candid(error.to_string()))?;
    pic.install_canister(root, TEST_ROOT_WASM.to_vec(), init_args, None);
    let components = (0..component_count)
        .map(|_| {
            let canister = pic.create_canister();
            pic.set_controllers(canister, None, vec![root])
                .expect("fresh fixture Component controller update");
            canister
        })
        .collect();
    Ok((pic, root, components))
}

struct TopLevelInstallContext<'a> {
    config: &'a ConfigModel,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    deployment: &'a ComponentGroupDeploymentSpec,
    fleet_directory: FleetDirectorySnapshot,
    group_directory: &'a ComponentGroupDirectory,
    group_placement: &'a ComponentGroupPlacementId,
    pic: &'a PocketIc,
    policy: &'a FleetAdmissionPolicy,
    release_build_id: ReleaseBuildId,
    role_artifacts: &'a BTreeMap<CanisterRole, ManagedRoleQualificationArtifact>,
    root: &'a FleetSubnetRootBinding,
    seed: &'a [u8; 32],
    topology: &'a ComponentTopology,
}

impl TopLevelInstallContext<'_> {
    fn install_all(
        &self,
        canisters: &[Principal],
    ) -> Result<(Vec<ComponentState>, Vec<NodeState>), ManagedComponentGroupQualificationError>
    {
        let mut components = Vec::with_capacity(self.deployment.members.len());
        let mut nodes = Vec::with_capacity(self.deployment.members.len());
        for (index, (member, canister)) in self
            .deployment
            .members
            .iter()
            .zip(canisters.iter().copied())
            .enumerate()
        {
            let plan = self.compile_node(index, member, canister)?;
            self.install_node(canister, &plan)?;
            components.push(plan.component);
            nodes.push(NodeState {
                component: public_component(&plan.public),
                directory: plan.directory,
                public: plan.public,
            });
        }
        Ok((components, nodes))
    }

    fn compile_node(
        &self,
        index: usize,
        member: &FlattenedComponentGroupDeploymentMember,
        canister: Principal,
    ) -> Result<TopLevelNodePlan, ManagedComponentGroupQualificationError> {
        let spec = self
            .topology
            .get(&member.component_spec)
            .ok_or_else(|| missing_component_spec(&member.component_spec))?;
        let index = u64::try_from(index).map_err(|_| {
            ManagedComponentGroupQualificationError::Authority(
                "managed Component Group member index exceeds u64".to_string(),
            )
        })?;
        let binding = ComponentBinding {
            authority: self.root.authority.clone(),
            canister_id: canister,
            component: ComponentInstanceId::from_generated_bytes(derived_identity(
                b"component",
                &[self.seed.as_slice(), &index.to_be_bytes()].concat(),
            )),
            component_spec: member.component_spec.clone(),
            fleet_subnet_root: self.root.fleet_subnet_root,
            placement_subnet: self.root.placement_subnet,
            role: spec.component_role.clone(),
            spec_hash: spec.spec_hash,
        };
        let deployment = ProtectedComponentDeployment::GroupMember {
            binding: binding.clone(),
            component_group: self.deployment.component_group.clone(),
            configuration_digest: self.configuration_digest,
            group_placement: self.group_placement.clone(),
            labels: member.labels.clone(),
            limits: member.limits.clone(),
            member_path: member.member_path.clone(),
            purpose: member.purpose.clone(),
        };
        let admitted_roles = self
            .config
            .component_spec_fleet_admission_roles(&member.component_spec)
            .ok_or_else(|| missing_component_spec(&member.component_spec))?;
        let component = ComponentState {
            admitted_roles,
            binding: binding.clone(),
            deployment: deployment.clone(),
            fleet_directory: self.fleet_directory.clone(),
            group_directory: self.group_directory.clone(),
            member: member.clone(),
            release_build_id: self.release_build_id,
        };
        let public = ManagedComponentNode {
            binding: ManagedCanisterBinding::Component(binding.clone()),
            canister_id: canister,
            component_group_member: member.member_path.clone(),
            component_spec: member.component_spec.clone(),
            parent_canister_id: None,
            role: spec.component_role.clone(),
        };
        let directory = ComponentRuntimeDirectoryPreparationRequest {
            authority: component.directory_authority(
                0,
                INITIAL_DIRECTORY_REVISION,
                derived_identity(b"component-registry-content", binding.component.as_bytes()),
            ),
            direct_children: Vec::new(),
            operation_id: derived_identity(b"install", canister.as_slice()),
        };
        let admission = component
            .admitted_roles
            .iter()
            .any(|role| role == &spec.component_role)
            .then(|| {
                compile_fleet_admission_projection(
                    self.policy,
                    ManagedCanisterBinding::Component(binding.clone()),
                )
            })
            .transpose()
            .map_err(|error| {
                ManagedComponentGroupQualificationError::Authority(error.to_string())
            })?;
        let payload = CanisterInitPayload {
            admission,
            authority: CanisterInitAuthority::Component {
                root: self.root.clone(),
                binding,
            },
            component_deployment: Box::new(deployment),
            install_id: directory.operation_id,
            release_build_id: self.release_build_id,
        };
        Ok(TopLevelNodePlan {
            component,
            directory,
            payload,
            public,
        })
    }

    fn install_node(
        &self,
        canister: Principal,
        plan: &TopLevelNodePlan,
    ) -> Result<(), ManagedComponentGroupQualificationError> {
        let artifact = self.role_artifacts.get(&plan.public.role).ok_or_else(|| {
            ManagedComponentGroupQualificationError::Config(format!(
                "no qualification artifact was supplied for Component role {}",
                plan.public.role
            ))
        })?;
        let init_args = encode_args((plan.payload.clone(), artifact.application_init_args.clone()))
            .map_err(|error| ManagedComponentGroupQualificationError::Candid(error.to_string()))?;
        self.pic.add_cycles(canister, artifact.install_cycles);
        self.pic.install_canister(
            canister,
            artifact.wasm.clone(),
            init_args,
            Some(self.root.fleet_subnet_root),
        );
        Ok(())
    }
}

struct TopLevelNodePlan {
    component: ComponentState,
    directory: ComponentRuntimeDirectoryPreparationRequest,
    payload: CanisterInitPayload,
    public: ManagedComponentNode,
}

#[derive(Clone)]
struct ComponentState {
    admitted_roles: Vec<CanisterRole>,
    binding: ComponentBinding,
    deployment: ProtectedComponentDeployment,
    fleet_directory: FleetDirectorySnapshot,
    group_directory: ComponentGroupDirectory,
    member: FlattenedComponentGroupDeploymentMember,
    release_build_id: ReleaseBuildId,
}

impl ComponentState {
    fn directory_authority(
        &self,
        descendant_count: u32,
        revision: u64,
        content_hash: [u8; 32],
    ) -> ComponentRuntimeDirectoryAuthority {
        ComponentRuntimeDirectoryAuthority {
            component: ComponentDirectoryHead {
                descendant_count,
                provenance: ComponentDirectoryProvenance {
                    component: self.binding.clone(),
                    component_registry_content_hash: content_hash,
                    component_registry_revision: revision,
                    source_fleet_subnet_root: self.binding.fleet_subnet_root,
                    synchronized_at_ns: revision,
                },
            },
            component_group: Some(self.group_directory.clone()),
            fleet: self.fleet_directory.clone(),
        }
    }
}

#[derive(Clone)]
struct NodeState {
    component: ComponentInstanceId,
    directory: ComponentRuntimeDirectoryPreparationRequest,
    public: ManagedComponentNode,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct FixtureChildAllocation {
    request_id: [u8; 32],
    parent: Principal,
    canister_role: CanisterRole,
    extra_arg: Option<Vec<u8>>,
    child: Principal,
    acknowledged: bool,
}

fn missing_component_spec(
    component_spec: &ComponentSpecId,
) -> ManagedComponentGroupQualificationError {
    ManagedComponentGroupQualificationError::Config(format!(
        "Component Spec {component_spec} is absent from compiled topology"
    ))
}

fn validate_role_artifacts(
    topology: &ComponentTopology,
    deployment: &ComponentGroupDeploymentSpec,
    artifacts: Vec<ManagedRoleQualificationArtifact>,
) -> Result<
    BTreeMap<CanisterRole, ManagedRoleQualificationArtifact>,
    ManagedComponentGroupQualificationError,
> {
    let mut required = BTreeSet::new();
    for member in &deployment.members {
        let spec = topology.get(&member.component_spec).ok_or_else(|| {
            ManagedComponentGroupQualificationError::Config(format!(
                "Component Spec {} is absent from compiled topology",
                member.component_spec
            ))
        })?;
        required.insert(spec.component_role.clone());
        for child in &spec.children {
            required.insert(child.role.clone());
        }
    }
    let mut supplied = BTreeMap::new();
    for artifact in artifacts {
        let role = artifact.role.clone();
        if supplied.insert(role.clone(), artifact).is_some() {
            return Err(ManagedComponentGroupQualificationError::Config(format!(
                "qualification artifact for role {role} is repeated"
            )));
        }
    }
    let missing = required
        .iter()
        .filter(|role| !supplied.contains_key(*role))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let extra = supplied
        .keys()
        .filter(|role| !required.contains(*role))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if !missing.is_empty() || !extra.is_empty() {
        return Err(ManagedComponentGroupQualificationError::Config(format!(
            "qualification role artifacts differ from selected topology: missing=[{}], extra=[{}]",
            missing.join(", "),
            extra.join(", ")
        )));
    }
    Ok(supplied)
}

fn compile_root(
    config: &ConfigModel,
    topology: &ComponentTopology,
    deployment: &ComponentGroupDeploymentSpec,
    root_principal: Principal,
    seed: &[u8; 32],
) -> Result<FleetSubnetRootBinding, ManagedComponentGroupQualificationError> {
    let coordinator = derived_principal(b"coordinator", seed);
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            coordinator,
            coordinator_subnet: SubnetId::from_principal(derived_principal(
                b"coordinator-subnet",
                seed,
            )),
            fleet: FleetBinding {
                app: config.app_id().clone(),
                fleet: FleetKey {
                    canonical_network_id: crate::ids::CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes(derived_identity(b"fleet", seed)),
                },
            },
        },
        epoch: 1,
    };
    let mut admissions = deployment
        .members
        .iter()
        .map(|member| {
            let spec = topology.get(&member.component_spec).ok_or_else(|| {
                ManagedComponentGroupQualificationError::Config(format!(
                    "Component Spec {} is absent from compiled topology",
                    member.component_spec
                ))
            })?;
            Ok::<_, ManagedComponentGroupQualificationError>(ComponentSpecAdmission {
                component_spec: member.component_spec.clone(),
                maximum_root_instances: spec.maximum_fleet_instances,
                spec_hash: spec.spec_hash,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    admissions.sort_by(|left, right| left.component_spec.cmp(&right.component_spec));
    admissions.dedup_by(|left, right| left.component_spec == right.component_spec);
    let component_topology_digest = topology
        .project_for_admissions(&admissions)
        .and_then(|projection| projection.digest())
        .map_err(|error| ManagedComponentGroupQualificationError::Authority(error.to_string()))?;
    Ok(FleetSubnetRootBinding {
        authority,
        component_admissions: admissions,
        component_topology_digest,
        fleet_subnet_root: root_principal,
        funding: test_root_funding_authority(),
        limits: test_root_limits(deployment.maximum_placements),
        placement_subnet: SubnetId::from_principal(derived_principal(b"placement-subnet", seed)),
    })
}

fn compile_group_directory(
    config: &ConfigModel,
    deployment: &ComponentGroupDeploymentSpec,
    ordinal: u32,
    root: &FleetSubnetRootBinding,
    principals: &[Principal],
    topology: &ComponentTopology,
    seed: &[u8; 32],
) -> Result<ComponentGroupDirectory, ManagedComponentGroupQualificationError> {
    let members = deployment
        .members
        .iter()
        .zip(principals)
        .enumerate()
        .map(|(index, (member, canister))| {
            let spec = topology.get(&member.component_spec).ok_or_else(|| {
                ManagedComponentGroupQualificationError::Config(format!(
                    "Component Spec {} is absent from compiled topology",
                    member.component_spec
                ))
            })?;
            Ok::<_, ManagedComponentGroupQualificationError>(ComponentGroupDirectoryMember {
                binding: ComponentBinding {
                    authority: root.authority.clone(),
                    canister_id: *canister,
                    component: ComponentInstanceId::from_generated_bytes(derived_identity(
                        b"component",
                        &[seed.as_slice(), &(index as u64).to_be_bytes()].concat(),
                    )),
                    component_spec: member.component_spec.clone(),
                    fleet_subnet_root: root.fleet_subnet_root,
                    placement_subnet: root.placement_subnet,
                    role: spec.component_role.clone(),
                    spec_hash: spec.spec_hash,
                },
                component_spec: member.component_spec.clone(),
                labels: member.labels.clone(),
                member_path: member.member_path.clone(),
                purpose: member.purpose.clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let configuration_digest = config
        .compile_component_deployment_configuration_digest()
        .map_err(|error| ManagedComponentGroupQualificationError::Config(error.to_string()))?;
    Ok(ComponentGroupDirectory {
        members,
        provenance: ComponentGroupDirectoryProvenance {
            authority: root.authority.clone(),
            component_group: deployment.component_group.clone(),
            fleet_subnet_root: root.fleet_subnet_root,
            group_placement: ComponentGroupPlacementId {
                deployment: deployment.deployment.clone(),
                ordinal,
            },
            operation_id: derived_identity(b"group-operation", seed),
            placement_receipt_content_hash: derived_identity(
                b"placement-receipt",
                configuration_digest.as_bytes(),
            ),
            plan_hash: derived_identity(b"placement-plan", seed),
        },
    })
}

fn compile_fleet_directory(
    root: &FleetSubnetRootBinding,
    seed: &[u8; 32],
) -> FleetDirectorySnapshot {
    FleetDirectorySnapshot {
        fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
            fleet_subnet_root: root.fleet_subnet_root,
            placement_subnet: root.placement_subnet,
            status: FleetSubnetRootStatus::Active,
        }],
        provenance: FleetDirectoryProvenance {
            registry: FleetRegistryVersion {
                authority: root.authority.clone(),
                content_hash: derived_identity(b"fleet-registry-content", seed),
                revision: 1,
            },
            source_fleet_subnet_root: root.fleet_subnet_root,
        },
        services: Vec::new(),
    }
}

fn configure_runtime(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
    directory: &ComponentRuntimeDirectoryPreparationRequest,
) -> Result<OperationReceipt, ManagedComponentGroupQualificationError> {
    let response: Result<ManagedCommandResponse, Error> = pic.update_candid_as(
        target,
        root,
        CANIC_COMMAND,
        (ManagedCommand::ConfigureRuntime(Box::new(
            directory.clone(),
        )),),
    )?;
    let response = response.map_err(ManagedComponentGroupQualificationError::Canic)?;
    let ManagedCommandResponse::OperationAccepted(receipt) = response else {
        return Err(ManagedComponentGroupQualificationError::UnexpectedResponse(
            "managed runtime configuration",
        ));
    };
    if receipt.operation_id != directory.operation_id {
        return Err(ManagedComponentGroupQualificationError::Authority(
            "managed runtime receipt operation differs from its install operation".to_string(),
        ));
    }
    Ok(receipt)
}

fn overview(
    pic: &PocketIc,
    target: Principal,
) -> Result<RoleOverviewResponse, ManagedComponentGroupQualificationError> {
    let response: Result<ManagedStatusResponse, Error> =
        pic.query_candid(target, CANIC_STATUS, (ManagedStatusRequest::Overview,))?;
    let response = response.map_err(ManagedComponentGroupQualificationError::Canic)?;
    let ManagedStatusResponse::Overview(status) = response else {
        return Err(ManagedComponentGroupQualificationError::UnexpectedResponse(
            "managed role overview",
        ));
    };
    Ok(status)
}

fn component_content_hash(
    component: ComponentInstanceId,
    revision: u64,
    existing: &[NodeState],
    new_node: &ManagedComponentNode,
) -> [u8; 32] {
    let mut catalogue = existing
        .iter()
        .filter(|node| node.component == component)
        .map(|node| node.public.clone())
        .collect::<Vec<_>>();
    catalogue.push(new_node.clone());
    catalogue.sort_by_key(|node| node.canister_id);
    component_content_hash_from_catalogue(component, revision, &catalogue)
}

fn component_content_hash_from_catalogue(
    component: ComponentInstanceId,
    revision: u64,
    catalogue: &[ManagedComponentNode],
) -> [u8; 32] {
    let mut bytes = b"canic/testing/managed-component-group/content/v1".to_vec();
    bytes.extend_from_slice(component.as_bytes());
    bytes.extend_from_slice(&revision.to_be_bytes());
    for node in catalogue {
        bytes.extend_from_slice(node.canister_id.as_slice());
        bytes.extend_from_slice(node.role.as_str().as_bytes());
    }
    derived_identity(b"component-content", &bytes)
}

const fn public_component(node: &ManagedComponentNode) -> ComponentInstanceId {
    match &node.binding {
        ManagedCanisterBinding::Component(binding) => binding.component,
        ManagedCanisterBinding::ComponentChild(binding) => binding.component.component,
    }
}

fn qualification_seed(
    input: &ManagedComponentGroupQualificationInput<'_>,
    root: Principal,
    components: &[Principal],
) -> [u8; 32] {
    let mut bytes = b"canic/testing/managed-component-group/v1".to_vec();
    bytes.extend_from_slice(input.app_config_source.as_bytes());
    bytes.extend_from_slice(input.component_group_deployment.as_bytes());
    bytes.extend_from_slice(root.as_slice());
    for component in components {
        bytes.extend_from_slice(component.as_slice());
    }
    derived_identity(b"qualification", &bytes)
}

fn derived_principal(domain: &[u8], seed: &[u8]) -> Principal {
    Principal::from_slice(&derived_identity(domain, seed)[..29])
}

fn derived_identity(domain: &[u8], seed: &[u8]) -> [u8; 32] {
    let mut bytes = b"canic/testing/managed-component-group/identity/v1".to_vec();
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(seed);
    sha256_bytes(&bytes)
        .try_into()
        .expect("SHA-256 helper always returns 32 bytes")
}

const fn test_root_funding_authority() -> FleetSubnetRootFundingAuthority {
    FleetSubnetRootFundingAuthority {
        icp_refill: None,
        root_funding: FleetSubnetRootFundingPolicy {
            budget: CyclesFundingBudget {
                maximum_cycles: Cycles::new(30_000_000_000_000),
                window_secs: 90 * 24 * 60 * 60,
            },
            cooldown_secs: 30 * 24 * 60 * 60,
            funding_profile: FleetFundingProfile::PreviewMultiSubnet,
            maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
            maximum_automatic_grants: 4,
            request_threshold: Cycles::new(10_000_000_000_000),
            target_balance: Cycles::new(30_000_000_000_000),
        },
    }
}

const fn test_root_limits(maximum_group_placements: u32) -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        canister_pool: FleetSubnetCanisterPoolConfig {
            canister_cycles: Cycles::new(1),
            creation_execution_margin: Cycles::new(1),
            maximum_size: 4_096,
            minimum_size: 1,
        },
        cycles_funding: CyclesFundingBudget {
            maximum_cycles: Cycles::new(1_000_000_000_000),
            window_secs: 3_600,
        },
        maximum_component_instances: 4_096,
        maximum_group_placements,
        maximum_registry_bytes: 8 * 1_048_576,
        maximum_wasm_store_bytes: 8 * 1_048_576,
    }
}

#[derive(CandidType)]
enum ManagedCommand {
    ConfigureRuntime(Box<ComponentRuntimeDirectoryPreparationRequest>),
    PrepareFleetAdmission(Box<FleetAdmissionPrepareTargetRequest>),
}

#[derive(CandidType, Deserialize)]
enum ManagedCommandResponse {
    OperationAccepted(OperationReceipt),
    PrepareFleetAdmission(Box<FleetAdmissionTargetReceipt>),
}

#[derive(CandidType)]
enum ManagedStatusRequest {
    Admission(PageRequest),
    Binding,
    Operation(OperationStatusRequest),
    Overview,
    Runtime,
}

#[derive(CandidType, Deserialize)]
enum ManagedStatusResponse {
    Admission(FleetAdmissionProjectionStatusResponse),
    Binding(ManagedCanisterBinding),
    Operation(Box<ManagedOperationStatusResponse>),
    Overview(RoleOverviewResponse),
    Runtime(CanicRuntimeStatus),
}

#[derive(CandidType, Deserialize)]
enum ManagedOperationStatusResponse {
    ConfigureRuntime(ComponentRuntimeOperationStatus),
}

/// Typed setup or lifecycle failure from the managed Component Group fixture.
#[derive(Debug)]
pub enum ManagedComponentGroupQualificationError {
    /// Protected authority could not be compiled consistently.
    Authority(String),
    /// Candid arguments could not be encoded.
    Candid(String),
    /// Checked-in application configuration was invalid or incomplete.
    Config(String),
    /// Canic rejected a protected lifecycle operation.
    Canic(Error),
    /// PocketIC rejected installation or same-release upgrade.
    Install(String),
    /// A bounded lifecycle observation did not reach terminal state.
    ProgressLimit {
        /// Operation that did not complete.
        operation: &'static str,
        /// Exact caller-selected bound.
        maximum_ticks: usize,
    },
    /// PocketIC transport or Candid decoding failed.
    Transport(CandidCallError),
    /// The generated Canic endpoint returned another valid variant.
    UnexpectedResponse(&'static str),
}

impl From<CandidCallError> for ManagedComponentGroupQualificationError {
    fn from(value: CandidCallError) -> Self {
        Self::Transport(value)
    }
}

impl fmt::Display for ManagedComponentGroupQualificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authority(reason) => write!(
                formatter,
                "managed Component Group authority is invalid: {reason}"
            ),
            Self::Candid(reason) => write!(
                formatter,
                "managed Component Group Candid encoding failed: {reason}"
            ),
            Self::Config(reason) => {
                write!(
                    formatter,
                    "managed Component Group config is invalid: {reason}"
                )
            }
            Self::Canic(error) => {
                write!(
                    formatter,
                    "managed Component Group lifecycle rejected: {error}"
                )
            }
            Self::Install(reason) => {
                write!(
                    formatter,
                    "managed Component Group installation failed: {reason}"
                )
            }
            Self::ProgressLimit {
                operation,
                maximum_ticks,
            } => write!(
                formatter,
                "{operation} did not complete within {maximum_ticks} PocketIC ticks"
            ),
            Self::Transport(error) => {
                write!(formatter, "managed Component Group call failed: {error}")
            }
            Self::UnexpectedResponse(operation) => write!(
                formatter,
                "{operation} returned an unexpected response variant"
            ),
        }
    }
}

impl std::error::Error for ManagedComponentGroupQualificationError {}
