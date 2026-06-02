# MSH Compact Audit: canic-cli replica

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
| `code_snapshot` | `fe8747c2` |
| `in_scope_roots` | `crates/canic-cli/src/replica/`, `crates/canic-cli/src/lib.rs` |
| `excluded_roots` | deploy modules currently under separate edit, historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused replica tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `5 / 10`.
- Tier: `Tier 2` because the module owns local ICP replica lifecycle commands,
  invokes ICP CLI start/stop/status paths, and probes local replica ownership.
- Patch mode: read-only.
- Cleanup result: one minor safe cleanup candidate was identified and applied
  after explicit maintainer request.

The module is current and local-replica scoped. It parses `canic replica
start|status|stop`, checks project ICP config before start, enforces requested
gateway port consistency, starts or stops the local ICP replica through
host-owned `IcpCli` helpers, reports local status in text or JSON, detects
stale ICP CLI status versus reachable HTTP status, and refuses to stop unknown
foreign local replica owners. The audited path does not install code, update
canisters, mutate deployment truth, register deployments, or call management
canister mutation primitives. The score reflects local process lifecycle
authority and external ICP CLI invocation.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for runtime-evidence and authority surfaces | prior terminal output |
| target inventory | `wc -l crates/canic-cli/src/replica/*.rs` | replica module totals `840` LOC across command and tests | terminal output |
| lifecycle command inspection | `sed -n '1,300p' crates/canic-cli/src/replica/mod.rs`; `sed -n '280,660p' crates/canic-cli/src/replica/mod.rs` | PASS: command owns local replica start/status/stop only, with config readiness, port, HTTP reachability, stale status, and foreign-owner guards | terminal output |
| focused test inspection | `sed -n '1,280p' crates/canic-cli/src/replica/tests.rs` | PASS: tests cover option parsing, JSON status shape, help text, foreign-owner parsing, missing project diagnostics, and not-running detection | terminal output |
| surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/replica` | PASS: no dead-code allowances or stale compatibility markers; exported items are limited to command error and CLI entry surface | terminal output |
| authority boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|canister_metadata|canister_query|canister_update|query|update|icp|dfx|fs::|read_|write_|metadata|resolve_current_canic_icp_root|resolve_installed_deployment|include_str!|source_between|start|stop" crates/canic-cli/src/replica` | PASS with expected local lifecycle signals: `local_replica_start_in`, `local_replica_stop_in`, `local_replica_status_in`, `local_replica_status_json_in`, `local_replica_ping`, `resolve_current_canic_icp_root`, and HTTP reachability checks; no canister/deployment mutation primitive found | terminal output |
| consumer check | `rg -n "replica::run|mod replica|ReplicaOptions|run_start|run_status|run_stop|ReplicaStatusJsonReport|replica_icp_error|extract_foreign_local_owner|probe_reachable_local_replica_owner" crates/canic-cli/src -g '*.rs'` | PASS: production entry is `replica::run`; helper surfaces are module-internal and covered by focused tests | terminal output |
| focused tests | `cargo test -p canic-cli replica -- --nocapture` | PASS: 18 filtered replica-related tests passed, including replica option/error/status checks | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `replica::run` CLI entry | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::replica`; this is the top-level local ICP replica lifecycle command. |
| `ReplicaOptions` parsing | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary. It keeps ICP binary, foreground/background, port assertion, debug, and JSON mode explicit. |
| `run_start` lifecycle path | `live-authority` | high | `RETAIN WITH OWNER` | Owner: replica module. It checks ICP project readiness, enforces configured port, detects already-running/stale states, and delegates local replica start to `IcpCli`. |
| `run_status` and JSON report | `live-authority` | high | `RETAIN WITH OWNER` | Owner: replica diagnostics. It reports ICP CLI status, local HTTP reachability, stale status, and JSON status source without mutating state. |
| `run_stop` lifecycle path | `live-authority` | high | `RETAIN WITH OWNER` | Owner: replica module. It delegates local replica stop to `IcpCli` and refuses to stop unknown reachable foreign processes. |
| Error mapping and foreign-owner parsing | `live-authority` | high | `RETAIN WITH OWNER` | Owner: replica module. It converts raw ICP CLI diagnostics into Canic ownership and setup errors. |
| Redundant reachable guard in `run_start` | `orphaned-helper` | high | `NARROW NOW` | Applied: inside `if local_gateway_reachable`, the nested guard now checks only `!icp_cli_running`. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Local replica start | replica CLI owns operator-requested local replica start through host ICP CLI wrapper | no | `run_start` validates config/port before `IcpCli::local_replica_start_in` | yes | lifecycle authority is local-replica scoped, not canister mutation | Medium |
| Local replica status | replica CLI owns direct local replica status reporting | no | `run_status` and `run_status_json` combine ICP CLI status with HTTP reachability | yes | status distinguishes stale CLI state from reachable local HTTP state | Low |
| Local replica stop | replica CLI owns operator-requested local replica stop through host ICP CLI wrapper | no | `run_stop` calls `IcpCli::local_replica_stop_in` and guards unknown reachable owners | yes | no broad process-kill or foreign-owner stop authority found | Medium |
| Canister/deployment mutation | install/deploy/register workflows own mutation | no | authority scan found no update/install/create/delete/register primitive | yes | no canister/deployment mutation authority drift found | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| Replica command/options/render | `cold` | none | CLI-only parsing/output; no canister runtime or wasm-sensitive path | focused replica tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Replica lifecycle orchestration | `warm` | none | start/stop changes can affect local developer state and foreign-owner safety | focused parser/error tests plus manual local proof for behavior changes | `RETAIN WITH OWNER` |
| Redundant reachable guard | `cold` | removed duplicate conjunct | negligible; branch condition is already proven by the enclosing `if` | focused replica tests | `NARROW NOW` |

## Disposition Ledger

| Disposition | Count |
| ---- | ----: |
| DELETE NOW | 0 |
| NARROW NOW | 1 |
| INLINE NOW | 0 |
| MOVE OWNER | 0 |
| MOVE TO TEST | 0 |
| RETAIN WITH OWNER | 6 |
| DEFER WITH TRIGGER | 0 |
| MEASURE FIRST | 0 |
| BLOCKED | 0 |

## Follow-up

Completed: removed the redundant `&& local_gateway_reachable` conjunct from the
nested `run_start` guard after `if local_gateway_reachable`.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo test -p canic-cli replica -- --nocapture` | PASS | 18 filtered replica-related tests passed |
| `cargo check -p canic-cli` | PASS | owning package compiles |
