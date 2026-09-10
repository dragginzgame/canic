use super::*;
use crate::test_support::temp_dir;
use std::io::Write as _;

fn infrastructure_build_fixture() -> (PathBuf, WorkspaceBuildContext) {
    let root = temp_dir("reuse-first-infrastructure");
    let app = root.join("app");
    let canic = root.join("upstream/canic");
    let control = root.join("upstream/canic-control-plane");
    for directory in [&app, &canic, &control] {
        fs::create_dir_all(directory.join("src")).unwrap();
    }
    fs::write(
        app.join("Cargo.toml"),
        r#"
[workspace]
[package]
name = "reuse-app"
version = "0.1.0"
edition = "2024"
[lib]
crate-type = ["cdylib"]
[dependencies]
canic = { path = "../upstream/canic", default-features = false }
[profile.fast]
inherits = "release"
"#,
    )
    .unwrap();
    fs::write(
        canic.join("Cargo.toml"),
        r#"
[package]
name = "canic"
version = "0.1.0"
edition = "2024"
[features]
fleet = ["dep:canic-control-plane"]
[dependencies]
canic-control-plane = { path = "../canic-control-plane", optional = true }
"#,
    )
    .unwrap();
    fs::write(
        control.join("Cargo.toml"),
        "[package]\nname = \"canic-control-plane\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        app.join("src/lib.rs"),
        "pub fn value() -> u8 { canic::value() }\n",
    )
    .unwrap();
    fs::write(canic.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n#[cfg(feature = \"fleet\")] pub use canic_control_plane::CONTROL;\n").unwrap();
    let control_source = control.join("src/lib.rs");
    fs::write(&control_source, "pub const CONTROL: u8 = 1;\n").unwrap();
    fs::write(app.join("canic.toml"), "fixture = true\n").unwrap();
    let lock = crate::cargo_command()
        .args(["generate-lockfile", "--offline", "--manifest-path"])
        .arg(app.join("Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let context = WorkspaceBuildContext {
        role: "root".into(),
        profile: crate::canister_build::CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: canic_core::ids::BuildNetwork::Local,
        workspace_root: app.clone(),
        icp_root: app.clone(),
        config_path: app.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    (root, context)
}

fn compile_infrastructure_fixture(context: &WorkspaceBuildContext) {
    let output = crate::cargo_command()
        .args(["build", "--locked", "--offline", "--manifest-path"])
        .arg(context.workspace_root.join("Cargo.toml"))
        .args([
            "--target",
            "wasm32-unknown-unknown",
            "--profile",
            "fast",
            "--features",
            "canic/fleet",
        ])
        .env(
            "CARGO_TARGET_DIR",
            crate::canister_build::cache::canister_build_target_root(&context.workspace_root),
        )
        .env("RUSTC_WRAPPER", "")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn first_infrastructure_build_and_replaced_cargo_records_preserve_source_authority() {
    let (root, context) = infrastructure_build_fixture();
    let control_source = root.join("upstream/canic-control-plane/src/lib.rs");
    let metadata =
        cargo_metadata_catalog_for_manifest(&context.workspace_root.join("Cargo.toml"), true, true)
            .unwrap();
    assert!(
        !metadata
            .packages
            .iter()
            .any(|package| package.name == "canic-control-plane")
    );
    let before = input_snapshot(&context, &[]).unwrap();
    assert!(before.files.contains_key(control_source.to_str().unwrap()));
    let target = crate::canister_build::cache::canister_build_target_root(&context.workspace_root);
    compile_infrastructure_fixture(&context);
    let after = input_snapshot(&context, &[]).unwrap();
    before.validate_after(&after).unwrap();
    assert_eq!(before.digest(), after.digest());

    // An earlier build may have recorded a now-unused external source.
    let old = root.join("previous.rs");
    fs::write(&old, "pub const PREVIOUS: u8 = 1;\n").unwrap();
    let record = target.join("wasm32-unknown-unknown/fast/reuse_app.d");
    assert!(record.is_file());
    fs::write(&record, format!("/unused.wasm: {}\n", old.display())).unwrap();
    let stale = input_snapshot(&context, &[]).unwrap();
    fs::remove_file(&record).unwrap();
    compile_infrastructure_fixture(&context);
    let refreshed = input_snapshot(&context, &[]).unwrap();
    stale.validate_after(&refreshed).unwrap();
    assert_ne!(stale.digest(), refreshed.digest());
    assert_eq!(after.digest(), refreshed.digest());
    fs::write(&control_source, "pub const CONTROL: u8 = 2;\n").unwrap();
    assert!(
        matches!(refreshed.validate_after(&input_snapshot(&context, &[]).unwrap()), Err(BuildReuseError::ChangedInput(path)) if path == control_source)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn governed_inputs_invalidate_reuse_even_when_file_lengths_are_unchanged() {
    let root = temp_dir("build-reuse-inputs");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\n[package]\nname = \"reuse-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    ).unwrap();
    fs::write(
        root.join("Cargo.lock"),
        "version = 4\n[[package]]\nname = \"reuse-fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub const VALUE: u8 = 1;\n").unwrap();
    fs::write(root.join("canic.toml"), "configuration = 1\n").unwrap();
    fs::write(root.join("tool"), "tool payload 1\n").unwrap();
    let context = WorkspaceBuildContext {
        role: "root".into(),
        profile: crate::canister_build::CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: canic_core::ids::BuildNetwork::Local,
        workspace_root: root.clone(),
        icp_root: root.clone(),
        config_path: root.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let tools = [root.join("tool")];
    let original = input_digest(&context, &tools).unwrap();
    for (path, replacement) in [
        ("src/lib.rs", "pub const VALUE: u8 = 2;\n"),
        ("canic.toml", "configuration = 2\n"),
        ("tool", "tool payload 2\n"),
        (
            "Cargo.lock",
            "# changed lock authority\nversion = 4\n[[package]]\nname = \"reuse-fixture\"\nversion = \"0.1.0\"\n",
        ),
    ] {
        let path = root.join(path);
        let bytes = fs::read(&path).unwrap();
        fs::write(&path, replacement).unwrap();
        assert_ne!(original, input_digest(&context, &tools).unwrap());
        fs::write(&path, bytes).unwrap();
    }
    let mut changed = context.clone();
    changed.build_network = canic_core::ids::BuildNetwork::Ic;
    assert_ne!(original, input_digest(&changed, &tools).unwrap());
    changed = context.clone();
    changed.profile = crate::canister_build::CanisterBuildProfile::Release;
    assert_ne!(original, input_digest(&changed, &tools).unwrap());
    assert_eq!(original, input_digest(&context, &tools).unwrap());
    let external = temp_dir("build-reuse-external");
    fs::create_dir_all(&external).unwrap();
    let included = external.join("shared.rs");
    fs::write(&included, "pub const SHARED: u8 = 1;\n").unwrap();
    let build_script = external.join("build.rs");
    fs::write(&build_script, "fn main() {}\n").unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(root.join("Cargo.toml"))
        .unwrap()
        .write_all(format!("build = {build_script:?}\n").as_bytes())
        .unwrap();
    let declared = input_digest(&context, &tools).unwrap();
    fs::write(
        &build_script,
        "fn main() { println!(\"cargo:rustc-cfg=changed\"); }\n",
    )
    .unwrap();
    assert_ne!(declared, input_digest(&context, &tools).unwrap());
    fs::OpenOptions::new().append(true).open(root.join("Cargo.toml")).unwrap()
        .write_all(b"\n[[example]]\nname = \"omitted-from-published-package\"\npath = \"examples/omitted.rs\"\n").unwrap();
    input_digest(&context, &tools).unwrap();
    let target = crate::canister_build::cache::canister_build_target_root(&root)
        .join("wasm32-unknown-unknown/fast");
    fs::create_dir_all(&target).unwrap();
    fs::write(
        target.join("reuse_fixture.d"),
        format!("/target/reuse_fixture.wasm: {}\n", included.display()),
    )
    .unwrap();
    let observed = input_digest(&context, &tools).unwrap();
    assert_ne!(original, observed);
    fs::write(&included, "pub const SHARED: u8 = 2;\n").unwrap();
    assert_ne!(observed, input_digest(&context, &tools).unwrap());
    fs::remove_dir_all(external).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_bytes_additions_deletions_and_renames_change_the_input_set() {
    let root = temp_dir("build-reuse-source");
    fs::create_dir_all(root.join("src")).unwrap();
    let source = root.join("src/lib.rs");
    fs::write(&source, b"one").unwrap();
    let collect = || {
        let mut files = BTreeMap::new();
        collect_files(&root, &root, &mut files, true).unwrap();
        files
    };
    let original = collect();
    fs::write(&source, b"two").unwrap();
    assert_ne!(original, collect());
    fs::write(&source, b"one").unwrap();
    assert_eq!(original, collect());
    fs::rename(&source, root.join("src/other.rs")).unwrap();
    assert_ne!(original, collect());
    fs::rename(root.join("src/other.rs"), &source).unwrap();
    fs::write(root.join("src/include.txt"), b"new input").unwrap();
    assert_ne!(original, collect());
    fs::remove_file(root.join("src/include.txt")).unwrap();
    fs::create_dir_all(root.join("target")).unwrap();
    fs::write(root.join("target/build-output"), b"generated").unwrap();
    assert_eq!(original, collect());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn output_snapshot_detects_corruption_missing_and_extra_files() {
    let root = temp_dir("build-reuse-output");
    fs::create_dir_all(&root).unwrap();
    let wasm = root.join("role.wasm");
    fs::write(&wasm, b"original").unwrap();
    let original = output_files(&root).unwrap();
    fs::write(&wasm, b"corrupt!").unwrap();
    assert_ne!(original, output_files(&root).unwrap());
    fs::write(&wasm, b"original").unwrap();
    assert_eq!(original, output_files(&root).unwrap());
    fs::write(root.join("extra.did"), b"extra").unwrap();
    assert_ne!(original, output_files(&root).unwrap());
    fs::remove_file(&wasm).unwrap();
    assert_ne!(original, output_files(&root).unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn cache_never_follows_a_replaced_output_symlink() {
    let root = temp_dir("build-reuse-symlink");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("target.wasm");
    fs::write(&target, b"wasm").unwrap();
    std::os::unix::fs::symlink(&target, root.join("role.wasm")).unwrap();
    assert!(matches!(
        output_files(&root),
        Err(BuildReuseError::Unsupported(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn executable_launchers_do_not_claim_their_delegated_tool_bytes() {
    let root = temp_dir("build-reuse-tool");
    fs::create_dir_all(&root).unwrap();
    let tool = root.join("tool");
    fs::write(&tool, b"#!/bin/sh\nexec unknown-tool \"$@\"\n").unwrap();
    assert!(matches!(
        require_native_tool(&tool),
        Err(BuildReuseError::UnboundTool(_))
    ));
    require_native_tool(&env::current_exe().unwrap()).unwrap();
    fs::remove_dir_all(root).unwrap();
}
