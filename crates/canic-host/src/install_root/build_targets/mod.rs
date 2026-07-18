use super::build_snapshot::InstallBuildTarget;
use crate::canister_build::{
    CurrentCanisterArtifactBuildOutput, WorkspaceBuildContext,
    build_workspace_canister_artifact_from_spec, workspace_build_context_once,
};
use crate::format::wasm_size_label;
use crate::release_set::artifact_root_path;
use crate::table::{ColumnAlign, render_separator, render_table_row, table_widths};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

pub(super) fn run_canic_build_targets(
    context: &WorkspaceBuildContext,
    targets: &[InstallBuildTarget],
) -> Result<Vec<CurrentCanisterArtifactBuildOutput>, Box<dyn std::error::Error>> {
    if workspace_build_context_once(context)? {
        for line in context.lines() {
            println!("{line}");
        }
        println!("config: {}", context.config_path.display());
        println!(
            "artifacts: {}",
            planned_build_artifact_root(&context.icp_root).display()
        );
        println!();
    }

    fs::create_dir_all(planned_build_artifact_root(&context.icp_root))?;
    println!("Building {} canisters", targets.len());
    println!();
    let headers = ["CANISTER", "PROGRESS", "WASM", "ELAPSED"];
    let planned_rows = targets
        .iter()
        .map(|target| {
            [
                target.role.clone(),
                progress_bar(targets.len(), targets.len(), 10),
                "000.00 MiB (gz 000.00 MiB)".to_string(),
                "0.00s".to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let alignments = [
        ColumnAlign::Left,
        ColumnAlign::Left,
        ColumnAlign::Right,
        ColumnAlign::Right,
    ];
    let widths = table_widths(&headers, &planned_rows);
    println!("{}", render_table_row(&headers, &widths, &alignments));
    println!("{}", render_separator(&widths));

    let mut outputs = Vec::with_capacity(targets.len());
    for (index, target) in targets.iter().enumerate() {
        let started_at = Instant::now();
        let target_context = context.with_role(&target.role);
        let output = build_workspace_canister_artifact_from_spec(&target_context, &target.spec)
            .map_err(|err| format!("artifact build failed for {}: {err}", target.role))?;
        let elapsed = started_at.elapsed();
        let artifact_size = wasm_artifact_size(&output.wasm_path, &output.wasm_gz_path)?;

        let row = [
            target.role.clone(),
            progress_bar(index + 1, targets.len(), 10),
            artifact_size,
            format!("{:.2}s", elapsed.as_secs_f64()),
        ];
        println!("{}", render_table_row(&row, &widths, &alignments));
        outputs.push(CurrentCanisterArtifactBuildOutput {
            role: target.role.clone(),
            output,
        });
    }

    println!();
    Ok(outputs)
}

pub(super) fn planned_build_artifact_root(icp_root: &Path) -> PathBuf {
    artifact_root_path(icp_root, "local")
}

fn wasm_artifact_size(
    wasm_path: &Path,
    wasm_gz_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let wasm_bytes = Some(fs::metadata(wasm_path)?.len());
    let gzip_bytes = fs::metadata(wasm_gz_path)
        .ok()
        .map(|metadata| metadata.len());
    Ok(wasm_size_label(wasm_bytes, gzip_bytes))
}

fn progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 || width == 0 {
        return "[] 0/0".to_string();
    }

    let filled = current.saturating_mul(width).div_ceil(total);
    let filled = filled.min(width);
    format!(
        "[{}{}] {current}/{total}",
        "#".repeat(filled),
        " ".repeat(width - filled)
    )
}
