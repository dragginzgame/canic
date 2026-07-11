use crate::durable_io::write_bytes;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

mod model;
mod mutation;
mod projection;
#[cfg(test)]
mod tests;

pub use model::{
    AttachedFleetRole, ConfiguredPoolExpectation, ConfiguredRoleLifecycle, DeclaredFleetRole,
    LOCAL_ROOT_MIN_READY_CYCLES, RenamedFleetRole,
};
pub(super) use mutation::{
    attach_fleet_role_source, declare_fleet_role_source, rename_fleet_role_source,
};
pub(super) use projection::{
    configured_bootstrap_roles_from_source, configured_controllers_from_source,
    configured_deployable_roles_from_source, configured_fleet_name_from_source,
    configured_local_root_create_cycles_from_source, configured_pool_expectations_from_source,
    configured_release_roles_from_source, configured_role_auto_create_from_source,
    configured_role_details_from_source, configured_role_kinds_from_source,
    configured_role_lifecycle_from_source, configured_role_metrics_profiles_from_source,
    configured_role_topups_from_source,
};

// Validate a package-backed role declaration without writing `canic.toml`.
pub fn plan_declare_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    role: &str,
    package: &str,
) -> Result<DeclaredFleetRole, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(config_path)?;
    let updated = declare_fleet_role_source(&source, expected_fleet, role, package)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()))?;
    Ok(updated.role)
}

// Validate a package-backed role topology attachment without writing `canic.toml`.
pub fn plan_attach_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    role: &str,
    subnet: &str,
    kind: &str,
) -> Result<AttachedFleetRole, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(config_path)?;
    let updated = attach_fleet_role_source(&source, expected_fleet, role, subnet, kind)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()))?;
    Ok(updated.role)
}

// Validate a role rename and package metadata update without writing files.
pub fn plan_rename_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    old_role: &str,
    new_role: &str,
) -> Result<RenamedFleetRole, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(config_path)?;
    let updated =
        rename_fleet_role_source(&source, config_path, expected_fleet, old_role, new_role)
            .map_err(|err| format!("invalid {}: {err}", config_path.display()))?;
    Ok(updated.role)
}

// Enumerate the configured ordinary roles that root must publish before bootstrap resumes.
pub fn configured_release_roles(
    config_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_release_roles_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate deployable roles in the single subnet that owns `root`.
pub fn configured_deployable_roles(
    config_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_deployable_roles_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate roles expected to exist after root bootstrap for status checks.
pub fn configured_bootstrap_roles(
    config_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_bootstrap_roles_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate the local install targets: root plus the ordinary roles owned by its subnet.
pub fn configured_install_targets(
    config_path: &Path,
    root_canister: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut targets = vec![root_canister.to_string()];
    targets.extend(configured_release_roles(config_path)?);
    Ok(targets)
}

// Estimate local root cycles needed to create bootstrap-owned canisters.
pub fn configured_local_root_create_cycles(
    config_path: &Path,
) -> Result<u128, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_local_root_create_cycles_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Read the required operator fleet name from an install config.
pub fn configured_fleet_name(config_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_fleet_name_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate configured top-level deployment controllers from an install config.
pub fn configured_controllers(
    config_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_controllers_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate configured pool identities for the single subnet that owns `root`.
pub fn configured_pool_expectations(
    config_path: &Path,
) -> Result<Vec<ConfiguredPoolExpectation>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_pool_expectations_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate declared role lifecycle state for one fleet config.
pub fn configured_role_lifecycle(
    config_path: &Path,
) -> Result<Vec<ConfiguredRoleLifecycle>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_role_lifecycle_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Declare a package-backed role without attaching it to topology.
pub fn declare_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    role: &str,
    package: &str,
) -> Result<DeclaredFleetRole, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(config_path)?;
    let updated = declare_fleet_role_source(&source, expected_fleet, role, package)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()))?;
    write_bytes(config_path, updated.source.as_bytes())?;
    Ok(updated.role)
}

// Attach a declared package-backed role directly to subnet topology.
pub fn attach_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    role: &str,
    subnet: &str,
    kind: &str,
) -> Result<AttachedFleetRole, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(config_path)?;
    let updated = attach_fleet_role_source(&source, expected_fleet, role, subnet, kind)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()))?;
    write_bytes(config_path, updated.source.as_bytes())?;
    Ok(updated.role)
}

// Rename a declared role and its role-bearing topology references.
pub fn rename_fleet_role(
    config_path: &Path,
    expected_fleet: &str,
    old_role: &str,
    new_role: &str,
) -> Result<RenamedFleetRole, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(config_path)?;
    let updated =
        rename_fleet_role_source(&source, config_path, expected_fleet, old_role, new_role)
            .map_err(|err| format!("invalid {}: {err}", config_path.display()))?;
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
) -> Result<(), Box<dyn std::error::Error>> {
    write_bytes(config_path, updated_config.as_bytes())?;
    let Some((package_path, package_source)) = package_update else {
        return Ok(());
    };

    if let Err(write_error) = write_bytes(package_path, package_source.as_bytes()) {
        if let Err(rollback_error) = write_bytes(config_path, original_config.as_bytes()) {
            return Err(format!(
                "failed to update {}: {write_error}; failed to restore {}: {rollback_error}",
                package_path.display(),
                config_path.display()
            )
            .into());
        }
        return Err(write_error.into());
    }

    Ok(())
}

// Select config paths whose required [fleet].name matches the requested fleet.
#[must_use]
pub fn matching_fleet_config_paths(choices: &[PathBuf], fleet: &str) -> Vec<PathBuf> {
    choices
        .iter()
        .filter_map(|path| match configured_fleet_name(path) {
            Ok(name) if name == fleet => Some(path.clone()),
            Ok(_) | Err(_) => None,
        })
        .collect()
}

// Enumerate configured role kinds across all subnets for operator-facing tables.
pub fn configured_role_kinds(
    config_path: &Path,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_role_kinds_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate enabled config capabilities across all configured roles.
pub fn configured_role_capabilities(
    config_path: &Path,
) -> Result<BTreeMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    let config = canic_core::bootstrap::parse_config_model(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()))?;
    let mut projected = BTreeMap::new();

    for role in config.attached_roles() {
        let contract = match crate::role_contract::resolve_declared_role_contract(
            config_path,
            &role,
            crate::role_contract::PackageValidationMode::Passive,
        ) {
            canic_core::role_contract::RoleContractResolution::Resolved { contract } => contract,
            canic_core::role_contract::RoleContractResolution::Rejected { errors } => {
                return Err(errors
                    .iter()
                    .map(|finding| {
                        format!(
                            "{}: {}",
                            finding.code(),
                            crate::role_contract::finding_detail(finding)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
                    .into());
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
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_role_auto_create_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate configured top-up policy summaries across all configured roles.
pub fn configured_role_topups(
    config_path: &Path,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_role_topups_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate resolved metrics profiles across all configured roles.
pub fn configured_role_metrics_profiles(
    config_path: &Path,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_role_metrics_profiles_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate verbose configured details across all configured roles.
pub fn configured_role_details(
    config_path: &Path,
) -> Result<BTreeMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_role_details_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}
