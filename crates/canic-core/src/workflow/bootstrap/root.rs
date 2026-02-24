//! Root bootstrap phase.
//!
//! This module defines the asynchronous bootstrap phase for the root canister.
//! It runs after runtime initialization and is responsible for all
//! cross-canister orchestration, topology creation, and reconciliation.

use crate::{
    InternalError,
    config::schema::SubnetConfig,
    dto::pool::CanisterPoolStatus,
    dto::validation::{ValidationIssue, ValidationReport},
    ids::BuildNetwork,
    ops::{
        config::ConfigOps,
        ic::{IcOps, network::NetworkOps},
        runtime::env::EnvOps,
        runtime::ready::ReadyOps,
        runtime::wasm::WasmOps,
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            pool::PoolOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        ic::{IcWorkflow, provision::ProvisionWorkflow},
        pool::{PoolWorkflow, query::PoolQuery},
        prelude::*,
        topology::guard::TopologyGuard,
    },
};
use std::collections::BTreeMap;

///
/// RootBootstrapContext
///

struct RootBootstrapContext {
    subnet_cfg: SubnetConfig,
    network: Option<BuildNetwork>,
}

impl RootBootstrapContext {
    fn load() -> Result<Self, InternalError> {
        let subnet_cfg = ConfigOps::current_subnet()?;
        let network = NetworkOps::build_network();

        Ok(Self {
            subnet_cfg,
            network,
        })
    }
}

/// ---------------------------------------------------------------------------
/// Root bootstrap entrypoints
/// ---------------------------------------------------------------------------

pub async fn bootstrap_init_root_canister() {
    if let Err(err) = WasmOps::require_initialized() {
        log!(Topic::Init, Error, "bootstrap (root:init) aborted: {err}");
        return;
    }

    let _guard = match TopologyGuard::try_enter() {
        Ok(g) => g,
        Err(err) => {
            log!(Topic::Init, Info, "bootstrap (root:init) skipped: {err}");
            return;
        }
    };

    log!(Topic::Init, Info, "bootstrap (root:init) start");

    // On fresh init, wait for configured pool imports before auto-create.
    // This avoids creating new canisters while reserve imports are still pending.
    root_import_pool_from_config(true).await;

    if let Err(err) = root_create_canisters().await {
        log!(Topic::Init, Error, "registry phase failed: {err}");
        return;
    }

    if let Err(err) = root_rebuild_directories_from_registry() {
        log!(
            Topic::Init,
            Error,
            "directory materialization failed: {err}"
        );
        return;
    }

    let report = root_validate_state();
    if !report.ok {
        log!(
            Topic::Init,
            Error,
            "bootstrap validation failed:\n{:#?}",
            report.issues
        );
        return;
    }

    log!(Topic::Init, Info, "bootstrap (root:init) complete");
    ReadyOps::mark_ready(super::ready_token());
}

/// Bootstrap workflow for the root canister after upgrade.
pub async fn bootstrap_post_upgrade_root_canister() {
    if let Err(err) = WasmOps::require_initialized() {
        log!(
            Topic::Init,
            Error,
            "bootstrap (root:upgrade) aborted: {err}"
        );
        return;
    }

    // Environment already exists; only enrich + reconcile
    log!(Topic::Init, Info, "bootstrap (root:upgrade) start");
    root_set_subnet_id().await;
    // Keep post-upgrade non-blocking; queued imports continue in background.
    root_import_pool_from_config(false).await;
    log!(Topic::Init, Info, "bootstrap (root:upgrade) complete");
    ReadyOps::mark_ready(super::ready_token());
}

/// Resolve and persist the subnet identifier for the root canister.
///
/// On IC:
/// - Failure to resolve subnet ID is fatal.
///
/// On local / test networks:
/// - Falls back to `canister_self()` deterministically.
pub async fn root_set_subnet_id() {
    let network = NetworkOps::build_network();

    match IcWorkflow::try_get_current_subnet_pid().await {
        Ok(Some(subnet_pid)) => {
            EnvOps::set_subnet_pid(subnet_pid);
            return;
        }

        Ok(None) => {
            if network == Some(BuildNetwork::Ic) {
                let msg = "try_get_current_subnet_pid returned None on ic; refusing to fall back";
                log!(Topic::Topology, Error, "{msg}");
                return;
            }
        }

        Err(err) => {
            if network == Some(BuildNetwork::Ic) {
                let msg = format!("try_get_current_subnet_pid failed on ic: {err}");
                log!(Topic::Topology, Error, "{msg}");
                return;
            }
        }
    }

    // Fallback path for non-IC environments
    let fallback = IcOps::canister_self();
    EnvOps::set_subnet_pid(fallback);

    log!(
        Topic::Topology,
        Info,
        "try_get_current_subnet_pid unavailable; using self as subnet: {fallback}"
    );
}

/// ---------------------------------------------------------------------------
/// Pool bootstrap
/// ---------------------------------------------------------------------------

/// Import any statically configured pool canisters for this subnet.
///
/// Failures are summarized so bootstrap can continue.
pub async fn root_import_pool_from_config(wait_for_queued_imports: bool) {
    let data = match RootBootstrapContext::load() {
        Ok(data) => data,
        Err(err) => {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import skipped: missing subnet config ({err})"
            );
            return;
        }
    };

    ensure_pool_imported(&data, wait_for_queued_imports).await;
}

/// ---------------------------------------------------------------------------
/// Canister creation
/// ---------------------------------------------------------------------------

/// Ensure all statically configured canisters for this subnet exist.
pub async fn root_create_canisters() -> Result<(), InternalError> {
    let data = RootBootstrapContext::load()?;

    log!(
        Topic::Init,
        Info,
        "auto_create roles: {:?}",
        data.subnet_cfg.auto_create
    );

    ensure_required_canisters(&data).await
}

pub fn root_rebuild_directories_from_registry() -> Result<(), InternalError> {
    let _ = ProvisionWorkflow::rebuild_directories_from_registry(None)?;

    Ok(())
}

#[expect(clippy::too_many_lines)]
async fn ensure_pool_imported(data: &RootBootstrapContext, wait_for_queued_imports: bool) {
    let initial_cfg = data
        .subnet_cfg
        .pool
        .import
        .initial
        .map_or_else(|| "unset".to_string(), |v| v.to_string());

    let import_list = match data.network {
        Some(BuildNetwork::Local) => data.subnet_cfg.pool.import.local.clone(),
        Some(BuildNetwork::Ic) => data.subnet_cfg.pool.import.ic.clone(),
        None => {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import skipped: build network not set"
            );
            return;
        }
    };

    let initial_limit = data
        .subnet_cfg
        .pool
        .import
        .initial
        .map_or(data.subnet_cfg.pool.minimum_size as usize, |count| {
            count as usize
        });

    log!(
        Topic::CanisterPool,
        Info,
        "pool import config: network={} minimum_size={} import.initial={} resolved_initial_limit={} wait_for_queued={}",
        data.network.map_or("unknown", BuildNetwork::as_str),
        data.subnet_cfg.pool.minimum_size,
        initial_cfg,
        initial_limit,
        wait_for_queued_imports
    );

    if !import_list.is_empty() {
        log!(
            Topic::CanisterPool,
            Info,
            "pool import candidates={} pids={}",
            import_list.len(),
            summarize_principals(&import_list, 12)
        );
    }

    if initial_limit == 0 && !data.subnet_cfg.auto_create.is_empty() {
        log!(
            Topic::CanisterPool,
            Warn,
            "pool import initial=0 with auto_create enabled; canisters may be created before queued imports are ready"
        );
    }

    if import_list.is_empty() {
        log!(
            Topic::CanisterPool,
            Warn,
            "pool import skipped: selected import list is empty for network={}",
            data.network.map_or("unknown", BuildNetwork::as_str)
        );
        log_pool_stats("after-empty-import-skip", data.subnet_cfg.pool.minimum_size);
        return;
    }

    let (initial, queued) = import_list.split_at(initial_limit.min(import_list.len()));
    let configured_initial = initial.len() as u64;
    let configured_queued = queued.len() as u64;

    let mut imported = 0_u64;
    let mut immediate_skipped = 0_u64;
    let mut immediate_failed = 0_u64;
    let mut immediate_already_present = 0_u64;

    let mut queued_added = 0_u64;
    let mut queued_requeued = 0_u64;
    let mut queued_skipped = 0_u64;
    let mut queued_failed = 0_u64;
    let mut queued_already_present = 0_u64;

    let mut immediate_imported_pids = Vec::new();
    let mut immediate_skipped_pids = Vec::new();
    let mut immediate_failed_pids = Vec::new();
    let mut immediate_present_pids = Vec::new();

    let mut queued_added_pids = Vec::new();
    let mut queued_skipped_pids = Vec::new();
    let mut queued_failed_pids = Vec::new();
    let mut queued_present_pids = Vec::new();

    for pid in initial {
        if PoolOps::contains(pid) {
            immediate_already_present += 1;
            immediate_present_pids.push(*pid);
            continue;
        }

        if matches!(PoolWorkflow::pool_import_canister(*pid).await, Ok(())) {
            if PoolOps::contains(pid) {
                imported += 1;
                immediate_imported_pids.push(*pid);
            } else {
                immediate_skipped += 1;
                immediate_skipped_pids.push(*pid);
            }
        } else {
            immediate_failed += 1;
            immediate_failed_pids.push(*pid);
        }
    }

    let queued_imports: Vec<Principal> = queued
        .iter()
        .copied()
        .filter(|pid| {
            if PoolOps::contains(pid) {
                queued_already_present += 1;
                queued_present_pids.push(*pid);
                false
            } else {
                true
            }
        })
        .collect();

    if !queued_imports.is_empty() {
        if wait_for_queued_imports {
            for pid in queued_imports {
                if matches!(PoolWorkflow::pool_import_canister(pid).await, Ok(())) {
                    if PoolOps::contains(&pid) {
                        queued_added += 1;
                        queued_added_pids.push(pid);
                    } else {
                        queued_skipped += 1;
                        queued_skipped_pids.push(pid);
                    }
                } else {
                    queued_failed += 1;
                    queued_failed_pids.push(pid);
                }
            }
        } else {
            log!(
                Topic::CanisterPool,
                Info,
                "pool import queued async candidates={} pids={}",
                queued_imports.len(),
                summarize_principals(&queued_imports, 12)
            );
            match PoolWorkflow::pool_import_queued_canisters(queued_imports).await {
                Ok(result) => {
                    queued_added = result.added;
                    queued_requeued = result.requeued;
                    queued_skipped = result.skipped;
                }
                Err(err) => {
                    queued_failed = configured_queued - queued_already_present;
                    log!(Topic::CanisterPool, Warn, "pool import queue failed: {err}");
                }
            }
        }
    }

    log!(
        Topic::CanisterPool,
        Info,
        "pool import immediate summary: configured={}, imported={imported}, skipped={immediate_skipped}, failed={immediate_failed}, present={immediate_already_present}",
        configured_initial
    );
    log!(
        Topic::CanisterPool,
        Info,
        "pool import immediate pids: imported={} skipped={} failed={} present={}",
        summarize_principals(&immediate_imported_pids, 12),
        summarize_principals(&immediate_skipped_pids, 12),
        summarize_principals(&immediate_failed_pids, 12),
        summarize_principals(&immediate_present_pids, 12),
    );

    if configured_queued > 0 {
        if queued_failed > 0 {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import queued summary: configured={}, failed={queued_failed}, present={queued_already_present}",
                configured_queued
            );
        } else {
            log!(
                Topic::CanisterPool,
                Info,
                "pool import queued summary: configured={}, added={queued_added}, requeued={queued_requeued}, skipped={queued_skipped}, present={queued_already_present}",
                configured_queued
            );
        }

        if wait_for_queued_imports {
            log!(
                Topic::CanisterPool,
                Info,
                "pool import queued pids: added={} skipped={} failed={} present={}",
                summarize_principals(&queued_added_pids, 12),
                summarize_principals(&queued_skipped_pids, 12),
                summarize_principals(&queued_failed_pids, 12),
                summarize_principals(&queued_present_pids, 12),
            );
        } else {
            log!(
                Topic::CanisterPool,
                Info,
                "pool import queued pids (best-effort): present={} (added/requeued/skipped resolved by scheduler)",
                summarize_principals(&queued_present_pids, 12),
            );
        }
    }

    log_pool_stats("after-import", data.subnet_cfg.pool.minimum_size);
}

async fn ensure_required_canisters(data: &RootBootstrapContext) -> Result<(), InternalError> {
    for role in &data.subnet_cfg.auto_create {
        // ALWAYS re-check live registry
        if SubnetRegistryOps::has_role(role) {
            log!(
                Topic::Init,
                Info,
                "auto_create: {role} already present in registry, skipping"
            );
            continue;
        }

        log!(Topic::Init, Info, "auto_create: creating {role}");

        CanisterLifecycleWorkflow::apply(CanisterLifecycleEvent::Create {
            role: role.clone(),
            parent: IcOps::canister_self(),
            extra_arg: None,
        })
        .await?;
    }

    Ok(())
}

pub fn root_validate_state() -> ValidationReport {
    let app_data = AppDirectoryOps::data();
    let subnet_data = SubnetDirectoryOps::data();

    let mut issues = Vec::new();

    let env_missing = EnvOps::missing_required_fields();
    let env_complete = env_missing.is_empty();
    if !env_complete {
        issues.push(ValidationIssue {
            code: "env_missing_fields".to_string(),
            message: format!("missing env fields: {}", env_missing.join(", ")),
        });
    }

    let registry_roles = SubnetRegistryOps::role_index();

    let (app_unique, app_consistent) = check_directory(
        "app_directory",
        &app_data.entries,
        &registry_roles,
        &mut issues,
    );
    let (subnet_unique, subnet_consistent) = check_directory(
        "subnet_directory",
        &subnet_data.entries,
        &registry_roles,
        &mut issues,
    );

    let unique_directory_roles = app_unique && subnet_unique;
    let registry_directory_consistent = app_consistent && subnet_consistent;
    let ok = env_complete && unique_directory_roles && registry_directory_consistent;

    ValidationReport {
        ok,
        registry_directory_consistent,
        unique_directory_roles,
        env_complete,
        issues,
    }
}

fn check_directory(
    label: &str,
    entries: &[(CanisterRole, Principal)],
    registry_roles: &BTreeMap<CanisterRole, Vec<Principal>>,
    issues: &mut Vec<ValidationIssue>,
) -> (bool, bool) {
    let mut unique = true;
    let mut consistent = true;
    let mut seen = BTreeMap::<CanisterRole, usize>::new();

    for (role, pid) in entries {
        let count = seen.entry(role.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            unique = false;
            issues.push(ValidationIssue {
                code: "directory_role_duplicate".to_string(),
                message: format!("{label} has duplicate role {role}"),
            });
        }

        match registry_roles.get(role) {
            None => {
                consistent = false;
                issues.push(ValidationIssue {
                    code: "directory_role_missing_in_registry".to_string(),
                    message: format!("{label} role {role} not present in registry"),
                });
            }
            Some(pids) if pids.len() > 1 => {
                consistent = false;
                issues.push(ValidationIssue {
                    code: "directory_role_duplicate_in_registry".to_string(),
                    message: format!(
                        "{label} role {role} has multiple registry entries ({})",
                        pids.len()
                    ),
                });
            }
            Some(pids) => {
                if pids[0] != *pid {
                    consistent = false;
                    issues.push(ValidationIssue {
                        code: "directory_role_pid_mismatch".to_string(),
                        message: format!(
                            "{label} role {role} points to {pid}, registry has {}",
                            pids[0]
                        ),
                    });
                }
            }
        }
    }

    (unique, consistent)
}

fn summarize_principals(pids: &[Principal], limit: usize) -> String {
    if pids.is_empty() {
        return "[]".to_string();
    }

    let shown: Vec<String> = pids.iter().take(limit).map(ToString::to_string).collect();
    let remaining = pids.len().saturating_sub(shown.len());

    if remaining == 0 {
        format!("[{}]", shown.join(", "))
    } else {
        format!("[{} ... +{remaining} more]", shown.join(", "))
    }
}

fn log_pool_stats(stage: &str, minimum_size: u8) {
    let snapshot = PoolQuery::pool_list();
    let mut ready = 0_usize;
    let mut pending = 0_usize;
    let mut failed = 0_usize;
    let mut pending_pids = Vec::new();
    let mut failed_pids = Vec::new();

    for entry in snapshot.entries {
        match entry.status {
            CanisterPoolStatus::Ready => {
                ready += 1;
            }
            CanisterPoolStatus::PendingReset => {
                pending += 1;
                pending_pids.push(entry.pid);
            }
            CanisterPoolStatus::Failed { .. } => {
                failed += 1;
                failed_pids.push(entry.pid);
            }
        }
    }

    let total = ready + pending + failed;
    log!(
        Topic::CanisterPool,
        Info,
        "pool stats ({stage}): total={total}, ready={ready}, pending_reset={pending}, failed={failed}, minimum_size={minimum_size}",
    );

    if ready < minimum_size as usize {
        log!(
            Topic::CanisterPool,
            Warn,
            "pool ready below minimum_size ({stage}): ready={ready}, minimum_size={minimum_size}",
        );
    }

    if pending > 0 {
        log!(
            Topic::CanisterPool,
            Info,
            "pool pending_reset pids: {}",
            summarize_principals(&pending_pids, 12)
        );
    }

    if failed > 0 {
        log!(
            Topic::CanisterPool,
            Warn,
            "pool failed pids: {}",
            summarize_principals(&failed_pids, 12)
        );
    }
}
