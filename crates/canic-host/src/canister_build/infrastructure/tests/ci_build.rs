//! Execute the CI artifact driver against fresh and stale output directories.

use crate::test_support::temp_dir;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = temp_dir("canic-ci-store-build");
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for file in [
            "scripts/ci/build-ci-wasm-artifacts.sh",
            "scripts/ci/require_icp.sh",
            "scripts/ci/ic-tool-pins.sh",
            "scripts/ci/list-config-canisters.sh",
            "tool-versions.env",
            "ci/ic-tools.tsv",
        ] {
            let destination = root.join(file);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(workspace.join(file), destination).unwrap();
        }
        fs::create_dir_all(root.join("bin")).unwrap();
        for tool in ["cargo", "icp", "ic-wasm", "candid-extractor"] {
            ic_host_process::testing::write_executable_script(
                &root.join("bin").join(tool),
                r#"#!/usr/bin/env bash
set -euo pipefail
case "${0##*/}" in
    icp) printf 'icp %s\n' "$EXPECTED_ICP"; exit ;;
    ic-wasm) printf 'ic-wasm %s\n' "$EXPECTED_IC_WASM"; exit ;;
    candid-extractor) exit ;;
esac
while [[ "$1" != -- ]]; do shift; done
shift
[[ $# == 7 && "$2" == fast && "$3" == "$FIXTURE_ROOT" && "$4" == "$FIXTURE_ROOT" ]]
[[ "$5" == "$FIXTURE_ROOT/canic.toml" && "$6" == --release-build-id && "$7" == "$EXPECTED_RELEASE" ]]
printf '%s\n' "$1" >> "$FIXTURE_ROOT/builds"
if [[ "$1" == wasm_store && "${FAIL_STORE:-0}" == 1 ]]; then exit 37; fi
mkdir -p "$FIXTURE_ROOT/artifacts"
artifact="$FIXTURE_ROOT/artifacts/$1.wasm.gz"
printf 'current-%s' "$1" > "$artifact"
printf '%s\n' "$artifact"
"#,
            )
            .unwrap();
        }
        fs::write(
            root.join("canic.toml"),
            "[roles.root]\n[component_specs.app]\ncomponent_role = \"app\"\n",
        )
        .unwrap();
        Self { root }
    }

    fn run(&self, selection: Option<&str>, fail_store: bool) -> Output {
        let path = std::env::join_paths(
            std::iter::once(self.root.join("bin"))
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let mut command = Command::new("bash");
        command
            .arg(self.root.join("scripts/ci/build-ci-wasm-artifacts.sh"))
            .arg("fast")
            .arg(self.root.join("canic.toml"))
            .env("PATH", path)
            .env("FIXTURE_ROOT", &self.root)
            .env("CANIC_RELEASE_BUILD_ID", "07".repeat(32))
            .env("EXPECTED_RELEASE", "07".repeat(32))
            .env(
                "EXPECTED_ICP",
                crate::test_support::ic_tool_pin("icp", "linux-x86_64", 1),
            )
            .env(
                "EXPECTED_IC_WASM",
                crate::test_support::ic_tool_pin("ic-wasm", "linux-x86_64", 1),
            )
            .env("FAIL_STORE", if fail_store { "1" } else { "0" })
            .env_remove("CANIC_REFERENCE_CANISTERS");
        if let Some(selection) = selection {
            command.env("CANIC_REFERENCE_CANISTERS", selection);
        }
        command.output().unwrap()
    }

    fn builds(&self) -> Vec<String> {
        fs::read_to_string(self.root.join("builds"))
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn root_artifact_driver_builds_current_store_with_the_same_inputs() {
    for selection in [
        Some("root"),
        Some("app root"),
        Some("root wasm_store app"),
        None,
    ] {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.root.join("artifacts")).unwrap();
        let store = fixture.root.join("artifacts/wasm_store.wasm.gz");
        if selection != Some("root") {
            fs::write(&store, b"stale predecessor artifact").unwrap();
        }
        let output = fixture.run(selection, false);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(fs::read(store).unwrap(), b"current-wasm_store");
        let builds = fixture.builds();
        assert_eq!(
            builds.iter().filter(|role| *role == "wasm_store").count(),
            1
        );
        assert!(builds.iter().any(|role| role == "root"));
    }
}

#[test]
fn store_failure_stops_root_build_and_component_only_builds_stay_scoped() {
    let fixture = Fixture::new();
    let output = fixture.run(Some("app root"), true);
    assert_eq!(output.status.code(), Some(37));
    assert!(!fixture.builds().iter().any(|role| role == "root"));
    assert!(!fixture.root.join("artifacts/root.wasm.gz").exists());
    let fixture = Fixture::new();
    assert!(fixture.run(Some("app"), false).status.success());
    assert_eq!(fixture.builds(), ["app"]);
    assert!(!fixture.root.join("artifacts/wasm_store.wasm.gz").exists());
}
