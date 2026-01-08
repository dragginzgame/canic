//! Root bootstrap phase.
//!
//! This module defines the asynchronous bootstrap phase for the root canister.
//! It runs after runtime initialization and is responsible for all
//! cross-canister orchestration, topology creation, and reconciliation.

use crate::{
    Error,
    config::schema::SubnetConfig,
    dto::validation::{ValidationIssue, ValidationReport},
    ops::{
        config::ConfigOps,
        ic::{
            IcOps,
            network::{BuildNetwork, NetworkOps},
        },
        runtime::env::{EnvOps, EnvSnapshot},
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            pool::PoolOps,
            registry::subnet::{SubnetRegistryOps, SubnetRegistrySnapshot},
        },
    },
    workflow::{
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        ic::{IcWorkflow, provision::ProvisionWorkflow},
        pool::PoolWorkflow,
        prelude::*,
        topology::guard::TopologyGuard,
    },
};
use std::collections::BTreeMap;

///
/// RootBootstrapSnapshot
///

struct RootBootstrapSnapshot {
    subnet_cfg: SubnetConfig,
    network: Option<BuildNetwork>,
}

impl RootBootstrapSnapshot {
    fn load() -> Result<Self, Error> {
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
    let _guard = match TopologyGuard::try_enter() {
        Ok(g) => g,
        Err(err) => {
            log!(Topic::Init, Info, "bootstrap (root:init) skipped: {err}");
            return;
        }
    };

    // ---------------- Phase 1: Registry ----------------
    log!(Topic::Init, Info, "bootstrap phase: REGISTRY");

    root_import_pool_from_config().await;

    if let Err(err) = root_create_canisters().await {
        log!(Topic::Init, Error, "registry phase failed: {err}");
        return;
    }

    // ---------------- Phase 2: Materialize ----------------
    log!(Topic::Init, Info, "bootstrap phase: MATERIALIZE");

    if let Err(err) = root_rebuild_directories_from_registry() {
        log!(
            Topic::Init,
            Error,
            "directory materialization failed: {err}"
        );
        return;
    }

    // ---------------- Phase 3: Validate ----------------
    log!(Topic::Init, Info, "bootstrap phase: VALIDATE");

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

    // ---------------- Phase 4: Completed ----------------
    log!(Topic::Init, Info, "bootstrap phase: COMPLETED");
}

/// Bootstrap workflow for the root canister after upgrade.
pub async fn bootstrap_post_upgrade_root_canister() {
    // Environment already exists; only enrich + reconcile
    log!(Topic::Init, Info, "bootstrap (root:upgrade) start");
    log!(
        Topic::Init,
        Info,
        "bootstrap (root:upgrade) resolve subnet id"
    );
    root_set_subnet_id().await;
    log!(
        Topic::Init,
        Info,
        "bootstrap (root:upgrade) import pool from config"
    );
    root_import_pool_from_config().await;
    log!(Topic::Init, Info, "bootstrap (root:upgrade) complete");
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
pub async fn root_import_pool_from_config() {
    let snapshot = match RootBootstrapSnapshot::load() {
        Ok(snapshot) => snapshot,
        Err(err) => {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import skipped: missing subnet config ({err})"
            );
            return;
        }
    };

    ensure_pool_imported(&snapshot).await;
}

/// ---------------------------------------------------------------------------
/// Canister creation
/// ---------------------------------------------------------------------------

/// Ensure all statically configured canisters for this subnet exist.
pub async fn root_create_canisters() -> Result<(), Error> {
    let snapshot = RootBootstrapSnapshot::load()?;

    log!(
        Topic::Init,
        Info,
        "auto_create roles: {:?}",
        snapshot.subnet_cfg.auto_create
    );

    ensure_required_canisters(&snapshot).await
}

pub fn root_rebuild_directories_from_registry() -> Result<(), Error> {
    let _ = ProvisionWorkflow::rebuild_directories_from_registry(None)?;

    Ok(())
}

#[expect(clippy::too_many_lines)]
async fn ensure_pool_imported(snapshot: &RootBootstrapSnapshot) {
    let import_list = match snapshot.network {
        Some(BuildNetwork::Local) => snapshot.subnet_cfg.pool.import.local.clone(),
        Some(BuildNetwork::Ic) => snapshot.subnet_cfg.pool.import.ic.clone(),
        None => {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import skipped: build network not set"
            );
            return;
        }
    };

    let initial_limit = snapshot
        .subnet_cfg
        .pool
        .import
        .initial
        .map_or(snapshot.subnet_cfg.pool.minimum_size as usize, |count| {
            count as usize
        });

    if initial_limit == 0 && !snapshot.subnet_cfg.auto_create.is_empty() {
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

    for pid in initial {
        if PoolOps::contains(pid) {
            immediate_already_present += 1;
            continue;
        }

        match PoolWorkflow::pool_import_canister(*pid).await {
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

    let queued_imports: Vec<Principal> = queued
        .iter()
        .copied()
        .filter(|pid| {
            if PoolOps::contains(pid) {
                queued_already_present += 1;
                false
            } else {
                true
            }
        })
        .collect();

    if !queued_imports.is_empty() {
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

    log!(
        Topic::CanisterPool,
        Info,
        "pool import immediate summary: configured={}, imported={imported}, skipped={immediate_skipped}, failed={immediate_failed}, present={immediate_already_present}",
        configured_initial
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
    }
}

async fn ensure_required_canisters(snapshot: &RootBootstrapSnapshot) -> Result<(), Error> {
    for role in &snapshot.subnet_cfg.auto_create {
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
    let registry_snapshot = SubnetRegistryOps::snapshot();
    let app_snapshot = AppDirectoryOps::snapshot();
    let subnet_snapshot = SubnetDirectoryOps::snapshot();
    let env_snapshot = EnvOps::snapshot();

    let mut issues = Vec::new();

    let env_missing = env_missing_fields(&env_snapshot);
    let env_complete = env_missing.is_empty();
    if !env_complete {
        issues.push(ValidationIssue {
            code: "env_missing_fields".to_string(),
            message: format!("missing env fields: {}", env_missing.join(", ")),
        });
    }

    let registry_roles = build_registry_role_index(&registry_snapshot);

    let (app_unique, app_consistent) = check_directory(
        "app_directory",
        &app_snapshot.entries,
        &registry_roles,
        &mut issues,
    );
    let (subnet_unique, subnet_consistent) = check_directory(
        "subnet_directory",
        &subnet_snapshot.entries,
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

fn env_missing_fields(snapshot: &EnvSnapshot) -> Vec<&'static str> {
    let mut missing = Vec::new();

    if snapshot.prime_root_pid.is_none() {
        missing.push("prime_root_pid");
    }
    if snapshot.subnet_role.is_none() {
        missing.push("subnet_role");
    }
    if snapshot.subnet_pid.is_none() {
        missing.push("subnet_pid");
    }
    if snapshot.root_pid.is_none() {
        missing.push("root_pid");
    }
    if snapshot.canister_role.is_none() {
        missing.push("canister_role");
    }
    if snapshot.parent_pid.is_none() {
        missing.push("parent_pid");
    }

    missing
}

fn build_registry_role_index(
    registry: &SubnetRegistrySnapshot,
) -> BTreeMap<CanisterRole, Vec<Principal>> {
    let mut roles = BTreeMap::<CanisterRole, Vec<Principal>>::new();

    for (pid, entry) in &registry.entries {
        roles.entry(entry.role.clone()).or_default().push(*pid);
    }

    roles
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
