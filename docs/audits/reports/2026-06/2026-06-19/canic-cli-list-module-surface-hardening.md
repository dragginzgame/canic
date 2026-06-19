# MSH Module Cleanup: canic-cli list

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
| `code_snapshot` | `16894709` |
| `in_scope_roots` | `crates/canic-cli/src/list/` |
| `excluded_roots` | backup/restore recovery workflows, install/deploy mutation, historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused list tests |
| `audit_tier` | `Tier 1` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `3 / 10`.
- Tier: `Tier 1`, because this module owns read-only operator output over
  installed deployment registry state, config declarations, live readiness,
  Canic metadata, cycle balances, and local wasm artifact sizes. It does not
  own install/deploy mutation, backup/recovery authority, stable storage, wasm
  payloads, or generated boundaries.
- Cleanup result: no high-confidence delete, narrow, inline, or move action was
  found. The retained surface has current CLI ownership and focused test
  coverage.

The module remains the `canic info list` and `canic fleet config` list
boundary. It parses list/config arguments, resolves installed registry or
declared config data, queries read-only live status and telemetry, and renders
stable text tables. It does not mutate controller state, install code, write
deployment truth, or own recovery behavior.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| runner review | `sed -n '1,260p' docs/audits/modular/module-cleanup-runner.md` | PASS: implementation-requested cleanup runner checked for this run | terminal output |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked for this run | terminal output |
| target inventory | `wc -l crates/canic-cli/src/list/*.rs crates/canic-cli/src/list/live/*.rs` | PASS: module totals `1804` LOC across command, config, options, render, live, and focused tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME|doc\\(hidden\\)" crates/canic-cli/src/list -g '*.rs'` | PASS: no stale compatibility or dead-code markers; exported items are command entry/error surfaces or private-module `pub(super)` helpers | terminal output |
| consumer check | `rg -n "list::run_info|list::run_config|run_info\\(|run_config\\(|ListCommandError|ListOptions|render_registry_tree|render_list_output|render_config_output|load_config_role_rows|missing_config_roles|selected_config_path|list_ready_statuses|list_cycle_balances|list_canic_versions|list_module_hashes|resolve_wasm_sizes" crates/canic-cli/src -g '*.rs'` | PASS: production entries are `list::run_info` and `list::run_config`; helper surfaces are module-internal or focused-test consumers | terminal output |
| authority boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|canister_metadata|canister_query|canister_update|query|update|icp|dfx|fs::|read_|write_|metadata|resolve_current_canic_icp_root|resolve_installed_deployment|thread::spawn|include_str!|source_between" crates/canic-cli/src/list -g '*.rs'` | PASS with expected read/query signals: ICP root resolution, installed deployment reads, local metadata reads, query-only status/version/cycles calls, and bounded per-entry query fan-out; no mutation primitive found | terminal output |
| focused tests | `cargo test --locked -p canic-cli list --lib -- --nocapture` | PASS: 50 filtered tests passed, including all list module tests | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| None | `REJECT CLEANUP` | No stale compatibility, orphaned helper, overexposed external surface, or one-caller helper with no invariant was found. | Focused list tests passed. |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `run_info` and `run_config` | `canic-cli::list` | Public CLI entry points for installed registry listing and fleet config listing. | Revisit only if the top-level command dispatcher absorbs leaf command execution. |
| `ListCommandError` | `canic-cli::list` | Command error boundary for usage, registry tree, local query, installed deployment, config, discovery, and registry parse failures. | Revisit if command error aggregation moves to a shared CLI dispatcher. |
| `ListOptions` and usage builders | `canic-cli::list::options` | Clap-backed parser for deployment-target list and fleet-template config commands, including hidden network/ICP controls. | Revisit with a CLI parser redesign. |
| Config row loaders | `canic-cli::list::config` | Read declared fleet config roles and passive role attributes for `canic fleet config`. | Revisit if host-owned config inspection grows a dedicated report API. |
| Live registry readers | `canic-cli::list::live` | Read installed deployment state and query readiness, Canic version, cycle balance, module hash, and local wasm artifact size for operator display. | Revisit if live registry observation moves to a shared telemetry module. |
| Render types and table functions | `canic-cli::list::render` | Stable text rendering for registry/config list output, including subtree rows and same-role module hash drift highlighting. | Revisit if list output becomes a shared table/report format. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Query fan-out helper shape | `thread::spawn` fan-out is on the live operator query path and changing it could alter latency, ordering, or failure behavior without a cleanup win. | Dedicated operator-query performance or behavior proof before changing concurrency shape. |
| Backup/restore-adjacent cleanup | Explicitly excluded by the current follow-up guidance for this CLI-tree pass. | Separate Tier 2 backup/restore MSH pass. |

## Verification

- `cargo fmt --all -- --check`: not run; no code edits.
- `cargo test --locked -p canic-cli list --lib -- --nocapture`: PASS, 50 filtered tests passed.
- `cargo check --locked -p canic-cli`: not run; focused list tests compiled `canic-cli`.
- `cargo clippy --locked -p canic-cli --all-targets --all-features -- -D warnings`: not run; no code edits.
- wasm/raw-size check: not applicable; CLI-only read/render audit with no code edits.
