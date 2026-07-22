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
    let Some(status) = git_output_bytes(workspace_root, ["status", "--porcelain=v1", "-z"]) else {
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

fn git_output_bytes<const N: usize>(workspace_root: &Path, args: [&str; N]) -> Option<Vec<u8>> {
    let mut command = Command::new("git");
    command.current_dir(workspace_root);
    clear_git_environment(&mut command);

    let output = command.args(args).output().ok()?;
    output.status.success().then_some(output.stdout)
}

fn clear_git_environment(command: &mut Command) {
    for key in [
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CEILING_DIRECTORIES",
        "GIT_COMMON_DIR",
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
