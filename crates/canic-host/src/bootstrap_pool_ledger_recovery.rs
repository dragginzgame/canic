//! Module: bootstrap_pool_ledger_recovery
//!
//! Responsibility: build the exact temporary helper used to recover an empty pool asset's Ledger cycles.
//! Does not own: recovery policy, Root inventory state, or live recovery effects.
//! Boundary: emits one release-bound support artifact from a generated minimal canister source.

use crate::{
    artifact_io::{WasmArtifactFinalization, finalize_wasm_artifact},
    bootstrap_store::{
        append_profile_config_args, render_profile, resolved_canic_package,
        resolved_wrapper_dependencies,
    },
    canister_build::{
        CanisterArtifactBuildOutput, CanisterBuildProfile, WorkspaceBuildContext,
        cache::{canister_build_target_root, configure_canister_cargo_command},
        extract_candid_bytes,
    },
    cargo_command,
    cargo_metadata::cargo_metadata,
    should_embed_candid_metadata,
};
use std::fs;

pub const POOL_LEDGER_RECOVERY_ROLE: &str = "pool_ledger_recovery";
const GENERATED_WRAPPER_RELATIVE: &str = ".icp/local/generated/canic-pool-ledger-recovery";
const GENERATED_WRAPPER_PACKAGE_NAME: &str = "canic-generated-pool-ledger-recovery";
const GENERATED_WRAPPER_CRATE_NAME: &str = "canister_pool_ledger_recovery";
const RELEASE_PROFILE: &[(&str, &str)] = &[
    ("opt-level", "\"z\""),
    ("lto", "true"),
    ("codegen-units", "1"),
    ("strip", "\"symbols\""),
    ("debug", "false"),
    ("panic", "\"abort\""),
    ("overflow-checks", "false"),
    ("incremental", "false"),
];
const FAST_PROFILE: &[(&str, &str)] = &[
    ("inherits", "\"release\""),
    ("lto", "false"),
    ("codegen-units", "16"),
    ("incremental", "false"),
];

pub fn build_pool_ledger_recovery_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    prepare_pool_ledger_recovery_build(context)?;
    let metadata = cargo_metadata(&context.workspace_root, true)?;
    let canic_package = resolved_canic_package(&metadata)?;
    let wrapper_root = context.icp_root.join(GENERATED_WRAPPER_RELATIVE);

    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    command.current_dir(&context.workspace_root).args([
        "build",
        "--locked",
        "--manifest-path",
        &wrapper_root.join("Cargo.toml").display().to_string(),
        "--target",
        "wasm32-unknown-unknown",
    ]);
    configure_canister_cargo_command(&mut command, &context.workspace_root);
    append_profile_config(&mut command, context.profile);
    command.args(context.profile.cargo_args());
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "Cargo failed to build the pool Ledger recovery helper: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let built_wasm_path = canister_build_target_root(&context.workspace_root)
        .join("wasm32-unknown-unknown")
        .join(context.profile.target_dir_name())
        .join(format!("{GENERATED_WRAPPER_CRATE_NAME}.wasm"));
    let candid = extract_candid_bytes(&built_wasm_path)?;
    finalize_pool_ledger_recovery_artifact(
        context,
        &canic_package.version,
        &built_wasm_path,
        &candid,
    )
}

/// Materialize and verify the generated helper's exact dependency authority.
///
/// Callers building a complete release set invoke this before compiling any
/// infrastructure artifact, so a dependency mismatch cannot arrive after the
/// Fleet Coordinator build has already consumed time.
pub fn prepare_pool_ledger_recovery_build(
    context: &WorkspaceBuildContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = cargo_metadata(&context.workspace_root, true)?;
    let canic_package = resolved_canic_package(&metadata)?;
    let dependencies = resolved_wrapper_dependencies(&metadata, canic_package)?;
    let wrapper_root = context.icp_root.join(GENERATED_WRAPPER_RELATIVE);
    fs::create_dir_all(wrapper_root.join("src"))?;
    fs::write(
        wrapper_root.join("Cargo.toml"),
        render_helper_manifest(&dependencies),
    )?;
    fs::write(
        wrapper_root.join("src/lib.rs"),
        include_str!("bootstrap_pool_ledger_recovery/canister.rs.txt"),
    )?;
    let workspace_lock = context.workspace_root.join("Cargo.lock");
    if !workspace_lock.is_file() {
        return Err("pool Ledger recovery helper requires the workspace Cargo.lock".into());
    }
    generate_verified_helper_lock(context, &wrapper_root, &workspace_lock, &dependencies)
}

fn render_helper_manifest(
    dependencies: &crate::bootstrap_store::GeneratedWrapperDependencies,
) -> String {
    let mut cargo_toml = format!(
        "[package]\n\
name = \"{GENERATED_WRAPPER_PACKAGE_NAME}\"\n\
version = \"0.0.0\"\n\
edition = \"2024\"\n\
publish = false\n\n\
[workspace]\n\
resolver = \"2\"\n\n\
[lib]\n\
name = \"{GENERATED_WRAPPER_CRATE_NAME}\"\n\
crate-type = [\"cdylib\", \"rlib\"]\n\n\
[dependencies]\n\
ic-cdk = \"={}\"\n\
candid = {{ version = \"={}\", default-features = false }}\n\
crypto-common = \"={}\"\n\
serde = {{ version = \"={}\", default-features = false, features = [\"derive\"] }}\n",
        dependencies.ic_cdk_version,
        dependencies.candid_version,
        dependencies.crypto_common_version,
        dependencies.serde_version,
    );
    render_profile(&mut cargo_toml, "release", RELEASE_PROFILE);
    render_profile(&mut cargo_toml, "fast", FAST_PROFILE);
    cargo_toml
}

fn finalize_pool_ledger_recovery_artifact(
    context: &WorkspaceBuildContext,
    package_version: &str,
    built_wasm_path: &std::path::Path,
    candid: &[u8],
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let protocol_role = canic_core::ids::CanisterRole::new(POOL_LEDGER_RECOVERY_ROLE);
    let protocol_capabilities = std::collections::BTreeSet::new();
    let profile = canic_core::role_contract::derive_protocol_profile_hashes(
        package_version,
        &protocol_role,
        &protocol_capabilities,
        candid,
    );
    let artifact_root = context.artifact_root().join(POOL_LEDGER_RECOVERY_ROLE);
    fs::create_dir_all(&artifact_root)?;
    let wasm_path = artifact_root.join(format!("{POOL_LEDGER_RECOVERY_ROLE}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{POOL_LEDGER_RECOVERY_ROLE}.wasm.gz"));
    let did_path = artifact_root.join(format!("{POOL_LEDGER_RECOVERY_ROLE}.did"));
    let transforms = finalize_wasm_artifact(&WasmArtifactFinalization {
        profile: context.profile,
        build_network: context.build_network,
        embed_candid: should_embed_candid_metadata(context.build_network),
        validate_sidecar_only: false,
        source_wasm_path: built_wasm_path,
        candid,
        wasm_path: &wasm_path,
        did_path: &did_path,
        wasm_gz_path: &wasm_gz_path,
    })?;

    Ok(CanisterArtifactBuildOutput {
        package_name: GENERATED_WRAPPER_PACKAGE_NAME.to_string(),
        package_version: package_version.to_string(),
        protocol_release_identity: package_version.to_string(),
        protocol_role,
        protocol_capabilities,
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        candid_sha256: profile.candid_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
        transforms,
    })
}

fn generate_verified_helper_lock(
    context: &WorkspaceBuildContext,
    wrapper_root: &std::path::Path,
    workspace_lock: &std::path::Path,
    dependencies: &crate::bootstrap_store::GeneratedWrapperDependencies,
) -> Result<(), Box<dyn std::error::Error>> {
    let helper_lock = wrapper_root.join("Cargo.lock");
    write_workspace_seeded_helper_lock(workspace_lock, &helper_lock, dependencies)?;

    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    command.current_dir(&context.workspace_root).args([
        "metadata",
        "--format-version",
        "1",
        "--offline",
        "--manifest-path",
        &wrapper_root.join("Cargo.toml").display().to_string(),
    ]);
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "Cargo could not normalize the exact workspace-seeded pool Ledger recovery helper graph before infrastructure compilation: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    require_helper_lock_within_workspace(workspace_lock, &helper_lock)?;
    require_helper_metadata_within_workspace(workspace_lock, &output.stdout)?;

    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    command.current_dir(&context.workspace_root).args([
        "metadata",
        "--format-version",
        "1",
        "--locked",
        "--offline",
        "--manifest-path",
        &wrapper_root.join("Cargo.toml").display().to_string(),
    ]);
    let stable = command.output()?;
    if !stable.status.success() {
        return Err(format!(
            "the normalized pool Ledger recovery helper lock is not stable under --locked before infrastructure compilation: {}",
            String::from_utf8_lossy(&stable.stderr)
        )
        .into());
    }
    require_helper_metadata_within_workspace(workspace_lock, &stable.stdout)?;
    Ok(())
}

fn write_workspace_seeded_helper_lock(
    workspace_lock: &std::path::Path,
    helper_lock: &std::path::Path,
    dependencies: &crate::bootstrap_store::GeneratedWrapperDependencies,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace: toml::Value = toml::from_str(&fs::read_to_string(workspace_lock)?)?;
    let requested = [
        ("candid", dependencies.candid_version.as_str()),
        ("crypto-common", dependencies.crypto_common_version.as_str()),
        ("ic-cdk", dependencies.ic_cdk_version.as_str()),
        ("serde", dependencies.serde_version.as_str()),
    ];
    let packages = lock_package_identities(&workspace)?;
    let mut direct = Vec::with_capacity(requested.len());
    let mut failures = Vec::new();
    for (name, version) in requested {
        match select_lock_package(&packages, name, Some(version), None) {
            Ok(identity) => direct.push(identity),
            Err(error) => failures.push(format!("{name} {version}: {error}")),
        }
    }
    if !failures.is_empty() {
        failures.sort();
        return Err(format!(
            "pool Ledger recovery helper dependencies are absent or ambiguous in the consuming workspace lock:\n- {}",
            failures.join("\n- ")
        )
        .into());
    }
    let mut generated = toml::map::Map::new();
    generated.insert(
        "name".to_string(),
        toml::Value::String(GENERATED_WRAPPER_PACKAGE_NAME.to_string()),
    );
    generated.insert(
        "version".to_string(),
        toml::Value::String("0.0.0".to_string()),
    );
    generated.insert(
        "dependencies".to_string(),
        toml::Value::Array(
            direct
                .into_iter()
                .map(|identity| toml::Value::String(lock_dependency_reference(&identity)))
                .collect(),
        ),
    );
    let workspace_packages = workspace
        .get_mut("package")
        .and_then(toml::Value::as_array_mut)
        .ok_or("Cargo.lock omits package records")?;
    if workspace_packages.iter().any(|package| {
        package.get("name").and_then(toml::Value::as_str) == Some(GENERATED_WRAPPER_PACKAGE_NAME)
    }) {
        return Err("workspace lock already contains the generated helper package".into());
    }
    workspace_packages.push(toml::Value::Table(generated));
    fs::write(helper_lock, toml::to_string(&workspace)?)?;
    Ok(())
}

fn require_helper_metadata_within_workspace(
    workspace_lock: &std::path::Path,
    helper_metadata: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace: toml::Value = toml::from_str(&fs::read_to_string(workspace_lock)?)?;
    let workspace_packages = lock_package_identities(&workspace)?;
    let helper: crate::cargo_metadata::CargoMetadata = serde_json::from_slice(helper_metadata)?;
    let mut mismatches = helper
        .packages
        .into_iter()
        .filter(|package| package.name != GENERATED_WRAPPER_PACKAGE_NAME)
        .filter_map(|package| {
            let matched = workspace_packages.iter().any(|identity| {
                identity.0 == package.name
                    && identity.1 == package.version
                    && identity.2 == package.source
            });
            (!matched).then(|| {
                format!(
                    "{} {} ({})",
                    package.name,
                    package.version,
                    package.source.as_deref().unwrap_or("workspace/path")
                )
            })
        })
        .collect::<Vec<_>>();
    mismatches.sort();
    if !mismatches.is_empty() {
        return Err(format!(
            "pool Ledger recovery helper resolved package identities outside the consuming workspace lock:\n- {}",
            mismatches.join("\n- ")
        )
        .into());
    }
    Ok(())
}

fn require_helper_lock_within_workspace(
    workspace_lock: &std::path::Path,
    helper_lock: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace: toml::Value = toml::from_str(&fs::read_to_string(workspace_lock)?)?;
    let helper: toml::Value = toml::from_str(&fs::read_to_string(helper_lock)?)?;
    let workspace_packages = lock_package_identities(&workspace)?;
    let mut mismatches = lock_package_identities(&helper)?
        .into_iter()
        .filter(|identity| identity.0 != GENERATED_WRAPPER_PACKAGE_NAME)
        .filter(|identity| !workspace_packages.contains(identity))
        .map(|identity| {
            format!(
                "{} {} ({})",
                identity.0,
                identity.1,
                identity.2.as_deref().unwrap_or("workspace/path")
            )
        })
        .collect::<Vec<_>>();
    mismatches.sort();
    if !mismatches.is_empty() {
        return Err(format!(
            "pool Ledger recovery helper lock selected identities outside the consuming workspace lock:\n- {}",
            mismatches.join("\n- ")
        )
        .into());
    }
    Ok(())
}

type LockPackageIdentity = (String, String, Option<String>, Option<String>);

fn select_lock_package(
    packages: &std::collections::BTreeSet<LockPackageIdentity>,
    name: &str,
    version: Option<&str>,
    source: Option<&str>,
) -> Result<LockPackageIdentity, String> {
    let candidates = packages
        .iter()
        .filter(|identity| {
            identity.0 == name
                && version.is_none_or(|version| identity.1 == version)
                && source.is_none_or(|source| identity.2.as_deref() == Some(source))
        })
        .cloned()
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [identity] => Ok(identity.clone()),
        [] => Err("not present".to_string()),
        _ => Err(format!(
            "ambiguous across {} locked identities",
            candidates.len()
        )),
    }
}

fn lock_dependency_reference(identity: &LockPackageIdentity) -> String {
    identity.2.as_ref().map_or_else(
        || format!("{} {}", identity.0, identity.1),
        |source| format!("{} {} ({source})", identity.0, identity.1),
    )
}

fn lock_package_identities(
    lock: &toml::Value,
) -> Result<std::collections::BTreeSet<LockPackageIdentity>, Box<dyn std::error::Error>> {
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or("Cargo.lock omits package records")?;
    packages.iter().map(lock_package_identity).collect()
}

fn lock_package_identity(
    package: &toml::Value,
) -> Result<LockPackageIdentity, Box<dyn std::error::Error>> {
    let table = package
        .as_table()
        .ok_or("Cargo.lock package is not a table")?;
    let field = |name: &str| {
        table
            .get(name)
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    };
    Ok((
        field("name").ok_or("Cargo.lock package omits name")?,
        field("version").ok_or("Cargo.lock package omits version")?,
        field("source"),
        field("checksum"),
    ))
}

fn append_profile_config(command: &mut std::process::Command, profile: CanisterBuildProfile) {
    match profile {
        CanisterBuildProfile::Debug => {}
        CanisterBuildProfile::Fast => {
            append_profile_config_args(command, "release", RELEASE_PROFILE);
            append_profile_config_args(command, "fast", FAST_PROFILE);
        }
        CanisterBuildProfile::Release => {
            append_profile_config_args(command, "release", RELEASE_PROFILE);
        }
    }
}

#[cfg(test)]
mod tests;
