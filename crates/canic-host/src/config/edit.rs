//! Locked, comment-preserving edits to local app and fleet declarations.
use super::{
    AppConfig, CanisterBinding, CanisterDefinition, Config, FleetDeclaration, FleetNetwork,
    MAX_CONFIG_BYTES, SelectedFleet, SourceApp, SourceConfig, load_config, name,
};
use crate::{Error, Network, Result, store, supplier};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

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

fn encode_config(config: &Config) -> Result<Vec<u8>> {
    config.validate()?;
    let bytes = toml::to_string_pretty(config)
        .map_err(supplier)?
        .into_bytes();
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(Error::Invalid("config exceeds 32 KiB".into()));
    }
    Ok(bytes)
}

pub(super) fn lock_path(path: &Path) -> PathBuf {
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
        Ok(_) => edit_config(
            path,
            |config, _| {
                if config.apps.contains_key(app) {
                    return Err(Error::Invalid(
                        "app already exists; use fleet init to add a fleet".into(),
                    ));
                }
                config.apps.insert(app.into(), SourceApp::default());
                Ok(())
            },
            Ok,
        ),
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
    edit_config(
        path,
        |config, _| {
            let definition = config.apps.get_mut(app).ok_or_else(|| {
                Error::Invalid(
                    "app does not exist in this config; use app list to inspect it".into(),
                )
            })?;
            if definition.fleets.contains_key(fleet) {
                return Err(Error::Invalid(
                    "fleet already exists; nothing changed".into(),
                ));
            }
            definition.fleets.insert(
                fleet.into(),
                FleetDeclaration {
                    snapshots: None,
                    lifetime: None,
                    receiver_policy: None,
                    top_up_children: vec![],
                    role_receivers: vec![],
                    subnet_child_limit: None,
                    allow_large_subnet: false,
                    registry: None,
                    standby_pool: None,
                    worker_pools: Vec::new(),
                    shard_pools: Vec::new(),
                    auth: Vec::new(),
                    network: FleetNetwork::Bound(network),
                    journal: None,
                    canisters: BTreeMap::new(),
                },
            );
            Ok(())
        },
        |config| config.select(Some(app), Some(fleet)),
    )
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
    edit_config(
        path,
        |config, current| {
            let owner = config
                .apps
                .get_mut(app)
                .ok_or_else(|| Error::Invalid("app does not exist".into()))?;
            if current.apps[app].canisters.contains_key(slot) {
                return Err(Error::Invalid(
                    "app canister already exists; nothing changed".into(),
                ));
            }
            owner.canisters.insert(slot.into(), definition);
            Ok(())
        },
        Ok,
    )
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
    edit_config(
        path,
        |config, current| {
            let owner = config
                .apps
                .get_mut(app)
                .ok_or_else(|| Error::Invalid("app does not exist".into()))?;
            if !current.apps[app].canisters.contains_key(slot) {
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
            Ok(())
        },
        |config| config.select(Some(app), Some(fleet)),
    )
}

fn edit_config<T>(
    path: &Path,
    update: impl FnOnce(&mut SourceConfig, &Config) -> Result<()>,
    project: impl FnOnce(Config) -> Result<T>,
) -> Result<T> {
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
    let mut declarations: SourceConfig = toml::from_str(source).map_err(supplier)?;
    let current = Config::try_from(declarations.clone())?;
    current.validate()?;
    update(&mut declarations, &current)?;
    let mut document = source.parse::<toml_edit::DocumentMut>().map_err(supplier)?;
    let additions = toml::to_string(&declarations)
        .map_err(supplier)?
        .parse::<toml_edit::DocumentMut>()
        .map_err(supplier)?;
    append_declarations(document.as_table_mut(), additions.as_table());
    let text = document.to_string();
    // All current edits add declarations. Refuse any future mutation this writer
    // cannot represent rather than silently dropping it or replacing comments.
    if text.len() > MAX_CONFIG_BYTES
        || toml::from_str::<SourceConfig>(&text).map_err(supplier)? != declarations
    {
        return Err(Error::Invalid(
            "config edit exceeds bounds or changes existing declarations".into(),
        ));
    }
    let resolved = Config::try_from(declarations)?;
    resolved.validate()?;
    let result = project(resolved)?;
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
