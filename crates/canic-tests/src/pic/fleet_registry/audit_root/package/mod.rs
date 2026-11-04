//! Build the audit entrypoint under the exact selected App's generated Root contract.

use canic_host::{canister_build::WorkspaceBuildContext, role_contract::RoleCargoGraphEvidence};
use ic_build::cargo::{observation::CargoMetadataSpec, selection::CargoFeatureSelection};
use ic_host_fs::durable::{lock_exclusive, open_regular_lock_file_with_parents};
use ic_host_process::tool::{OutputLimit, OutputLimits, capture_command};
use std::{fs, path::PathBuf, process::Command, time::Duration};

pub(super) fn prepare(
    context: &WorkspaceBuildContext,
    evidence: &RoleCargoGraphEvidence,
) -> PathBuf {
    let mut manifest: toml::Value = toml::from_str(
        &fs::read_to_string(&evidence.role_manifest_path).expect("canonical Root manifest"),
    )
    .unwrap();
    manifest["package"]["name"] = "root_probe".into();
    manifest["lib"]["name"] = "root_probe".into();
    let manifest = toml::to_string(&manifest).unwrap();
    let source = context.workspace_root.join("canisters/audit/root_probe");
    let library = format!(
        "use canic::__internal::ic_wire;\n{}",
        fs::read_to_string(source.join("src/lib.rs")).unwrap()
    );
    let protocol = format!(
        "{}\nuse canic::__internal::ic_wire;\n",
        fs::read_to_string(source.join("src/protocol/mod.rs")).unwrap()
    );
    let build = fs::read_to_string(source.join("build.rs")).unwrap();
    let lock = fs::read(evidence.cargo_workspace_root.join("Cargo.lock")).unwrap();
    let identity = canic_core::cdk::utils::hash::sha256_hex(
        &serde_json::to_vec(&(&manifest, &library, &protocol, &build, &lock)).unwrap(),
    );
    let root = context
        .workspace_root
        .join(".canic/test-packages/audit-root")
        .join(identity);
    let lock_file = open_regular_lock_file_with_parents(&root.with_extension("lock")).unwrap();
    lock_exclusive(&lock_file).unwrap();
    // This fixture generation is immutable once its selected lock is resolved.
    if !root.join("prepared").is_file() {
        fs::create_dir_all(root.join("src/protocol")).unwrap();
        fs::create_dir_all(root.join("examples/wire_schema")).unwrap();
        fs::write(root.join("Cargo.toml"), manifest).unwrap();
        fs::write(root.join("Cargo.lock"), lock).unwrap();
        fs::write(root.join("build.rs"), build).unwrap();
        fs::write(root.join("src/lib.rs"), library).unwrap();
        fs::write(root.join("src/protocol/mod.rs"), protocol).unwrap();
        fs::write(root.join("examples/wire_schema/main.rs"), "use root_probe as _;\nfn main() -> Result<(), Box<dyn std::error::Error>> { canic::endpoint::schema::producer::run() }\n").unwrap();
        // Only the generated package identity changes; seed from the already
        // resolved canonical graph and never contact the registry here.
        let metadata = CargoMetadataSpec {
            manifest_path: &root.join("Cargo.toml"),
            include_dependencies: true,
            filter_platform: None,
            locked: false,
            offline: true,
            features: &CargoFeatureSelection::default(),
        };
        let cargo = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
        let mut command = metadata.command(cargo).unwrap();
        capture_command(
            &mut command,
            OutputLimits {
                stdout: OutputLimit::Terminate(16 * 1024 * 1024),
                stderr: OutputLimit::Terminate(1024 * 1024),
                timeout: Some(Duration::from_secs(120)),
            },
        )
        .expect("resolve generated audit package lock")
        .require_success()
        .expect("resolve generated audit package lock")
        .require_complete()
        .expect("complete generated audit metadata");
        fs::write(root.join("prepared"), b"").unwrap();
    }
    root
}
