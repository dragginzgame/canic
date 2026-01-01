use crate::{
    Error,
    cdk::types::Principal,
    ids::{CanisterRole, SubnetRole},
    infra::ic::{Network, build_network},
    storage::memory::env::EnvData,
    workflow::bootstrap::BootstrapError,
};

fn ensure_nonroot_env(canister_role: CanisterRole, mut env: EnvData) -> Result<EnvData, Error> {
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

    env.prime_root_pid.get_or_insert(root_pid);
    env.subnet_role.get_or_insert(SubnetRole::PRIME);
    env.subnet_pid.get_or_insert(subnet_pid);
    env.root_pid.get_or_insert(root_pid);
    env.canister_role.get_or_insert(canister_role);
    env.parent_pid.get_or_insert(root_pid);

    Ok(env)
}
