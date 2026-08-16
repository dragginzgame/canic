//! Root bootstrap phase.
//!
//! This module defines the asynchronous bootstrap phase for the root canister.
//! It runs after runtime initialization and is responsible for all
//! cross-canister orchestration, topology creation, and reconciliation.

use crate::{
    ids::{BuildNetwork, CanisterRole},
    ops::{component_registry::ComponentRegistryOps, storage::template::TemplateChunkedOps},
    workflow::runtime::template::WasmStorePublicationWorkflow,
};
use canic_core::api::lifecycle::metrics::{
    LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
};
use canic_core::control_plane_support::{
    config::ComponentTopology,
    error::InternalError,
    ops::{
        config::ConfigOps,
        ic::build_network::BuildNetworkOps,
        runtime::{
            bootstrap::{BootstrapPhaseLabel, BootstrapStatusOps},
            env::EnvOps,
            ready::ReadyOps,
        },
    },
    workflow::{
        ic::IcWorkflow, runtime::fleet_activation::FleetActivationWorkflow,
        topology::guard::TopologyGuard,
    },
};
use canic_core::{dto::fleet_activation::FleetActivationPhase, log, log::Topic};
use std::collections::BTreeSet;

///
/// RootBootstrapContext
///

struct RootBootstrapContext {
    component_topology: ComponentTopology,
}

impl RootBootstrapContext {
    fn load() -> Result<Self, InternalError> {
        let component_topology = ConfigOps::component_topology()?;
        Ok(Self { component_topology })
    }

    fn managed_release_roles(&self) -> BTreeSet<CanisterRole> {
        self.component_topology
            .component_specs
            .iter()
            .map(|component_spec| component_spec.component_role.clone())
            .collect()
    }
}

/// ---------------------------------------------------------------------------
/// Root bootstrap entrypoints
/// ---------------------------------------------------------------------------

fn root_missing_staged_release_roles(
    data: &RootBootstrapContext,
) -> Result<Vec<CanisterRole>, InternalError> {
    let mut missing = Vec::new();

    for role in data.managed_release_roles() {
        if role.is_wasm_store() {
            continue;
        }

        if !TemplateChunkedOps::has_publishable_chunked_approved_for_role(&role)? {
            missing.push(role);
        }
    }

    Ok(missing)
}

fn record_root_bootstrap_metric(phase: LifecycleMetricPhase, outcome: LifecycleMetricOutcome) {
    LifecycleMetricsApi::record_bootstrap(phase, LifecycleMetricRole::Root, outcome);
}

fn mark_root_bootstrap_failed(phase: LifecycleMetricPhase, message: String) {
    record_root_bootstrap_metric(phase, LifecycleMetricOutcome::Failed);
    BootstrapStatusOps::mark_failed(message);
}

fn fleet_is_prepared() -> bool {
    matches!(
        FleetActivationWorkflow::status(),
        Ok(status) if status.phase == FleetActivationPhase::Prepared
    )
}

#[must_use]
pub fn activation_preparation_complete() -> bool {
    BootstrapStatusOps::snapshot().phase
        == BootstrapPhaseLabel::ROOT_INIT_ACTIVATION_PREPARED.as_str()
        && ComponentRegistryOps::current().is_some()
}

fn complete_or_wait_for_root_activation() {
    if ComponentRegistryOps::current().is_none() {
        record_root_bootstrap_metric(LifecycleMetricPhase::Init, LifecycleMetricOutcome::Waiting);
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_INIT_WAITING_COMPONENT_REGISTRY);
        log!(
            Topic::Init,
            Info,
            "bootstrap (root:init) waiting for prepared Component Registry authority"
        );
        return;
    }

    if fleet_is_prepared() {
        record_root_bootstrap_metric(LifecycleMetricPhase::Init, LifecycleMetricOutcome::Waiting);
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_INIT_ACTIVATION_PREPARED);
        log!(
            Topic::Init,
            Info,
            "bootstrap (root:init) prepared managed inventory for Fleet activation"
        );
        return;
    }

    if !crate::workflow::component_registry::root_runtime_activation_receipt_complete() {
        record_root_bootstrap_metric(LifecycleMetricPhase::Init, LifecycleMetricOutcome::Waiting);
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_INIT_ACTIVATION_PREPARED);
        log!(
            Topic::Init,
            Info,
            "bootstrap (root:init) waiting for terminal Component inventory activation receipt"
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

pub async fn bootstrap_init_root_canister() {
    record_root_bootstrap_metric(LifecycleMetricPhase::Init, LifecycleMetricOutcome::Started);

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
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_INIT_WAITING_STAGED_RELEASES);
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
            BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_INIT_SKIPPED);
            log!(Topic::Init, Info, "bootstrap (root:init) skipped: {err}");
            return;
        }
    };

    log!(Topic::Init, Info, "bootstrap (root:init) start");

    BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_INIT_SET_SUBNET_ID);
    if let Err(err) = root_set_subnet_id().await {
        let message = format!("subnet identity phase failed: {err}");
        log!(Topic::Init, Error, "{message}");
        mark_root_bootstrap_failed(LifecycleMetricPhase::Init, message);
        return;
    }

    complete_or_wait_for_root_activation();
}

/// Bootstrap workflow for the root canister after upgrade.
pub async fn bootstrap_post_upgrade_root_canister() {
    record_root_bootstrap_metric(
        LifecycleMetricPhase::PostUpgrade,
        LifecycleMetricOutcome::Started,
    );

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
        BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_UPGRADE_WAITING_STAGED_RELEASES);
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
    BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_UPGRADE_SET_SUBNET_ID);
    if let Err(err) = root_set_subnet_id().await {
        let message = format!("subnet identity phase failed: {err}");
        log!(Topic::Init, Error, "{message}");
        mark_root_bootstrap_failed(LifecycleMetricPhase::PostUpgrade, message);
        return;
    }
    BootstrapStatusOps::set_phase(BootstrapPhaseLabel::ROOT_UPGRADE_RECONCILE_WASM_STORE);
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
/// IC builds resolve the authoritative subnet from the NNS registry. Local and
/// test builds use the explicit subnet identity seeded by lifecycle init.
pub async fn root_set_subnet_id() -> Result<(), InternalError> {
    let build_network = BuildNetworkOps::build_network().ok_or_else(InternalError::invariant)?;

    if build_network != BuildNetwork::Ic {
        let subnet_pid = EnvOps::subnet_pid()?;
        log!(
            Topic::Topology,
            Info,
            "root subnet identity initialized from lifecycle env: {subnet_pid}"
        );
        return Ok(());
    }

    match IcWorkflow::try_get_current_subnet_pid().await {
        Ok(Some(subnet_pid)) => {
            EnvOps::set_subnet_pid(subnet_pid);
            Ok(())
        }

        Ok(None) => Err(InternalError::lifecycle_failure()),

        Err(_err) => Err(InternalError::lifecycle_failure()),
    }
}

async fn root_reconcile_wasm_store() -> Result<(), InternalError> {
    ensure_required_wasm_store_canister()?;
    canic_core::perf!("bootstrap_ensure_wasm_store");

    let removed = WasmStorePublicationWorkflow::prune_unconfigured_managed_releases()?;
    if removed > 0 {
        log!(
            Topic::Init,
            Warn,
            "ws: removed {removed} stale managed release(s) no longer present in config"
        );
    }
    canic_core::perf!("bootstrap_prune_store_catalog");

    import_default_wasm_store_catalog().await
}

pub(super) fn ensure_required_wasm_store_canister() -> Result<(), InternalError> {
    let binding = WasmStorePublicationWorkflow::ensure_bootstrap_wasm_store()?;
    log!(
        Topic::Init,
        Info,
        "ws: adopted sibling Store {binding} present"
    );
    Ok(())
}

async fn import_default_wasm_store_catalog() -> Result<(), InternalError> {
    WasmStorePublicationWorkflow::import_current_store_catalog().await?;
    canic_core::perf!("bootstrap_import_store_catalog");

    log!(Topic::Init, Info, "ws: imported default catalog");

    Ok(())
}
