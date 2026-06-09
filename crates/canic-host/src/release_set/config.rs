use crate::format::cycles_tc;
use canic_core::{
    bootstrap::{compiled::MetricsProfile, parse_config_model},
    ids::CanisterRole,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use toml::Value as TomlValue;

#[derive(Clone, Copy)]
enum RootSubnetRoleScope {
    Release,
    Deployable,
}

const DEFAULT_INITIAL_CYCLES: u128 = 5_000_000_000_000;
pub const LOCAL_ROOT_MIN_READY_CYCLES: u128 = 100_000_000_000_000;
const DEFAULT_RANDOMNESS_RESEED_INTERVAL_SECS: u64 = 3600;

///
/// ConfiguredPoolExpectation
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfiguredPoolExpectation {
    pub pool: String,
    pub canister_role: String,
}

///
/// ConfiguredRoleLifecycle
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfiguredRoleLifecycle {
    pub fleet: String,
    pub role: String,
    pub display: String,
    pub declaration_kind: String,
    pub package: String,
    pub attached: bool,
    pub state: String,
    pub topology: Option<String>,
}

///
/// DeclaredFleetRole
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredFleetRole {
    pub fleet: String,
    pub role: String,
    pub display: String,
    pub package: String,
}

///
/// AttachedFleetRole
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttachedFleetRole {
    pub fleet: String,
    pub role: String,
    pub display: String,
    pub subnet: String,
    pub kind: String,
    pub topology: String,
}

///
/// RenamedFleetRole
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenamedFleetRole {
    pub fleet: String,
    pub old_role: String,
    pub new_role: String,
    pub old_display: String,
    pub new_display: String,
    pub package_manifest: Option<PathBuf>,
    pub package_manifest_note: Option<String>,
}

impl RootSubnetRoleScope {
    const fn includes_root(self) -> bool {
        matches!(self, Self::Deployable)
    }
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
    fs::write(config_path, updated.source)?;
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
    fs::write(config_path, updated.source)?;
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
    fs::write(config_path, updated.source)?;
    if let (Some(path), Some(source)) = (&updated.package_manifest, &updated.package_source) {
        fs::write(path, source)?;
    }
    Ok(updated.role)
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
    configured_role_capabilities_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
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

// Enumerate declared role lifecycle state from raw config source.
pub(super) fn configured_role_lifecycle_from_source(
    config_source: &str,
) -> Result<Vec<ConfiguredRoleLifecycle>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let fleet = config
        .fleet_name()
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?
        .to_string();
    let attached_roles = config.attached_roles();
    let mut topology = BTreeMap::<CanisterRole, Vec<String>>::new();

    for (subnet_role, subnet) in &config.subnets {
        for (role, canister) in &subnet.canisters {
            topology
                .entry(role.clone())
                .or_default()
                .push(format!("{subnet_role}/{role}"));

            if let Some(scaling) = &canister.scaling {
                for (pool, scale_pool) in &scaling.pools {
                    topology
                        .entry(scale_pool.canister_role.clone())
                        .or_default()
                        .push(format!("{subnet_role}/{role}/scaling/{pool}"));
                }
            }

            if let Some(sharding) = &canister.sharding {
                for (pool, shard_pool) in &sharding.pools {
                    topology
                        .entry(shard_pool.canister_role.clone())
                        .or_default()
                        .push(format!("{subnet_role}/{role}/sharding/{pool}"));
                }
            }

            if let Some(directory) = &canister.directory {
                for (pool, directory_pool) in &directory.pools {
                    topology
                        .entry(directory_pool.canister_role.clone())
                        .or_default()
                        .push(format!("{subnet_role}/{role}/directory/{pool}"));
                }
            }
        }
    }

    Ok(config
        .roles
        .iter()
        .map(|(role, declaration)| {
            let role_name = role.as_str().to_string();
            let attached = attached_roles.contains(role);
            ConfiguredRoleLifecycle {
                fleet: fleet.clone(),
                display: format!("{fleet}.{role}"),
                role: role_name,
                declaration_kind: if role.is_root() { "root" } else { "canister" }.to_string(),
                package: declaration.package.clone(),
                attached,
                state: if attached { "attached" } else { "declared" }.to_string(),
                topology: topology.get(role).map(|labels| labels.join(",")),
            }
        })
        .collect())
}

#[derive(Debug)]
pub(super) struct DeclaredFleetRoleSource {
    pub(super) source: String,
    pub(super) role: DeclaredFleetRole,
}

#[derive(Debug)]
pub(super) struct AttachedFleetRoleSource {
    pub(super) source: String,
    pub(super) role: AttachedFleetRole,
}

#[derive(Debug)]
pub(super) struct RenamedFleetRoleSource {
    pub(super) source: String,
    pub(super) package_manifest: Option<PathBuf>,
    pub(super) package_source: Option<String>,
    pub(super) role: RenamedFleetRole,
}

pub(super) fn declare_fleet_role_source(
    config_source: &str,
    expected_fleet: &str,
    role: &str,
    package: &str,
) -> Result<DeclaredFleetRoleSource, Box<dyn std::error::Error>> {
    let role = role.trim();
    let package = package.trim();
    if role.is_empty() {
        return Err("role must not be empty".into());
    }
    if package.is_empty() {
        return Err("package must not be empty".into());
    }
    if role == "root" {
        return Err("root role must be attached to topology; declare ordinary roles only".into());
    }
    if !role
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("role must contain only ASCII letters, numbers, '_' or '-'".into());
    }

    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let actual_fleet = config
        .fleet_name()
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?;
    if actual_fleet != expected_fleet {
        return Err(format!(
            "selected config declares fleet {actual_fleet:?}, not {expected_fleet:?}"
        )
        .into());
    }

    let role_id = CanisterRole::owned(role.to_string());
    if config.declares_role(&role_id) {
        return Err(format!("role {expected_fleet}.{role} is already declared").into());
    }

    let mut source = config_source.trim_end().to_string();
    source.push_str("\n\n[roles.");
    source.push_str(&toml_string_literal(role));
    source.push_str("]\nkind = \"canister\"\npackage = ");
    source.push_str(&toml_string_literal(package));
    source.push('\n');

    parse_config_model(&source).map_err(|err| err.to_string())?;

    Ok(DeclaredFleetRoleSource {
        source,
        role: DeclaredFleetRole {
            fleet: expected_fleet.to_string(),
            role: role.to_string(),
            display: format!("{expected_fleet}.{role}"),
            package: package.to_string(),
        },
    })
}

pub(super) fn attach_fleet_role_source(
    config_source: &str,
    expected_fleet: &str,
    role: &str,
    subnet: &str,
    kind: &str,
) -> Result<AttachedFleetRoleSource, Box<dyn std::error::Error>> {
    let role = role.trim();
    let subnet = subnet.trim();
    let kind = kind.trim();
    validate_role_name(role)?;
    validate_subnet_name(subnet)?;
    validate_attach_kind(kind)?;
    if role == "root" {
        return Err("root role must already be attached through root topology".into());
    }

    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let actual_fleet = config
        .fleet_name()
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?;
    if actual_fleet != expected_fleet {
        return Err(format!(
            "selected config declares fleet {actual_fleet:?}, not {expected_fleet:?}"
        )
        .into());
    }

    let role_id = CanisterRole::owned(role.to_string());
    config
        .roles
        .get(&role_id)
        .ok_or_else(|| format!("role {expected_fleet}.{role} is not declared"))?;
    if config.attached_roles().contains(&role_id) {
        return Err(format!("role {expected_fleet}.{role} is already attached").into());
    }

    let mut source = config_source.trim_end().to_string();
    source.push_str("\n\n[subnets.");
    source.push_str(&toml_string_literal(subnet));
    source.push_str(".canisters.");
    source.push_str(&toml_string_literal(role));
    source.push_str("]\nkind = ");
    source.push_str(&toml_string_literal(kind));
    source.push('\n');

    parse_config_model(&source).map_err(|err| err.to_string())?;

    Ok(AttachedFleetRoleSource {
        source,
        role: AttachedFleetRole {
            fleet: expected_fleet.to_string(),
            role: role.to_string(),
            display: format!("{expected_fleet}.{role}"),
            subnet: subnet.to_string(),
            kind: kind.to_string(),
            topology: format!("{subnet}/{role}"),
        },
    })
}

pub(super) fn rename_fleet_role_source(
    config_source: &str,
    config_path: &Path,
    expected_fleet: &str,
    old_role: &str,
    new_role: &str,
) -> Result<RenamedFleetRoleSource, Box<dyn std::error::Error>> {
    let old_role = old_role.trim();
    let new_role = new_role.trim();
    validate_role_name(old_role)?;
    validate_role_name(new_role)?;
    if old_role == "root" || new_role == "root" {
        return Err("root role cannot be renamed through fleet role rename".into());
    }
    if old_role == new_role {
        return Err("old role and new role must differ".into());
    }

    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let actual_fleet = config
        .fleet_name()
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?;
    if actual_fleet != expected_fleet {
        return Err(format!(
            "selected config declares fleet {actual_fleet:?}, not {expected_fleet:?}"
        )
        .into());
    }

    let old_id = CanisterRole::owned(old_role.to_string());
    let new_id = CanisterRole::owned(new_role.to_string());
    let declaration = config
        .roles
        .get(&old_id)
        .ok_or_else(|| format!("role {expected_fleet}.{old_role} is not declared"))?;
    if config.declares_role(&new_id) {
        return Err(format!("role {expected_fleet}.{new_role} is already declared").into());
    }

    let source = rename_config_role_references(config_source, old_role, new_role)?;
    parse_config_model(&source).map_err(|err| err.to_string())?;

    let (package_manifest, package_source, package_manifest_note) =
        config_path.parent().map_or_else(
            || (None, None, Some("config path has no parent".to_string())),
            |parent| {
                let manifest = parent.join(&declaration.package).join("Cargo.toml");
                match update_package_manifest_role(&manifest, expected_fleet, old_role, new_role) {
                    Ok(Some(updated)) => (Some(manifest), Some(updated), None),
                    Ok(None) => (
                        None,
                        None,
                        Some(format!(
                            "{} did not contain matching [package.metadata.canic] fleet/role metadata",
                            manifest.display()
                        )),
                    ),
                    Err(err) => (None, None, Some(err.to_string())),
                }
            },
        );

    Ok(RenamedFleetRoleSource {
        source,
        package_manifest: package_manifest.clone(),
        package_source,
        role: RenamedFleetRole {
            fleet: expected_fleet.to_string(),
            old_role: old_role.to_string(),
            new_role: new_role.to_string(),
            old_display: format!("{expected_fleet}.{old_role}"),
            new_display: format!("{expected_fleet}.{new_role}"),
            package_manifest,
            package_manifest_note,
        },
    })
}

fn rename_config_role_references(
    source: &str,
    old_role: &str,
    new_role: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let old_literal = toml_string_literal(old_role);
    let new_literal = toml_string_literal(new_role);
    let mut updated = Vec::new();

    for line in source.lines() {
        let mut line = rename_role_header(line, old_role, new_role)?;
        let trimmed = line.trim_start();
        if toml_assignment_key(trimmed) == Some("canister_role")
            || toml_assignment_key(trimmed) == Some("app_index")
        {
            line = line.replace(&old_literal, &new_literal);
        }
        updated.push(line);
    }

    let mut result = updated.join("\n");
    if source.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

fn rename_role_header(
    line: &str,
    old_role: &str,
    new_role: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') || trimmed.starts_with("[[") {
        return Ok(line.to_string());
    }

    let Some(prefix_len) = line.find('[') else {
        return Ok(line.to_string());
    };
    let inner = &trimmed[1..trimmed.len() - 1];
    let mut path = parse_toml_dotted_path(inner)?;
    let rename_roles_header = path.len() == 2 && path[0] == "roles" && path[1] == old_role;
    let rename_canister_header =
        path.len() >= 4 && path[0] == "subnets" && path[2] == "canisters" && path[3] == old_role;

    if rename_roles_header {
        path[1] = new_role.to_string();
    } else if rename_canister_header {
        path[3] = new_role.to_string();
    } else {
        return Ok(line.to_string());
    }

    Ok(format!(
        "{}[{}]",
        &line[..prefix_len],
        path.iter()
            .map(|part| toml_string_literal(part))
            .collect::<Vec<_>>()
            .join(".")
    ))
}

fn parse_toml_dotted_path(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars();
    let mut in_quote = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if !in_quote => in_quote = true,
            '"' if in_quote => in_quote = false,
            '\\' if in_quote => {
                let Some(escaped) = chars.next() else {
                    return Err("unterminated TOML escape in table header".into());
                };
                current.push(escaped);
            }
            '.' if !in_quote => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            ch => current.push(ch),
        }
    }

    if in_quote {
        return Err("unterminated quoted TOML table header".into());
    }
    parts.push(current.trim().to_string());
    Ok(parts)
}

fn toml_assignment_key(line: &str) -> Option<&str> {
    let (key, _) = line.split_once('=')?;
    Some(key.trim())
}

fn update_package_manifest_role(
    manifest: &Path,
    expected_fleet: &str,
    old_role: &str,
    new_role: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if !manifest.is_file() {
        return Ok(None);
    }

    let source = fs::read_to_string(manifest)?;
    let metadata = toml::from_str::<TomlValue>(&source)?;
    let Some(canic_metadata) = metadata
        .get("package")
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("metadata"))
        .and_then(TomlValue::as_table)
        .and_then(|metadata| metadata.get("canic"))
        .and_then(TomlValue::as_table)
    else {
        return Ok(None);
    };
    if canic_metadata.get("fleet").and_then(TomlValue::as_str) != Some(expected_fleet)
        || canic_metadata.get("role").and_then(TomlValue::as_str) != Some(old_role)
    {
        return Ok(None);
    }

    Ok(Some(rename_package_metadata_role_source(
        &source, old_role, new_role,
    )))
}

fn rename_package_metadata_role_source(source: &str, old_role: &str, new_role: &str) -> String {
    let mut in_canic_metadata = false;
    let old_literal = toml_string_literal(old_role);
    let new_literal = toml_string_literal(new_role);
    let mut lines = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_canic_metadata = trimmed == "[package.metadata.canic]";
        }
        if in_canic_metadata && toml_assignment_key(line.trim_start()) == Some("role") {
            lines.push(line.replace(&old_literal, &new_literal));
        } else {
            lines.push(line.to_string());
        }
    }

    let mut result = lines.join("\n");
    if source.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn validate_role_name(role: &str) -> Result<(), Box<dyn std::error::Error>> {
    if role.is_empty() {
        return Err("role must not be empty".into());
    }
    if !role
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("role must contain only ASCII letters, numbers, '_' or '-'".into());
    }
    Ok(())
}

fn validate_subnet_name(subnet: &str) -> Result<(), Box<dyn std::error::Error>> {
    if subnet.is_empty() {
        return Err("subnet must not be empty".into());
    }
    if !subnet
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("subnet must contain only ASCII letters, numbers, '_' or '-'".into());
    }
    Ok(())
}

fn validate_attach_kind(kind: &str) -> Result<(), Box<dyn std::error::Error>> {
    if matches!(
        kind,
        "service" | "singleton" | "shard" | "replica" | "instance"
    ) {
        return Ok(());
    }

    Err("kind must be one of: service, singleton, shard, replica, instance".into())
}

fn toml_string_literal(value: &str) -> String {
    let mut escaped = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

// Enumerate enabled config capabilities from raw config source.
pub(super) fn configured_role_capabilities_from_source(
    config_source: &str,
) -> Result<BTreeMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut capabilities = BTreeMap::<String, BTreeSet<String>>::new();

    for subnet in config.subnets.values() {
        for (role, canister) in &subnet.canisters {
            let mut role_capabilities = BTreeSet::new();
            if canister.auth.delegated_token_signer || canister.auth.role_attestation_cache {
                role_capabilities.insert("auth".to_string());
            }
            if canister.sharding.is_some() {
                role_capabilities.insert("sharding".to_string());
            }
            if canister.scaling.is_some() {
                role_capabilities.insert("scaling".to_string());
            }
            if canister.directory.is_some() {
                role_capabilities.insert("directory".to_string());
            }
            if canister.standards.icrc21 {
                role_capabilities.insert("icrc21".to_string());
            }
            if !role_capabilities.is_empty() {
                capabilities
                    .entry(role.as_str().to_string())
                    .or_default()
                    .extend(role_capabilities);
            }
        }
    }

    Ok(capabilities
        .into_iter()
        .map(|(role, capabilities)| (role, capabilities.into_iter().collect()))
        .collect())
}

// Enumerate derived auto-created service roles from raw config source.
pub(super) fn configured_role_auto_create_from_source(
    config_source: &str,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut auto_create = BTreeSet::<String>::new();

    for subnet in config.subnets.values() {
        auto_create.extend(
            subnet
                .auto_create_roles()
                .iter()
                .map(|role| role.as_str().to_string()),
        );
    }

    Ok(auto_create)
}

// Enumerate configured top-up policy summaries from raw config source.
pub(super) fn configured_role_topups_from_source(
    config_source: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut topups = BTreeMap::<String, String>::new();

    for subnet in config.subnets.values() {
        for (role, canister) in &subnet.canisters {
            if let Some(policy) = &canister.topup {
                topups.insert(
                    role.as_str().to_string(),
                    format!(
                        "{} @ {}",
                        cycles_tc(policy.amount.to_u128()),
                        cycles_tc(policy.threshold.to_u128())
                    ),
                );
            }
        }
    }

    Ok(topups)
}

// Enumerate resolved metrics profiles from raw config source.
pub(super) fn configured_role_metrics_profiles_from_source(
    config_source: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut profiles = BTreeMap::<String, String>::new();

    for subnet in config.subnets.values() {
        for (role, canister) in &subnet.canisters {
            let role_name = role.as_str().to_string();
            let profile = metrics_profile_label(canister.resolved_metrics_profile(role));
            match profiles.get(&role_name) {
                Some(existing) if existing != profile => {
                    profiles.insert(role_name, "mixed".to_string());
                }
                Some(_) => {}
                None => {
                    profiles.insert(role_name, profile.to_string());
                }
            }
        }
    }

    Ok(profiles)
}

// Estimate local root create funding from the root subnet bootstrap obligations.
pub(super) fn configured_local_root_create_cycles_from_source(
    config_source: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut root_subnet = None;

    for (subnet_role, subnet) in &config.subnets {
        if !subnet.canisters.keys().any(CanisterRole::is_root) {
            continue;
        }
        if root_subnet.is_some() {
            return Err(format!(
                "multiple subnets define a root canister; expected exactly one root subnet (found at least '{subnet_role}')"
            )
            .into());
        }
        root_subnet = Some(subnet);
    }

    let subnet = root_subnet.ok_or_else(|| {
        "no subnet defines a root canister; expected exactly one root subnet".to_string()
    })?;

    let mut cycles = subnet
        .get_canister(&CanisterRole::WASM_STORE)
        .map_or(DEFAULT_INITIAL_CYCLES, |cfg| cfg.initial_cycles.to_u128());
    for role in subnet.auto_create_roles() {
        if let Some(cfg) = subnet.get_canister(&role) {
            cycles = cycles.saturating_add(cfg.initial_cycles.to_u128());
        }
    }
    cycles = cycles.saturating_add(
        u128::from(subnet.pool.minimum_size).saturating_mul(DEFAULT_INITIAL_CYCLES),
    );

    Ok(cycles.saturating_add(LOCAL_ROOT_MIN_READY_CYCLES))
}

// Enumerate verbose configured details from raw config source.
pub(super) fn configured_role_details_from_source(
    config_source: &str,
) -> Result<BTreeMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut details = BTreeMap::<String, BTreeSet<String>>::new();

    for role in &config.app_index {
        details
            .entry(role.as_str().to_string())
            .or_default()
            .insert("app_index".to_string());
    }

    for subnet in config.subnets.values() {
        for role in subnet.auto_create_roles() {
            details
                .entry(role.as_str().to_string())
                .or_default()
                .insert("auto_create".to_string());
        }
        for role in subnet.subnet_index_roles() {
            details
                .entry(role.as_str().to_string())
                .or_default()
                .insert("subnet_index".to_string());
        }

        for (role, canister) in &subnet.canisters {
            let role_details = details.entry(role.as_str().to_string()).or_default();
            let profile = canister.resolved_metrics_profile(role);
            let profile_source = if canister.metrics.profile.is_some() {
                "configured"
            } else {
                "inferred"
            };
            role_details.insert(format!(
                "metrics profile={} tiers={} ({profile_source})",
                metrics_profile_label(profile),
                metrics_profile_tiers_label(profile)
            ));
            if canister.initial_cycles.to_u128() != DEFAULT_INITIAL_CYCLES {
                role_details.insert(format!("initial_cycles={}", canister.initial_cycles));
            }
            if !canister.randomness.enabled {
                role_details.insert("randomness=off".to_string());
            } else if randomness_source_label(canister.randomness.source) != "ic"
                || canister.randomness.reseed_interval_secs
                    != DEFAULT_RANDOMNESS_RESEED_INTERVAL_SECS
            {
                role_details.insert(format!(
                    "randomness={} reseed={}s",
                    randomness_source_label(canister.randomness.source),
                    canister.randomness.reseed_interval_secs
                ));
            }
            if canister.auth.delegated_token_signer {
                role_details.insert("auth delegated-token-signer".to_string());
            }
            if canister.auth.role_attestation_cache {
                role_details.insert("auth role-attestation-cache".to_string());
            }
            if canister.standards.icrc21 {
                role_details.insert("standard icrc21".to_string());
            }
            if let Some(scaling) = &canister.scaling {
                for (pool_name, pool) in &scaling.pools {
                    role_details.insert(format!(
                        "scaling {pool_name}->{} initial={} min={} max={}",
                        pool.canister_role.as_str(),
                        pool.policy.initial_workers,
                        pool.policy.min_workers,
                        pool.policy.max_workers
                    ));
                }
            }
            if let Some(sharding) = &canister.sharding {
                for (pool_name, pool) in &sharding.pools {
                    role_details.insert(format!(
                        "sharding {pool_name}->{} cap={} initial={} max={}",
                        pool.canister_role.as_str(),
                        pool.policy.capacity,
                        pool.policy.initial_shards,
                        pool.policy.max_shards
                    ));
                }
            }
            if let Some(directory) = &canister.directory {
                for (pool_name, pool) in &directory.pools {
                    role_details.insert(format!(
                        "directory {pool_name}->{} key={}",
                        pool.canister_role.as_str(),
                        pool.key_name
                    ));
                }
            }
        }
    }

    Ok(details
        .into_iter()
        .filter(|(_, details)| !details.is_empty())
        .map(|(role, details)| (role, details.into_iter().collect()))
        .collect())
}

fn randomness_source_label(source: impl std::fmt::Debug) -> String {
    format!("{source:?}").to_ascii_lowercase()
}

const fn metrics_profile_label(profile: MetricsProfile) -> &'static str {
    match profile {
        MetricsProfile::Leaf => "leaf",
        MetricsProfile::Hub => "hub",
        MetricsProfile::Storage => "storage",
        MetricsProfile::Root => "root",
        MetricsProfile::Full => "full",
    }
}

const fn metrics_profile_tiers_label(profile: MetricsProfile) -> &'static str {
    match profile {
        MetricsProfile::Leaf => "core,runtime,security",
        MetricsProfile::Hub => "core,placement,runtime,security",
        MetricsProfile::Storage => "core,runtime,storage",
        MetricsProfile::Root | MetricsProfile::Full => {
            "core,placement,platform,runtime,security,storage"
        }
    }
}

// Read the required operator fleet name from raw config source.
pub(super) fn configured_fleet_name_from_source(
    config_source: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = toml::from_str::<TomlValue>(config_source)?;
    let name = config
        .get("fleet")
        .and_then(TomlValue::as_table)
        .and_then(|fleet| fleet.get("name"))
        .and_then(TomlValue::as_str)
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?;
    Ok(name.to_string())
}

// Enumerate configured top-level deployment controllers from raw config source.
pub(super) fn configured_controllers_from_source(
    config_source: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut controllers = config
        .controllers
        .iter()
        .map(canic_core::cdk::types::Principal::to_text)
        .collect::<Vec<_>>();
    controllers.sort();
    controllers.dedup();
    Ok(controllers)
}

// Enumerate configured pool identities for the single subnet that owns `root`.
pub(super) fn configured_pool_expectations_from_source(
    config_source: &str,
) -> Result<Vec<ConfiguredPoolExpectation>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut root_subnet = None;

    for (subnet_role, subnet) in &config.subnets {
        if !subnet.canisters.keys().any(CanisterRole::is_root) {
            continue;
        }

        if root_subnet.is_some() {
            return Err(format!(
                "multiple subnets define a root canister; expected exactly one root subnet (found at least '{subnet_role}')"
            )
            .into());
        }

        root_subnet = Some(subnet);
    }

    let subnet = root_subnet.ok_or_else(|| {
        "no subnet defines a root canister; expected exactly one root subnet".to_string()
    })?;
    let mut pools = BTreeMap::<String, ConfiguredPoolExpectation>::new();

    for canister in subnet.canisters.values() {
        if let Some(scaling) = &canister.scaling {
            for (pool_name, pool) in &scaling.pools {
                pools.insert(
                    format!("scaling:{pool_name}:{}", pool.canister_role.as_str()),
                    ConfiguredPoolExpectation {
                        pool: pool_name.clone(),
                        canister_role: pool.canister_role.as_str().to_string(),
                    },
                );
            }
        }
        if let Some(sharding) = &canister.sharding {
            for (pool_name, pool) in &sharding.pools {
                pools.insert(
                    format!("sharding:{pool_name}:{}", pool.canister_role.as_str()),
                    ConfiguredPoolExpectation {
                        pool: pool_name.clone(),
                        canister_role: pool.canister_role.as_str().to_string(),
                    },
                );
            }
        }
        if let Some(directory) = &canister.directory {
            for (pool_name, pool) in &directory.pools {
                pools.insert(
                    format!("directory:{pool_name}:{}", pool.canister_role.as_str()),
                    ConfiguredPoolExpectation {
                        pool: pool_name.clone(),
                        canister_role: pool.canister_role.as_str().to_string(),
                    },
                );
            }
        }
    }

    Ok(pools.into_values().collect())
}

// Enumerate the configured ordinary roles for the single subnet that owns `root`.
pub(super) fn configured_release_roles_from_source(
    config_source: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    configured_root_subnet_roles_from_source(config_source, RootSubnetRoleScope::Release)
}

// Enumerate deployable roles for the single subnet that owns `root`, except the
// implicit `wasm_store` bootstrap canister.
pub(super) fn configured_deployable_roles_from_source(
    config_source: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    configured_root_subnet_roles_from_source(config_source, RootSubnetRoleScope::Deployable)
}

// Enumerate roles expected to be present once root bootstrap has completed.
pub(super) fn configured_bootstrap_roles_from_source(
    config_source: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut root_subnet = None;

    for (subnet_role, subnet) in &config.subnets {
        if !subnet.canisters.keys().any(CanisterRole::is_root) {
            continue;
        }

        if root_subnet.is_some() {
            return Err(format!(
                "multiple subnets define a root canister; expected exactly one root subnet (found at least '{subnet_role}')"
            )
            .into());
        }

        root_subnet = Some(subnet);
    }

    let subnet = root_subnet.ok_or_else(|| {
        "no subnet defines a root canister; expected exactly one root subnet".to_string()
    })?;

    let mut roles = BTreeSet::<String>::new();
    roles.insert(CanisterRole::ROOT.as_str().to_string());
    roles.extend(
        subnet
            .auto_create_roles()
            .iter()
            .map(|role| role.as_str().to_string()),
    );

    for role in subnet.auto_create_roles() {
        let Some(canister) = subnet.get_canister(&role) else {
            continue;
        };

        if let Some(sharding) = &canister.sharding {
            for pool in sharding.pools.values() {
                if pool.policy.initial_shards > 0 {
                    roles.insert(pool.canister_role.as_str().to_string());
                }
            }
        }

        if let Some(scaling) = &canister.scaling {
            for pool in scaling.pools.values() {
                if pool.policy.initial_workers > 0 {
                    roles.insert(pool.canister_role.as_str().to_string());
                }
            }
        }
    }

    Ok(sort_root_subnet_roles(roles.into_iter().collect()))
}

// Enumerate roles for the single configured subnet that owns `root`.
fn configured_root_subnet_roles_from_source(
    config_source: &str,
    scope: RootSubnetRoleScope,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut root_subnet_roles = None;

    for (subnet_role, subnet) in &config.subnets {
        if !subnet.canisters.keys().any(CanisterRole::is_root) {
            continue;
        }

        if root_subnet_roles.is_some() {
            return Err(format!(
                "multiple subnets define a root canister; expected exactly one root subnet (found at least '{subnet_role}')"
            )
            .into());
        }

        root_subnet_roles = Some(
            subnet
                .canisters
                .keys()
                .filter(|role| !role.is_wasm_store())
                .filter(|role| scope.includes_root() || !role.is_root())
                .map(|role| role.as_str().to_string())
                .collect::<Vec<_>>(),
        );
    }

    let root_subnet_roles = root_subnet_roles.ok_or_else(|| {
        "no subnet defines a root canister; expected exactly one root subnet".to_string()
    })?;

    Ok(sort_root_subnet_roles(root_subnet_roles))
}

// Sort display/build roles deterministically, keeping `root` first when present.
fn sort_root_subnet_roles(mut roles: Vec<String>) -> Vec<String> {
    roles.sort_by(|left, right| {
        match (
            left == CanisterRole::ROOT.as_str(),
            right == CanisterRole::ROOT.as_str(),
        ) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => left.cmp(right),
        }
    });
    roles
}
