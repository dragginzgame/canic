//! Canic supplies build environment and target policy to IC Build's Cargo grouping.
use crate::{canister_build::WorkspaceBuildContext, cargo_command};
use ic_host_process::tool::{OutputLimit, OutputLimits, capture_command};
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
            // Grouping serves the default-feature, locked configured-role build.
            locked: true,
            ..Default::default()
        },
        |spec| {
            let mut command = cargo_command();
            context.apply_to_command(&mut command);
            let output = capture_command(
                &mut spec.command(command)?,
                OutputLimits {
                    stdout: OutputLimit::Terminate(usize::MAX),
                    stderr: OutputLimit::Terminate(usize::MAX),
                    timeout: None,
                },
            )?
            .require_success()?
            .require_complete()?;
            Ok(String::from_utf8(output.stdout)?)
        },
    )
}
