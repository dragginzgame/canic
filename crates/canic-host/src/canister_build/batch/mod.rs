//! Canic supplies build environment and target policy to IC Build's Cargo grouping.
use crate::{canister_build::WorkspaceBuildContext, cargo_command};
use ic_host_process::tool::{OutputLimit, OutputLimits};
use std::path::Path;

pub(super) fn compatible_package_groups(
    context: &WorkspaceBuildContext,
    workspace: &Path,
    packages: &[String],
) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    // Preserve the previous inspector's full output capture and no elapsed deadline.
    ic_build::cargo::compatible_package_groups(
        workspace,
        packages,
        "wasm32-unknown-unknown",
        &ic_build::cargo::selection::CargoSelection {
            locked: true,
            ..Default::default()
        },
        OutputLimits {
            stdout: OutputLimit::Terminate(usize::MAX),
            stderr: OutputLimit::Terminate(usize::MAX),
            timeout: None,
        },
        || {
            let mut command = cargo_command();
            context.apply_to_command(&mut command);
            command
        },
    )
}
