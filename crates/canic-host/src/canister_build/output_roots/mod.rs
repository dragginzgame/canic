//! Module: canister_build::output_roots
//!
//! Responsibility: describe resolved Cargo output roots and observable workspace sharing.
//! Does not own: target selection, locking, cache identity, or build admission.
//! Boundary: best-effort filesystem observations produce advisory build diagnostics.

use crate::canister_build::cache::{canister_build_target_root, declaration_target_root};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Selected output path and independently observed physical workspace ownership.
struct OutputRootObservation {
    selected: PathBuf,
    resolved: Option<PathBuf>,
    shared_workspace: Option<PathBuf>,
}

impl OutputRootObservation {
    fn observe(workspace: &Path, selected: PathBuf) -> Self {
        let resolved = ic_host_fs::path::canonicalize_allow_missing(&selected, workspace).ok();
        let workspace = fs::canonicalize(workspace).ok();
        let shared_workspace = resolved.as_ref().and_then(|output| {
            let workspace = workspace.as_ref()?;
            output
                .ancestors()
                .find(|ancestor| {
                    *ancestor != workspace.as_path()
                        && ancestor.join("Cargo.toml").is_file()
                        && output.starts_with(ancestor.join("target"))
                })
                .map(Path::to_path_buf)
        });
        Self {
            selected,
            resolved,
            shared_workspace,
        }
    }

    fn lines(&self, label: &str) -> Vec<String> {
        let mut lines = vec![match &self.resolved {
            Some(resolved) => format!(
                "{label}: {} (resolved: {})",
                self.selected.display(),
                resolved.display()
            ),
            None => format!(
                "{label}: {} (physical path unavailable)",
                self.selected.display()
            ),
        }];
        if let Some(workspace) = &self.shared_workspace {
            lines.push(format!(
                "Warning: {label} points into another workspace's mutable target output ({}). Independent Canic workspace locks do not protect shared output. Use a dedicated target directory for each checkout and share sccache instead.",
                workspace.display()
            ));
        }
        lines
    }
}

pub fn diagnostic_lines(workspace: &Path) -> Vec<String> {
    [
        ("Cargo output", canister_build_target_root(workspace)),
        ("Declaration output", declaration_target_root(workspace)),
    ]
    .into_iter()
    .flat_map(|(label, selected)| OutputRootObservation::observe(workspace, selected).lines(label))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    #[cfg(unix)]
    fn isolated_checkout_target_alias_identifies_the_other_workspace() {
        let root = temp_dir("shared-output-diagnostic");
        let editable = root.join("editable");
        let frozen = root.join("frozen");
        fs::create_dir_all(editable.join("target")).unwrap();
        fs::create_dir_all(&frozen).unwrap();
        for workspace in [&editable, &frozen] {
            fs::write(workspace.join("Cargo.toml"), "[workspace]\n").unwrap();
        }
        std::os::unix::fs::symlink(editable.join("target"), frozen.join("target")).unwrap();
        for suffix in ["target/canic-wasm", "target/canic-wasm/declarations"] {
            let output = OutputRootObservation::observe(&frozen, frozen.join(suffix));
            assert_eq!(
                output.shared_workspace,
                Some(editable.canonicalize().unwrap())
            );
            assert_eq!(
                output.resolved,
                Some(editable.canonicalize().unwrap().join(suffix))
            );
            assert!(
                !frozen.join(suffix).exists(),
                "diagnostics must not create output"
            );
        }
        let direct = OutputRootObservation::observe(&frozen, editable.join("target/custom"));
        assert_eq!(
            direct.shared_workspace,
            Some(editable.canonicalize().unwrap())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn independent_and_explicit_external_output_roots_remain_valid() {
        let root = temp_dir("independent-output-diagnostic");
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("Cargo.toml"), "[workspace]\n").unwrap();
        for target in [
            workspace.join("target/canic-wasm"),
            workspace.join("custom"),
            root.join("dedicated-output"),
        ] {
            let output = OutputRootObservation::observe(&workspace, target.clone());
            assert_eq!(output.selected, target);
            assert!(output.resolved.is_some());
            assert!(output.shared_workspace.is_none());
            assert!(!target.exists());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
