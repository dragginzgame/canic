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
    /// Total authorized creation attachments in cycles.
    pub creation_budget: u64,
    /// Economic payer for new children under this grant.
    #[serde(default)]
    pub creation_payer: stored::CreationPayer,
    /// Final deadline for dispatching new creation work, in IC nanoseconds.
    pub expires_at_ns: Option<u64>,
}
/// A dedicated child per project key, bounded independently from shared shards.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryConfig {
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
    /// Total authorized creation attachments in cycles.
    pub creation_budget: u64,
    /// Economic payer for new children under this grant.
    #[serde(default)]
    pub creation_payer: stored::CreationPayer,
    /// Deadline for new work, required when planning.
    pub expires_at_ns: Option<u64>,
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
    /// Lifetime creation debit ceiling.
    pub creation_budget: u64,
    /// Economic payer for new children under this grant.
    #[serde(default)]
    pub creation_payer: stored::CreationPayer,
    /// Last authorized creation time.
    pub expires_at_ns: Option<u64>,
}

impl WorkerPoolConfig {
    fn validate(
        &self,
        slots: &BTreeMap<String, CanisterConfig>,
        cap: Option<u8>,
        shards: &[ShardPoolConfig],
        instances: Option<&RegistryConfig>,
    ) -> Result<()> {
        let cap = cap.ok_or_else(|| {
            Error::Invalid("worker_pool requires an explicit subnet_child_limit".into())
        })?;
        super::positive_count("worker_pool.max_workers", self.max_workers, cap)?;
        validate_deadline("worker_pool", self.expires_at_ns)?;
        let hub = validate_pool(
            slots,
            &self.component,
            &self.root,
            &self.store,
            &self.release,
            None,
        )?;
        for (component, root, store) in shards
            .iter()
            .map(|p| (&p.component, &p.root, &p.store))
            .chain(instances.map(|p| (&p.component, &p.root, &p.store)))
        {
            if root != &self.root
                || store != &self.store
                || component == &self.component
                || (instances.is_some_and(|p| p.component == *component)
                    && parent(slots, component)? == hub)
            {
                return Err(Error::Invalid(format!(
                    "worker_pool {} and {component} require distinct children on the same Root and Store, and a separate registry",
                    self.component
                )));
            }
        }
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
    limit: Option<u8>,
    shards: &[ShardPoolConfig],
    instances: Option<&RegistryConfig>,
    workers: &[WorkerPoolConfig],
    standby: Option<&stored::StandbyPolicy>,
) -> Result<()> {
    validate_capacity(limit, shards, instances, workers, standby)?;
    for pool in shards {
        pool.validate(slots)?;
    }
    if let Some(pool) = instances {
        pool.validate(slots, shards)?;
    }
    for pool in workers {
        pool.validate(slots, limit, shards, instances)?;
    }
    Ok(())
}

fn validate_capacity(
    limit: Option<u8>,
    shards: &[ShardPoolConfig],
    instances: Option<&RegistryConfig>,
    workers: &[WorkerPoolConfig],
    standby: Option<&stored::StandbyPolicy>,
) -> Result<()> {
    let mut components = std::collections::BTreeSet::new();
    for pool in shards {
        if !components.insert(&pool.component)
            || pool.root != shards[0].root
            || pool.store != shards[0].store
        {
            return Err(Error::Invalid(format!(
                "shard_pools repeats {:?} or differs in Root or Store",
                pool.component
            )));
        }
    }
    if shards.len() > 1 && limit.is_none() {
        return Err(Error::Invalid(
            "multiple shard pools require subnet_child_limit".into(),
        ));
    }
    if shards.len() > usize::from(limit.unwrap_or(2)) {
        return Err(Error::Invalid(
            "too many shard pools for the subnet child limit".into(),
        ));
    }
    for pool in workers {
        if !components.insert(&pool.component) {
            return Err(Error::Invalid(format!(
                "worker_pools repeats component {:?}",
                pool.component
            )));
        }
        if pool.root != workers[0].root || pool.store != workers[0].store {
            return Err(Error::Invalid(format!(
                "worker pool {:?} must share the fleet Root and Store",
                pool.component
            )));
        }
    }
    if workers.len() > usize::from(limit.unwrap_or_default()) {
        return Err(Error::Invalid(format!(
            "{} worker pools exceed subnet_child_limit = {:?}",
            workers.len(),
            limit
        )));
    }
    if let Some(p) = standby {
        let cap = limit
            .ok_or_else(|| Error::Invalid("standby_pool requires subnet_child_limit".into()))?;
        name(&p.root)?;
        for (label, count) in [
            ("min_available_spares", usize::from(p.minimum_available)),
            ("maximum_creations", usize::from(p.maximum_creations)),
            ("imports", p.imports.len()),
        ] {
            if count > usize::from(cap) {
                return Err(Error::Invalid(format!(
                    "standby_pool.{label} = {count} exceeds subnet_child_limit = {cap}"
                )));
            }
        }
        validate_deadline("standby_pool", Some(p.expires_at_ns))?;
        if shards
            .iter()
            .map(|p| &p.root)
            .chain(instances.iter().map(|p| &p.root))
            .chain(workers.iter().map(|p| &p.root))
            .any(|root| root != &p.root)
        {
            return Err(Error::Invalid(format!(
                "standby_pool.root = {:?} must match the placement Root",
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
    if let Some(n) = limit {
        super::positive_count("subnet_child_limit", n, stored::MAX_SUBNET_CHILDREN)?;
        if shards.is_empty() && instances.is_none() && workers.is_empty() && standby.is_none() {
            return Err(Error::Invalid(format!(
                "subnet_child_limit = {n} requires a shard_pool, registry, worker_pool or standby_pool"
            )));
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
        let ceiling = limit.unwrap_or(2);
        if count > ceiling {
            let source = if limit.is_some() {
                "subnet_child_limit"
            } else {
                "the default subnet child limit"
            };
            return Err(Error::Invalid(format!(
                "{pool}.{field} = {count} exceeds {source} of {ceiling}"
            )));
        }
        if let Some(old) = previous {
            if limit.is_none() {
                return Err(Error::Invalid(format!(
                    "{pool}.increase_from = {old} requires an explicit subnet_child_limit"
                )));
            }
            if old == 0 || old >= count {
                return Err(Error::Invalid(format!(
                    "{pool}.increase_from = {old} must be positive and less than {field} = {count}"
                )));
            }
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
        super::positive_count(
            "shard_pool.max_shards",
            self.max_shards,
            stored::MAX_SUBNET_CHILDREN,
        )?;
        super::positive_count(
            "shard_pool.keys_per_shard",
            self.keys_per_shard,
            stored::MAX_KEYS_PER_SHARD,
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
            .shard_pools
            .iter()
            .find(|pool| pool.component == component)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "no shard pool configured for component {component:?}"
                ))
            })?;
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
            keys_per_shard: policy.keys_per_shard,
            creation_payer: policy.creation_payer,
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
    fn validate(
        &self,
        slots: &BTreeMap<String, CanisterConfig>,
        shared: &[ShardPoolConfig],
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
        for other in shared {
            if self.root != other.root
                || self.store != other.store
                || self.component == other.component
                || hub == parent(slots, &other.component)?
            {
                return Err(Error::Invalid(format!(
                    "registry (root {:?}, store {:?}, component {:?}, parent {hub:?}) and shard_pool (root {:?}, store {:?}, component {:?}, hub {:?}) require the same root and store and distinct children and parents",
                    self.root,
                    self.store,
                    self.component,
                    other.root,
                    other.store,
                    other.component,
                    parent(slots, &other.component)?
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
