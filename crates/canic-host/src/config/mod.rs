//! Local application declarations. Names describe deployments; they grant no authority.

#[cfg(test)]
mod tests;

mod discovery;
pub(crate) mod sharding;

use crate::{Error, Network, Plan, PlanInput, Result, plan, store, supplier};
pub use discovery::{DiscoveredConfig, discover_configs, resolve_config_path};
use serde::{Deserialize, Serialize};
pub use sharding::{InstancePoolConfig, PoolGrant, ShardPoolConfig};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const MAX_CONFIG_BYTES: usize = 32 * 1024;

/// Applications and their independently selected deployment fleets.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Logical applications; each owns zero or more fleets.
    pub apps: BTreeMap<String, AppConfig>,
}

/// One logical application, which may have multiple deployed instances.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// Reusable canister definitions shared by every fleet of this app.
    pub canisters: BTreeMap<String, CanisterDefinition>,
    /// Named deployments; an application can be defined and built without any.
    #[serde(default)]
    pub fleets: BTreeMap<String, FleetDeclaration>,
}

/// Application-owned topology and optional artifact input; never deployment authority.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterDefinition {
    /// Application responsibility, independent of controllers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Immediate parent definition in this app; no implicit controller or spawn grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Wasm input relative to the config file; exact bytes are retained by planning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm: Option<PathBuf>,
    /// Local Cargo recipe; no build input grants deployment authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildInput>,
}

/// Explicit workspace manifest and package for an app-owned build.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildInput {
    /// Workspace manifest, relative to the config file unless absolute.
    pub manifest: PathBuf,
    /// Cargo package producing one Wasm library.
    pub package: String,
    /// Cargo features selected for the Wasm library build.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Optional app-owned native command codec binary target, built beside the Wasm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
}

/// Fleet-owned observation binding; no application topology or artifact overrides.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterBinding {
    /// Existing identity for observation; does not prove existence or control.
    pub id: String,
    /// Select the profile demo's read-only status adapter for this identity.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub profile_manager: bool,
}

/// A local simulator can be declared before it has an address or trust anchor.
/// Only a fully bound network can enter a deployment plan or journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FleetNetwork {
    /// Existing network and trust data, with the same encoding as retained plans.
    Bound(Network),
    /// A local simulator that has not been started.
    Unbound(UnboundLocal),
}

/// Exact declaration of a local fleet without runtime bindings.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum UnboundLocal {
    /// Empty by design. Partial trust data is invalid.
    Local {},
}

/// Deployment inputs for one instance of an app's shared definitions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetDeclaration {
    /// Optional install-time policy for explicitly selected application roles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_policy: Option<crate::ReceiverPolicy>,
    /// Combined lifetime capacity of all placement pools on this Root's managed subnet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subnet_child_limit: Option<u8>,
    /// Dedicated children owned by one keyed-instance hub.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_pool: Option<InstancePoolConfig>,
    /// Optional fleet-owned bounds for the declared shard component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shard_pool: Option<ShardPoolConfig>,
    /// Exact receiver enrollments. A historical single table is also accepted.
    #[serde(
        default,
        deserialize_with = "crate::auth::declarations",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub auth: Vec<crate::AuthConfig>,
    /// Explicit deployment network and trust anchor.
    pub network: FleetNetwork,
    /// Journal path relative to the config file unless absolute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal: Option<PathBuf>,
    /// Existing identities by app definition name; absent entries remain unbound.
    #[serde(default)]
    pub canisters: BTreeMap<String, CanisterBinding>,
}

/// Resolved app definitions and fleet bindings used by planning and observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetConfig {
    /// Optional install-time policy for explicitly selected application roles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_policy: Option<crate::ReceiverPolicy>,
    /// Combined lifetime capacity of all placement pools on this Root's managed subnet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subnet_child_limit: Option<u8>,
    /// Dedicated children owned by one keyed-instance hub.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_pool: Option<InstancePoolConfig>,
    /// Optional fleet-owned bounds for the declared shard component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shard_pool: Option<ShardPoolConfig>,
    /// Exact receiver enrollments. A historical single table is also accepted.
    #[serde(
        default,
        deserialize_with = "crate::auth::declarations",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub auth: Vec<crate::AuthConfig>,
    /// Explicit deployment network and trust anchor.
    pub network: Network,
    /// Existing journal, relative to the configuration file unless absolute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal: Option<PathBuf>,
    /// Named canisters belonging to this fleet.
    pub canisters: BTreeMap<String, CanisterConfig>,
}

/// The selected app and fleet, retained in each observation snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SelectedFleet {
    /// Logical application name.
    pub app: String,
    /// Fleet name within the application.
    pub fleet: String,
    /// Deployment configuration for this fleet only.
    pub config: FleetConfig,
}

/// A local slot declaration. An existing identity grants no deployment authority.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanisterConfig {
    /// Existing canister to observe; blocks new creation in this slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Application responsibility, independent of IC controller relationships.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Immediate parent slot in this fleet's declared application tree.
    /// This declaration grants no creation, management or routing authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// App-owned Wasm input, relative to the configuration file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm: Option<PathBuf>,
    /// Select the profile demo's read-only manager status adapter.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub profile_manager: bool,
}

/// Names retained inside the approved deployment plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentName {
    /// Fleet name at planning time. Field order preserves existing plan digests.
    pub fleet: String,
    /// App name at planning time.
    pub app: String,
    /// Canister slot name at planning time.
    pub canister: String,
}

fn name(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(Error::Invalid(
            "names must be 1–64 ASCII letters, digits, hyphens or underscores".into(),
        ));
    }
    Ok(())
}

impl DeploymentName {
    pub(crate) fn validate(&self) -> Result<()> {
        name(&self.fleet)?;
        name(&self.app)?;
        name(&self.canister)
    }
}

impl Config {
    /// Validate bounded application/fleet declarations before use.
    ///
    /// # Errors
    /// Refuses unsupported formats, invalid names, networks or oversized declarations.
    pub fn validate(&self) -> Result<()> {
        if self.apps.is_empty() || self.apps.len() > 16 {
            return Err(Error::Invalid("config needs 1–16 apps".into()));
        }
        let mut total = 0;
        let mut membership = BTreeMap::new();
        for (app, definition) in &self.apps {
            name(app)?;
            for canister in definition.canisters.values() {
                if let Some(build) = &canister.build {
                    name(&build.package)?;
                    for feature in &build.features {
                        name(feature)?;
                    }
                    if let Some(client) = &build.client {
                        name(client)?;
                    }
                    if build.manifest.as_os_str().is_empty() || canister.wasm.is_none() {
                        return Err(Error::Invalid(
                            "build recipe needs a manifest and Wasm output".into(),
                        ));
                    }
                }
            }
            validate_canisters(&definition.resolve_canisters(&BTreeMap::new()))?;
            if definition.fleets.len() > 16 {
                return Err(Error::Invalid("each app supports at most 16 fleets".into()));
            }
            for (fleet, config) in &definition.fleets {
                name(fleet)?;
                if matches!(config.network, FleetNetwork::Unbound(_)) {
                    if !config.canisters.is_empty() || !config.auth.is_empty() {
                        return Err(Error::Invalid(
                            "bind the local network before adding identities or Auth enrollment"
                                .into(),
                        ));
                    }
                    let slots = definition.resolve_slots(config)?;
                    validate_canisters(&slots)?;
                    if let Some(policy) = &config.receiver_policy {
                        policy.validate(&slots)?;
                    }
                    sharding::validate_capacity(
                        config.subnet_child_limit,
                        config.shard_pool.as_ref(),
                        config.instance_pool.as_ref(),
                    )?;
                    if let Some(pool) = &config.shard_pool {
                        pool.validate(&slots)?;
                    }
                    if let Some(pool) = &config.instance_pool {
                        pool.validate(&slots, config.shard_pool.as_ref())?;
                    }
                    total += slots.len();
                    continue;
                }
                let config = definition.resolve(config)?;
                config.validate()?;
                total += config.canisters.len();
                let network = serde_json::to_vec(&config.network).map_err(supplier)?;
                for declaration in config.canisters.values() {
                    if let Some(id) = &declaration.id {
                        let identity = crate::network::principal(id)?.to_text();
                        if let Some(owner) =
                            membership.insert((network.clone(), identity), format!("{app}/{fleet}"))
                        {
                            return Err(Error::Invalid(format!(
                                "canister is already declared in {owner} on this network; one canister cannot be a member of two fleets"
                            )));
                        }
                    }
                }
            }
        }
        if total > 128 {
            return Err(Error::Invalid("config exceeds 128 canisters".into()));
        }
        Ok(())
    }

    /// Select an app independently of its fleets; only a unique choice may be inferred.
    ///
    /// # Errors
    /// Refuses missing or ambiguous app names and invalid configuration.
    pub fn select_app(&self, app: Option<&str>) -> Result<(&str, &AppConfig)> {
        self.validate()?;
        let (name, definition) = select(&self.apps, app, "app")?;
        Ok((name.as_str(), definition))
    }

    /// Select a fleet within its app; only a unique choice may be inferred.
    ///
    /// # Errors
    /// Refuses missing or ambiguous app/fleet names and invalid configuration.
    pub fn select(&self, app: Option<&str>, fleet: Option<&str>) -> Result<SelectedFleet> {
        let (app, definition) = self.select_app(app)?;
        let (fleet, config) = select(&definition.fleets, fleet, "fleet")?;
        Ok(SelectedFleet {
            app: app.to_owned(),
            fleet: fleet.clone(),
            config: definition.resolve(config)?,
        })
    }
}

impl AppConfig {
    fn resolve(&self, fleet: &FleetDeclaration) -> Result<FleetConfig> {
        let FleetNetwork::Bound(network) = &fleet.network else {
            return Err(Error::Invalid(
                "local fleet requires its simulator URL, trust anchor and routing binding".into(),
            ));
        };
        let mut config = FleetConfig {
            receiver_policy: fleet.receiver_policy.clone(),
            subnet_child_limit: fleet.subnet_child_limit,
            instance_pool: fleet.instance_pool.clone(),
            shard_pool: fleet.shard_pool.clone(),
            auth: fleet.auth.clone(),
            network: network.clone(),
            journal: fleet.journal.clone(),
            canisters: self.resolve_slots(fleet)?,
        };
        config.resolve_receiver_grants()?;
        Ok(config)
    }

    fn resolve_slots(&self, fleet: &FleetDeclaration) -> Result<BTreeMap<String, CanisterConfig>> {
        let mut canisters = self.resolve_canisters(&fleet.canisters);
        for (root, store) in fleet
            .shard_pool
            .iter()
            .map(|p| (&p.root, &p.store))
            .chain(fleet.instance_pool.iter().map(|p| (&p.root, &p.store)))
        {
            for (slot, role) in [(root, "root"), (store, "wasm-store")] {
                if self.canisters.contains_key(slot) {
                    continue;
                }
                let binding = fleet.canisters.get(slot);
                if binding.is_some_and(|b| b.profile_manager) {
                    return Err(Error::Invalid(
                        "supplied pool infrastructure cannot be a profile manager".into(),
                    ));
                }
                canisters.insert(
                    slot.clone(),
                    CanisterConfig {
                        role: Some(role.into()),
                        id: binding.map(|b| b.id.clone()),
                        ..Default::default()
                    },
                );
            }
        }
        if fleet
            .canisters
            .keys()
            .any(|slot| !canisters.contains_key(slot))
        {
            return Err(Error::Invalid("fleet binding must name an app canister definition or supplied shard infrastructure".into()));
        }
        Ok(canisters)
    }

    fn resolve_canisters(
        &self,
        bindings: &BTreeMap<String, CanisterBinding>,
    ) -> BTreeMap<String, CanisterConfig> {
        self.canisters
            .iter()
            .map(|(slot, definition)| {
                let binding = bindings.get(slot);
                (
                    slot.clone(),
                    CanisterConfig {
                        id: binding.map(|binding| binding.id.clone()),
                        role: definition.role.clone(),
                        parent: definition.parent.clone(),
                        wasm: definition.wasm.clone(),
                        profile_manager: binding.is_some_and(|binding| binding.profile_manager),
                    },
                )
            })
            .collect()
    }
}

fn select<'a, T>(
    values: &'a BTreeMap<String, T>,
    selected: Option<&str>,
    kind: &str,
) -> Result<(&'a String, &'a T)> {
    if let Some(selected) = selected {
        if let Some(entry) = values.get_key_value(selected) {
            return Ok(entry);
        }
    } else if values.len() == 1 {
        return Ok(values.first_key_value().expect("one entry"));
    }
    if values.is_empty() {
        return Err(Error::Invalid(format!(
            "no {kind}s configured; use {kind} init to add one"
        )));
    }
    let reason = if selected.is_some() {
        format!("unknown {kind}")
    } else {
        format!("multiple {kind}s configured")
    };
    Err(Error::Invalid(format!(
        "{reason}; select --{kind} from: {}",
        values.keys().cloned().collect::<Vec<_>>().join(", ")
    )))
}

impl FleetConfig {
    fn validate(&self) -> Result<()> {
        if let Some(policy) = &self.receiver_policy {
            policy.validate(&self.canisters)?;
        }
        sharding::validate_capacity(
            self.subnet_child_limit,
            self.shard_pool.as_ref(),
            self.instance_pool.as_ref(),
        )?;
        if let Some(pool) = &self.shard_pool {
            pool.validate(&self.canisters)?;
        }
        if let Some(pool) = &self.instance_pool {
            pool.validate(&self.canisters, self.shard_pool.as_ref())?;
        }
        if self.auth.len() > canic::auth::enrollment::MAX_RECEIVERS {
            return Err(Error::Invalid("too many Auth receiver declarations".into()));
        }
        let mut receivers = std::collections::BTreeSet::new();
        for auth in &self.auth {
            if !receivers.insert(auth.grant(self)?.receiver) {
                return Err(Error::Invalid("duplicate Auth receiver binding".into()));
            }
        }
        self.network.validate()?;
        if self
            .journal
            .as_ref()
            .is_some_and(|path| path.as_os_str().is_empty())
        {
            return Err(Error::Invalid("journal path must not be empty".into()));
        }
        validate_canisters(&self.canisters)
    }
}

fn validate_canisters(canisters: &BTreeMap<String, CanisterConfig>) -> Result<()> {
    if canisters.len() > 32 {
        return Err(Error::Invalid(
            "each app supports at most 32 canisters".into(),
        ));
    }
    let mut identities = std::collections::BTreeSet::new();
    let mut managers = 0;
    for (slot, declaration) in canisters {
        name(slot)?;
        if declaration
            .wasm
            .as_ref()
            .is_some_and(|path| path.as_os_str().is_empty() || path.as_os_str().len() > 4096)
        {
            return Err(Error::Invalid("Wasm path must contain 1–4096 bytes".into()));
        }
        if let Some(role) = &declaration.role {
            name(role)?;
        }
        let mut ancestors = std::collections::BTreeSet::from([slot.as_str()]);
        let mut parent = declaration.parent.as_deref();
        while let Some(next) = parent {
            name(next)?;
            if !ancestors.insert(next) {
                return Err(Error::Invalid(format!(
                    "canister parent cycle at slot {slot}"
                )));
            }
            parent = canisters
                .get(next)
                .ok_or_else(|| {
                    Error::Invalid(format!("parent slot {next} must exist in the same app"))
                })?
                .parent
                .as_deref();
        }
        if let Some(id) = &declaration.id {
            crate::network::principal(id)?;
            if !identities.insert(id) {
                return Err(Error::Invalid(
                    "canister identity is declared in multiple slots of one fleet".into(),
                ));
            }
        }
        if declaration.profile_manager {
            if declaration.id.is_none() {
                return Err(Error::Invalid(
                    "profile manager observation requires an existing canister id".into(),
                ));
            }
            managers += 1;
        }
    }
    if managers > 16 {
        return Err(Error::Invalid("fleet exceeds 16 profile managers".into()));
    }
    Ok(())
}
impl SelectedFleet {
    /// Resolve a named canister's app-owned Wasm input beside the config file.
    ///
    /// An explicit CLI path is allowed only when the app has no artifact path.
    /// This selects input bytes, not installed code or approval authority.
    ///
    /// # Errors
    /// Refuses unknown slots, missing input and competing artifact selections.
    pub fn wasm_path(
        &self,
        config_path: &Path,
        slot: &str,
        explicit: Option<&Path>,
    ) -> Result<PathBuf> {
        let declaration = self
            .config
            .canisters
            .get(slot)
            .ok_or_else(|| Error::Invalid("unknown app canister definition".into()))?;
        match (&declaration.wasm, explicit) {
            (Some(_), Some(_)) => Err(Error::Invalid(
                "Wasm input is defined by the app; omit --wasm instead of overriding it".into(),
            )),
            (Some(path), None) => Ok(config_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(path)),
            (None, Some(path)) => Ok(path.into()),
            (None, None) => Err(Error::Invalid(
                "define the app canister's wasm input or supply --wasm".into(),
            )),
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        name(&self.app)?;
        name(&self.fleet)?;
        self.config.validate()
    }

    /// Resolve the journal beside the config, isolated by both app and fleet name.
    #[must_use]
    pub fn journal_path(&self, config_path: &Path) -> PathBuf {
        config_path.parent().unwrap_or_else(|| Path::new(".")).join(
            self.config.journal.clone().unwrap_or_else(|| {
                PathBuf::from(".canic/apps")
                    .join(&self.app)
                    .join("fleets")
                    .join(&self.fleet)
            }),
        )
    }

    pub(crate) fn known_targets(
        &self,
        records: &[(crate::FleetOperation, Network)],
    ) -> std::collections::BTreeSet<String> {
        self.config
            .canisters
            .values()
            .filter_map(|slot| slot.id.clone())
            .chain(
                records
                    .iter()
                    .filter(|(operation, network)| {
                        *network == self.config.network
                            && operation.deployment.as_ref().is_some_and(|name| {
                                name.app == self.app && name.fleet == self.fleet
                            })
                    })
                    .filter_map(|(operation, _)| operation.canister.clone()),
            )
            .collect()
    }

    pub(crate) fn contains(
        &self,
        operation: &crate::FleetOperation,
        network: &Network,
        known: &std::collections::BTreeSet<String>,
    ) -> bool {
        if let Some(name) = &operation.deployment {
            name.app == self.app && name.fleet == self.fleet
        } else {
            *network == self.config.network
                && operation
                    .canister
                    .as_ref()
                    .is_some_and(|id| known.contains(id))
        }
    }

    /// Check an explicit operation against the selected fleet without changing its authority.
    ///
    /// # Errors
    /// Refuses unreadable evidence or operations outside this app, fleet or network.
    pub fn check_operation(&self, root: &Path, id: &str) -> Result<()> {
        self.validate()?;
        {
            let store = crate::store::Store::open(root)?;
            let dir = store.operation(id)?;
            if crate::fleet_plan::exists(&dir)? {
                return if crate::fleet_plan::load(&dir, id)?.belongs(self) {
                    Ok(())
                } else {
                    Err(Error::Invalid(
                        "operation does not belong to the selected app/fleet".into(),
                    ))
                };
            }
            let (plan, _) = crate::plan::load(&dir, id)?;
            if plan.application_call().is_some() {
                return if plan.network == self.config.network
                    && plan
                        .deployment
                        .as_ref()
                        .is_some_and(|d| d.app == self.app && d.fleet == self.fleet)
                {
                    Ok(())
                } else {
                    Err(Error::Invalid(
                        "call does not belong to selected app/fleet".into(),
                    ))
                };
            }
        }
        let (operation, network) = crate::fleet::local_operation(root, id, crate::now()?)?;
        let records = crate::fleet::inventory(root, false, crate::now()?)?.records;
        if !self.contains(&operation, &network, &self.known_targets(&records))
            || network != self.config.network
        {
            return Err(Error::Invalid("operation does not belong to the selected app/fleet; select its fleet or inspect the original journal with --no-config".into()));
        }
        Ok(())
    }

    /// Check an upgrade target before retaining a plan in the selected fleet.
    ///
    /// This is a local selection check, not proof of controller authority.
    ///
    /// # Errors
    /// Refuses undeclared targets without retained named identity on this network.
    pub fn check_target(&self, root: &Path, target: &str) -> Result<()> {
        self.validate()?;
        crate::network::principal(target)?;
        let records = crate::fleet::inventory(root, true, crate::now()?)?.records;
        if !self.known_targets(&records).contains(target) {
            return Err(Error::Invalid("upgrade target does not belong to the selected app/fleet; select its fleet before planning".into()));
        }
        Ok(())
    }
}

/// Write a new local declaration without creating a journal or contacting a network.
///
/// # Errors
/// Refuses invalid declarations and existing destination files.
pub fn init_config(path: &Path, config: &Config) -> Result<()> {
    let bytes = encode_config(config)?;
    crate::output_publication::publish(
        path,
        &bytes,
        ic_host_fs::Destination::CreateNew,
        0o700,
        0o600,
    )
}

pub(super) fn encode_config(config: &Config) -> Result<Vec<u8>> {
    config.validate()?;
    let bytes = toml::to_string_pretty(config)
        .map_err(supplier)?
        .into_bytes();
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(Error::Invalid("config exceeds 32 KiB".into()));
    }
    Ok(bytes)
}

fn lock_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".canic-lock");
    value.into()
}

/// Initialize a fleetless app, preserving any existing apps and workspace files.
///
/// Creates a missing config exclusively; edits an existing config under the same
/// sidecar lock as [`init_fleet`]. Never opens journals or contacts a network.
/// Concurrent first-time initializers may lose the create race and must inspect
/// the resulting config before retrying.
///
/// # Errors
/// Refuses existing app names, invalid names, unreadable or invalid
/// configs, symlinks, config limits and lock contention. A publication sync failure
/// can leave the complete update visible; inspect the config before retrying.
pub fn init_app(path: &Path, app: &str) -> Result<Config> {
    name(app)?;
    let definition = AppConfig {
        canisters: BTreeMap::new(),
        fleets: BTreeMap::new(),
    };
    match std::fs::symlink_metadata(path) {
        Ok(_) => edit_config(path, |config| {
            if config.apps.contains_key(app) {
                return Err(Error::Invalid(
                    "app already exists; use fleet init to add a fleet".into(),
                ));
            }
            config.apps.insert(app.into(), definition);
            Ok(config.clone())
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let config = Config {
                apps: BTreeMap::from([(app.into(), definition)]),
            };
            init_config(path, &config)?;
            Ok(config)
        }
        Err(error) => Err(supplier(error)),
    }
}

/// Add a fleet using the app's shared definitions, with no identity bindings or journal creation.
///
/// Canic writers serialize through a persistent sidecar lock and publish complete
/// configuration bytes atomically. This does not coordinate external editors.
///
/// # Errors
/// Refuses missing apps, existing fleets, invalid or oversized declarations, symlink
/// inputs and lock contention. A publication sync failure can leave the complete
/// updated file visible; inspect the config before retrying.
pub fn init_fleet(path: &Path, app: &str, fleet: &str, network: Network) -> Result<SelectedFleet> {
    name(app)?;
    name(fleet)?;
    network.validate()?;
    edit_config(path, |config| {
        let definition = config.apps.get_mut(app).ok_or_else(|| {
            Error::Invalid("app does not exist in this config; use app list to inspect it".into())
        })?;
        if definition.fleets.contains_key(fleet) {
            return Err(Error::Invalid(
                "fleet already exists; nothing changed".into(),
            ));
        }
        definition.fleets.insert(
            fleet.into(),
            FleetDeclaration {
                receiver_policy: None,
                subnet_child_limit: None,
                instance_pool: None,
                shard_pool: None,
                auth: Vec::new(),
                network: FleetNetwork::Bound(network),
                journal: None,
                canisters: BTreeMap::new(),
            },
        );
        config.select(Some(app), Some(fleet))
    })
}

/// Add one reusable canister definition to an app and every fleet projection.
///
/// Existing definitions, fleet bindings and journals are never overwritten.
/// The shared locked writer validates the complete configuration before publication.
///
/// # Errors
/// Refuses unknown apps, occupied/invalid names, invalid topology, bounds and lock contention.
pub fn add_app_canister(
    path: &Path,
    app: &str,
    slot: &str,
    definition: CanisterDefinition,
) -> Result<Config> {
    name(app)?;
    name(slot)?;
    edit_config(path, |config| {
        let owner = config
            .apps
            .get_mut(app)
            .ok_or_else(|| Error::Invalid("app does not exist".into()))?;
        if owner.canisters.contains_key(slot) {
            return Err(Error::Invalid(
                "app canister already exists; nothing changed".into(),
            ));
        }
        owner.canisters.insert(slot.into(), definition);
        Ok(config.clone())
    })
}

/// Bind an app canister to an existing identity in one fleet, for observation only.
///
/// No network calls, journal access or controller changes occur. Existing bindings
/// cannot be replaced; a config edit never retargets approved operations.
///
/// # Errors
/// Refuses unknown app/fleet/definition, occupied bindings, conflicting identities,
/// invalid principals, config limits and lock contention.
pub fn bind_canister(
    path: &Path,
    app: &str,
    fleet: &str,
    slot: &str,
    mut binding: CanisterBinding,
) -> Result<SelectedFleet> {
    name(app)?;
    name(fleet)?;
    name(slot)?;
    binding.id = crate::network::principal(&binding.id)?.to_text();
    edit_config(path, |config| {
        let owner = config
            .apps
            .get_mut(app)
            .ok_or_else(|| Error::Invalid("app does not exist".into()))?;
        if !owner.canisters.contains_key(slot) {
            return Err(Error::Invalid(
                "declare the canister with app canister add before binding it".into(),
            ));
        }
        let deployment = owner
            .fleets
            .get_mut(fleet)
            .ok_or_else(|| Error::Invalid("fleet does not exist".into()))?;
        if deployment.canisters.contains_key(slot) {
            return Err(Error::Invalid(
                "canister identity is already bound; nothing changed".into(),
            ));
        }
        deployment.canisters.insert(slot.into(), binding);
        config.select(Some(app), Some(fleet))
    })
}

fn edit_config<T>(path: &Path, update: impl FnOnce(&mut Config) -> Result<T>) -> Result<T> {
    // Refuse missing/invalid input before creating a sidecar; reload under lock.
    load_config(path)?;
    let _lock = crate::sidecar_lock::acquire(&lock_path(path)).map_err(|error| {
        if matches!(&error, ic_host_fs::Error::Io(cause) if cause.kind() == std::io::ErrorKind::WouldBlock) {
            Error::Invalid("config is being edited by another Canic command".into())
        } else {
            supplier(error)
        }
    })?;
    let source = store::read(path, MAX_CONFIG_BYTES)?;
    let source = std::str::from_utf8(&source).map_err(supplier)?;
    let mut config: Config = toml::from_str(source).map_err(supplier)?;
    config.validate()?;
    let result = update(&mut config)?;
    config.validate()?;
    let mut document = source.parse::<toml_edit::DocumentMut>().map_err(supplier)?;
    let additions = toml::to_string(&config)
        .map_err(supplier)?
        .parse::<toml_edit::DocumentMut>()
        .map_err(supplier)?;
    append_declarations(document.as_table_mut(), additions.as_table());
    let text = document.to_string();
    // All current edits add declarations. Refuse any future mutation this writer
    // cannot represent rather than silently dropping it or replacing comments.
    if text.len() > MAX_CONFIG_BYTES || toml::from_str::<Config>(&text).map_err(supplier)? != config
    {
        return Err(Error::Invalid(
            "config edit exceeds bounds or changes existing declarations".into(),
        ));
    }
    let bytes = text.as_bytes();
    crate::output_publication::publish(
        path,
        bytes,
        ic_host_fs::Destination::Replace,
        0o700,
        0o600,
    )?;
    Ok(result)
}

fn append_declarations(target: &mut dyn toml_edit::TableLike, source: &toml_edit::Table) {
    for (key, value) in source {
        match target.get_mut(key) {
            Some(existing) => {
                if let (Some(existing), Some(value)) =
                    (existing.as_table_like_mut(), value.as_table())
                {
                    append_declarations(existing, value);
                }
            }
            None => {
                target.insert(key, value.clone());
            }
        }
    }
}

/// Read one bounded local declaration without following a file symlink.
///
/// # Errors
/// Refuses missing, malformed, unsupported or oversized configuration files.
pub fn load_config(path: &Path) -> Result<Config> {
    let bytes = store::read(path, MAX_CONFIG_BYTES)?;
    let config: Config =
        toml::from_str(std::str::from_utf8(&bytes).map_err(supplier)?).map_err(supplier)?;
    config.validate()?;
    Ok(config)
}

/// Plan the first deployment of a named slot, retaining its original operation on repetition.
///
/// # Errors
/// Refuses undeclared slots, conflicting plans, or attempts to repurpose an occupied slot.
pub fn create_configured_plan(
    root: &Path,
    wasm: &Path,
    input: PlanInput,
    selection: &SelectedFleet,
    canister: &str,
) -> Result<(String, Plan)> {
    selection.validate()?;
    if selection.config.receiver_selected(canister) {
        return Err(Error::Invalid("selected Auth role needs its approved receiver install arguments; use a managed Store release or supply the receiver envelope to the independent installer".into()));
    }
    if selection
        .config
        .canisters
        .get(canister)
        .is_some_and(|slot| slot.parent.is_some())
    {
        return Err(Error::Invalid(
            "a child slot requires explicit Root-owned planning with --root-module".into(),
        ));
    }
    if input.network != selection.config.network
        || input.target.is_some()
        || input.previous_module.is_some()
        || !selection
            .config
            .canisters
            .get(canister)
            .is_some_and(|slot| slot.id.is_none())
    {
        return Err(Error::Invalid("select an unbound canister in this app/fleet for first deployment; existing canisters require an explicit target/hash upgrade plan".into()));
    }
    plan::retain(
        root,
        wasm,
        input,
        Some(DeploymentName {
            fleet: selection.fleet.clone(),
            app: selection.app.clone(),
            canister: canister.into(),
        }),
        None,
        crate::now()?,
    )
}
