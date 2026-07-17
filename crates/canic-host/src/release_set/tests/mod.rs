use super::config::{
    attach_fleet_role_source, configured_bootstrap_roles_from_source,
    configured_controllers_from_source, configured_deployable_roles_from_source,
    configured_fleet_name_from_source, configured_local_root_create_cycles_from_source,
    configured_pool_expectations_from_source, configured_release_roles_from_config,
    configured_role_auto_create_from_source, configured_role_details_from_source,
    configured_role_kinds_from_source, configured_role_lifecycle_from_source,
    configured_role_metrics_profiles_from_source, configured_role_topups_from_source,
    declare_fleet_role_source, rename_fleet_role_source,
};
use super::stage::{read_release_artifact, resolve_release_artifact_path};
use super::{
    CanisterManifestError, canister_manifest_path, canisters_root, config_path,
    plan_attach_fleet_role, plan_declare_fleet_role, plan_rename_fleet_role, root_manifest_path,
};
use crate::test_support::temp_dir;
use flate2::{Compression, write::GzEncoder};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

mod artifacts;
mod config;
mod mutations;
mod paths;
mod roles;

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
enabled = false

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
