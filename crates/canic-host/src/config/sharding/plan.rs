//! Build the offline pool approval from the selected fleet and its exact Wasm inputs.
use super::{PoolGrant, parent, topology};
use crate::{
    DeploymentName, Error, FleetPlanning, Result, SelectedFleet, artifact, store, supplier,
};
use canic_management_protocol::{application::Arguments, stored};
use serde::Serialize;
use std::{collections::BTreeSet, path::Path};

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
    // Grant construction checks each pooled component while hashing those same bytes.
    let pooled: BTreeSet<_> = fleet
        .shard_pools
        .iter()
        .map(|p| &p.component)
        .chain(fleet.registry.iter().map(|p| &p.component))
        .chain(fleet.worker_pools.iter().map(|p| &p.component))
        .collect();
    for (name, slot) in &fleet.canisters {
        if slot.wasm.is_some()
            && !pooled.contains(name)
            && !matches!(slot.role.as_deref(), Some("root" | "wasm-store"))
        {
            checked_wasm(selected, config_path, name)?;
        }
    }
    let now = crate::now()?;
    let shards = fleet
        .shard_pools
        .iter()
        .map(|pool| {
            check_deadline("shard_pool", pool.expires_at_ns, now)?;
            let manifest = manifest(selected, config_path, &pool.component, &pool.release)?;
            let args = install_args(config_path, pool.install_args.as_deref())?;
            grant(
                fleet.shard_pool_approval(&pool.component, manifest, args)?,
                stored::CONFIGURE_SHARDS,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let capacity = |root: &str| {
        fleet
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
    for (shard, old) in shards
        .iter()
        .zip(&fleet.shard_pools)
        .filter_map(|(grant, pool)| Some((grant, pool.increase_from?)))
    {
        let mut prior = shard.approval.clone();
        let next = std::mem::replace(&mut prior.max_shards, old);
        increases.push(grant(
            stored::IncreasePool::Shards(prior, next),
            stored::INCREASE_POOL,
        )?);
    }
    let bindings =
        |component: &str, root_name: &str, store_name: &str, release, args| -> Result<_> {
            let manifest = manifest(selected, config_path, component, release)?;
            let (root, hub, store) = fleet.pool_bindings(component, root_name, store_name)?;
            let arguments = topology(&fleet.canisters, component, &root, &hub, &args)?;
            let arguments =
                fleet.pool_receiver_arguments(component, root_name, &root, arguments)?;
            Ok((root, hub, stored::Reference { store, manifest }, arguments))
        };
    let instances = fleet
        .registry
        .as_ref()
        .map(|p| {
            let (root, hub, reference, arguments) = bindings(
                &p.component,
                &p.root,
                &p.store,
                &p.release,
                install_args(config_path, p.install_args.as_deref())?,
            )?;
            grant(
                stored::InstancePool {
                    root,
                    hub,
                    reference,
                    arguments,
                    component: p.component.clone(),
                    max_instances: p.max_instances,
                    creation_payer: p.creation_payer,
                    creation_budget: p.creation_budget,
                    expires_at_ns: check_deadline("registry", p.expires_at_ns, now)?,
                },
                stored::CONFIGURE_INSTANCES,
            )
        })
        .transpose()?;
    let workers = fleet
        .worker_pools
        .iter()
        .map(|p| {
            if fleet.receiver_selected(&p.component) {
                return Err(Error::Invalid(
                    "worker pools do not support receiver installation".into(),
                ));
            }
            let args = match p.minimum_reuse_cycles {
                Some(minimum) => {
                    canic::topology::worker::reuse_policy(minimum).map_err(supplier)?
                }
                None => canic::topology::WORKER_GENERATION.to_vec(),
            };
            let (root, hub, reference, arguments) = bindings(
                &p.component,
                &p.root,
                &p.store,
                &p.release,
                Arguments::new(args).map_err(supplier)?,
            )?;
            grant(
                stored::InstancePool {
                    root,
                    hub,
                    reference,
                    arguments,
                    component: p.component.clone(),
                    max_instances: p.max_workers,
                    creation_payer: p.creation_payer,
                    creation_budget: p.creation_budget,
                    expires_at_ns: check_deadline("worker_pool", p.expires_at_ns, now)?,
                },
                stored::CONFIGURE_WORKERS,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let standby = fleet
        .standby_pool
        .as_ref()
        .map(|p| {
            let mut policy = p.clone();
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
            if let Some(import) = p.imports.iter().find(|i| i.previous_root.as_ref() == Some(&binding)) {
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
            policy.root = binding;
            grant(policy, stored::CONFIGURE_STANDBY)
        })
        .transpose()?;
    if let Some(root) = instances
        .as_ref()
        .or(workers.first())
        .map(|g| &g.approval.root)
        .or_else(|| standby.as_ref().map(|g| &g.approval.root))
        .or_else(|| (shards.len() > 1).then(|| &shards[0].approval.root))
    {
        if let Some((instance, old)) = instances
            .as_ref()
            .zip(fleet.registry.as_ref().and_then(|p| p.increase_from))
        {
            let mut prior = instance.approval.clone();
            let next = std::mem::replace(&mut prior.max_instances, old);
            increases.push(grant(
                stored::IncreasePool::Instances(prior, next),
                stored::INCREASE_POOL,
            )?);
        }
        return Ok(FleetPlanning::PlacementPools {
            capacity: capacity(root)?,
            standby,
            increases,
            app: selected.app.clone(),
            fleet: selected.fleet.clone(),
            network: fleet.network.clone(),
            controller: controller.to_owned(),
            shards,
            instances: instances.map(Box::new),
            workers,
        });
    }
    let PoolGrant { approval, request } = shards
        .into_iter()
        .next()
        .ok_or_else(|| Error::Invalid("no placement pools".into()))?;
    Ok(FleetPlanning::ShardPool {
        capacity: capacity(&approval.root)?,
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
    })
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
                .shard_pools
                .iter()
                .any(|pool| pool.component == component),
            "worker" => fleet
                .config
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
