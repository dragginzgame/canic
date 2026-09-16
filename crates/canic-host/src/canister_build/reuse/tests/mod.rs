mod complete;

use super::*;
use crate::test_support::temp_dir;
use std::{io::Write as _, process::Command};

const REUSE_CONFIG: &str = r#"[app]
name = "reuse"
[roles.root]
kind = "root"
[roles.app]
kind = "canister"
package = "."
[component_specs.app]
component_role = "app"
maximum_instances = 1
"#;

// Re-exec only this case with private Cargo output paths. Make exports
// a shared target; these fixtures must not discover or replace each other's .d
// records. A child environment avoids mutating the parallel libtest process.
fn run_with_private_cargo_target(test: fn()) {
    const CHILD_ENV: &str = "CANIC_TEST_PRIVATE_REUSE_TARGET";
    let thread = std::thread::current();
    let test_name = thread.name().expect("libtest names each test thread");
    if env::var(CHILD_ENV).as_deref() == Ok(test_name) {
        test();
        return;
    }
    let scratch = temp_dir("reuse-cargo-target");
    fs::create_dir_all(&scratch).unwrap();
    let output = Command::new(env::current_exe().unwrap())
        .args(["--exact", test_name])
        .env(CHILD_ENV, test_name)
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .env("CARGO_BUILD_BUILD_DIR", scratch.join("build"))
        .output()
        .unwrap();
    fs::remove_dir_all(scratch).unwrap();
    assert!(
        output.status.success(),
        "isolated reuse test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

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
    fs::write(app.join("canic.toml"), REUSE_CONFIG).unwrap();
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
    let context = infrastructure_build_context(app);
    (root, context)
}

fn infrastructure_build_context(app: PathBuf) -> WorkspaceBuildContext {
    WorkspaceBuildContext {
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
    }
}

fn compile_infrastructure_fixture(context: &WorkspaceBuildContext) {
    compile_infrastructure_fixture_at(
        context,
        &crate::canister_build::cache::canister_build_target_root(&context.workspace_root),
    );
}

fn compile_infrastructure_fixture_at(context: &WorkspaceBuildContext, target: &Path) {
    let output = crate::cargo_command()
        .args(["build", "--locked", "--offline", "--manifest-path"])
        .arg(context.workspace_root.join("Cargo.toml"))
        .args([
            "--target",
            "wasm32-unknown-unknown",
            "--profile",
            context.profile.target_dir_name(),
            "--features",
            "canic/fleet",
        ])
        .env("CARGO_TARGET_DIR", target)
        .env_remove("CARGO_BUILD_BUILD_DIR")
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
fn repeated_cargo_inputs_preserve_one_snapshot_and_refresh_the_next() {
    run_with_private_cargo_target(repeated_cargo_inputs);
}

fn repeated_cargo_inputs() {
    let (root, context) = infrastructure_build_fixture();
    let input = context.workspace_root.join("src/lib.rs");
    let mut files = BTreeMap::new();
    add_file(&input, &mut files).unwrap();
    let before = BuildInputSnapshot {
        identity: "same build context".into(),
        files: files.clone(),
    };
    let target = crate::canister_build::cache::canister_build_target_root(&context.workspace_root)
        .join("wasm32-unknown-unknown/fast");
    fs::create_dir_all(&target).unwrap();
    for role in ["root", "app"] {
        fs::write(
            target.join(format!("{role}.d")),
            format!("{role}.wasm: {} {}\n", input.display(), input.display()),
        )
        .unwrap();
    }
    fs::write(&input, "pub fn changed() {}\n").unwrap();
    dependencies::append_observed_cargo_inputs(&context, &mut files).unwrap();
    assert_eq!(files, before.files);
    let mut fresh = BTreeMap::new();
    dependencies::append_observed_cargo_inputs(&context, &mut fresh).unwrap();
    assert!(matches!(
        before.validate_after(&BuildInputSnapshot {
            identity: before.identity.clone(),
            files: fresh,
        }),
        Err(BuildReuseError::ChangedInput(path)) if path == input
    ));
    fs::remove_file(&input).unwrap();
    let mut missing = BTreeMap::new();
    dependencies::append_observed_cargo_inputs(&context, &mut missing).unwrap();
    assert_eq!(missing[input.to_str().unwrap()], "absent");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_build_generated_config_is_output_in_both_target_trees() {
    run_with_private_cargo_target(first_generated_config_build);
}

#[test]
fn retained_generated_exports_are_observed_before_dependency_discovery() {
    run_with_private_cargo_target(retained_generated_exports);
}

fn retained_generated_exports() {
    let (root, context) = infrastructure_build_fixture();
    let foreign = root.join("other checkout/target");
    fs::create_dir_all(&foreign).unwrap();
    let config = foreign.join("canic.compact.toml");
    let model = foreign.join("canic.compiled.rs");
    let authority = foreign.join("canic.role-runtime-authority.rs");
    fs::write(&config, "observed = true\n").unwrap();
    fs::write(&model, "pub const MODEL: u8 = 3;\n").unwrap();
    fs::write(&authority, "pub const AUTHORITY: u8 = 4;\n").unwrap();
    fs::write(
        context.workspace_root.join("build.rs"),
        format!(
            r#"
fn main() {{
    println!("cargo:rustc-env=CANIC_CONFIG_SOURCE_PATH={{}}", {config:?});
    println!("cargo::rustc-env=CANIC_CONFIG_MODEL_PATH={{}}", {model:?});
    println!("cargo:rustc-env=CANIC_ROLE_RUNTIME_AUTHORITY_PATH={{}}", {authority:?});
    println!("cargo:rerun-if-changed=build.rs");
}}
"#
        ),
    )
    .unwrap();
    let source = r#"
pub const CONFIG: &str = include_str!(env!("CANIC_CONFIG_SOURCE_PATH"));
include!(env!("CANIC_CONFIG_MODEL_PATH"));
include!(env!("CANIC_ROLE_RUNTIME_AUTHORITY_PATH"));
pub fn value() -> u8 { canic::value() + MODEL + AUTHORITY }
"#;
    fs::write(context.workspace_root.join("src/lib.rs"), source).unwrap();
    let cold = input_snapshot(&context, &[]).unwrap();
    let targets = [
        crate::canister_build::cache::canister_build_target_root(&context.workspace_root),
        crate::canister_build::cache::declaration_target_root(&context.workspace_root),
    ];
    for target in &targets {
        compile_infrastructure_fixture_at(&context, target);
        fs::remove_file(target.join("wasm32-unknown-unknown/fast/reuse_app.d")).unwrap();
    }
    let exported = input_snapshot(&context, &[]).unwrap();
    for path in [&config, &model, &authority] {
        assert!(exported.files.contains_key(path.to_str().unwrap()));
        assert!(!cold.files.contains_key(path.to_str().unwrap()));
    }
    assert!(matches!(
        cold.validate_after(&exported),
        Err(BuildReuseError::UnobservedInput(_))
    ));

    // Force Rust recompilation while preserving build-script output, as in an
    // incomplete copied target. The new .d files add no unobserved source bytes.
    fs::write(
        context.workspace_root.join("src/lib.rs"),
        format!("{source}\n// recompile consumer\n"),
    )
    .unwrap();
    let before = input_snapshot(&context, &[]).unwrap();
    for target in &targets {
        compile_infrastructure_fixture_at(&context, target);
        assert!(
            target
                .join("wasm32-unknown-unknown/fast/reuse_app.d")
                .is_file()
        );
    }
    before
        .validate_after(&input_snapshot(&context, &[]).unwrap())
        .unwrap();
    fs::write(&config, "observed = false\n").unwrap();
    assert!(
        matches!(before.validate_after(&input_snapshot(&context, &[]).unwrap()),
        Err(BuildReuseError::ChangedInput(path)) if path == config)
    );
    fs::remove_dir_all(root).unwrap();
}

#[expect(
    clippy::too_many_lines,
    reason = "one Cargo fixture qualifies cold and warm generation, source drift and foreign-input discovery"
)]
fn first_generated_config_build() {
    let (root, context) = infrastructure_build_fixture();
    let producer = context.workspace_root.join("build.rs");
    let catalogue = context.workspace_root.join("src/translations.json");
    fs::write(&catalogue, "{ \"revision\": 1 }\n").unwrap();
    fs::write(
        &producer,
        r#"
fn main() {
    let config = std::fs::read("canic.toml").unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("canic.compact.toml"), config).unwrap();
    let catalogue = std::fs::read_to_string("src/translations.json").unwrap();
    let compact: String = catalogue.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    std::fs::write(output.join("translations.json"), compact).unwrap();
    println!("cargo::rustc-env=FIXTURE_CONFIG_PATH={}", output.join("canic.compact.toml").canonicalize().unwrap().display());
    println!("cargo::rerun-if-changed=canic.toml");
    println!("cargo::rerun-if-changed=src/translations.json");
}
"#,
    )
    .unwrap();
    fs::write(
        context.workspace_root.join("src/lib.rs"),
        r#"
pub const CONFIG: &str = include_str!(env!("FIXTURE_CONFIG_PATH"));
pub const CATALOGUE: &str = include_str!(concat!(env!("OUT_DIR"), "/translations.json"));
pub fn value() -> u8 { canic::value() }
"#,
    )
    .unwrap();
    let before = input_snapshot(&context, &[]).unwrap();
    assert!(before.files.contains_key(producer.to_str().unwrap()));
    for target in [
        crate::canister_build::cache::canister_build_target_root(&context.workspace_root),
        crate::canister_build::cache::declaration_target_root(&context.workspace_root),
    ] {
        compile_infrastructure_fixture_at(&context, &target);
        let record = target.join("wasm32-unknown-unknown/fast/reuse_app.d");
        let dependencies = fs::read_to_string(record).unwrap();
        assert!(dependencies.contains("canic.compact.toml"));
        assert!(
            dependencies.contains(
                target
                    .join("wasm32-unknown-unknown/fast/build")
                    .to_str()
                    .unwrap()
            )
        );
    }
    let after = input_snapshot(&context, &[]).unwrap();
    before.validate_after(&after).unwrap();
    assert_eq!(before.digest(), after.digest());
    // A warm build regenerates both declaration and runtime outputs after an
    // authored catalogue edit made before the build. Only its source is frozen.
    fs::write(&catalogue, "{ \"revision\": 2 }\n").unwrap();
    let edited = input_snapshot(&context, &[]).unwrap();
    assert_ne!(edited.digest(), after.digest());
    for target in [
        crate::canister_build::cache::canister_build_target_root(&context.workspace_root),
        crate::canister_build::cache::declaration_target_root(&context.workspace_root),
    ] {
        let outputs = fs::read_dir(target.join("wasm32-unknown-unknown/fast/build"))
            .unwrap()
            .map(|entry| entry.unwrap().path().join("out/translations.json"))
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        assert!(!outputs.is_empty());
        for output in &outputs {
            assert_eq!(fs::read_to_string(output).unwrap(), "{\"revision\":1}");
            assert!(!edited.files.contains_key(output.to_str().unwrap()));
        }
        compile_infrastructure_fixture_at(&context, &target);
        for output in outputs {
            assert_eq!(fs::read_to_string(output).unwrap(), "{\"revision\":2}");
        }
    }
    edited
        .validate_after(&input_snapshot(&context, &[]).unwrap())
        .unwrap();
    fs::write(&catalogue, "{ \"revision\": 3 }\n").unwrap();
    assert!(
        matches!(edited.validate_after(&input_snapshot(&context, &[]).unwrap()),
        Err(BuildReuseError::ChangedInput(path)) if path == catalogue)
    );
    fs::write(&catalogue, "{ \"revision\": 2 }\n").unwrap();
    fs::write(&producer, "fn main() {}\n").unwrap();
    assert!(
        matches!(edited.validate_after(&input_snapshot(&context, &[]).unwrap()),
        Err(BuildReuseError::ChangedInput(path)) if path == producer)
    );

    // A copied Cargo output can still name generated input in another checkout.
    // Its generated filename does not make that external input prevalidated.
    let foreign = root.join("other-checkout/target/canic.compact.toml");
    fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    fs::write(&foreign, "foreign = true\n").unwrap();
    fs::write(
        context.workspace_root.join("src/lib.rs"),
        format!("pub const CONFIG: &str = include_str!({foreign:?});\n"),
    )
    .unwrap();
    let before_foreign = input_snapshot(&context, &[]).unwrap();
    assert!(!before_foreign.files.contains_key(foreign.to_str().unwrap()));
    compile_infrastructure_fixture(&context);
    assert!(
        matches!(before_foreign.validate_after(&input_snapshot(&context, &[]).unwrap()),
        Err(BuildReuseError::UnobservedInput(path)) if path == foreign)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_release_build_admits_cargo_discovery_of_absent_package_prefixed_build_script() {
    run_with_private_cargo_target(first_release_build_with_absent_cargo_input);
}

fn first_release_build_with_absent_cargo_input() {
    let (root, mut context) = infrastructure_build_fixture();
    context.profile = crate::canister_build::CanisterBuildProfile::Release;
    let manifest = context.workspace_root.join("Cargo.toml");
    let contents = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        contents.replace(
            "edition = \"2024\"",
            "edition = \"2024\"\nbuild = \"src/build.rs\"",
        ),
    )
    .unwrap();
    let source = context.workspace_root.join("src/build.rs");
    fs::write(
        &source,
        "fn main() { println!(\"cargo::rerun-if-changed=app/src/build.rs\"); }\n",
    )
    .unwrap();
    let missing = context.workspace_root.join("app/src/build.rs");
    let before = input_snapshot(&context, &[]).unwrap();
    assert!(!before.files.contains_key(missing.to_str().unwrap()));
    for target in [
        crate::canister_build::cache::canister_build_target_root(&context.workspace_root),
        crate::canister_build::cache::declaration_target_root(&context.workspace_root),
    ] {
        let record = target.join("wasm32-unknown-unknown/release/reuse_app.d");
        assert!(!record.exists());
        compile_infrastructure_fixture_at(&context, &target);
        assert!(
            fs::read_to_string(record)
                .unwrap()
                .contains(missing.to_str().unwrap())
        );
    }
    assert!(!missing.exists());
    let after = input_snapshot(&context, &[]).unwrap();
    assert_eq!(after.files[missing.to_str().unwrap()], "absent");
    before.validate_after(&after).unwrap();
    assert_ne!(before.digest(), after.digest());
    compile_infrastructure_fixture(&context);
    let replay = input_snapshot(&context, &[]).unwrap();
    after.validate_after(&replay).unwrap();
    assert_eq!(after.digest(), replay.digest());

    fs::write(&source, b"fn main() {}\n").unwrap();
    assert!(
        matches!(after.validate_after(&input_snapshot(&context, &[]).unwrap()), Err(BuildReuseError::ChangedInput(path)) if path == source)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_infrastructure_build_and_replaced_cargo_records_preserve_source_authority() {
    run_with_private_cargo_target(first_infrastructure_build_fixture_preserves_source_authority);
}

fn first_infrastructure_build_fixture_preserves_source_authority() {
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
    run_with_private_cargo_target(governed_fixture_inputs_invalidate_reuse);
}

fn governed_fixture_inputs_invalidate_reuse() {
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
    fs::write(root.join("canic.toml"), REUSE_CONFIG).unwrap();
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
    let original_snapshot = input_snapshot(&context, &tools).unwrap();
    let original = original_snapshot.digest();
    let changed_config = format!("# changed configuration input\n{REUSE_CONFIG}");
    for (path, replacement) in [
        ("src/lib.rs", "pub const VALUE: u8 = 2;\n"),
        ("canic.toml", changed_config.as_str()),
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
    let restored = input_snapshot(&context, &tools).unwrap();
    original_snapshot
        .validate_after(&restored)
        .expect("restored fixture inputs retain their original authority");
    assert_eq!(original, restored.digest());
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

#[test]
fn declared_fixture_payloads_participate_in_cache_inputs_inside_excluded_directories() {
    let root = temp_dir("reuse-declared-fixture");
    fs::create_dir_all(root.join(".canic/source")).unwrap();
    fs::write(root.join("canic.toml"), REUSE_CONFIG).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[package.metadata.canic]\nfixture = \".canic/source/fixture.json\"\n").unwrap();
    fs::write(root.join(".canic/source/fixture.json"), serde_json::to_vec(&serde_json::json!({
        "format_hash": "01".repeat(32), "completion_summary": "02".repeat(32), "chunk_paths": ["rows.bin"]
    })).unwrap()).unwrap();
    let rows = root.join(".canic/source/rows.bin");
    fs::write(&rows, [3; 64]).unwrap();
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
    let mut files = BTreeMap::new();
    collect_files(&root, &root, &mut files, true).unwrap();
    assert!(!files.contains_key(rows.to_str().unwrap()));
    append_fixture_inputs(&context, &mut files).unwrap();
    assert!(files.contains_key(rows.to_str().unwrap()));
    let before = BuildInputSnapshot {
        identity: "fixture".into(),
        files,
    };
    fs::write(&rows, [4; 64]).unwrap();
    let mut files = BTreeMap::new();
    collect_files(&root, &root, &mut files, true).unwrap();
    append_fixture_inputs(&context, &mut files).unwrap();
    let after = BuildInputSnapshot {
        identity: "fixture".into(),
        files,
    };
    assert_ne!(before.digest(), after.digest());
    assert!(
        matches!(before.validate_after(&after), Err(BuildReuseError::ChangedInput(path)) if path == rows)
    );
    fs::remove_dir_all(root).unwrap();
}
