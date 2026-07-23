mod error;
mod model;
mod mutation;
mod projection;
#[cfg(test)]
mod tests;

use crate::durable_io::write_bytes;
use canic_core::bootstrap::{compiled::ConfigModel, parse_config_model};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

pub use error::{
    AppConfigDeclaration, AppConfigError, AppConfigIoOperation, AppConfigMutationConflict,
    AppConfigNameField, AppConfigNameIssue, AppConfigOperation, AppConfigPackageIssue,
    AppConfigTomlOperation,
};
pub use model::{
    AttachedFleetRole, ConfiguredPoolExpectation, ConfiguredRoleLifecycle, DeclaredFleetRole,
    LOCAL_ROOT_MIN_READY_CYCLES, RenamedFleetRole,
};
pub(super) use mutation::{
    attach_fleet_role_source, declare_fleet_role_source, rename_fleet_role_source,
};
pub use projection::configured_release_roles_from_config;
pub(super) use projection::{
    app_identity_from_source, configured_bootstrap_roles_from_config,
    configured_controllers_from_config, configured_deployable_roles_from_config,
    configured_local_root_create_cycles_from_config, configured_pool_expectations_from_config,
    configured_role_auto_create_from_config, configured_role_details_from_config,
    configured_role_kinds_from_config, configured_role_lifecycle_from_config,
    configured_role_metrics_profiles_from_config, configured_role_topups_from_config,
};

/// One immutable, validated view of an App configuration file.
///
/// Commands that need several projections should load this once so every
/// decision is derived from the same bytes on disk.
#[derive(Debug)]
pub struct AppConfigSnapshot {
    path: PathBuf,
    config: ConfigModel,
}

impl AppConfigSnapshot {
    pub fn load(path: &Path) -> Result<Self, AppConfigError> {
        let source = read_config_source(path)?;
        let config = parse_config_model(&source)
            .map_err(|source| AppConfigError::CoreConfig {
                operation: AppConfigOperation::Project,
                source,
            })
            .map_err(|error| error.at_config_path(path))?;
        Ok(Self {
            path: path.to_path_buf(),
            config,
        })
    }

    #[must_use]
    pub const fn model(&self) -> &ConfigModel {
        &self.config
    }

    #[must_use]
    pub const fn app_id(&self) -> &str {
        self.config.app_id().as_str()
    }

    #[must_use]
    pub fn deployable_roles(&self) -> Vec<String> {
        configured_deployable_roles_from_config(&self.config)
    }

    #[must_use]
    pub fn bootstrap_roles(&self) -> Vec<String> {
        configured_bootstrap_roles_from_config(&self.config)
    }

    #[must_use]
    pub fn local_root_create_cycles(&self) -> u128 {
        configured_local_root_create_cycles_from_config(&self.config)
    }

    #[must_use]
    pub fn controllers(&self) -> Vec<String> {
        configured_controllers_from_config(&self.config)
    }

    #[must_use]
    pub fn pool_expectations(&self) -> Vec<ConfiguredPoolExpectation> {
        configured_pool_expectations_from_config(&self.config)
    }

    #[must_use]
    pub fn role_lifecycle(&self) -> Vec<ConfiguredRoleLifecycle> {
        configured_role_lifecycle_from_config(&self.config)
    }

    #[must_use]
    pub fn role_kinds(&self) -> BTreeMap<String, String> {
        configured_role_kinds_from_config(&self.config)
    }

    pub fn role_capabilities(&self) -> Result<BTreeMap<String, Vec<String>>, AppConfigError> {
        let mut projected = BTreeMap::new();

        for role in self.config.attached_roles() {
            let contract = match crate::role_contract::resolve_declared_role_contract(
                &self.path,
                &self.config,
                &role,
                crate::role_contract::PackageValidationMode::Passive,
            ) {
                canic_core::role_contract::RoleContractResolution::Resolved { contract } => {
                    contract
                }
                canic_core::role_contract::RoleContractResolution::Rejected { errors } => {
                    return Err(AppConfigError::RoleContractRejected { errors });
                }
            };
            let labels = project_role_capabilities(&contract.capabilities);
            if !labels.is_empty() {
                projected.insert(role.as_str().to_string(), labels);
            }
        }

        Ok(projected)
    }

    #[must_use]
    pub fn role_auto_create(&self) -> BTreeSet<String> {
        configured_role_auto_create_from_config(&self.config)
    }

    #[must_use]
    pub fn role_topups(&self) -> BTreeMap<String, String> {
        configured_role_topups_from_config(&self.config)
    }

    #[must_use]
    pub fn role_metrics_profiles(&self) -> BTreeMap<String, String> {
        configured_role_metrics_profiles_from_config(&self.config)
    }

    #[must_use]
    pub fn role_details(&self) -> BTreeMap<String, Vec<String>> {
        configured_role_details_from_config(&self.config)
    }
}

/// Read only `[app].name` for candidate discovery and malformed-config diagnostics.
///
/// Operational projections must use [`AppConfigSnapshot`].
pub fn read_app_config_identity(path: &Path) -> Result<String, AppConfigError> {
    let source = read_config_source(path)?;
    app_identity_from_source(&source).map_err(|error| error.at_config_path(path))
}

// Validate a package-backed role declaration without writing `canic.toml`.
pub fn plan_declare_fleet_role(
    config_path: &Path,
    expected_app: &str,
    role: &str,
    package: &str,
) -> Result<DeclaredFleetRole, AppConfigError> {
    let source = read_config_source(config_path)?;
    let updated = declare_fleet_role_source(&source, expected_app, role, package)
        .map_err(|error| error.at_config_path(config_path))?;
    Ok(updated.role)
}

// Validate a package-backed role topology attachment without writing `canic.toml`.
pub fn plan_attach_fleet_role(
    config_path: &Path,
    expected_app: &str,
    role: &str,
    subnet: &str,
    kind: &str,
) -> Result<AttachedFleetRole, AppConfigError> {
    let source = read_config_source(config_path)?;
    let updated = attach_fleet_role_source(&source, expected_app, role, subnet, kind)
        .map_err(|error| error.at_config_path(config_path))?;
    Ok(updated.role)
}

// Validate a role rename and package metadata update without writing files.
pub fn plan_rename_fleet_role(
    config_path: &Path,
    expected_app: &str,
    old_role: &str,
    new_role: &str,
) -> Result<RenamedFleetRole, AppConfigError> {
    let source = read_config_source(config_path)?;
    let updated = rename_fleet_role_source(&source, config_path, expected_app, old_role, new_role)
        .map_err(|error| error.at_config_path(config_path))?;
    Ok(updated.role)
}

// Declare a package-backed role without attaching it to topology.
pub fn declare_fleet_role(
    config_path: &Path,
    expected_app: &str,
    role: &str,
    package: &str,
) -> Result<DeclaredFleetRole, AppConfigError> {
    let source = read_config_source(config_path)?;
    let updated = declare_fleet_role_source(&source, expected_app, role, package)
        .map_err(|error| error.at_config_path(config_path))?;
    write_bytes(config_path, updated.source.as_bytes()).map_err(|source| {
        AppConfigError::io(AppConfigIoOperation::WriteConfig, config_path, source)
    })?;
    Ok(updated.role)
}

// Attach a declared package-backed role directly to subnet topology.
pub fn attach_fleet_role(
    config_path: &Path,
    expected_app: &str,
    role: &str,
    subnet: &str,
    kind: &str,
) -> Result<AttachedFleetRole, AppConfigError> {
    let source = read_config_source(config_path)?;
    let updated = attach_fleet_role_source(&source, expected_app, role, subnet, kind)
        .map_err(|error| error.at_config_path(config_path))?;
    write_bytes(config_path, updated.source.as_bytes()).map_err(|source| {
        AppConfigError::io(AppConfigIoOperation::WriteConfig, config_path, source)
    })?;
    Ok(updated.role)
}

// Rename a declared role and its role-bearing topology references.
pub fn rename_fleet_role(
    config_path: &Path,
    expected_app: &str,
    old_role: &str,
    new_role: &str,
) -> Result<RenamedFleetRole, AppConfigError> {
    let source = read_config_source(config_path)?;
    let updated = rename_fleet_role_source(&source, config_path, expected_app, old_role, new_role)
        .map_err(|error| error.at_config_path(config_path))?;
    commit_role_rename_sources(
        config_path,
        &source,
        &updated.source,
        updated
            .package_manifest
            .as_deref()
            .zip(updated.package_source.as_deref()),
    )?;
    Ok(updated.role)
}

fn commit_role_rename_sources(
    config_path: &Path,
    original_config: &str,
    updated_config: &str,
    package_update: Option<(&Path, &str)>,
) -> Result<(), AppConfigError> {
    commit_role_rename_sources_with_writer(
        config_path,
        original_config,
        updated_config,
        package_update,
        write_bytes,
    )
}

fn commit_role_rename_sources_with_writer(
    config_path: &Path,
    original_config: &str,
    updated_config: &str,
    package_update: Option<(&Path, &str)>,
    mut write: impl FnMut(&Path, &[u8]) -> io::Result<()>,
) -> Result<(), AppConfigError> {
    write(config_path, updated_config.as_bytes()).map_err(|source| {
        AppConfigError::io(AppConfigIoOperation::WriteConfig, config_path, source)
    })?;
    let Some((package_path, package_source)) = package_update else {
        return Ok(());
    };

    if let Err(source) = write(package_path, package_source.as_bytes()) {
        let mutation = AppConfigError::io(
            AppConfigIoOperation::WritePackageManifest,
            package_path,
            source,
        );
        if let Err(source) = write(config_path, original_config.as_bytes()) {
            let rollback =
                AppConfigError::io(AppConfigIoOperation::RestoreConfig, config_path, source);
            return Err(AppConfigError::RollbackFailed {
                mutation: Box::new(mutation),
                rollback: Box::new(rollback),
            });
        }
        return Err(mutation);
    }

    Ok(())
}

pub(in crate::release_set) fn project_role_capabilities(
    capabilities: &BTreeSet<canic_core::role_contract::RoleCapabilityKey>,
) -> Vec<String> {
    use canic_core::role_contract::RoleCapabilityKey;

    let mut labels = BTreeSet::new();
    for capability in capabilities {
        match capability {
            RoleCapabilityKey::DelegatedTokenIssuer
            | RoleCapabilityKey::DelegatedTokenVerifier
            | RoleCapabilityKey::RoleAttestationSigner
            | RoleCapabilityKey::RoleAttestationVerifier => {
                labels.insert("auth");
            }
            RoleCapabilityKey::Directory => {
                labels.insert("directory");
            }
            RoleCapabilityKey::Icrc21 => {
                labels.insert("icrc21");
            }
            RoleCapabilityKey::Scaling => {
                labels.insert("scaling");
            }
            RoleCapabilityKey::Sharding => {
                labels.insert("sharding");
            }
            RoleCapabilityKey::Root
            | RoleCapabilityKey::RootControlPlane
            | RoleCapabilityKey::Runtime
            | RoleCapabilityKey::WasmStore => {}
        }
    }
    labels.into_iter().map(str::to_string).collect()
}

fn read_config_source(config_path: &Path) -> Result<String, AppConfigError> {
    fs::read_to_string(config_path)
        .map_err(|source| AppConfigError::io(AppConfigIoOperation::ReadConfig, config_path, source))
}
