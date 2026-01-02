use crate::{
    Error,
    cdk::api::{canister_self, trap},
    dto::subnet::{SubnetContextParams, SubnetIdentity},
    ids::{CanisterRole, SubnetRole},
    ops::{
        config::ConfigOps,
        config::network::{Network, build_network},
        runtime::env::{EnvOps, EnvSnapshot},
        storage::{
            directory::subnet::SubnetDirectoryOps, pool::PoolOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{
        ic::network::try_get_current_subnet_pid,
        orchestrator::{CanisterLifecycleOrchestrator, LifecycleEvent},
        pool::{pool_import_canister, pool_import_queued_canisters},
        prelude::*,
    },
};

/// ---------------------------------------------------------------------------
/// Root bootstrap entrypoints
/// ---------------------------------------------------------------------------

/// Bootstrap workflow for the root canister during init.
pub async fn bootstrap_init_root_canister() -> Result<(), Error> {
    // 1. Resolve runtime identity
    let identity = resolve_root_subnet_identity().await?;

    // 2. Initialize canonical environment
    root_init_env(identity)?;

    // 3. Bootstrap dependent subsystems
    root_import_pool_from_config().await;
    root_create_canisters().await?;

    Ok(())
}

/// Bootstrap workflow for the root canister after upgrade.
pub async fn bootstrap_post_upgrade_root_canister() {
    // Environment already exists; only enrich + reconcile
    let _ = root_set_subnet_id().await;
    root_import_pool_from_config().await;
}

/// ---------------------------------------------------------------------------
/// Environment initialization
/// ---------------------------------------------------------------------------

/// Initialize canonical environment state for the root canister.
///
/// Must be called exactly once during the IC `init` hook.
pub fn root_init_env(identity: SubnetIdentity) -> Result<(), Error> {
    let self_pid = canister_self();

    let (subnet_pid, subnet_role, prime_root_pid) = match identity {
        SubnetIdentity::Prime => {
            // Prime subnet: root == prime root == subnet
            (self_pid, SubnetRole::PRIME, self_pid)
        }

        SubnetIdentity::Standard(params) => {
            // Standard subnet syncing from prime
            (self_pid, params.subnet_type, params.prime_root_pid)
        }

        SubnetIdentity::Manual(pid) => {
            // Test/support only: explicit subnet override
            (pid, SubnetRole::MANUAL, pid)
        }
    };

    let snapshot = EnvSnapshot {
        prime_root_pid: Some(prime_root_pid),
        root_pid: Some(self_pid),
        subnet_pid: Some(subnet_pid),
        subnet_role: Some(subnet_role),
        canister_role: Some(CanisterRole::ROOT),
        parent_pid: Some(prime_root_pid),
    };

    EnvOps::import(snapshot)
}

/// ---------------------------------------------------------------------------
/// Environment enrichment
/// ---------------------------------------------------------------------------

pub async fn resolve_root_subnet_identity() -> Result<SubnetIdentity, Error> {
    // Attempt runtime discovery via registry
    match try_get_current_subnet_pid().await {
        Ok(Some(subnet_pid)) => {
            // If subnet pid == self, this is the prime root
            if subnet_pid == canister_self() {
                Ok(SubnetIdentity::Prime)
            } else {
                // Standard subnet; prime root must already be known
                let prime_root_pid = EnvOps::prime_root_pid()?;
                let subnet_role = EnvOps::subnet_role()?; // or derive if needed

                Ok(SubnetIdentity::Standard(SubnetContextParams {
                    subnet_type: subnet_role,
                    prime_root_pid,
                }))
            }
        }

        // Registry unavailable → test / local
        Ok(None) | Err(_) => Ok(SubnetIdentity::Manual(canister_self())),
    }
}

/// Resolve and persist the subnet identifier for the root canister.
///
/// On IC:
/// - Failure to resolve subnet ID is fatal.
///
/// On local / test networks:
/// - Falls back to `canister_self()` deterministically.
pub async fn root_set_subnet_id() -> Result<(), Error> {
    let network = build_network();

    match try_get_current_subnet_pid().await {
        Ok(Some(subnet_pid)) => {
            EnvOps::set_subnet_pid(subnet_pid);
            return Ok(());
        }

        Ok(None) => {
            if network == Some(Network::Ic) {
                let msg = "try_get_current_subnet_pid returned None on ic; refusing to fall back";
                log!(Topic::Topology, Error, "{msg}");
                trap(msg);
            }
        }

        Err(err) => {
            if network == Some(Network::Ic) {
                let msg = format!("try_get_current_subnet_pid failed on ic: {err}");
                log!(Topic::Topology, Error, "{msg}");
                trap(&msg);
            }
        }
    }

    // Fallback path for non-IC environments
    let fallback = canister_self();
    EnvOps::set_subnet_pid(fallback);

    log!(
        Topic::Topology,
        Info,
        "try_get_current_subnet_pid unavailable; using self as subnet: {fallback}"
    );

    Ok(())
}

/// ---------------------------------------------------------------------------
/// Pool bootstrap
/// ---------------------------------------------------------------------------

/// Import any statically configured pool canisters for this subnet.
///
/// Failures are summarized so bootstrap can continue.
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

    let initial_limit = subnet_cfg
        .pool
        .import
        .initial
        .map_or(subnet_cfg.pool.minimum_size as usize, |count| {
            count as usize
        });

    if initial_limit == 0 && !subnet_cfg.auto_create.is_empty() {
        log!(
            Topic::CanisterPool,
            Warn,
            "pool import initial=0 with auto_create enabled; canisters may be created before queued imports are ready"
        );
    }

    if import_list.is_empty() {
        return;
    }

    let (initial, queued) = import_list.split_at(initial_limit.min(import_list.len()));

    let mut imported = 0_u64;
    let mut immediate_skipped = 0_u64;
    let mut immediate_failed = 0_u64;

    let mut queued_added = 0_u64;
    let mut queued_requeued = 0_u64;
    let mut queued_skipped = 0_u64;
    let mut queued_failed = 0_u64;

    for pid in initial {
        match pool_import_canister(*pid).await {
            Ok(()) => {
                if PoolOps::contains(pid) {
                    imported += 1;
                } else {
                    immediate_skipped += 1;
                }
            }
            Err(_) => immediate_failed += 1,
        }
    }

    if !queued.is_empty() {
        match pool_import_queued_canisters(queued.to_vec()).await {
            Ok(result) => {
                queued_added = result.added;
                queued_requeued = result.requeued;
                queued_skipped = result.skipped;
            }
            Err(err) => {
                queued_failed = queued.len() as u64;
                log!(Topic::CanisterPool, Warn, "pool import queue failed: {err}");
            }
        }
    }

    log!(
        Topic::CanisterPool,
        Info,
        "pool import immediate summary: configured={}, imported={imported}, skipped={immediate_skipped}, failed={immediate_failed}",
        initial.len()
    );

    if !queued.is_empty() {
        if queued_failed > 0 {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import queued summary: configured={}, failed={queued_failed}",
                queued.len()
            );
        } else {
            log!(
                Topic::CanisterPool,
                Info,
                "pool import queued summary: configured={}, added={queued_added}, requeued={queued_requeued}, skipped={queued_skipped}",
                queued.len()
            );
        }
    }
}

/// ---------------------------------------------------------------------------
/// Canister creation
/// ---------------------------------------------------------------------------

/// Ensure all statically configured canisters for this subnet exist.
pub async fn root_create_canisters() -> Result<(), Error> {
    let subnet_cfg = ConfigOps::current_subnet()?;

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

    // Emit topology summary
    for (pid, role) in SubnetRegistryOps::export_roles() {
        log!(Topic::Init, Info, "🥫 {} ({})", role, pid);
    }

    Ok(())
}
