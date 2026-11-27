//! Resolve an operator's shard-pool declaration against the chosen app and fleet.
use super::{CanisterConfig, FleetConfig, name};
mod plan;
pub(crate) use plan::{check_pool_requirements, preview};

use crate::{Error, Result, supplier};
use canic_management_protocol::stored;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Fleet-specific limits and explicitly named infrastructure for one shard component.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardPoolConfig {
    /// Component to allocate. Its declared parent is the requesting hub.
    pub component: String,
    /// Configured effect owner.
    pub root: String,
    /// Configured artifact supplier.
    pub store: String,
    /// Exact release label in the selected Store. Planning computes its manifest from the app Wasm.
    pub release: ic_wasm_store_protocol::Release,
    /// Optional file of exact application initialization bytes, relative to canic.toml.
    pub install_args: Option<PathBuf>,
    /// Wrap fresh install bytes with the resolved parent and installing Root.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub install_topology: bool,
    /// Maximum lifetime shard count.
    pub max_shards: u8,
    /// Maximum live keys per shard.
    pub keys_per_shard: u32,
    /// Previous approved pool count when planning an increase on an existing Root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub increase_from: Option<u8>,
    /// Lifetime ceiling on requester charges for fresh and reserve acquisitions.
    pub creation_budget: u64,
    /// Final deadline for dispatching new creation work, in IC nanoseconds.
    pub expires_at_ns: Option<u64>,
}
/// A dedicated child per project key, bounded independently from shared shards.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryConfig {
    /// Optional child that each allocated instance can request for itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub singleton: Option<SingletonConfig>,
    /// Declared child component; its parent owns the pool.
    pub component: String,
    /// Installing management owner.
    pub root: String,
    /// Versioned artifact owner.
    pub store: String,
    /// Release selected for all dedicated children.
    pub release: ic_wasm_store_protocol::Release,
    /// Application bytes nested inside the approved parent topology.
    pub install_args: Option<PathBuf>,
    /// Maximum dedicated allocations.
    pub max_instances: u8,
    /// Previous approved pool count when planning an increase on an existing Root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub increase_from: Option<u8>,
    /// Lifetime ceiling on requester charges for fresh and reserve acquisitions.
    pub creation_budget: u64,
    /// Deadline for new work, required when planning.
    pub expires_at_ns: Option<u64>,
}

/// A registry-wide singleton uses the registry's Root, Store and expiry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SingletonConfig {
    /// Direct child of the allocated instance in the app topology.
    pub component: String,
    /// Exact release to install.
    pub release: ic_wasm_store_protocol::Release,
    /// Optional application initialization bytes.
    pub install_args: Option<PathBuf>,
    /// Maximum registry-wide allocations under this approval.
    pub maximum: u32,
    /// Lifetime ceiling on requester charges for fresh and reserve acquisitions.
    pub creation_budget: u64,
}

/// Named surplus canisters on one network with the same release authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StandbyImports {
    /// Source network: `ic` or `local`. Must match the fleet.
    pub network: String,
    /// Ordered canister principals to reserve.
    pub principals: Vec<String>,
    /// Former fleet Root required to release every principal in this group.
    pub previous_root: Option<String>,
}

/// Operator declaration resolved to Root's ordered, per-principal approval.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StandbyPoolConfig {
    /// Fleet Root slot.
    pub root: String,
    /// Desired ready spares.
    pub min_available_spares: u8,
    /// Lifetime allowance for paid creations.
    pub maximum_creations: u8,
    /// Cutoff for paid creation in IC nanoseconds.
    pub expires_at_ns: u64,
    /// Cycle floor for each imported canister.
    pub minimum_import_cycles: std::num::NonZeroU64,
    /// Explicit network and principals for each import group.
    #[serde(default)]
    pub imports: Vec<StandbyImports>,
}

impl StandbyPoolConfig {
    pub(super) fn resolve(&self, network: &super::FleetNetwork) -> Result<stored::StandbyPolicy> {
        let expected = match network {
            super::FleetNetwork::Bound(crate::Network::Mainnet) => "ic",
            super::FleetNetwork::Bound(crate::Network::Local { .. })
            | super::FleetNetwork::Unbound(_) => "local",
        };
        let mut imports = Vec::new();
        for (index, group) in self.imports.iter().enumerate() {
            if group.network != expected {
                return Err(Error::Invalid(format!(
                    "standby_pool.imports[{index}].network = {:?}; this fleet requires {expected:?}",
                    group.network
                )));
            }
            if group.principals.is_empty() {
                return Err(Error::Invalid(format!(
                    "standby_pool.imports[{index}].principals requires at least one canister"
                )));
            }
            imports.extend(
                group
                    .principals
                    .iter()
                    .map(|principal| stored::StandbyImport {
                        principal: principal.clone(),
                        previous_root: group.previous_root.clone(),
                    }),
            );
        }
        Ok(stored::StandbyPolicy {
            root: self.root.clone(),
            minimum_available: self.min_available_spares,
            maximum_creations: self.maximum_creations,
            expires_at_ns: self.expires_at_ns,
            minimum_import_cycles: self.minimum_import_cycles,
            imports,
        })
    }
}

/// A hub's allowance for reusable workers on this subnet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerPoolConfig {
    /// Approved application child role.
    pub component: String,
    /// Management and artifact owners.
    pub root: String,
    /// Versioned artifact owner.
    pub store: String,
    /// Approved Wasm release.
    pub release: ic_wasm_store_protocol::Release,
    /// Minimum observed child balance for return to standby. Omission grants no reuse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_reuse_cycles: Option<std::num::NonZeroU64>,
    /// Worker ceiling, including parked allocations.
    pub max_workers: u8,
    /// Lifetime ceiling on requester charges for fresh and reserve acquisitions.
    pub creation_budget: u64,
    /// Last authorized creation time.
    pub expires_at_ns: Option<u64>,
}

impl WorkerPoolConfig {
    fn validate(&self, slots: &BTreeMap<String, CanisterConfig>, cap: u32) -> Result<()> {
        super::positive_count(
            "worker_pool.max_workers",
            self.max_workers,
            cap.min(u32::from(stored::MAX_INSTANCES)) as u8,
        )?;
        validate_deadline("worker_pool", self.expires_at_ns)?;
        validate_pool(
            slots,
            &self.component,
            &self.root,
            &self.store,
            &self.release,
            None,
        )?;
        Ok(())
    }
}

/// Decoded grant and the exact request to submit to Root.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PoolGrant<T> {
    /// Controller-approved input.
    pub approval: T,
    /// Original Wire request frame.
    pub request: Vec<u8>,
}

pub(super) fn validate_pools(
    slots: &BTreeMap<String, CanisterConfig>,
    limit: Option<u32>,
    allow_large_subnet: bool,
    shards: &[ShardPoolConfig],
    instances: Option<&RegistryConfig>,
    workers: &[WorkerPoolConfig],
    standby: Option<&stored::StandbyPolicy>,
) -> Result<()> {
    let cap = limit.unwrap_or(stored::DEFAULT_SUBNET_CHILD_LIMIT);
    super::positive_count("subnet_child_limit", cap, u32::MAX)?;
    if !stored::SubnetCapacity::permits_limit(cap, allow_large_subnet) {
        return Err(Error::Invalid(format!(
            "subnet_child_limit = {cap} exceeds {}; set allow_large_subnet = true to approve this higher ceiling",
            stored::DEFAULT_SUBNET_CHILD_LIMIT
        )));
    }
    validate_capacity(limit, shards, instances, workers, standby)?;
    for pool in shards {
        pool.validate(slots)?;
    }
    if let Some(pool) = instances {
        pool.validate(
            slots,
            shards
                .iter()
                .map(|p| &p.component)
                .chain(workers.iter().map(|p| &p.component)),
        )?;
    }
    for pool in workers {
        pool.validate(slots, limit.unwrap_or(stored::DEFAULT_SUBNET_CHILD_LIMIT))?;
    }
    Ok(())
}

fn validate_capacity(
    limit: Option<u32>,
    shards: &[ShardPoolConfig],
    instances: Option<&RegistryConfig>,
    workers: &[WorkerPoolConfig],
    standby: Option<&stored::StandbyPolicy>,
) -> Result<()> {
    for (field, count, maximum) in [
        ("shard_pools", shards.len(), stored::MAX_SHARDS),
        ("worker_pools", workers.len(), stored::MAX_INSTANCES),
    ] {
        if count > usize::from(maximum) {
            return Err(Error::Invalid(format!(
                "{field} declares {count} pools; maximum is {maximum}"
            )));
        }
    }
    let mut components = std::collections::BTreeSet::new();
    let mut placement = None;
    for (component, root, store) in shards
        .iter()
        .map(|p| (&p.component, &p.root, &p.store))
        .chain(instances.map(|p| (&p.component, &p.root, &p.store)))
        .chain(workers.iter().map(|p| (&p.component, &p.root, &p.store)))
    {
        if !components.insert(component) {
            return Err(Error::Invalid(format!(
                "placement component {component:?} is declared in more than one pool"
            )));
        }
        let (first, expected_root, expected_store) =
            *placement.get_or_insert((component, root, store));
        for (field, value, expected) in [
            ("root", root, expected_root),
            ("store", store, expected_store),
        ] {
            if value != expected {
                return Err(Error::Invalid(format!(
                    "pool {component:?}.{field} = {value:?} must match pool {first:?}.{field} = {expected:?}"
                )));
            }
        }
    }
    let cap = limit.unwrap_or(stored::DEFAULT_SUBNET_CHILD_LIMIT);
    for (kind, count) in [("shard", shards.len()), ("worker", workers.len())] {
        if count > cap as usize {
            return Err(Error::Invalid(format!(
                "{count} {kind} pools exceed subnet_child_limit = {cap}"
            )));
        }
    }
    if let Some(p) = standby {
        let reserve_limit = cap.min(u32::from(stored::MAX_STANDBY_POOL_ENTRIES));
        name(&p.root)?;
        for (label, count) in [
            ("min_available_spares", usize::from(p.minimum_available)),
            ("maximum_creations", usize::from(p.maximum_creations)),
            ("imports", p.imports.len()),
        ] {
            if count > reserve_limit as usize && count <= cap as usize {
                return Err(Error::Invalid(format!(
                    "standby_pool.{label} = {count} exceeds the standby pool ledger limit of {reserve_limit}"
                )));
            }
            if count > cap as usize {
                return Err(Error::Invalid(format!(
                    "standby_pool.{label} = {count} exceeds subnet_child_limit = {cap}"
                )));
            }
        }
        validate_deadline("standby_pool", Some(p.expires_at_ns))?;
        if let Some((component, root, _)) = placement
            && root != &p.root
        {
            return Err(Error::Invalid(format!(
                "standby_pool.root = {:?} must match pool {component:?}.root = {root:?}",
                p.root
            )));
        }
        let mut seen = std::collections::BTreeSet::new();
        for candidate in &p.imports {
            let text = &candidate.principal;
            if let Some(previous) = &candidate.previous_root {
                let id = crate::network::principal(previous)?;
                if id.as_slice().last() != Some(&1) || previous == text || id.to_text() != *previous
                {
                    return Err(Error::Invalid(format!(
                        "invalid standby_pool previous_root {previous} for {text}"
                    )));
                }
            }
            let id = crate::network::principal(text)?;
            if id.to_text() != *text || id.as_slice().last() != Some(&1) || !seen.insert(id) {
                return Err(Error::Invalid(format!(
                    "standby_pool.imports contains invalid or duplicate principal {text}"
                )));
            }
        }
    }
    for (pool, field, count, previous) in shards
        .iter()
        .map(|p| ("shard_pool", "max_shards", p.max_shards, p.increase_from))
        .chain(instances.map(|p| {
            (
                "registry",
                "max_instances",
                p.max_instances,
                p.increase_from,
            )
        }))
    {
        if u32::from(count) > cap {
            return Err(Error::Invalid(format!(
                "{pool}.{field} = {count} exceeds subnet_child_limit of {cap}"
            )));
        }
        if let Some(old) = previous
            && (old == 0 || old >= count)
        {
            return Err(Error::Invalid(format!(
                "{pool}.increase_from = {old} must be positive and less than {field} = {count}"
            )));
        }
    }
    Ok(())
}

fn validate_deadline(pool: &str, value: Option<u64>) -> Result<()> {
    if value == Some(0) {
        return Err(Error::Invalid(format!(
            "{pool}.expires_at_ns = 0 must be a positive Unix timestamp in nanoseconds"
        )));
    }
    Ok(())
}

impl ShardPoolConfig {
    fn deadline(&self) -> Result<u64> {
        self.expires_at_ns.ok_or_else(|| {
            Error::Invalid("shard_pool.expires_at_ns is missing; a growth approval requires an explicit deadline".into())
        })
    }

    fn validate(&self, slots: &BTreeMap<String, CanisterConfig>) -> Result<()> {
        validate_pool(
            slots,
            &self.component,
            &self.root,
            &self.store,
            &self.release,
            self.install_args.as_deref(),
        )?;
        super::positive_count("shard_pool.max_shards", self.max_shards, stored::MAX_SHARDS)?;
        super::positive_count(
            "shard_pool.keys_per_shard",
            self.keys_per_shard,
            canic::sharding::MAX_KEYS_PER_SHARD,
        )?;
        validate_deadline("shard_pool", self.expires_at_ns)?;
        Ok(())
    }
}
impl FleetConfig {
    /// Prepare the exact controller approval from this fleet and its selected immutable Store release.
    /// Preparation is local. The operator submits the resulting grant to its bound Root.
    /// # Errors
    /// Refuses incomplete declarations, unbound principals or invalid release metadata.
    pub fn shard_pool_approval(
        &self,
        component: &str,
        manifest: ic_wasm_store_protocol::Manifest,
        arguments: canic_management_protocol::application::Arguments,
    ) -> Result<stored::ShardPool> {
        let policy = self
            .policy
            .shard_pools
            .iter()
            .find(|pool| pool.component == component)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "no shard pool configured for component {component:?}"
                ))
            })?;
        self.approve_shard_policy(policy, manifest, arguments)
    }

    fn approve_shard_policy(
        &self,
        policy: &ShardPoolConfig,
        manifest: ic_wasm_store_protocol::Manifest,
        arguments: canic_management_protocol::application::Arguments,
    ) -> Result<stored::ShardPool> {
        policy.validate(&self.canisters)?;
        if !manifest.valid() || manifest.release != policy.release {
            return Err(Error::Invalid("invalid shard release manifest".into()));
        }
        let (root, hub, store) =
            self.pool_bindings(&policy.component, &policy.root, &policy.store)?;
        let arguments = if policy.install_topology {
            topology(&self.canisters, &policy.component, &root, &hub, &arguments)?
        } else {
            arguments
        };
        let arguments =
            self.pool_receiver_arguments(&policy.component, &policy.root, &root, arguments)?;
        Ok(stored::ShardPool {
            arguments,
            pool: policy.component.clone(),
            root,
            hub,
            max_shards: policy.max_shards,
            creation_budget: policy.creation_budget,
            expires_at_ns: policy.deadline()?,
            reference: stored::Reference { store, manifest },
        })
    }
}

impl FleetConfig {
    fn pool_receiver_arguments(
        &self,
        component: &str,
        root_slot: &str,
        root: &str,
        arguments: canic_management_protocol::application::Arguments,
    ) -> Result<canic_management_protocol::application::Arguments> {
        if !self.receiver_selected(component) {
            return Ok(arguments);
        }
        if self
            .policy
            .receiver_policy
            .as_ref()
            .is_none_or(|p| p.root != root_slot)
        {
            return Err(Error::Invalid(
                "pool must use the receiver policy Root".into(),
            ));
        }
        let bytes = self.receiver_install_args(component, root, arguments.as_slice().to_vec())?;
        canic_management_protocol::application::Arguments::new(bytes).map_err(supplier)
    }

    fn pool_bindings(
        &self,
        component: &str,
        root: &str,
        store: &str,
    ) -> Result<(String, String, String)> {
        let id = |slot: &str| {
            let value = self
                .canisters
                .get(slot)
                .and_then(|c| c.id.clone())
                .ok_or_else(|| Error::Invalid(format!("missing pool binding for {slot}")))?;
            let principal = crate::network::principal(&value)?;
            if principal.to_text() != value || principal.as_slice().last() != Some(&1) {
                return Err(Error::Invalid(format!(
                    "pool binding {slot} must identify a canonical canister"
                )));
            }
            Ok(value)
        };
        if self
            .canisters
            .get(component)
            .is_some_and(|c| c.id.is_some())
        {
            return Err(Error::Invalid(
                "a pooled component cannot bind a fixed instance".into(),
            ));
        }
        Ok((
            id(root)?,
            id(parent(&self.canisters, component)?)?,
            id(store)?,
        ))
    }
}

fn topology(
    canisters: &BTreeMap<String, CanisterConfig>,
    component: &str,
    root: &str,
    hub: &str,
    arguments: &canic_management_protocol::application::Arguments,
) -> Result<canic_management_protocol::application::Arguments> {
    let bytes = canic::topology::Install {
        root: crate::network::principal(root)?,
        parent: crate::network::principal(hub)?,
        role: canisters
            .get(component)
            .and_then(|entry| entry.role.as_ref())
            .ok_or_else(|| {
                Error::Invalid(format!("topology install requires a role for {component}"))
            })?
            .clone(),
        application: arguments.as_slice().to_vec(),
    }
    .encode()
    .map_err(|e| Error::Invalid(e.into()))?;
    canic_management_protocol::application::Arguments::new(bytes).map_err(supplier)
}

fn parent<'a>(slots: &'a BTreeMap<String, CanisterConfig>, component: &str) -> Result<&'a str> {
    slots
        .get(component)
        .and_then(|c| c.parent.as_deref())
        .ok_or_else(|| {
            Error::Invalid(format!(
                "pool component {component:?} must have a declared parent"
            ))
        })
}

impl RegistryConfig {
    fn validate<'a>(
        &self,
        slots: &BTreeMap<String, CanisterConfig>,
        other_components: impl Iterator<Item = &'a String>,
    ) -> Result<()> {
        let hub = validate_pool(
            slots,
            &self.component,
            &self.root,
            &self.store,
            &self.release,
            self.install_args.as_deref(),
        )?;
        super::positive_count(
            "registry.max_instances",
            self.max_instances,
            stored::MAX_INSTANCES,
        )?;
        validate_deadline("registry", self.expires_at_ns)?;
        if let Some(child) = &self.singleton {
            let owner = validate_pool(
                slots,
                &child.component,
                &self.root,
                &self.store,
                &child.release,
                child.install_args.as_deref(),
            )?;
            if owner != self.component || slots[&child.component].id.is_some() {
                return Err(Error::Invalid(format!(
                    "registry.singleton.component = {:?} requires an unbound child of {:?}",
                    child.component, self.component
                )));
            }
            super::positive_count("registry.singleton.maximum", child.maximum, u32::MAX)?;
        }
        for component in other_components {
            if parent(slots, component)? == hub {
                return Err(Error::Invalid(format!(
                    "registry component {:?} and pool component {component:?} both have parent {hub:?}; the registry requires a separate parent",
                    self.component
                )));
            }
        }
        Ok(())
    }
}

fn validate_pool<'a>(
    slots: &'a BTreeMap<String, CanisterConfig>,
    component: &str,
    root: &str,
    store: &str,
    release: &ic_wasm_store_protocol::Release,
    args: Option<&Path>,
) -> Result<&'a str> {
    if !release.valid() {
        return Err(Error::Invalid(format!(
            "pool release.name = {:?}, release.version = {:?}; each must contain 1–64 ASCII letters, digits, dots, hyphens or underscores",
            release.name, release.version
        )));
    }
    if args.is_some_and(|p| p.as_os_str().is_empty()) {
        return Err(Error::Invalid(
            "pool install_args = \"\"; expected a nonempty file path".into(),
        ));
    }
    let hub = parent(slots, component)?;
    let mut names = std::collections::BTreeSet::new();
    for slot in [component, hub, root, store] {
        name(slot)?;
        if !slots.contains_key(slot) {
            return Err(Error::Invalid(format!(
                "pool slot {slot:?} is missing from the canister declarations"
            )));
        }
        if !names.insert(slot) {
            return Err(Error::Invalid(format!(
                "pool slot {slot:?} is repeated; component {component:?}, hub {hub:?}, root {root:?} and store {store:?} must be distinct"
            )));
        }
    }
    Ok(hub)
}
