use super::{
    attach_fleet_role_source, canister_manifest_path, canisters_root, config_path,
    configured_bootstrap_roles_from_source, configured_controllers_from_source,
    configured_deployable_roles_from_source, configured_fleet_name_from_source,
    configured_install_targets, configured_local_root_create_cycles_from_source,
    configured_pool_expectations_from_source, configured_release_roles_from_source,
    configured_role_auto_create_from_source, configured_role_capabilities_from_source,
    configured_role_details_from_source, configured_role_kinds_from_source,
    configured_role_lifecycle_from_source, configured_role_metrics_profiles_from_source,
    configured_role_topups_from_source, declare_fleet_role_source, read_release_artifact,
    rename_fleet_role_source, root_manifest_path,
};
use crate::test_support::temp_dir;
use flate2::{Compression, write::GzEncoder};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

mod artifacts;
mod config;
mod mutations;
mod paths;
mod roles;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
const REAL_CONFIG: &str = r#"
controllers = []
app_index = ["user_hub", "scale_hub"]

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.project_instance]
kind = "canister"
package = "project_instance"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[app]
init_mode = "enabled"
[app.whitelist]

[auth.delegated_tokens]
enabled = true

[standards]
icrc21 = true

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "service"

[subnets.prime.canisters.scale_hub]
kind = "service"
"#;

const MULTI_ROOT_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.secondary.canisters.root]
kind = "root"
"#;

const NO_ROOT_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.user_hub]
kind = "service"
"#;

fn with_guarded_env<T>(test: impl FnOnce() -> T) -> T {
    let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap();
    test()
}

fn restore_env(key: &str, previous: Option<std::ffi::OsString>) {
    unsafe {
        if let Some(value) = previous {
            std::env::set_var(key, value);
        } else {
            std::env::remove_var(key);
        }
    }
}

struct TempWorkspace {
    path: PathBuf,
}

impl TempWorkspace {
    fn new() -> Self {
        let path = temp_dir("canic-host-release-set-tests");
        fs::create_dir_all(&path).expect("create temp workspace");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
