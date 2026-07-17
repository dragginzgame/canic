mod error;
mod model;
mod mutation;
mod projection;
#[cfg(test)]
mod tests;

use crate::durable_io::write_bytes;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::Path,
};

pub use error::{
    FleetConfigDeclaration, FleetConfigError, FleetConfigIoOperation, FleetConfigMutationConflict,
    FleetConfigNameField, FleetConfigNameIssue, FleetConfigOperation, FleetConfigPackageIssue,
    FleetConfigTomlOperation,
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
    configured_bootstrap_roles_from_source, configured_controllers_from_source,
    configured_deployable_roles_from_source, configured_fleet_name_from_source,
    configured_local_root_create_cycles_from_source, configured_pool_expectations_from_source,
    configured_role_auto_create_from_source, configured_role_details_from_source,
    configured_role_kinds_from_source, configured_role_lifecycle_from_source,
    configured_role_metrics_profiles_from_source, configured_role_topups_from_source,
};

// Validate a package-backed role declaration without writing `canic.toml`.
pub fn plan_declare_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    role: &str,
    package: &str,
) -> Result<DeclaredFleetRole, FleetConfigError> {
    let source = read_config_source(config_path)?;
    let updated = declare_fleet_role_source(&source, expected_fleet, role, package)
        .map_err(|error| error.at_config_path(config_path))?;
    Ok(updated.role)
}

// Validate a package-backed role topology attachment without writing `canic.toml`.
pub fn plan_attach_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    role: &str,
    subnet: &str,
    kind: &str,
) -> Result<AttachedFleetRole, FleetConfigError> {
    let source = read_config_source(config_path)?;
    let updated = attach_fleet_role_source(&source, expected_fleet, role, subnet, kind)
        .map_err(|error| error.at_config_path(config_path))?;
    Ok(updated.role)
}

// Validate a role rename and package metadata update without writing files.
pub fn plan_rename_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    old_role: &str,
    new_role: &str,
) -> Result<RenamedFleetRole, FleetConfigError> {
    let source = read_config_source(config_path)?;
    let updated =
        rename_fleet_role_source(&source, config_path, expected_fleet, old_role, new_role)
            .map_err(|error| error.at_config_path(config_path))?;
    Ok(updated.role)
}

// Enumerate deployable roles in the single subnet that owns `root`.
pub fn configured_deployable_roles(config_path: &Path) -> Result<Vec<String>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_deployable_roles_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Enumerate roles expected to exist after root bootstrap for status checks.
pub fn configured_bootstrap_roles(config_path: &Path) -> Result<Vec<String>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_bootstrap_roles_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Estimate local root cycles needed to create bootstrap-owned canisters.
pub fn configured_local_root_create_cycles(config_path: &Path) -> Result<u128, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_local_root_create_cycles_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Read the required operator fleet name from an install config.
pub fn configured_fleet_name(config_path: &Path) -> Result<String, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_fleet_name_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Enumerate configured top-level deployment controllers from an install config.
pub fn configured_controllers(config_path: &Path) -> Result<Vec<String>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_controllers_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Enumerate configured pool identities for the single subnet that owns `root`.
pub fn configured_pool_expectations(
    config_path: &Path,
) -> Result<Vec<ConfiguredPoolExpectation>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_pool_expectations_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Enumerate declared role lifecycle state for one fleet config.
pub fn configured_role_lifecycle(
    config_path: &Path,
) -> Result<Vec<ConfiguredRoleLifecycle>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_role_lifecycle_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Declare a package-backed role without attaching it to topology.
pub fn declare_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    role: &str,
    package: &str,
) -> Result<DeclaredFleetRole, FleetConfigError> {
    let source = read_config_source(config_path)?;
    let updated = declare_fleet_role_source(&source, expected_fleet, role, package)
        .map_err(|error| error.at_config_path(config_path))?;
    write_bytes(config_path, updated.source.as_bytes()).map_err(|source| {
        FleetConfigError::io(FleetConfigIoOperation::WriteConfig, config_path, source)
    })?;
    Ok(updated.role)
}

// Attach a declared package-backed role directly to subnet topology.
pub fn attach_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    role: &str,
    subnet: &str,
    kind: &str,
) -> Result<AttachedFleetRole, FleetConfigError> {
    let source = read_config_source(config_path)?;
    let updated = attach_fleet_role_source(&source, expected_fleet, role, subnet, kind)
        .map_err(|error| error.at_config_path(config_path))?;
    write_bytes(config_path, updated.source.as_bytes()).map_err(|source| {
        FleetConfigError::io(FleetConfigIoOperation::WriteConfig, config_path, source)
    })?;
    Ok(updated.role)
}

// Rename a declared role and its role-bearing topology references.
pub fn rename_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    old_role: &str,
    new_role: &str,
) -> Result<RenamedFleetRole, FleetConfigError> {
    let source = read_config_source(config_path)?;
    let updated =
        rename_fleet_role_source(&source, config_path, expected_fleet, old_role, new_role)
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
) -> Result<(), FleetConfigError> {
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
) -> Result<(), FleetConfigError> {
    write(config_path, updated_config.as_bytes()).map_err(|source| {
        FleetConfigError::io(FleetConfigIoOperation::WriteConfig, config_path, source)
    })?;
    let Some((package_path, package_source)) = package_update else {
        return Ok(());
    };

    if let Err(source) = write(package_path, package_source.as_bytes()) {
        let mutation = FleetConfigError::io(
            FleetConfigIoOperation::WritePackageManifest,
            package_path,
            source,
        );
        if let Err(source) = write(config_path, original_config.as_bytes()) {
            let rollback =
                FleetConfigError::io(FleetConfigIoOperation::RestoreConfig, config_path, source);
            return Err(FleetConfigError::RollbackFailed {
                mutation: Box::new(mutation),
                rollback: Box::new(rollback),
            });
        }
        return Err(mutation);
    }

    Ok(())
}

// Enumerate configured role kinds across all subnets for operator-facing tables.
pub fn configured_role_kinds(
    config_path: &Path,
) -> Result<BTreeMap<String, String>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_role_kinds_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Enumerate enabled config capabilities across all configured roles.
pub fn configured_role_capabilities(
    config_path: &Path,
) -> Result<BTreeMap<String, Vec<String>>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    let config = canic_core::bootstrap::parse_config_model(&config_source)
        .map_err(|source| FleetConfigError::CoreConfig {
            operation: FleetConfigOperation::Project,
            source,
        })
        .map_err(|error| error.at_config_path(config_path))?;
    let mut projected = BTreeMap::new();

    for role in config.attached_roles() {
        let contract = match crate::role_contract::resolve_declared_role_contract(
            config_path,
            &role,
            crate::role_contract::PackageValidationMode::Passive,
        ) {
            canic_core::role_contract::RoleContractResolution::Resolved { contract } => contract,
            canic_core::role_contract::RoleContractResolution::Rejected { errors } => {
                return Err(FleetConfigError::RoleContractRejected { errors });
            }
        };
        let labels = project_role_capabilities(&contract.capabilities);
        if !labels.is_empty() {
            projected.insert(role.as_str().to_string(), labels);
        }
    }

    Ok(projected)
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
            RoleCapabilityKey::IcpRefill => {
                labels.insert("icp_refill");
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

// Enumerate roles derived for root auto-create.
pub fn configured_role_auto_create(
    config_path: &Path,
) -> Result<BTreeSet<String>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_role_auto_create_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Enumerate configured top-up policy summaries across all configured roles.
pub fn configured_role_topups(
    config_path: &Path,
) -> Result<BTreeMap<String, String>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_role_topups_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Enumerate resolved metrics profiles across all configured roles.
pub fn configured_role_metrics_profiles(
    config_path: &Path,
) -> Result<BTreeMap<String, String>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_role_metrics_profiles_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

// Enumerate verbose configured details across all configured roles.
pub fn configured_role_details(
    config_path: &Path,
) -> Result<BTreeMap<String, Vec<String>>, FleetConfigError> {
    let config_source = read_config_source(config_path)?;
    configured_role_details_from_source(&config_source)
        .map_err(|error| error.at_config_path(config_path))
}

fn read_config_source(config_path: &Path) -> Result<String, FleetConfigError> {
    fs::read_to_string(config_path).map_err(|source| {
        FleetConfigError::io(FleetConfigIoOperation::ReadConfig, config_path, source)
    })
}
