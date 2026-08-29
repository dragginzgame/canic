use crate::{
    artifact_io::{WasmArtifactFinalization, finalize_wasm_artifact},
    bootstrap_candid::resolve_infrastructure_candid,
    canister_build::{
        CanisterArtifactBuildOutput, CanisterBuildProfile, WorkspaceBuildContext,
        cache::{canister_build_target_root, configure_canister_cargo_command},
        extract_candid_bytes,
    },
    cargo_command,
    cargo_metadata::{CargoMetadata, CargoMetadataPackage, cargo_metadata},
    durable_io::write_bytes,
    role_contract::{
        PackageValidationMode, RolePackageValidation, finding_detail,
        resolve_built_in_wasm_store_contract, validate_built_in_wasm_store_package,
    },
    should_embed_candid_metadata,
};
use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const WASM_STORE_ROLE: &str = "wasm_store";
const GENERATED_WRAPPER_RELATIVE: &str = ".icp/local/generated/canic-wasm-store";
const CANONICAL_WASM_STORE_DID_FILE: &str = "wasm_store.did";
const CANONICAL_WASM_STORE_CRATE_NAME: &str = "canister_wasm_store";
const GENERATED_WRAPPER_PACKAGE_NAME: &str = "canic-generated-wasm-store";
const CANIC_FAMILY_CRATES: &[&str] = &["canic-control-plane", "canic-core", "canic-macros"];
const WASM_STORE_RELEASE_PROFILE: &[(&str, &str)] = &[
    ("opt-level", "\"z\""),
    ("lto", "true"),
    ("codegen-units", "1"),
    ("strip", "\"symbols\""),
    ("debug", "false"),
    ("panic", "\"abort\""),
    ("overflow-checks", "false"),
    ("incremental", "false"),
];
const WASM_STORE_FAST_PROFILE: &[(&str, &str)] = &[
    ("inherits", "\"release\""),
    ("lto", "false"),
    ("codegen-units", "16"),
    ("incremental", "false"),
];

#[derive(Clone, Debug)]
struct BootstrapWasmStoreSource {
    manifest_path: PathBuf,
    package_name: String,
    package_version: String,
    canonical_did_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedWrapperDependencies {
    pub(crate) canic_version: String,
    pub(crate) candid_version: String,
    pub(crate) crypto_common_version: String,
    pub(crate) ic_cdk_version: String,
    pub(crate) serde_version: String,
}

// Build the implicit bootstrap `wasm_store` artifact and populate the canonical
// local ICP artifact paths for downstream/root builds.
pub fn build_bootstrap_wasm_store_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let source = resolve_bootstrap_wasm_store_source(&context.workspace_root, &context.icp_root)?;
    require_built_in_wasm_store_contract(&source.manifest_path)?;
    let artifact_root = context.artifact_root().join(WASM_STORE_ROLE);
    fs::create_dir_all(&artifact_root)?;

    run_wasm_store_cargo_build(context, &source.manifest_path, None, true)?;

    let target_root = canister_build_target_root(&context.workspace_root);
    let built_wasm_path = target_root
        .join("wasm32-unknown-unknown")
        .join(context.profile.target_dir_name())
        .join(format!("{CANONICAL_WASM_STORE_CRATE_NAME}.wasm"));
    let candid = extract_candid_bytes(&built_wasm_path)?;
    let capabilities = canic_core::role_contract::built_in_role_capabilities(
        canic_core::role_contract::BuiltInRoleKind::WasmStore,
    );
    let profile = canic_core::role_contract::derive_protocol_profile_hashes(
        &source.package_version,
        &canic_core::ids::CanisterRole::new(WASM_STORE_ROLE),
        &capabilities,
        &candid,
    );
    run_wasm_store_cargo_build(
        context,
        &source.manifest_path,
        Some(profile.protocol_profile_digest),
        false,
    )?;

    let wasm_path = artifact_root.join(format!("{WASM_STORE_ROLE}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{WASM_STORE_ROLE}.wasm.gz"));
    let did_path = artifact_root.join(format!("{WASM_STORE_ROLE}.did"));
    let profile_path = artifact_root.join(".build-profile");
    let embed_candid = should_embed_candid_metadata(context.build_network);
    let artifact_candid = if embed_candid {
        resolve_wasm_store_candid(context, &source)?
    } else {
        candid.clone()
    };
    if artifact_candid != candid {
        return Err("Wasm Store materialized Candid differs from its compiled profile".into());
    }
    let transforms = finalize_wasm_artifact(&WasmArtifactFinalization {
        profile: context.profile,
        build_network: context.build_network,
        embed_candid,
        validate_sidecar_only: false,
        source_wasm_path: &built_wasm_path,
        candid: &artifact_candid,
        wasm_path: &wasm_path,
        did_path: &did_path,
        wasm_gz_path: &wasm_gz_path,
    })?;
    write_bytes(&profile_path, context.profile.target_dir_name().as_bytes())?;

    Ok(CanisterArtifactBuildOutput {
        package_name: source.package_name,
        package_version: source.package_version.clone(),
        protocol_release_identity: source.package_version,
        protocol_role: canic_core::ids::CanisterRole::new(WASM_STORE_ROLE),
        protocol_capabilities: capabilities,
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        candid_sha256: profile.candid_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
        transforms,
    })
}

fn require_built_in_wasm_store_contract(
    manifest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let evidence =
        match validate_built_in_wasm_store_package(manifest_path, PackageValidationMode::Build) {
            RolePackageValidation::Supported(evidence) => evidence,
            RolePackageValidation::Unsupported(finding) => {
                return Err(format!("{}: {}", finding.code(), finding_detail(&finding)).into());
            }
        };
    match resolve_built_in_wasm_store_contract(&evidence) {
        canic_core::role_contract::RoleContractResolution::Resolved { .. } => Ok(()),
        canic_core::role_contract::RoleContractResolution::Rejected { errors } => Err(errors
            .iter()
            .map(|finding| format!("{}: {}", finding.code(), finding_detail(finding)))
            .collect::<Vec<_>>()
            .join("; ")
            .into()),
    }
}

// Resolve the canonical published/workspace `canic-wasm-store` source or fall
// back to a generated wrapper when downstreams only depend on `canic`.
fn resolve_bootstrap_wasm_store_source(
    workspace_root: &Path,
    icp_root: &Path,
) -> Result<BootstrapWasmStoreSource, Box<dyn std::error::Error>> {
    let metadata = cargo_metadata(workspace_root, true)?;
    let canic_package = resolved_canic_package(&metadata)?;

    if let Some(source) = resolve_canonical_bootstrap_wasm_store_source(&metadata, canic_package)? {
        return Ok(source);
    }

    let dependencies = resolved_wrapper_dependencies(&metadata, canic_package)?;
    let wrapper_root = ensure_generated_wasm_store_wrapper(
        icp_root,
        workspace_root,
        &canic_package.manifest_path,
        &dependencies,
    )?;
    Ok(BootstrapWasmStoreSource {
        manifest_path: wrapper_root.join("Cargo.toml"),
        package_name: GENERATED_WRAPPER_PACKAGE_NAME.to_string(),
        package_version: canic_package.version.clone(),
        canonical_did_path: None,
    })
}

pub fn resolved_canic_package(
    metadata: &CargoMetadata,
) -> Result<&CargoMetadataPackage, Box<dyn std::error::Error>> {
    let matches = metadata
        .packages
        .iter()
        .filter(|package| package.name == "canic")
        .collect::<Vec<_>>();
    let [package] = matches.as_slice() else {
        return Err(format!(
            "built-in artifact source requires exactly one resolved 'canic' package; found {}",
            matches.len()
        )
        .into());
    };
    Ok(package)
}

// Prefer the exact resolved `canic-wasm-store` package, then the exact sibling
// source belonging to the selected Canic package.
fn resolve_canonical_bootstrap_wasm_store_source(
    metadata: &CargoMetadata,
    canic_package: &CargoMetadataPackage,
) -> Result<Option<BootstrapWasmStoreSource>, Box<dyn std::error::Error>> {
    let matches = metadata
        .packages
        .iter()
        .filter(|package| {
            package.name == "canic-wasm-store"
                && package.version == canic_package.version
                && package.source == canic_package.source
        })
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(
            "bootstrap wasm_store source resolved more than once for the selected Canic package"
                .into(),
        );
    }
    if let [package] = matches.as_slice() {
        let source_root = package
            .manifest_path
            .parent()
            .expect("manifest path must have parent")
            .to_path_buf();
        return Ok(Some(BootstrapWasmStoreSource {
            manifest_path: package.manifest_path.clone(),
            package_name: package.name.clone(),
            package_version: package.version.clone(),
            canonical_did_path: Some(source_root.join(CANONICAL_WASM_STORE_DID_FILE)),
        }));
    }

    let canic_root = canic_package
        .manifest_path
        .parent()
        .expect("canic manifest path must have parent");
    let sibling_root = canic_root.parent().expect("canic root must have parent");
    let registry_version = registry_package_version_suffix(&canic_package.manifest_path, "canic")
        .filter(|version| *version == canic_package.version);
    let sibling_dir = registry_version.map_or_else(
        || "canic-wasm-store".to_string(),
        |version| format!("canic-wasm-store-{version}"),
    );
    let sibling_manifest = sibling_root.join(sibling_dir).join("Cargo.toml");
    if sibling_manifest.is_file() {
        require_package_manifest_identity(
            &sibling_manifest,
            "canic-wasm-store",
            &canic_package.version,
        )?;
        let source_root = sibling_manifest
            .parent()
            .expect("manifest path must have parent")
            .to_path_buf();
        return Ok(Some(BootstrapWasmStoreSource {
            manifest_path: sibling_manifest,
            package_name: "canic-wasm-store".to_string(),
            package_version: canic_package.version.clone(),
            canonical_did_path: Some(source_root.join(CANONICAL_WASM_STORE_DID_FILE)),
        }));
    }

    Ok(None)
}

pub fn resolved_wrapper_dependencies(
    metadata: &CargoMetadata,
    canic_package: &CargoMetadataPackage,
) -> Result<GeneratedWrapperDependencies, Box<dyn std::error::Error>> {
    let canic_core = resolved_normal_dependency(metadata, canic_package, "canic-core")?;
    let candid = resolved_normal_dependency(metadata, canic_core, "candid")?;
    let ic_cdk = resolved_normal_dependency(metadata, canic_core, "ic-cdk")?;
    let serde = resolved_normal_dependency(metadata, canic_core, "serde")?;
    let ic_principal = resolved_normal_dependency(metadata, candid, "ic_principal")?;
    let sha2 = resolved_normal_dependency(metadata, ic_principal, "sha2")?;
    let digest = resolved_normal_dependency(metadata, sha2, "digest")?;
    let crypto_common = resolved_normal_dependency(metadata, digest, "crypto-common")?;
    Ok(GeneratedWrapperDependencies {
        canic_version: canic_package.version.clone(),
        candid_version: candid.version.clone(),
        crypto_common_version: crypto_common.version.clone(),
        ic_cdk_version: ic_cdk.version.clone(),
        serde_version: serde.version.clone(),
    })
}

fn resolved_normal_dependency<'a>(
    metadata: &'a CargoMetadata,
    parent: &CargoMetadataPackage,
    dependency_name: &str,
) -> Result<&'a CargoMetadataPackage, Box<dyn std::error::Error>> {
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or("bootstrap wasm_store cargo metadata omitted the resolved dependency graph")?;
    let node = resolve
        .nodes
        .iter()
        .find(|node| node.id == parent.id)
        .ok_or_else(|| {
            format!(
                "bootstrap wasm_store cargo metadata omitted the graph node for {}",
                parent.name
            )
        })?;
    let matches = node
        .deps
        .iter()
        .filter(|dependency| dependency.dep_kinds.iter().any(|kind| kind.kind.is_none()))
        .filter_map(|dependency| {
            metadata
                .packages
                .iter()
                .find(|package| package.id == dependency.pkg)
        })
        .filter(|package| package.name == dependency_name)
        .collect::<Vec<_>>();
    let [package] = matches.as_slice() else {
        return Err(format!(
            "bootstrap wasm_store requires exactly one resolved normal {dependency_name} dependency from {}; found {}",
            parent.name,
            matches.len()
        )
        .into());
    };
    Ok(package)
}

// Render the generated wrapper under `.icp/local/generated/canic-wasm-store`.
fn ensure_generated_wasm_store_wrapper(
    icp_root: &Path,
    workspace_root: &Path,
    canic_manifest_path: &Path,
    dependencies: &GeneratedWrapperDependencies,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let wrapper_root = icp_root.join(GENERATED_WRAPPER_RELATIVE);
    fs::create_dir_all(wrapper_root.join("src"))?;

    let canic_root = canic_manifest_path
        .parent()
        .expect("canic manifest path must have parent");
    let patch_table =
        generated_wasm_store_wrapper_patch_table(canic_manifest_path, &dependencies.canic_version)?;
    let mut cargo_toml = format!(
        "[package]\n\
name = \"{GENERATED_WRAPPER_PACKAGE_NAME}\"\n\
version = \"0.0.0\"\n\
edition = \"2024\"\n\
publish = false\n\n\
[package.metadata.canic]\n\
app = \"wasm_store\"\n\
role = \"wasm_store\"\n\n\
[workspace]\n\
resolver = \"2\"\n\n\
[lib]\n\
name = \"{CANONICAL_WASM_STORE_CRATE_NAME}\"\n\
crate-type = [\"cdylib\", \"rlib\"]\n\n\
[dependencies]\n\
canic = {{ path = \"{}\", default-features = false, features = [\"wasm-store-canister\"] }}\n\
ic-cdk = \"={}\"\n\
candid = {{ version = \"={}\", default-features = false }}\n\n\
[build-dependencies]\n\
canic = {{ path = \"{}\", default-features = false, features = [] }}\n",
        canic_root.display(),
        dependencies.ic_cdk_version,
        dependencies.candid_version,
        canic_root.display()
    );

    render_profile(&mut cargo_toml, "release", WASM_STORE_RELEASE_PROFILE);
    render_profile(&mut cargo_toml, "fast", WASM_STORE_FAST_PROFILE);

    if !patch_table.is_empty() {
        cargo_toml.push('\n');
        cargo_toml.push_str(&patch_table);
    }

    fs::write(wrapper_root.join("Cargo.toml"), cargo_toml)?;
    let config_path_env = canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV;
    fs::write(
        wrapper_root.join("build.rs"),
        format!(
            "fn main() {{\n    let config_path = std::env::var({config_path_env:?})\n        .expect({error:?});\n\n    canic::build!(config_path);\n}}\n",
            error = format!("{config_path_env} must be set for generated wasm_store wrapper"),
        ),
    )?;
    fs::write(
        wrapper_root.join("src/lib.rs"),
        "#![expect(clippy::unused_async)]\n\ncanic::start_wasm_store!();\ncanic::finish!();\n",
    )?;

    let workspace_lock = workspace_root.join("Cargo.lock");
    if workspace_lock.is_file() {
        fs::copy(workspace_lock, wrapper_root.join("Cargo.lock"))?;
    }

    Ok(wrapper_root)
}

// Generate the `[patch.crates-io]` table for sibling packaged Canic crates.
pub fn generated_wasm_store_wrapper_patch_table(
    canic_manifest_path: &Path,
    canic_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let canic_root = canic_manifest_path
        .parent()
        .expect("canic manifest path must have parent");
    let sibling_root = canic_root.parent().expect("canic root must have parent");
    let registry_version = registry_package_version_suffix(canic_manifest_path, "canic")
        .filter(|version| *version == canic_version);
    let mut rendered = String::new();

    for crate_name in CANIC_FAMILY_CRATES {
        let sibling_dir = registry_version.map_or_else(
            || (*crate_name).to_string(),
            |version| format!("{crate_name}-{version}"),
        );
        let manifest_path = sibling_root.join(sibling_dir).join("Cargo.toml");

        if !manifest_path.is_file() {
            continue;
        }
        require_package_manifest_identity(&manifest_path, crate_name, canic_version)?;

        let crate_root = manifest_path
            .parent()
            .expect("manifest path must have parent");
        let _ = writeln!(
            rendered,
            "{crate_name} = {{ path = \"{}\" }}",
            crate_root.display()
        );
    }

    if rendered.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("[patch.crates-io]\n{rendered}"))
    }
}

pub fn require_package_manifest_identity(
    manifest_path: &Path,
    expected_name: &str,
    expected_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(manifest_path)?;
    let manifest = toml::from_str::<toml::Value>(&source)?;
    let package = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("package table missing from {}", manifest_path.display()))?;
    let name = package
        .get("name")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    let version = package.get("version");
    let version_matches = version
        .and_then(toml::Value::as_str)
        .is_some_and(|version| version == expected_version)
        || version
            .and_then(toml::Value::as_table)
            .and_then(|version| version.get("workspace"))
            .and_then(toml::Value::as_bool)
            == Some(true);
    if name != expected_name || !version_matches {
        let observed_version = version
            .and_then(toml::Value::as_str)
            .unwrap_or("<not an exact or workspace version>");
        return Err(format!(
            "built-in artifact sibling {} must be package {expected_name} {expected_version}; found {name} {observed_version}",
            manifest_path.display()
        )
        .into());
    }
    Ok(())
}

pub fn registry_package_version_suffix<'a>(
    manifest_path: &'a Path,
    crate_name: &str,
) -> Option<&'a str> {
    let parent_name = manifest_path.parent()?.file_name()?.to_str()?;
    parent_name.strip_prefix(&format!("{crate_name}-"))
}

pub fn render_profile(output: &mut String, profile: &str, settings: &[(&str, &str)]) {
    let _ = writeln!(output, "\n[profile.{profile}]");
    for (key, value) in settings {
        let _ = writeln!(output, "{key} = {value}");
    }
}

// Build the chosen `canic-wasm-store` source/wrapper for one target profile.
fn run_wasm_store_cargo_build(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    protocol_profile_digest: Option<canic_core::role_contract::ProtocolProfileDigest>,
    force_candid_export: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = wasm_store_cargo_build_command(context, manifest_path, force_candid_export);
    if let Some(digest) = protocol_profile_digest {
        command.env(
            canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV,
            digest.to_string(),
        );
    }

    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "cargo build failed for bootstrap wasm_store: {}",
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn wasm_store_cargo_build_command(
    context: &WorkspaceBuildContext,
    manifest_path: &Path,
    force_candid_export: bool,
) -> Command {
    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    command
        .env_remove(canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV)
        .current_dir(&context.workspace_root)
        .env(
            canic_core::role_contract::CANONICAL_BUILD_MARKER_ENV,
            canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE,
        )
        .args([
            if force_candid_export {
                "rustc"
            } else {
                "build"
            },
            "--locked",
            "--manifest-path",
            &manifest_path.display().to_string(),
            "--target",
            "wasm32-unknown-unknown",
        ]);
    configure_canister_cargo_command(&mut command, &context.workspace_root);
    append_wasm_store_profile_config_args(&mut command, context.profile);
    command.args(context.profile.cargo_args());
    if force_candid_export {
        command.args([
            "--lib",
            "--",
            "--cfg",
            "canic_export_candid",
            "--check-cfg=cfg(canic_export_candid)",
        ]);
    }
    command
}

fn append_wasm_store_profile_config_args(command: &mut Command, profile: CanisterBuildProfile) {
    match profile {
        CanisterBuildProfile::Debug => {}
        CanisterBuildProfile::Fast => {
            append_profile_config_args(command, "release", WASM_STORE_RELEASE_PROFILE);
            append_profile_config_args(command, "fast", WASM_STORE_FAST_PROFILE);
        }
        CanisterBuildProfile::Release => {
            append_profile_config_args(command, "release", WASM_STORE_RELEASE_PROFILE);
        }
    }
}

pub fn append_profile_config_args(command: &mut Command, profile: &str, settings: &[(&str, &str)]) {
    for (key, value) in settings {
        command
            .arg("--config")
            .arg(format!("profile.{profile}.{key}={value}"));
    }
}

// Copy or regenerate the `.did` file that matches the built bootstrap artifact.
fn resolve_wasm_store_candid(
    context: &WorkspaceBuildContext,
    source: &BootstrapWasmStoreSource,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let target_root = canister_build_target_root(&context.workspace_root);
    let selected_wasm_path = target_root
        .join("wasm32-unknown-unknown")
        .join(context.profile.target_dir_name())
        .join(format!("{CANONICAL_WASM_STORE_CRATE_NAME}.wasm"));

    resolve_infrastructure_candid(
        WASM_STORE_ROLE,
        source.canonical_did_path.as_deref(),
        context.refresh_canonical_infrastructure_did,
        &selected_wasm_path,
        || Ok(()),
    )
}

#[cfg(test)]
mod tests;
