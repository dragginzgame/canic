//! Resolve an operator's shard-pool declaration against the chosen app and fleet.
use super::{CanisterConfig, FleetConfig, name};
use crate::{
    DeploymentName, Error, FleetPlanning, Result, SelectedFleet, artifact, store, supplier,
};
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
    /// Final deadline for dispatching new creation work, in IC nanoseconds.
    pub expires_at_ns: Option<u64>,
}
/// A dedicated child per project key, bounded independently from shared shards.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstancePoolConfig {
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
    /// Deadline for new work, required when planning.
    pub expires_at_ns: Option<u64>,
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
    /// Worker ceiling, including parked allocations.
    pub max_workers: u8,
    /// Lifetime creation debit ceiling.
    pub creation_budget: u64,
    /// Last authorized creation time.
    pub expires_at_ns: Option<u64>,
}

impl WorkerPoolConfig {
    fn recipe(&self) -> InstancePoolConfig {
        InstancePoolConfig {
            component: self.component.clone(),
            root: self.root.clone(),
            store: self.store.clone(),
            release: self.release.clone(),
            max_instances: self.max_workers,
            creation_budget: self.creation_budget,
            expires_at_ns: self.expires_at_ns,
            install_args: None,
            increase_from: None,
        }
    }
    pub(super) fn validate(
        &self,
        slots: &BTreeMap<String, CanisterConfig>,
        cap: Option<u8>,
        shards: Option<&ShardPoolConfig>,
        instances: Option<&InstancePoolConfig>,
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
            .map(|p| (&p.component, &p.root, &p.store))
            .into_iter()
            .chain(instances.map(|p| (&p.component, &p.root, &p.store)))
        {
            if root != &self.root
                || store != &self.store
                || component == &self.component
                || parent(slots, component)? == hub
            {
                return Err(Error::Invalid(format!(
                    "worker_pool {} and {component} require distinct children and hubs on the same Root and Store",
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

fn grant<T: Serialize>(approval: T, method: u64) -> Result<PoolGrant<T>> {
    let request = ic_wire::encode(
        &approval,
        ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Request,
        },
        stored::LIMITS,
    )
    .map_err(supplier)?;
    Ok(PoolGrant { approval, request })
}

pub(super) fn validate_capacity(
    limit: Option<u8>,
    shards: Option<&ShardPoolConfig>,
    instances: Option<&InstancePoolConfig>,
    workers: Option<&WorkerPoolConfig>,
) -> Result<()> {
    if let Some(n) = limit {
        super::positive_count("subnet_child_limit", n, stored::MAX_SUBNET_CHILDREN)?;
        if shards.is_none() && instances.is_none() && workers.is_none() {
            return Err(Error::Invalid(format!(
                "subnet_child_limit = {n} requires a shard_pool, instance_pool or worker_pool"
            )));
        }
    }
    for (pool, field, count, previous) in [
        shards.map(|p| ("shard_pool", "max_shards", p.max_shards, p.increase_from)),
        instances.map(|p| {
            (
                "instance_pool",
                "max_instances",
                p.max_instances,
                p.increase_from,
            )
        }),
    ]
    .into_iter()
    .flatten()
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

fn inputs(
    selected: &SelectedFleet,
    config: &Path,
    component: &str,
    release: &ic_wasm_store_protocol::Release,
    args: Option<&Path>,
) -> Result<(
    ic_wasm_store_protocol::Manifest,
    canic_management_protocol::application::Arguments,
)> {
    let wasm = store::read(
        &selected.wasm_path(config, component, None)?,
        artifact::MAX_ARTIFACT_BYTES,
    )?;
    artifact::admit_bounded(&wasm, artifact::MAX_ARTIFACT_BYTES)?;
    let input = args
        .map(|path| {
            store::read(
                &config.with_file_name(path),
                canic_management_protocol::application::MAX_ARGUMENT_BYTES,
            )
        })
        .transpose()?
        .unwrap_or_default();
    Ok((
        crate::publication::release_manifest(release.clone(), &wasm),
        canic_management_protocol::application::Arguments::new(input).map_err(supplier)?,
    ))
}

pub(crate) fn preview(
    config_path: &Path,
    selected: &SelectedFleet,
    controller: &str,
) -> Result<FleetPlanning> {
    let principal = crate::network::principal(controller)?;
    if principal.to_text() != controller
        || principal == ic_platform_types::Principal::anonymous()
        || principal.as_slice().is_empty()
    {
        return Err(Error::Invalid(
            "expected a canonical controller principal".into(),
        ));
    }
    let now = crate::now()?;
    let shards = selected
        .config
        .shard_pool
        .as_ref()
        .map(|pool| {
            check_deadline("shard_pool", pool.expires_at_ns, now)?;
            let (manifest, args) = inputs(
                selected,
                config_path,
                &pool.component,
                &pool.release,
                pool.install_args.as_deref(),
            )?;
            grant(
                selected.config.shard_pool_approval(manifest, args)?,
                stored::CONFIGURE_SHARDS,
            )
        })
        .transpose()?;
    let capacity = |root: &str| {
        selected
            .config
            .subnet_child_limit
            .map(|max_children| {
                grant(
                    stored::SubnetCapacity {
                        root: root.to_owned(),
                        max_children,
                    },
                    stored::CONFIGURE_CAPACITY,
                )
            })
            .transpose()
    };
    let mut increases = Vec::new();
    if let Some((shard, old)) = shards.as_ref().zip(
        selected
            .config
            .shard_pool
            .as_ref()
            .and_then(|pool| pool.increase_from),
    ) {
        let mut prior = shard.approval.clone();
        let next = std::mem::replace(&mut prior.max_shards, old);
        increases.push(grant(
            stored::IncreasePool::Shards(prior, next),
            stored::INCREASE_POOL,
        )?);
    }
    let network = selected.config.network.clone();
    let controller = controller.to_owned();
    let allocation = |pool: &InstancePoolConfig,
                      label: &str,
                      method|
     -> Result<PoolGrant<stored::InstancePool>> {
        let expires_at_ns = check_deadline(label, pool.expires_at_ns, now)?;
        let (manifest, args) = inputs(
            selected,
            config_path,
            &pool.component,
            &pool.release,
            pool.install_args.as_deref(),
        )?;
        let (root, hub, store) =
            selected
                .config
                .pool_bindings(&pool.component, &pool.root, &pool.store)?;
        if selected.config.receiver_selected(&pool.component) {
            return Err(Error::Invalid("dedicated children require a topology install frame; receiver installation is unsupported".into()));
        }
        let arguments = topology(
            &selected.config.canisters,
            &pool.component,
            &root,
            &hub,
            &args,
        )?;
        grant(
            stored::InstancePool {
                root,
                hub,
                component: pool.component.clone(),
                reference: stored::Reference { store, manifest },
                arguments,
                max_instances: pool.max_instances,
                creation_budget: pool.creation_budget,
                expires_at_ns,
            },
            method,
        )
    };
    let instances = selected
        .config
        .instance_pool
        .as_ref()
        .map(|p| allocation(p, "instance_pool", stored::CONFIGURE_INSTANCES))
        .transpose()?;
    let workers = selected
        .config
        .worker_pool
        .as_ref()
        .map(|p| allocation(&p.recipe(), "worker_pool", stored::CONFIGURE_WORKERS))
        .transpose()?;
    if let Some(first) = instances.as_ref().or(workers.as_ref()) {
        if let Some((instance, old)) = instances.as_ref().zip(
            selected
                .config
                .instance_pool
                .as_ref()
                .and_then(|p| p.increase_from),
        ) {
            let mut prior = instance.approval.clone();
            let next = std::mem::replace(&mut prior.max_instances, old);
            increases.push(grant(
                stored::IncreasePool::Instances(prior, next),
                stored::INCREASE_POOL,
            )?);
        }
        return Ok(FleetPlanning::PlacementPools {
            capacity: capacity(&first.approval.root)?,
            increases,
            app: selected.app.clone(),
            fleet: selected.fleet.clone(),
            network,
            controller,
            shards: shards.map(Box::new),
            instances: instances.map(Box::new),
            workers: workers.map(Box::new),
        });
    }
    let PoolGrant { approval, request } =
        shards.ok_or_else(|| Error::Invalid("no placement pools".into()))?;
    Ok(FleetPlanning::ShardPool {
        capacity: capacity(&approval.root)?,
        increases,
        request,
        deployment: DeploymentName {
            app: selected.app.clone(),
            fleet: selected.fleet.clone(),
            canister: approval.pool.clone(),
        },
        network,
        controller,
        approval: Box::new(approval),
    })
}

fn validate_deadline(pool: &str, value: Option<u64>) -> Result<()> {
    if value == Some(0) {
        return Err(Error::Invalid(format!(
            "{pool}.expires_at_ns = 0 must be a positive Unix timestamp in nanoseconds"
        )));
    }
    Ok(())
}

fn check_deadline(pool: &str, deadline: Option<u64>, now: u64) -> Result<u64> {
    let deadline = deadline.ok_or_else(|| {
        Error::Invalid(format!(
            "{pool}.expires_at_ns is missing; a growth approval requires an explicit deadline"
        ))
    })?;
    if deadline <= now {
        return Err(Error::Invalid(format!(
            "{pool}.expires_at_ns = {deadline} has expired; expected a deadline later than {now} (Unix nanoseconds)"
        )));
    }
    Ok(deadline)
}

impl ShardPoolConfig {
    fn deadline(&self) -> Result<u64> {
        self.expires_at_ns.ok_or_else(|| {
            Error::Invalid("shard_pool.expires_at_ns is missing; a growth approval requires an explicit deadline".into())
        })
    }

    pub(super) fn validate(&self, slots: &BTreeMap<String, CanisterConfig>) -> Result<()> {
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
        manifest: ic_wasm_store_protocol::Manifest,
        arguments: canic_management_protocol::application::Arguments,
    ) -> Result<stored::ShardPool> {
        let policy = self
            .shard_pool
            .as_ref()
            .ok_or_else(|| Error::Invalid("no shard pool configured".into()))?;
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
        let arguments = if self.receiver_selected(&policy.component) {
            if self
                .receiver_policy
                .as_ref()
                .is_none_or(|p| p.root != policy.root)
            {
                return Err(Error::Invalid(
                    "shard pool must use the receiver policy Root".into(),
                ));
            }
            let bytes = self.receiver_install_args(
                &policy.component,
                &root,
                arguments.as_slice().to_vec(),
            )?;
            canic_management_protocol::application::Arguments::new(bytes).map_err(supplier)?
        } else {
            arguments
        };
        Ok(stored::ShardPool {
            arguments,
            pool: policy.component.clone(),
            root,
            hub,
            max_shards: policy.max_shards,
            keys_per_shard: policy.keys_per_shard,
            creation_budget: policy.creation_budget,
            expires_at_ns: policy.deadline()?,
            reference: stored::Reference { store, manifest },
        })
    }
}

impl FleetConfig {
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
                "pool component {component:?} must have a declared parent hub"
            ))
        })
}

impl InstancePoolConfig {
    pub(super) fn validate(
        &self,
        slots: &BTreeMap<String, CanisterConfig>,
        shared: Option<&ShardPoolConfig>,
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
            "instance_pool.max_instances",
            self.max_instances,
            stored::MAX_INSTANCES,
        )?;
        validate_deadline("instance_pool", self.expires_at_ns)?;
        if let Some(other) = shared
            && (self.root != other.root
                || self.store != other.store
                || self.component == other.component
                || hub == parent(slots, &other.component)?)
        {
            return Err(Error::Invalid(format!(
                "instance_pool (root {:?}, store {:?}, component {:?}, hub {hub:?}) and shard_pool (root {:?}, store {:?}, component {:?}, hub {:?}) require the same root and store and distinct children and hubs",
                self.root,
                self.store,
                self.component,
                other.root,
                other.store,
                other.component,
                parent(slots, &other.component)?
            )));
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
