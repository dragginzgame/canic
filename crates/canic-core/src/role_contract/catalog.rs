//! Module: role_contract::catalog
//!
//! Responsibility: own typed Canic feature, capability, and allocation policy tables.
//! Does not own: Cargo parsing, stable records, state descriptors, or report labels.
//! Boundary: pure role policy consumes these tables; Cargo parity tests mirror them.

use crate::role_contract::{
    allocation::{allocation_definition, validate_canonical_allocations},
    model::{
        BuiltInRoleKind, CanicFeatureEffect, CanicFeatureKey, RoleCapabilityKey,
        RoleContractFinding, StateAllocationKey,
    },
};
use std::collections::BTreeSet;

///
/// FeatureDefinition
///
/// Canonical public Canic feature name and state effect.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureDefinition {
    pub key: CanicFeatureKey,
    pub cargo_name: &'static str,
    pub effect: CanicFeatureEffect,
}

///
/// CapabilityRequirement
///
/// Public feature required by one derived role capability.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityRequirement {
    pub capability: RoleCapabilityKey,
    pub config_key: &'static str,
    pub feature: CanicFeatureKey,
    pub reason: &'static str,
}

#[derive(Clone, Copy)]
struct FeatureImplication {
    from: CanicFeatureKey,
    to: CanicFeatureKey,
}

#[derive(Clone, Copy)]
struct CapabilityAllocation {
    capability: RoleCapabilityKey,
    allocation: StateAllocationKey,
}

#[derive(Clone, Copy)]
struct FeatureAllocation {
    feature: CanicFeatureKey,
    allocation: StateAllocationKey,
}

#[derive(Clone, Copy)]
struct BuiltInAllocation {
    role: BuiltInRoleKind,
    allocation: StateAllocationKey,
}

const FEATURE_DEFINITIONS: &[FeatureDefinition] = &[
    feature(
        CanicFeatureKey::AuthChainKeyEcdsa,
        "auth-chain-key-ecdsa",
        CanicFeatureEffect::NoState,
    ),
    feature(
        CanicFeatureKey::AuthChainKeyRootSign,
        "auth-chain-key-root-sign",
        CanicFeatureEffect::NoState,
    ),
    feature(
        CanicFeatureKey::AuthDelegatedTokenVerify,
        "auth-delegated-token-verify",
        CanicFeatureEffect::NoState,
    ),
    feature(
        CanicFeatureKey::AuthIssuerCanisterSigCreate,
        "auth-issuer-canister-sig-create",
        CanicFeatureEffect::NoState,
    ),
    feature(
        CanicFeatureKey::AuthIssuerCanisterSigVerify,
        "auth-issuer-canister-sig-verify",
        CanicFeatureEffect::NoState,
    ),
    feature(
        CanicFeatureKey::AuthRootCanisterSigCreate,
        "auth-root-canister-sig-create",
        CanicFeatureEffect::NoState,
    ),
    feature(
        CanicFeatureKey::AuthRootCanisterSigVerify,
        "auth-root-canister-sig-verify",
        CanicFeatureEffect::NoState,
    ),
    feature(
        CanicFeatureKey::BlobStorage,
        "blob-storage",
        CanicFeatureEffect::StateBearing,
    ),
    feature(
        CanicFeatureKey::BlobStorageBilling,
        "blob-storage-billing",
        CanicFeatureEffect::StateBearing,
    ),
    feature(
        CanicFeatureKey::ControlPlane,
        "control-plane",
        CanicFeatureEffect::StateBearing,
    ),
    feature(
        CanicFeatureKey::Metrics,
        "metrics",
        CanicFeatureEffect::NoState,
    ),
    feature(
        CanicFeatureKey::Sharding,
        "sharding",
        CanicFeatureEffect::StateBearing,
    ),
    feature(
        CanicFeatureKey::WasmStoreCanister,
        "wasm-store-canister",
        CanicFeatureEffect::StateBearing,
    ),
];

const DEFAULT_FEATURES: &[CanicFeatureKey] = &[CanicFeatureKey::Metrics];

const FEATURE_IMPLICATIONS: &[FeatureImplication] = &[
    FeatureImplication {
        from: CanicFeatureKey::AuthChainKeyRootSign,
        to: CanicFeatureKey::AuthChainKeyEcdsa,
    },
    FeatureImplication {
        from: CanicFeatureKey::AuthDelegatedTokenVerify,
        to: CanicFeatureKey::AuthChainKeyEcdsa,
    },
    FeatureImplication {
        from: CanicFeatureKey::AuthDelegatedTokenVerify,
        to: CanicFeatureKey::AuthIssuerCanisterSigVerify,
    },
    FeatureImplication {
        from: CanicFeatureKey::BlobStorageBilling,
        to: CanicFeatureKey::BlobStorage,
    },
];

const CAPABILITY_REQUIREMENTS: &[CapabilityRequirement] = &[
    requirement(
        RoleCapabilityKey::DelegatedTokenIssuer,
        "auth.delegated_token_issuer",
        CanicFeatureKey::AuthIssuerCanisterSigCreate,
        "delegated-token issuers create issuer canister-signature proofs",
    ),
    requirement(
        RoleCapabilityKey::DelegatedTokenIssuer,
        "auth.delegated_token_issuer",
        CanicFeatureKey::AuthDelegatedTokenVerify,
        "delegated-token issuers verify delegated-token root proof material",
    ),
    requirement(
        RoleCapabilityKey::DelegatedTokenVerifier,
        "auth.delegated_token_verifier",
        CanicFeatureKey::AuthDelegatedTokenVerify,
        "delegated-token verifiers verify delegated-token root proof material",
    ),
    requirement(
        RoleCapabilityKey::RoleAttestationSigner,
        "auth.role_attestation_cache",
        CanicFeatureKey::AuthRootCanisterSigCreate,
        "root signs role-attestation canister-signature proofs for cache users",
    ),
    requirement(
        RoleCapabilityKey::RoleAttestationVerifier,
        "auth.role_attestation_cache",
        CanicFeatureKey::AuthRootCanisterSigVerify,
        "role-attestation caches verify root canister-signature proofs locally",
    ),
    requirement(
        RoleCapabilityKey::RootControlPlane,
        "roles.root.kind",
        CanicFeatureKey::ControlPlane,
        "root roles compile the Canic control plane",
    ),
    requirement(
        RoleCapabilityKey::Sharding,
        "sharding",
        CanicFeatureKey::Sharding,
        "sharding roles compile sharding state and policy",
    ),
    requirement(
        RoleCapabilityKey::WasmStore,
        "built_in.wasm_store",
        CanicFeatureKey::WasmStoreCanister,
        "the built-in wasm_store compiles template storage and GC state",
    ),
];

const CAPABILITY_ALLOCATIONS: &[CapabilityAllocation] = &[
    capability_allocation(
        RoleCapabilityKey::Runtime,
        StateAllocationKey::CoreRuntimeTopology,
    ),
    capability_allocation(
        RoleCapabilityKey::Runtime,
        StateAllocationKey::CoreRuntimeEnvironment,
    ),
    capability_allocation(
        RoleCapabilityKey::Runtime,
        StateAllocationKey::CoreReplayReceipts,
    ),
    capability_allocation(
        RoleCapabilityKey::Runtime,
        StateAllocationKey::CoreRuntimeObservability,
    ),
    capability_allocation(
        RoleCapabilityKey::Runtime,
        StateAllocationKey::CoreRuntimeIntent,
    ),
    capability_allocation(
        RoleCapabilityKey::Root,
        StateAllocationKey::CoreRootAppRegistry,
    ),
    capability_allocation(RoleCapabilityKey::Root, StateAllocationKey::CoreAuthState),
    capability_allocation(
        RoleCapabilityKey::DelegatedTokenIssuer,
        StateAllocationKey::CoreAuthState,
    ),
    capability_allocation(
        RoleCapabilityKey::DelegatedTokenVerifier,
        StateAllocationKey::CoreAuthState,
    ),
    capability_allocation(
        RoleCapabilityKey::RoleAttestationSigner,
        StateAllocationKey::CoreAuthState,
    ),
    capability_allocation(
        RoleCapabilityKey::RoleAttestationVerifier,
        StateAllocationKey::CoreAuthState,
    ),
    capability_allocation(
        RoleCapabilityKey::Root,
        StateAllocationKey::CoreIcpRefillRecords,
    ),
    capability_allocation(RoleCapabilityKey::Root, StateAllocationKey::CanisterPool),
    capability_allocation(
        RoleCapabilityKey::Directory,
        StateAllocationKey::CanisterPool,
    ),
    capability_allocation(
        RoleCapabilityKey::Directory,
        StateAllocationKey::DirectoryRegistry,
    ),
    capability_allocation(RoleCapabilityKey::Scaling, StateAllocationKey::CanisterPool),
    capability_allocation(
        RoleCapabilityKey::Scaling,
        StateAllocationKey::ScalingRegistry,
    ),
    capability_allocation(
        RoleCapabilityKey::Sharding,
        StateAllocationKey::CanisterPool,
    ),
    capability_allocation(
        RoleCapabilityKey::RootControlPlane,
        StateAllocationKey::TemplateManifests,
    ),
    capability_allocation(
        RoleCapabilityKey::RootControlPlane,
        StateAllocationKey::TemplateChunkSets,
    ),
    capability_allocation(
        RoleCapabilityKey::RootControlPlane,
        StateAllocationKey::TemplateChunkRefs,
    ),
    capability_allocation(
        RoleCapabilityKey::RootControlPlane,
        StateAllocationKey::TemplateChunkPayloads,
    ),
    capability_allocation(
        RoleCapabilityKey::RootControlPlane,
        StateAllocationKey::ControlPlaneSubnetState,
    ),
    capability_allocation(
        RoleCapabilityKey::WasmStore,
        StateAllocationKey::TemplateManifests,
    ),
    capability_allocation(
        RoleCapabilityKey::WasmStore,
        StateAllocationKey::TemplateChunkSets,
    ),
    capability_allocation(
        RoleCapabilityKey::WasmStore,
        StateAllocationKey::TemplateChunkRefs,
    ),
    capability_allocation(
        RoleCapabilityKey::WasmStore,
        StateAllocationKey::TemplateChunkPayloads,
    ),
    capability_allocation(
        RoleCapabilityKey::WasmStore,
        StateAllocationKey::WasmStoreGcState,
    ),
];

const FEATURE_ALLOCATIONS: &[FeatureAllocation] = &[
    feature_allocation(
        CanicFeatureKey::BlobStorage,
        StateAllocationKey::StoredBlobs,
    ),
    feature_allocation(
        CanicFeatureKey::BlobStorage,
        StateAllocationKey::BlobDeletionPending,
    ),
    feature_allocation(
        CanicFeatureKey::BlobStorage,
        StateAllocationKey::StorageGatewayPrincipals,
    ),
    feature_allocation(
        CanicFeatureKey::BlobStorageBilling,
        StateAllocationKey::BlobStorageBilling,
    ),
    feature_allocation(
        CanicFeatureKey::ControlPlane,
        StateAllocationKey::TemplateManifests,
    ),
    feature_allocation(
        CanicFeatureKey::ControlPlane,
        StateAllocationKey::TemplateChunkSets,
    ),
    feature_allocation(
        CanicFeatureKey::ControlPlane,
        StateAllocationKey::TemplateChunkRefs,
    ),
    feature_allocation(
        CanicFeatureKey::ControlPlane,
        StateAllocationKey::TemplateChunkPayloads,
    ),
    feature_allocation(
        CanicFeatureKey::ControlPlane,
        StateAllocationKey::ControlPlaneSubnetState,
    ),
    feature_allocation(CanicFeatureKey::Sharding, StateAllocationKey::CanisterPool),
    feature_allocation(
        CanicFeatureKey::Sharding,
        StateAllocationKey::ShardingRegistry,
    ),
    feature_allocation(
        CanicFeatureKey::Sharding,
        StateAllocationKey::ShardingAssignments,
    ),
    feature_allocation(
        CanicFeatureKey::Sharding,
        StateAllocationKey::ShardingActiveSet,
    ),
    feature_allocation(
        CanicFeatureKey::WasmStoreCanister,
        StateAllocationKey::TemplateManifests,
    ),
    feature_allocation(
        CanicFeatureKey::WasmStoreCanister,
        StateAllocationKey::TemplateChunkSets,
    ),
    feature_allocation(
        CanicFeatureKey::WasmStoreCanister,
        StateAllocationKey::TemplateChunkRefs,
    ),
    feature_allocation(
        CanicFeatureKey::WasmStoreCanister,
        StateAllocationKey::TemplateChunkPayloads,
    ),
    feature_allocation(
        CanicFeatureKey::WasmStoreCanister,
        StateAllocationKey::WasmStoreGcState,
    ),
];

const BUILT_IN_ALLOCATIONS: &[BuiltInAllocation] = &[
    built_in_allocation(
        BuiltInRoleKind::WasmStore,
        StateAllocationKey::TemplateManifests,
    ),
    built_in_allocation(
        BuiltInRoleKind::WasmStore,
        StateAllocationKey::TemplateChunkSets,
    ),
    built_in_allocation(
        BuiltInRoleKind::WasmStore,
        StateAllocationKey::TemplateChunkRefs,
    ),
    built_in_allocation(
        BuiltInRoleKind::WasmStore,
        StateAllocationKey::TemplateChunkPayloads,
    ),
    built_in_allocation(
        BuiltInRoleKind::WasmStore,
        StateAllocationKey::WasmStoreGcState,
    ),
];

const fn feature(
    key: CanicFeatureKey,
    cargo_name: &'static str,
    effect: CanicFeatureEffect,
) -> FeatureDefinition {
    FeatureDefinition {
        key,
        cargo_name,
        effect,
    }
}

const fn requirement(
    capability: RoleCapabilityKey,
    config_key: &'static str,
    feature: CanicFeatureKey,
    reason: &'static str,
) -> CapabilityRequirement {
    CapabilityRequirement {
        capability,
        config_key,
        feature,
        reason,
    }
}

const fn capability_allocation(
    capability: RoleCapabilityKey,
    allocation: StateAllocationKey,
) -> CapabilityAllocation {
    CapabilityAllocation {
        capability,
        allocation,
    }
}

const fn feature_allocation(
    feature: CanicFeatureKey,
    allocation: StateAllocationKey,
) -> FeatureAllocation {
    FeatureAllocation {
        feature,
        allocation,
    }
}

const fn built_in_allocation(
    role: BuiltInRoleKind,
    allocation: StateAllocationKey,
) -> BuiltInAllocation {
    BuiltInAllocation { role, allocation }
}

impl CanicFeatureKey {
    pub const ALL: &'static [Self] = &[
        Self::AuthChainKeyEcdsa,
        Self::AuthChainKeyRootSign,
        Self::AuthDelegatedTokenVerify,
        Self::AuthIssuerCanisterSigCreate,
        Self::AuthIssuerCanisterSigVerify,
        Self::AuthRootCanisterSigCreate,
        Self::AuthRootCanisterSigVerify,
        Self::BlobStorage,
        Self::BlobStorageBilling,
        Self::ControlPlane,
        Self::Metrics,
        Self::Sharding,
        Self::WasmStoreCanister,
    ];

    #[must_use]
    pub fn cargo_name(self) -> &'static str {
        feature_definition(self).cargo_name
    }

    #[must_use]
    pub fn effect(self) -> CanicFeatureEffect {
        feature_definition(self).effect
    }

    #[must_use]
    pub fn from_cargo_name(name: &str) -> Option<Self> {
        FEATURE_DEFINITIONS
            .iter()
            .find(|definition| definition.cargo_name == name)
            .map(|definition| definition.key)
    }
}

#[must_use]
pub const fn feature_definitions() -> &'static [FeatureDefinition] {
    FEATURE_DEFINITIONS
}

#[must_use]
pub const fn default_features() -> &'static [CanicFeatureKey] {
    DEFAULT_FEATURES
}

pub fn implied_features(feature: CanicFeatureKey) -> impl Iterator<Item = CanicFeatureKey> {
    FEATURE_IMPLICATIONS
        .iter()
        .filter(move |implication| implication.from == feature)
        .map(|implication| implication.to)
}

pub fn capability_requirements(
    capability: RoleCapabilityKey,
) -> impl Iterator<Item = &'static CapabilityRequirement> {
    CAPABILITY_REQUIREMENTS
        .iter()
        .filter(move |requirement| requirement.capability == capability)
}

pub fn capability_allocations(
    capability: RoleCapabilityKey,
) -> impl Iterator<Item = StateAllocationKey> {
    CAPABILITY_ALLOCATIONS
        .iter()
        .filter(move |rule| rule.capability == capability)
        .map(|rule| rule.allocation)
}

pub fn feature_allocations(feature: CanicFeatureKey) -> impl Iterator<Item = StateAllocationKey> {
    FEATURE_ALLOCATIONS
        .iter()
        .filter(move |rule| rule.feature == feature)
        .map(|rule| rule.allocation)
}

pub fn built_in_allocations(role: BuiltInRoleKind) -> impl Iterator<Item = StateAllocationKey> {
    BUILT_IN_ALLOCATIONS
        .iter()
        .filter(move |rule| rule.role == role)
        .map(|rule| rule.allocation)
}

pub fn validate_catalog() -> Result<(), RoleContractFinding> {
    validate_canonical_allocations()?;

    let keys = FEATURE_DEFINITIONS
        .iter()
        .map(|definition| definition.key)
        .collect::<BTreeSet<_>>();
    if keys.len() != FEATURE_DEFINITIONS.len() || keys.len() != CanicFeatureKey::ALL.len() {
        return Err(RoleContractFinding::CatalogInvalid {
            reason: "public Canic features are not defined exactly once".to_string(),
        });
    }

    for feature in CanicFeatureKey::ALL {
        let allocation_count = feature_allocations(*feature).count();
        match feature.effect() {
            CanicFeatureEffect::NoState if allocation_count != 0 => {
                return Err(RoleContractFinding::CatalogInvalid {
                    reason: format!("no-state feature {} assigns state", feature.cargo_name()),
                });
            }
            CanicFeatureEffect::StateBearing if allocation_count == 0 => {
                return Err(RoleContractFinding::CatalogInvalid {
                    reason: format!(
                        "state-bearing feature {} has no allocation",
                        feature.cargo_name()
                    ),
                });
            }
            CanicFeatureEffect::NoState | CanicFeatureEffect::StateBearing => {}
        }
    }

    for allocation in CAPABILITY_ALLOCATIONS
        .iter()
        .map(|rule| rule.allocation)
        .chain(FEATURE_ALLOCATIONS.iter().map(|rule| rule.allocation))
        .chain(BUILT_IN_ALLOCATIONS.iter().map(|rule| rule.allocation))
    {
        if allocation_definition(allocation).is_none() {
            return Err(RoleContractFinding::CatalogInvalid {
                reason: format!("allocation rule references missing definition: {allocation:?}"),
            });
        }
    }

    for start in CanicFeatureKey::ALL {
        let mut frontier = vec![*start];
        let mut visited = BTreeSet::new();
        while let Some(feature) = frontier.pop() {
            for implied in implied_features(feature) {
                if implied == *start {
                    return Err(RoleContractFinding::CatalogInvalid {
                        reason: format!(
                            "public feature implication cycle reaches {}",
                            start.cargo_name()
                        ),
                    });
                }
                if visited.insert(implied) {
                    frontier.push(implied);
                }
            }
        }
    }

    Ok(())
}

fn feature_definition(feature: CanicFeatureKey) -> &'static FeatureDefinition {
    FEATURE_DEFINITIONS
        .iter()
        .find(|definition| definition.key == feature)
        .expect("every CanicFeatureKey must have one static definition")
}
