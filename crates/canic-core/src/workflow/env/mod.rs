pub mod mapper;
pub mod query;

pub use mapper::EnvMapper;

use crate::{
    Error,
    domain::policy::env::{EnvInput, EnvPolicyError, validate_or_default},
    dto::env::EnvView,
    infra::network::Network,
    ops::runtime::{
        env::{EnvOps, EnvSnapshot},
        network::NetworkOps,
    },
    workflow::{bootstrap::BootstrapError, prelude::*},
};

pub fn init_env_from_view(env_view: EnvView, role: CanisterRole) -> Result<(), Error> {
    let mut snapshot = EnvMapper::view_to_snapshot(env_view);
    snapshot.canister_role = Some(role);

    let network = NetworkOps::current_network().unwrap_or(Network::Local);
    let input = EnvInput {
        prime_root_pid: snapshot.prime_root_pid,
        subnet_role: snapshot.subnet_role,
        subnet_pid: snapshot.subnet_pid,
        root_pid: snapshot.root_pid,
        canister_role: snapshot.canister_role,
        parent_pid: snapshot.parent_pid,
    };
    let validated = match validate_or_default(network, input) {
        Ok(validated) => validated,
        Err(EnvPolicyError::MissingEnvFields(missing)) => {
            return Err(BootstrapError::MissingEnvFields(missing).into());
        }
    };

    EnvOps::import(EnvSnapshot {
        prime_root_pid: Some(validated.prime_root_pid),
        subnet_role: Some(validated.subnet_role),
        subnet_pid: Some(validated.subnet_pid),
        root_pid: Some(validated.root_pid),
        canister_role: Some(validated.canister_role),
        parent_pid: Some(validated.parent_pid),
    })
}
