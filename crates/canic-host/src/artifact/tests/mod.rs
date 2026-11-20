mod inspection;

use super::*;

#[test]
fn selected_package_ignores_dependency_wasm_but_refuses_missing_or_ambiguous_outputs() {
    let records = br#"{"reason":"compiler-artifact","package_id":"dependency","filenames":["dependency.wasm","dependency.rlib"]}
{"reason":"compiler-artifact","package_id":"selected","filenames":["selected.wasm","selected.rlib"]}
{"reason":"build-finished","success":true}"#;
    assert_eq!(
        selected_wasm(records, "selected").unwrap(),
        PathBuf::from("selected.wasm")
    );
    assert!(selected_wasm(records, "missing").is_err());
    let ambiguous = br#"{"reason":"compiler-artifact","package_id":"selected","filenames":["first.wasm","second.wasm"]}"#;
    assert!(selected_wasm(ambiguous, "selected").is_err());
}

#[test]
fn cargo_diagnostics_select_compiler_errors_and_escape_controls() {
    let mut stdout = serde_json::to_vec(&serde_json::json!({
            "reason": "compiler-message", "message": {
                "level": "error", "rendered": "error[E0425]: missing name\n --> app.rs:4\n\u{1b}[31m\u{202e}"
            }
        })).unwrap();
    let stderr = b"Compiling app\nerror: could not compile app".to_vec();
    let diagnostic = cargo_diagnostic(&stdout, &stderr, false).unwrap();
    assert!(diagnostic.contains("error[E0425]: missing name\n --> app.rs:4"));
    assert!(!diagnostic.contains("could not compile"));
    assert!(!diagnostic.contains(['\u{1b}', '\u{202e}']));
    stdout = b"not a compiler record".to_vec();
    assert!(
        cargo_diagnostic(&stdout, &stderr, false)
            .unwrap()
            .contains("could not compile")
    );
    assert!(cargo_diagnostic(&[], &[], false).is_none());
}

#[test]
fn cargo_diagnostics_bound_unicode_and_mark_incomplete_capture() {
    let mut stderr = "é".repeat(5000).into_bytes();
    let stdout = Vec::new();
    let diagnostic = cargo_diagnostic(&stdout, &stderr, false).unwrap();
    assert!(diagnostic.starts_with(&"é".repeat(4096)));
    assert!(diagnostic.ends_with("[Cargo diagnostic truncated]"));
    assert!(diagnostic.len() < 8300);
    stderr = b"short error".to_vec();
    assert!(
        cargo_diagnostic(&stdout, &stderr, true)
            .unwrap()
            .ends_with("[Cargo diagnostic truncated]")
    );
}

#[test]
fn missing_cargo_package_reports_its_name_and_preserves_build_output() {
    let dir = crate::tests::directory();
    let manifest = dir.join("Cargo.toml");
    std::fs::write(&manifest, "[workspace]\n[package]\nname = 'diagnostic-fixture'\nversion = '0.0.0'\nedition = '2024'\n[lib]\npath = 'lib.rs'\ncrate-type = ['cdylib']\n").unwrap();
    std::fs::write(
        dir.join("Cargo.lock"),
        "version = 4\n[[package]]\nname = 'diagnostic-fixture'\nversion = '0.0.0'\n",
    )
    .unwrap();
    std::fs::write(dir.join("lib.rs"), "pub fn example() {}\n").unwrap();
    let output = dir.join("app.wasm");
    std::fs::write(&output, b"existing output").unwrap();
    let error = build(
        &manifest,
        "missing-canic-fixture-package",
        &[],
        Some(&output),
    )
    .unwrap_err();
    assert!(matches!(error, Error::Supplier(_)));
    assert!(
        error.to_string().contains("missing-canic-fixture-package"),
        "{error}"
    );
    assert_eq!(std::fs::read(output).unwrap(), b"existing output");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn generated_role_enforces_endpoint_schema_and_lifecycle_locations_per_crate() {
    use std::fs::{create_dir_all, write};
    let tree = tempfile::tempdir().unwrap();
    let project = tree.path().join("source");
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    crate::create_starter(&project, "demo", source).unwrap();
    let package = project.join("starter-app");
    let manifest = package.join("Cargo.toml");
    let validate = |metadata: serde_json::Value| {
        let package: super::source_layout::Package = serde_json::from_value(serde_json::json!({
            "id": "starter-app", "name": "starter-app", "manifest_path": manifest, "metadata": metadata
        })).unwrap();
        package.validate()
    };
    let metadata: toml::Table = std::fs::read_to_string(&manifest).unwrap().parse().unwrap();
    let starter_policy = serde_json::to_value(&metadata["package"]["metadata"]).unwrap();
    validate(starter_policy.clone()).unwrap();
    // Exercise a second role which explicitly adopts the conventional directory layout.
    create_dir_all(package.join("src/endpoints")).unwrap();
    std::fs::rename(
        package.join("src/lib.rs"),
        package.join("src/endpoints/mod.rs"),
    )
    .unwrap();
    write(package.join("src/lib.rs"), "mod endpoints;").unwrap();
    let default = serde_json::json!({"canic":{"layout":{}}});
    validate(default.clone()).unwrap();
    for (label, decl, expected) in [
        (
            "endpoints",
            "#[canic::query] fn who() -> u8 { 1 }",
            "src/endpoints",
        ),
        (
            "schema",
            "#[entity(name = \"Row\")] struct Row {}",
            "src/schema",
        ),
        (
            "lifecycle",
            "#[runtime::post_upgrade] fn reopen() {}",
            "src/lifecycle.rs",
        ),
    ] {
        let misplaced = package.join("src/custom.rs");
        write(&misplaced, decl).unwrap();
        let error = validate(default.clone()).unwrap_err().to_string();
        for part in [
            "starter-app",
            expected,
            &format!("package.metadata.canic.layout.{label}"),
        ] {
            assert!(error.contains(part), "{error}");
        }
        // Only this package chooses a different location for this declaration class.
        validate(serde_json::json!({"canic":{"layout":{(label): "src"}}})).unwrap_err();
        if label != "endpoints" {
            validate(serde_json::json!({"canic":{"layout":{(label): "src/custom.rs"}}})).unwrap();
        }
        let proper = if label == "lifecycle" {
            package.join(expected)
        } else {
            package.join(expected).join("example.rs")
        };
        create_dir_all(proper.parent().unwrap()).unwrap();
        std::fs::rename(misplaced, proper).unwrap();
        validate(default.clone()).unwrap();
    }
    // Evolve a generated role while keeping the stateless starter's own policy.
    let independent = project.join("other");
    crate::create_starter(&independent, "other", source).unwrap();
    let other: super::source_layout::Package = serde_json::from_value(serde_json::json!({
        "id": "other", "name": "other", "manifest_path": independent.join("starter-app/Cargo.toml"),
        "metadata": starter_policy
    }))
    .unwrap();
    let mount = r#"canic::mount_db_role! { hub, database = schema::Accounts, config = "app.config", authority = "app", pages = 8, auth_read = "read" }"#;
    write(package.join("src/endpoints/role.rs"), mount).unwrap();
    assert!(
        validate(default.clone())
            .unwrap_err()
            .to_string()
            .contains("src/setup.rs")
    );
    std::fs::rename(
        package.join("src/endpoints/role.rs"),
        package.join("src/setup.rs"),
    )
    .unwrap();
    validate(default).unwrap();
    std::fs::rename(package.join("src/endpoints"), package.join("src/api")).unwrap();
    validate(serde_json::json!({"canic":{"layout":{"endpoints":"src/api"}}})).unwrap();
    other.validate().unwrap();
    write(
        independent.join("starter-app/src/api.rs"),
        "#[canic::update] fn put() {}",
    )
    .unwrap();
    assert!(
        other
            .validate()
            .unwrap_err()
            .to_string()
            .contains("expected src/lib.rs")
    );
}

#[test]
fn source_layout_requires_real_source_directory_and_regular_rust_files() {
    let project = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(external.path(), project.path().join("src")).unwrap();
    let package: super::source_layout::Package = serde_json::from_value(serde_json::json!({
        "id": "demo", "name": "demo", "manifest_path": project.path().join("Cargo.toml"),
        "metadata": {"canic":{"layout":{}}}
    }))
    .unwrap();
    assert!(
        package
            .validate()
            .unwrap_err()
            .to_string()
            .contains("cannot validate symlink src")
    );
    std::fs::remove_file(project.path().join("src")).unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(project.path().join("src/pipe.rs"))
            .status()
            .unwrap()
            .success()
    );
    assert!(
        package
            .validate()
            .unwrap_err()
            .to_string()
            .contains("expected a regular Rust source file at src/pipe.rs")
    );
}

#[test]
fn build_modules_read_custom_sections_from_the_artifact() {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    assert!(compiled_modules(&wasm).is_empty());
    for name in [
        "canic:module:auth",
        "canic:module:db",
        "canic:module:db",
        "other",
    ] {
        let len = u8::try_from(name.len()).unwrap();
        wasm.extend([0, len + 2, len]);
        wasm.extend(name.bytes());
        wasm.push(1);
    }
    assert_eq!(compiled_modules(&wasm), ["DB", "Auth"]);
    wasm.pop();
    assert!(compiled_modules(&wasm).is_empty());
}
