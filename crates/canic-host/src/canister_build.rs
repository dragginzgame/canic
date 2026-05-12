use crate::{
    artifact_io::{
        embed_candid_metadata, maybe_shrink_wasm_artifact, write_bytes_atomically,
        write_gzip_artifact, write_wasm_artifact,
    },
    bootstrap_store::{BootstrapWasmStoreBuildOutput, build_bootstrap_wasm_store_artifact},
    cargo_command, icp_environment_from_env,
    release_set::{
        canister_manifest_path, emit_root_release_set_manifest_if_ready, icp_root, workspace_root,
    },
    remove_optional_file, should_export_candid_artifacts,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use toml::Value as TomlValue;

pub use crate::build_profile::CanisterBuildProfile;

const ROOT_ROLE: &str = "root";
const WASM_STORE_ROLE: &str = "wasm_store";
const LOCAL_ARTIFACT_ROOT_RELATIVE: &str = ".icp/local/canisters";
const WASM_TARGET: &str = "wasm32-unknown-unknown";

///
/// CanisterArtifactBuildOutput
///

#[derive(Clone, Debug)]
pub struct CanisterArtifactBuildOutput {
    pub artifact_root: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub did_path: PathBuf,
    pub manifest_path: Option<PathBuf>,
}

///
/// WorkspaceBuildContext
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceBuildContext {
    pub profile: String,
    pub requested_profile: String,
    pub network: String,
    pub workspace_root: PathBuf,
    pub icp_root: PathBuf,
}

impl WorkspaceBuildContext {
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            "Canic build:".to_string(),
            format!("profile: {}", self.profile),
            format!("network: {}", self.network),
            format!("workspace: {}", self.workspace_root.display()),
        ];

        if self.requested_profile != "unset" {
            lines.push(format!("requested profile: {}", self.requested_profile));
        }
        if self.icp_root != self.workspace_root {
            lines.push(format!("icp root: {}", self.icp_root.display()));
        }

        lines
    }
}

// Print the current build context once per caller session so caller builds
// stay readable without repeating root/profile diagnostics for every canister.
pub fn print_current_workspace_build_context_once(
    profile: CanisterBuildProfile,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(context) = current_workspace_build_context_once(profile)? {
        eprintln!("{}", context.lines().join("\n"));
    }

    Ok(())
}

// Return the current build context once per caller session.
pub fn current_workspace_build_context_once(
    profile: CanisterBuildProfile,
) -> Result<Option<WorkspaceBuildContext>, Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let icp_root = icp_root()?;
    let marker_dir = icp_root.join(".icp");
    fs::create_dir_all(&marker_dir)?;

    let requested_profile = env::var("CANIC_WASM_PROFILE").unwrap_or_else(|_| "unset".to_string());
    let network = icp_environment_from_env();
    let marker_key = env::var("CANIC_BUILD_CONTEXT_SESSION")
        .ok()
        .unwrap_or_else(|| {
            icp_ancestor_process_id()
                .or_else(parent_process_id)
                .unwrap_or_else(std::process::id)
                .to_string()
        });
    let marker_file = marker_dir.join(format!(".canic-build-context-{marker_key}"));

    if marker_file.exists() {
        return Ok(None);
    }

    fs::write(&marker_file, [])?;
    Ok(Some(WorkspaceBuildContext {
        profile: profile.target_dir_name().to_string(),
        requested_profile,
        network,
        workspace_root,
        icp_root,
    }))
}

// Build one visible Canic canister artifact for the current workspace.
pub fn build_current_workspace_canister_artifact(
    canister_name: &str,
    profile: CanisterBuildProfile,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let icp_root = icp_root()?;
    build_canister_artifact(&workspace_root, &icp_root, canister_name, profile)
}

// Build one visible Canic canister artifact and keep the thin-root special cases.
fn build_canister_artifact(
    workspace_root: &Path,
    icp_root: &Path,
    canister_name: &str,
    profile: CanisterBuildProfile,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    if canister_name == WASM_STORE_ROLE {
        return build_hidden_wasm_store_artifact(workspace_root, icp_root, profile);
    }

    let canister_manifest_path = canister_manifest_path(workspace_root, canister_name);
    let canister_package_name = load_canister_package_name(&canister_manifest_path)?;
    let artifact_root = icp_root
        .join(LOCAL_ARTIFACT_ROOT_RELATIVE)
        .join(canister_name);
    let wasm_path = artifact_root.join(format!("{canister_name}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{canister_name}.wasm.gz"));
    let did_path = artifact_root.join(format!("{canister_name}.did"));
    let require_embedded_release_artifacts = canister_name == ROOT_ROLE;

    if require_embedded_release_artifacts {
        build_bootstrap_wasm_store_artifact(workspace_root, icp_root, profile)?;
    }

    fs::create_dir_all(&artifact_root)?;
    remove_stale_icp_candid_sidecars(&artifact_root)?;

    let release_wasm_path = run_canister_build(
        workspace_root,
        icp_root,
        &canister_manifest_path,
        &canister_package_name,
        profile,
        require_embedded_release_artifacts,
    )?;
    write_wasm_artifact(&release_wasm_path, &wasm_path)?;
    maybe_shrink_wasm_artifact(&wasm_path)?;

    let network = icp_environment_from_env();
    if should_export_candid_artifacts(&network) {
        let debug_wasm_path = run_canister_build(
            workspace_root,
            icp_root,
            &canister_manifest_path,
            &canister_package_name,
            CanisterBuildProfile::Debug,
            require_embedded_release_artifacts,
        )?;
        extract_candid(&debug_wasm_path, &did_path)?;
        embed_candid_metadata(&wasm_path, &did_path)?;
    } else {
        remove_optional_file(&did_path)?;
    }
    write_gzip_artifact(&wasm_path, &wasm_gz_path)?;

    let manifest_path =
        emit_root_release_set_manifest_if_ready(workspace_root, icp_root, &network)?;

    Ok(CanisterArtifactBuildOutput {
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        manifest_path,
    })
}

// Read the real package name from one canister manifest so downstreams are not
// forced to mirror the reference `canister_<role>` naming scheme.
fn load_canister_package_name(manifest_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(manifest_path)?;
    let manifest = toml::from_str::<TomlValue>(&manifest_source)?;
    let package_name = manifest
        .get("package")
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("name"))
        .and_then(TomlValue::as_str)
        .ok_or_else(|| format!("missing package.name in {}", manifest_path.display()))?;

    Ok(package_name.to_string())
}

// Run one wasm-target cargo build for the requested canister manifest/profile.
fn run_canister_build(
    workspace_root: &Path,
    icp_root: &Path,
    manifest_path: &Path,
    package_name: &str,
    profile: CanisterBuildProfile,
    require_embedded_release_artifacts: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace_root.join("target"), PathBuf::from);
    let mut command = cargo_command();
    command
        .current_dir(workspace_root)
        .env("CARGO_TARGET_DIR", &target_root)
        .env("CANIC_ICP_ROOT", icp_root)
        .args([
            "build",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--target",
            WASM_TARGET,
        ])
        .args(profile.cargo_args());

    if require_embedded_release_artifacts {
        command.env("CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS", "1");
    }

    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "cargo build failed for {}: {}",
            manifest_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(target_root
        .join(WASM_TARGET)
        .join(profile.target_dir_name())
        .join(format!("{}.wasm", package_name.replace('-', "_"))))
}

// Extract the service `.did` from one debug wasm so Candid stays deterministic.
fn extract_candid(
    debug_wasm_path: &Path,
    did_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("candid-extractor")
        .arg(debug_wasm_path)
        .output()
        .map_err(|err| {
            format!(
                "failed to run candid-extractor for {}: {err}",
                debug_wasm_path.display()
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "candid-extractor failed for {}: {}",
            debug_wasm_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    write_bytes_atomically(did_path, &output.stdout)?;
    Ok(())
}

// Remove stale ICP-generated Candid sidecars so local surface scans match the
// extracted `<role>.did` artifact we actually ship and verify.
fn remove_stale_icp_candid_sidecars(artifact_root: &Path) -> std::io::Result<()> {
    for relative in [
        "constructor.did",
        "service.did",
        "service.did.d.ts",
        "service.did.js",
    ] {
        let path = artifact_root.join(relative);
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    Ok(())
}

// Route the implicit bootstrap store through the published public builder.
fn build_hidden_wasm_store_artifact(
    workspace_root: &Path,
    icp_root: &Path,
    profile: CanisterBuildProfile,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let output = build_bootstrap_wasm_store_artifact(workspace_root, icp_root, profile)?;
    Ok(map_bootstrap_output(output))
}

// Normalize the bootstrap store builder output to the public canister-artifact shape.
fn map_bootstrap_output(output: BootstrapWasmStoreBuildOutput) -> CanisterArtifactBuildOutput {
    CanisterArtifactBuildOutput {
        artifact_root: output.artifact_root,
        wasm_path: output.wasm_path,
        wasm_gz_path: output.wasm_gz_path,
        did_path: output.did_path,
        manifest_path: None,
    }
}

// Read the current parent process id from Linux procfs when available.
fn parent_process_id() -> Option<u32> {
    let stat = fs::read_to_string("/proc/self/stat").ok()?;
    parse_parent_process_id(&stat)
}

// Walk ancestor processes until the wrapping `icp` process is found.
fn icp_ancestor_process_id() -> Option<u32> {
    let mut pid = parent_process_id()?;
    loop {
        if process_comm(pid).as_deref() == Some("icp") {
            return Some(pid);
        }

        let parent = process_parent_id(pid)?;
        if parent == 0 || parent == pid {
            return None;
        }
        pid = parent;
    }
}

// Read one ancestor's parent process id from procfs.
fn process_parent_id(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_parent_process_id(&stat)
}

// Read one process command name from procfs.
fn process_comm(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|comm| comm.trim().to_string())
}

// Parse Linux `/proc/<pid>/stat` enough to extract the parent process id.
fn parse_parent_process_id(stat: &str) -> Option<u32> {
    let (_, suffix) = stat.rsplit_once(") ")?;
    let mut parts = suffix.split_whitespace();
    let _state = parts.next()?;
    parts.next()?.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::{parse_parent_process_id, remove_stale_icp_candid_sidecars};
    use crate::test_support::temp_dir;
    use std::fs;

    #[test]
    fn parse_parent_process_id_accepts_proc_stat_shape() {
        let stat = "12345 (build_canister_ar) S 67890 0 0 0";
        assert_eq!(parse_parent_process_id(stat), Some(67890));
    }

    #[test]
    fn remove_stale_icp_candid_sidecars_keeps_primary_role_did() {
        let temp_root = temp_dir("canic-canister-build-sidecars");
        let _ = fs::remove_dir_all(&temp_root);
        fs::create_dir_all(&temp_root).unwrap();

        for name in [
            "constructor.did",
            "service.did",
            "service.did.d.ts",
            "service.did.js",
            "app.did",
        ] {
            fs::write(temp_root.join(name), "x").unwrap();
        }

        remove_stale_icp_candid_sidecars(&temp_root).unwrap();

        assert!(!temp_root.join("constructor.did").exists());
        assert!(!temp_root.join("service.did").exists());
        assert!(!temp_root.join("service.did.d.ts").exists());
        assert!(!temp_root.join("service.did.js").exists());
        assert!(temp_root.join("app.did").exists());

        let _ = fs::remove_dir_all(temp_root);
    }
}
