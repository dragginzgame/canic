//! Module: fleet_install_plan::authority::tests
//!
//! Responsibility: verify deterministic workspace-source identity at Git worktree boundaries.
//! Does not own: release builds, deployment planning, or external effects.
//! Boundary: exercises the authority owner's private source-snapshot compiler.

use super::*;
use crate::test_support::temp_dir;

struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    fn new(prefix: &str) -> Self {
        let root = temp_dir(prefix);
        fs::create_dir_all(root.join("crates/demo/src")).expect("create test workspace");
        fs::write(root.join("Cargo.toml"), b"[workspace]\n").expect("write workspace manifest");
        run_git(&root, &["init"]);
        Self { root }
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("remove test workspace");
    }
}

#[test]
fn workspace_snapshot_omits_tracked_worktree_deletions() {
    let workspace = TestWorkspace::new("canic-workspace-source-deletion");
    let source = workspace.root.join("crates/demo/src/lib.rs");
    fs::write(&source, b"pub fn retained() {}\n").expect("write tracked source");
    run_git(&workspace.root, &["add", "."]);

    fs::remove_file(&source).expect("delete tracked source from worktree");
    let deleted_digest = workspace_source_snapshot_sha256(&workspace.root)
        .expect("tracked worktree deletion is absent from snapshot");
    assert_eq!(
        git_stdout(&workspace.root, &["ls-files", "crates/demo/src/lib.rs"]),
        "crates/demo/src/lib.rs\n",
        "deleted path must remain tracked in the Git index"
    );

    fs::write(&source, b"pub fn retained() {}\n").expect("restore tracked source");
    let restored_digest =
        workspace_source_snapshot_sha256(&workspace.root).expect("hash restored tracked source");
    assert_ne!(deleted_digest, restored_digest);

    fs::remove_file(&source).expect("delete tracked source again");
    assert_eq!(
        workspace_source_snapshot_sha256(&workspace.root)
            .expect("repeat tracked worktree deletion is absent from snapshot"),
        deleted_digest
    );
}

#[test]
fn workspace_snapshot_hashes_modifications_and_accepted_untracked_files() {
    let workspace = TestWorkspace::new("canic-workspace-source-current-bytes");
    let source = workspace.root.join("crates/demo/src/lib.rs");
    fs::write(&source, b"pub fn value() -> u8 { 1 }\n").expect("write tracked source");
    run_git(&workspace.root, &["add", "."]);
    let tracked_digest =
        workspace_source_snapshot_sha256(&workspace.root).expect("hash tracked source");

    fs::write(&source, b"pub fn value() -> u8 { 2 }\n").expect("modify tracked source");
    let modified_digest =
        workspace_source_snapshot_sha256(&workspace.root).expect("hash modified source");
    assert_ne!(tracked_digest, modified_digest);

    fs::write(
        workspace.root.join("crates/demo/src/untracked.rs"),
        b"pub const UNTRACKED: bool = true;\n",
    )
    .expect("write accepted untracked source");
    let untracked_digest =
        workspace_source_snapshot_sha256(&workspace.root).expect("hash untracked source");
    assert_ne!(modified_digest, untracked_digest);

    fs::write(
        workspace.root.join(".gitignore"),
        b"/crates/demo/src/ignored.rs\n",
    )
    .expect("write ignore rule");
    fs::write(
        workspace.root.join("crates/demo/src/ignored.rs"),
        b"pub const IGNORED: bool = true;\n",
    )
    .expect("write ignored source");
    assert_eq!(
        workspace_source_snapshot_sha256(&workspace.root).expect("hash with ignored source"),
        untracked_digest
    );
}

fn run_git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("run Git command");
    assert!(
        output.status.success(),
        "Git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_stdout(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("run Git command");
    assert!(output.status.success(), "Git command must succeed");
    String::from_utf8(output.stdout).expect("Git output is UTF-8")
}
