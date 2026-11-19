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
    /// Maximum lifetime keys per shard.
    pub keys_per_shard: u16,
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
    /// Total authorized creation attachments in cycles.
    pub creation_budget: u64,
    /// Deadline for new work, required when planning.
    pub expires_at_ns: Option<u64>,
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
            check_deadline(pool.expires_at_ns, now)?;
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
    let network = selected.config.network.clone();
    let controller = controller.to_owned();
    if let Some(pool) = &selected.config.instance_pool {
        let expires_at_ns = check_deadline(pool.expires_at_ns, now)?;
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
        let arguments = topology(&root, &hub, &args)?;
        let instances = grant(
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
            stored::CONFIGURE_INSTANCES,
        )?;
        return Ok(FleetPlanning::PlacementPools {
            app: selected.app.clone(),
            fleet: selected.fleet.clone(),
            network,
            controller,
            shards: shards.map(Box::new),
            instances: Box::new(instances),
        });
    }
    let PoolGrant { approval, request } =
        shards.ok_or_else(|| Error::Invalid("no placement pools".into()))?;
    Ok(FleetPlanning::ShardPool {
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

fn check_deadline(deadline: Option<u64>, now: u64) -> Result<u64> {
    let deadline = deadline.ok_or_else(|| {
        Error::Invalid("pool growth approval requires an explicit deadline".into())
    })?;
    if deadline <= now {
        return Err(Error::Invalid("pool growth approval has expired".into()));
    }
    Ok(deadline)
}

impl ShardPoolConfig {
    fn deadline(&self) -> Result<u64> {
        self.expires_at_ns.ok_or_else(|| {
            Error::Invalid("shard growth approval requires an explicit deadline".into())
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
        if !(1..=stored::SHARD_SLOTS.len()).contains(&usize::from(self.max_shards))
            || !(1..=stored::MAX_KEYS_PER_SHARD).contains(&self.keys_per_shard)
            || self.expires_at_ns == Some(0)
        {
            return Err(Error::Invalid(
                "shard pool bounds or deadline outside the supported range".into(),
            ));
        }
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
            topology(&root, &hub, &arguments)?
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
    root: &str,
    hub: &str,
    arguments: &canic_management_protocol::application::Arguments,
) -> Result<canic_management_protocol::application::Arguments> {
    let bytes = canic::topology::Install {
        root: crate::network::principal(root)?,
        parent: crate::network::principal(hub)?,
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
        .ok_or_else(|| Error::Invalid("pool component must have a declared parent hub".into()))
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
        if !(1..=stored::MAX_INSTANCES).contains(&self.max_instances)
            || self.expires_at_ns == Some(0)
        {
            return Err(Error::Invalid(
                "instance pool bounds or deadline outside the supported range".into(),
            ));
        }
        if let Some(other) = shared
            && (self.root != other.root
                || self.store != other.store
                || self.component == other.component
                || hub == parent(slots, &other.component)?)
        {
            return Err(Error::Invalid(
                "placement pools require one Root and Store and distinct children and hubs".into(),
            ));
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
    if !release.valid() || args.is_some_and(|p| p.as_os_str().is_empty()) {
        return Err(Error::Invalid(
            "invalid pool release label or install argument path".into(),
        ));
    }
    let hub = parent(slots, component)?;
    let mut names = std::collections::BTreeSet::new();
    for slot in [component, hub, root, store] {
        name(slot)?;
        if !slots.contains_key(slot) || !names.insert(slot) {
            return Err(Error::Invalid(
                "child, hub, Root and Store must be distinct declared components".into(),
            ));
        }
    }
    Ok(hub)
}
