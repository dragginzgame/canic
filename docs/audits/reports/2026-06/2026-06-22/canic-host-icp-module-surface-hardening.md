# Module Surface Hardening: canic-host icp

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
| `code_snapshot` | `4bcad983` |
| `in_scope_roots` | `crates/canic-host/src/icp/` |
| `excluded_roots` | higher-level install/deploy/backup orchestration, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `4 / 10`.
- Tier: `Tier 2`, because this module owns host process execution wrappers for
  local replica lifecycle, canister start/stop/top-up, snapshot
  create/download/upload/restore, CLI version gating, JSON receipt parsing, and
  restore raw-command execution.
- Cleanup result: no high-confidence delete, narrow, inline, or move action was
  found. The retained public surface is broad, but current consumers map to
  live CLI, backup, restore, deployment-observation, readiness, metrics, and
  cycles flows.

The module remains the host-owned ICP CLI adapter. It constructs commands,
applies network/environment targeting, validates the `icp` executable before
normal command execution, parses typed JSON receipts, and exposes dry-run
display strings for operator previews. It does not own higher-level install,
deployment-truth, backup, restore, or controller policy decisions.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| cleanup runner fallback review | `sed -n '1,280p' docs/audits/modular/module-cleanup-runner.md` | PASS: cleanup runner checked as the implementation fallback; no patch was needed | prior terminal output |
| MSH definition review | `sed -n '1,320p' docs/audits/modular/module-surface-hardening.md`; `sed -n '321,620p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked for this run | prior terminal output |
| target inventory | `find crates/canic-host/src/icp -maxdepth 2 -type f | sort`; `wc -l crates/canic-host/src/icp/*.rs` | PASS: module totals `1659` LOC across command, run, canister, snapshot, replica, version, model, error, and tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/icp -g '*.rs'` | PASS: no dead-code or stale-compatibility markers found; public surface is command/runner/model/receipt/error facade plus `IcpCli` methods | terminal output |
| consumer check | `rg -n "IcpCli|IcpCommandError|IcpRawOutput|IcpSnapshot|IcpCanisterStatus|IcpCliVersion|run_json|run_output|run_output_with_stderr|run_raw_output|run_status|run_status_inherit|run_success|add_candid_arg|add_debug_arg|add_output_arg|add_target_args|command_display|default_command|default_command_in|ensure_command_compatible|existing_local_canister_candid_path|local_canister_candid_path|parse_snapshot_id|parse_icp_cli_version|is_supported_icp_cli_version|REQUIRED_ICP_CLI_VERSION|ICP_CLI_SUPPORTED_VERSION_RANGE" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: public types and free functions are consumed by host deployment observation/install/release-set helpers, CLI status/replica/cycles/token/list/backup/snapshot/restore, and backup runner flows | terminal output |
| method consumer check | `rg -n "\\.canister_call_output|\\.canister_call_output_with_candid|\\.canister_call_arg_output|\\.canister_call_arg_output_with_candid|\\.canister_query_output|\\.canister_query_output_with_candid|\\.canister_query_arg_output|\\.canister_query_arg_output_with_candid|\\.canister_metadata_output|\\.canister_status\\(|\\.canister_top_up_output|\\.canister_status_report|\\.stop_canister\\(|\\.start_canister\\(|\\.snapshot_create|\\.snapshot_download|\\.snapshot_upload|\\.snapshot_restore|\\.local_replica_" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: command methods are reachable through live read/query, replica, backup, snapshot, cycles, and restore flows | terminal output |
| authority boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|snapshot|top-up|call|--query|network start|network stop|run_status|run_status_inherit|run_output|run_json|Command::new|\\.output\\(|\\.spawn\\(" crates/canic-host/src/icp -g '*.rs'` | PASS with expected mutation signals: module adapts ICP CLI mutations but does not make higher-level policy decisions | terminal output |
| focused tests | `cargo test --locked -p canic-host icp:: -- --nocapture` | PASS: 20 focused tests passed | terminal output |

## Reachable Surface Inventory

| Item | Kind | Visibility | Consumer Evidence | Consumer Should Exist? | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `IcpCli` and command-context methods | model plus inherent API | `pub` | Host and CLI construct scoped ICP CLI contexts for network, environment, and project-root execution. | Yes | Canonical host adapter for selected `icp` executable, network/environment targeting, and project-root override. | `live-authority` | `canic-host::icp` | `RETAIN WITH OWNER` | Changing this can break all host/CLI ICP execution. |
| Command helpers (`add_target_args`, `add_output_arg`, `add_candid_arg`, `command_display`, defaults) | free functions | `pub` | Install/release-set helpers, cycles/token custom calls, support/candid tests, and dry-run output use these directly. | Yes | Shared command construction avoids divergent targeting, JSON, Candid, and display semantics. | `live-authority` | `canic-host::icp::command` | `RETAIN WITH OWNER` | Narrowing requires rewiring several current command builders. |
| Run helpers (`run_output`, `run_json`, `run_status`, `run_status_inherit`, `run_success`, `run_raw_output`) | free functions | `pub` | Install, release-set, replica, cycles/token, and restore execution paths use these helpers. | Yes | Central process execution and version-gating boundary for `icp` commands. | `live-authority` | `canic-host::icp::run` | `RETAIN WITH OWNER` | Mutating process execution behavior is deployment-adjacent. |
| Canister methods on `IcpCli` | inherent API | `pub` | Readiness, metadata, subnet registry, metrics, cycles, backup, snapshot, restore, and deployment observation call these methods. | Yes | Canonical typed command surface for query/update/status/top-up/start/stop operations. | `live-authority` | `canic-host::icp::canister` | `RETAIN WITH OWNER` | Contains controller and canister mutation adapters. |
| Snapshot methods and receipts | inherent API and DTOs | `pub` | CLI snapshot download, backup capture, and restore apply use snapshot create/download/upload/restore and receipts. | Yes | Canonical ICP CLI snapshot adapter and typed JSON receipt contract. | `live-authority` | `canic-host::icp::snapshot` | `RETAIN WITH OWNER` | Backup/recovery adjacent; removal needs recovery proof. |
| Replica methods | inherent API | `pub` | CLI status/replica commands use local replica start/status/stop/ping helpers. | Yes | Canonical local replica lifecycle adapter. | `live-authority` | `canic-host::icp::replica` | `RETAIN WITH OWNER` | Local process lifecycle behavior is operator-visible. |
| Version helpers and constants | parsing/gating API | `pub` | CLI status/medic, process runners, and tests validate supported `icp` versions. | Yes | Fail-closed `icp` executable compatibility boundary. | `live-authority` | `canic-host::icp::version` | `RETAIN WITH OWNER` | Weakening version gate can run unsupported commands. |
| Error and typed output models | error/DTO surface | `pub` | CLI and host command errors map `IcpCommandError`; deployment observation consumes status DTOs. | Yes | Typed host boundary for process, JSON, snapshot, and status failures. | `live-authority` | `canic-host::icp::model` and `error` | `RETAIN WITH OWNER` | Error variant removal requires all CLI/backup mappings to change. |

## Dead / Stale Surface Signals

| Candidate | File | Lines | Signal | Current Consumers | Consumer Should Exist? | Authority Reason | Surface Class | Deletion Confidence | Disposition | Risk If Removed |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| None found | `crates/canic-host/src/icp/` | N/A | No `dead_code`, stale compatibility, shim, deprecated, temporary, or TODO/FIXME marker found. | N/A | N/A | N/A | N/A | N/A | `REJECT CLEANUP` | N/A |

## Runtime Authority Drift Check

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Command targeting | Caller-selected environment/network plus `CANIC_ICP_LOCAL_*` env overrides for local environment targeting. | No stronger owner in inspected module. | `add_target_args`, `add_local_network_target`, focused render tests. | Yes | No stale network selector found. | Incorrect targeting can mutate/query the wrong network. |
| Version gating | `compatible_version_output` and `ensure_command_compatible` gate normal `icp` execution. | Restore raw command execution has its own `run_raw_output` gate when program basename is `icp`. | `run_*` helpers and unsupported fake-`icp` test. | Yes | Gate is centralized and tested. | Removing the gate can run unsupported CLI versions. |
| Canister mutation | Higher-level backup/restore/install/cycles code decides whether to stop/start/top-up/snapshot; `icp` only adapts commands. | Yes, orchestration lives outside this module. | Consumer scan and canister/snapshot method bodies. | Yes | No policy drift into `icp`; it remains an adapter. | Deleting adapter methods would break live authority flows. |
| Snapshot receipts | ICP CLI JSON output is parsed into typed host receipt DTOs. | No alternate host receipt parser found. | `IcpSnapshotCreateReceipt`, `IcpSnapshotUploadReceipt`, JSON tests. | Yes | Receipt DTOs are passive boundary data. | Shape changes require backup/restore validation. |
| Canister status reports | ICP CLI JSON output is parsed into typed status DTOs and projected by deployment-truth observation. | Deployment-truth owns interpretation, not the raw parse shape. | `IcpCanisterStatusReport`, deployment-truth consumers. | Yes | No authority reconstruction drift found. | DTO narrowing needs deployment-truth tests. |

## Complexity And Runtime Shape

| Module | Complexity Signal | Retention Justification | Dead-Surface Link | Public/Hidden Items | Current Consumers | Shrink Action | Disposition | Expected Blast Radius | Risk |
| ---- | ---- | ---- | ---- | ----: | ---- | ---- | ---- | ---- | ---- |
| `canister.rs` | Parallel no-arg/arg/query/update/display helpers. | Keeps command shape explicit for query vs update and dry-run parity. | None found. | Broad inherent API. | CLI metrics/cycles/snapshot/backup/endpoints, host readiness/registry/deployment observation. | None. | `RETAIN WITH OWNER` | Broad. | Mutating canister command surface. |
| `snapshot.rs` | Create/upload receipt methods plus ID convenience methods and dry-run display methods. | Backup/restore needs typed receipts, artifact paths, and operator preview strings. | None found. | Public snapshot API and private token parser. | CLI snapshot and backup/restore flows. | None. | `RETAIN WITH OWNER` | Backup/recovery adjacent. | High if changed without recovery proof. |
| `replica.rs` | Rooted and non-rooted local replica helpers. | CLI status and replica commands need project-scoped and default-context lifecycle checks. | None found. | Public lifecycle API and private command builder. | CLI status/replica flows. | None. | `RETAIN WITH OWNER` | Operator-visible local process behavior. | Medium. |
| `run.rs` | Multiple process execution shapes. | Separate stdout, stderr, JSON, inherited-I/O, success, and raw restore command contracts. | None found. | Public runner API plus private stderr/exit helpers. | Install, release-set, restore, cycles/token, replica. | None. | `RETAIN WITH OWNER` | Broad. | Process execution and diagnostics. |

## Facade / Generated Boundary Review

| Surface | Boundary Type | Generated Consumer Evidence | Could Narrow? | Required Replacement | Deletion Confidence | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `pub mod icp` from `canic-host` | Host facade | No generated consumer found. | Not in this slice; external workspace consumers and CLI/backup rely on it. | Dedicated narrower host command/report APIs would need a larger migration. | Low | `RETAIN WITH OWNER` | Public host adapter surface. |
| `parse_snapshot_id` | Host helper facade | No generated consumer found. | Maybe, but current tests and legacy output parsing are harmless and backup/snapshot adjacent. | Remove only after all snapshot create flows use JSON receipt parsing exclusively. | Low | `DEFER WITH TRIGGER` | Snapshot output compatibility risk. |

## Feature / Diagnostics / Test Surface Review

| Surface | Feature/Cfg | Production Consumer? | Test/Diagnostics Consumer? | Visibility Could Narrow? | Action | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `#[cfg(test)] mod tests` | test only | No | Yes, 20 focused tests. | Already test-only. | None. | `RETAIN WITH OWNER` | Low. |
| Dry-run display methods | normal production | Yes, CLI dry-run/operator previews. | Yes, render tests. | No safe narrowing found. | None. | `RETAIN WITH OWNER` | Operator-visible output drift. |
| `run_raw_output` | normal production | Yes, restore command execution. | Indirect restore tests outside this module. | No; restore imports it through `canic_host::icp`. | None. | `RETAIN WITH OWNER` | Restore execution behavior. |

## Removal Safety Plan

| Candidate | Action | Disposition | Owner Boundary | Hotness | Required Proof | Focused Validation | Wasm Raw Bytes Relevant? | Follow-Up Trigger |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| None | No patch. | `REJECT CLEANUP` | `canic-host::icp` | `cold` operator/process execution, but deployment/recovery adjacent | N/A | `cargo test --locked -p canic-host icp:: -- --nocapture` | No | N/A |
| `parse_snapshot_id` legacy text parser | Keep for now. | `DEFER WITH TRIGGER` | `canic-host::icp::snapshot` | `cold` | Prove no production flow consumes non-JSON snapshot-create output and remove `SnapshotIdUnavailable` mappings at the same time. | Snapshot/backup/restore focused tests. | No | All snapshot create flows rely only on `snapshot_create_receipt`. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Canister/snapshot mutation API narrowing | Public host/CLI/backup/restore consumers rely on this facade, and it is recovery/deployment adjacent. | Owner-approved API migration plus backup/restore/snapshot/deployment validation. |
| Process runner shape changes | `run_status_inherit`, stderr streaming, version checks, and raw restore command execution are operator-visible. | Focused behavior tests and CLI/restore validation before changing execution shape. |

## Verification

- `cargo fmt --all -- --check`: not run; no source edits.
- `cargo test --locked -p canic-host icp:: -- --nocapture`: PASS, 20 focused tests passed.
- `cargo check --locked -p canic-host`: not run; focused tests compiled `canic-host`.
- `cargo clippy --locked -p canic-host --all-targets --all-features -- -D warnings`: not run; no source edits.
- wasm/raw-size check: not applicable; host process-adapter audit with no runtime wasm payload change.
