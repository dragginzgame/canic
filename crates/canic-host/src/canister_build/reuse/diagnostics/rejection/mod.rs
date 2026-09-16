//! Module: canister_build::reuse::diagnostics::rejection
//!
//! Responsibility: retain bounded evidence for a rejected post-build input comparison.
//! Does not own: source admission, reusable artifacts, environment values or source contents.
//! Boundary: diagnostic persistence never replaces the original validation result.

use crate::{
    canister_build::{
        WorkspaceBuildContext,
        cache::{canister_build_target_root, declaration_target_root},
        reuse::{BuildReuseError, snapshot::BuildInputSnapshot},
    },
    durable_io::write_bytes,
};
use canic_core::ids::ReleaseBuildId;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

const LIMIT: usize = 256 * 1024;

/// Invocation paths observed before compilation, kept separately from cache authority.
#[derive(Serialize)]
pub(in crate::canister_build::reuse) struct InputLocations {
    workspace_root: PathBuf,
    config_path: PathBuf,
    output_roots: BTreeMap<&'static str, OutputRoot>,
}

#[derive(Serialize)]
struct OutputRoot {
    selected: PathBuf,
    resolved: Option<PathBuf>,
}

impl InputLocations {
    pub(in crate::canister_build::reuse) fn capture(context: &WorkspaceBuildContext) -> Self {
        let mut roots = BTreeMap::from([
            (
                "runtime",
                canister_build_target_root(&context.workspace_root),
            ),
            (
                "declarations",
                declaration_target_root(&context.workspace_root),
            ),
        ]);
        if let Some(parent) = context.config_path.parent() {
            roots.insert("generated", parent.join(".canic/generated"));
        }
        if let Some(path) = env::var_os("CARGO_BUILD_BUILD_DIR").map(PathBuf::from) {
            roots.insert(
                "intermediate",
                if path.is_absolute() {
                    path
                } else {
                    context.workspace_root.join(path)
                },
            );
        }
        Self {
            workspace_root: context.workspace_root.clone(),
            config_path: context.config_path.clone(),
            output_roots: roots
                .into_iter()
                .map(|(kind, selected)| {
                    let resolved = selected.canonicalize().ok();
                    (kind, OutputRoot { selected, resolved })
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum RejectionKind {
    AuthorityChanged,
    InputChanged,
    UnobservedInput,
}

#[derive(Serialize)]
struct RejectedInput<'a> {
    path: &'a Path,
    // Null means the path is absent from that inventory, not that the file is absent.
    before_snapshot_value: Option<&'a str>,
    after_snapshot_value: Option<&'a str>,
}

#[derive(Serialize)]
struct RejectionEvidence<'a> {
    schema_version: u8,
    release_build_id: ReleaseBuildId,
    kind: RejectionKind,
    before_inputs_sha256: String,
    after_inputs_sha256: String,
    before_input_count: usize,
    after_input_count: usize,
    before_locations: &'a InputLocations,
    after_locations: InputLocations,
    affected_input: Option<RejectedInput<'a>>,
}

pub(in crate::canister_build::reuse) fn retain_rejection(
    context: &WorkspaceBuildContext,
    release_build_id: ReleaseBuildId,
    before_locations: &InputLocations,
    before: &BuildInputSnapshot,
    after: &BuildInputSnapshot,
    error: &BuildReuseError,
) -> Option<PathBuf> {
    let (kind, path) = match error {
        BuildReuseError::Changed => (RejectionKind::AuthorityChanged, None),
        BuildReuseError::ChangedInput(path) => (RejectionKind::InputChanged, Some(path)),
        BuildReuseError::UnobservedInput(path) => (RejectionKind::UnobservedInput, Some(path)),
        _ => return None,
    };
    let evidence = RejectionEvidence {
        schema_version: 1,
        release_build_id,
        kind,
        before_inputs_sha256: before.digest(),
        after_inputs_sha256: after.digest(),
        before_input_count: before.files.len(),
        after_input_count: after.files.len(),
        before_locations,
        after_locations: InputLocations::capture(context),
        affected_input: path.map(|path| RejectedInput {
            path,
            before_snapshot_value: path
                .to_str()
                .and_then(|key| before.files.get(key))
                .map(String::as_str),
            after_snapshot_value: path
                .to_str()
                .and_then(|key| after.files.get(key))
                .map(String::as_str),
        }),
    };
    let bytes = serde_json::to_vec(&evidence).ok()?;
    if bytes.len() > LIMIT {
        return None;
    }
    let path = context
        .icp_root
        .join(".canic/build-reuse")
        .join(format!("rejected-{release_build_id}.json"));
    write_bytes(&path, &bytes).ok()?;
    Some(path)
}
