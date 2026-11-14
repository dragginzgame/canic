//! New source workspace publication; no deployment, signer or journal access.
use crate::{
    AppConfig, CanisterDefinition, Config, Error, FleetDeclaration, Network, Result, supplier,
};
use ic_host_fs::durable;
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

fn shell_path(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .ok_or_else(|| Error::Invalid("starter paths require UTF-8".into()))?;
    Ok(format!("'{}'", value.replace('\'', "'\"'\"'")))
}

fn files(
    app: &str,
    fleet: &str,
    network: Network,
    source: &Path,
) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
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
                            ..Default::default()
                        },
                    ),
                    (
                        "app".into(),
                        CanisterDefinition {
                            role: Some("app".into()),
                            parent: Some("root".into()),
                            wasm: Some("artifacts/app.wasm".into()),
                        },
                    ),
                ]),
                fleets: BTreeMap::from([(
                    fleet.into(),
                    FleetDeclaration {
                        network,
                        journal: None,
                        canisters: BTreeMap::new(),
                    },
                )]),
            },
        )]),
    };
    let config_bytes = crate::config::encode_config(&config)?;
    let root = manifest(&source.join("Cargo.toml"))?;
    let package = root
        .get("workspace")
        .and_then(|value| value.get("package"))
        .ok_or_else(|| Error::Invalid("Canic source requires a workspace package policy".into()))?;
    if package.get("version").and_then(toml::Value::as_str) != Some(env!("CARGO_PKG_VERSION"))
        || manifest(&source.join("crates/canic/Cargo.toml"))?
            .get("package")
            .and_then(|v| v.get("name"))
            .and_then(toml::Value::as_str)
            != Some("canic")
        || manifest(&source.join("crates/canic-root/Cargo.toml"))?
            .get("package")
            .and_then(|v| v.get("name"))
            .and_then(toml::Value::as_str)
            != Some("canic-root")
    {
        return Err(Error::Invalid(
            "select a Canic source checkout matching this CLI version".into(),
        ));
    }
    let workspace = &root["workspace"];
    let dependencies = workspace
        .get("dependencies")
        .ok_or_else(|| Error::Invalid("missing Canic dependencies".into()))?;
    let mut selected = serde_json::Map::new();
    for name in ["ic-platform-runtime", "ic-wire", "toko-tooling"] {
        let value = dependencies
            .get(name)
            .ok_or_else(|| Error::Invalid(format!("missing Canic dependency {name}")))?;
        selected.insert(name.into(), serde_json::to_value(value).map_err(supplier)?);
    }
    selected.insert("canic".into(), json!({"path":source.join("crates/canic")}));
    let lints = workspace
        .get("lints")
        .filter(|value| value.is_table())
        .ok_or_else(|| Error::Invalid("missing Canic workspace lints".into()))?;
    let generated = json!({
        "workspace": {
            "members":["app"], "resolver":"3",
            "package": {"version":"0.0.0", "edition":package.get("edition").ok_or_else(|| Error::Invalid("missing edition".into()))?, "rust-version":package.get("rust-version").ok_or_else(|| Error::Invalid("missing Rust version".into()))?, "publish":false},
            "dependencies":selected, "lints":lints,
        }
    });
    let app_manifest = json!({
        "package":{"name":"starter-app", "version":{"workspace":true}, "edition":{"workspace":true}, "rust-version":{"workspace":true}, "publish":{"workspace":true}},
        "lib":{"crate-type":["cdylib","rlib"]},
        "dependencies":{"canic":{"workspace":true},"ic-wire":{"workspace":true}},
        "target":{
            "cfg(any())":{"dev-dependencies":{"toko-tooling":{"workspace":true}}},
            "cfg(target_family = \"wasm\")":{"dependencies":{"ic-platform-runtime":{"workspace":true,"features":["cdk"]}}}
        },
        "lints":{"workspace":true},
    });
    let readme = format!(
        include_str!("templates/README.md"),
        shell_path(&source.join("Cargo.toml"))?
    );
    Ok(BTreeMap::from([
        ("canic.json".into(), config_bytes),
        (
            "Cargo.toml".into(),
            toml::to_string_pretty(&generated)
                .map_err(supplier)?
                .into_bytes(),
        ),
        (
            "app/Cargo.toml".into(),
            toml::to_string_pretty(&app_manifest)
                .map_err(supplier)?
                .into_bytes(),
        ),
        (
            "app/src/lib.rs".into(),
            include_bytes!("templates/lib.rs").to_vec(),
        ),
        (
            "app/src/endpoint.rs".into(),
            include_bytes!("templates/endpoint.rs").to_vec(),
        ),
        (
            "rust-toolchain.toml".into(),
            crate::store::read(&source.join("rust-toolchain.toml"), 4096)?,
        ),
        (
            ".gitignore".into(),
            b"/target/\n/artifacts/\n/.canic/\n*.canic-lock\n".to_vec(),
        ),
        ("README.md".into(), readme.into_bytes()),
    ]))
}

/// Generate a new standalone greeting workspace and Root/app declarations.
///
/// Source and config publish together under a new directory name. Existing
/// directories, files and symlinks are refused. Canic source is resolved to its
/// physical path before embedding dependency/build paths; no dependency fetch,
/// Cargo build, journal opening or network call occurs.
///
/// # Errors
/// Refuses invalid names/networks, wrong source versions, occupied destinations,
/// missing parent directories and filesystem failures. Interrupted staging is
/// retained at the path named in the error; a publication-sync failure can leave
/// the complete final directory visible. Inspect these paths before retrying.
pub fn create_starter(
    destination: &Path,
    app: &str,
    fleet: &str,
    network: Network,
    source: &Path,
) -> Result<PathBuf> {
    let source = source.canonicalize().map_err(supplier)?;
    let files = files(app, fleet, network, &source)?;
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
    let staging = ic_host_fs::directory::create_unique_directory(
        &parent,
        ".canic-starter",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .map_err(supplier)?;
    let publish = || -> Result<()> {
        for (path, bytes) in &files {
            let output = staging.join(path);
            durable::prepare_private_parents(output.parent().unwrap_or(&staging))
                .map_err(supplier)?;
            crate::store::put(&output, bytes)?;
        }
        durable::commit_existing_directory_with(
            &staging,
            &destination,
            |_held, path| {
                for (file, bytes) in &files {
                    if crate::store::read(&path.join(file), bytes.len())? != *bytes {
                        return Err(Error::Invalid(
                            "starter staging changed before publication".into(),
                        ));
                    }
                }
                Ok::<(), Error>(())
            },
            |_, _| Ok(()),
        )
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
    Ok(destination.join("canic.json"))
}
