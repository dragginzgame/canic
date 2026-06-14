use crate::icp::{self, CANIC_ICP_LOCAL_NETWORK_URL_ENV, CANIC_ICP_LOCAL_ROOT_KEY_ENV};
use crate::replica_query;
use std::{env, ffi::OsString, path::Path};

pub(super) struct BuildEnvGuard {
    previous_network: Option<OsString>,
    previous_config_path: Option<OsString>,
    previous_icp_root: Option<OsString>,
    previous_local_network_url: Option<OsString>,
    previous_local_root_key: Option<OsString>,
}

impl BuildEnvGuard {
    pub(super) fn apply(network: &str, config_path: &Path, icp_root: &Path) -> Self {
        let guard = Self {
            previous_network: env::var_os("ICP_ENVIRONMENT"),
            previous_config_path: env::var_os("CANIC_CONFIG_PATH"),
            previous_icp_root: env::var_os("CANIC_ICP_ROOT"),
            previous_local_network_url: env::var_os(CANIC_ICP_LOCAL_NETWORK_URL_ENV),
            previous_local_root_key: env::var_os(CANIC_ICP_LOCAL_ROOT_KEY_ENV),
        };
        set_env("ICP_ENVIRONMENT", network);
        set_env("CANIC_CONFIG_PATH", config_path);
        set_env("CANIC_ICP_ROOT", icp_root);
        if let Some(target) = local_replica_icp_target(network, icp_root) {
            set_env(CANIC_ICP_LOCAL_NETWORK_URL_ENV, target.url);
            set_env(CANIC_ICP_LOCAL_ROOT_KEY_ENV, target.root_key);
        } else {
            remove_env(CANIC_ICP_LOCAL_NETWORK_URL_ENV);
            remove_env(CANIC_ICP_LOCAL_ROOT_KEY_ENV);
        }
        guard
    }
}

impl Drop for BuildEnvGuard {
    fn drop(&mut self) {
        restore_env("ICP_ENVIRONMENT", self.previous_network.take());
        restore_env("CANIC_CONFIG_PATH", self.previous_config_path.take());
        restore_env("CANIC_ICP_ROOT", self.previous_icp_root.take());
        restore_env(
            CANIC_ICP_LOCAL_NETWORK_URL_ENV,
            self.previous_local_network_url.take(),
        );
        restore_env(
            CANIC_ICP_LOCAL_ROOT_KEY_ENV,
            self.previous_local_root_key.take(),
        );
    }
}

struct LocalReplicaIcpTarget {
    url: String,
    root_key: String,
}

fn local_replica_icp_target(network: &str, icp_root: &Path) -> Option<LocalReplicaIcpTarget> {
    if !replica_query::should_use_local_replica_query(Some(network)) {
        return None;
    }
    if icp_ping(icp_root, network).unwrap_or(false) {
        return None;
    }
    let root_key = replica_query::local_replica_root_key_from_root(Some(network), icp_root)
        .ok()
        .flatten()?;
    Some(LocalReplicaIcpTarget {
        url: replica_query::local_replica_endpoint_from_root(Some(network), icp_root),
        root_key,
    })
}

fn set_env<K, V>(key: K, value: V)
where
    K: AsRef<std::ffi::OsStr>,
    V: AsRef<std::ffi::OsStr>,
{
    // Install builds are single-threaded host orchestration. The environment is
    // scoped by BuildEnvGuard so Cargo build scripts see the selected fleet.
    unsafe {
        env::set_var(key, value);
    }
}

fn remove_env<K>(key: K)
where
    K: AsRef<std::ffi::OsStr>,
{
    // Install builds are single-threaded host orchestration. The environment is
    // scoped by BuildEnvGuard so Cargo build scripts see the selected fleet.
    unsafe {
        env::remove_var(key);
    }
}

fn restore_env(key: &str, value: Option<OsString>) {
    // See set_env: this restores the single-threaded install build context.
    if let Some(value) = value {
        set_env(key, value);
    } else {
        remove_env(key);
    }
}

pub(super) fn ensure_icp_environment_ready(
    icp_root: &Path,
    network: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if icp_ping(icp_root, network)? {
        return Ok(());
    }
    if replica_query::should_use_local_replica_query(Some(network))
        && replica_query::local_replica_status_reachable_from_root(Some(network), icp_root)
    {
        println!(
            "Replica reachable via HTTP status endpoint even though ICP CLI reports network '{network}' stopped; continuing from ICP root {}.",
            icp_root.display()
        );
        return Ok(());
    }

    Err(format!(
        "icp environment is not running for network '{network}'\nStart the target replica in another terminal with `canic replica start` and rerun."
    )
    .into())
}

fn icp_ping(icp_root: &Path, network: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut command = icp::default_command_in(icp_root);
    command.args(["network", "ping", network]);
    Ok(icp::run_success(&mut command)?)
}
