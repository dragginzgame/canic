//! Real pinned-compiler output custody and failed compilation refusal.

use super::*;
use crate::test_support::temp_dir;
use sha2_host::{Digest, Sha256};

#[test]
fn incompatible_package_is_refused_before_execution() {
    let root = temp_dir("browser-compiler-pin");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("package.json"),
        br#"{"name":"typescript","version":"0.0.0"}"#,
    )
    .unwrap();
    assert!(matches!(
        admit(Path::new("must-not-execute"), &root),
        Err(BrowserCompilerError::Selection)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires CANIC_FRONTEND_NODE, CANIC_FRONTEND_TYPESCRIPT and CANIC_WIRE_SCHEMA from a managed build"]
fn pinned_compiler_retains_complete_outputs_and_rejects_type_errors() {
    let root = temp_dir("browser-compiler");
    fs::create_dir_all(&root).unwrap();
    let schema = PathBuf::from(std::env::var_os("CANIC_WIRE_SCHEMA").unwrap());
    let bytes = fs::read(&schema).unwrap();
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut bindings = WireBindings {
        schema_sha256: Sha256::digest(&bytes).into(),
        typescript: fs::read_to_string(schema.with_extension("ts")).unwrap(),
        browser_modules: serde_json::from_value(document["browser_modules"].clone()).unwrap(),
    };
    let compiler = BrowserCompiler {
        node: std::env::var_os("CANIC_FRONTEND_NODE").unwrap().into(),
        typescript: std::env::var_os("CANIC_FRONTEND_TYPESCRIPT")
            .unwrap()
            .into(),
    };
    let destination = root.join("bundle.json");
    let first = compile(&root.join("cache"), &destination, &compiler, &bindings).unwrap();
    assert!(!first.acquisition.is_reused());
    for name in MODULES {
        for suffix in ["ts", "js", "d.ts"] {
            assert!(!first.files[&format!("{name}.{suffix}")].is_empty());
        }
    }
    let retained = fs::read(&destination).unwrap();
    fs::write(&destination, b"interrupted output").unwrap();
    let second = compile(&root.join("cache"), &destination, &compiler, &bindings).unwrap();
    assert!(second.acquisition.is_reused());
    assert_eq!(second.files, first.files);
    assert_eq!(fs::read(&destination).unwrap(), retained);
    bindings
        .typescript
        .push_str("\nconst invalid: string = 7;\n");
    assert!(matches!(
        compile(&root.join("cache"), &destination, &compiler, &bindings),
        Err(BrowserCompilerError::Tool(_))
    ));
    assert_eq!(fs::read(&destination).unwrap(), retained);
    drop((first, second));
    fs::remove_dir_all(root).unwrap();
}
