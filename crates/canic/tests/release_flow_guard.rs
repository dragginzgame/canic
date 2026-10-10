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
    for path in [
        "scripts/ci/next-release-version.sh",
        "scripts/ci/finalize-release-changelog.awk",
        "scripts/release/rewrite-owned-lock.sh",
        "scripts/release/adapter.sh",
        "scripts/ci/read-cargo-workspace-version.sh",
        "scripts/ci/rewrite-local-lock-versions.pl",
        "scripts/ci/require-jq.sh",
    ] {
        write_file(
            root,
            path,
            &fs::read_to_string(workspace_root().join(path)).unwrap(),
        );
    }
}

fn install_remote_state_guard(root: &Path) {
    let source =
        fs::read_to_string(workspace_root().join("scripts/ci/check-release-remote-state.sh"))
            .expect("release remote-state guard should be readable");
    write_executable(root, "scripts/ci/check-release-remote-state.sh", &source);
}

fn install_fast_patch_guard(root: &Path) {
    let source =
        fs::read_to_string(workspace_root().join("scripts/ci/check-fast-patch-eligibility.sh"))
            .expect("fast patch guard should be readable");
    write_executable(root, "scripts/ci/check-fast-patch-eligibility.sh", &source);
    let reader = fs::read_to_string(workspace_root().join("scripts/ci/read-release-validation.sh"))
        .expect("validation receipt reader should be readable");
    write_executable(root, "scripts/ci/read-release-validation.sh", &reader);
    let jq_resolver = fs::read_to_string(workspace_root().join("scripts/ci/require-jq.sh"))
        .expect("jq resolver should be readable");
    write_executable(root, "scripts/ci/require-jq.sh", &jq_resolver);
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
    install_remote_state_guard(&root);
    commit_all(&root, "implementation");
    let origin = root.join(".git/fixture-origin.git");
    run_git(
        &root,
        &[
            "init",
            "--bare",
            origin.to_str().expect("origin path should be UTF-8"),
        ],
    );
    run_git(
        &root,
        &[
            "remote",
            "add",
            "origin",
            origin.to_str().expect("origin path should be UTF-8"),
        ],
    );
    let branch = git_output(&root, &["branch", "--show-current"]);
    run_git(&root, &["push", "--set-upstream", "origin", &branch]);
    root
}

fn git_output(root: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout)
        .expect("git output should be UTF-8")
        .trim()
        .to_string()
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
fn release_draft_preflight_accepts_release_notes_without_a_handoff() {
    let root = unique_temp_repo("draft-release-notes");
    fs::create_dir_all(&root).expect("temp repo should be created");
    install_version_reader(&root);
    write_executable(
        &root,
        "scripts/ci/check-release-draft-ready.sh",
        &fs::read_to_string(workspace_root().join("scripts/ci/check-release-draft-ready.sh"))
            .expect("release draft guard should be readable"),
    );
    write_executable(
        &root,
        "scripts/ci/read-workspace-version.sh",
        "#!/usr/bin/env bash\nprintf '%s\\n' '0.92.7'\n",
    );
    write_file(
        &root,
        "docs/changelog/0.92.md",
        "# Fixture changelog\n\n## [0.92.8]\n",
    );
    write_file(&root, "CHANGELOG.md", "## [0.92.8]\n");
    let output = Command::new("bash")
        .arg("scripts/ci/check-release-draft-ready.sh")
        .arg("patch")
        .current_dir(&root)
        .output()
        .expect("release draft guard should run");

    assert!(
        output.status.success(),
        "release notes alone should satisfy draft preflight\n{}",
        output_text(&output)
    );
    let _ = fs::remove_dir_all(root);
}

fn validation_receipt(version: &str, source: &str, gate: &str) -> String {
    format!(
        "{{\"schema\":1,\"version\":\"{version}\",\"source\":\"{source}\",\"date\":\"2026-08-27\",\"gate\":\"{gate}\"}}\n"
    )
}

#[test]
fn governed_bump_writes_structured_validation_receipt() {
    assert_governed_receipt(None, "complete", false);
}

#[test]
fn governed_bump_replaces_structured_validation_receipt() {
    assert_governed_receipt(Some("original receipt\n"), "fast", false);
}

#[test]
fn failed_bump_restores_receipt_presence_and_contents() {
    assert_governed_receipt(None, "complete", true);
    assert_governed_receipt(Some("original receipt\n"), "complete", true);
}

fn create_receipt_repo(previous_receipt: Option<&str>, handoff: &str) -> PathBuf {
    let root = unique_temp_repo("bump-receipt");
    fs::create_dir_all(&root).expect("temp repo should be created");
    write_file(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.92.7\"\n\n\
         [package]\nname = \"receipt-fixture\"\nversion.workspace = true\nedition = \"2024\"\n",
    );
    write_file(&root, "src/lib.rs", "");
    write_file(
        &root,
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"receipt-fixture\"\nversion = \"0.92.7\"\n",
    );
    for consumer in ["consumer", "embedded-consumer"] {
        let consumer_root = root.join("integrations/blob-service").join(consumer);
        write_file(
            &consumer_root,
            "Cargo.toml",
            &format!(
                "[workspace]\n[package]\nname = \"fixture-{consumer}\"\n\
                 version = \"0.1.0\"\nedition = \"2024\"\n\
                 [dependencies]\nreceipt-fixture = {{ path = \"../../..\" }}\n"
            ),
        );
        write_file(&consumer_root, "src/lib.rs", "");
        write_file(
            &consumer_root,
            "Cargo.lock",
            &format!(
                "version = 4\n\n[[package]]\nname = \"fixture-{consumer}\"\n\
                 version = \"0.1.0\"\ndependencies = [\"receipt-fixture\"]\n\n\
                 [[package]]\nname = \"receipt-fixture\"\nversion = \"0.92.7\"\n"
            ),
        );
    }
    write_file(
        &root,
        "docs/changelog/0.92.md",
        "# Fixture changelog\n\n## [0.92.8]\n",
    );
    write_file(&root, "CHANGELOG.md", "## [0.92.8]\n");
    write_file(&root, "docs/status/current.md", handoff);
    if let Some(receipt) = previous_receipt {
        write_file(&root, "release-validation.json", receipt);
    }
    write_file(
        &root,
        "scripts/dev/install_dev.sh",
        "CANIC_CLI_VERSION=\"${CANIC_CLI_VERSION:-0.92.7}\"\n",
    );
    install_version_reader(&root);
    write_executable(
        &root,
        "scripts/ci/check-release-draft-ready.sh",
        &fs::read_to_string(workspace_root().join("scripts/ci/check-release-draft-ready.sh"))
            .expect("release draft guard should be readable"),
    );
    write_executable(
        &root,
        "scripts/ci/check-release-remote-state.sh",
        "#!/usr/bin/env bash\nexit 0\n",
    );
    write_executable(
        &root,
        "scripts/ci/sync-release-surface-version.sh",
        "#!/usr/bin/env bash\nsed -i 's/0.92.7/0.92.8/' scripts/dev/install_dev.sh\n",
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
    "set-version --workspace --offline 0.92.8")
        sed -i 's/0.92.7/0.92.8/' Cargo.toml
        ;;
    *)
        exec "$CANIC_FIXTURE_CARGO" "$@"
        ;;
esac
"#,
    );
    write_executable(
        &root,
        "fake-bin/git",
        r#"#!/usr/bin/env bash
set -euo pipefail
case "$*" in
    'rev-parse --show-toplevel') pwd ;;
    'rev-parse HEAD') printf '%s\n' aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ;;
    'status --porcelain') ;;
    'ls-files -- Cargo.toml :(glob)**/Cargo.toml') printf 'Cargo.toml\n' ;;
    'rev-parse v0.92.8')
        [[ -f occupied-tag ]] || exit 1
        cp release-validation.json observed-validation.json
        ;;
    *) echo "unexpected fixture Git request: $*" >&2; exit 2 ;;
esac
"#,
    );
    root
}

fn assert_governed_receipt(previous_receipt: Option<&str>, gate: &str, fail_after_receipt: bool) {
    let handoff = "Current source remains descriptive.\n";
    let root = create_receipt_repo(previous_receipt, handoff);
    let original_lock = fs::read_to_string(root.join("Cargo.lock")).unwrap();
    let original_consumer_locks: Vec<_> = ["consumer", "embedded-consumer"]
        .into_iter()
        .map(|consumer| {
            let path = root
                .join("integrations/blob-service")
                .join(consumer)
                .join("Cargo.lock");
            let contents = fs::read_to_string(&path).unwrap();
            (path, contents)
        })
        .collect();
    let validated_head = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    if fail_after_receipt {
        write_file(&root, "occupied-tag", "retained exact tag\n");
    }
    let path = format!(
        "{}:{}",
        root.join("fake-bin").display(),
        env::var("PATH").unwrap_or_default()
    );

    let output = Command::new("bash")
        .arg(workspace_root().join("scripts/ci/bump-version.sh"))
        .arg("patch")
        .current_dir(&root)
        // Validation inherits the outer release identity; this workspace owns its own.
        .env("RELEASE_VERSION", "0.92.8")
        .env("RELEASE_DATE", "2026-08-29")
        .env("CANIC_RELEASE_DATE", "2026-08-29")
        .env("CANIC_RELEASE_VALIDATED", "1")
        .env("CANIC_RELEASE_VALIDATED_HEAD", validated_head)
        .env("CANIC_RELEASE_VALIDATION_KIND", gate)
        .env("CANIC_FIXTURE_CARGO", env!("CARGO"))
        .env("CARGO_NET_OFFLINE", "true")
        .env("PATH", path)
        .output()
        .expect("bump script should run");

    assert_eq!(
        output.status.success(),
        !fail_after_receipt,
        "{}",
        output_text(&output)
    );
    assert_eq!(
        fs::read_to_string(root.join("docs/status/current.md")).unwrap(),
        handoff
    );
    if fail_after_receipt {
        for (path, contents) in original_consumer_locks {
            assert_eq!(fs::read_to_string(path).unwrap(), contents);
        }
        let receipt = fs::read_to_string(root.join("release-validation.json")).ok();
        assert_eq!(receipt.as_deref(), previous_receipt);
        assert!(
            fs::read_to_string(root.join("Cargo.toml"))
                .unwrap()
                .contains("version = \"0.92.7\"")
        );
        assert_eq!(
            fs::read_to_string(root.join("Cargo.lock")).unwrap(),
            original_lock
        );
    } else {
        assert!(
            fs::read_to_string(root.join("Cargo.lock"))
                .unwrap()
                .contains("version = \"0.92.8\"")
        );
    }
    let receipt_path = if fail_after_receipt {
        root.join("observed-validation.json")
    } else {
        root.join("release-validation.json")
    };
    let parsed = Command::new("bash")
            .args([
                "-c",
                r#"source "$1"; require_jq || exit; exec "$JQ_BIN" -er '[.schema, .version, .source, .date, .gate] | @tsv' "$2""#,
                "receipt",
            ])
            .arg(workspace_root().join("scripts/ci/require-jq.sh"))
            .arg(receipt_path)
            .output()
            .expect("structured receipt should parse");
    assert!(parsed.status.success());
    assert_eq!(
        String::from_utf8(parsed.stdout).unwrap().trim(),
        format!("1\t0.92.8\t{validated_head}\t2026-08-29\t{gate}")
    );
    let _ = fs::remove_dir_all(root);
}

fn create_fast_patch_repo(name: &str) -> PathBuf {
    create_fast_patch_repo_with_gate(name, "complete")
}

fn create_fast_patch_repo_with_gate(name: &str, gate: &str) -> PathBuf {
    let root = unique_temp_repo(name);
    fs::create_dir_all(&root).expect("temp repo should be created");
    run_git(&root, &["init"]);
    write_file(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.92.7\"\n",
    );
    write_file(
        &root,
        "Cargo.lock",
        "[[package]]\nname = \"transitive\"\nversion = \"0.10.1\"\nchecksum = \"old\"\n",
    );
    install_version_reader(&root);
    install_fast_patch_guard(&root);
    commit_all(&root, "validated source");
    let source = git_output(&root, &["rev-parse", "HEAD"]);
    write_file(
        &root,
        "release-validation.json",
        &validation_receipt("0.92.7", &source, gate),
    );
    commit_all(&root, "Release 0.92.7");
    tag_release(&root, "0.92.7");
    root
}

fn run_fast_patch_eligibility(root: &Path) -> Output {
    Command::new("bash")
        .args([
            "scripts/ci/check-fast-patch-eligibility.sh",
            "--eligibility-only",
        ])
        .current_dir(root)
        .output()
        .expect("fast patch eligibility should run")
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

    assert!(
        !output.status.success(),
        "guard should reject a missing tag"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_push_guard_rejects_tag_on_another_commit() {
    let root = create_release_repo("push-wrong-tag-target");
    tag_release(&root, "0.92.8");
    create_release_commit(&root);

    let output = run_push_guard(&root);

    assert!(
        !output.status.success(),
        "guard should reject a tag that does not identify HEAD"
    );
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
    assert!(text.contains("v0.92.8"));
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
    write_file(
        &root,
        "CHANGELOG.md",
        "# Descriptive root changelog without a release-summary schema\n",
    );
    write_file(
        &root,
        "docs/changelog/0.92.md",
        "# Fixture changelog\n\n## 0.92.8 - Unreleased\n",
    );
    let status_document = "Development handoff.\n";
    write_file(&root, "docs/status/current.md", status_document);
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

    assert!(!output.status.success(), "the fixture sync must fail");
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
    assert_eq!(
        fs::read_to_string(root.join("docs/status/current.md")).unwrap(),
        status_document
    );
    let status = Command::new("git")
        .args(["status", "--short"])
        .current_dir(&root)
        .output()
        .expect("restored fixture status should resolve");
    assert!(status.status.success());
    assert!(
        status.stdout.is_empty(),
        "rollback must restore a clean repo"
    );
    let _ = fs::remove_dir_all(root);
}

fn install_failing_release_fixture_commands(root: &Path) {
    write_executable(
        root,
        "scripts/ci/check-release-draft-ready.sh",
        "#!/usr/bin/env bash\nexit 0\n",
    );
    write_executable(
        root,
        "scripts/ci/check-release-remote-state.sh",
        "#!/usr/bin/env bash\nexit 0\n",
    );
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
    locate-project\ *) exit 0 ;;
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
    let output = Command::new("bash")
        .arg(workspace_root().join("scripts/ci/test-release-candidate.sh"))
        .output()
        .expect("release content fixtures should execute");
    assert!(
        output.status.success(),
        "guard must accept only governed release mutations\n{}",
        output_text(&output)
    );
}

#[test]
fn fast_patch_eligibility_accepts_docs_and_rejects_runtime_source() {
    let root = create_fast_patch_repo("fast-eligibility");
    write_file(&root, "docs/note.md", "non-runtime correction\n");
    commit_all(&root, "documentation correction");

    let accepted = run_fast_patch_eligibility(&root);
    assert!(
        accepted.status.success(),
        "documentation-only patch should be eligible\n{}",
        output_text(&accepted)
    );

    write_file(&root, "src/lib.rs", "pub fn runtime_change() {}\n");
    commit_all(&root, "runtime change");
    let rejected = run_fast_patch_eligibility(&root);
    assert!(!rejected.status.success());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn fast_patch_eligibility_rejects_dependency_resolution_changes() {
    let output = Command::new("bash")
        .arg(workspace_root().join("scripts/ci/test-fast-patch-eligibility.sh"))
        .output()
        .expect("fast eligibility fixtures should run");
    assert!(output.status.success(), "{}", output_text(&output));
}

#[test]
fn fast_patch_eligibility_reuses_complete_receipt_through_a_fast_release() {
    let root = create_fast_patch_repo("fast-receipt-chain");
    write_file(&root, "docs/first.md", "first fast patch\n");
    commit_all(&root, "first fast source");
    let first_source = git_output(&root, &["rev-parse", "HEAD"]);
    write_file(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.92.8\"\n",
    );
    write_file(
        &root,
        "release-validation.json",
        &validation_receipt("0.92.8", &first_source, "fast"),
    );
    commit_all(&root, "Release 0.92.8");
    tag_release(&root, "0.92.8");
    write_file(&root, "docs/second.md", "second fast patch\n");
    commit_all(&root, "second fast source");

    let accepted = run_fast_patch_eligibility(&root);
    assert!(
        accepted.status.success(),
        "fast release should retain its complete ancestor basis\n{}",
        output_text(&accepted)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn make_release_targets_are_sequential_and_push_is_guarded() {
    let output = Command::new("bash")
        .arg(workspace_root().join("scripts/ci/test-release-recipes.sh"))
        .output()
        .expect("release recipe fixtures should run");
    assert!(output.status.success(), "{}", output_text(&output));
}

#[test]
fn fast_patch_requires_a_complete_validation_ancestor() {
    let root = create_fast_patch_repo_with_gate("fast-without-complete", "fast");
    write_file(&root, "docs/note.md", "documentation correction\n");
    commit_all(&root, "documentation correction");
    assert!(!run_fast_patch_eligibility(&root).status.success());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_cadence_reports_structured_counts_without_enforcing_the_advisory() {
    let root = unique_temp_repo("cadence");
    let report = fs::read_to_string(workspace_root().join("scripts/dev/report-release-cadence.sh"))
        .expect("cadence report source");
    write_executable(&root, "scripts/dev/report-release-cadence.sh", &report);
    write_executable(
        &root,
        "bin/git",
        "#!/bin/sh\nprintf '%s\\n' \"$CANIC_TEST_TAGS\"\n",
    );
    let path = env::join_paths(
        std::iter::once(root.join("bin"))
            .chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
    )
    .expect("fixture command path");
    for count in [0, 11, 12] {
        let mut tags = (0..count)
            .map(|patch| format!("v0.92.{patch}"))
            .collect::<Vec<_>>();
        tags.extend(["v0.93.0".to_string(), "v0.92.1-preview".to_string()]);
        let output = Command::new("bash")
            .args(["scripts/dev/report-release-cadence.sh", "--tsv", "0.92.12"])
            .current_dir(&root)
            .env("PATH", &path)
            .env("CANIC_TEST_TAGS", tags.join("\n"))
            .output()
            .expect("cadence report runs");
        assert!(output.status.success(), "{}", output_text(&output));
        assert_eq!(
            String::from_utf8(output.stdout).expect("TSV record"),
            format!("0.92\t{count}\t12\t{}\n", count + 1)
        );
    }
    let invalid = Command::new("bash")
        .args(["scripts/dev/report-release-cadence.sh", "--tsv", "invalid"])
        .current_dir(&root)
        .output()
        .expect("invalid cadence invocation runs");
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    fs::remove_dir_all(root).expect("remove cadence fixture");
}

#[test]
fn structured_validation_receipt_rejects_invalid_records() {
    let root = create_fast_patch_repo("receipt-schema");
    let source = git_output(&root, &["rev-parse", "HEAD"]);
    let valid = validation_receipt("0.92.7", &source, "complete");
    for invalid in [
        "not json".to_string(),
        "{}".to_string(),
        valid.replace("\"schema\":1", "\"schema\":2"),
        valid.replace("0.92.7", "0.92.6"),
        valid.replace(&source, "invalid-source"),
        valid.replace("2026-08-27", "not-a-date"),
        valid.replace("complete", "unknown"),
        valid.replace("\"schema\":1", "\"extra\":true,\"schema\":1"),
        format!("{valid}{valid}"),
    ] {
        write_file(&root, "release-validation.json", &invalid);
        commit_all(&root, "invalid receipt fixture");
        let output = Command::new("bash")
            .args(["scripts/ci/read-release-validation.sh", "HEAD", "0.92.7"])
            .current_dir(&root)
            .output()
            .expect("receipt reader should run");
        assert!(
            !output.status.success(),
            "invalid record was admitted: {invalid}"
        );
    }
    fs::remove_file(root.join("release-validation.json")).unwrap();
    commit_all(&root, "missing receipt fixture");
    let output = Command::new("bash")
        .args(["scripts/ci/read-release-validation.sh", "HEAD", "0.92.7"])
        .current_dir(&root)
        .output()
        .expect("receipt reader should run");
    assert!(!output.status.success());
    let _ = fs::remove_dir_all(root);
}
