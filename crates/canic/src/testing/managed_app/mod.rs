//! Module: testing::managed_app
//!
//! Responsibility: construct and drive one exact managed-App lifecycle in PocketIC.
//! Does not own: application assertions, production lifecycle, policy mutation, or artifacts.
//! Boundary: synthetic authority is local to the caller-owned PocketIC instance.

#[cfg(test)]
mod tests;

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
            ComponentDirectoryHead, ComponentDirectoryProvenance,
            ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimePhase,
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
        role::{ComponentRuntimeOperationStatus, OperationReceipt, OperationStatusRequest},
    },
    ids::{
        ComponentBinding, ComponentGroupPlacementId, ComponentInstanceId, ComponentSpecAdmission,
        ComponentSpecId, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetId,
        FleetKey, FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
        FleetSubnetRootLimits, ManagedCanisterBinding, SubnetId,
    },
    protocol::{CANIC_COMMAND, CANIC_STATUS},
};
use candid::{CandidType, Deserialize, Principal, encode_args, encode_one};
use canic_core::{
    bootstrap::parse_config_model,
    cdk::{types::Cycles, utils::hash::sha256_bytes},
    ids::{
        FleetFundingProfile, FleetSubnetRootFundingAuthority, FleetSubnetRootFundingPolicy,
        ReleaseBuildId,
    },
    shared_support::fleet_admission_policy::{
        compile_fleet_admission_projection, compile_installed_fleet_admission_policy,
    },
};
use ic_testkit::pic::{
    CandidCallError, CandidCallExt, CanisterInstallExt, PocketIc, PocketIcBuilder,
};
use std::{fmt, time::Duration};

const DEFAULT_INSTALL_CYCLES: u128 = 10_000_000_000_000;
const DEFAULT_STATUS_PAGE_LIMIT: u64 = 256;

/// Exact downstream inputs for one managed-App qualification fixture.
pub struct ManagedAppQualificationInput<'a> {
    /// Fleet-admitted callers embedded in the initial local projection.
    pub admitted_principals: Vec<Principal>,
    /// Checked-in Canic configuration source compiled into the tested App Wasm.
    pub app_config_source: &'a str,
    /// Exact Component Group deployment containing the managed App occurrence.
    pub component_group_deployment: &'a str,
    /// Placement ordinal used by the synthetic local group authority.
    pub component_group_ordinal: u32,
    /// Exact Component Spec installed by this fixture.
    pub component_spec: &'a str,
    /// Cycles added to the created PocketIC canister before installation.
    pub install_cycles: u128,
    /// Finalized release-build identity embedded in the managed init payload.
    pub release_build_id: &'a str,
    /// Exact managed App Wasm under qualification.
    pub wasm: Vec<u8>,
}

impl<'a> ManagedAppQualificationInput<'a> {
    /// Construct a managed-App input with bounded test defaults.
    #[must_use]
    pub const fn new(
        app_config_source: &'a str,
        component_group_deployment: &'a str,
        component_spec: &'a str,
        release_build_id: &'a str,
        admitted_principals: Vec<Principal>,
        wasm: Vec<u8>,
    ) -> Self {
        Self {
            admitted_principals,
            app_config_source,
            component_group_deployment,
            component_group_ordinal: 0,
            component_spec,
            install_cycles: DEFAULT_INSTALL_CYCLES,
            release_build_id,
            wasm,
        }
    }
}

/// Installed managed App plus the exact synthetic Root authority that drives it.
pub struct ManagedAppFixture {
    app: Principal,
    directory: ComponentRuntimeDirectoryPreparationRequest,
    pic: PocketIc,
    root: Principal,
    wasm: Vec<u8>,
}

impl ManagedAppFixture {
    /// Installed App Principal.
    #[must_use]
    pub const fn app(&self) -> Principal {
        self.app
    }

    /// Borrow the caller-owned PocketIC so application-specific endpoints can be tested directly.
    #[must_use]
    pub const fn pic(&self) -> &PocketIc {
        &self.pic
    }

    /// Exact synthetic Root Principal authorized to drive the managed lifecycle.
    #[must_use]
    pub const fn root(&self) -> Principal {
        self.root
    }

    /// Configure the exact retained runtime directory through Canic's managed command endpoint.
    pub fn configure_runtime(&self) -> Result<OperationReceipt, ManagedAppQualificationError> {
        let response: Result<ManagedCommandResponse, Error> = self.pic.update_candid_as(
            self.app,
            self.root,
            CANIC_COMMAND,
            (ManagedCommand::ConfigureRuntime(Box::new(
                self.directory.clone(),
            )),),
        )?;
        let response = response.map_err(ManagedAppQualificationError::Canic)?;
        let ManagedCommandResponse::OperationAccepted(receipt) = response else {
            return Err(ManagedAppQualificationError::UnexpectedResponse(
                "managed runtime configuration",
            ));
        };
        if receipt.operation_id != self.directory.operation_id {
            return Err(ManagedAppQualificationError::Authority(
                "managed runtime receipt operation differs from the exact install operation"
                    .to_string(),
            ));
        }
        Ok(receipt)
    }

    /// Advance bounded PocketIC rounds until the managed runtime is active.
    pub fn wait_until_active(
        &self,
        maximum_ticks: usize,
    ) -> Result<(), ManagedAppQualificationError> {
        for _ in 0..maximum_ticks {
            if self.runtime_is_active()? {
                return Ok(());
            }
            self.pic.advance_time(Duration::from_secs(1));
            self.pic.tick();
            self.pic.tick();
        }
        Err(ManagedAppQualificationError::ProgressLimit {
            operation: "managed runtime activation",
            maximum_ticks,
        })
    }

    /// Configure the runtime and wait for terminal activation.
    pub fn configure_and_wait_until_active(
        &self,
        maximum_ticks: usize,
    ) -> Result<OperationReceipt, ManagedAppQualificationError> {
        let receipt = self.configure_runtime()?;
        self.wait_until_active(maximum_ticks)?;
        Ok(receipt)
    }

    /// Read the exact protected local admission projection as the owning Root.
    pub fn admission_status(
        &self,
    ) -> Result<FleetAdmissionProjectionStatusResponse, ManagedAppQualificationError> {
        let response: Result<ManagedStatusResponse, Error> = self.pic.query_candid_as(
            self.app,
            self.root,
            CANIC_STATUS,
            (ManagedStatusRequest::Admission(PageRequest {
                limit: DEFAULT_STATUS_PAGE_LIMIT,
                offset: 0,
            }),),
        )?;
        let response = response.map_err(ManagedAppQualificationError::Canic)?;
        let ManagedStatusResponse::Admission(status) = response else {
            return Err(ManagedAppQualificationError::UnexpectedResponse(
                "managed admission status",
            ));
        };
        Ok(status)
    }

    /// Prepare one exact successor projection, leaving the App fenced for application assertions.
    pub fn prepare_admission_successor(
        &self,
        operation_id: [u8; 32],
        admitted_principals: Vec<Principal>,
    ) -> Result<FleetAdmissionTargetReceipt, ManagedAppQualificationError> {
        let current = self.admission_status()?;
        let successor_policy = compile_installed_fleet_admission_policy(
            current.authority.fleet.clone(),
            current.generation.checked_add(1).ok_or_else(|| {
                ManagedAppQualificationError::Authority(
                    "managed admission generation is exhausted".to_string(),
                )
            })?,
            admitted_principals,
            Vec::new(),
        )
        .map_err(|error| ManagedAppQualificationError::Authority(error.to_string()))?;
        let successor = compile_fleet_admission_projection(&successor_policy, current.target)
            .map_err(|error| ManagedAppQualificationError::Authority(error.to_string()))?;
        let request = FleetAdmissionPrepareTargetRequest {
            operation_id,
            expected_generation: current.generation,
            expected_policy_digest: current.policy_digest,
            successor,
        };
        let response: Result<ManagedCommandResponse, Error> = self.pic.update_candid_as(
            self.app,
            self.root,
            CANIC_COMMAND,
            (ManagedCommand::PrepareFleetAdmission(Box::new(request)),),
        )?;
        let response = response.map_err(ManagedAppQualificationError::Canic)?;
        let ManagedCommandResponse::PrepareFleetAdmission(receipt) = response else {
            return Err(ManagedAppQualificationError::UnexpectedResponse(
                "managed admission preparation",
            ));
        };
        Ok(receipt)
    }

    /// Upgrade the exact App to the same Wasm and retain its stable lifecycle state.
    pub fn upgrade_same_release(
        &self,
        install_code_cooldown: Duration,
    ) -> Result<(), ManagedAppQualificationError> {
        self.pic
            .wait_out_install_code_rate_limit(install_code_cooldown);
        self.pic
            .upgrade_canister(
                self.app,
                self.wasm.clone(),
                encode_one(())
                    .map_err(|error| ManagedAppQualificationError::Candid(error.to_string()))?,
                None,
            )
            .map_err(|error| ManagedAppQualificationError::Install(error.to_string()))
    }

    fn runtime_is_active(&self) -> Result<bool, ManagedAppQualificationError> {
        let response: Result<ManagedStatusResponse, Error> = self.pic.query_candid_as(
            self.app,
            self.root,
            CANIC_STATUS,
            (ManagedStatusRequest::Operation(OperationStatusRequest {
                operation_id: self.directory.operation_id,
            }),),
        )?;
        let response = response.map_err(ManagedAppQualificationError::Canic)?;
        let ManagedStatusResponse::Operation(status) = response else {
            return Err(ManagedAppQualificationError::UnexpectedResponse(
                "managed runtime status",
            ));
        };
        let ManagedOperationStatusResponse::ConfigureRuntime(status) = *status;
        Ok(status.runtime.phase == ComponentRuntimePhase::Active)
    }
}

/// Standalone-local App installed without managed authority.
pub struct StandaloneAppFixture {
    app: Principal,
    pic: PocketIc,
    wasm: Vec<u8>,
}

impl StandaloneAppFixture {
    /// Installed App Principal.
    #[must_use]
    pub const fn app(&self) -> Principal {
        self.app
    }

    /// Borrow the caller-owned PocketIC for application-specific assertions.
    #[must_use]
    pub const fn pic(&self) -> &PocketIc {
        &self.pic
    }

    /// Upgrade the standalone-local App to the same Wasm.
    pub fn upgrade_same_release(
        &self,
        install_code_cooldown: Duration,
    ) -> Result<(), ManagedAppQualificationError> {
        self.pic
            .wait_out_install_code_rate_limit(install_code_cooldown);
        self.pic
            .upgrade_canister(
                self.app,
                self.wasm.clone(),
                encode_one(())
                    .map_err(|error| ManagedAppQualificationError::Candid(error.to_string()))?,
                None,
            )
            .map_err(|error| ManagedAppQualificationError::Install(error.to_string()))
    }
}

/// Install one exact managed App into a fresh application-subnet PocketIC.
///
/// # Panics
///
/// Panics only if PocketIC rejects creation or installation before the managed
/// Canic protocol can return typed evidence.
pub fn install_managed_app(
    input: ManagedAppQualificationInput<'_>,
) -> Result<ManagedAppFixture, ManagedAppQualificationError> {
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let app = pic.create_canister();
    pic.add_cycles(app, input.install_cycles);
    let compiled = compile_managed_app(&input, app)?;
    pic.install_canister(app, input.wasm.clone(), compiled.init_args, None);
    Ok(ManagedAppFixture {
        app,
        directory: compiled.directory,
        pic,
        root: compiled.root,
        wasm: input.wasm,
    })
}

/// Install one standalone-local App into a fresh application-subnet PocketIC.
///
/// # Panics
///
/// Panics if PocketIC rejects creation or installation.
#[must_use]
pub fn install_standalone_app(wasm: Vec<u8>, install_cycles: u128) -> StandaloneAppFixture {
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let app = pic.create_canister();
    pic.add_cycles(app, install_cycles);
    pic.install_canister(
        app,
        wasm.clone(),
        encode_one(None::<Vec<u8>>).expect("encode standalone-local init argument"),
        None,
    );
    StandaloneAppFixture { app, pic, wasm }
}

#[derive(Debug)]
struct CompiledManagedApp {
    directory: ComponentRuntimeDirectoryPreparationRequest,
    init_args: Vec<u8>,
    root: Principal,
}

#[expect(
    clippy::too_many_lines,
    reason = "one pure boundary derives the mutually consistent init and Directory authorities"
)]
fn compile_managed_app(
    input: &ManagedAppQualificationInput<'_>,
    app: Principal,
) -> Result<CompiledManagedApp, ManagedAppQualificationError> {
    let config = parse_config_model(input.app_config_source)
        .map_err(|error| ManagedAppQualificationError::Config(error.to_string()))?;
    let component_spec = input
        .component_spec
        .parse::<ComponentSpecId>()
        .map_err(|error| ManagedAppQualificationError::Config(error.to_string()))?;
    let component_topology = config
        .compile_component_topology()
        .map_err(|error| ManagedAppQualificationError::Config(error.to_string()))?;
    let spec = component_topology.get(&component_spec).ok_or_else(|| {
        ManagedAppQualificationError::Config(format!(
            "Component Spec {component_spec} is not declared"
        ))
    })?;
    let deployment_id = input
        .component_group_deployment
        .parse::<canic_core::ids::ComponentGroupDeploymentId>()
        .map_err(|error| ManagedAppQualificationError::Config(error.to_string()))?;
    let deployments = config
        .compile_component_group_deployment_topology()
        .map_err(|error| ManagedAppQualificationError::Config(error.to_string()))?;
    let deployment = deployments.get(&deployment_id).ok_or_else(|| {
        ManagedAppQualificationError::Config(format!(
            "Component Group deployment {deployment_id} is not declared"
        ))
    })?;
    let mut matching_members = deployment
        .members
        .iter()
        .filter(|member| member.component_spec == component_spec);
    let member = matching_members.next().ok_or_else(|| {
        ManagedAppQualificationError::Config(format!(
            "deployment {deployment_id} does not contain Component Spec {component_spec}"
        ))
    })?;
    if matching_members.next().is_some() {
        return Err(ManagedAppQualificationError::Config(format!(
            "deployment {deployment_id} contains Component Spec {component_spec} more than once"
        )));
    }
    if input.admitted_principals.is_empty() {
        return Err(ManagedAppQualificationError::Authority(
            "managed-App qualification requires at least one admitted Principal".to_string(),
        ));
    }

    let seed = qualification_seed(input, app);
    let root_principal = derived_principal(b"root", &seed);
    let coordinator = derived_principal(b"coordinator", &seed);
    let placement_subnet = SubnetId::from_principal(derived_principal(b"placement-subnet", &seed));
    let coordinator_subnet =
        SubnetId::from_principal(derived_principal(b"coordinator-subnet", &seed));
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: crate::ids::CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes(derived_identity(b"fleet", &seed)),
        },
        app: config.app_id().clone(),
    };
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet.clone(),
            coordinator_subnet,
            coordinator,
        },
        epoch: 1,
    };
    let admissions = vec![ComponentSpecAdmission {
        component_spec: component_spec.clone(),
        maximum_root_instances: spec.maximum_fleet_instances,
        spec_hash: spec.spec_hash,
    }];
    let component_topology_digest = component_topology
        .project_for_admissions(&admissions)
        .and_then(|topology| topology.digest())
        .map_err(|error| ManagedAppQualificationError::Authority(error.to_string()))?;
    let root = FleetSubnetRootBinding {
        authority: authority.clone(),
        component_admissions: admissions,
        component_topology_digest,
        fleet_subnet_root: root_principal,
        funding: test_root_funding_authority(),
        limits: test_root_limits(spec.maximum_fleet_instances),
        placement_subnet,
    };
    let binding = ComponentBinding {
        authority,
        canister_id: app,
        component: ComponentInstanceId::from_generated_bytes(derived_identity(b"component", &seed)),
        component_spec,
        fleet_subnet_root: root_principal,
        placement_subnet,
        role: spec.component_role.clone(),
        spec_hash: spec.spec_hash,
    };
    let group_placement = ComponentGroupPlacementId {
        deployment: deployment.deployment.clone(),
        ordinal: input.component_group_ordinal,
    };
    let protected_deployment = ProtectedComponentDeployment::GroupMember {
        binding: binding.clone(),
        component_group: deployment.component_group.clone(),
        configuration_digest: config
            .compile_component_deployment_configuration_digest()
            .map_err(|error| ManagedAppQualificationError::Config(error.to_string()))?,
        group_placement: group_placement.clone(),
        labels: member.labels.clone(),
        limits: member.limits.clone(),
        member_path: member.member_path.clone(),
        purpose: member.purpose.clone(),
    };
    let policy = compile_installed_fleet_admission_policy(
        fleet,
        1,
        input.admitted_principals.clone(),
        Vec::new(),
    )
    .map_err(|error| ManagedAppQualificationError::Authority(error.to_string()))?;
    let admission = compile_fleet_admission_projection(
        &policy,
        ManagedCanisterBinding::Component(binding.clone()),
    )
    .map_err(|error| ManagedAppQualificationError::Authority(error.to_string()))?;
    let install_id = derived_identity(b"install", &seed);
    let release_build_id = input
        .release_build_id
        .parse::<ReleaseBuildId>()
        .map_err(|error| ManagedAppQualificationError::Authority(error.to_string()))?;
    let directory = directory_request(
        &seed,
        install_id,
        &root,
        &binding,
        &group_placement,
        deployment,
        member,
    );
    let payload = CanisterInitPayload {
        admission: Some(admission),
        authority: CanisterInitAuthority::Component { binding, root },
        component_deployment: Box::new(protected_deployment),
        install_id,
        release_build_id,
    };
    let init_args = encode_args((payload, None::<Vec<u8>>))
        .map_err(|error| ManagedAppQualificationError::Candid(error.to_string()))?;
    Ok(CompiledManagedApp {
        directory,
        init_args,
        root: root_principal,
    })
}

fn directory_request(
    seed: &[u8; 32],
    operation_id: [u8; 32],
    root: &FleetSubnetRootBinding,
    binding: &ComponentBinding,
    group_placement: &ComponentGroupPlacementId,
    deployment: &canic_core::bootstrap::compiled::ComponentGroupDeploymentSpec,
    member: &canic_core::bootstrap::compiled::FlattenedComponentGroupDeploymentMember,
) -> ComponentRuntimeDirectoryPreparationRequest {
    ComponentRuntimeDirectoryPreparationRequest {
        authority: ComponentRuntimeDirectoryAuthority {
            component: ComponentDirectoryHead {
                descendant_count: 0,
                provenance: ComponentDirectoryProvenance {
                    component: binding.clone(),
                    component_registry_content_hash: derived_identity(
                        b"component-registry-content",
                        seed,
                    ),
                    component_registry_revision: 1,
                    source_fleet_subnet_root: root.fleet_subnet_root,
                    synchronized_at_ns: 1,
                },
            },
            component_group: Some(ComponentGroupDirectory {
                members: vec![ComponentGroupDirectoryMember {
                    binding: binding.clone(),
                    component_spec: member.component_spec.clone(),
                    labels: member.labels.clone(),
                    member_path: member.member_path.clone(),
                    purpose: member.purpose.clone(),
                }],
                provenance: ComponentGroupDirectoryProvenance {
                    authority: root.authority.clone(),
                    component_group: deployment.component_group.clone(),
                    fleet_subnet_root: root.fleet_subnet_root,
                    group_placement: group_placement.clone(),
                    operation_id: derived_identity(b"group-operation", seed),
                    placement_receipt_content_hash: derived_identity(b"placement-receipt", seed),
                    plan_hash: derived_identity(b"placement-plan", seed),
                },
            }),
            fleet: FleetDirectorySnapshot {
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
            },
        },
        direct_children: Vec::new(),
        operation_id,
    }
}

fn qualification_seed(input: &ManagedAppQualificationInput<'_>, canister: Principal) -> [u8; 32] {
    let mut bytes = b"canic/testing/managed-app/v1".to_vec();
    bytes.extend_from_slice(input.app_config_source.as_bytes());
    bytes.extend_from_slice(input.component_group_deployment.as_bytes());
    bytes.extend_from_slice(input.component_spec.as_bytes());
    bytes.extend_from_slice(canister.as_slice());
    derived_identity(b"qualification", &bytes)
}

fn derived_principal(domain: &[u8], seed: &[u8]) -> Principal {
    Principal::from_slice(&derived_identity(domain, seed)[..29])
}

fn derived_identity(domain: &[u8], seed: &[u8]) -> [u8; 32] {
    let mut bytes = b"canic/testing/managed-app/identity/v1".to_vec();
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

const fn test_root_limits(maximum_component_instances: u32) -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        canister_pool: FleetSubnetCanisterPoolConfig {
            canister_cycles: Cycles::new(1),
            creation_execution_margin: Cycles::new(1),
            maximum_size: maximum_component_instances,
            minimum_size: 1,
        },
        cycles_funding: CyclesFundingBudget {
            maximum_cycles: Cycles::new(1_000_000_000_000),
            window_secs: 3_600,
        },
        maximum_component_instances,
        maximum_group_placements: maximum_component_instances,
        maximum_registry_bytes: 1_048_576,
        maximum_wasm_store_bytes: 64 * 1_048_576,
    }
}

#[derive(CandidType)]
enum ManagedCommand {
    ConfigureRuntime(Box<ComponentRuntimeDirectoryPreparationRequest>),
    PrepareFleetAdmission(Box<FleetAdmissionPrepareTargetRequest>),
}

#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the test decoder mirrors the generated managed response wire"
)]
enum ManagedCommandResponse {
    OperationAccepted(OperationReceipt),
    PrepareFleetAdmission(FleetAdmissionTargetReceipt),
}

#[derive(CandidType)]
enum ManagedStatusRequest {
    Admission(PageRequest),
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the test decoder mirrors the generated managed status wire"
)]
enum ManagedStatusResponse {
    Admission(FleetAdmissionProjectionStatusResponse),
    Operation(Box<ManagedOperationStatusResponse>),
}

#[derive(CandidType, Deserialize)]
enum ManagedOperationStatusResponse {
    ConfigureRuntime(ComponentRuntimeOperationStatus),
}

/// Typed setup or lifecycle failure from the managed-App qualification surface.
#[derive(Debug)]
pub enum ManagedAppQualificationError {
    /// Protected authority could not be compiled consistently.
    Authority(String),
    /// Candid arguments could not be encoded.
    Candid(String),
    /// Checked-in App configuration was invalid or ambiguous.
    Config(String),
    /// Canic rejected a protected lifecycle operation.
    Canic(Error),
    /// PocketIC rejected installation or same-release upgrade.
    Install(String),
    /// A bounded lifecycle observation did not reach terminal state.
    ProgressLimit {
        operation: &'static str,
        maximum_ticks: usize,
    },
    /// PocketIC transport or Candid decoding failed.
    Transport(CandidCallError),
    /// The generated Canic endpoint returned another valid variant.
    UnexpectedResponse(&'static str),
}

impl From<CandidCallError> for ManagedAppQualificationError {
    fn from(value: CandidCallError) -> Self {
        Self::Transport(value)
    }
}

impl fmt::Display for ManagedAppQualificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authority(reason) => {
                write!(formatter, "managed-App authority is invalid: {reason}")
            }
            Self::Candid(reason) => {
                write!(formatter, "managed-App Candid encoding failed: {reason}")
            }
            Self::Config(reason) => write!(formatter, "managed-App config is invalid: {reason}"),
            Self::Canic(error) => write!(formatter, "managed-App lifecycle rejected: {error}"),
            Self::Install(reason) => write!(formatter, "managed-App installation failed: {reason}"),
            Self::ProgressLimit {
                operation,
                maximum_ticks,
            } => write!(
                formatter,
                "{operation} did not complete within {maximum_ticks} PocketIC ticks"
            ),
            Self::Transport(error) => write!(formatter, "managed-App call failed: {error}"),
            Self::UnexpectedResponse(operation) => {
                write!(
                    formatter,
                    "{operation} returned an unexpected response variant"
                )
            }
        }
    }
}

impl std::error::Error for ManagedAppQualificationError {}
