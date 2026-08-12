use super::build_snapshot::InstallBuildTarget;
use super::output::{TerminalActivity, TerminalStyle};
use crate::canister_build::{
    CurrentCanisterArtifactBuildOutput, WorkspaceBuildContext,
    build_workspace_canister_artifacts_from_specs, workspace_build_context_once,
};
use crate::format::wasm_size_label;
use crate::should_export_candid_artifacts;
use crate::table::{ColumnAlign, render_bordered_table};
use std::{collections::BTreeSet, fs, path::Path, time::Instant};

pub(super) fn run_canic_build_targets(
    context: &WorkspaceBuildContext,
    targets: &[InstallBuildTarget],
) -> Result<Vec<CurrentCanisterArtifactBuildOutput>, Box<dyn std::error::Error>> {
    if context.release_build_id.is_none() {
        return Err("complete install build is missing its durable release-build identity".into());
    }
    if workspace_build_context_once(context)? {
        for line in context.lines() {
            println!("{line}");
        }
        println!("config: {}", context.config_path.display());
        println!("artifacts: {}", context.artifact_root().display());
        println!();
    }

    fs::create_dir_all(context.artifact_root())?;
    let style = TerminalStyle::detected();
    style.print_section(
        "Build application Wasm",
        &format!("{} configured canisters", targets.len()),
    );
    let headers = ["CANISTER", "STATUS", "WASM"];
    let alignments = [ColumnAlign::Left, ColumnAlign::Left, ColumnAlign::Right];

    let cargo_workspace_count = targets
        .iter()
        .map(|target| &target.spec.cargo_workspace_root)
        .collect::<BTreeSet<_>>()
        .len();
    let cargo_pass_count = cargo_workspace_count
        * if should_export_candid_artifacts(context.build_network)
            && context.profile != crate::canister_build::CanisterBuildProfile::Debug
        {
            2
        } else {
            1
        };
    let started_at = Instant::now();
    let activity = TerminalActivity::start(format!(
        "{} | {} across {}",
        counted_label(targets.len(), "canister", "canisters"),
        counted_label(cargo_pass_count, "Cargo pass", "Cargo passes"),
        counted_label(cargo_workspace_count, "workspace", "workspaces")
    ));
    let specs = targets
        .iter()
        .map(|target| target.spec.clone())
        .collect::<Vec<_>>();
    let build = build_workspace_canister_artifacts_from_specs(context, &specs);
    activity.finish();
    let built_outputs = build.map_err(|err| format!("configured artifact build failed: {err}"))?;
    if built_outputs.len() != targets.len() {
        return Err("configured artifact batch returned an incomplete output set".into());
    }
    let elapsed = started_at.elapsed();

    let mut outputs = Vec::with_capacity(targets.len());
    let mut rows = Vec::with_capacity(targets.len());
    for (target, output) in targets.iter().zip(built_outputs) {
        let artifact_size = wasm_artifact_size(&output.wasm_path, &output.wasm_gz_path)?;

        rows.push([target.role.clone(), style.success("done"), artifact_size]);
        outputs.push(CurrentCanisterArtifactBuildOutput {
            role: target.role.clone(),
            output,
        });
    }

    println!("{}", render_bordered_table(&headers, &rows, &alignments));
    style.print_section(
        "Application Wasm ready",
        &format!(
            "{} in {:.2}s via {}",
            counted_label(targets.len(), "canister", "canisters"),
            elapsed.as_secs_f64(),
            counted_label(cargo_pass_count, "Cargo pass", "Cargo passes")
        ),
    );
    println!();
    Ok(outputs)
}

pub(super) fn wasm_artifact_size(
    wasm_path: &Path,
    wasm_gz_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let wasm_bytes = Some(fs::metadata(wasm_path)?.len());
    let gzip_bytes = fs::metadata(wasm_gz_path)
        .ok()
        .map(|metadata| metadata.len());
    Ok(wasm_size_label(wasm_bytes, gzip_bytes))
}

pub(super) fn progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 || width == 0 {
        return "[] 0/0".to_string();
    }

    let filled = current.saturating_mul(width).div_ceil(total);
    let filled = filled.min(width);
    format!(
        "[{}{}] {current}/{total}",
        "#".repeat(filled),
        ".".repeat(width - filled)
    )
}

fn counted_label(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_matches_current_canister_ordinal() {
        assert_eq!(progress_bar(1, 2, 12), "[######......] 1/2");
        assert_eq!(progress_bar(2, 2, 12), "[############] 2/2");
    }
}
