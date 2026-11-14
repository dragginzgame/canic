//! Local application declarations. Names describe deployments; they grant no authority.
use crate::{Error, Network, Plan, PlanInput, Result, plan, store, supplier};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;
mod unique;

const MAX_CONFIG_BYTES: usize = 32 * 1024;

/// Applications and their independently selected deployment fleets.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Configuration format, currently two.
    pub version: u32,
    /// Logical applications; each owns one or more fleets.
    #[serde(deserialize_with = "unique::map")]
    pub apps: BTreeMap<String, AppConfig>,
}

/// One logical application, which may have multiple deployed instances.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// Named deployments of this application.
    #[serde(deserialize_with = "unique::map")]
    pub fleets: BTreeMap<String, FleetConfig>,
}

/// One deployed application instance on an explicitly trusted network.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetConfig {
    /// Explicit deployment network and trust anchor.
    pub network: Network,
    /// Existing journal, relative to the configuration file unless absolute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal: Option<PathBuf>,
    /// Named canisters belonging to this fleet.
    #[serde(deserialize_with = "unique::map")]
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
        if self.version != 2 || self.apps.is_empty() || self.apps.len() > 16 {
            return Err(Error::Invalid(
                "config needs version 2 and 1–16 apps".into(),
            ));
        }
        let mut total = 0;
        let mut membership = BTreeMap::new();
        for (app, definition) in &self.apps {
            name(app)?;
            if definition.fleets.is_empty() || definition.fleets.len() > 16 {
                return Err(Error::Invalid("each app needs 1–16 fleets".into()));
            }
            for (fleet, config) in &definition.fleets {
                name(fleet)?;
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

    /// Select a fleet within its app; only a unique choice may be inferred.
    ///
    /// # Errors
    /// Refuses missing or ambiguous app/fleet names and invalid configuration.
    pub fn select(&self, app: Option<&str>, fleet: Option<&str>) -> Result<SelectedFleet> {
        self.validate()?;
        let (app, definition) = select(&self.apps, app, "app")?;
        let (fleet, config) = select(&definition.fleets, fleet, "fleet")?;
        Ok(SelectedFleet {
            app: app.clone(),
            fleet: fleet.clone(),
            config: config.clone(),
        })
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
    Err(Error::Invalid(format!(
        "select --{kind} from: {}",
        values.keys().cloned().collect::<Vec<_>>().join(", ")
    )))
}

impl FleetConfig {
    fn validate(&self) -> Result<()> {
        self.network.validate()?;
        if self
            .journal
            .as_ref()
            .is_some_and(|path| path.as_os_str().is_empty())
        {
            return Err(Error::Invalid("journal path must not be empty".into()));
        }
        if self.canisters.len() > 32 {
            return Err(Error::Invalid(
                "each fleet supports at most 32 canisters".into(),
            ));
        }
        let mut identities = std::collections::BTreeSet::new();
        let mut managers = 0;
        for (slot, declaration) in &self.canisters {
            name(slot)?;
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
}

impl SelectedFleet {
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
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        ic_host_fs::durable::prepare_private_parents(parent).map_err(supplier)?;
    }
    store::put(path, &bytes)
}

fn encode_config(config: &Config) -> Result<Vec<u8>> {
    config.validate()?;
    let bytes = serde_json::to_vec_pretty(config).map_err(supplier)?;
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

/// Initialize a new app and its first empty fleet, preserving any existing apps.
///
/// Creates a missing config exclusively; edits an existing config under the same
/// sidecar lock as [`init_fleet`]. Never opens journals or contacts a network.
/// Concurrent first-time initializers may lose the create race and must inspect
/// the resulting config before retrying.
///
/// # Errors
/// Refuses existing app names, invalid names/networks, unreadable or invalid
/// configs, symlinks, config limits and lock contention. A publication sync failure
/// can leave the complete update visible; inspect the config before retrying.
pub fn init_app(path: &Path, app: &str, fleet: &str, network: Network) -> Result<Config> {
    name(app)?;
    name(fleet)?;
    network.validate()?;
    let definition = AppConfig {
        fleets: BTreeMap::from([(
            fleet.into(),
            FleetConfig {
                network,
                journal: None,
                canisters: BTreeMap::new(),
            },
        )]),
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
                version: 2,
                apps: BTreeMap::from([(app.into(), definition)]),
            };
            init_config(path, &config)?;
            Ok(config)
        }
        Err(error) => Err(supplier(error)),
    }
}

/// Add an empty fleet to an existing app without changing other declarations or journals.
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
            FleetConfig {
                network,
                journal: None,
                canisters: BTreeMap::new(),
            },
        );
        config.select(Some(app), Some(fleet))
    })
}

/// Declare a canister slot, optionally naming an existing canister for observation.
///
/// Uses the same locked, atomic config update as [`init_fleet`]. Existing slot
/// identities and journals are never replaced or reset. An ID is a local selection,
/// not proof of existence or control. No network calls or journal access occur.
///
/// # Errors
/// Refuses missing apps/fleets, duplicate or invalid slot names, invalid principals,
/// conflicting membership on the same network, config limits and lock contention.
/// After-publication sync errors require inspecting the config.
pub fn add_canister(
    path: &Path,
    app: &str,
    fleet: &str,
    slot: &str,
    id: Option<&str>,
) -> Result<SelectedFleet> {
    name(app)?;
    name(fleet)?;
    name(slot)?;
    let id = id
        .map(crate::network::principal)
        .transpose()?
        .map(|id| id.to_text());
    edit_config(path, |config| {
        let definition = config
            .apps
            .get_mut(app)
            .and_then(|app| app.fleets.get_mut(fleet))
            .ok_or_else(|| {
                Error::Invalid(
                    "app/fleet does not exist; use fleet list to inspect this config".into(),
                )
            })?;
        if definition.canisters.contains_key(slot) {
            return Err(Error::Invalid(
                "canister slot already exists; nothing changed".into(),
            ));
        }
        definition.canisters.insert(
            slot.into(),
            CanisterConfig {
                id,
                profile_manager: false,
            },
        );
        config.select(Some(app), Some(fleet))
    })
}

fn edit_config<T>(path: &Path, update: impl FnOnce(&mut Config) -> Result<T>) -> Result<T> {
    use ic_host_fs::durable::{self, PublicationMode, WriteOptions};

    // Refuse missing/invalid input before creating a sidecar; reload under lock.
    load_config(path)?;
    let lock = durable::open_private_regular_lock_file(&lock_path(path)).map_err(supplier)?;
    durable::try_lock_exclusive(&lock).map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            Error::Invalid("config is being edited by another Canic command".into())
        } else {
            supplier(error)
        }
    })?;
    let mut config = load_config(path)?;
    let result = update(&mut config)?;
    let bytes = encode_config(&config)?;
    durable::write_bytes(
        path,
        WriteOptions {
            mode: PublicationMode::Replace,
            permissions: 0o600,
        },
        &bytes,
    )
    .map_err(supplier)?;
    Ok(result)
}

/// Read one bounded local declaration without following a file symlink.
///
/// # Errors
/// Refuses missing, malformed, unsupported or oversized configuration files.
pub fn load_config(path: &Path) -> Result<Config> {
    let config: Config =
        serde_json::from_slice(&store::read(path, MAX_CONFIG_BYTES)?).map_err(supplier)?;
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
    )
}
