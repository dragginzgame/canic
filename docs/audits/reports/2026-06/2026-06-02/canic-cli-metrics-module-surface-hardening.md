# MSH Compact Audit: canic-cli metrics

## Preamble

| Field | Value |
| ---- | ---- |
| `method_version` | `MSH-2.0` |
| `surface_taxonomy` | `ST-1` |
| `authority_taxonomy` | `AT-1` |
| `deletion_confidence_model` | `DC-1` |
| `compatibility_policy` | `pre-1.0-hard-cut` |
| `wasm_signal_rule` | `raw-wasm-primary` |
| `hot_path_risk_model` | `HP-1` |
| `proof_policy` | `read-only-first` |
| `baseline_report` | `N/A` |
| `comparability_status` | `non-comparable`: first targeted MSH run for this module |
| `code_snapshot` | `ef49146a` |
| `in_scope_roots` | `crates/canic-cli/src/metrics/`, `crates/canic-cli/src/lib.rs` |
| `excluded_roots` | historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused metrics tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `4 / 10`.
- Tier: `Tier 2` because the module queries live canister telemetry and parses
  runtime metric payloads.
- Patch mode: read-only.
- Cleanup result: no safe `DELETE NOW`, `NARROW NOW`, or `INLINE NOW` action
  was identified in this run.

The module is current and query-only. It parses `canic metrics`, resolves an
installed deployment registry, filters by role or canister, queries each target
with the `canic_metrics` query method, parses JSON or response-Candid metric
pages, and renders text or JSON. The audited path does not install code, update
canisters, mutate deployment state, register deployments, or call DFX/ICP
mutation commands. The score reflects live query and parser complexity, not
mutation authority.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md`; `sed -n '261,560p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for query/encode and runtime-evidence surfaces | prior terminal output |
| target inventory | `wc -l crates/canic-cli/src/metrics/*.rs` | metrics module totals `843` LOC across model/options/parse/render/transport/tests | terminal output |
| command inspection | `sed -n '1,220p' crates/canic-cli/src/metrics/mod.rs`; `sed -n '1,220p' crates/canic-cli/src/metrics/options.rs` | PASS: command parses deployment target, metric kind, filters, output mode, network, and ICP path before delegating to transport/render | terminal output |
| parser/render/transport inspection | `sed -n '1,180p' crates/canic-cli/src/metrics/model.rs`; `sed -n '1,200p' crates/canic-cli/src/metrics/render.rs`; `sed -n '1,220p' crates/canic-cli/src/metrics/parse.rs`; `sed -n '1,280p' crates/canic-cli/src/metrics/transport.rs` | PASS: model is passive DTO-like CLI output data; parser handles JSON and response-Candid pages; transport performs query-only canister calls | terminal output |
| focused test inspection | `sed -n '1,240p' crates/canic-cli/src/metrics/tests.rs` | PASS: tests cover kind parsing, deployment wording, JSON shape, missing deployment guidance, JSON parsing, response-Candid parsing, and malformed JSON fallback behavior | terminal output |
| surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/metrics` | PASS: no dead-code allowances or stale compatibility markers; exported items are private-module or crate CLI entry surfaces | terminal output |
| authority boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|canister_query|canister_update|query|update|icp|dfx|thread::spawn|fs::|write_|read_|metadata|resolve_current_canic_icp_root|resolve_installed_deployment" crates/canic-cli/src/metrics` | PASS with expected query/read/output signals: `canister_query_arg_output`, `resolve_installed_deployment_from_root`, `resolve_current_canic_icp_root`, `thread::spawn`, and output writers; no mutation primitive found | terminal output |
| consumer check | `rg -n "metrics::run|mod metrics|MetricsOptions|metrics_report|parse_metrics_page|write_metrics_report|CANIC_METRICS_METHOD" crates/canic-cli/src -g '*.rs'` | PASS: production entry is `metrics::run`; helper surfaces are module-internal and covered by focused tests | terminal output |
| stale-signal scan | `rg -n "include_str!|source_between|allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|TODO|FIXME|legacy|compat|fallback|shim|deprecated" crates/canic-cli/src/metrics` | PASS: only intentional response-Candid parser fallback test names were found | terminal output |
| focused tests | `cargo test -p canic-cli metrics -- --nocapture` | PASS: 10 metrics tests passed | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `metrics::run` CLI entry | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::metrics`; this is the top-level CLI entry for runtime telemetry queries. |
| `MetricsOptions` parsing | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary. It keeps deployment target, metric kind, filters, output mode, network, and ICP binary selection explicit. |
| `metrics_report` transport orchestration | `live-authority` | high | `RETAIN WITH OWNER` | Owner: metrics transport. It resolves installed deployment state, filters registry entries, and gathers query-only canister reports. |
| `query_metrics` | `live-authority` | high | `RETAIN WITH OWNER` | Owner: metrics transport. It calls `canic_metrics` through `IcpCli::canister_query_arg_output`, not update/install paths. |
| Metric response parser | `live-authority` | high | `RETAIN WITH OWNER` | Owner: metrics parser. JSON and response-Candid parsing support current ICP CLI response shapes, with focused tests for both. |
| Text/JSON rendering | `live-authority` | medium | `RETAIN WITH OWNER` | Owner: CLI output boundary. Rendering maps structured metric values to stable text/JSON output without owning metric collection semantics. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Metrics collection | canister runtime owns `canic_metrics` data; CLI only queries and renders | no | `query_metrics` calls `canister_query_arg_output` with `CANIC_METRICS_METHOD` | yes | no metric mutation authority found | Low |
| Deployment registry selection | installed deployment resolution owns target registry lookup | no | `resolve_installed_deployment_from_root` provides registry entries; CLI only filters them | yes | no parallel deployment discovery path found | Low |
| Canister/deployment mutation | install/deploy/register workflows own mutation | no | authority scan found no update/install/create/delete/register primitive | yes | no mutation authority drift found | Low |
| Parser fallback | parser owns compatibility with current JSON and response-Candid query output shapes | no | focused tests cover malformed JSON and response-Candid fallback | yes | fallback is current response parsing, not stale public surface | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| Metrics command/options/render | `cold` | none | CLI-only parsing/rendering; no canister runtime or wasm-sensitive path | focused metrics tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Metrics parser | `warm` | none | response parsing is operator-facing query output; simplification could break ICP CLI wrapper shapes | focused parser tests for JSON and response-Candid | `RETAIN WITH OWNER` |
| Metrics transport thread fanout | `warm` | none | query fanout is bounded by resolved registry/filter set; changes should preserve per-canister error isolation | focused transport tests plus manual/live proof for behavior changes | `RETAIN WITH OWNER` |

## Disposition Ledger

| Disposition | Count |
| ---- | ----: |
| DELETE NOW | 0 |
| NARROW NOW | 0 |
| INLINE NOW | 0 |
| MOVE OWNER | 0 |
| MOVE TO TEST | 0 |
| RETAIN WITH OWNER | 6 |
| DEFER WITH TRIGGER | 0 |
| MEASURE FIRST | 0 |
| BLOCKED | 0 |

## Follow-up

No cleanup follow-up was found.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo test -p canic-cli metrics -- --nocapture` | PASS | 10 metrics tests passed |
| `cargo check -p canic-cli` | PASS | owning package compiles |
