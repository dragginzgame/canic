//! Module: role_contract::policy
//!
//! Responsibility: derive role capabilities and resolve the pure role contract.
//! Does not own: package discovery, Cargo graph validation, descriptors, or rendering.
//! Boundary: callers provide typed direct features after host/build evidence validation.

use crate::{
    config::schema::{ConfigModel, RoleDeclarationKind},
    ids::CanisterRole,
    role_contract::{
        allocation::allocation_definition,
        catalog::{
            built_in_allocations, capability_allocations, capability_requirements,
            default_features, feature_allocations, implied_features, validate_catalog,
        },
        model::{
            BuiltInRoleKind, CanicFeatureKey, ResolvedRoleContract, ResolvedStateAllocation,
            RoleCapabilityKey, RoleContractFinding, RoleContractInput, RoleContractResolution,
            RoleContractSource, RoleFeatureRequirement, SelectionProvenance, StateAllocationKey,
        },
    },
};
use std::collections::{BTreeMap, BTreeSet};

#[must_use]
pub fn resolve_role_contract(input: RoleContractInput<'_>) -> RoleContractResolution {
    if let Err(error) = validate_catalog() {
        return RoleContractResolution::Rejected {
            errors: vec![error],
        };
    }

    let (role, built_in, capabilities) = match input.source {
        RoleContractSource::Declared { config, role } => {
            let capabilities = match derive_role_capabilities(config, role) {
                Ok(capabilities) => capabilities,
                Err(error) => {
                    return RoleContractResolution::Rejected {
                        errors: vec![error],
                    };
                }
            };
            (role.clone(), None, capabilities)
        }
        RoleContractSource::BuiltIn(kind) => (
            built_in_role(kind),
            Some(kind),
            built_in_role_capabilities(kind),
        ),
    };

    let effective_features =
        resolve_effective_features(input.declared_features, input.default_features_enabled);
    let requirements = requirements_for_capabilities(&capabilities);
    let missing = requirements
        .iter()
        .filter(|requirement| !effective_features.contains(&requirement.feature))
        .map(|requirement| RoleContractFinding::RequiredFeatureMissing {
            capability: requirement.capability,
            feature: requirement.feature,
        })
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return RoleContractResolution::Rejected { errors: missing };
    }

    let selections = collect_allocation_selections(&capabilities, &effective_features, built_in);
    let allocations = match materialize_allocations(selections) {
        Ok(allocations) => allocations,
        Err(error) => {
            return RoleContractResolution::Rejected {
                errors: vec![error],
            };
        }
    };

    RoleContractResolution::Resolved {
        contract: ResolvedRoleContract {
            role,
            built_in,
            capabilities,
            required_features: requirements
                .iter()
                .map(|requirement| requirement.feature)
                .collect(),
            effective_features,
            allocations,
        },
    }
}

pub fn required_features_for_role(
    config: &ConfigModel,
    role: &CanisterRole,
) -> Result<Vec<RoleFeatureRequirement>, RoleContractFinding> {
    derive_role_capabilities(config, role)
        .map(|capabilities| requirements_for_capabilities(&capabilities))
}

#[must_use]
pub fn built_in_role_capabilities(kind: BuiltInRoleKind) -> BTreeSet<RoleCapabilityKey> {
    let mut capabilities = BTreeSet::new();
    match kind {
        BuiltInRoleKind::WasmStore => {
            capabilities.insert(RoleCapabilityKey::Runtime);
            capabilities.insert(RoleCapabilityKey::WasmStore);
        }
    }
    capabilities
}

pub fn derive_role_capabilities(
    config: &ConfigModel,
    role: &CanisterRole,
) -> Result<BTreeSet<RoleCapabilityKey>, RoleContractFinding> {
    let Some(declaration) = config.roles.get(role) else {
        return Err(RoleContractFinding::RoleUnknown { role: role.clone() });
    };

    let mut capabilities = BTreeSet::from([RoleCapabilityKey::Runtime]);
    if declaration.kind == RoleDeclarationKind::Root {
        capabilities.insert(RoleCapabilityKey::Root);
        capabilities.insert(RoleCapabilityKey::RootControlPlane);
    }

    for subnet in config.subnets.values() {
        let Some(canister) = subnet.canisters.get(role) else {
            continue;
        };

        if canister.sharding.is_some() {
            capabilities.insert(RoleCapabilityKey::Sharding);
        }
        if canister.scaling.is_some() {
            capabilities.insert(RoleCapabilityKey::Scaling);
        }
        if canister.directory.is_some() {
            capabilities.insert(RoleCapabilityKey::Directory);
        }
        if canister
            .topup
            .as_ref()
            .and_then(|topup| topup.icp_refill.as_ref())
            .is_some_and(|icp_refill| icp_refill.enabled)
        {
            capabilities.insert(RoleCapabilityKey::IcpRefill);
        }
        if canister.standards.icrc21 {
            capabilities.insert(RoleCapabilityKey::Icrc21);
        }
        if canister.auth.role_attestation_cache {
            capabilities.insert(RoleCapabilityKey::RoleAttestationVerifier);
        }
        if canister.auth.delegated_token_issuer {
            capabilities.insert(RoleCapabilityKey::DelegatedTokenIssuer);
        }
        if canister.auth.delegated_token_verifier {
            capabilities.insert(RoleCapabilityKey::DelegatedTokenVerifier);
        }
    }

    if capabilities.contains(&RoleCapabilityKey::Root)
        && config.subnets.values().any(|subnet| {
            subnet
                .canisters
                .values()
                .any(|canister| canister.auth.role_attestation_cache)
        })
    {
        capabilities.insert(RoleCapabilityKey::RoleAttestationSigner);
    }

    Ok(capabilities)
}

#[must_use]
pub fn resolve_effective_features(
    mut declared_features: BTreeSet<CanicFeatureKey>,
    default_features_enabled: bool,
) -> BTreeSet<CanicFeatureKey> {
    if default_features_enabled {
        declared_features.extend(default_features());
    }

    let mut frontier = declared_features.iter().copied().collect::<Vec<_>>();
    while let Some(feature) = frontier.pop() {
        for implied in implied_features(feature) {
            if declared_features.insert(implied) {
                frontier.push(implied);
            }
        }
    }

    declared_features
}

fn requirements_for_capabilities(
    capabilities: &BTreeSet<RoleCapabilityKey>,
) -> Vec<RoleFeatureRequirement> {
    let mut requirements = BTreeMap::new();
    for capability in capabilities {
        for rule in capability_requirements(*capability) {
            requirements
                .entry(rule.feature)
                .or_insert(RoleFeatureRequirement {
                    capability: rule.capability,
                    config_key: rule.config_key,
                    feature: rule.feature,
                    reason: rule.reason,
                });
        }
    }
    requirements.into_values().collect()
}

fn collect_allocation_selections(
    capabilities: &BTreeSet<RoleCapabilityKey>,
    effective_features: &BTreeSet<CanicFeatureKey>,
    built_in: Option<BuiltInRoleKind>,
) -> BTreeMap<StateAllocationKey, BTreeSet<SelectionProvenance>> {
    let mut selections = BTreeMap::<StateAllocationKey, BTreeSet<SelectionProvenance>>::new();

    for capability in capabilities {
        for allocation in capability_allocations(*capability) {
            selections
                .entry(allocation)
                .or_default()
                .insert(SelectionProvenance::Capability(*capability));
        }
    }
    for feature in effective_features {
        for allocation in feature_allocations(*feature) {
            selections
                .entry(allocation)
                .or_default()
                .insert(SelectionProvenance::EffectiveFeature(*feature));
        }
    }
    if let Some(role) = built_in {
        for allocation in built_in_allocations(role) {
            selections
                .entry(allocation)
                .or_default()
                .insert(SelectionProvenance::BuiltInRole(role));
        }
    }

    selections
}

fn materialize_allocations(
    selections: BTreeMap<StateAllocationKey, BTreeSet<SelectionProvenance>>,
) -> Result<Vec<ResolvedStateAllocation>, RoleContractFinding> {
    let mut memory_owners = BTreeMap::new();
    let mut allocations = Vec::with_capacity(selections.len());

    for (key, selected_by) in selections {
        let Some(definition) = allocation_definition(key) else {
            return Err(RoleContractFinding::CatalogInvalid {
                reason: format!("selected allocation has no definition: {key:?}"),
            });
        };
        for memory_id in definition.memory_ids {
            if let Some(first) = memory_owners.insert(*memory_id, key) {
                return Err(RoleContractFinding::MemoryIdCollision {
                    memory_id: *memory_id,
                    first,
                    second: key,
                });
            }
        }
        allocations.push(ResolvedStateAllocation {
            key,
            owner: definition.owner,
            memory_ids: definition.memory_ids.to_vec(),
            selected_by,
        });
    }

    Ok(allocations)
}

const fn built_in_role(kind: BuiltInRoleKind) -> CanisterRole {
    match kind {
        BuiltInRoleKind::WasmStore => CanisterRole::WASM_STORE,
    }
}
