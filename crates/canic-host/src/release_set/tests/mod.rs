use super::config::{
    attach_app_role_source, configured_bootstrap_roles_from_config,
    configured_pool_expectations_from_config, configured_role_auto_create_from_config,
    configured_role_details_from_config, configured_role_kinds_from_config,
    configured_role_lifecycle_from_config, configured_role_metrics_profiles_from_config,
    configured_role_topups_from_config, declare_app_role_source, rename_app_role_source,
};
use super::{
    app_sources_root, config_path, plan_attach_app_role, plan_declare_app_role,
    plan_rename_app_role,
};
use crate::test_support::temp_dir;
use canic_core::bootstrap::{compiled::ConfigModel, parse_config_model};
use std::{
    fs,
    path::{Path, PathBuf},
};

mod config;
mod mutations;
mod paths;
mod roles;

fn parsed_config(source: &str) -> ConfigModel {
    parse_config_model(source).expect("valid test config")
}

const REAL_CONFIG: &str = r#"
[app]
name = "demo"
init_mode = "enabled"


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
[auth.delegated_tokens]
enabled = false

[standards]
icrc21 = true



[component_specs.user_hub]
component_role = "user_hub"
maximum_instances = 1

[component_specs.scale_hub]
component_role = "scale_hub"
maximum_instances = 1
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
