//! Root bootstrap phase.
//!
//! This module defines the asynchronous bootstrap phase for the root canister.
//! It runs after runtime initialization and is responsible for all
//! cross-canister orchestration, topology creation, and reconciliation.

use crate::{
    ids::{BuildNetwork, CanisterRole},
    ops::storage::template::{TemplateChunkedOps, TemplateManifestOps},
    workflow::runtime::template::WasmStorePublicationWorkflow,
};
use canic_core::api::lifecycle::metrics::{
    LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
};
use canic_core::api::runtime::install::ModuleSourceRuntimeApi;
use canic_core::{__control_plane_core as cp_core, log, log::Topic};
use cp_core::{
    InternalError,
    config::schema::SubnetConfig,
    dto::{
        pool::CanisterPoolStatus,
        validation::{ValidationIssue, ValidationReport},
    },
    ops::{
        config::ConfigOps,
        ic::{IcOps, network::NetworkOps},
        runtime::{bootstrap::BootstrapStatusOps, env::EnvOps, ready::ReadyOps},
        storage::{
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
            pool::PoolOps,
            registry::{app::AppRegistryOps, subnet::SubnetRegistryOps},
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

fn root_has_embedded_wasm_store_bootstrap() -> bool {
    ModuleSourceRuntimeApi::has_embedded_module_source(&CanisterRole::WASM_STORE)
}

fn root_missing_staged_release_roles(
    data: &RootBootstrapContext,
) -> Result<Vec<CanisterRole>, InternalError> {
    let mut missing = Vec::new();

    for role in &data.subnet_cfg.auto_create {
        if role.is_wasm_store() {
            continue;
        }

        if !TemplateChunkedOps::has_publishable_chunked_approved_for_role(role)? {
            missing.push(role.clone());
        }
    }

    Ok(missing)
}

fn validation_failure_summary(report: &ValidationReport) -> String {
    report.issues.first().map_or_else(
        || "bootstrap validation failed".to_string(),
        |issue| format!("bootstrap validation failed: {}", issue.message),
    )
}

fn record_root_bootstrap_metric(phase: LifecycleMetricPhase, outcome: LifecycleMetricOutcome) {
    LifecycleMetricsApi::record_bootstrap(phase, LifecycleMetricRole::Root, outcome);
}

fn mark_root_bootstrap_failed(phase: LifecycleMetricPhase, message: String) {
    record_root_bootstrap_metric(phase, LifecycleMetricOutcome::Failed);
    BootstrapStatusOps::mark_failed(message);
}

pub async fn bootstrap_init_root_canister() {
    record_root_bootstrap_metric(LifecycleMetricPhase::Init, LifecycleMetricOutcome::Started);

    if !root_has_embedded_wasm_store_bootstrap() {
        let message =
            "bootstrap (root:init) embedded wasm_store bootstrap module is not registered";
        mark_root_bootstrap_failed(LifecycleMetricPhase::Init, message.to_string());
        log!(Topic::Init, Error, "{message}");
        return;
    }

    let data = match RootBootstrapContext::load() {
        Ok(data) => data,
        Err(err) => {
            let message = format!("bootstrap (root:init) bootstrap preflight failed: {err}");
            mark_root_bootstrap_failed(LifecycleMetricPhase::Init, message.clone());
            log!(Topic::Init, Error, "{message}");
            return;
        }
    };

    let missing_roles = match root_missing_staged_release_roles(&data) {
        Ok(missing_roles) => missing_roles,
        Err(err) => {
            let message = format!("bootstrap (root:init) release-set preflight failed: {err}");
            mark_root_bootstrap_failed(LifecycleMetricPhase::Init, message.clone());
            log!(Topic::Init, Error, "{message}");
            return;
        }
    };

    if !missing_roles.is_empty() {
        record_root_bootstrap_metric(LifecycleMetricPhase::Init, LifecycleMetricOutcome::Waiting);
        BootstrapStatusOps::set_phase("root:init:waiting_staged_releases");
        log!(
            Topic::Init,
            Info,
            "bootstrap (root:init) waiting for staged release roles: {:?}",
            missing_roles
        );
        return;
    }

    let _guard = match TopologyGuard::try_enter() {
        Ok(g) => g,
        Err(err) => {
            record_root_bootstrap_metric(
                LifecycleMetricPhase::Init,
                LifecycleMetricOutcome::Skipped,
            );
            BootstrapStatusOps::set_phase("root:init:skipped");
            log!(Topic::Init, Info, "bootstrap (root:init) skipped: {err}");
            return;
        }
    };

    log!(Topic::Init, Info, "bootstrap (root:init) start");

    BootstrapStatusOps::set_phase("root:init:set_subnet_id");
    root_set_subnet_id().await;

    // On fresh init, only wait for the configured initial import slice before
    // auto-create. Remaining static imports are queued for the pool worker.
    BootstrapStatusOps::set_phase("root:init:import_pool");
    root_import_pool_from_config(false).await;
    canic_core::perf!("bootstrap_import_pool");

    BootstrapStatusOps::set_phase("root:init:create_canisters");
    if let Err(err) = root_create_canisters().await {
        let message = format!("registry phase failed: {err}");
        log!(Topic::Init, Error, "{message}");
        mark_root_bootstrap_failed(LifecycleMetricPhase::Init, message);
        return;
    }
    canic_core::perf!("bootstrap_create_canisters");

    BootstrapStatusOps::set_phase("root:init:rebuild_indexes");
    if let Err(err) = root_rebuild_indexes_from_registry() {
        let message = format!("index materialization failed: {err}");
        log!(Topic::Init, Error, "{message}");
        mark_root_bootstrap_failed(LifecycleMetricPhase::Init, message);
        return;
    }
    canic_core::perf!("bootstrap_rebuild_indexes");

    BootstrapStatusOps::set_phase("root:init:validate");
    let report = root_validate_state();
    canic_core::perf!("bootstrap_validate_state");
    if !report.ok {
        mark_root_bootstrap_failed(
            LifecycleMetricPhase::Init,
            validation_failure_summary(&report),
        );
        log!(
            Topic::Init,
            Error,
            "bootstrap validation failed:\n{:#?}",
            report.issues
        );
        return;
    }

    log!(Topic::Init, Info, "bootstrap (root:init) complete");
    record_root_bootstrap_metric(
        LifecycleMetricPhase::Init,
        LifecycleMetricOutcome::Completed,
    );
    ReadyOps::mark_ready();
}

/// Bootstrap workflow for the root canister after upgrade.
pub async fn bootstrap_post_upgrade_root_canister() {
    record_root_bootstrap_metric(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricOutcome::Started,
    );

    if !root_has_embedded_wasm_store_bootstrap() {
        let message =
            "bootstrap (root:upgrade) embedded wasm_store bootstrap module is not registered";
        mark_root_bootstrap_failed(LifecycleMetricPhase::PostUpgrade, message.to_string());
        log!(Topic::Init, Error, "{message}");
        return;
    }

    let data = match RootBootstrapContext::load() {
        Ok(data) => data,
        Err(err) => {
            let message = format!("bootstrap (root:upgrade) bootstrap preflight failed: {err}");
            log!(Topic::Init, Error, "{message}");
            mark_root_bootstrap_failed(LifecycleMetricPhase::PostUpgrade, message);
            return;
        }
    };

    let missing_roles = match root_missing_staged_release_roles(&data) {
        Ok(missing_roles) => missing_roles,
        Err(err) => {
            let message = format!("bootstrap (root:upgrade) release-set preflight failed: {err}");
            log!(Topic::Init, Error, "{message}");
            mark_root_bootstrap_failed(LifecycleMetricPhase::PostUpgrade, message);
            return;
        }
    };

    if !missing_roles.is_empty() {
        record_root_bootstrap_metric(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricOutcome::Waiting,
        );
        BootstrapStatusOps::set_phase("root:upgrade:waiting_staged_releases");
        log!(
            Topic::Init,
            Info,
            "bootstrap (root:upgrade) waiting for staged release roles: {:?}",
            missing_roles
        );
        return;
    }

    // Environment already exists; only enrich + reconcile
    log!(Topic::Init, Info, "bootstrap (root:upgrade) start");
    BootstrapStatusOps::set_phase("root:upgrade:set_subnet_id");
    root_set_subnet_id().await;
    // Keep post-upgrade non-blocking; queued imports continue in background.
    BootstrapStatusOps::set_phase("root:upgrade:import_pool");
    root_import_pool_from_config(false).await;
    BootstrapStatusOps::set_phase("root:upgrade:reconcile_wasm_store");
    if let Err(err) = root_reconcile_wasm_store().await {
        let message = format!("wasm store reconcile failed: {err}");
        log!(Topic::Init, Error, "{message}");
        mark_root_bootstrap_failed(LifecycleMetricPhase::PostUpgrade, message);
        return;
    }
    log!(Topic::Init, Info, "bootstrap (root:upgrade) complete");
    record_root_bootstrap_metric(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricOutcome::Completed,
    );

    ReadyOps::mark_ready();
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
            AppRegistryOps::upsert(subnet_pid, IcOps::canister_self());
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
    AppRegistryOps::upsert(fallback, fallback);

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
                "pool import skipped: no subnet cfg ({err})"
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
        "auto_create: {:?}",
        data.subnet_cfg.auto_create
    );

    ensure_required_wasm_store_canister().await?;
    canic_core::perf!("bootstrap_ensure_wasm_store");
    WasmStorePublicationWorkflow::publish_staged_release_set_to_current_store().await?;
    canic_core::perf!("bootstrap_publish_release_set");

    // Publication already mirrors each selected managed-store binding back into
    // root-owned manifest state. Re-importing the full fleet catalog here is
    // redundant on init and can force an expensive snapshot of the just-
    // retired rollover store before bootstrap completes.

    ensure_required_canisters(&data).await
}

pub fn root_rebuild_indexes_from_registry() -> Result<(), InternalError> {
    let _ = ProvisionWorkflow::rebuild_indexes_from_registry(None)?;

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
                "pool import skipped: no build network"
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
        "pool import cfg: net={} min={} init={} limit={} wait={}",
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
            "pool import init=0 with auto_create; queued imports may lag creation"
        );
    }

    if import_list.is_empty() {
        log!(
            Topic::CanisterPool,
            Warn,
            "pool import skipped: empty list for net={}",
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
                "pool import queued async count={} pids={}",
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
        "pool import now: cfg={} ok={imported} skip={immediate_skipped} fail={immediate_failed} present={immediate_already_present}",
        configured_initial
    );
    log!(
        Topic::CanisterPool,
        Info,
        "pool import now pids: ok={} skip={} fail={} present={}",
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
                "pool import queued: cfg={} fail={queued_failed} present={queued_already_present}",
                configured_queued
            );
        } else {
            log!(
                Topic::CanisterPool,
                Info,
                "pool import queued: cfg={} added={queued_added} requeued={queued_requeued} skip={queued_skipped} present={queued_already_present}",
                configured_queued
            );
        }

        if wait_for_queued_imports {
            log!(
                Topic::CanisterPool,
                Info,
                "pool import queued pids: added={} skip={} fail={} present={}",
                summarize_principals(&queued_added_pids, 12),
                summarize_principals(&queued_skipped_pids, 12),
                summarize_principals(&queued_failed_pids, 12),
                summarize_principals(&queued_present_pids, 12),
            );
        } else {
            log!(
                Topic::CanisterPool,
                Info,
                "pool import queued pids: present={} (scheduler resolves added/requeued/skip)",
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
            log!(Topic::Init, Info, "auto_create: {role} present; skip");
            continue;
        }

        if !TemplateManifestOps::has_approved_for_role(role)? {
            log!(
                Topic::Init,
                Warn,
                "auto_create: skipping {role}; approved manifest not staged"
            );
            continue;
        }

        let manifest = TemplateManifestOps::approved_for_role_response(role)?;
        log!(
            Topic::Init,
            Info,
            "auto_create: creating {role} from {}@{}",
            manifest.template_id,
            manifest.version
        );

        CanisterLifecycleWorkflow::apply(CanisterLifecycleEvent::Create {
            role: role.clone(),
            parent: IcOps::canister_self(),
            extra_arg: None,
        })
        .await?;
        canic_core::perf!("bootstrap_create_role");
    }

    Ok(())
}

async fn root_reconcile_wasm_store() -> Result<(), InternalError> {
    ensure_required_wasm_store_canister().await?;
    canic_core::perf!("bootstrap_ensure_wasm_store");

    let deprecated = WasmStorePublicationWorkflow::prune_unconfigured_managed_releases()?;
    if deprecated > 0 {
        log!(
            Topic::Init,
            Warn,
            "ws: deprecated {deprecated} stale managed release(s) no longer present in config"
        );
    }
    canic_core::perf!("bootstrap_prune_store_catalog");

    import_default_wasm_store_catalog().await
}

async fn ensure_required_wasm_store_canister() -> Result<(), InternalError> {
    let role = CanisterRole::WASM_STORE;

    let existing_bindings = WasmStorePublicationWorkflow::sync_registered_wasm_store_inventory();
    if !existing_bindings.is_empty() {
        log!(Topic::Init, Info, "ws: {role} present; skip");
        return Ok(());
    }

    log!(Topic::Init, Info, "ws: create {role}");

    CanisterLifecycleWorkflow::apply(CanisterLifecycleEvent::Create {
        role,
        parent: IcOps::canister_self(),
        extra_arg: None,
    })
    .await?;
    canic_core::perf!("bootstrap_create_wasm_store");
    let _ = WasmStorePublicationWorkflow::sync_registered_wasm_store_inventory();
    canic_core::perf!("bootstrap_sync_store_inventory");

    Ok(())
}

async fn import_default_wasm_store_catalog() -> Result<(), InternalError> {
    WasmStorePublicationWorkflow::import_current_store_catalog().await?;
    canic_core::perf!("bootstrap_import_store_catalog");

    log!(Topic::Init, Info, "ws: imported default catalog");

    Ok(())
}

pub fn root_validate_state() -> ValidationReport {
    let app_data = AppIndexOps::data();
    let subnet_data = SubnetIndexOps::data();

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

    let (app_unique, app_consistent) =
        check_index("app_index", &app_data.entries, &registry_roles, &mut issues);
    let (subnet_unique, subnet_consistent) = check_index(
        "subnet_index",
        &subnet_data.entries,
        &registry_roles,
        &mut issues,
    );

    let unique_index_roles = app_unique && subnet_unique;
    let registry_index_consistent = app_consistent && subnet_consistent;
    let ok = env_complete && unique_index_roles && registry_index_consistent;

    ValidationReport {
        ok,
        registry_index_consistent,
        unique_index_roles,
        env_complete,
        issues,
    }
}

fn check_index(
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
                code: "index_role_duplicate".to_string(),
                message: format!("{label} has duplicate role {role}"),
            });
        }

        match registry_roles.get(role) {
            None => {
                consistent = false;
                issues.push(ValidationIssue {
                    code: "index_role_missing_in_registry".to_string(),
                    message: format!("{label} role {role} not present in registry"),
                });
            }
            Some(pids) if pids.len() > 1 => {
                consistent = false;
                issues.push(ValidationIssue {
                    code: "index_role_duplicate_in_registry".to_string(),
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
                        code: "index_role_pid_mismatch".to_string(),
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
