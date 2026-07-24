//! Module: workflow::runtime::root
//!
//! Responsibility: initialize and restore root canister runtime services.
//! Does not own: IC lifecycle hooks, endpoint authorization, or config schemas.
//! Boundary: lifecycle adapters call this after stable-memory restore or init input decode.

use crate::{
    InternalError, InternalErrorOrigin, VERSION,
    domain::policy::pure::env::{EnvInput, EnvPolicyError, validate_or_default},
    dto::fleet_activation::CurrentRootInstallIdentity,
    ids::{CanisterRole, SubnetSlotId},
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::{IcOps, build_network::BuildNetworkOps, release_build::ReleaseBuildOps},
        runtime::{
            env::EnvOps, fleet_activation::FleetActivationRuntimeOps, memory::MemoryRegistryOps,
        },
        storage::{
            fleet_activation::FleetActivationOps, registry::subnet::SubnetRegistryOps,
            state::app::AppStateOps,
        },
    },
    workflow::runtime::{
        RuntimeWorkflow, auth::RuntimeAuthWorkflow, log_memory_summary,
        rebuild_root_derived_storage_indexes, require_no_resumable_refill_for_upgrade,
    },
};

///
/// init_root_canister
/// Bootstraps the root canister runtime and environment.
///

pub fn init_root_canister(identity: CurrentRootInstallIdentity) -> Result<(), InternalError> {
    // --- Phase 1: Init base systems ---
    MemoryRegistryOps::bootstrap_registry().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("memory init failed: {err}"),
        )
    })?;
    rebuild_root_derived_storage_indexes()?;
    FleetActivationRuntimeOps::set_managed();
    crate::log::set_ready();
    let embedded_release_build_id = ReleaseBuildOps::embedded_release_build_id()?;
    FleetActivationOps::initialize_root_prepared(identity.clone(), embedded_release_build_id)
        .map_err(crate::ops::storage::StorageOpsError::from)?;

    // --- Phase 2: Runtime header and env registration ---
    IcOps::println("");
    IcOps::println("");
    IcOps::println("");
    crate::log!(
        Topic::Init,
        Info,
        "🔧 --------------------- canic v{VERSION} -----------------------",
    );
    crate::log!(Topic::Init, Info, "🏁 init: root ({identity:?})");
    log_memory_summary();

    let self_pid = IcOps::canister_self();
    let subnet_pid = self_pid;
    let subnet_role = SubnetSlotId::DEFAULT;
    let prime_root_pid = self_pid;
    let module_hash = identity.expected_module_hash.map(|hash| hash.to_vec());

    let input = EnvInput {
        prime_root_pid: Some(prime_root_pid),
        subnet_role: Some(subnet_role),
        subnet_pid: Some(subnet_pid),
        root_pid: Some(self_pid),
        canister_role: Some(CanisterRole::ROOT),
        parent_pid: Some(prime_root_pid),
    };

    let build_network = BuildNetworkOps::build_network().ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "build network unavailable; set ICP_ENVIRONMENT=local|ic at build time".to_string(),
        )
    })?;
    crate::log!(Topic::Init, Info, "build network: {build_network}");
    let validated = match validate_or_default(input) {
        Ok(validated) => validated,
        Err(EnvPolicyError::MissingEnvFields(missing)) => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("env args missing {missing}; local builds require explicit env fields"),
            ));
        }
    };

    if let Err(err) = EnvOps::import_validated(validated) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("env import failed: {err}"),
        ));
    }

    let app_mode = ConfigOps::app_init_mode().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("app mode init failed: {err}"),
        )
    })?;
    AppStateOps::init_mode(app_mode);
    RuntimeAuthWorkflow::ensure_root_crypto_contract()?;

    let created_at = IcOps::now_secs();
    SubnetRegistryOps::register_root_with_module_hash(self_pid, created_at, module_hash);

    Ok(())
}

///
/// post_upgrade_root_canister
///

pub fn post_upgrade_root_canister_after_memory_init() -> Result<(), InternalError> {
    rebuild_root_derived_storage_indexes()?;
    FleetActivationRuntimeOps::set_managed();
    require_no_resumable_refill_for_upgrade()?;
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 post_upgrade_root_canister");
    log_memory_summary();

    // --- Phase 2 intentionally omitted: post-upgrade does not re-import env or directories.
    RuntimeAuthWorkflow::ensure_root_crypto_contract()?;

    // --- Phase 3: Service startup ---
    RuntimeWorkflow::start_all_root().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("root service startup failed: {err}"),
        )
    })?;

    Ok(())
}
