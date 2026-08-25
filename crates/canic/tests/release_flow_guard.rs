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

fn install_version_reader(root: &Path) {
    let source = fs::read_to_string(workspace_root().join("scripts/ci/read-workspace-version.sh"))
        .expect("workspace-version reader should be readable");
    write_executable(root, "scripts/ci/read-workspace-version.sh", &source);
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
    install_version_reader(&root);
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

fn create_candidate_repo(name: &str) -> (PathBuf, String) {
    let root = unique_temp_repo(name);
    fs::create_dir_all(&root).expect("temp repo should be created");
    run_git(&root, &["init"]);
    write_file(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.92.7\"\n",
    );
    write_file(&root, "Cargo.lock", "# lock\n");
    write_file(&root, "CHANGELOG.md", "- `0.92.8` fixture release\n");
    write_file(
        &root,
        "docs/changelog/0.92.md",
        "# Fixture changelog\n\n## 0.92.8 - Unreleased\n",
    );
    write_file(
        &root,
        "docs/status/current.md",
        "Release governance: source development state; no validated release candidate is staged.\n",
    );
    write_file(
        &root,
        "scripts/dev/install_dev.sh",
        "CANIC_CLI_VERSION=\"${CANIC_CLI_VERSION:-0.92.7}\"\n",
    );
    install_version_reader(&root);
    let candidate_guard =
        fs::read_to_string(workspace_root().join("scripts/ci/check-release-candidate.sh"))
            .expect("candidate guard should be readable");
    write_executable(
        &root,
        "scripts/ci/check-release-candidate.sh",
        &candidate_guard,
    );
    write_file(&root, "src/lib.rs", "pub fn validated_source() {}\n");
    commit_all(&root, "validated source");
    let source = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .expect("source revision should resolve");
    assert!(source.status.success());
    let source = String::from_utf8(source.stdout)
        .expect("source revision should be UTF-8")
        .trim()
        .to_string();

    write_file(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.92.8\"\n",
    );
    write_file(
        &root,
        "scripts/dev/install_dev.sh",
        "CANIC_CLI_VERSION=\"${CANIC_CLI_VERSION:-0.92.8}\"\n",
    );
    write_file(
        &root,
        "docs/changelog/0.92.md",
        "# Fixture changelog\n\n## 0.92.8 - 2026-08-25\n",
    );
    write_file(
        &root,
        "docs/status/current.md",
        &format!(
            "Release validation: `0.92.8` was validated from source `{source}` on `2026-08-25`; the release commit may differ only in governed release surfaces.\n"
        ),
    );
    (root, source)
}

fn run_candidate_guard(root: &Path) -> Output {
    Command::new("bash")
        .arg("scripts/ci/check-release-candidate.sh")
        .current_dir(root)
        .output()
        .expect("candidate guard should run")
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
fn release_push_guard_uses_tagged_head_while_local_changes_remain_unpushed() {
    let root = create_release_repo("push-dirty");
    create_release_commit(&root);
    tag_release(&root, "0.92.8");

    write_file(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"9.9.9\"\n",
    );
    run_git(&root, &["add", "Cargo.toml"]);
    write_file(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"10.0.0\"\n",
    );
    write_file(&root, "untracked.txt", "dirty\n");

    let output = run_push_guard(&root);
    let text = output_text(&output);

    assert!(
        output.status.success(),
        "guard should validate committed HEAD independently of local changes\n{text}"
    );
    assert!(text.contains("with v0.92.8"));
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
    write_file(&root, "CHANGELOG.md", "- `0.92.8` fixture release\n");
    write_file(
        &root,
        "docs/changelog/0.92.md",
        "# Fixture changelog\n\n## 0.92.8 - Unreleased\n",
    );
    write_file(
        &root,
        "docs/status/current.md",
        "Release governance: source development state; no validated release candidate is staged.\n",
    );
    write_file(&root, "scripts/dev/install_dev.sh", install_script);
    install_version_reader(&root);
    install_failing_release_fixture_commands(&root);
    commit_all(&root, "initial");

    let validated_head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .expect("git revision should resolve");
    assert!(validated_head.status.success());
    let validated_head = String::from_utf8(validated_head.stdout)
        .expect("revision should be UTF-8")
        .trim()
        .to_string();

    let path = format!(
        "{}:{}",
        root.join("fake-bin").display(),
        env::var("PATH").unwrap_or_default()
    );
    let output = Command::new("bash")
        .arg(workspace_root().join("scripts/ci/bump-version.sh"))
        .arg("patch")
        .current_dir(&root)
        .env("CANIC_RELEASE_VALIDATED", "1")
        .env("CANIC_RELEASE_VALIDATED_HEAD", validated_head)
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

fn install_failing_release_fixture_commands(root: &Path) {
    write_executable(
        root,
        "scripts/ci/sync-release-surface-version.sh",
        "#!/usr/bin/env bash\nsed -i 's/0.92.7/0.92.8/' scripts/dev/install_dev.sh\nexit 23\n",
    );
    write_executable(
        root,
        "fake-bin/cargo",
        r#"#!/usr/bin/env bash
set -euo pipefail
case "$*" in
    "set-version --help")
        exit 0
        ;;
    "get --version")
        printf 'cargo-get 1.4.0\n'
        ;;
    get\ --entry\ *\ workspace.package.version)
        awk '/^version = / { gsub(/"/, "", $3); print $3; exit }' "$3/Cargo.toml"
        ;;
    "set-version --workspace --bump patch")
        sed -i 's/0.92.7/0.92.8/g' Cargo.toml crates/demo/Cargo.toml
        ;;
    "update --workspace --offline")
        printf '# regenerated lock\n' >Cargo.lock
        ;;
    *)
        echo "unexpected cargo arguments: $*" >&2
        exit 2
        ;;
esac
"#,
    );
}

#[test]
fn release_candidate_accepts_only_sealed_release_mutation_after_validation() {
    let (root, source) = create_candidate_repo("candidate-release-only");

    let output = run_candidate_guard(&root);

    assert!(
        output.status.success(),
        "guard should accept governed release mutation\n{}",
        output_text(&output)
    );
    assert!(output_text(&output).contains(&format!("validated source {source}")));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_candidate_rejects_unsealed_changelog_and_late_source_change() {
    let (root, _) = create_candidate_repo("candidate-rejections");
    write_file(
        &root,
        "docs/changelog/0.92.md",
        "# Fixture changelog\n\n## 0.92.8 - Unreleased\n",
    );
    let unsealed = run_candidate_guard(&root);
    assert!(!unsealed.status.success());
    assert!(output_text(&unsealed).contains("changelog is not sealed"));

    write_file(
        &root,
        "docs/changelog/0.92.md",
        "# Fixture changelog\n\n## 0.92.8 - 2026-08-25\n",
    );
    write_file(
        &root,
        "src/lib.rs",
        "pub fn validated_source() {}\npub fn late_change() {}\n",
    );
    let late_change = run_candidate_guard(&root);
    assert!(!late_change.status.success());
    assert!(
        output_text(&late_change)
            .contains("validated source is followed by non-release change: src/lib.rs")
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

    let release_commit = "release-commit:\n\t@scripts/ci/check-release-index.sh\n\t@$(MAKE) --no-print-directory release-candidate";
    assert!(
        makefile.contains(release_commit),
        "release-commit must verify the exact post-bump candidate before tagging"
    );

    let release_push = "release-push:\n\t@bash scripts/ci/check-release-push-ready.sh\n\t@CANIC_RELEASE_PUSH_READY=1 bash scripts/ci/push-release.sh";
    assert!(
        makefile.contains(release_push),
        "release-push must perform only readiness checking and the atomic push"
    );
}
