//! Private, version-neutral source snapshot for the allocation-peer fixture.
//!
//! Normalize only the workspace's own version; external dependency requirements stay intact.

use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

/// Synthetic version used only when compiling the embedded test peer.
pub(super) const FIXTURE_VERSION: &str = "0.0.0";

/// Invocation-owned copy; removal never touches the source checkout or build cache.
pub(super) struct FixtureSource {
    pub(super) root: PathBuf,
}

impl FixtureSource {
    pub(super) fn prepare(workspace: &Path) -> io::Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let parent = workspace.join("target/embedded-root");
        fs::create_dir_all(&parent)?;
        let root = parent.join(format!("source-{}-{nonce}", std::process::id()));
        fs::create_dir(&root)?;
        let snapshot = Self { root };
        let listing = output(Command::new("git").current_dir(workspace).args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ]))?;
        for name in listing
            .split(|byte| *byte == 0)
            .filter(|name| !name.is_empty())
        {
            let name = std::str::from_utf8(name).map_err(io::Error::other)?;
            let source = workspace.join(name);
            if !source.exists() {
                continue; // An unstaged deletion is part of the current source tree.
            }
            let kind = fs::symlink_metadata(&source)?.file_type();
            if !kind.is_file() && !kind.is_symlink() {
                return Err(io::Error::other(format!(
                    "fixture source must be a file: {name}"
                )));
            }
            let destination = snapshot.root.join(name);
            fs::create_dir_all(
                destination
                    .parent()
                    .ok_or_else(|| io::Error::other("source parent"))?,
            )?;
            if kind.is_symlink() {
                std::os::unix::fs::symlink(fs::read_link(source)?, destination)?;
            } else {
                fs::copy(source, destination)?;
            }
        }
        bind_dependency_paths(&snapshot.root, workspace)?;
        normalize(&snapshot.root)?;
        Ok(snapshot)
    }
}

// Keep in-workspace dependencies in the snapshot and external dependencies at
// their original read-only locations when the workspace moves under target/.
pub(super) fn bind_dependency_paths(root: &Path, workspace: &Path) -> io::Result<()> {
    let path = root.join("Cargo.toml");
    let mut manifest: toml::Value =
        toml::from_str(&fs::read_to_string(&path)?).map_err(io::Error::other)?;
    let dependencies = manifest
        .get_mut("workspace")
        .and_then(|workspace| workspace.get_mut("dependencies"))
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| io::Error::other("fixture requires workspace dependencies"))?;
    for dependency in dependencies
        .iter_mut()
        .map(|(_, value)| value)
        .filter_map(toml::Value::as_table_mut)
    {
        let Some(selected) = dependency.get("path") else {
            continue;
        };
        let selected = selected
            .as_str()
            .ok_or_else(|| io::Error::other("dependency path must be a string"))?;
        let resolved = workspace.join(selected).canonicalize()?;
        let bound = resolved.strip_prefix(workspace).unwrap_or(&resolved);
        let bound = bound
            .to_str()
            .ok_or_else(|| io::Error::other("dependency path must be UTF-8"))?;
        dependency.insert("path".into(), bound.into());
    }
    fs::write(path, toml::to_string(&manifest).map_err(io::Error::other)?)
}

pub(super) fn normalize(root: &Path) -> io::Result<()> {
    let manifest = root.join("Cargo.toml");
    let lock = root.join("Cargo.lock");
    let before = external_packages(&fs::read_to_string(&lock)?)?;
    fs::write(
        &manifest,
        normalize_manifest(&fs::read_to_string(&manifest)?)?,
    )?;
    output(
        Command::new("cargo")
            .current_dir(root)
            .args(["update", "--workspace", "--offline"]),
    )?;
    if external_packages(&fs::read_to_string(lock)?)? != before {
        return Err(io::Error::other(
            "fixture normalization changed external dependencies",
        ));
    }
    Ok(())
}

fn external_packages(contents: &str) -> io::Result<Vec<toml::Value>> {
    let lock: toml::Value = toml::from_str(contents).map_err(io::Error::other)?;
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| io::Error::other("fixture lock package inventory missing"))?;
    Ok(packages
        .iter()
        .filter(|package| package.get("source").is_some())
        .cloned()
        .collect())
}

pub(super) fn normalize_manifest(contents: &str) -> io::Result<String> {
    let mut manifest: toml::Value = toml::from_str(contents).map_err(io::Error::other)?;
    let workspace = manifest
        .get_mut("workspace")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| io::Error::other("fixture requires a workspace manifest"))?;
    let package = workspace
        .get_mut("package")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| io::Error::other("fixture requires inherited package metadata"))?;
    let version = package
        .get("version")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| io::Error::other("workspace package version missing"))?
        .to_owned();
    package.insert("version".into(), FIXTURE_VERSION.into());
    if let Some(dependencies) = workspace
        .get_mut("dependencies")
        .and_then(toml::Value::as_table_mut)
    {
        for dependency in dependencies
            .iter_mut()
            .map(|(_, value)| value)
            .filter_map(toml::Value::as_table_mut)
        {
            if dependency.contains_key("path")
                && dependency.get("version").and_then(toml::Value::as_str) == Some(&version)
            {
                dependency.insert("version".into(), FIXTURE_VERSION.into());
            }
        }
    }
    toml::to_string(&manifest).map_err(io::Error::other)
}

impl Drop for FixtureSource {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(super) fn output(command: &mut Command) -> io::Result<Vec<u8>> {
    let result = command.output()?;
    if !result.status.success() {
        return Err(io::Error::other(format!(
            "fixture command failed ({}): {}",
            result.status,
            String::from_utf8_lossy(&result.stderr),
        )));
    }
    Ok(result.stdout)
}

/// Remap source names before code generation, including panic-location strings.
pub(super) fn rustflags(source: &Path) -> io::Result<String> {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .ok_or_else(|| io::Error::other("Cargo home is unavailable"))?
        .canonicalize()?;
    let sysroot = String::from_utf8(output(Command::new("rustc").args(["--print", "sysroot"]))?)
        .map_err(io::Error::other)?;
    let mut flags = Vec::new();
    for (path, name) in [
        (source, "/canic-fixture"),
        (cargo_home.as_path(), "/cargo"),
        (Path::new(sysroot.trim()), "/rust-toolchain"),
    ] {
        flags.push(format!("--remap-path-prefix={}={name}", path.display()));
    }
    Ok(flags.join("\u{1f}"))
}
