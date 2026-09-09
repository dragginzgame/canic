use super::*;
use crate::test_support::temp_dir;
use std::io::Write as _;

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
