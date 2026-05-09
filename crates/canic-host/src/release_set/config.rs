use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Clone, Copy)]
enum RootSubnetRoleScope {
    Release,
    Fleet,
}

const DEFAULT_INITIAL_CYCLES: u128 = 5_000_000_000_000;
const LOCAL_ROOT_BOOTSTRAP_RESERVE_CYCLES: u128 = 50_000_000_000_000;
const DEFAULT_RANDOMNESS_RESEED_INTERVAL_SECS: u64 = 3600;
const TENTH_TC: u128 = 100_000_000_000;

impl RootSubnetRoleScope {
    const fn includes_root(self) -> bool {
        matches!(self, Self::Fleet)
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

// Enumerate the configured fleet roles in the single subnet that owns `root`.
pub fn configured_fleet_roles(
    config_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_fleet_roles_from_source(&config_source)
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

// Enumerate roles declared in subnet auto_create sets.
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

// Enumerate auto-created roles from raw config source.
pub(super) fn configured_role_auto_create_from_source(
    config_source: &str,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut auto_create = BTreeSet::<String>::new();

    for subnet in config.subnets.values() {
        auto_create.extend(
            subnet
                .auto_create
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
            if let Some(policy) = &canister.topup_policy {
                topups.insert(
                    role.as_str().to_string(),
                    format!(
                        "{} @ {}",
                        format_cycles_tenths(policy.amount.to_u128()),
                        format_cycles_tenths(policy.threshold.to_u128())
                    ),
                );
            }
        }
    }

    Ok(topups)
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
    for role in &subnet.auto_create {
        if let Some(cfg) = subnet.get_canister(role) {
            cycles = cycles.saturating_add(cfg.initial_cycles.to_u128());
        }
    }
    cycles = cycles.saturating_add(
        u128::from(subnet.pool.minimum_size).saturating_mul(DEFAULT_INITIAL_CYCLES),
    );

    Ok(cycles.saturating_add(LOCAL_ROOT_BOOTSTRAP_RESERVE_CYCLES))
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
        for role in &subnet.auto_create {
            details
                .entry(role.as_str().to_string())
                .or_default()
                .insert("auto_create".to_string());
        }
        for role in &subnet.subnet_index {
            details
                .entry(role.as_str().to_string())
                .or_default()
                .insert("subnet_index".to_string());
        }

        for (role, canister) in &subnet.canisters {
            let role_details = details.entry(role.as_str().to_string()).or_default();
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

fn format_cycles_tenths(cycles: u128) -> String {
    let tenths = cycles.saturating_add(TENTH_TC / 2) / TENTH_TC;
    format!("{}.{}TC", tenths / 10, tenths % 10)
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
    configured_root_subnet_roles_from_source(config_source, RootSubnetRoleScope::Release)
}

// Enumerate all configured roles for the single subnet that owns `root`, except
// the implicit `wasm_store` bootstrap canister.
pub(super) fn configured_fleet_roles_from_source(
    config_source: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    configured_root_subnet_roles_from_source(config_source, RootSubnetRoleScope::Fleet)
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
