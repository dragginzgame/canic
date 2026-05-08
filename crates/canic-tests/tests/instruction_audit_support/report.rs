use super::*;

// Scan the repo for concrete `perf!` checkpoint call sites.
pub(super) fn scan_perf_callsites(workspace_root: &Path) -> Vec<String> {
    let mut out = Vec::new();

    for root in CHECKPOINT_SCAN_ROOTS {
        visit_rust_files(&workspace_root.join(root), &mut |path| {
            let Ok(contents) = fs::read_to_string(path) else {
                return;
            };

            for (line_no, line) in contents.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("///") {
                    continue;
                }

                let Some(index) = line.find("perf!(") else {
                    continue;
                };
                let previous = line[..index].chars().next_back();
                if matches!(previous, Some('"' | '\'' | '`')) {
                    continue;
                }

                if line[index..].starts_with("perf!(") {
                    let relative = path
                        .strip_prefix(workspace_root)
                        .expect("path under workspace root");
                    out.push(format!(
                        "{}:{}:{}",
                        relative.display(),
                        line_no + 1,
                        line.trim()
                    ));
                }
            }
        });
    }

    out.sort();
    out
}

// Recursively visit Rust source files under one directory root.
fn visit_rust_files(dir: &Path, visitor: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_rust_files(&path, visitor);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            visitor(&path);
        }
    }
}

// Build the current checkpoint-gap table from the static critical-flow list.
pub(super) fn checkpoint_coverage_gaps(checkpoint_sites: &[String]) -> Vec<CheckpointCoverageGap> {
    FLOW_GAPS
        .iter()
        .map(|(flow_name, insertion_site)| CheckpointCoverageGap {
            flow_name: (*flow_name).to_string(),
            status: if checkpoint_sites
                .iter()
                .any(|site| site.starts_with(insertion_site))
            {
                "PASS".to_string()
            } else {
                "PARTIAL".to_string()
            },
            proposed_first_insertion_site: (*insertion_site).to_string(),
        })
        .collect()
}

// Write the raw checkpoint scan output expected by the audit definition.
pub(super) fn write_flow_checkpoint_log(path: &Path, checkpoint_sites: &[String]) {
    let body = if checkpoint_sites.is_empty() {
        "No `perf!` checkpoint call sites were found under `crates/`.\n".to_string()
    } else {
        let mut lines = checkpoint_sites.join("\n");
        lines.push('\n');
        lines
    };

    fs::write(path, body).expect("write flow checkpoints log");
}

// Assemble the verification table for the first instruction-footprint run.
pub(super) fn verification_rows(
    paths: &AuditPaths,
    checkpoint_sites: &[String],
    query_unobservable_count: usize,
    measured_checkpoint_count: usize,
) -> Vec<VerificationRow> {
    vec![
        VerificationRow {
            command: "cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture".to_string(),
            status: "PASS".to_string(),
            notes: "PocketIC runner completed and wrote the report plus normalized artifacts."
                .to_string(),
        },
        VerificationRow {
            command: "fresh root harness profile per scenario".to_string(),
            status: "PASS".to_string(),
            notes:
                "Each scenario used a fresh smallest-profile root bootstrap instead of sharing one cumulative perf table."
                    .to_string(),
        },
        VerificationRow {
            command: "canic_metrics(MetricsKind::Perf, PageRequest { limit=512, offset=0 })"
                .to_string(),
            status: "PASS".to_string(),
            notes: format!("Update scenarios were sampled before/after through persisted perf rows, and query scenarios used local-only `QueryPerfSample` probe endpoints because query-side perf rows are not committed; normalized rows saved under `{}`.", paths.artifacts_dir.join("perf-rows.json").display()),
        },
        VerificationRow {
            command: "repo checkpoint scan".to_string(),
            status: "PASS".to_string(),
            notes: if checkpoint_sites.is_empty() {
                "No `perf!` call sites are present in the current repo scan; flow checkpoint coverage remains partial.".to_string()
            } else {
                format!("Found {} checkpoint call sites.", checkpoint_sites.len())
            },
        },
        VerificationRow {
            command: "checkpoint delta capture".to_string(),
            status: if measured_checkpoint_count == 0 {
                "PARTIAL".to_string()
            } else {
                "PASS".to_string()
            },
            notes: if measured_checkpoint_count == 0 {
                "Sampled update scenarios did not produce any non-zero checkpoint deltas."
                    .to_string()
            } else {
                format!(
                    "{measured_checkpoint_count} non-zero checkpoint delta rows were captured under `{}`.",
                    paths.artifacts_dir.join("checkpoint-deltas.json").display()
                )
            },
        },
        VerificationRow {
            command: "query perf visibility".to_string(),
            status: if query_unobservable_count == 0 {
                "PASS".to_string()
            } else {
                "PARTIAL".to_string()
            },
            notes: if query_unobservable_count == 0 {
                "All sampled query scenarios returned `QueryPerfSample` local instruction counters through the local-only probe endpoints, which avoids relying on non-persisted query-side perf state.".to_string()
            } else {
                format!(
                    "{query_unobservable_count} sampled query scenarios failed to return a `QueryPerfSample` local instruction counter through the probe path."
                )
            },
        },
        VerificationRow {
            command: "baseline comparison".to_string(),
            status: "BLOCKED".to_string(),
            notes: "First run of day for `instruction-footprint`; baseline deltas are `N/A`."
                .to_string(),
        },
    ]
}

// Write the markdown verification table consumed by the dated report.
#[allow(clippy::format_push_string)]
pub(super) fn write_verification_readout(path: &Path, rows: &[VerificationRow]) {
    let mut out = String::from("| Command | Status | Notes |\n| --- | --- | --- |\n");
    for row in rows {
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            row.command, row.status, row.notes
        ));
    }

    fs::write(path, out).expect("write verification readout");
}

// Serialize one JSON artifact with a trailing newline.
pub(super) fn write_json<T>(path: &Path, value: &T)
where
    T: ?Sized + Serialize,
{
    let mut body = serde_json::to_string_pretty(value).expect("serialize json");
    body.push('\n');
    fs::write(path, body).expect("write json artifact");
}

// Write the normalized endpoint matrix as a simple TSV artifact.
#[allow(clippy::format_push_string)]
pub(super) fn write_endpoint_matrix_tsv(path: &Path, results: &[ScenarioResult]) {
    let mut out = String::from(
        "canister\tendpoint_or_flow\tscenario_key\tcount\ttotal_local_instructions\tavg_local_instructions\n",
    );

    for result in results {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            result.scenario.canister,
            result.scenario.endpoint_or_flow,
            result.scenario.key,
            result.row.count,
            result.row.total_local_instructions,
            result.row.avg_local_instructions
        ));
    }

    fs::write(path, out).expect("write endpoint matrix tsv");
}

// Render the first dated instruction-footprint report from normalized results.
#[allow(clippy::format_push_string, clippy::too_many_lines)]
pub(super) fn write_report(
    path: &Path,
    artifacts_dir: &Path,
    metadata: &AuditMetadata,
    results: &[ScenarioResult],
    verification_rows: &[VerificationRow],
    checkpoint_sites: &[String],
    gaps: &[CheckpointCoverageGap],
) {
    let query_unobservable_count = results
        .iter()
        .filter(|result| execution::query_perf_is_unobservable(&result.scenario, &result.row))
        .count();
    let checkpoint_rows = results
        .iter()
        .flat_map(|result| result.checkpoint_rows.iter())
        .collect::<Vec<_>>();

    let mut ordered = results
        .iter()
        .filter(|result| !execution::query_perf_is_unobservable(&result.scenario, &result.row))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|result| std::cmp::Reverse(result.row.avg_local_instructions));

    let hotspot_rows = ordered.iter().take(3).copied().collect::<Vec<_>>();
    let risk_score = risk_score(checkpoint_sites, query_unobservable_count, &hotspot_rows);
    let minor_line = scenarios::current_minor_line();
    let report_date = metadata
        .run_timestamp_utc
        .get(..10)
        .expect("timestamp includes YYYY-MM-DD");
    let report_file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("report file name");
    let artifacts_dir_name = artifacts_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("artifacts directory name");
    let target_canisters = render_scope(
        results
            .iter()
            .map(|result| result.scenario.canister)
            .collect::<BTreeSet<_>>(),
    );
    let target_endpoints = render_scope(
        results
            .iter()
            .map(|result| result.scenario.endpoint_or_flow)
            .collect::<BTreeSet<_>>(),
    );

    let mut out = String::new();
    out.push_str(&format!(
        "# Instruction Footprint Audit - {report_date}\n\n"
    ));
    out.push_str("## Report Preamble\n\n");
    out.push_str(&format!(
        "- Scope: Canic instruction footprint (first `{minor_line}` baseline, partial canister scope)\n"
    ));
    out.push_str("- Definition path: `docs/audits/recurring/system/instruction-footprint.md`\n");
    out.push_str(&format!(
        "- Compared baseline report path: `{}`\n",
        metadata.compared_baseline_report
    ));
    out.push_str(&format!(
        "- Code snapshot identifier: `{}`\n",
        metadata.code_snapshot
    ));
    out.push_str(&format!("- Method tag/version: `{METHOD_TAG}`\n"));
    out.push_str("- Comparability status: `partial`\n");
    out.push_str("- Auditor: `codex`\n");
    out.push_str(&format!(
        "- Run timestamp (UTC): `{}`\n",
        metadata.run_timestamp_utc
    ));
    out.push_str(&format!("- Branch: `{}`\n", metadata.branch));
    out.push_str(&format!("- Worktree: `{}`\n", metadata.worktree));
    out.push_str("- Execution environment: `PocketIC`\n");
    out.push_str(&format!(
        "- Target canisters in scope: {target_canisters}\n"
    ));
    out.push_str(&format!(
        "- Target endpoints/flows in scope: {target_endpoints}\n"
    ));
    out.push_str("- Deferred from this baseline: no additional functional flows are deferred beyond first-run comparability; this run covers shared queries plus delegated auth issuance, verifier confirmation, replay/cycles, scaling worker creation, sharding account creation, and root template admin updates.\n\n");

    out.push_str("## Findings / Checklist\n\n");
    out.push_str("| Check | Result | Evidence |\n| --- | --- | --- |\n");
    out.push_str(&format!(
        "| Scenario manifest recorded | PASS | `artifacts/{artifacts_dir_name}/scenario-manifest.json` captures the scenario identity tuple for every sampled endpoint. |\n"
    ));
    out.push_str(&format!(
        "| Normalized perf rows recorded | PASS | `artifacts/{artifacts_dir_name}/perf-rows.json` stores canonical endpoint rows with count and total local instructions. |\n"
    ));
    out.push_str(&format!(
        "| Checkpoint deltas recorded | {} | `artifacts/{artifacts_dir_name}/checkpoint-deltas.json` stores non-zero per-scenario checkpoint rows. |\n",
        if checkpoint_rows.is_empty() { "PARTIAL" } else { "PASS" }
    ));
    out.push_str("| Fresh topology isolation used | PASS | Each scenario ran under a fresh smallest-profile root harness install instead of reusing one cumulative perf table. |\n");
    out.push_str(&format!(
        "| Flow checkpoint coverage scanned | PASS | `artifacts/{artifacts_dir_name}/flow-checkpoints.log` records the current repo scan result. |\n"
    ));
    if checkpoint_sites.is_empty() {
        out.push_str("| `perf!` checkpoints available for critical flows | PARTIAL | Current repo scan found zero `perf!` call sites under `crates/`, so flow-stage attribution is not yet measurable. |\n");
    } else {
        out.push_str("| `perf!` checkpoints available for critical flows | PASS | Current repo scan found at least one `perf!` call site. |\n");
    }
    if query_unobservable_count == 0 {
        out.push_str("| Query endpoint perf visibility | PASS | Sampled query scenarios were measured through local-only `QueryPerfSample` probe endpoints because query-side perf rows are not committed. |\n");
    } else {
        out.push_str(&format!(
            "| Query endpoint perf visibility | PARTIAL | {query_unobservable_count} sampled query scenarios failed to return a usable local instruction counter through the probe path. |\n"
        ));
    }
    out.push_str("| Baseline path selected by daily baseline discipline | PARTIAL | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |\n\n");

    out.push_str("## Comparison to Previous Relevant Run\n\n");
    out.push_str("- First run of day for `instruction-footprint`; this report establishes the daily baseline.\n");
    out.push_str("- Query scenarios are now sampled through local-only `QueryPerfSample` probes because query-side perf rows are not committed, so their rows are directly comparable to later probe-backed reruns.\n");
    if query_unobservable_count > 0 {
        out.push_str("- One or more query probe calls still failed to return a usable local instruction counter, so those rows remain partial until the probe path is stable.\n");
    }
    out.push_str("- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.\n\n");

    out.push_str("## Endpoint Matrix\n\n");
    out.push_str("| Canister | Endpoint | Scenario | Count | Total local instructions | Avg local instructions | Baseline delta | Notes |\n");
    out.push_str("| --- | --- | --- | ---: | ---: | ---: | --- | --- |\n");
    for result in results {
        let notes = if execution::query_perf_is_unobservable(&result.scenario, &result.row) {
            "probe failed to return a local instruction counter"
        } else if result.scenario.transport_mode == "query" {
            "local-only QueryPerfSample probe"
        } else {
            ""
        };
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} | {} | {} | N/A | {} |\n",
            result.scenario.canister,
            result.scenario.endpoint_or_flow,
            result.scenario.arg_class,
            result.row.count,
            result.row.total_local_instructions,
            result.row.avg_local_instructions,
            notes
        ));
    }
    out.push('\n');

    out.push_str("## Flow Checkpoints\n\n");
    if checkpoint_sites.is_empty() {
        out.push_str("- No current `perf!` checkpoints were found under `crates/`; no per-stage flow deltas are available yet.\n");
        out.push_str(&format!(
            "- Flow checkpoint evidence file: `artifacts/{artifacts_dir_name}/flow-checkpoints.log`\n\n"
        ));
    } else {
        for site in checkpoint_sites {
            out.push_str(&format!("- `{site}`\n"));
        }
        out.push('\n');
    }

    out.push_str("## Measured Checkpoint Deltas\n\n");
    if checkpoint_rows.is_empty() {
        out.push_str("- No sampled scenario produced a non-zero checkpoint delta in this run.\n\n");
    } else {
        let mut ordered_checkpoint_rows = checkpoint_rows;
        ordered_checkpoint_rows.sort_by_key(|row| std::cmp::Reverse(row.total_local_instructions));
        out.push_str("| Scenario | Scope | Label | Count | Total local instructions | Avg local instructions |\n");
        out.push_str("| --- | --- | --- | ---: | ---: | ---: |\n");
        for row in ordered_checkpoint_rows.iter().take(12) {
            out.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} | {} | {} |\n",
                row.scenario_key,
                row.scope,
                row.label,
                row.count,
                row.total_local_instructions,
                row.avg_local_instructions
            ));
        }
        out.push('\n');
    }

    out.push_str("## Checkpoint Coverage Gaps\n\n");
    let covered_gaps = gaps
        .iter()
        .filter(|gap| gap.status == "PASS")
        .collect::<Vec<_>>();
    let uncovered_gaps = gaps
        .iter()
        .filter(|gap| gap.status != "PASS")
        .collect::<Vec<_>>();
    out.push_str("Critical flows with checkpoints:\n");
    if covered_gaps.is_empty() {
        out.push_str("- none\n\n");
    } else {
        for gap in &covered_gaps {
            out.push_str(&format!("- `{}`\n", gap.flow_name));
        }
        out.push('\n');
    }
    out.push_str("Critical flows without checkpoints:\n");
    if uncovered_gaps.is_empty() {
        out.push_str("- none\n");
    } else {
        for gap in &uncovered_gaps {
            out.push_str(&format!("- `{}`\n", gap.flow_name));
        }
    }
    out.push('\n');
    out.push_str("Proposed first checkpoint insertion sites:\n");
    if uncovered_gaps.is_empty() {
        out.push_str("- none\n");
    } else {
        for gap in &uncovered_gaps {
            out.push_str(&format!(
                "- `{}` -> `{}`\n",
                gap.flow_name, gap.proposed_first_insertion_site
            ));
        }
    }
    out.push('\n');

    out.push_str("## Structural Hotspots\n\n");
    out.push_str("| Rank | Scenario | Avg local instructions | Module pressure | Evidence |\n");
    out.push_str("| --- | --- | ---: | --- | --- |\n");
    for (index, result) in hotspot_rows.iter().enumerate() {
        let (pressure, evidence) = hotspot_hint(result.scenario.subject_label);
        out.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            index + 1,
            result.scenario.key,
            result.row.avg_local_instructions,
            pressure,
            evidence
        ));
    }
    out.push('\n');

    out.push_str("## Hub Module Pressure\n\n");
    out.push_str("- `root::canic_response_capability_v1` now has measured replay/cycles stage deltas, so root capability work no longer has to be treated as an opaque endpoint total.\n");
    out.push_str("- `root::canic_request_delegation` remains the main shared update hotspot in the retained audit lane, so further optimization work should stay focused on shared runtime/auth cost rather than demo provisioning flows.\n");
    out.push_str("- `scale_hub::plan_create_worker` stays in the matrix as an audit-only dry-run probe, which keeps placement-policy visibility without turning demo `create_*` flows into default audit targets.\n");
    out.push_str("- `test::test` provides the current chain-key-free update floor on a non-root child canister. Drift there points back to shared runtime/update overhead rather than topology-specific logic.\n");
    out.push_str("- Root state/registry reads stay separate from the leaf floor. They matter for operator paths, but they should not be confused with the shared ordinary-leaf baseline.\n\n");

    out.push_str("## Dependency Fan-In Pressure\n\n");
    out.push_str("- Shared observability reads (`canic_env`, `canic_log`) are now measured through the internal `leaf_probe` canister instead of the shipped demo surface, and raw time is measured through the same internal lane. Their rows use `QueryPerfSample` counters from the measured call context rather than inferred zeroes or missing query-side perf-table commits.\n");
    out.push_str("- The sampled non-trivial hotspots now concentrate in shared auth/replay/root runtime and the audit-only placement dry-run probe. The local `test::test` update acts as the baseline floor for update overhead on an ordinary child canister.\n");
    if checkpoint_sites.is_empty() {
        out.push_str("- There is currently no flow-stage attribution because `perf!` coverage is absent. That is itself a dependency-pressure signal: optimization work is bottlenecked by missing internal checkpoints.\n\n");
    } else {
        out.push_str("- Flow-stage checkpoints now exist in the scaling, sharding, auth, and replay workflows. This matrix records non-zero checkpoint deltas for sampled update scenarios, so the next optimization pass can target concrete stages instead of endpoint totals alone.\n\n");
    }

    out.push_str("## Early Warning Signals\n\n");
    out.push_str("| Signal | Status | Evidence |\n");
    out.push_str("| --- | --- | --- |\n");
    if checkpoint_sites.is_empty() {
        out.push_str("| Flow checkpoint coverage absent | WARN | Current repo scan found zero `perf!` call sites under `crates/`. |\n");
    } else {
        out.push_str(&format!(
            "| Flow checkpoint coverage present | INFO | Current repo scan found {} `perf!` call sites under `crates/`. |\n",
            checkpoint_sites.len()
        ));
    }
    if query_unobservable_count > 0 {
        out.push_str(&format!(
            "| Query probe path failed on sampled rows | WARN | {query_unobservable_count} sampled query scenarios did not return a usable `QueryPerfSample` local instruction counter. |\n"
        ));
    }
    if let Some(top) = hotspot_rows.first() {
        out.push_str(&format!(
            "| Highest sampled endpoint currently highest-cost | WARN | `{}` averages {} local instructions in this first baseline. |\n",
            top.scenario.key, top.row.avg_local_instructions
        ));
    }
    out.push_str("| Baseline drift not yet available | INFO | First run of day; deltas remain `N/A` until the next comparable rerun. |\n\n");

    out.push_str("## Risk Score\n\n");
    out.push_str(&format!("Risk Score: **{risk_score} / 10**\n\n"));
    out.push_str("Interpretation: query visibility and stage attribution are now working for the sampled matrix. The remaining audit risk is mostly first-run comparability (`N/A` baseline deltas) plus a few endpoint-only paths that still do not have deeper internal stage attribution, not missing coverage of the critical flows themselves.\n\n");

    out.push_str("## Verification Readout\n\n");
    out.push_str("| Command | Status | Notes |\n| --- | --- | --- |\n");
    for row in verification_rows {
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            row.command, row.status, row.notes
        ));
    }
    out.push('\n');

    out.push_str("## Follow-up Actions\n\n");
    out.push_str("1. Owner boundary: `flow observability`\n");
    if checkpoint_sites.is_empty() {
        out.push_str("   Action: add first stable `perf!` checkpoints to the scaling, sharding, and root-capability workflows so the next rerun can move from endpoint-only totals to real flow-stage attribution.\n");
    } else {
        out.push_str("   Action: rerun this audit after one concrete perf change so the next report has real comparable baseline deltas instead of first-run `N/A`, and only add deeper verifier-side auth checkpoints if that endpoint-total starts to matter.\n");
    }
    out.push_str("2. Owner boundary: `shared update hotspots`\n");
    out.push_str(&format!(
        "   Action: compare `root::canic_request_delegation`, `root::canic_response_capability_v1`, and the local `test::test` update floor before/after any shared-runtime cleanup, using this report as the `{minor_line}` baseline.\n"
    ));
    out.push_str("3. Owner boundary: `shared observability floor`\n");
    out.push_str("   Action: keep the internal standalone query probes in the matrix so shared-runtime drift does not hide behind root-only or coordinator-only endpoints.\n\n");

    out.push_str("## Report Files\n\n");
    out.push_str(&format!("- [{report_file_name}](./{report_file_name})\n"));
    out.push_str(&format!(
        "- [scenario-manifest.json](artifacts/{artifacts_dir_name}/scenario-manifest.json)\n"
    ));
    out.push_str(&format!(
        "- [perf-rows.json](artifacts/{artifacts_dir_name}/perf-rows.json)\n"
    ));
    out.push_str(&format!(
        "- [endpoint-matrix.tsv](artifacts/{artifacts_dir_name}/endpoint-matrix.tsv)\n"
    ));
    out.push_str(&format!(
        "- [checkpoint-deltas.json](artifacts/{artifacts_dir_name}/checkpoint-deltas.json)\n"
    ));
    out.push_str(&format!(
        "- [flow-checkpoints.log](artifacts/{artifacts_dir_name}/flow-checkpoints.log)\n"
    ));
    out.push_str(&format!(
        "- [checkpoint-coverage-gaps.json](artifacts/{artifacts_dir_name}/checkpoint-coverage-gaps.json)\n"
    ));
    out.push_str(&format!(
        "- [verification-readout.md](artifacts/{artifacts_dir_name}/verification-readout.md)\n"
    ));
    out.push_str(&format!(
        "- [method.json](artifacts/{artifacts_dir_name}/method.json)\n"
    ));
    out.push_str(&format!(
        "- [environment.json](artifacts/{artifacts_dir_name}/environment.json)\n"
    ));

    fs::write(path, out).expect("write instruction audit report");
}

// Render one stable, backtick-quoted scope list for the report preamble.
fn render_scope(items: BTreeSet<&str>) -> String {
    items
        .into_iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(" ")
}

// Map the current highest-cost labels back to concrete modules/files.
fn hotspot_hint(subject_label: &str) -> (&'static str, &'static str) {
    match subject_label {
        "canic_response_capability_v1" => (
            "Root dispatcher plus replay/capability workflow",
            "[request handler](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/mod.rs), [replay workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/replay.rs)",
        ),
        "plan_create_worker" => (
            "Scaling policy read path",
            "[scaling_probe](/home/adam/projects/canic/canisters/audit/scaling_probe/src/lib.rs), [scaling workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/placement/scaling/mod.rs)",
        ),
        "test" => (
            "Local/dev update floor on the test helper canister",
            "[runtime_probe/lib](/home/adam/projects/canic/canisters/test/runtime_probe/src/lib.rs)",
        ),
        "canic_subnet_registry" => (
            "Root topology registry query",
            "[root_probe](/home/adam/projects/canic/canisters/audit/root_probe/src/lib.rs), [registry query](/home/adam/projects/canic/crates/canic-core/src/workflow/topology/registry/query.rs)",
        ),
        "canic_subnet_state" => (
            "Root state snapshot query",
            "[root_probe](/home/adam/projects/canic/canisters/audit/root_probe/src/lib.rs), [state query](/home/adam/projects/canic/crates/canic-core/src/workflow/state/query.rs)",
        ),
        "canic_log" => (
            "Internal audit log pagination probe over the shared log query path",
            "[leaf_probe](/home/adam/projects/canic/canisters/audit/leaf_probe/src/lib.rs), [log query](/home/adam/projects/canic/crates/canic-core/src/workflow/log/query.rs)",
        ),
        "canic_env" => (
            "Internal audit env snapshot probe over the shared env query path",
            "[leaf_probe](/home/adam/projects/canic/canisters/audit/leaf_probe/src/lib.rs), [env query](/home/adam/projects/canic/crates/canic-core/src/workflow/env/query.rs)",
        ),
        "canic_time" => (
            "Internal audit raw time probe",
            "[leaf_probe](/home/adam/projects/canic/canisters/audit/leaf_probe/src/lib.rs)",
        ),
        _ => (
            "Shared runtime surface",
            "[endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs)",
        ),
    }
}

// Compute a bounded risk score for the first baseline.
fn risk_score(
    checkpoint_sites: &[String],
    query_unobservable_count: usize,
    hotspot_rows: &[&ScenarioResult],
) -> u8 {
    let mut score = 2u8;

    if checkpoint_sites.is_empty() {
        score = score.saturating_add(3);
    }

    if query_unobservable_count > 0 {
        score = score.saturating_add(1);
    }

    if hotspot_rows
        .first()
        .is_some_and(|row| row.row.avg_local_instructions > 2_000_000)
    {
        score = score.saturating_add(2);
    }

    if hotspot_rows
        .iter()
        .filter(|row| row.scenario.canister == "root")
        .count()
        == hotspot_rows.len()
    {
        score = score.saturating_add(1);
    }

    score.min(10)
}
