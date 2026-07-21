//! Module: role_contract::model
//!
//! Responsibility: define passive typed role-contract inputs, outputs, and findings.
//! Does not own: Cargo evidence collection, state descriptors, or report rendering.
//! Boundary: host/build consumers provide typed feature evidence to pure core policy.

use crate::{config::schema::ConfigModel, ids::CanisterRole};
use std::collections::BTreeSet;

///
/// CanicFeatureKey
///
/// Maintained public feature of the `canic` facade crate.
///

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CanicFeatureKey {
    AuthChainKeyEcdsa,
    AuthChainKeyRootSign,
    AuthDelegatedTokenVerify,
    AuthIssuerCanisterSigCreate,
    AuthIssuerCanisterSigVerify,
    AuthRootCanisterSigCreate,
    AuthRootCanisterSigVerify,
    BlobStorage,
    BlobStorageBilling,
    ControlPlane,
    IcpRefill,
    Metrics,
    Sharding,
    WasmStoreCanister,
}

///
/// CanicFeatureEffect
///
/// Whether enabling one public Canic feature selects stable state.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanicFeatureEffect {
    NoState,
    StateBearing,
}

///
/// RoleCapabilityKey
///
/// Typed behavior derived from validated role configuration or built-in identity.
///

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RoleCapabilityKey {
    DelegatedTokenIssuer,
    DelegatedTokenVerifier,
    Directory,
    IcpRefill,
    Icrc21,
    RoleAttestationSigner,
    RoleAttestationVerifier,
    Root,
    RootControlPlane,
    Runtime,
    Scaling,
    Sharding,
    WasmStore,
}

///
/// StateAllocationKey
///
/// Typed identity of one active Canic-managed stable-memory allocation group.
///

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StateAllocationKey {
    BlobDeletionPending,
    BlobStorageBilling,
    CanisterPool,
    ControlPlaneSubnetState,
    CoreAuthState,
    CoreIcpRefillRecords,
    CoreReplayReceipts,
    CoreRootAppRegistry,
    CoreRuntimeEnvironment,
    CoreRuntimeIntent,
    CoreRuntimeObservability,
    CoreRuntimeTopology,
    DirectoryRegistry,
    ScalingRegistry,
    ShardingActiveSet,
    ShardingAssignments,
    ShardingRegistry,
    StorageGatewayPrincipals,
    StoredBlobs,
    TemplateChunkPayloads,
    TemplateChunkRefs,
    TemplateChunkSets,
    TemplateManifests,
    WasmStoreGcState,
}

///
/// AllocationOwner
///
/// Crate that owns the records and storage implementation for an allocation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationOwner {
    CanicControlPlane,
    CanicCore,
}

impl AllocationOwner {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanicControlPlane => "canic-control-plane",
            Self::CanicCore => "canic-core",
        }
    }
}

///
/// MemoryId
///
/// Typed stable-memory manager ID owned by the allocation registry.
///

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MemoryId(u8);

impl MemoryId {
    #[must_use]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

///
/// AllocationDefinition
///
/// Canonical assignment of one or more stable-memory IDs to an active allocation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocationDefinition {
    pub key: StateAllocationKey,
    pub owner: AllocationOwner,
    pub memory_ids: &'static [MemoryId],
}

///
/// BuiltInRoleKind
///
/// Canic-managed role that is not resolved from a fleet role declaration.
///

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BuiltInRoleKind {
    WasmStore,
}

///
/// RoleContractSource
///
/// Validated configuration source used by pure role-contract resolution.
///

pub enum RoleContractSource<'a> {
    BuiltIn(BuiltInRoleKind),
    Declared {
        config: &'a ConfigModel,
        role: &'a CanisterRole,
    },
}

///
/// RoleContractInput
///
/// Typed feature evidence for one pure role-contract resolution.
///

pub struct RoleContractInput<'a> {
    pub source: RoleContractSource<'a>,
    pub declared_features: BTreeSet<CanicFeatureKey>,
    pub default_features_enabled: bool,
}

///
/// RoleFeatureRequirement
///
/// Public Canic feature required by one derived role capability.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleFeatureRequirement {
    pub capability: RoleCapabilityKey,
    pub config_key: &'static str,
    pub feature: CanicFeatureKey,
    pub reason: &'static str,
}

///
/// SelectionProvenance
///
/// Typed reason one active allocation was selected for a role contract.
///

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SelectionProvenance {
    BuiltInRole(BuiltInRoleKind),
    Capability(RoleCapabilityKey),
    EffectiveFeature(CanicFeatureKey),
}

///
/// ResolvedStateAllocation
///
/// Active allocation selected for one resolved role contract.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedStateAllocation {
    pub key: StateAllocationKey,
    pub owner: AllocationOwner,
    pub memory_ids: Vec<MemoryId>,
    pub selected_by: BTreeSet<SelectionProvenance>,
}

///
/// ResolvedRoleContract
///
/// Consumable role contract returned only when all blocking checks pass.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedRoleContract {
    pub role: CanisterRole,
    pub built_in: Option<BuiltInRoleKind>,
    pub capabilities: BTreeSet<RoleCapabilityKey>,
    pub required_features: BTreeSet<CanicFeatureKey>,
    pub effective_features: BTreeSet<CanicFeatureKey>,
    pub allocations: Vec<ResolvedStateAllocation>,
}

///
/// RoleContractFinding
///
/// Blocking failure produced by pure role-contract validation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoleContractFinding {
    AllocationDescriptorDuplicate {
        key: StateAllocationKey,
    },
    AllocationDescriptorIdMismatch {
        key: StateAllocationKey,
        expected: Vec<MemoryId>,
        actual: Vec<MemoryId>,
    },
    AllocationDescriptorMissing {
        key: StateAllocationKey,
    },
    CatalogInvalid {
        reason: String,
    },
    BuiltInPackageUnavailable {
        role: BuiltInRoleKind,
    },
    CanicVersionMismatch {
        expected: String,
        actual: String,
    },
    CargoCatalogDrift {
        reason: String,
    },
    DependencyShapeUnsupported {
        reason: String,
    },
    MemoryIdCollision {
        memory_id: MemoryId,
        first: StateAllocationKey,
        second: StateAllocationKey,
    },
    MultipleCanicPackages {
        packages: Vec<String>,
    },
    PackageAmbiguous {
        role: CanisterRole,
    },
    PackageMetadataMismatch {
        expected_fleet: String,
        expected_role: CanisterRole,
        actual_fleet: Option<String>,
        actual_role: Option<String>,
    },
    PackageMissing {
        role: CanisterRole,
    },
    RequiredFeatureMissing {
        capability: RoleCapabilityKey,
        feature: CanicFeatureKey,
    },
    RoleUnknown {
        role: CanisterRole,
    },
    RuntimeCanicDependencyMissing {
        role: CanisterRole,
    },
}

impl RoleContractFinding {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::AllocationDescriptorDuplicate { .. } => {
                "role_contract_allocation_descriptor_duplicate"
            }
            Self::AllocationDescriptorIdMismatch { .. } => "role_contract_allocation_id_mismatch",
            Self::AllocationDescriptorMissing { .. } => {
                "role_contract_allocation_descriptor_missing"
            }
            Self::BuiltInPackageUnavailable { .. } => "role_contract_builtin_package_unavailable",
            Self::CanicVersionMismatch { .. } => "role_contract_canic_version_mismatch",
            Self::CargoCatalogDrift { .. } => "role_contract_cargo_catalog_drift",
            Self::CatalogInvalid { .. } => "role_contract_catalog_invalid",
            Self::DependencyShapeUnsupported { .. } => "role_contract_dependency_shape_unsupported",
            Self::MemoryIdCollision { .. } => "role_contract_memory_id_collision",
            Self::MultipleCanicPackages { .. } => "role_contract_multiple_canic_packages",
            Self::PackageAmbiguous { .. } => "role_contract_package_ambiguous",
            Self::PackageMetadataMismatch { .. } => "role_contract_package_metadata_mismatch",
            Self::PackageMissing { .. } => "role_contract_package_missing",
            Self::RequiredFeatureMissing { .. } => "role_contract_required_feature_missing",
            Self::RoleUnknown { .. } => "role_contract_role_unknown",
            Self::RuntimeCanicDependencyMissing { .. } => {
                "role_contract_runtime_canic_dependency_missing"
            }
        }
    }
}

///
/// RoleContractResolution
///
/// Resolved contract or blocking findings; rejected results never carry a contract.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoleContractResolution {
    Rejected { errors: Vec<RoleContractFinding> },
    Resolved { contract: ResolvedRoleContract },
}
