//! Module: config::topology
//!
//! Responsibility: compile validated App configuration into canonical Component Topology.
//! Does not own: physical placement, root admission selection, Registry state, or lifecycle.
//! Boundary: emits bounded non-recursive topology and stable domain-separated SHA-256 identities.

#[cfg(test)]
mod tests;
mod validation;

use crate::{
    cdk::types::Cycles,
    config::schema::{
        BindingConfig, CanisterAuthConfig, ComponentChildConfig, ComponentChildKind,
        ComponentSpecConfig, ConfigModel, CyclesFundingPolicyConfig, DiagnosticsCanisterConfig,
        MetricsCanisterConfig, MetricsProfile, ScalePoolPolicy, ScalingConfig, ShardPoolPolicy,
        ShardingConfig, StandardsCanisterConfig, TopupPolicy,
    },
    ids::{
        CanisterRole, ComponentSpecAdmission, ComponentSpecId, ComponentTopologyDigest,
        CyclesFundingBudget,
    },
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error as ThisError;

const COMPONENT_SPEC_HASH_DOMAIN: &[u8] = b"canic/component-spec/v1";
const COMPONENT_TOPOLOGY_HASH_DOMAIN: &[u8] = b"canic/component-topology/v1";
const COMPONENT_TOPOLOGY_SCHEMA_VERSION: u32 = 1;

/// Maximum canonical bytes accepted for one Fleet-wide Component Topology.
pub const MAX_COMPONENT_TOPOLOGY_CANONICAL_BYTES: usize = 2_097_152;

impl ConfigModel {
    /// Compile this validated declaration model into its bounded canonical topology.
    pub fn compile_component_topology(&self) -> Result<ComponentTopology, ComponentTopologyError> {
        ComponentTopology::compile(self)
    }
}

///
/// ComponentTopology
///
/// Canonically ordered, non-recursive Fleet Component admission graph.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentTopology {
    pub component_specs: Vec<ComponentSpec>,
}

impl ComponentTopology {
    /// Compile one validated config into canonical Component Spec order.
    pub fn compile(config: &ConfigModel) -> Result<Self, ComponentTopologyError> {
        let mut component_specs = Vec::with_capacity(config.component_specs.len());

        for (component_spec, source) in &config.component_specs {
            component_specs.push(compile_component_spec(config, component_spec, source)?);
        }

        let topology = Self { component_specs };
        topology.canonical_bytes()?;
        Ok(topology)
    }

    /// Return the exact compiled Spec for one declaration ID.
    #[must_use]
    pub fn get(&self, component_spec: &ComponentSpecId) -> Option<&ComponentSpec> {
        self.component_specs
            .binary_search_by(|candidate| candidate.component_spec.cmp(component_spec))
            .ok()
            .map(|index| &self.component_specs[index])
    }

    /// Encode the complete topology with its frozen canonical schema.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ComponentTopologyError> {
        let mut encoder = CanonicalEncoder::new(COMPONENT_TOPOLOGY_HASH_DOMAIN);
        encoder.u64(self.component_specs.len() as u64);

        for component_spec in &self.component_specs {
            encode_compiled_component_spec(&mut encoder, component_spec);
        }

        encoder.finish("Component Topology")
    }

    /// Compute the digest of this exact canonical topology projection.
    pub fn digest(&self) -> Result<ComponentTopologyDigest, ComponentTopologyError> {
        let bytes = self.canonical_bytes()?;
        Ok(ComponentTopologyDigest::from_bytes(
            Sha256::digest(bytes).into(),
        ))
    }

    /// Validate canonical root admissions and project their exact Spec closure.
    pub fn project_for_admissions(
        &self,
        admissions: &[ComponentSpecAdmission],
    ) -> Result<Self, ComponentTopologyError> {
        if admissions.is_empty() {
            return Err(ComponentTopologyError::EmptyRootAdmissions);
        }

        let mut previous: Option<&ComponentSpecId> = None;
        let mut component_specs = Vec::with_capacity(admissions.len());

        for admission in admissions {
            if let Some(previous) = previous
                && previous >= &admission.component_spec
            {
                return Err(ComponentTopologyError::NonCanonicalAdmissionOrder {
                    previous: previous.clone(),
                    current: admission.component_spec.clone(),
                });
            }
            previous = Some(&admission.component_spec);

            if admission.maximum_root_instances == 0 {
                return Err(ComponentTopologyError::ZeroRootAdmission {
                    component_spec: admission.component_spec.clone(),
                });
            }

            let component_spec = self.get(&admission.component_spec).ok_or_else(|| {
                ComponentTopologyError::UnknownAdmissionSpec {
                    component_spec: admission.component_spec.clone(),
                }
            })?;
            if admission.spec_hash != component_spec.spec_hash {
                return Err(ComponentTopologyError::AdmissionSpecHashMismatch {
                    component_spec: admission.component_spec.clone(),
                    expected: component_spec.spec_hash,
                    received: admission.spec_hash,
                });
            }
            if admission.maximum_root_instances > component_spec.maximum_fleet_instances {
                return Err(ComponentTopologyError::RootAdmissionExceedsFleetMaximum {
                    component_spec: admission.component_spec.clone(),
                    maximum_root_instances: admission.maximum_root_instances,
                    maximum_fleet_instances: component_spec.maximum_fleet_instances,
                });
            }

            component_specs.push(component_spec.clone());
        }

        let projected = Self { component_specs };
        projected.canonical_bytes()?;
        Ok(projected)
    }

    /// Validate complete Fleet admission coverage and per-Spec sums.
    pub fn validate_fleet_admissions(
        &self,
        root_admissions: &[&[ComponentSpecAdmission]],
    ) -> Result<(), ComponentTopologyError> {
        let mut totals = self
            .component_specs
            .iter()
            .map(|spec| (spec.component_spec.clone(), 0_u32))
            .collect::<BTreeMap<_, _>>();

        for admissions in root_admissions {
            self.project_for_admissions(admissions)?;
            for admission in *admissions {
                let total = totals.get_mut(&admission.component_spec).ok_or_else(|| {
                    ComponentTopologyError::UnknownAdmissionSpec {
                        component_spec: admission.component_spec.clone(),
                    }
                })?;
                *total = total
                    .checked_add(admission.maximum_root_instances)
                    .ok_or_else(|| ComponentTopologyError::FleetAdmissionOverflow {
                        component_spec: admission.component_spec.clone(),
                    })?;
            }
        }

        for component_spec in &self.component_specs {
            let admitted = totals[&component_spec.component_spec];
            if admitted == 0 {
                return Err(ComponentTopologyError::MissingFleetAdmission {
                    component_spec: component_spec.component_spec.clone(),
                });
            }
            if admitted > component_spec.maximum_fleet_instances {
                return Err(ComponentTopologyError::FleetAdmissionsExceedMaximum {
                    component_spec: component_spec.component_spec.clone(),
                    admitted,
                    maximum_fleet_instances: component_spec.maximum_fleet_instances,
                });
            }
        }

        Ok(())
    }
}

///
/// ComponentSpec
///
/// One compiled Component role, immutable Spec hash, limits, and direct-child set.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentSpec {
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub component_role: CanisterRole,
    pub maximum_fleet_instances: u32,
    pub limits: ComponentLimits,
    pub children: Vec<ComponentChildSpec>,
}

///
/// ComponentLimits
///
/// Explicit aggregate quotas compiled for every concrete instance of one Spec.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentLimits {
    pub maximum_children: u32,
    pub maximum_registry_bytes: u64,
    pub cycles_funding: CyclesFundingBudget,
}

///
/// ComponentChildSpec
///
/// One canonically ordered direct structural edge owned by a Component Spec.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentChildSpec {
    pub role: CanisterRole,
    pub kind: ComponentChildKind,
    pub maximum_instances: u32,
    pub cycles_funding: ComponentChildFundingPolicy,
}

///
/// ComponentChildFundingPolicy
///
/// Compiled child-level funding limits without configuration-only deserializers.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentChildFundingPolicy {
    pub max_per_request: Cycles,
    pub max_per_child: Cycles,
    pub cooldown_secs: u64,
}

///
/// ComponentTopologyError
///
/// Typed failure while compiling, encoding, or projecting Component Topology.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ComponentTopologyError {
    #[error("Component Spec '{component_spec}' admission hash does not match compiled topology")]
    AdmissionSpecHashMismatch {
        component_spec: ComponentSpecId,
        expected: [u8; 32],
        received: [u8; 32],
    },

    #[error("{subject} canonical encoding is {actual_bytes} bytes, exceeding {maximum_bytes}")]
    CanonicalBytesExceeded {
        subject: &'static str,
        actual_bytes: usize,
        maximum_bytes: usize,
    },

    #[error("protected binding field '{field}' must not use the anonymous principal")]
    AnonymousBindingPrincipal { field: &'static str },

    #[error("Component binding authority does not match its Fleet Subnet Root")]
    BindingAuthorityMismatch,

    #[error(
        "Component binding role '{received}' does not match Spec '{component_spec}' role '{expected}'"
    )]
    BindingComponentRoleMismatch {
        component_spec: ComponentSpecId,
        expected: CanisterRole,
        received: CanisterRole,
    },

    #[error("Component binding placement Subnet does not match its Fleet Subnet Root")]
    BindingPlacementSubnetMismatch,

    #[error("Component binding root principal does not match its Fleet Subnet Root")]
    BindingRootMismatch,

    #[error("Component binding hash does not match Spec '{component_spec}'")]
    BindingSpecHashMismatch { component_spec: ComponentSpecId },

    #[error("Component Child role '{role}' is not admitted by Spec '{component_spec}'")]
    ChildRoleNotAdmitted {
        component_spec: ComponentSpecId,
        role: CanisterRole,
    },

    #[error("protected Component Child principal must differ from its owner and root")]
    ChildPrincipalConflictsWithOwner,

    #[error("protected Component principal must differ from its Coordinator and root")]
    ComponentPrincipalConflictsWithAuthority,

    #[error("Fleet root set contains duplicate root principal {fleet_subnet_root}")]
    DuplicateFleetSubnetRootPrincipal {
        fleet_subnet_root: candid::Principal,
    },

    #[error("Fleet root set contains duplicate placement Subnet {placement_subnet}")]
    DuplicateFleetSubnetRootSubnet {
        placement_subnet: crate::ids::SubnetId,
    },

    #[error("Component Spec '{component_spec}' Fleet admission sum overflowed")]
    FleetAdmissionOverflow { component_spec: ComponentSpecId },

    #[error(
        "Component Spec '{component_spec}' Fleet admission sum {admitted} exceeds maximum {maximum_fleet_instances}"
    )]
    FleetAdmissionsExceedMaximum {
        component_spec: ComponentSpecId,
        admitted: u32,
        maximum_fleet_instances: u32,
    },

    #[error("Component Spec '{component_spec}' has no positive Fleet root admission")]
    MissingFleetAdmission { component_spec: ComponentSpecId },

    #[error("role '{role}' has no package declaration while compiling Component Topology")]
    MissingRoleDeclaration { role: CanisterRole },

    #[error("Fleet Subnet Root limit '{field}' must be positive")]
    NonPositiveRootLimit { field: &'static str },

    #[error(
        "root Component admissions are not in strict Component Spec order: '{previous}' before '{current}'"
    )]
    NonCanonicalAdmissionOrder {
        previous: ComponentSpecId,
        current: ComponentSpecId,
    },

    #[error(
        "Component Spec '{component_spec}' root admission {maximum_root_instances} exceeds Fleet maximum {maximum_fleet_instances}"
    )]
    RootAdmissionExceedsFleetMaximum {
        component_spec: ComponentSpecId,
        maximum_root_instances: u32,
        maximum_fleet_instances: u32,
    },

    #[error("Fleet Subnet Root authority does not match the other roots in this Fleet plan")]
    RootAuthorityMismatch,

    #[error("Fleet Subnet Root principal must differ from its Coordinator principal")]
    RootPrincipalConflictsWithCoordinator,

    #[error("Fleet Subnet Root topology digest does not match its exact admitted projection")]
    RootTopologyDigestMismatch {
        expected: ComponentTopologyDigest,
        received: ComponentTopologyDigest,
    },

    #[error("Fleet Subnet Root must contain at least one Component Spec admission")]
    EmptyRootAdmissions,

    #[error("root admission references unknown Component Spec '{component_spec}'")]
    UnknownAdmissionSpec { component_spec: ComponentSpecId },

    #[error("Component Spec '{component_spec}' root admission must be positive")]
    ZeroRootAdmission { component_spec: ComponentSpecId },
}

fn compile_component_spec(
    config: &ConfigModel,
    component_spec: &ComponentSpecId,
    source: &ComponentSpecConfig,
) -> Result<ComponentSpec, ComponentTopologyError> {
    let component_package = role_package(config, &source.component_role)?;
    let spec_hash = component_spec_hash(config, source, component_package)?;
    let children = source
        .children
        .iter()
        .map(|(role, child)| ComponentChildSpec {
            role: role.clone(),
            kind: child.kind,
            maximum_instances: child.maximum_instances,
            cycles_funding: ComponentChildFundingPolicy {
                max_per_request: child.cycles_funding.max_per_request.clone(),
                max_per_child: child.cycles_funding.max_per_child.clone(),
                cooldown_secs: child.cycles_funding.cooldown_secs,
            },
        })
        .collect();

    Ok(ComponentSpec {
        component_spec: component_spec.clone(),
        spec_hash,
        component_role: source.component_role.clone(),
        maximum_fleet_instances: source.maximum_instances,
        limits: ComponentLimits {
            maximum_children: source.limits.maximum_children,
            maximum_registry_bytes: source.limits.maximum_registry_bytes,
            cycles_funding: CyclesFundingBudget {
                window_secs: source.limits.cycles_funding.window_secs,
                maximum_cycles: source.limits.cycles_funding.maximum_cycles.clone(),
            },
        },
        children,
    })
}

fn component_spec_hash(
    config: &ConfigModel,
    source: &ComponentSpecConfig,
    component_package: &str,
) -> Result<[u8; 32], ComponentTopologyError> {
    let mut encoder = CanonicalEncoder::new(COMPONENT_SPEC_HASH_DOMAIN);

    // The fixed depth marker makes the non-recursive authority shape explicit.
    encoder.u8(1);
    encoder.string(source.component_role.as_str());
    encoder.string(component_package);
    encoder.u32(source.maximum_instances);
    encode_component_limits(&mut encoder, source);
    encode_component_runtime_policy(&mut encoder, source);
    encode_scaling(&mut encoder, source.scaling.as_ref());
    encode_sharding(&mut encoder, source.sharding.as_ref());
    encode_binding(&mut encoder, source.binding.as_ref());
    encoder.u64(source.children.len() as u64);

    for (role, child) in &source.children {
        encoder.string(role.as_str());
        encoder.string(role_package(config, role)?);
        encode_component_child(&mut encoder, child);
    }

    let bytes = encoder.finish("Component Spec")?;
    Ok(Sha256::digest(bytes).into())
}

fn role_package<'a>(
    config: &'a ConfigModel,
    role: &CanisterRole,
) -> Result<&'a str, ComponentTopologyError> {
    config
        .roles
        .get(role)
        .map(|declaration| declaration.package.as_str())
        .ok_or_else(|| ComponentTopologyError::MissingRoleDeclaration { role: role.clone() })
}

fn encode_compiled_component_spec(encoder: &mut CanonicalEncoder, spec: &ComponentSpec) {
    encoder.string(spec.component_spec.as_str());
    encoder.bytes(&spec.spec_hash);
    encoder.string(spec.component_role.as_str());
    encoder.u32(spec.maximum_fleet_instances);
    encoder.u32(spec.limits.maximum_children);
    encoder.u64(spec.limits.maximum_registry_bytes);
    encode_cycles_budget(encoder, &spec.limits.cycles_funding);
    encoder.u64(spec.children.len() as u64);

    for child in &spec.children {
        encoder.string(child.role.as_str());
        encoder.u8(component_child_kind_tag(child.kind));
        encoder.u32(child.maximum_instances);
        encode_compiled_child_funding_policy(encoder, &child.cycles_funding);
    }
}

fn encode_component_limits(encoder: &mut CanonicalEncoder, source: &ComponentSpecConfig) {
    encoder.u32(source.limits.maximum_children);
    encoder.u64(source.limits.maximum_registry_bytes);
    encoder.u64(source.limits.cycles_funding.window_secs);
    encoder.u128(source.limits.cycles_funding.maximum_cycles.to_u128());
}

fn encode_component_runtime_policy(encoder: &mut CanonicalEncoder, source: &ComponentSpecConfig) {
    encoder.u128(source.initial_cycles.to_u128());
    encode_topup(encoder, source.topup.as_ref());
    encode_cycles_funding_policy(encoder, &source.cycles_funding);
    encode_auth(encoder, &source.auth);
    encode_standards(encoder, &source.standards);
    encode_diagnostics(encoder, source.diagnostics);
    encode_metrics(encoder, source.metrics);
}

fn encode_component_child(encoder: &mut CanonicalEncoder, child: &ComponentChildConfig) {
    encoder.u8(component_child_kind_tag(child.kind));
    encoder.u32(child.maximum_instances);
    encoder.u128(child.initial_cycles.to_u128());
    encode_topup(encoder, child.topup.as_ref());
    encode_cycles_funding_policy(encoder, &child.cycles_funding);
    encode_auth(encoder, &child.auth);
    encode_standards(encoder, &child.standards);
    encode_diagnostics(encoder, child.diagnostics);
    encode_metrics(encoder, child.metrics);
}

fn encode_cycles_budget(encoder: &mut CanonicalEncoder, budget: &CyclesFundingBudget) {
    encoder.u64(budget.window_secs);
    encoder.u128(budget.maximum_cycles.to_u128());
}

fn encode_cycles_funding_policy(
    encoder: &mut CanonicalEncoder,
    policy: &CyclesFundingPolicyConfig,
) {
    encoder.u128(policy.max_per_request.to_u128());
    encoder.u128(policy.max_per_child.to_u128());
    encoder.u64(policy.cooldown_secs);
}

fn encode_compiled_child_funding_policy(
    encoder: &mut CanonicalEncoder,
    policy: &ComponentChildFundingPolicy,
) {
    encoder.u128(policy.max_per_request.to_u128());
    encoder.u128(policy.max_per_child.to_u128());
    encoder.u64(policy.cooldown_secs);
}

fn encode_topup(encoder: &mut CanonicalEncoder, topup: Option<&TopupPolicy>) {
    let Some(topup) = topup else {
        encoder.u8(0);
        return;
    };

    encoder.u8(1);
    encoder.u128(topup.threshold.to_u128());
    encoder.u128(topup.amount.to_u128());
}

fn encode_scaling(encoder: &mut CanonicalEncoder, scaling: Option<&ScalingConfig>) {
    let pools = scaling.map_or(0, |config| config.pools.len());
    encoder.u64(pools as u64);

    if let Some(scaling) = scaling {
        for (name, pool) in &scaling.pools {
            encoder.string(name);
            encoder.string(pool.canister_role.as_str());
            encode_scale_pool_policy(encoder, &pool.policy);
        }
    }
}

fn encode_scale_pool_policy(encoder: &mut CanonicalEncoder, policy: &ScalePoolPolicy) {
    encoder.u32(policy.initial_workers);
    encoder.u32(policy.min_workers);
    encoder.u32(policy.max_workers);
}

fn encode_sharding(encoder: &mut CanonicalEncoder, sharding: Option<&ShardingConfig>) {
    let pools = sharding.map_or(0, |config| config.pools.len());
    encoder.u64(pools as u64);

    if let Some(sharding) = sharding {
        for (name, pool) in &sharding.pools {
            encoder.string(name);
            encoder.string(pool.canister_role.as_str());
            encode_shard_pool_policy(encoder, &pool.policy);
        }
    }
}

fn encode_shard_pool_policy(encoder: &mut CanonicalEncoder, policy: &ShardPoolPolicy) {
    encoder.u32(policy.capacity);
    encoder.u32(policy.initial_shards);
    encoder.u32(policy.max_shards);
}

fn encode_binding(encoder: &mut CanonicalEncoder, binding: Option<&BindingConfig>) {
    let pools = binding.map_or(0, |config| config.pools.len());
    encoder.u64(pools as u64);

    if let Some(binding) = binding {
        for (name, pool) in &binding.pools {
            encoder.string(name);
            encoder.string(pool.canister_role.as_str());
            encoder.string(&pool.key_name);
        }
    }
}

fn encode_auth(encoder: &mut CanonicalEncoder, auth: &CanisterAuthConfig) {
    encoder.boolean(auth.delegated_token_issuer);
    encoder.boolean(auth.delegated_token_verifier);
    encoder.boolean(auth.role_attestation_cache);
}

fn encode_standards(encoder: &mut CanonicalEncoder, standards: &StandardsCanisterConfig) {
    encoder.boolean(standards.icrc21);
}

fn encode_diagnostics(encoder: &mut CanonicalEncoder, diagnostics: DiagnosticsCanisterConfig) {
    encoder.boolean(diagnostics.memory_ledger);
}

fn encode_metrics(encoder: &mut CanonicalEncoder, metrics: MetricsCanisterConfig) {
    match metrics.profile {
        None => encoder.u8(0),
        Some(profile) => {
            encoder.u8(1);
            encoder.u8(metrics_profile_tag(profile));
        }
    }
}

const fn component_child_kind_tag(kind: ComponentChildKind) -> u8 {
    match kind {
        ComponentChildKind::Singleton => 0,
        ComponentChildKind::Replica => 1,
        ComponentChildKind::Shard => 2,
        ComponentChildKind::Instance => 3,
    }
}

const fn metrics_profile_tag(profile: MetricsProfile) -> u8 {
    match profile {
        MetricsProfile::Leaf => 0,
        MetricsProfile::Hub => 1,
        MetricsProfile::Storage => 2,
        MetricsProfile::Root => 3,
        MetricsProfile::Full => 4,
    }
}

struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    fn new(domain: &[u8]) -> Self {
        let mut encoder = Self { bytes: Vec::new() };
        encoder.bytes(domain);
        encoder.u32(COMPONENT_TOPOLOGY_SCHEMA_VERSION);
        encoder
    }

    fn boolean(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn finish(self, subject: &'static str) -> Result<Vec<u8>, ComponentTopologyError> {
        if self.bytes.len() > MAX_COMPONENT_TOPOLOGY_CANONICAL_BYTES {
            return Err(ComponentTopologyError::CanonicalBytesExceeded {
                subject,
                actual_bytes: self.bytes.len(),
                maximum_bytes: MAX_COMPONENT_TOPOLOGY_CANONICAL_BYTES,
            });
        }
        Ok(self.bytes)
    }
}
