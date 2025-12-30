use crate::{
    Error,
    cdk::api::{canister_self, trap},
    infra::ic::{Network, build_network},
    log::Topic,
    ops::{
        config::ConfigOps,
        prelude::*,
        runtime::env::EnvOps,
        storage::{directory::SubnetDirectoryOps, pool::PoolOps, registry::SubnetRegistryOps},
    },
    workflow::{
        ic::network::try_get_current_subnet_pid,
        orchestrator::{CanisterLifecycleOrchestrator, LifecycleEvent},
        pool::pool_import_canister,
    },
};

/// Bootstrap workflow for the root canister during init.
pub async fn bootstrap_init_root_canister() -> Result<(), Error> {
    // Example sequence — adjust as needed
    root_set_subnet_id().await;
    root_import_pool_from_config().await;

    root_create_canisters().await?;

    Ok(())
}

/// Bootstrap workflow for the root canister after upgrade.
pub async fn bootstrap_post_upgrade_root_canister() {
    root_set_subnet_id().await;
    root_import_pool_from_config().await;
}

/// Initializes the subnet identifier for the root canister.
///
/// This attempts to resolve the subnet ID via the NNS registry and records it
/// into durable environment state. This value is required by downstream
/// topology, placement, and orchestration logic.
///
/// If the registry is unavailable (e.g. PocketIC or local testing), the
/// canister's own principal is used as a deterministic fallback.
pub async fn root_set_subnet_id() {
    // Preferred path: query the NNS registry for the subnet this canister
    // currently belongs to.
    let subnet_result = try_get_current_subnet_pid().await;
    match subnet_result {
        Ok(Some(subnet_pid)) => {
            EnvOps::set_subnet_pid(subnet_pid);
            return;
        }

        Ok(None) => {
            if build_network() == Some(Network::Ic) {
                let msg = "try_get_current_subnet_pid returned None on ic; refusing to fall back";
                log!(Topic::Topology, Error, "{msg}");
                trap(msg);
            }
        }

        Err(err) => {
            if build_network() == Some(Network::Ic) {
                let msg = format!("try_get_current_subnet_pid failed on ic: {err}");
                log!(Topic::Topology, Error, "{msg}");
                trap(&msg);
            }
        }
    }

    // Fallback path: environments without a registry (e.g. PocketIC).
    // Using self ensures a stable, non-null subnet identifier.
    let fallback = canister_self();
    EnvOps::set_subnet_pid(fallback);

    log!(
        Topic::Topology,
        Info,
        "try_get_current_subnet_pid unavailable; using self as subnet: {fallback}"
    );
}

/// Import any statically configured pool canisters for this subnet.
///
/// Import failures are summarized so bootstrap can continue.
pub async fn root_import_pool_from_config() {
    let subnet_cfg = match ConfigOps::current_subnet() {
        Ok(cfg) => cfg,
        Err(err) => {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import skipped: missing subnet config ({err})"
            );
            return;
        }
    };
    let import_list = match build_network() {
        Some(Network::Local) => subnet_cfg.pool.import.local,
        Some(Network::Ic) => subnet_cfg.pool.import.ic,
        None => {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import skipped: build network not set"
            );
            return;
        }
    };

    if import_list.is_empty() {
        return;
    }

    let mut attempted = 0_u64;
    let mut imported = 0_u64;
    let mut skipped = 0_u64;
    let mut failed = 0_u64;

    for pid in import_list {
        attempted += 1;
        match pool_import_canister(pid).await {
            Ok(()) => {
                if PoolOps::contains(&pid) {
                    imported += 1;
                } else {
                    skipped += 1;
                }
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    log!(
        Topic::CanisterPool,
        Info,
        "pool import summary: configured={attempted}, imported={imported}, skipped={skipped}, failed={failed}"
    );
}

/// Ensures all statically configured canisters for this subnet exist.
///
/// This function:
/// - Reads the subnet configuration
/// - Issues creation requests for any auto-create roles
/// - Emits a summary of the resulting topology
///
/// Intended to run during root bootstrap or upgrade flows.
/// Safe to re-run: skips roles that already exist in the subnet registry.
pub async fn root_create_canisters() -> Result<(), Error> {
    // Load the effective configuration for the current subnet.
    let subnet_cfg = ConfigOps::current_subnet()?;

    // Creation pass: ensure all auto-create canister roles exist.
    for role in &subnet_cfg.auto_create {
        if let Some(pid) = SubnetDirectoryOps::get(role) {
            log!(
                Topic::Init,
                Info,
                "auto_create: {role} already registered as {pid}, skipping"
            );
            continue;
        }

        CanisterLifecycleOrchestrator::apply(LifecycleEvent::Create {
            role: role.clone(),
            parent: canister_self(),
            extra_arg: None,
        })
        .await?;
    }

    // Reporting pass: emit the current topology for observability/debugging.
    for (pid, role) in SubnetRegistryOps::export_roles() {
        log!(Topic::Init, Info, "🥫 {} ({})", role, pid);
    }

    Ok(())
}
