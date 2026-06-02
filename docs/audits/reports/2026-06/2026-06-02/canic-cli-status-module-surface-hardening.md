# MSH Compact Audit: canic-cli status

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
| `in_scope_roots` | `crates/canic-cli/src/status/`, `crates/canic-cli/src/lib.rs` |
| `excluded_roots` | deploy modules currently under separate edit, historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused status tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `4 / 10`.
- Tier: `Tier 2` because the module reads deployment truth, observes local
  replica state, and can verify local deployment roots.
- Patch mode: read-only.
- Cleanup result: no safe `DELETE NOW`, `NARROW NOW`, or `INLINE NOW` action
  was identified in this run.

The module is current and diagnostic-only. It parses `canic status`, resolves
the current Canic ICP root, inspects local ICP project readiness, reads
configured fleet choices and installed deployment state, observes ICP CLI/local
replica status, optionally verifies local deployment roots, and renders a
compact text summary. The audited path does not install code, update
canisters, mutate deployment state, register deployments, or call DFX/ICP
mutation commands. The score reflects live/local observation and deployment
truth reads, not mutation authority.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for deployment-truth and runtime-evidence surfaces | terminal output |
| target inventory | `wc -l crates/canic-cli/src/status/*.rs` | status module totals `658` LOC across command and tests | terminal output |
| command/report inspection | `sed -n '1,260p' crates/canic-cli/src/status/mod.rs`; `sed -n '220,460p' crates/canic-cli/src/status/mod.rs` | PASS: command parses hidden network/ICP controls, loads local project and deployment status, classifies local root state, and renders text output | terminal output |
| focused test inspection | `sed -n '1,320p' crates/canic-cli/src/status/tests.rs` | PASS: tests cover option parsing, rendering, empty workspaces, HTTP fallback replica status, lost local targets, project-relative paths, local verification gating, role classification, and help text | terminal output |
| surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/status` | PASS: no dead-code allowances or stale compatibility markers; exported items are limited to command error and CLI entry surface | terminal output |
| authority boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|canister_metadata|canister_query|canister_update|query|update|icp|dfx|fs::|read_|write_|metadata|resolve_current_canic_icp_root|resolve_installed_deployment|include_str!|source_between" crates/canic-cli/src/status` | PASS with expected read/observation signals: `IcpCli::version`, `local_replica_project_running_in`, `resolve_current_canic_icp_root`, `read_installed_deployment_state_from_root`, `resolve_installed_deployment_from_root`, and test-only filesystem fixture writes; no mutation primitive found | terminal output |
| consumer check | `rg -n "status::run|mod status|StatusOptions|load_status_report|render_status_report|deployed_label|classify_local_deployment" crates/canic-cli/src -g '*.rs'` | PASS: production entry is `status::run`; helper surfaces are module-internal and covered by focused tests | terminal output |
| focused tests | `cargo test -p canic-cli status -- --nocapture` | PASS: 31 filtered status-related tests passed, including the 9 `status::tests::*` checks | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `status::run` CLI entry | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::status`; this is the top-level compact project status command. |
| `StatusOptions` parsing | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary. It keeps hidden network and ICP binary selection explicit while the public help remains compact. |
| `load_status_report` orchestration | `live-authority` | high | `RETAIN WITH OWNER` | Owner: status module. It composes local project readiness, replica status, deployment state, and rendered rows without mutating state. |
| ICP CLI/local replica status observation | `live-authority` | high | `RETAIN WITH OWNER` | Owner: status diagnostics. It reports ICP CLI version and local replica state through read/status helpers. |
| Installed deployment row classification | `live-authority` | high | `RETAIN WITH OWNER` | Owner: status diagnostics. It reads installed state and classifies local root health so operators can distinguish yes/no/unknown/partial/lost/error. |
| Text rendering | `live-authority` | medium | `RETAIN WITH OWNER` | Owner: CLI output boundary. Rendering maps the status report to the compact operator summary and lost-local guidance note. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Local project readiness | host ICP config helpers own project-root and `icp.yaml` inspection | no | `resolve_current_canic_icp_root` and `inspect_canic_icp_yaml_from_root` provide read-only status inputs | yes | no project config mutation found | Low |
| Replica status | ICP CLI/local HTTP status helpers own local replica observation | no | `IcpCli::local_replica_project_running_in` and `replica_query::local_replica_status_reachable_from_root` are status checks | yes | no start/stop/update authority found | Low |
| Deployment registry state | installed deployment helpers own deployment truth reads | no | `read_installed_deployment_state_from_root` and `resolve_installed_deployment_from_root` provide state/registry observations | yes | no register/install/write path found | Low |
| Canister/deployment mutation | install/deploy/register workflows own mutation | no | authority scan found no update/install/create/delete/register primitive | yes | no mutation authority drift found | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| Status command/options/render | `cold` | none | CLI-only parsing/rendering; no canister runtime or wasm-sensitive path | focused status tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Status report loading | `warm` | none | local observation and deployment-truth reads are operator-facing diagnostics; narrowing could hide current lost/partial local states | focused status tests plus manual/local proof for behavior changes | `RETAIN WITH OWNER` |
| Local deployment classifier | `warm` | none | simplification could collapse yes/partial/lost/unknown/error distinctions operators use to recover local state | focused classifier/render tests | `RETAIN WITH OWNER` |

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
| `cargo test -p canic-cli status -- --nocapture` | PASS | 31 filtered status-related tests passed, including 9 `status::tests::*` checks |
| `cargo check -p canic-cli` | PASS | owning package compiles |
