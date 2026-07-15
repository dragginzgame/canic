use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn unique_temp_repo(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "canic-release-flow-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn run_git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git should run");
    assert!(
        output.status.success(),
        "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory should be created");
    }
    fs::write(&path, contents).unwrap_or_else(|err| panic!("failed to write {relative}: {err}"));
}

fn write_executable(root: &Path, relative: &str, contents: &str) {
    write_file(root, relative, contents);
    let path = root.join(relative);
    let mut permissions = fs::metadata(&path)
        .expect("executable metadata should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("executable mode should be set");
}

fn commit_all(root: &Path, message: &str) {
    run_git(root, &["add", "."]);
    run_git(
        root,
        &[
            "-c",
            "user.name=Canic Test",
            "-c",
            "user.email=canic@example.invalid",
            "commit",
            "-m",
            message,
        ],
    );
}

fn tag_release(root: &Path, version: &str) {
    run_git(
        root,
        &[
            "-c",
            "user.name=Canic Test",
            "-c",
            "user.email=canic@example.invalid",
            "tag",
            "-a",
            &format!("v{version}"),
            "-m",
            &format!("Release {version}"),
        ],
    );
}

fn create_release_repo(name: &str) -> PathBuf {
    let root = unique_temp_repo(name);
    fs::create_dir_all(&root).expect("temp repo should be created");
    run_git(&root, &["init"]);
    write_file(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.92.8\"\n",
    );
    write_file(&root, "Cargo.lock", "# initial\n");
    commit_all(&root, "implementation");
    root
}

fn create_release_commit(root: &Path) {
    write_file(root, "Cargo.lock", "# release\n");
    commit_all(root, "Release 0.92.8");
}

fn run_push_guard(root: &Path) -> Output {
    Command::new("bash")
        .arg(workspace_root().join("scripts/ci/check-release-push-ready.sh"))
        .current_dir(root)
        .output()
        .expect("release push guard should run")
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn release_push_guard_accepts_clean_tagged_release_head() {
    let root = create_release_repo("push-ready");
    create_release_commit(&root);
    tag_release(&root, "0.92.8");

    let output = run_push_guard(&root);

    assert!(
        output.status.success(),
        "guard should accept the exact release commit and tag\n{}",
        output_text(&output)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_push_guard_rejects_missing_tag() {
    let root = create_release_repo("push-missing-tag");
    create_release_commit(&root);

    let output = run_push_guard(&root);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "guard should reject a missing tag"
    );
    assert!(text.contains("annotated tag v0.92.8 is missing"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_push_guard_rejects_tag_on_another_commit() {
    let root = create_release_repo("push-wrong-tag-target");
    tag_release(&root, "0.92.8");
    create_release_commit(&root);

    let output = run_push_guard(&root);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "guard should reject a tag that does not identify HEAD"
    );
    assert!(text.contains("v0.92.8 does not identify HEAD"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_push_guard_rejects_dirty_release_head() {
    let root = create_release_repo("push-dirty");
    create_release_commit(&root);
    tag_release(&root, "0.92.8");
    write_file(&root, "untracked.txt", "dirty\n");

    let output = run_push_guard(&root);
    let text = output_text(&output);

    assert!(!output.status.success(), "guard should reject dirty state");
    assert!(text.contains("worktree or index is not clean"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn failed_version_surface_sync_restores_every_mutated_file() {
    let root = unique_temp_repo("bump-rollback");
    fs::create_dir_all(&root).expect("temp repo should be created");
    run_git(&root, &["init"]);

    let cargo_toml =
        "[workspace]\nmembers = [\"crates/demo\"]\n\n[workspace.package]\nversion = \"0.92.7\"\n";
    let member_toml = "[package]\nname = \"demo\"\nversion = \"0.92.7\"\nedition = \"2024\"\n";
    let cargo_lock = "# original lock\n";
    let install_script = "CANIC_CLI_VERSION=\"${CANIC_CLI_VERSION:-0.92.7}\"\n";

    write_file(&root, "Cargo.toml", cargo_toml);
    write_file(&root, "crates/demo/Cargo.toml", member_toml);
    write_file(&root, "Cargo.lock", cargo_lock);
    write_file(&root, "scripts/dev/install_dev.sh", install_script);
    write_executable(
        &root,
        "scripts/ci/sync-release-surface-version.sh",
        "#!/usr/bin/env bash\nsed -i 's/0.92.7/0.92.8/' scripts/dev/install_dev.sh\nexit 23\n",
    );
    write_executable(
        &root,
        "fake-bin/cargo",
        r#"#!/usr/bin/env bash
set -euo pipefail
case "$*" in
    "set-version --help")
        exit 0
        ;;
    "get workspace.package.version")
        awk '/^version = / { gsub(/"/, "", $3); print $3; exit }' Cargo.toml
        ;;
    "set-version --workspace --bump patch")
        sed -i 's/0.92.7/0.92.8/g' Cargo.toml crates/demo/Cargo.toml
        ;;
    "generate-lockfile")
        printf '# regenerated lock\n' >Cargo.lock
        ;;
    *)
        echo "unexpected cargo arguments: $*" >&2
        exit 2
        ;;
esac
"#,
    );
    commit_all(&root, "initial");

    let path = format!(
        "{}:{}",
        root.join("fake-bin").display(),
        env::var("PATH").unwrap_or_default()
    );
    let output = Command::new("bash")
        .arg(workspace_root().join("scripts/ci/bump-version.sh"))
        .arg("patch")
        .current_dir(&root)
        .env("CANIC_RELEASE_GATES_PASSED", "1")
        .env("PATH", path)
        .output()
        .expect("bump script should run");
    let text = output_text(&output);

    assert!(!output.status.success(), "the fixture sync must fail");
    assert!(text.contains("restored all release surfaces to 0.92.7"));
    assert_eq!(
        fs::read_to_string(root.join("Cargo.toml")).unwrap(),
        cargo_toml
    );
    assert_eq!(
        fs::read_to_string(root.join("crates/demo/Cargo.toml")).unwrap(),
        member_toml
    );
    assert_eq!(
        fs::read_to_string(root.join("Cargo.lock")).unwrap(),
        cargo_lock
    );
    assert_eq!(
        fs::read_to_string(root.join("scripts/dev/install_dev.sh")).unwrap(),
        install_script
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn make_release_targets_are_sequential_and_push_is_guarded() {
    let makefile =
        fs::read_to_string(workspace_root().join("Makefile")).expect("Makefile should be readable");

    let release_patch = "release-patch:\n\t@$(MAKE) patch\n\t@$(MAKE) release-stage\n\t@$(MAKE) release-commit\n\t@$(MAKE) release-push";
    assert!(
        makefile.contains(release_patch),
        "release-patch must invoke each phase sequentially"
    );

    let release_push = "release-push:\n\t@bash scripts/ci/check-release-push-ready.sh\n\tcargo clean\n\tgit push --atomic --follow-tags";
    assert!(
        makefile.contains(release_push),
        "release-push must validate and clean before an atomic push"
    );
}
