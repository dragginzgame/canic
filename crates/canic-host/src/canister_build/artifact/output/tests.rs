//! Cargo-selected custom targets remain exact across collisions and stale uplifted files.

use super::*;
use crate::artifact_io::CapturedWasmArtifact;
use std::fs;

#[test]
fn stale_unreported_files_and_ambiguous_artifacts_cannot_be_selected() {
    let root = crate::test_support::temp_dir("cargo-artifact-selection");
    fs::create_dir_all(&root).unwrap();
    let manifest = root.join("Cargo.toml");
    fs::write(&manifest, "").unwrap();
    let stale = root.join("guessed_package.wasm");
    fs::write(&stale, b"stale").unwrap();
    assert!(matches!(
        select(
            b"{\"reason\":\"build-finished\",\"success\":true}\n",
            &manifest
        ),
        Err(CargoArtifactError::Selection { count: 0, .. })
    ));
    let artifact = serde_json::json!({
        "reason": "compiler-artifact", "manifest_path": manifest,
        "target": {"name": "custom_library", "crate_types": ["cdylib"]},
        "filenames": [root.join("custom_library.wasm")], "fresh": true,
    });
    assert_eq!(
        select(&serde_json::to_vec(&artifact).unwrap(), &manifest).unwrap(),
        root.join("custom_library.wasm")
    );
    let mut ambiguous = artifact.clone();
    ambiguous["filenames"] = serde_json::json!([root.join("one.wasm"), root.join("two.wasm")]);
    assert!(matches!(
        select(&serde_json::to_vec(&ambiguous).unwrap(), &manifest),
        Err(CargoArtifactError::Selection { count: 2, .. })
    ));
    let mut foreign = artifact;
    let foreign_manifest = root.join("foreign.toml");
    fs::write(&foreign_manifest, "").unwrap();
    foreign["manifest_path"] = serde_json::json!(foreign_manifest);
    assert!(matches!(
        select(&serde_json::to_vec(&foreign).unwrap(), &manifest),
        Err(CargoArtifactError::Selection { count: 0, .. })
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cargo_custom_lib_outputs_are_captured_before_another_workspace_overwrites_them() {
    let root = crate::test_support::temp_dir("cargo-artifact-collision");
    let target = root.join("target");
    fs::create_dir_all(root.join("artifacts")).unwrap();
    let mut retained = Vec::new();
    for (workspace, marker) in [("first", 1), ("second", 2)] {
        let directory = root.join(workspace);
        fs::create_dir_all(directory.join("src")).unwrap();
        let manifest = directory.join("Cargo.toml");
        fs::write(&manifest, "[workspace]\n[package]\nname='collision'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='custom_library'\ncrate-type=['cdylib']\n").unwrap();
        fs::write(
            directory.join("src/lib.rs"),
            format!("#[unsafe(no_mangle)] pub extern \"C\" fn marker() -> u32 {{ {marker} }}\n"),
        )
        .unwrap();
        crate::test_support::generate_fixture_lockfile(&directory);
        let built = crate::cargo_command()
            .current_dir(&directory)
            .args([
                "build",
                "--locked",
                "--offline",
                "--target",
                "wasm32-unknown-unknown",
                "--message-format=json-render-diagnostics",
            ])
            .env("CARGO_TARGET_DIR", &target)
            .env_remove("CARGO_BUILD_BUILD_DIR")
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let path = select(&built.stdout, &manifest).unwrap();
        assert_eq!(path.file_name().unwrap(), "custom_library.wasm");
        let bytes = fs::read(&path).unwrap();
        let captured = CapturedWasmArtifact::capture(
            &path,
            &root.join("artifacts").join(format!("{workspace}.wasm")),
        )
        .unwrap();
        retained.push((path, bytes, captured));
    }
    assert_eq!(retained[0].0, retained[1].0);
    assert_ne!(retained[0].1, retained[1].1);
    for (_, bytes, captured) in &retained {
        assert_eq!(&fs::read(captured.path()).unwrap(), bytes);
    }
    assert_eq!(fs::read(&retained[0].0).unwrap(), retained[1].1);
    drop(retained);
    fs::remove_dir_all(root).unwrap();
}
