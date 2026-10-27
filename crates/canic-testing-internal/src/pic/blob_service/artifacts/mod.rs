//! Module: pic::blob_service::artifacts
//!
//! Responsibility: acquire complete Blob consumer builds through the production CLI.
//! Boundary: isolate build-only environment from the caller-owned PocketIC server.

use canic_contracts::ids::{BuildNetwork, ReleaseBuildId};

use std::path::PathBuf;

const BUILD_WORKER: &str = "CANIC_BLOB_BUILD_WORKER";
const BUILD_RESULT: &str = "release-build-id";

/// The two framework-owned consumer compositions under qualification.
#[derive(Clone, Copy)]
pub(in crate::pic) enum Composition {
    Dedicated,
    Embedded,
}

impl Composition {
    pub(in crate::pic) fn paths(self) -> (PathBuf, PathBuf) {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let relative = match self {
            Self::Dedicated => "integrations/blob-service/consumer",
            Self::Embedded => "integrations/blob-service/embedded-consumer",
        };
        let consumer = workspace.join(relative);
        let config = consumer.join("canic.toml");
        (consumer, config)
    }

    const fn app(self) -> &'static str {
        match self {
            Self::Dedicated => "consumer-app",
            Self::Embedded => "embedded-app",
        }
    }

    pub(in crate::pic) const fn role(self) -> &'static str {
        match self {
            Self::Dedicated => "blob",
            Self::Embedded => "backend",
        }
    }
}

/// Expected contract release from the independently locked consumer under test.
pub(super) fn blob_contract_release(composition: Composition) -> String {
    let (consumer, _) = composition.paths();
    let lock: toml::Value =
        toml::from_str(&std::fs::read_to_string(consumer.join("Cargo.lock")).unwrap()).unwrap();
    let mut contracts = lock["package"]
        .as_array()
        .expect("consumer lock packages")
        .iter()
        .filter(|package| package["name"].as_str() == Some("ic-blob-storage-contracts"));
    let contract = contracts.next().expect("selected consumer Blob contract");
    assert!(
        contracts.next().is_none(),
        "one consumer Blob contract identity"
    );
    contract["version"]
        .as_str()
        .expect("locked Blob contract version")
        .to_owned()
}

/// Execute only artifact construction when reentered by the isolated worker.
pub(in crate::pic) fn write_worker_result(composition: Composition) -> bool {
    if std::env::var_os(BUILD_WORKER).is_none() {
        return false;
    }
    std::fs::write(BUILD_RESULT, build_release(composition).to_string()).unwrap();
    true
}

/// Resolve current inputs and obtain one complete managed build, including cache admission.
pub(in crate::pic) fn release_build(rust_path: &str) -> ReleaseBuildId {
    let scratch = std::env::temp_dir().join(format!("blob-build-{}", std::process::id()));
    std::fs::create_dir(&scratch).expect("fresh build-worker result directory");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", rust_path, "--nocapture"])
        .current_dir(&scratch)
        .env(BUILD_WORKER, "1")
        .env("PATH", build_path())
        .env("RUSTC_WRAPPER", "")
        .env("RUSTC_WORKSPACE_WRAPPER", "")
        .env_remove("CANIC_TEST_SCRATCH")
        .env_remove("IC_TESTKIT_POCKET_IC_URL")
        .env_remove("TMPDIR")
        .status()
        .expect("run the Blob build worker with native inputs");
    assert!(status.success(), "isolated Blob build failed");
    let result = scratch.join(BUILD_RESULT);
    let build = std::fs::read_to_string(&result)
        .expect("the selected worker executed and returned a build identity")
        .parse()
        .expect("canonical managed build identity");
    std::fs::remove_file(result).unwrap();
    std::fs::remove_dir(scratch).unwrap();
    build
}

fn build_path() -> std::ffi::OsString {
    let path = std::env::var_os("PATH").expect("build tools PATH");
    let scratch = std::env::var_os("CANIC_TEST_SCRATCH").map(PathBuf::from);
    let directories = std::env::split_paths(&path).map(|directory| {
        if scratch
            .as_ref()
            .is_some_and(|scratch| directory.starts_with(scratch))
        {
            // The governed runner prepends its private symlink to native ICP.
            let icp = std::fs::canonicalize(directory.join("icp")).unwrap();
            icp.parent().unwrap().to_path_buf()
        } else {
            directory
        }
    });
    std::env::join_paths(directories).expect("native build tools PATH")
}

fn build_release(composition: Composition) -> canic_contracts::ids::ReleaseBuildId {
    let (consumer, config) = composition.paths();
    let consumer_text = consumer.to_str().unwrap();
    canic_cli::run(
        [
            "--environment",
            "local",
            "build",
            composition.app(),
            "--workspace",
            consumer_text,
            "--icp-root",
            consumer_text,
            "--config",
            config.to_str().unwrap(),
            "--profile",
            "fast",
        ]
        .into_iter()
        .map(std::ffi::OsString::from),
    )
    .expect("build the complete managed Blob App");
    let context = canic_host::canister_build::WorkspaceBuildContext {
        role: composition.role().into(),
        profile: canic_host::canister_build::CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: BuildNetwork::Local,
        workspace_root: consumer.clone(),
        icp_root: consumer,
        config_path: config,
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    canic_host::canister_build::CanisterArtifactBuilder::for_profile(context.profile)
        .unwrap()
        .prepare_complete_build_reuse(&context, |_| Ok(()))
        .unwrap()
        .load()
        .unwrap()
        .expect("admit the complete managed Blob release")
        .release_build_id
}
