use std::{
    path::{Path, PathBuf},
    process::Command,
};

use canic_core::cdk::utils::hash::sha256_hex;

use super::model::{DIRTY_SUMMARY_ALGORITHM, SourceDirtyPolicyV1, SourceProvenanceV1, SourceVcsV1};

pub(super) fn source_provenance(workspace_root: &Path) -> SourceProvenanceV1 {
    if !is_git_worktree_root(workspace_root) {
        return unknown_source_provenance();
    }

    let Some(revision) = git_output_text(workspace_root, ["rev-parse", "HEAD"]) else {
        return unknown_source_provenance();
    };
    let branch = git_output_text(workspace_root, ["rev-parse", "--abbrev-ref", "HEAD"]);
    let Some(status) = git_status_bytes(workspace_root) else {
        return SourceProvenanceV1 {
            schema_version: 1,
            vcs: SourceVcsV1::Git,
            revision: Some(revision),
            branch,
            dirty: None,
            dirty_policy: SourceDirtyPolicyV1::Unknown,
            dirty_summary_digest: None,
            dirty_summary_algorithm: None,
        };
    };

    let dirty = !status.is_empty();
    SourceProvenanceV1 {
        schema_version: 1,
        vcs: SourceVcsV1::Git,
        revision: Some(revision),
        branch,
        dirty: Some(dirty),
        dirty_policy: if dirty {
            SourceDirtyPolicyV1::DirtyRecorded
        } else {
            SourceDirtyPolicyV1::Clean
        },
        dirty_summary_digest: dirty.then(|| sha256_hex(&status)),
        dirty_summary_algorithm: dirty.then(|| DIRTY_SUMMARY_ALGORITHM.to_string()),
    }
}

fn is_git_worktree_root(workspace_root: &Path) -> bool {
    let Some(top_level) = git_output_text(workspace_root, ["rev-parse", "--show-toplevel"]) else {
        return false;
    };
    let Ok(top_level) = PathBuf::from(top_level).canonicalize() else {
        return false;
    };
    let Ok(workspace_root) = workspace_root.canonicalize() else {
        return false;
    };

    top_level == workspace_root
}

const fn unknown_source_provenance() -> SourceProvenanceV1 {
    SourceProvenanceV1 {
        schema_version: 1,
        vcs: SourceVcsV1::Unknown,
        revision: None,
        branch: None,
        dirty: None,
        dirty_policy: SourceDirtyPolicyV1::Unknown,
        dirty_summary_digest: None,
        dirty_summary_algorithm: None,
    }
}

fn git_output_text<const N: usize>(workspace_root: &Path, args: [&str; N]) -> Option<String> {
    String::from_utf8(git_output_bytes(workspace_root, args)?)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_status_bytes(workspace_root: &Path) -> Option<Vec<u8>> {
    git_output_bytes(
        workspace_root,
        [
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ],
    )
}

fn git_command(workspace_root: &Path) -> Command {
    let mut command = crate::build_environment::command("git");
    command.current_dir(workspace_root);
    clear_git_environment(&mut command);
    command.args(["-c", "core.fsmonitor=false"]);
    command
}

fn git_output_bytes<const N: usize>(workspace_root: &Path, args: [&str; N]) -> Option<Vec<u8>> {
    let output = git_command(workspace_root).args(args).output().ok()?;
    output.status.success().then_some(output.stdout)
}

fn clear_git_environment(command: &mut Command) {
    for key in [
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CEILING_DIRECTORIES",
        "GIT_COMMON_DIR",
        "GIT_CONFIG",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_GLOBAL",
        "GIT_CONFIG_NOSYSTEM",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_SYSTEM",
        "GIT_DIR",
        "GIT_DISCOVERY_ACROSS_FILESYSTEM",
        "GIT_INDEX_FILE",
        "GIT_NAMESPACE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_PREFIX",
        "GIT_WORK_TREE",
    ] {
        command.env_remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::fs;

    #[test]
    fn dirty_status_includes_untracked_source_despite_repository_configuration() {
        let root = temp_dir("canic-provenance-untracked-source");
        fs::create_dir_all(&root).unwrap();
        assert!(
            git_command(&root)
                .args(["init", "--quiet"])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            git_command(&root)
                .args(["config", "status.showUntrackedFiles", "no"])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            git_command(&root)
                .args(["config", "core.excludesFile", ""])
                .status()
                .unwrap()
                .success()
        );
        fs::write(
            root.join("canic_untracked_fixture.rs"),
            "pub fn untracked() {}\n",
        )
        .unwrap();
        assert_eq!(
            git_output_bytes(&root, ["status", "--porcelain=v1", "-z"]),
            Some(Vec::new()),
        );
        assert_eq!(
            git_status_bytes(&root),
            Some(b"?? canic_untracked_fixture.rs\0".to_vec()),
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_queries_remove_injected_git_configuration() {
        let root = temp_dir("canic-provenance-git-config");
        fs::create_dir_all(&root).unwrap();
        assert!(
            git_command(&root)
                .args(["init", "--quiet"])
                .status()
                .unwrap()
                .success()
        );
        let mut command = git_command(&root);
        command
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.bare")
            .env("GIT_CONFIG_VALUE_0", "true")
            .env("GIT_CONFIG_PARAMETERS", "'core.bare=true'");
        clear_git_environment(&mut command);
        let output = command
            .args(["rev-parse", "--is-bare-repository"])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"false\n");
        fs::remove_dir_all(root).unwrap();
    }
}
