use crate::{
    InternalError, InternalErrorOrigin, VERSION,
    domain::policy::env::{EnvInput, EnvPolicyError, validate_or_default},
    dto::subnet::SubnetIdentity,
    ids::{CanisterRole, SubnetRole},
    ops::{
        config::ConfigOps,
        ic::{IcOps, network::NetworkOps},
        runtime::{env::EnvOps, memory::MemoryRegistryOps},
        storage::{registry::subnet::SubnetRegistryOps, state::app::AppStateOps},
    },
    workflow::prelude::*,
};

use super::{RuntimeWorkflow, ensure_root_delegated_auth_crypto_contract, log_memory_summary};

///
/// init_root_canister
/// Bootstraps the root canister runtime and environment.
///

pub fn init_root_canister(identity: SubnetIdentity) -> Result<(), InternalError> {
    // --- Phase 1: Init base systems ---
    let memory_summary = MemoryRegistryOps::bootstrap_registry().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("memory init failed: {err}"),
        )
    })?;
    crate::log::set_ready();

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
    log_memory_summary(&memory_summary);

    let self_pid = IcOps::canister_self();
    let (subnet_pid, subnet_role, prime_root_pid) = match identity {
        SubnetIdentity::Prime => (self_pid, SubnetRole::PRIME, self_pid),
        SubnetIdentity::Standard(params) => (self_pid, params.subnet_type, params.prime_root_pid),
        SubnetIdentity::Manual => (IcOps::canister_self(), SubnetRole::PRIME, self_pid),
    };

    let input = EnvInput {
        prime_root_pid: Some(prime_root_pid),
        subnet_role: Some(subnet_role),
        subnet_pid: Some(subnet_pid),
        root_pid: Some(self_pid),
        canister_role: Some(CanisterRole::ROOT),
        parent_pid: Some(prime_root_pid),
    };

    let network = NetworkOps::build_network().ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "runtime network unavailable; set DFX_NETWORK=local|ic at build time".to_string(),
        )
    })?;
    crate::log!(Topic::Init, Info, "build network: {network}");
    let validated = match validate_or_default(network, input) {
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
    ensure_root_delegated_auth_crypto_contract()?;

    let created_at = IcOps::now_secs();
    SubnetRegistryOps::register_root(self_pid, created_at);

    // --- Phase 3: Service startup ---
    RuntimeWorkflow::start_all_root().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("root service startup failed: {err}"),
        )
    })?;

    Ok(())
}

///
/// post_upgrade_root_canister
///

pub fn post_upgrade_root_canister_after_memory_init(
    memory_summary: crate::ops::runtime::memory::MemoryRegistryInitSummary,
) -> Result<(), InternalError> {
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 post_upgrade_root_canister");
    log_memory_summary(&memory_summary);

    // --- Phase 2 intentionally omitted: post-upgrade does not re-import env or directories.
    ensure_root_delegated_auth_crypto_contract()?;

    // --- Phase 3: Service startup ---
    RuntimeWorkflow::start_all_root().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("root service startup failed: {err}"),
        )
    })?;

    Ok(())
}
