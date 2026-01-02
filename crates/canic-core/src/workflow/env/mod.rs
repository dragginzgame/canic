pub mod mapper;
pub mod query;

pub use mapper::EnvMapper;

use crate::{
    Error,
    cdk::types::Principal,
    dto::env::EnvView,
    ids::{CanisterRole, SubnetRole},
    infra::ic::{Network, build_network},
    ops::runtime::env::{EnvOps, EnvSnapshot},
    workflow::bootstrap::BootstrapError,
};

pub(crate) fn init_env_from_view(env_view: EnvView, role: CanisterRole) -> Result<(), Error> {
    let mut snapshot = EnvMapper::view_to_snapshot(env_view);
    snapshot.canister_role = Some(role.clone());

    let snapshot = ensure_nonroot_env_snapshot(role, snapshot)?;
    EnvOps::import(snapshot)
}

fn ensure_nonroot_env_snapshot(
    canister_role: CanisterRole,
    mut env: EnvSnapshot,
) -> Result<EnvSnapshot, Error> {
    let mut missing = Vec::new();
    if env.prime_root_pid.is_none() {
        missing.push("prime_root_pid");
    }
    if env.subnet_role.is_none() {
        missing.push("subnet_role");
    }
    if env.subnet_pid.is_none() {
        missing.push("subnet_pid");
    }
    if env.root_pid.is_none() {
        missing.push("root_pid");
    }
    if env.canister_role.is_none() {
        missing.push("canister_role");
    }
    if env.parent_pid.is_none() {
        missing.push("parent_pid");
    }

    if missing.is_empty() {
        return Ok(env);
    }

    if build_network() == Some(Network::Ic) {
        return Err(BootstrapError::MissingEnvFields(missing.join(", ")).into());
    }

    let root_pid = Principal::from_slice(&[0xBB; 29]);
    let subnet_pid = Principal::from_slice(&[0xAA; 29]);

    if env.prime_root_pid.is_none() {
        env.prime_root_pid = Some(root_pid);
    }
    if env.subnet_role.is_none() {
        env.subnet_role = Some(SubnetRole::PRIME);
    }
    if env.subnet_pid.is_none() {
        env.subnet_pid = Some(subnet_pid);
    }
    if env.root_pid.is_none() {
        env.root_pid = Some(root_pid);
    }
    if env.canister_role.is_none() {
        env.canister_role = Some(canister_role);
    }
    if env.parent_pid.is_none() {
        env.parent_pid = Some(root_pid);
    }

    Ok(env)
}
