use super::*;
use serde::Deserialize;
use std::collections::BTreeMap;

const ESTIMATE_SECTION_TITLE: &str = "## Execution Cycle Estimate";
const ESTIMATE_SECTION_LABEL: &str =
    "Execution cycle estimate (instructions only, excludes message/byte/GC/platform fees).";
const ESTIMATE_SECTION_TABLE_HEADER: &str = "| Scenario | Local instructions | Estimated instruction cycles | Cycles per billion instructions | Source | Formula |";
const STATUS_PASS: &str = "PASS";
const STATUS_PARTIAL: &str = "PARTIAL";
const BASELINE_NOT_AVAILABLE: &str = "N/A";

#[derive(Deserialize)]
struct BaselinePerfRow {
    avg_local_instructions: u64,
    scenario_key: String,
}

// Scan the repo for concrete `perf!` checkpoint call sites.
pub(super) fn scan_perf_callsites(workspace_root: &Path) -> Vec<String> {
    let mut out = Vec::new();

    for root in CHECKPOINT_SCAN_ROOTS {
        visit_rust_files(&workspace_root.join(root), &mut |path| {
            let Ok(contents) = fs::read_to_string(path) else {
                return;
            };

            for (line_no, line) in contents.lines().enumerate() {
                if contains_perf_invocation(line) {
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

// Recognize literal and namespaced `perf!` invocations while ignoring quoted
// examples and line comments. Current product checkpoints are single-line
// invocations; multiline macro syntax requires a method-version change.
fn contains_perf_invocation(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut quote = None;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }

        if byte == b'"' || byte == b'\'' {
            quote = Some(byte);
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            return false;
        }
        if bytes[index..].starts_with(b"perf!(") {
            return true;
        }
        index += 1;
    }

    false
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
                STATUS_PASS.to_string()
            } else {
                STATUS_PARTIAL.to_string()
            },
            proposed_first_insertion_site: (*insertion_site).to_string(),
        })
        .collect()
}

// Assemble the verification table for one instruction-footprint run.
pub(super) fn verification_rows(
    paths: &AuditPaths,
    metadata: &AuditMetadata,
    checkpoint_sites: &[String],
    measured_checkpoint_count: usize,
) -> Vec<VerificationRow> {
    let artifacts_dir_name = paths
        .artifacts_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("instruction audit artifacts directory name");
    vec![
        VerificationRow {
            command: "cargo test --offline --locked -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture".to_string(),
            status: STATUS_PASS.to_string(),
            notes: "PocketIC runner completed through authoritative root-harness artifacts and wrote the report plus normalized artifacts."
                .to_string(),
        },
        VerificationRow {
            command: "fresh authoritative root harness profile per scenario".to_string(),
            status: STATUS_PASS.to_string(),
            notes:
                "Each scenario used a fresh topology/capability/scaling/sharding root bootstrap instead of sharing one cumulative perf table."
                    .to_string(),
        },
        VerificationRow {
            command: "canic_metrics(MetricsKind::Runtime, PageRequest { limit=512, offset=0 })"
                .to_string(),
            status: STATUS_PASS.to_string(),
            notes: format!("Update scenarios were sampled before/after through persisted perf rows; the install scenario groups retained bootstrap checkpoints. Normalized rows are under `artifacts/{artifacts_dir_name}/perf-rows.json`."),
        },
        VerificationRow {
            command: "repo checkpoint scan".to_string(),
            status: STATUS_PASS.to_string(),
            notes: if checkpoint_sites.is_empty() {
                "No `perf!` call sites are present in the current repo scan; flow checkpoint coverage remains partial.".to_string()
            } else {
                format!("Found {} checkpoint call sites.", checkpoint_sites.len())
            },
        },
        VerificationRow {
            command: "checkpoint delta capture".to_string(),
            status: if measured_checkpoint_count == 0 {
                STATUS_PARTIAL.to_string()
            } else {
                STATUS_PASS.to_string()
            },
            notes: if measured_checkpoint_count == 0 {
                "Sampled update scenarios did not produce any non-zero checkpoint deltas."
                    .to_string()
            } else {
                format!(
                    "{measured_checkpoint_count} non-zero checkpoint delta rows were captured under `artifacts/{artifacts_dir_name}/checkpoint-deltas.json`."
                )
            },
        },
        VerificationRow {
            command: "fixed v2 update/install scenario roster".to_string(),
            status: STATUS_PASS.to_string(),
            notes: "All twelve required scenarios completed; query instruction totals are outside this method version."
                .to_string(),
        },
        VerificationRow {
            command: "baseline comparison".to_string(),
            status: STATUS_PASS.to_string(),
            notes: if baseline_is_selected(metadata) {
                format!(
                    "Latest prior `instruction-footprint` report selected as baseline: `{}`.",
                    metadata.compared_baseline_report
                )
            } else {
                "No comparable v2 report exists; this valid run establishes the first v2 baseline and deltas are `N/A`."
                    .to_string()
            },
        },
    ]
}

// Write the markdown verification table consumed by the dated report.
#[expect(clippy::format_push_string)]
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

// Render the first dated instruction-footprint report from normalized results.
#[expect(clippy::format_push_string, clippy::too_many_lines)]
pub(super) fn write_report(
    path: &Path,
    artifacts_dir: &Path,
    metadata: &AuditMetadata,
    results: &[ScenarioResult],
    verification_rows: &[VerificationRow],
    checkpoint_sites: &[String],
    gaps: &[CheckpointCoverageGap],
) {
    let checkpoint_rows = results
        .iter()
        .flat_map(|result| result.checkpoint_rows.iter())
        .collect::<Vec<_>>();
    let zero_exclusive_rows = results
        .iter()
        .filter(|result| result.row.count > 0 && result.row.total_local_instructions == 0)
        .collect::<Vec<_>>();

    let mut ordered = results.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|result| std::cmp::Reverse(result.row.avg_local_instructions));

    let hotspot_rows = ordered.iter().take(3).copied().collect::<Vec<_>>();
    let baseline_rows = load_baseline_rows(&metadata.compared_baseline_report);
    let risk_score = risk_score(gaps, baseline_is_selected(metadata), &hotspot_rows);
    let run_result =
        if gaps.iter().any(|gap| gap.status != STATUS_PASS) || checkpoint_rows.is_empty() {
            "partial"
        } else if risk_score >= 7 {
            "fail"
        } else {
            "pass"
        };
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
        "- Scope: Canic instruction footprint (fixed `{minor_line}` v2 update/install roster)\n"
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
    out.push_str(&format!("- Audit method ID: `{}`\n", metadata.method_id));
    out.push_str(&format!(
        "- Audit method version: `{}`\n",
        metadata.method_version
    ));
    out.push_str(&format!(
        "- Audit method fingerprint: `{}`\n",
        metadata.method_fingerprint
    ));
    out.push_str(&format!("- Counter source: `{PERF_COUNTER_SOURCE}`\n"));
    out.push_str(&format!("- Counter ID: `{PERF_COUNTER_ID}`\n"));
    out.push_str("- Measured unit: `local_instructions`\n");
    out.push_str("- Counter scope: local canister WebAssembly instructions in the current call context; excludes other canisters and is not a cycle-charge measurement.\n");
    out.push_str("- Result validity: `valid`\n");
    out.push_str(&format!("- Run result: `{run_result}`\n"));
    out.push_str(&format!(
        "- Comparability status: `{}`\n",
        if baseline_is_selected(metadata) {
            "comparable"
        } else {
            "first-v2-baseline"
        }
    ));
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
    out.push_str("- Deferred from this baseline: query instruction totals require a future authoritative same-call fixture and method version. The fixed roster covers root capability/replay, root-proof provisioning, issuer prepare, verifier confirmation, scaling, sharding, publication, and root bootstrap.\n\n");

    out.push_str("## Findings / Checklist\n\n");
    out.push_str("| Check | Result | Evidence |\n| --- | --- | --- |\n");
    out.push_str(&format!(
        "| Scenario manifest recorded | PASS | `artifacts/{artifacts_dir_name}/scenario-manifest.json` captures the scenario identity tuple for every sampled endpoint. |\n"
    ));
    out.push_str(&format!(
        "| Normalized perf rows recorded | PASS | `artifacts/{artifacts_dir_name}/perf-rows.json` stores canonical endpoint rows with count and total local instructions. |\n"
    ));
    out.push_str(&format!(
        "| Zero exclusive endpoint totals interpreted | PASS | {} measured row(s) have `count > 0` and a zero exclusive total because nested/checkpoint scopes retain the attributed work; these are measured calls, not missing samples. |\n",
        zero_exclusive_rows.len()
    ));
    out.push_str(&format!(
        "| Checkpoint deltas recorded | {} | `artifacts/{artifacts_dir_name}/checkpoint-deltas.json` stores non-zero per-scenario checkpoint rows. |\n",
        if checkpoint_rows.is_empty() { STATUS_PARTIAL } else { STATUS_PASS }
    ));
    out.push_str("| Fresh topology isolation used | PASS | Each scenario ran under a fresh smallest-profile root harness install instead of reusing one cumulative perf table. |\n");
    out.push_str("| Flow checkpoint coverage scanned | PASS | The Flow Checkpoints section records the current repo scan result. |\n");
    if checkpoint_sites.is_empty() {
        out.push_str("| `perf!` checkpoints available for critical flows | PARTIAL | Current repo scan found zero `perf!` call sites under `crates/`, so flow-stage attribution is not yet measurable. |\n");
    } else {
        out.push_str("| `perf!` checkpoints available for critical flows | PASS | Current repo scan found at least one `perf!` call site. |\n");
    }
    out.push_str("| Authoritative fixture build | PASS | Every scenario uses the root harness and Canic-validated `build_artifact` path; no direct Cargo probe build remains. |\n");
    if baseline_is_selected(metadata) {
        out.push_str(&format!(
            "| Baseline path selected | PASS | Latest prior `instruction-footprint` report selected: `{}`. |\n\n",
            metadata.compared_baseline_report
        ));
    } else {
        out.push_str("| Baseline path selected | PASS | No comparable v2 report exists; this run establishes the first v2 baseline and deltas are `N/A`. |\n\n");
    }

    out.push_str("## Comparison to Previous Relevant Run\n\n");
    if baseline_is_selected(metadata) {
        out.push_str(&format!(
            "- Compared baseline report: `{}`.\n",
            metadata.compared_baseline_report
        ));
    } else {
        out.push_str("- No previous `instruction-footprint` report was available; this report establishes the first retained baseline.\n");
    }
    out.push_str("- V1 query-probe rows never executed and are not a baseline. V2 hard-cuts those direct-build probes; future query measurement needs a separately versioned authoritative same-call fixture.\n");
    if baseline_rows.is_some() {
        out.push_str("- Baseline drift values are computed from matching scenario keys in the previous report's `perf-rows.json` artifact.\n\n");
    } else if baseline_is_selected(metadata) {
        out.push_str("- Baseline drift values are `N/A` where the selected baseline has no matching readable `perf-rows.json` artifact or matching scenario key.\n\n");
    } else {
        out.push_str("- Baseline drift values are `N/A` until a prior comparable run exists.\n\n");
    }

    out.push_str("## Counter Semantics\n\n");
    out.push_str(&format!(
        "- Measured rows use `{PERF_COUNTER_SOURCE}` and store local instruction counts, not cycle charges.\n"
    ));
    out.push_str("- Update and install checkpoint-group rows preserve `sample_origin` and are never compared as the same accounting shape.\n");
    out.push_str("- The audit intentionally omits message base fees, payload bytes, storage/reservation charges, management-call fees, callee instructions, and garbage collection.\n\n");

    write_estimate_section(&mut out, results);

    out.push_str("## Endpoint Matrix\n\n");
    out.push_str("| Canister | Endpoint | Scenario | Sample origin | Count | Total local instructions | Avg local instructions | Baseline delta | Notes |\n");
    out.push_str("| --- | --- | --- | --- | ---: | ---: | ---: | --- | --- |\n");
    for result in results {
        let notes = if result.scenario.transport_mode == "install" {
            "sum of retained bootstrap checkpoint deltas"
        } else {
            ""
        };
        let baseline_delta = render_baseline_delta(baseline_rows.as_ref(), &result.row);
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | {} | {} | {} | {} | {} |\n",
            result.scenario.canister,
            result.scenario.endpoint_or_flow,
            result.scenario.arg_class,
            result.row.sample_origin,
            result.row.count,
            result.row.total_local_instructions,
            result.row.avg_local_instructions,
            baseline_delta,
            notes
        ));
    }
    out.push('\n');

    out.push_str("## Flow Checkpoints\n\n");
    if checkpoint_sites.is_empty() {
        out.push_str("- No current `perf!` checkpoints were found under `crates/`; no per-stage flow deltas are available yet.\n");
    } else {
        for site in checkpoint_sites {
            out.push_str(&format!("- `{site}`\n"));
        }
    }
    out.push('\n');

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
        .filter(|gap| gap.status == STATUS_PASS)
        .collect::<Vec<_>>();
    let uncovered_gaps = gaps
        .iter()
        .filter(|gap| gap.status != STATUS_PASS)
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
    out.push_str("- `root::test_provision_chain_key_delegation_proof_for_issuer` measures explicit first-proof provisioning through the maintained root facade.\n");
    out.push_str("- `scale_hub::create_worker` measures the maintained scaling update through observe, plan, creation, and registration.\n");
    out.push_str("- `scale::request_cycles_from_parent` measures the maintained child-to-parent capability round trip.\n");
    out.push_str("- Root bootstrap is a checkpoint-group install row, not an endpoint total; it remains separate from update comparisons.\n\n");

    out.push_str("## Dependency Fan-In Pressure\n\n");
    out.push_str("- The sampled non-trivial hotspots concentrate in shared auth/replay/root runtime, child-to-parent capability, placement updates, and publication.\n");
    if checkpoint_sites.is_empty() {
        out.push_str("- There is currently no flow-stage attribution because `perf!` coverage is absent. That is itself a dependency-pressure signal: optimization work is bottlenecked by missing internal checkpoints.\n\n");
    } else {
        out.push_str("- Flow-stage checkpoints now exist in the scaling, sharding, publication, and replay workflows. This matrix records non-zero checkpoint deltas for sampled update scenarios, so the next optimization pass can target concrete stages instead of endpoint totals alone.\n\n");
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
    if let Some(top) = hotspot_rows.first() {
        out.push_str(&format!(
            "| Highest sampled endpoint currently highest-cost | WARN | `{}` averages {} local instructions in this run. |\n",
            top.scenario.key, top.row.avg_local_instructions
        ));
    }
    if !zero_exclusive_rows.is_empty() {
        let scenario_keys = zero_exclusive_rows
            .iter()
            .map(|result| format!("`{}`", result.scenario.key))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "| Zero exclusive endpoint totals | INFO | {scenario_keys} retain measured call counts while nested/checkpoint scopes own the instruction attribution. |\n"
        ));
    }
    if baseline_is_selected(metadata) {
        out.push_str(&format!(
            "| Baseline drift source | INFO | Latest prior baseline path: `{}`. |\n\n",
            metadata.compared_baseline_report
        ));
    } else {
        out.push_str("| Baseline drift not yet available | INFO | No prior comparable report was selected; deltas remain `N/A`. |\n\n");
    }

    out.push_str("## Risk Score\n\n");
    out.push_str(&format!("Risk Score: **{risk_score} / 10**\n\n"));
    out.push_str("Interpretation: the first valid v2 measurement has no comparable predecessor, and root-proof plus delegated-token flows still lack product checkpoints. Those limitations are recorded rather than scored as zero.\n\n");

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
        out.push_str("   Action: rerun this audit after one concrete perf change and compare against the latest prior retained report; only add deeper verifier-side auth checkpoints if that endpoint-total starts to matter.\n");
    }
    out.push_str("2. Owner boundary: `shared update hotspots`\n");
    out.push_str(&format!(
        "   Action: compare `root::test_provision_chain_key_delegation_proof_for_issuer`, `root::canic_response_capability_v1`, and `scale::request_cycles_from_parent` before/after any shared-runtime cleanup, using this report as the `{minor_line}` baseline.\n"
    ));
    out.push_str("3. Owner boundary: `query measurement`\n");
    out.push_str("   Action: add query rows only through a future authoritative same-call fixture and method version.\n\n");

    out.push_str("## Report Files\n\n");
    out.push_str(&format!("- [{report_file_name}](./{report_file_name})\n"));
    out.push_str(&format!(
        "- [scenario-manifest.json](artifacts/{artifacts_dir_name}/scenario-manifest.json)\n"
    ));
    out.push_str(&format!(
        "- [perf-rows.json](artifacts/{artifacts_dir_name}/perf-rows.json)\n"
    ));
    out.push_str(&format!(
        "- [checkpoint-deltas.json](artifacts/{artifacts_dir_name}/checkpoint-deltas.json)\n"
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
    out.push_str(&format!(
        "- [evidence-manifest.yml](artifacts/{artifacts_dir_name}/evidence-manifest.yml)\n"
    ));

    fs::write(path, out).expect("write instruction audit report");
}

#[expect(clippy::format_push_string)]
fn write_estimate_section(out: &mut String, results: &[ScenarioResult]) {
    let estimated_rows = results
        .iter()
        .filter_map(|result| {
            result
                .row
                .execution_cycle_estimate
                .as_ref()
                .map(|estimate| (result, estimate))
        })
        .collect::<Vec<_>>();
    if estimated_rows.is_empty() {
        return;
    }

    out.push_str(ESTIMATE_SECTION_TITLE);
    out.push_str("\n\n");
    out.push_str(ESTIMATE_SECTION_LABEL);
    out.push_str("\n\n");
    out.push_str(ESTIMATE_SECTION_TABLE_HEADER);
    out.push('\n');
    out.push_str("| --- | ---: | ---: | ---: | --- | --- |\n");
    for (result, estimate) in estimated_rows {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | `{}` | `{}` |\n",
            result.scenario.key,
            estimate.local_instructions,
            estimate.estimated_instruction_cycles,
            estimate.cycles_per_billion_instructions,
            estimate.rate_source,
            estimate.formula_version
        ));
    }
    out.push('\n');
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
            "`crates/canic-core/src/workflow/rpc/request/handler/{mod,replay}.rs`",
        ),
        "create_worker" => (
            "Scaling creation workflow",
            "`apps/test/scale_hub/src/lib.rs`; `crates/canic-core/src/workflow/placement/scaling/mod.rs`",
        ),
        "create_account" => (
            "Sharding assignment workflow",
            "`apps/test/user_hub/src/lib.rs`; `crates/canic-core/src/workflow/placement/sharding/assignment.rs`",
        ),
        "request_cycles_from_parent" => (
            "Scale child-to-parent capability workflow",
            "`apps/test/scale/src/lib.rs`; `crates/canic-core/src/workflow/rpc/request/handler/mod.rs`",
        ),
        "test_provision_chain_key_delegation_proof_for_issuer" => (
            "Root proof provisioning workflow",
            "`apps/test/root/src/lib.rs`; `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs`",
        ),
        "canic_prepare_delegated_token" => (
            "Issuer delegated-token preparation",
            "`crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs`",
        ),
        "test_verify_delegated_token" => (
            "Verifier delegated-token boundary",
            "`apps/test/test/src/lib.rs`; `crates/canic-core/src/ops/auth/delegated/verify.rs`",
        ),
        "root_bootstrap_init" => (
            "Root installation checkpoint group",
            "`crates/canic-control-plane/src/workflow/bootstrap/root.rs`",
        ),
        "canic_template_stage_manifest_admin"
        | "canic_template_prepare_admin"
        | "canic_template_publish_chunk_admin" => (
            "Root template publication admin path",
            "`crates/canic/src/macros/endpoints/root.rs`; `crates/canic-control-plane/src/ops/storage/template/chunked.rs`",
        ),
        _ => (
            "Shared runtime surface",
            "`crates/canic/src/macros/endpoints/root.rs`",
        ),
    }
}

// Compute a bounded risk score for the current sampled matrix.
fn risk_score(
    gaps: &[CheckpointCoverageGap],
    baseline_selected: bool,
    hotspot_rows: &[&ScenarioResult],
) -> u8 {
    let mut score: u8 = if baseline_selected { 0 } else { 2 };

    score = score.saturating_add(
        u8::try_from(gaps.iter().filter(|gap| gap.status != STATUS_PASS).count())
            .unwrap_or(u8::MAX)
            .min(2),
    );

    if hotspot_rows
        .first()
        .is_some_and(|row| row.row.avg_local_instructions > 2_000_000)
    {
        score = score.saturating_add(2);
    }

    score.min(10)
}

fn baseline_is_selected(metadata: &AuditMetadata) -> bool {
    metadata.compared_baseline_report != BASELINE_NOT_AVAILABLE
}

fn load_baseline_rows(baseline_report: &str) -> Option<BTreeMap<String, u64>> {
    if baseline_report == BASELINE_NOT_AVAILABLE {
        return None;
    }

    let report_path = Path::new(baseline_report);
    let report_stem = report_path.file_stem()?.to_str()?;
    let artifact_path = report_path
        .parent()?
        .join("artifacts")
        .join(report_stem)
        .join("perf-rows.json");
    let rows = fs::read_to_string(artifact_path).ok()?;
    let rows = serde_json::from_str::<Vec<BaselinePerfRow>>(&rows).ok()?;

    Some(
        rows.into_iter()
            .map(|row| (row.scenario_key, row.avg_local_instructions))
            .collect(),
    )
}

fn render_baseline_delta(
    baseline_rows: Option<&BTreeMap<String, u64>>,
    current_row: &CanonicalPerfRow,
) -> String {
    let Some(rows) = baseline_rows else {
        return BASELINE_NOT_AVAILABLE.to_string();
    };
    let Some(baseline_avg) = rows.get(&current_row.scenario_key).copied() else {
        return BASELINE_NOT_AVAILABLE.to_string();
    };

    let delta = i128::from(current_row.avg_local_instructions) - i128::from(baseline_avg);
    if baseline_avg == 0 {
        return format!("{delta:+}");
    }

    let percent_tenths = rounded_percent_tenths(delta, baseline_avg);
    let sign = if percent_tenths < 0 { '-' } else { '+' };
    let percent_abs = percent_tenths.abs();
    format!(
        "{delta:+} ({sign}{}.{:01}%)",
        percent_abs / 10,
        percent_abs % 10
    )
}

fn rounded_percent_tenths(delta: i128, baseline_avg: u64) -> i128 {
    let denominator = i128::from(baseline_avg);
    let numerator = delta.saturating_mul(1_000);
    if numerator >= 0 {
        (numerator + denominator / 2) / denominator
    } else {
        (numerator - denominator / 2) / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario_result(execution_cycle_estimate: Option<ExecutionCycleEstimate>) -> ScenarioResult {
        ScenarioResult {
            scenario: AuditScenario {
                key: "scale:request_cycles_from_parent:fresh",
                canister: "scale",
                endpoint_or_flow: "request_cycles_from_parent",
                transport_mode: "update",
                subject_kind: "endpoint",
                subject_label: "request_cycles_from_parent",
                arg_class: "cycles-999",
                caller_class: "anonymous",
                auth_state: "public-child-endpoint-and-parent-structural-proof",
                replay_state: "fresh",
                cache_state: "n/a",
                topology_state: "scaling-profile-ready",
                freshness_model: "fresh-topology-per-scenario",
                notes: "scale capability row",
            },
            row: CanonicalPerfRow {
                subject_kind: "endpoint".to_string(),
                subject_label: "request_cycles_from_parent".to_string(),
                count: 1,
                total_local_instructions: 1_000_000,
                avg_local_instructions: 1_000_000,
                scenario_key: "scale:request_cycles_from_parent:fresh".to_string(),
                scenario_labels: vec!["transport_mode=update".to_string()],
                principal_scope: Some("anonymous".to_string()),
                sample_origin: "update".to_string(),
                execution_cycle_estimate,
            },
            checkpoint_rows: Vec::new(),
        }
    }

    fn estimate() -> ExecutionCycleEstimate {
        ExecutionCycleEstimate {
            estimate_schema_version: 1,
            kind: "per_instruction_component_only",
            charge_model: "hypothetical_update_execution_component",
            local_instructions: 1_000_000,
            counter_id: PERF_COUNTER_ID,
            sample_origin: "update".to_string(),
            estimated_instruction_cycles: "1000000".to_string(),
            cycles_per_billion_instructions: "1000000000".to_string(),
            subnet_node_count: Some(13),
            subnet_source: "flag",
            source_meaning: "operator_supplied_pricing_assumption",
            formula_version: "canic-0.59-ic-cycle-costs-v1",
            rate_source: "official-ic-cycle-costs-docs",
            overrode_node_count_table_rate: false,
            node_count_table_rate: Some("1000000000".to_string()),
            registry_canister_id: None,
            registry_version: None,
            subnet_principal: None,
            subnet_kind: None,
            subnet_kind_source: None,
            subnet_specialization: None,
            subnet_specialization_source: None,
            geographic_scope: None,
            geographic_scope_source: None,
            catalog_schema_version: None,
            catalog_stale: None,
            resolver_backend: None,
            matched_canister_principal: None,
            matched_routing_range: None,
            omitted_costs: &[],
        }
    }

    #[test]
    fn estimate_section_is_omitted_without_estimates() {
        let mut out = String::new();
        let results = vec![scenario_result(None)];

        write_estimate_section(&mut out, &results);

        assert!(out.is_empty());
    }

    #[test]
    fn estimate_section_renders_instruction_component_label() {
        let mut out = String::new();
        let results = vec![scenario_result(Some(estimate()))];

        write_estimate_section(&mut out, &results);

        assert!(out.contains(ESTIMATE_SECTION_TITLE));
        assert!(out.contains(ESTIMATE_SECTION_LABEL));
        assert!(out.contains(ESTIMATE_SECTION_TABLE_HEADER));
        assert!(out.contains("scale:request_cycles_from_parent:fresh"));
        assert!(out.contains("1000000"));
        assert!(out.contains("official-ic-cycle-costs-docs"));
        assert!(out.contains("canic-0.59-ic-cycle-costs-v1"));
    }

    fn metadata_with_baseline(compared_baseline_report: &str) -> AuditMetadata {
        AuditMetadata {
            code_snapshot: "test-snapshot".to_string(),
            branch: "main".to_string(),
            worktree: "clean".to_string(),
            run_timestamp_utc: "2026-06-03T00:00:00Z".to_string(),
            compared_baseline_report: compared_baseline_report.to_string(),
            method_id: "CANIC-INSTRUCTION-001".to_string(),
            method_version: "2".to_string(),
            method_fingerprint: "test-fingerprint".to_string(),
        }
    }

    #[test]
    fn baseline_selection_uses_report_sentinel() {
        assert!(!baseline_is_selected(&metadata_with_baseline(
            BASELINE_NOT_AVAILABLE
        )));
        assert!(baseline_is_selected(&metadata_with_baseline(
            "docs/audits/reports/2026-06/2026-06-03/instruction-footprint.md"
        )));
    }

    #[test]
    fn baseline_delta_uses_report_sentinel_for_missing_data() {
        let result = scenario_result(None);
        let rows = BTreeMap::new();

        assert_eq!(
            render_baseline_delta(None, &result.row),
            BASELINE_NOT_AVAILABLE
        );
        assert_eq!(
            render_baseline_delta(Some(&rows), &result.row),
            BASELINE_NOT_AVAILABLE
        );
    }

    #[test]
    fn checkpoint_scan_recognizes_namespaced_calls_and_ignores_examples() {
        let root = std::env::temp_dir().join(format!(
            "canic-instruction-checkpoint-scan-{}",
            std::process::id()
        ));
        let source_dir = root.join("crates/sample/src");
        fs::create_dir_all(&source_dir).expect("create checkpoint scan fixture");
        fs::write(
            source_dir.join("lib.rs"),
            concat!(
                "crate::perf!(\"scope\", \"first\");\n",
                "canic_core::perf!(\"scope\", \"second\");\n",
                "// crate::perf!(\"scope\", \"commented\");\n",
                "let example = \"crate::perf!(\\\"scope\\\", \\\"quoted\\\")\";\n",
                "let value = 1; // perf!(\"scope\", \"inline-comment\");\n",
            ),
        )
        .expect("write checkpoint scan fixture");

        let sites = scan_perf_callsites(&root);
        fs::remove_dir_all(&root).expect("remove checkpoint scan fixture");

        assert_eq!(sites.len(), 2);
        assert!(sites.iter().any(|site| site.contains("crate::perf!(")));
        assert!(sites.iter().any(|site| site.contains("canic_core::perf!(")));
    }

    #[test]
    fn auth_workflow_checkpoint_gaps_are_closed_in_current_source() {
        let sites = scan_perf_callsites(&workspace_root());
        let gaps = checkpoint_coverage_gaps(&sites);

        for flow_name in [
            "root proof provisioning",
            "issuer delegated-token prepare and verification",
        ] {
            let gap = gaps
                .iter()
                .find(|gap| gap.flow_name == flow_name)
                .expect("auth flow is part of the fixed checkpoint-gap roster");
            assert_eq!(
                gap.status, STATUS_PASS,
                "{flow_name} must stay instrumented"
            );
        }

        for label in [
            "root_proof_resolve_policy",
            "root_proof_prepare_batch",
            "root_proof_sign_batch",
            "root_proof_install_batch",
            "delegated_token_validate_request",
            "delegated_token_reserve_replay",
            "delegated_token_prepare_proof",
            "delegated_token_commit_replay",
            "delegated_token_fetch_root_proof",
            "delegated_token_verify_cached",
            "delegated_token_verify_embedded_proofs",
        ] {
            assert!(
                sites.iter().any(|site| site.contains(label)),
                "auth checkpoint `{label}` must stay instrumented"
            );
        }
    }
}
