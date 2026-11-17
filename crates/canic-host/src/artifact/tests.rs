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
