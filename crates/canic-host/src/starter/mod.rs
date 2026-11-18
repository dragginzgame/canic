//! New source workspace publication; no deployment, signer or journal access.
use crate::{AppConfig, CanisterDefinition, Config, Error, Result, supplier};
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
#[error(
    "starter publication failed: {source}; inspect staging {staging:?} and destination {destination:?}"
)]
struct PublicationError {
    staging: PathBuf,
    destination: PathBuf,
    #[source]
    source: Error,
}

fn manifest(path: &Path) -> Result<toml::Value> {
    let bytes = crate::store::read(path, 65_536)?;
    let text = std::str::from_utf8(&bytes).map_err(supplier)?;
    toml::from_str(text).map_err(supplier)
}

fn files(app: &str, source: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let config = Config {
        apps: BTreeMap::from([(
            app.into(),
            AppConfig {
                canisters: BTreeMap::from([
                    (
                        "root".into(),
                        CanisterDefinition {
                            role: Some("root".into()),
                            wasm: Some("artifacts/root.wasm".into()),
                            build: Some(crate::BuildInput {
                                manifest: source.join("Cargo.toml"),
                                package: "canic-root".into(),
                                client: None,
                            }),
                            ..Default::default()
                        },
                    ),
                    (
                        "store".into(),
                        CanisterDefinition {
                            role: Some("wasm-store".into()),
                            parent: Some("root".into()),
                            wasm: Some("artifacts/store.wasm".into()),
                            build: Some(crate::BuildInput {
                                manifest: source.join("Cargo.toml"),
                                package: "ic-wasm-store".into(),
                                client: None,
                            }),
                        },
                    ),
                    (
                        "app".into(),
                        CanisterDefinition {
                            role: Some("app".into()),
                            parent: Some("root".into()),
                            wasm: Some("artifacts/app.wasm".into()),
                            build: Some(crate::BuildInput {
                                manifest: "Cargo.toml".into(),
                                package: "starter-app".into(),
                                client: None,
                            }),
                        },
                    ),
                ]),
                fleets: BTreeMap::new(),
            },
        )]),
    };
    let config_bytes = crate::config::encode_config(&config)?;
    let root = manifest(&source.join("Cargo.toml"))?;
    let package = root
        .get("workspace")
        .and_then(|value| value.get("package"))
        .ok_or_else(|| Error::Invalid("Canic source requires a workspace package policy".into()))?;
    if package.get("version").and_then(toml::Value::as_str) != Some(env!("CARGO_PKG_VERSION")) {
        return Err(Error::Invalid(
            "select a Canic source checkout matching this CLI version".into(),
        ));
    }
    for name in ["canic", "canic-root"] {
        if manifest(&source.join("crates").join(name).join("Cargo.toml"))?
            .get("package")
            .and_then(|v| v.get("name"))
            .and_then(toml::Value::as_str)
            != Some(name)
        {
            return Err(Error::Invalid(format!(
                "Canic source is missing the {name} package"
            )));
        }
    }
    let workspace = &root["workspace"];
    let dependencies = workspace
        .get("dependencies")
        .ok_or_else(|| Error::Invalid("missing Canic dependencies".into()))?;
    let mut selected = serde_json::Map::new();
    let tooling = dependencies
        .get("toko-tooling")
        .ok_or_else(|| Error::Invalid("missing Canic dependency toko-tooling".into()))?;
    selected.insert(
        "toko-tooling".into(),
        serde_json::to_value(tooling).map_err(supplier)?,
    );
    selected.insert("canic".into(), json!({"path":source.join("crates/canic")}));
    let lints = workspace
        .get("lints")
        .filter(|value| value.is_table())
        .ok_or_else(|| Error::Invalid("missing Canic workspace lints".into()))?;
    let generated = json!({
        "workspace": {
            "members":["starter-app"], "resolver":"3",
            "package": {"version":"0.0.0", "edition":package.get("edition").ok_or_else(|| Error::Invalid("missing edition".into()))?, "rust-version":package.get("rust-version").ok_or_else(|| Error::Invalid("missing Rust version".into()))?, "publish":false},
            "dependencies":selected, "lints":lints,
        }
    });
    Ok(BTreeMap::from([
        ("canic.toml".into(), config_bytes),
        (
            "Cargo.toml".into(),
            toml::to_string_pretty(&generated)
                .map_err(supplier)?
                .into_bytes(),
        ),
        (
            "starter-app/Cargo.toml".into(),
            include_bytes!("templates/app.toml").to_vec(),
        ),
        (
            "starter-app/src/lib.rs".into(),
            crate::store::read(&source.join("apps/template/src/lib.rs"), 65_536)?,
        ),
        (
            "starter-app/src/endpoints/mod.rs".into(),
            crate::store::read(&source.join("apps/template/src/endpoints/mod.rs"), 65_536)?,
        ),
        (
            "rust-toolchain.toml".into(),
            crate::store::read(&source.join("rust-toolchain.toml"), 4096)?,
        ),
        (
            ".gitignore".into(),
            b"/target/\n/artifacts/\n/.canic/\n*.canic-lock\n".to_vec(),
        ),
        (
            "README.md".into(),
            include_bytes!("templates/README.md").to_vec(),
        ),
    ]))
}

/// Generate a new standalone greeting workspace and Root/Store/app declarations.
///
/// Source and config publish together under a new directory name. Existing
/// directories, files and symlinks are refused. Canic source is resolved to its
/// physical path before embedding dependency/build paths; no dependency fetch,
/// Cargo build, journal opening or network call occurs.
///
/// # Errors
/// Refuses invalid names, wrong source versions, occupied destinations,
/// missing parent directories and filesystem failures. Interrupted staging is
/// retained at the path named in the error; a publication-sync failure can leave
/// the complete final directory visible. Inspect these paths before retrying.
pub fn create_starter(destination: &Path, app: &str, source: &Path) -> Result<PathBuf> {
    let source = source.canonicalize().map_err(supplier)?;
    let files = files(app, &source)?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .map_err(supplier)?;
    let name = destination
        .file_name()
        .ok_or_else(|| Error::Invalid("starter requires a new directory name".into()))?;
    let destination = parent.join(name);
    match std::fs::symlink_metadata(&destination) {
        Ok(_) => {
            return Err(Error::Invalid(
                "starter destination already exists; nothing changed".into(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(supplier(error)),
    }
    let held_parent = ic_host_fs::Directory::open(&parent, 0)
        .map_err(supplier)?
        .ok_or_else(|| Error::Invalid("starter parent disappeared".into()))?;
    let (staging_name, held) = crate::staging_directory::allocate(&held_parent, ".canic-starter")?;
    let staging = parent.join(&staging_name);
    let publish = || -> Result<()> {
        for (path, bytes) in &files {
            let parents = path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .components()
                .map(|component| match component {
                    std::path::Component::Normal(name) => Ok(name),
                    _ => Err(Error::Invalid(
                        "starter member needs relative normal components".into(),
                    )),
                })
                .collect::<Result<Vec<_>>>()?;
            let parent = held
                .prepare_directories(&parents, 0o700, 0)
                .map_err(supplier)?;
            let name = path
                .file_name()
                .ok_or_else(|| Error::Invalid("starter member needs a filename".into()))?;
            crate::store::put_in(&parent, name, bytes)?;
        }
        for (file, bytes) in &files {
            if crate::store::read(&staging.join(file), bytes.len())? != *bytes {
                return Err(Error::Invalid(
                    "starter staging changed before publication".into(),
                ));
            }
        }
        held_parent
            .publish_directory(&staging_name, &held, name)
            .map_err(supplier)?;
        Ok(())
    };
    publish().map_err(|source| {
        supplier(PublicationError {
            staging,
            destination: destination.clone(),
            source,
        })
    })?;
    Ok(destination.join("canic.toml"))
}
