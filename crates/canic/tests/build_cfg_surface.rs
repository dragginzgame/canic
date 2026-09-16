use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

// Run the real build macro from this already-linked test executable. The tiny
// Cargo consumer below exercises Cargo invalidation without rebuilding Canic.
#[test]
fn build_macro_tracks_config_without_tracking_neighboring_artifacts() {
    if env::var_os("CANIC_BUILD_FRESHNESS_CHILD").is_some() {
        println!();
        canic::build!("canic.toml");
        return;
    }
    let root = env::temp_dir().join(format!(
        "canic-build-freshness-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("config")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"
[workspace]
[package]
name = "canic_freshness_probe"
version = "0.0.0"
edition = "2024"
[package.metadata.canic]
app = "standalone"
role = "minimal"
"#,
    )
    .unwrap();
    fs::write(
        root.join("Cargo.lock"),
        "version = 4\n[[package]]\nname = \"canic_freshness_probe\"\nversion = \"0.0.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub const ROLE: &str = env!(\"CANIC_CANISTER_ROLE\");\n",
    )
    .unwrap();
    fs::write(root.join("build.rs"), BUILD_MACRO_PROBE).unwrap();
    let config = root.join("config/canic.toml");
    let source = "[app]\nname = \"standalone\"\ninit_mode = \"enabled\"\n[roles.minimal]\nkind = \"canister\"\npackage = \".\"\n[auth.delegated_tokens]\nenabled = false\n";
    fs::write(&config, source).unwrap();
    let settled = settle_build_macro(&root);
    fs::create_dir_all(root.join("config/.canic/release-builds")).unwrap();
    fs::write(
        root.join("config/.canic/release-builds/artifact.wasm"),
        b"unrelated output",
    )
    .unwrap();
    assert_build_macro_runs(&root, settled);
    fs::write(
        &config,
        format!("{source}\n# reviewed configuration changed\n"),
    )
    .unwrap();
    assert_build_macro_runs(&root, settled + 1);
    assert_build_macro_runs(&root, settled + 1);
    let generated = fs::read_to_string(root.join("generated-path")).unwrap();
    let expected = fs::read(&generated).unwrap();
    fs::remove_file(&generated).unwrap();
    assert_build_macro_runs(&root, settled + 2);
    assert_eq!(fs::read(&generated).unwrap(), expected);
    let repaired = settle_build_macro(&root);
    fs::write(&generated, b"tampered authority").unwrap();
    assert_build_macro_runs(&root, repaired + 1);
    assert_eq!(fs::read(&generated).unwrap(), expected);
    let repaired = settle_build_macro(&root);
    fs::remove_file(&config).unwrap();
    assert!(!run_build_macro_probe(&root).status.success());
    fs::write(&config, source).unwrap();
    assert_build_macro_runs(&root, repaired + 1);
    let manifest = root.join("Cargo.toml");
    let metadata = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        metadata.replace("role = \"minimal\"", "role = \"other\""),
    )
    .unwrap();
    assert!(!run_build_macro_probe(&root).status.success());
    fs::write(&manifest, metadata).unwrap();
    assert_build_macro_runs(&root, repaired + 2);
    fs::remove_dir_all(root).unwrap();
}

fn run_build_macro_probe(root: &Path) -> Output {
    Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .current_dir(root)
        .args(["check", "--locked", "--offline"])
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env_remove("CARGO_BUILD_BUILD_DIR")
        .env(
            "CANIC_INTERNAL_BUILD_CONFIG_PATH",
            root.join("config/canic.toml"),
        )
        .env(
            "CANIC_BUILD_FRESHNESS_EXECUTABLE",
            env::current_exe().unwrap(),
        )
        .env("CANIC_BUILD_FRESHNESS_LOG", root.join("runs"))
        .env("CANIC_BUILD_FRESHNESS_OUTPUT", root.join("generated-path"))
        .output()
        .unwrap()
}

#[track_caller]
fn assert_build_macro_runs(root: &Path, expected: usize) {
    assert_eq!(
        build_macro_runs(root),
        expected,
        "only changed authoritative inputs or missing generated outputs should rerun the build macro"
    );
}

fn build_macro_runs(root: &Path) -> usize {
    let output = run_build_macro_probe(root);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read(root.join("runs")).unwrap().len()
}

fn settle_build_macro(root: &Path) -> usize {
    let first = build_macro_runs(root);
    let settled = build_macro_runs(root);
    // A newly written output can trigger one repair-watch rerun; unchanged
    // output must then stay fresh instead of continually rewriting itself.
    assert!(settled <= first + 1);
    assert_build_macro_runs(root, settled);
    settled
}

const BUILD_MACRO_PROBE: &str = r#"
use std::{env, fs, io::Write, process::Command};
fn main() {
    let output = Command::new(env::var_os("CANIC_BUILD_FRESHNESS_EXECUTABLE").unwrap())
        .env("CANIC_BUILD_FRESHNESS_CHILD", "1")
        .args(["--exact", "build_macro_tracks_config_without_tracking_neighboring_artifacts", "--nocapture"])
        .output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    for line in String::from_utf8(output.stdout).unwrap().lines().filter(|line| line.starts_with("cargo:")) {
        println!("{line}");
        if let Some(path) = line.strip_prefix("cargo:rustc-env=CANIC_ROLE_RUNTIME_AUTHORITY_PATH=") {
            fs::write(env::var_os("CANIC_BUILD_FRESHNESS_OUTPUT").unwrap(), path).unwrap();
        }
    }
    fs::OpenOptions::new().create(true).append(true)
        .open(env::var_os("CANIC_BUILD_FRESHNESS_LOG").unwrap()).unwrap().write_all(b"x").unwrap();
}
"#;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn rust_sources_under(path: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(path).expect("macro source directory should be readable") {
        let path = entry.expect("macro source entry should be readable").path();
        if path.is_dir() {
            sources.extend(rust_sources_under(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    sources
}

fn custom_identifiers(source: &str) -> BTreeSet<String> {
    let mut identifiers = BTreeSet::new();
    let mut remaining = source;
    while let Some(offset) = remaining.find("canic_") {
        let candidate = &remaining[offset..];
        let length = candidate
            .bytes()
            .take_while(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
            .count();
        identifiers.insert(candidate[..length].to_string());
        remaining = &candidate[length..];
    }
    identifiers
}

fn consumed_custom_cfgs(root: &Path) -> BTreeSet<String> {
    rust_sources_under(&root.join("crates/canic/src/macros"))
        .into_iter()
        .flat_map(|path| {
            let source = fs::read_to_string(path).expect("macro source should be readable");
            source
                .split("#[cfg")
                .skip(1)
                .filter_map(|suffix| suffix.split_once(']').map(|(expression, _)| expression))
                .flat_map(custom_identifiers)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn emitted_custom_cfgs(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter(|line| line.contains("cargo:rustc-cfg=canic_"))
        .flat_map(custom_identifiers)
        .collect()
}

#[test]
fn custom_cfg_catalog_is_exact_and_singly_owned() {
    let root = workspace_root();
    let expected = canic::__build::CANIC_CUSTOM_CFG_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected.len(), canic::__build::CANIC_CUSTOM_CFG_NAMES.len());
    assert_eq!(consumed_custom_cfgs(&root), expected);

    let build_macro = fs::read_to_string(root.join("crates/canic/src/macros/build.rs"))
        .expect("build macro source should be readable");
    assert_eq!(emitted_custom_cfgs(&build_macro), expected);
    assert!(!build_macro.contains("cargo:rustc-check-cfg=cfg(canic_"));
    assert!(build_macro.contains("$crate::__build::CANIC_CUSTOM_CFG_NAMES"));

    let facade_build = fs::read_to_string(root.join("crates/canic/build.rs"))
        .expect("facade build script should be readable");
    assert!(facade_build.contains("include!(\"src/build_support/cfg_catalog.rs\")"));
    assert!(facade_build.contains("for custom_cfg in CANIC_CUSTOM_CFG_NAMES"));
    assert!(!facade_build.contains("cargo:rustc-check-cfg=cfg(canic_"));

    let core_build = fs::read_to_string(root.join("crates/canic-core/build.rs"))
        .expect("core build script should be readable");
    assert!(!core_build.contains("canic_is_root"));

    for removed in [
        "CANIC_APP_ROLE",
        "CANIC_APP=",
        "CANIC_CANISTER_ROLE_DECLARED",
        "CANIC_CANISTER_ROLE_ATTACHED",
    ] {
        assert!(
            !build_macro.contains(removed),
            "removed compile-time output returned: {removed}"
        );
    }
}
