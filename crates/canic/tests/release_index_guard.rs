use std::{
    fs,
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
    std::env::temp_dir().join(format!("canic-{name}-{}-{nanos}", std::process::id()))
}

fn run_git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git should run");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
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

fn create_repo(name: &str) -> PathBuf {
    let root = unique_temp_repo(name);
    fs::create_dir_all(&root).expect("temp repo should be created");
    run_git(&root, &["init"]);
    write_file(&root, "Cargo.toml", "[workspace]\n");
    write_file(&root, "Cargo.lock", "# lock\n");
    write_file(&root, "README.md", "readme\n");
    write_file(&root, "crates/canic-host/README.md", "host readme\n");
    write_file(&root, "scripts/dev/install_dev.sh", "#!/usr/bin/env bash\n");
    write_file(
        &root,
        "scripts/ci/sync-release-surface-version.sh",
        "#!/usr/bin/env bash\n",
    );
    write_file(&root, "src/lib.rs", "pub fn marker() {}\n");
    run_git(&root, &["add", "."]);
    run_git(
        &root,
        &[
            "-c",
            "user.name=Canic Test",
            "-c",
            "user.email=canic@example.invalid",
            "commit",
            "-m",
            "initial",
        ],
    );
    root
}

fn run_guard(root: &Path) -> Output {
    let script = workspace_root().join("scripts/ci/check-release-index.sh");
    Command::new(script)
        .current_dir(root)
        .output()
        .expect("release index guard should run")
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn release_index_guard_accepts_complete_release_files() {
    let root = create_repo("release-index-ok");
    write_file(&root, "Cargo.toml", "[workspace]\n# bumped\n");
    write_file(
        &root,
        "scripts/dev/install_dev.sh",
        "#!/usr/bin/env bash\n# bumped\n",
    );
    run_git(&root, &["add", "Cargo.toml", "scripts/dev/install_dev.sh"]);

    let output = run_guard(&root);

    assert!(
        output.status.success(),
        "guard should accept release-only index\n{}",
        output_text(&output)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_index_guard_rejects_empty_index() {
    let root = create_repo("release-index-empty");

    let output = run_guard(&root);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "guard should reject an empty index"
    );
    assert!(text.contains("No staged release files"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_index_guard_rejects_staged_deletions() {
    let root = create_repo("release-index-deletion");
    write_file(&root, "Cargo.toml", "[workspace]\n# bumped\n");
    run_git(&root, &["add", "Cargo.toml"]);
    run_git(&root, &["rm", "README.md"]);

    let output = run_guard(&root);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "guard should reject staged deletions"
    );
    assert!(text.contains("Release commit index contains staged deletions"));
    assert!(text.contains("README.md"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_index_guard_rejects_staged_non_release_files() {
    let root = create_repo("release-index-non-release");
    write_file(
        &root,
        "src/lib.rs",
        "pub fn marker() {}\npub fn changed() {}\n",
    );
    run_git(&root, &["add", "src/lib.rs"]);

    let output = run_guard(&root);
    let text = output_text(&output);

    assert!(!output.status.success(), "guard should reject source files");
    assert!(text.contains("Release commit index contains non-release files"));
    assert!(text.contains("src/lib.rs"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn release_index_guard_rejects_partially_staged_release_files() {
    let root = create_repo("release-index-partial");
    write_file(&root, "Cargo.toml", "[workspace]\n# staged\n");
    run_git(&root, &["add", "Cargo.toml"]);
    write_file(&root, "Cargo.toml", "[workspace]\n# staged\n# unstaged\n");

    let output = run_guard(&root);
    let text = output_text(&output);

    assert!(
        !output.status.success(),
        "guard should reject partially staged release files"
    );
    assert!(text.contains("Release files are staged with additional unstaged changes"));
    assert!(text.contains("Cargo.toml"));
    let _ = fs::remove_dir_all(root);
}
