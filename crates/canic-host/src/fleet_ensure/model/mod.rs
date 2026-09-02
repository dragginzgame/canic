//! Module: fleet_ensure::model
//!
//! Responsibility: own current-generation Fleet reconciliation state and conservation records.
//! Does not own: transport parsing, policy decisions, persistence, or IC effects.
//! Boundary: workflow persists these records before and after every effect.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FLEET_ENSURE_SCHEMA_VERSION: u16 = 1;
pub const MAX_FLEET_ENSURE_CANISTERS: usize = 4_096;
pub const MAX_FLEET_ENSURE_PROTOCOL_STEPS: usize = 4_096;

/// One exact observed canister state used by planning and effect reconciliation.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveCanister {
    pub canister_version: Option<u64>,
    pub controllers: Vec<String>,
    #[serde(with = "u128_text")]
    pub cycles: u128,
    pub module_sha256: Option<String>,
    pub principal: String,
    pub reinstall_required: bool,
    pub root_owned_lifecycle: Option<RootOwnedCanisterLifecycle>,
    pub status: CanisterRuntimeStatus,
}

/// Root-owned lifecycle of one explicitly seeded paid canister.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootOwnedCanisterLifecycle {
    Claimed,
    Idle,
    /// Last exact current-generation balance retained while Root status is fenced.
    Retained,
    Store,
    Workload,
}

/// Management status relevant to reconciliation.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanisterRuntimeStatus {
    Running,
    Stopped,
    Stopping,
}

/// Complete live snapshot of the explicitly controlled Fleet estate.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetObservation {
    pub additional_controlled_cycles: BTreeMap<String, u128>,
    pub canisters: BTreeMap<String, Option<LiveCanister>>,
    #[serde(with = "u128_text")]
    pub ledger_fee_cycles: u128,
    #[serde(with = "u128_text")]
    pub operator_cycles: u128,
    pub protocol_ready: BTreeMap<String, bool>,
}

/// Maintained disposition of one configured canister.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DesiredPresence {
    Absent,
    Present,
}

/// Exact method used by a controlled source to transfer its cycles before retirement.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DrainAuthority {
    pub candid: String,
    pub method: String,
    pub destination: String,
    pub maximum_execution_burn_cycles: String,
}

/// Current desired state for one controlled canister.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredCanister {
    #[serde(default)]
    pub canic_init: Option<DesiredCanisterInit>,
    #[serde(default)]
    pub controller_canisters: Vec<String>,
    pub controllers: Vec<String>,
    pub drain: Option<DrainAuthority>,
    pub initial_cycles: String,
    pub init_arg: Option<String>,
    pub init_candid: Option<String>,
    pub kind: DesiredCanisterKind,
    pub minimum_cycles: String,
    pub name: String,
    pub parent: Option<String>,
    pub presence: DesiredPresence,
    pub principal: Option<String>,
    #[serde(skip)]
    pub(crate) protocol_binding: Option<crate::protocol_binding::RegistryProtocolBinding>,
    pub replace: bool,
    pub subnet: String,
    pub wasm: Option<String>,
}

/// Canic-owned role class used to derive Fleet control-plane choreography.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DesiredCanisterKind {
    Auxiliary,
    Component,
    Coordinator,
    Pool,
    Root,
    Store,
}

/// Canic-owned typed initialization role for one infrastructure canister.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "role")]
pub enum DesiredCanisterInit {
    Coordinator,
    Root { root: String },
    Store { root: String },
}

/// One current-only idempotent protocol transition owned by the Fleet journal.
///
/// The three argument documents use Candid text. They may bind a dynamically
/// created canister with `{{principal:<logical-name>}}`, the retained operation
/// string with `{{operation_id}}`, or its 32-byte value with
/// `{{operation_id_blob}}`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DesiredProtocolStep {
    pub canister: String,
    pub candid: String,
    pub command_args: String,
    pub command_method: String,
    pub expected_status: String,
    pub maximum_execution_burn_cycles: String,
    pub name: String,
    pub status_args: String,
    pub status_method: String,
}

/// Current-only desired Fleet contract consumed by `canic fleet ensure`.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredFleet {
    #[serde(default)]
    pub bootstrap: Option<DesiredFleetBootstrap>,
    pub canisters: Vec<DesiredCanister>,
    pub cycles_ledger: String,
    pub environment: String,
    pub fleet: String,
    pub ledger_fee_cycles: String,
    pub management_creation_fee_cycles: String,
    pub material_cycle_threshold: String,
    pub maximum_observation_burn_cycles: String,
    pub maximum_stalled_observations: u32,
    pub maximum_update_burn_cycles: String,
    pub operator: String,
    #[serde(default)]
    pub protocol: Option<DesiredFleetProtocol>,
    #[serde(skip)]
    pub(crate) protocol_steps: Vec<DesiredProtocolStep>,
    pub schema_version: u16,
    /// Logical name of the present controlled canister that owns retirement receipts.
    pub treasury: String,
}

/// Complete current-generation Canic initialization authority generated from App and Fleet input.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredFleetBootstrap {
    pub admission: canic_core::ids::FleetAdmissionPolicyTemplate,
    pub app: canic_core::ids::AppId,
    pub canonical_network_id: canic_core::ids::CanonicalNetworkId,
    pub component_deployment_configuration:
        canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    pub coordinator: String,
    pub coordinator_subnet: canic_core::ids::SubnetId,
    pub fleet_id: canic_core::ids::FleetId,
    pub fresh_estate: bool,
    pub release_build_id: canic_core::ids::ReleaseBuildId,
    pub root_funding: Option<canic_core::ids::FleetCoordinatorRootFundingPolicy>,
    pub roots: Vec<DesiredFleetBootstrapRoot>,
}

/// One Root/Store pair's generated immutable initialization authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredFleetBootstrapRoot {
    pub canister_pool_imports: Vec<String>,
    pub component_admissions: Vec<canic_core::ids::ComponentSpecAdmission>,
    pub component_topology_digest: canic_core::ids::ComponentTopologyDigest,
    pub funding: canic_core::ids::FleetSubnetRootFundingAuthority,
    pub limits: canic_core::ids::FleetSubnetRootLimits,
    pub placement_subnet: canic_core::ids::SubnetId,
    pub root: String,
    pub store: String,
}

/// Current Canic-owned control-plane intent compiled from one App topology.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredFleetProtocol {
    /// Canonical application `canic.toml` used to compile Component topology.
    pub app_config: String,
    /// Exact initial group placements assigned to declared Root roles.
    pub component_group_placements: Vec<DesiredComponentGroupPlacement>,
    /// Exact Coordinator Candid used only by Canic's typed protocol adapter.
    pub coordinator_candid: String,
    /// Exact Root Candid used only by Canic's typed protocol adapter.
    pub root_candid: String,
    /// Exact Store Candid used only by Canic's typed protocol adapter.
    pub store_candid: String,
}

/// One typed initial Component Group placement owned by Canic policy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredComponentGroupPlacement {
    pub deployment: String,
    pub ordinal: u32,
    pub root: String,
}

/// Content identities resolved from current desired-state artifact paths by ops.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DesiredFleetArtifacts {
    pub drain_candid_sha256_by_canister: BTreeMap<String, String>,
    pub init_arg_sha256_by_canister: BTreeMap<String, String>,
    pub init_candid_sha256_by_canister: BTreeMap<String, String>,
    pub wasm_sha256_by_canister: BTreeMap<String, String>,
    pub(crate) protocol_by_step: BTreeMap<String, ProtocolArtifactDigests>,
}

/// Content identities for one declarative protocol transition.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProtocolArtifactDigests {
    pub candid_sha256: String,
    pub command_args_sha256: String,
    pub expected_status_sha256: String,
    pub status_args_sha256: String,
}

/// Validated numeric funding bounds for one desired canister.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterCyclePolicy {
    #[serde(with = "u128_text")]
    pub initial_cycles: u128,
    #[serde(with = "u128_text")]
    pub minimum_cycles: u128,
}

/// Exact effect kind retained before execution.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EnsureAction {
    Create {
        controller_canisters: Vec<String>,
        controllers: Vec<String>,
        created_at_time: u64,
        ledger: String,
        name: String,
        #[serde(with = "u128_text")]
        requested_initial_cycles: u128,
        subnet: String,
    },
    Delete {
        #[serde(with = "u128_text")]
        maximum_remaining_cycles: u128,
        name: String,
        principal: String,
    },
    Fund {
        #[serde(with = "u128_text")]
        amount: u128,
        created_at_time: u64,
        #[serde(default, with = "u128_text", skip_serializing_if = "is_zero_u128")]
        expected_post_cycles: u128,
        #[serde(default, with = "u128_text", skip_serializing_if = "is_zero_u128")]
        funding_deficit_cycles: u128,
        #[serde(default, with = "u128_text", skip_serializing_if = "is_zero_u128")]
        funding_margin_cycles: u128,
        ledger: String,
        name: String,
        principal: String,
    },
    Install {
        canic_init: Option<DesiredCanisterInit>,
        init_arg: Option<String>,
        init_arg_sha256: Option<String>,
        init_candid: Option<String>,
        init_candid_sha256: Option<String>,
        mode: InstallMode,
        name: String,
        principal: String,
        wasm: String,
        wasm_sha256: String,
    },
    FleetProtocol {
        action: Box<CurrentFleetProtocolAction>,
        candid: String,
        candid_sha256: String,
        #[serde(with = "u128_text")]
        maximum_execution_burn_cycles: u128,
        name: String,
        principal: String,
    },
    Protocol {
        candid: String,
        candid_sha256: String,
        command_args: String,
        command_args_sha256: String,
        command_method: String,
        expected_status: String,
        expected_status_sha256: String,
        #[serde(with = "u128_text")]
        maximum_execution_burn_cycles: u128,
        name: String,
        principal: String,
        status_args: String,
        status_args_sha256: String,
        status_method: String,
    },
    SetControllers {
        controller_canisters: Vec<String>,
        controllers: Vec<String>,
        name: String,
        principal: String,
    },
    Start {
        name: String,
        principal: String,
    },
    Stop {
        name: String,
        principal: String,
    },
    Transfer {
        #[serde(with = "u128_text")]
        amount: u128,
        candid: String,
        candid_sha256: String,
        destination: String,
        #[serde(with = "u128_text")]
        maximum_execution_burn_cycles: u128,
        method: String,
        name: String,
        principal: String,
    },
}

impl EnsureAction {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Create { name, .. }
            | Self::Delete { name, .. }
            | Self::FleetProtocol { name, .. }
            | Self::Fund { name, .. }
            | Self::Install { name, .. }
            | Self::Protocol { name, .. }
            | Self::SetControllers { name, .. }
            | Self::Start { name, .. }
            | Self::Stop { name, .. }
            | Self::Transfer { name, .. } => name,
        }
    }
}

/// Closed current-only Canic choreography emitted from roles and topology.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[expect(
    clippy::large_enum_variant,
    reason = "each exact typed action is boxed by EnsureAction before durable retention"
)]
pub enum CurrentFleetProtocolAction {
    ActivateRegistry {
        expected_registry: canic_core::dto::fleet_registry::FleetRegistry,
        expected_version: canic_core::dto::fleet_registry::FleetRegistryVersion,
        request: canic_core::dto::fleet_registry::FleetRegistryActivationRequest,
    },
    ActivateRegistryMirror {
        expected: canic_core::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationResponse,
        request: canic_core::dto::fleet_registry::FleetSubnetRootRegistrySyncRequest,
    },
    AdoptStore {
        request: canic_core::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest,
    },
    BootstrapStore {
        expected: canic_core::dto::root_store::RootStoreBootstrapResponse,
        request: canic_core::dto::root_store::RootStoreBootstrapRequest,
    },
    JoinRoot {
        expected_registry: canic_core::dto::fleet_registry::FleetRegistry,
        expected_version: canic_core::dto::fleet_registry::FleetRegistryVersion,
        request: canic_core::dto::fleet_registry::FleetSubnetRootJoinRequest,
    },
    PrepareStoreChunkSet {
        request: canic_control_plane::dto::template::TemplateChunkSetPrepareInput,
    },
    PrepareComponentRegistry {
        expected: canic_core::dto::component_registry::RootComponentRegistryStatusResponse,
        request: canic_core::dto::component_registry::RootComponentRegistryPreparationRequest,
    },
    ProvisionComponents {
        plan_hash: [u8; 32],
        request: canic_core::dto::component_provisioning::FleetComponentProvisioningPrepareRequest,
    },
    PublishStoreChunk {
        request: canic_control_plane::dto::template::TemplateChunkInput,
    },
    StageStoreManifest {
        request: canic_control_plane::dto::template::TemplateManifestInput,
    },
    SynchronizeRegistry {
        expected: canic_core::dto::fleet_registry::FleetSubnetRootRegistrySyncResponse,
        request: canic_core::dto::fleet_registry::FleetSubnetRootRegistrySyncRequest,
    },
}

impl CurrentFleetProtocolAction {
    /// Role that exclusively owns this current control-plane transition.
    #[must_use]
    pub const fn target_kind(&self) -> DesiredCanisterKind {
        match self {
            Self::ActivateRegistry { .. }
            | Self::JoinRoot { .. }
            | Self::ProvisionComponents { .. } => DesiredCanisterKind::Coordinator,
            Self::ActivateRegistryMirror { .. }
            | Self::AdoptStore { .. }
            | Self::BootstrapStore { .. }
            | Self::PrepareComponentRegistry { .. }
            | Self::SynchronizeRegistry { .. } => DesiredCanisterKind::Root,
            Self::PrepareStoreChunkSet { .. }
            | Self::PublishStoreChunk { .. }
            | Self::StageStoreManifest { .. } => DesiredCanisterKind::Store,
        }
    }

    /// Durable operation identity when this transition has a protocol-owned replay key.
    #[must_use]
    pub const fn operation_id(&self) -> Option<[u8; 32]> {
        match self {
            Self::ActivateRegistry { .. }
            | Self::JoinRoot { .. }
            | Self::PrepareComponentRegistry { .. }
            | Self::PrepareStoreChunkSet { .. }
            | Self::PublishStoreChunk { .. }
            | Self::StageStoreManifest { .. } => None,
            Self::ActivateRegistryMirror { request, .. }
            | Self::SynchronizeRegistry { request, .. } => Some(request.operation_id),
            Self::AdoptStore { request } => Some(request.operation_id),
            Self::BootstrapStore { request, .. } => Some(request.operation_id),
            Self::ProvisionComponents { request, .. } => Some(request.operation_id),
        }
    }
}

/// Explicit Wasm installation mode selected from live state.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
    Install,
    Reinstall,
}

/// Planned disposition and effects for one controlled canister.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPlan {
    pub actions: Vec<EnsureAction>,
    pub disposition: CanisterDisposition,
    pub name: String,
    #[serde(with = "u128_text")]
    pub observed_cycles: u128,
    pub principal: Option<String>,
}

/// Operator-visible canister disposition.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CanisterDisposition {
    Create,
    Delete,
    Replace,
    Reinstall,
    Reuse,
}

/// Exact cycle conservation proof reviewed before apply.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CycleConservation {
    #[serde(with = "u128_text")]
    pub expected_post_operation_cycles: u128,
    #[serde(with = "u128_text")]
    pub maximum_execution_burn_cycles: u128,
    #[serde(with = "u128_text")]
    pub maximum_new_funding_cycles: u128,
    #[serde(with = "u128_text")]
    pub maximum_operator_debit_cycles: u128,
    #[serde(with = "u128_text")]
    pub maximum_unavoidable_fee_cycles: u128,
    #[serde(with = "u128_text")]
    pub observed_controlled_cycles: u128,
    #[serde(with = "u128_text")]
    pub retained_in_reused_canisters_cycles: u128,
    #[serde(with = "u128_text")]
    pub scheduled_transfer_cycles: u128,
}

/// Terminal measured conservation result from the exact applied operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActualCycleConservation {
    #[serde(with = "u128_text")]
    pub exact_unavoidable_fee_cycles: u128,
    #[serde(with = "u128_text")]
    pub final_controlled_cycles: u128,
    #[serde(with = "u128_text")]
    pub measured_execution_burn_cycles: u128,
    #[serde(with = "u128_text")]
    pub observed_starting_cycles: u128,
    #[serde(with = "u128_text")]
    pub operator_debit_cycles: u128,
    #[serde(with = "u128_text")]
    pub received_new_funding_cycles: u128,
}

/// Whether the first live balance of a newly created canister is covered by
/// the exact observation-burn authority reviewed in the desired Fleet.
///
/// Creation response evidence owns the requested amount. The subsequent live
/// status owns the retained balance, which may be lower only by the explicitly
/// bounded observation burn incurred before that status is read.
pub(crate) const fn create_balance_is_terminal(
    actual_cycles: Option<u128>,
    requested_initial_cycles: u128,
    maximum_observation_burn_cycles: u128,
) -> bool {
    let Some(actual_cycles) = actual_cycles else {
        return false;
    };
    if actual_cycles > requested_initial_cycles {
        return false;
    }
    requested_initial_cycles - actual_cycles <= maximum_observation_burn_cycles
}

/// Exact normalized desired input retained by one in-progress operation.
///
/// The public desired document cannot declare internal protocol steps. This
/// wrapper persists those Canic-owned steps without making them an input
/// surface and restores them before any resumed observation or effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewedDesiredFleetRecord {
    desired: DesiredFleet,
}

impl ReviewedDesiredFleetRecord {
    pub(crate) fn capture(desired: &DesiredFleet) -> Self {
        Self {
            desired: desired.clone(),
        }
    }

    /// Borrow the complete normalized desired input, including internal steps.
    #[must_use]
    pub const fn desired(&self) -> &DesiredFleet {
        &self.desired
    }

    /// Consume the retained authority as the complete normalized desired input.
    #[must_use]
    pub fn into_desired(self) -> DesiredFleet {
        self.desired
    }
}

#[derive(Deserialize, Serialize)]
struct ReviewedDesiredFleetRecordProjection {
    desired: DesiredFleet,
    protocol_steps: Vec<DesiredProtocolStep>,
}

impl Serialize for ReviewedDesiredFleetRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ReviewedDesiredFleetRecordProjection {
            desired: self.desired.clone(),
            protocol_steps: self.desired.protocol_steps.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ReviewedDesiredFleetRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut projection = ReviewedDesiredFleetRecordProjection::deserialize(deserializer)?;
        projection.desired.protocol_steps = projection.protocol_steps;
        Ok(Self {
            desired: projection.desired,
        })
    }
}

/// Immutable reviewed plan produced from desired state plus one live observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetEnsurePlan {
    pub canisters: Vec<CanisterPlan>,
    pub conservation: CycleConservation,
    pub desired_sha256: String,
    pub environment: String,
    pub fleet: String,
    pub operation_id: String,
    pub plan_sha256: String,
    pub planned_at_time: u64,
    pub protocol_actions: Vec<EnsureAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_start_authority: Option<Box<RetainedRootStartAuthorityRecord>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_desired: Option<Box<ReviewedDesiredFleetRecord>>,
    pub schema_version: u16,
    #[serde(default, skip_serializing_if = "FleetEnsurePlanScope::is_full")]
    pub scope: FleetEnsurePlanScope,
    /// Exact terminal operation that owns the currently active Registry observation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_inventory_operation_id: Option<String>,
}

/// Exact authority scope of one current Fleet Ensure plan.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetEnsurePlanScope {
    /// Complete desired-state convergence after all protected roles are observable.
    #[default]
    Full,
    /// Same-identity start of exact management-verified retained Roots only.
    RootStartPrerequisite,
}

impl FleetEnsurePlanScope {
    /// Stable operator-facing name shared by text and JSON reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::RootStartPrerequisite => "root_start_prerequisite",
        }
    }

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "Serde skip_serializing_if requires a borrowed field predicate"
    )]
    const fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }
}

/// Exact management-canister observation of one configured Fleet Subnet Root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootManagementCanisterObservation {
    pub live: LiveCanister,
    pub name: String,
    pub subnet: String,
}

/// Read-only management evidence available before protected Root ingress can run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootManagementObservation {
    pub operator_cycles: u128,
    pub roots: BTreeMap<String, RootManagementCanisterObservation>,
}

/// One exact retained Root module accepted only for a same-identity Start prerequisite.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedRootStartBinding {
    pub controllers: Vec<String>,
    pub name: String,
    pub predecessor_module_sha256: String,
    pub principal: String,
    pub subnet: String,
}

/// Generator-owned authority for starting verified retained Roots before protected observation.
///
/// This record cannot authorize installation, replacement, funding, or any paid effect. The
/// reviewed prerequisite plan embeds it so later apply and replay do not depend on a mutable file.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedRootStartAuthorityRecord {
    pub authority_sha256: String,
    pub environment: String,
    pub fleet: String,
    pub fleet_id: canic_core::ids::FleetId,
    pub release_build_id: canic_core::ids::ReleaseBuildId,
    pub roots: Vec<RetainedRootStartBinding>,
    pub schema_version: u16,
    pub successor_module_sha256: String,
}

impl RetainedRootStartAuthorityRecord {
    pub(crate) fn seal(&mut self) {
        self.authority_sha256 = self.expected_sha256();
    }

    pub(crate) fn has_valid_digest(&self) -> bool {
        self.authority_sha256 == self.expected_sha256()
    }

    fn expected_sha256(&self) -> String {
        let mut canonical = self.clone();
        canonical.authority_sha256.clear();
        let bytes = serde_json::to_vec(&canonical)
            .expect("retained Root-start authority is JSON serializable");
        let mut framed = b"canic:fleet-ensure:root-start-authority:v1".to_vec();
        framed.extend_from_slice(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
        framed.extend_from_slice(&bytes);
        canic_core::cdk::utils::hash::sha256_hex(&framed)
    }
}

/// Durable state of one planned effect.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectState {
    Applied,
    Intent,
    Issued,
}

/// Durable outcome of one reviewed Fleet ensure operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetEnsureCompletion {
    Converged,
    InProgress,
    ReplanRequired,
}

/// Durable intent/result record for one action.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffectRecord {
    pub action_sha256: String,
    pub created_principal: Option<String>,
    #[serde(with = "option_u128_text")]
    pub destination_post_cycles: Option<u128>,
    #[serde(with = "option_u128_text")]
    pub destination_pre_cycles: Option<u128>,
    #[serde(with = "option_u128_text")]
    pub post_cycles: Option<u128>,
    #[serde(with = "option_u128_text")]
    pub pre_cycles: Option<u128>,
    #[serde(default)]
    pub pre_canister_version: Option<u64>,
    pub progress_identity: Option<String>,
    pub receipt: Option<String>,
    pub state: EffectState,
}

/// Complete retained/live balance tuple for one exact retirement transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetirementTransferBalances {
    pub destination_after: u128,
    pub destination_before: u128,
    pub maximum_execution_burn: u128,
    pub source_after: u128,
    pub source_before: u128,
    pub transfer_amount: u128,
}

/// Model-owned result of reconciling both controlled sides of a retirement transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetirementTransferReconciliation {
    Conserved {
        destination_credit: u128,
        source_debit: u128,
    },
    Pending,
}

/// Exact invariant failure while reconciling a retirement transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetirementTransferInvariantError {
    ArithmeticOverflow,
    BalanceMovedInInvalidDirection,
    Unbalanced {
        destination_credit: u128,
        maximum_source_debit: u128,
        source_debit: u128,
    },
}

/// Require a matching source debit and treasury credit before retirement may continue.
pub fn reconcile_retirement_transfer(
    balances: RetirementTransferBalances,
) -> Result<RetirementTransferReconciliation, RetirementTransferInvariantError> {
    let source_debit = balances
        .source_before
        .checked_sub(balances.source_after)
        .ok_or(RetirementTransferInvariantError::BalanceMovedInInvalidDirection)?;
    let destination_credit = balances
        .destination_after
        .checked_sub(balances.destination_before)
        .ok_or(RetirementTransferInvariantError::BalanceMovedInInvalidDirection)?;
    let maximum_source_debit = balances
        .transfer_amount
        .checked_add(balances.maximum_execution_burn)
        .ok_or(RetirementTransferInvariantError::ArithmeticOverflow)?;
    if source_debit == 0 && destination_credit == 0 {
        return Ok(RetirementTransferReconciliation::Pending);
    }
    if source_debit >= balances.transfer_amount
        && source_debit <= maximum_source_debit
        && destination_credit == balances.transfer_amount
    {
        return Ok(RetirementTransferReconciliation::Conserved {
            destination_credit,
            source_debit,
        });
    }
    Err(RetirementTransferInvariantError::Unbalanced {
        destination_credit,
        maximum_source_debit,
        source_debit,
    })
}

/// Current discovered identities retained only to seed live observation.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetEnsureStateRecord {
    pub active_registry: Option<canic_core::dto::fleet_registry::FleetRegistry>,
    /// Exact applied action identities for the adjacent completed reinstalls.
    #[serde(default)]
    pub completed_reinstall_action_sha256: BTreeMap<String, String>,
    /// Exact operation owning the adjacent completed-reinstall thresholds.
    #[serde(default)]
    pub completed_reinstall_operation_id: Option<String>,
    /// Pre-effect version thresholds for exact journal-proved reinstalls in the
    /// immediately preceding nonterminal operation.
    #[serde(default)]
    pub completed_reinstalls: BTreeMap<String, u64>,
    pub fleet: String,
    pub pending_principals: BTreeMap<String, String>,
    pub principals: BTreeMap<String, String>,
    #[serde(default)]
    pub retained_cycles_by_principal: BTreeMap<String, u128>,
    pub schema_version: u16,
    pub topology: BTreeMap<String, FleetEnsureTopologyRecord>,
}

/// Current typed topology retained independently from any historical install evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetEnsureTopologyRecord {
    pub kind: DesiredCanisterKind,
    pub module_hash: Option<String>,
    pub parent: Option<String>,
    pub protocol_binding: Option<crate::protocol_binding::RegistryProtocolBinding>,
    pub role: Option<String>,
}

/// Sole current-generation host journal for one Fleet ensure operation.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetEnsureJournalRecord {
    pub completion: FleetEnsureCompletion,
    pub effects: Vec<EffectRecord>,
    pub fleet: String,
    #[serde(with = "u128_text")]
    pub initial_controlled_cycles: u128,
    #[serde(with = "u128_text")]
    pub initial_operator_cycles: u128,
    pub operation_id: String,
    pub plan_sha256: String,
    pub schema_version: u16,
    pub stalled_observations: u32,
}

/// Result returned by one plan-only or apply invocation.

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FleetEnsureReport {
    pub actual_conservation: Option<ActualCycleConservation>,
    pub effects_applied: u32,
    pub plan: FleetEnsurePlan,
    pub terminal: bool,
}

const fn is_zero_u128(value: &u128) -> bool {
    *value == 0
}

mod u128_text {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

mod option_u128_text {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[expect(
        clippy::ref_option,
        reason = "serde(with) requires the serializer to accept the field by reference"
    )]
    pub fn serialize<S>(value: &Option<u128>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.map(|cycles| cycles.to_string()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u128>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|value| value.parse().map_err(serde::de::Error::custom))
            .transpose()
    }
}
