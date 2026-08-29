//! Module: bootstrap_pool_ledger_recovery
//!
//! Responsibility: build the exact temporary helper used to recover an empty pool asset's Ledger cycles.
//! Does not own: recovery policy, Root inventory state, or live recovery effects.
//! Boundary: emits one release-bound support artifact from a generated minimal canister source.

use crate::{
    artifact_io::finalize_wasm_artifact,
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
    durable_io::write_bytes,
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
    let metadata = cargo_metadata(&context.workspace_root, true)?;
    let canic_package = resolved_canic_package(&metadata)?;
    let dependencies = resolved_wrapper_dependencies(&metadata, canic_package)?;
    let wrapper_root = context.icp_root.join(GENERATED_WRAPPER_RELATIVE);
    fs::create_dir_all(wrapper_root.join("src"))?;
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
    fs::write(wrapper_root.join("Cargo.toml"), cargo_toml)?;
    fs::write(
        wrapper_root.join("src/lib.rs"),
        include_str!("bootstrap_pool_ledger_recovery/canister.rs.txt"),
    )?;
    let workspace_lock = context.workspace_root.join("Cargo.lock");
    if !workspace_lock.is_file() {
        return Err("pool Ledger recovery helper requires the workspace Cargo.lock".into());
    }
    generate_verified_helper_lock(context, &wrapper_root, &workspace_lock)?;

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
    let protocol_role = canic_core::ids::CanisterRole::new(POOL_LEDGER_RECOVERY_ROLE);
    let protocol_capabilities = std::collections::BTreeSet::new();
    let profile = canic_core::role_contract::derive_protocol_profile_hashes(
        &canic_package.version,
        &protocol_role,
        &protocol_capabilities,
        &candid,
    );
    let artifact_root = context.artifact_root().join(POOL_LEDGER_RECOVERY_ROLE);
    fs::create_dir_all(&artifact_root)?;
    let wasm_path = artifact_root.join(format!("{POOL_LEDGER_RECOVERY_ROLE}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{POOL_LEDGER_RECOVERY_ROLE}.wasm.gz"));
    let did_path = artifact_root.join(format!("{POOL_LEDGER_RECOVERY_ROLE}.did"));
    fs::copy(&built_wasm_path, &wasm_path)?;
    write_bytes(&did_path, &candid)?;
    let transforms = finalize_wasm_artifact(
        context.profile,
        should_embed_candid_metadata(context.build_network),
        &wasm_path,
        &did_path,
        &wasm_gz_path,
    )?;

    Ok(CanisterArtifactBuildOutput {
        package_name: GENERATED_WRAPPER_PACKAGE_NAME.to_string(),
        package_version: canic_package.version.clone(),
        protocol_release_identity: canic_package.version.clone(),
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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    command.current_dir(&context.workspace_root).args([
        "generate-lockfile",
        "--offline",
        "--manifest-path",
        &wrapper_root.join("Cargo.toml").display().to_string(),
    ]);
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "Cargo failed to lock the pool Ledger recovery helper: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    require_helper_lock_within_workspace(workspace_lock, &wrapper_root.join("Cargo.lock"))
}

fn require_helper_lock_within_workspace(
    workspace_lock: &std::path::Path,
    helper_lock: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace: toml::Value = toml::from_str(&fs::read_to_string(workspace_lock)?)?;
    let helper: toml::Value = toml::from_str(&fs::read_to_string(helper_lock)?)?;
    let workspace_packages = lock_package_identities(&workspace)?;
    for identity in lock_package_identities(&helper)? {
        if identity.0 == GENERATED_WRAPPER_PACKAGE_NAME {
            continue;
        }
        if !workspace_packages.contains(&identity) {
            return Err(format!(
                "pool Ledger recovery helper resolved outside the workspace lock: {} {}",
                identity.0, identity.1
            )
            .into());
        }
    }
    Ok(())
}

type LockPackageIdentity = (String, String, Option<String>, Option<String>);

fn lock_package_identities(
    lock: &toml::Value,
) -> Result<std::collections::BTreeSet<LockPackageIdentity>, Box<dyn std::error::Error>> {
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or("Cargo.lock omits package records")?;
    packages
        .iter()
        .map(|package| {
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
        })
        .collect()
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
