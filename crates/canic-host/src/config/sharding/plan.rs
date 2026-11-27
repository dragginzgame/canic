//! Build the offline pool approval from the selected fleet and its exact Wasm inputs.
use super::{FleetConfig, PoolGrant, RegistryConfig, WorkerPoolConfig, parent, topology};
use crate::{
    DeploymentName, Error, FleetPlanning, Result, SelectedFleet, artifact, store, supplier,
};
use canic_management_protocol::{application::Arguments, stored};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

fn checked_wasm(selected: &SelectedFleet, config: &Path, component: &str) -> Result<Vec<u8>> {
    let bytes = store::read(
        &selected.wasm_path(config, component, None)?,
        artifact::MAX_ARTIFACT_BYTES,
    )?;
    check_pool_requirements(&bytes, component, selected)?;
    Ok(bytes)
}

fn manifest(
    selected: &SelectedFleet,
    config: &Path,
    component: &str,
    release: &ic_wasm_store_protocol::Release,
) -> Result<ic_wasm_store_protocol::Manifest> {
    let wasm = checked_wasm(selected, config, component)?;
    artifact::check_wasm_header_and_size(&wasm, artifact::MAX_ARTIFACT_BYTES)?;
    Ok(crate::publication::release_manifest(release.clone(), &wasm))
}

fn install_args(config: &Path, args: Option<&Path>) -> Result<Arguments> {
    let input = args
        .map(|path| {
            store::read(
                &config.with_file_name(path),
                canic_management_protocol::application::MAX_ARGUMENT_BYTES,
            )
        })
        .transpose()?
        .unwrap_or_default();
    Arguments::new(input).map_err(supplier)
}

struct PoolInstall<'a> {
    component: &'a str,
    root: &'a str,
    store: &'a str,
    release: &'a ic_wasm_store_protocol::Release,
}

impl PoolInstall<'_> {
    fn resolve(
        &self,
        selected: &SelectedFleet,
        config: &Path,
        args: &Arguments,
    ) -> Result<(String, String, stored::Reference, Arguments)> {
        let fleet = &selected.config;
        let manifest = manifest(selected, config, self.component, self.release)?;
        let (root, hub, store) = fleet.pool_bindings(self.component, self.root, self.store)?;
        let arguments = topology(&fleet.canisters, self.component, &root, &hub, args)?;
        let arguments =
            fleet.pool_receiver_arguments(self.component, self.root, &root, arguments)?;
        Ok((root, hub, stored::Reference { store, manifest }, arguments))
    }
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
    let fleet = &selected.config;
    let deadlines = check_inputs(selected, config_path)?;
    use canic::{method::Contract, sharding::ConfigureShardRoutes};
    let mut routes = BTreeMap::new();
    let mut increases = Vec::new();
    let mut shards = Vec::with_capacity(fleet.policy.shard_pools.len());
    for pool in &fleet.policy.shard_pools {
        let manifest = manifest(selected, config_path, &pool.component, &pool.release)?;
        let args = install_args(config_path, pool.install_args.as_deref())?;
        let approval = fleet.approve_shard_policy(pool, manifest, args)?;
        if let Some(old) = pool.increase_from {
            let mut prior = approval.clone();
            prior.max_shards = old;
            increases.push(grant(
                stored::IncreasePoolLimit::Shards(prior, approval.max_shards),
                stored::INCREASE_POOL_LIMIT,
            )?);
        }
        let input = (approval.pool.clone(), pool.keys_per_shard);
        routes.insert(
            approval.pool.clone(),
            PoolGrant {
                request: ConfigureShardRoutes::encode_request(&input).map_err(supplier)?,
                approval: input,
            },
        );
        shards.push(creation_grant(
            approval,
            stored::ChildCreationGrant::Shared,
        )?);
    }
    let instances = if let Some(pool) = &fleet.policy.registry {
        let instance = registry_grant(
            controller,
            pool,
            selected,
            config_path,
            deadlines[pool.component.as_str()],
        )?;
        if let Some(old) = pool.increase_from {
            let mut prior = instance.approval.clone();
            prior.max_instances = old;
            increases.push(grant(
                stored::IncreasePoolLimit::Instances(prior, instance.approval.max_instances),
                stored::INCREASE_POOL_LIMIT,
            )?);
        }
        Some(instance)
    } else {
        None
    };
    let workers = fleet
        .policy
        .worker_pools
        .iter()
        .map(|p| worker_grant(p, selected, config_path, deadlines[p.component.as_str()]))
        .collect::<Result<Vec<_>>>()?;
    let standby = fleet
        .standby_pool
        .as_ref()
        .map(|p| standby_grant(fleet, p))
        .transpose()?;
    let root = instances
        .as_ref()
        .or(workers.first())
        .map(|g| &g.approval.root)
        .or_else(|| standby.as_ref().map(|g| &g.approval.root))
        .or_else(|| shards.first().map(|g| &g.approval.root))
        .ok_or_else(|| Error::Invalid("no placement pools".into()))?;
    let max_children = fleet
        .policy
        .subnet_child_limit
        .unwrap_or(stored::DEFAULT_SUBNET_CHILD_LIMIT);
    let capacity = Some(grant(
        stored::SubnetCapacity {
            root: root.clone(),
            max_children,
            allow_large_subnet: fleet.policy.allow_large_subnet,
        },
        stored::CONFIGURE_SUBNET_CAPACITY,
    )?);

    if instances.is_none() && workers.is_empty() && standby.is_none() && shards.len() == 1 {
        let PoolGrant { approval, request } = shards.remove(0);
        return Ok(FleetPlanning::ShardPool {
            routes: routes
                .remove(&approval.pool)
                .expect("configured hub routes"),
            capacity,
            increases,
            request,
            deployment: DeploymentName {
                app: selected.app.clone(),
                fleet: selected.fleet.clone(),
                canister: approval.pool.clone(),
            },
            network: fleet.network.clone(),
            controller: controller.to_owned(),
            approval: Box::new(approval),
        });
    }
    let singleton = if let (Some(policy), Some(registry)) = (&fleet.policy.registry, &instances)
        && let Some(rule) = &policy.singleton
    {
        let receiver = fleet
            .receiver_selected(&rule.component)
            .then(|| {
                fleet.pool_receiver_arguments(
                    &rule.component,
                    &policy.root,
                    &registry.approval.root,
                    Arguments::new(Vec::new()).map_err(supplier)?,
                )
            })
            .transpose()?;
        let role = fleet.canisters[&rule.component]
            .role
            .clone()
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "singleton {:?} needs a declared role",
                    rule.component
                ))
            })?;
        super::super::positive_count("registry.singleton.maximum", rule.maximum, max_children)?;
        Some(Box::new(creation_grant(
            stored::SingletonGrant {
                receiver,
                registry: registry.approval.hub.clone(),
                role,
                reference: stored::Reference {
                    store: registry.approval.reference.store.clone(),
                    manifest: manifest(selected, config_path, &rule.component, &rule.release)?,
                },
                application: install_args(config_path, rule.install_args.as_deref())?,
                creation_budget: rule.creation_budget,
                maximum: rule.maximum,
                expires_at_ns: registry.approval.expires_at_ns,
            },
            stored::ChildCreationGrant::PerParent,
        )?))
    } else {
        None
    };
    Ok(FleetPlanning::PlacementPools {
        routes,
        singleton,
        capacity,
        standby,
        increases,
        app: selected.app.clone(),
        fleet: selected.fleet.clone(),
        network: fleet.network.clone(),
        controller: controller.to_owned(),
        shards,
        instances: instances.map(Box::new),
        workers,
    })
}

fn check_inputs<'a>(
    selected: &'a SelectedFleet,
    config_path: &Path,
) -> Result<BTreeMap<&'a str, u64>> {
    let fleet = &selected.config;
    // Grant construction checks each pooled component while hashing those same bytes.
    let now = crate::now()?;
    let mut deadlines = BTreeMap::new();
    let shards = fleet
        .policy
        .shard_pools
        .iter()
        .map(|p| ("shard_pool", &p.component, p.expires_at_ns));
    let registry = fleet
        .policy
        .registry
        .iter()
        .map(|p| ("registry", &p.component, p.expires_at_ns));
    let workers = fleet
        .policy
        .worker_pools
        .iter()
        .map(|p| ("worker_pool", &p.component, p.expires_at_ns));
    for (kind, name, deadline) in shards.chain(registry).chain(workers) {
        deadlines.insert(name.as_str(), check_deadline(kind, deadline, now)?);
    }
    for (name, slot) in &fleet.canisters {
        if slot.wasm.is_some()
            && !deadlines.contains_key(name.as_str())
            && !matches!(slot.role.as_deref(), Some("root" | "wasm-store"))
        {
            checked_wasm(selected, config_path, name)?;
        }
    }
    Ok(deadlines)
}

fn registry_grant(
    operator: &str,
    p: &RegistryConfig,
    selected: &SelectedFleet,
    config_path: &Path,
    expires_at_ns: u64,
) -> Result<PoolGrant<stored::InstancePool>> {
    let inner = install_args(config_path, p.install_args.as_deref())?;
    let args =
        selected
            .config
            .bind_hub_operator(&p.component, operator, inner.as_slice().to_vec())?;
    let (root, hub, reference, arguments) = PoolInstall {
        component: &p.component,
        root: &p.root,
        store: &p.store,
        release: &p.release,
    }
    .resolve(
        selected,
        config_path,
        &Arguments::new(args).map_err(supplier)?,
    )?;
    creation_grant(
        stored::InstancePool {
            root,
            hub,
            reference,
            arguments,
            component: p.component.clone(),
            max_instances: p.max_instances,
            creation_budget: p.creation_budget,
            expires_at_ns,
        },
        stored::ChildCreationGrant::Dedicated,
    )
}

fn worker_grant(
    p: &WorkerPoolConfig,
    selected: &SelectedFleet,
    config_path: &Path,
    expires_at_ns: u64,
) -> Result<PoolGrant<stored::InstancePool>> {
    if selected.config.receiver_selected(&p.component) {
        return Err(Error::Invalid(
            "worker pools do not support receiver installation".into(),
        ));
    }
    let args = match p.minimum_reuse_cycles {
        Some(minimum) => canic::topology::worker::reuse_policy(minimum).map_err(supplier)?,
        None => canic::topology::WORKER_GENERATION.to_vec(),
    };
    let (root, hub, reference, arguments) = PoolInstall {
        component: &p.component,
        root: &p.root,
        store: &p.store,
        release: &p.release,
    }
    .resolve(
        selected,
        config_path,
        &Arguments::new(args).map_err(supplier)?,
    )?;
    creation_grant(
        stored::InstancePool {
            root,
            hub,
            reference,
            arguments,
            component: p.component.clone(),
            max_instances: p.max_workers,
            creation_budget: p.creation_budget,
            expires_at_ns,
        },
        stored::ChildCreationGrant::Worker,
    )
}

fn standby_grant(
    fleet: &FleetConfig,
    p: &stored::StandbyPolicy,
) -> Result<PoolGrant<stored::StandbyPolicy>> {
    let binding = fleet
        .canisters
        .get(&p.root)
        .filter(|c| c.role.as_deref() == Some("root"))
        .and_then(|c| c.id.clone())
        .ok_or_else(|| {
            Error::Invalid(format!(
                "standby_pool.root = {:?} requires a bound Root",
                p.root
            ))
        })?;
    if let Some(import) = p
        .imports
        .iter()
        .find(|i| i.previous_root.as_ref() == Some(&binding))
    {
        return Err(Error::Invalid(format!(
            "standby_pool.imports previous_root = {binding} for {} matches the destination Root; previous_root must name a different Root for a handoff",
            import.principal
        )));
    }
    if p.imports.iter().any(|id| {
        fleet
            .canisters
            .values()
            .any(|c| c.id.as_ref() == Some(&id.principal))
    }) {
        return Err(Error::Invalid(
            "standby_pool.imports conflicts with a bound fleet canister".into(),
        ));
    }
    let mut policy = p.clone();
    policy.root = binding;
    grant(policy, stored::CONFIGURE_STANDBY)
}

/// Match the requirements declared by each built app canister to this fleet and hub.
pub(crate) fn check_pool_requirements(
    bytes: &[u8],
    hub: &str,
    fleet: &SelectedFleet,
) -> Result<()> {
    const PREFIX: &str = "canic:pool:";
    // Historical plans also admit opaque Wasm. Parse sections only for potential consumers.
    if !bytes
        .windows(PREFIX.len())
        .any(|window| window == PREFIX.as_bytes())
    {
        return Ok(());
    }
    for section in wasmparser::Parser::new(0).parse_all(bytes) {
        let wasmparser::Payload::CustomSection(section) = section.map_err(supplier)? else {
            continue;
        };
        let Some(binding) = section.name().strip_prefix(PREFIX) else {
            continue;
        };
        let (kind, component) = binding.split_once(':').ok_or_else(|| {
            Error::Invalid(format!("{hub} has malformed pool requirement {binding:?}"))
        })?;
        let declared = match kind {
            "shard" => fleet
                .config
                .policy
                .shard_pools
                .iter()
                .any(|pool| pool.component == component),
            "worker" => fleet
                .config
                .policy
                .worker_pools
                .iter()
                .any(|pool| pool.component == component),
            _ => false,
        };
        if !declared || parent(&fleet.config.canisters, component)? != hub {
            return Err(Error::Invalid(format!(
                "hub {hub:?} requires {kind} pool {component:?} in fleet {:?}; declare that pool with {hub:?} as its component's parent",
                fleet.fleet
            )));
        }
    }
    Ok(())
}

fn creation_grant<T: Clone>(
    approval: T,
    kind: fn(T) -> stored::ChildCreationGrant,
) -> Result<PoolGrant<T>> {
    use canic::method::Contract;
    let request =
        stored::ApproveChildCreation::encode_request(&kind(approval.clone())).map_err(supplier)?;
    Ok(PoolGrant { approval, request })
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
