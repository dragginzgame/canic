use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

// Enumerate the configured ordinary roles that root must publish before bootstrap resumes.
pub fn configured_release_roles(
    config_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_release_roles_from_source(&config_source)
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

// Read the required operator fleet name from an install config.
pub fn configured_fleet_name(config_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_fleet_name_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate configured role kinds across all subnets for operator-facing tables.
pub fn configured_role_kinds(
    config_path: &Path,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_role_kinds_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate configured role kinds from raw config source.
pub(super) fn configured_role_kinds_from_source(
    config_source: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut kinds = BTreeMap::<String, String>::new();

    for subnet in config.subnets.values() {
        for (role, canister) in &subnet.canisters {
            let role = role.as_str().to_string();
            let kind = canister.kind.to_string();
            match kinds.get(&role) {
                Some(existing) if existing != &kind => {
                    kinds.insert(role, "mixed".to_string());
                }
                Some(_) => {}
                None => {
                    kinds.insert(role, kind);
                }
            }
        }
    }

    Ok(kinds)
}

// Read the required operator fleet name from raw config source.
pub(super) fn configured_fleet_name_from_source(
    config_source: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let name = config
        .fleet
        .and_then(|fleet| fleet.name)
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?;
    Ok(name)
}

// Enumerate the configured ordinary roles for the single subnet that owns `root`.
pub(super) fn configured_release_roles_from_source(
    config_source: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut roles = BTreeSet::new();
    let mut root_subnet_roles = None;

    for (subnet_role, subnet) in &config.subnets {
        if !subnet.canisters.keys().any(CanisterRole::is_root) {
            continue;
        }

        if root_subnet_roles.is_some() {
            return Err(format!(
                "multiple subnets define a root canister; release-set staging requires exactly one root subnet (found at least '{subnet_role}')"
            )
            .into());
        }

        root_subnet_roles = Some(
            subnet
                .canisters
                .keys()
                .filter(|role| !role.is_root() && !role.is_wasm_store())
                .map(|role| role.as_str().to_string())
                .collect::<Vec<_>>(),
        );
    }

    let root_subnet_roles = root_subnet_roles.ok_or_else(|| {
        "no subnet defines a root canister; release-set staging requires exactly one root subnet"
            .to_string()
    })?;

    for role in root_subnet_roles {
        roles.insert(role);
    }

    Ok(roles.into_iter().collect())
}
