# Module Surface Hardening: canic-host install_root

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
| `in_scope_roots` | `crates/canic-host/src/install_root/` |
| `excluded_roots` | lower-level deployment-truth implementation internals, ICP CLI adapter internals, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `3 / 10`.
- Tier: `Tier 2`, because this module owns the local root install workflow,
  deployment-truth gate integration, install-state persistence, root
  registration/verification recovery, phase receipts, artifact-promotion
  receipts, canister creation/install/funding, and local readiness checks.
- Cleanup result: test-only convenience imports were removed from the
  `install_root` module root and moved to direct owner-module imports in the
  test module. The facade root was then narrowed further by moving install
  option shape, identity resolution, host-clock helpers, and current-install
  capability constants into focused owner modules. Production behavior is
  unchanged, while the module root now stays focused on discovery and the
  top-level install sequence.

The module remains the host-owned install authority boundary, so the residual
score should not drop below `3 / 10` without moving actual mutation authority
out of scope. It may mutate local install state, create/install/fund/root
canisters, stage release-set artifacts, and write deployment-truth receipts, but
it gates those mutations behind deployment-truth checks and execution preflight
receipts. Read-only check entry points are explicitly separated from mutating
install and registration paths.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,220p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked for this run | terminal output |
| cleanup runner review | `sed -n '1,220p' docs/audits/modular/module-cleanup-runner.md` | PASS: cleanup runner checked for the implementation-requested narrowing slice | terminal output |
| target inventory | `find crates/canic-host/src/install_root -type f -name '*.rs' | sort`; `wc -l crates/canic-host/src/install_root/*.rs crates/canic-host/src/install_root/*/*.rs crates/canic-host/src/install_root/*/*/*.rs` | PASS: module totals `8032` LOC across `56` Rust files covering install workflow, state, receipts, truth gates, phase operations, readiness, registration, and tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/install_root -g '*.rs'` | PASS: public facade and internal operation surfaces identified; stale markers are live legacy-state rejection and diagnostic fallback paths | terminal output |
| consumer check | `rg -n "InstallRootOptions|install_root\\(|discover_current_canic_config_choices|current_canic_project_root|discover_canic_config_choices|discover_canic_project_root_from|discover_project_canic_config_choices|project_fleet_roots|RegisterDeploymentStateOptions|VerifyDeploymentRootOptions|register_deployment_state|verify_registered_deployment_root|latest_deployment_truth_receipt_path_from_root|InstallState|RootVerificationStatus|read_named_deployment_install_state|read_named_deployment_install_state_from_root|check_install_deployment_truth|check_install_execution_preflight" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: facade surface is live across CLI install/deploy/register/root verification, deployment catalog, installed-deployment, deployment-truth, medic, list, metrics, cycles, token, snapshot, and backup flows | terminal output |
| authority boundary scan | `rg -n "install_root|register_deployment_state|verify_registered_deployment_root|write_install_state|read_deployment_install_state|deployment_truth|DeploymentCheck|ExecutionPreflight|receipt|create_canister|install_code|top-up|snapshot|restore|fs::write|fs::read|create_dir_all|run_command|icp::run|Command::new|canonicalize|current_dir" crates/canic-host/src/install_root -g '*.rs'` | PASS with expected mutation and persistence signals; read-only checks and mutating paths are separated | terminal output |
| cleanup patch | direct source inspection and diff review | PASS: removed module-root `#[cfg(test)] use` imports and moved test-only access to direct owner-module imports in `tests/mod.rs` | source diff |
| facade split patch | direct source inspection and diff review; `wc -l crates/canic-host/src/install_root/mod.rs crates/canic-host/src/install_root/options.rs crates/canic-host/src/install_root/identity.rs crates/canic-host/src/install_root/clock.rs crates/canic-host/src/install_root/capabilities.rs` | PASS: `mod.rs` narrowed to `181` lines; install options, identity resolution, clock labels, and current-install capability constants moved to focused child modules | source diff |
| focused tests | `cargo test --locked -p canic-host install_root:: -- --nocapture` | PASS: 80 focused tests passed after cleanup | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | PASS | terminal output |

## Reachable Surface Inventory

| Item | Kind | Visibility | Consumer Evidence | Consumer Should Exist? | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `InstallRootOptions` and `install_root` | facade DTO and workflow entry | `pub` | CLI `install` and `deploy install` construct options and call install. | Yes | Canonical local thin-root install workflow and mutation authority. | `live-authority` | `canic-host::install_root` | `RETAIN WITH OWNER` | High; can create/install/fund/stage root. |
| Config discovery exports | facade helpers | `pub` re-exports | CLI scaffold/status/fleets/build/list and `icp_config` consume project/fleet discovery helpers. | Yes | Shared host project/fleet selection boundary. | `live-authority` | `install_root::config_selection` | `RETAIN WITH OWNER` | Medium; wrong discovery affects operator targets. |
| Deployment registration and verification | facade DTOs/functions | `pub` | CLI deploy register/root verification and tests use recovery path. | Yes | Explicit unverified registration and evidence-bound root verification after the state hard cut. | `live-authority` | `install_root::deployment_registration` | `RETAIN WITH OWNER` | High; local state and root verification authority. |
| `InstallState` and readers | persisted DTO and read API | `pub` | Deployment catalog, installed-deployment, deployment truth observation/planning, CLI medic/snapshot/backup/list/cycles/token consume state. | Yes | Canonical local deployment-target state schema and read boundary. | `live-authority` | `install_root::state` | `RETAIN WITH OWNER` | High; persisted format. |
| Deployment truth check/preflight | read-only facade | `pub` | CLI deploy checks and tests use read-only preflight before mutation. | Yes | Read-only deployment-truth evidence and executor capability validation. | `live-authority` | `install_root::truth_check` | `RETAIN WITH OWNER` | High; safety gate authority. |
| Receipt discovery | facade helper | `pub` | CLI deploy resume-report and tests locate latest deployment-truth receipt. | Yes | Canonical local receipt path/discovery contract. | `live-diagnostics` | `install_root::receipt_io` | `RETAIN WITH OWNER` | Medium; operator evidence. |
| Internal phase operations | operation structs/trait | `pub(in crate::install_root)` or `pub(super)` | Install workflow and tests verify operation evidence and ordering. | Yes | Keeps install mutation phases explicit and receipt-bearing. | `live-authority` | `install_root::operations`, `activation`, `preparation`, `plan_artifacts` | `RETAIN WITH OWNER` | High; phase ordering and receipt semantics. |

## Dead / Stale Surface Signals

| Candidate | File | Lines | Signal | Current Consumers | Consumer Should Exist? | Authority Reason | Surface Class | Deletion Confidence | Disposition | Risk If Removed |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Legacy fleet install-state rejection | `state.rs` | `legacy_fleet_install_state_path`, `reject_legacy_fleet_state` | `legacy` signal | State reads and tests prove old fleet-scoped state fails closed. | Yes | Guards the post-0.46 deployment-target state hard cut and tells operators how to recover. | `live-authority` | Low | `RETAIN WITH OWNER` | Removing it could silently ignore stale local authority evidence. |
| Registry roles JSON diagnostic fallback | `readiness/diagnostics` | diagnostic fallback tests | `fallback` signal | Readiness failure diagnostics only. | Yes | Preserves operator diagnostics when registry-role reads cannot decode the preferred shape. | `live-diagnostics` | Low | `RETAIN WITH OWNER` | Removing it would weaken failure triage without simplifying mutation authority. |
| Test-only convenience imports from `mod.rs` | crate test imports | `#[cfg(test)] use ...` | test-only production-adjacent visibility | Focused install-root tests. | Yes, but not through the module root. | Tests assert install ordering, phase evidence, state hard cuts, and recovery invariants. | `overexposed-internal` | Medium | `NARROW NOW` | Fixed by importing directly from owner modules in `tests/mod.rs`. |
| Facade-owned option/identity/clock/capability helpers | `mod.rs` | `InstallRootOptions`, `resolve_install_identity`, `current_unix_secs`, `current_unix_timestamp_label`, `CURRENT_INSTALL_REQUIRED_CAPABILITIES` | module-root hub pressure | Install workflow and child modules. | Yes, but not as root-owned implementation detail. | These are live authority inputs and evidence labels, but each has a narrower owner than the top-level facade. | `overcentralized-internal` | Medium | `MOVE NOW` | Fixed by moving to `options.rs`, `identity.rs`, `clock.rs`, and `capabilities.rs`. |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| Test-only imports in `install_root::mod.rs` for command, truth-gate, operation, receipt, state, timing, staging, and readiness internals | `NARROW NOW` | These imports existed only to make the child test module's glob import convenient. Tests can access the private owner modules directly, so production module-root surface does not need to carry them. | `cargo test --locked -p canic-host install_root:: -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |
| `InstallRootOptions`, install identity resolution, unix timestamp helpers, and current-install required capabilities | `MOVE NOW` | The values are live, but the facade root does not need to own their implementation. Child modules can import focused owners without changing the public `InstallRootOptions` re-export or install behavior. | `cargo test --locked -p canic-host install_root:: -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |

## Runtime Authority Drift Check

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Mutating install flow | `install_root` resolves identity, prepares deployment truth, emits manifest receipt, runs activation phases, writes state, and writes promotion receipt. | No stronger owner inside host. | `install_root`, `prepare_install_deployment_truth`, phase operations. | Yes | Authority is centralized and ordered through pre-gate evidence. | Install ordering mistakes can mutate wrong canisters. |
| Read-only install truth | `check_install_deployment_truth` and `check_install_execution_preflight` build checks without state writes or activation mutation. | Deployment-truth crate owns lower-level check construction. | Consumer scan and ordering tests. | Yes | Read-only and mutating surfaces are separated. | Read-only drift into writes would be high severity. |
| Local deployment state | `InstallState` schema and deployment-target path under `.canic/<network>/deployments`. | Deployment catalog and installed-deployment consume but do not own schema. | `state.rs`, legacy-state tests. | Yes | Persisted state ownership is explicit. | Schema or path changes need migration proof. |
| Registration recovery | `register_deployment_state` writes minimal unverified state only with explicit acknowledgement; verification promotes only with evidence-satisfied check and digest guard. | No alternate recovery owner found. | `deployment_registration.rs`, root verification tests. | Yes | Recovery path is intentionally narrow and evidence-bound. | Weakening can bless wrong roots. |
| Deployment receipts | Phase and gate receipts are written around preflight, pre-gate, success, and failure paths. | Deployment-truth owns receipt DTOs; install_root owns local persistence. | `phase_receipts`, `receipt_io`, receipt tests. | Yes | Receipt persistence has clear owner and validation. | Receipt loss weakens auditability. |

## Complexity And Runtime Shape

| Module | Complexity Signal | Retention Justification | Dead-Surface Link | Public/Hidden Items | Current Consumers | Shrink Action | Disposition | Expected Blast Radius | Risk |
| ---- | ---- | ---- | ---- | ----: | ---- | ---- | ---- | ---- | ---- |
| `mod.rs` | Moderate orchestration fan-in after narrowing to a 181-line facade. | Root install workflow must sequence deployment truth, activation, state, receipts, and output. | Test-only module-root imports and root-owned helpers narrowed in this slice. | Public facade plus direct test owner-module imports. | CLI install/deploy plus tests. | Completed for this slice. | `RETAIN WITH OWNER` | Broad. | High install mutation, lower surface-drift risk. |
| `options.rs`, `identity.rs`, `clock.rs`, `capabilities.rs` | Small focused owner modules. | Keeps option shape, configured identity validation, evidence timestamp labels, and required executor capabilities out of the facade root. | Facade-owned helper pressure closed in this slice. | Public option DTO re-exported through facade; other helpers are `pub(super)`. | Install workflow, truth checks, execution preflight, receipts, state writers. | Created in this slice. | `RETAIN WITH OWNER` | Low to medium. | Authority inputs remain live but localized. |
| `state.rs` | Persisted schema and legacy hard-cut guard. | Local deployment state is consumed across catalog, installed-deployment, deployment truth, and CLI commands. | Legacy guard is live. | Public DTO/readers, private path helpers. | Broad. | None. | `RETAIN WITH OWNER` | Broad. | Persisted format. |
| `deployment_registration.rs` | Recovery path plus verification receipt generation. | Explicit operator recovery and evidence-bound verification are separate from normal install. | None found. | Public registration/verification API. | CLI deploy register/root. | None. | `RETAIN WITH OWNER` | Medium. | Root authority. |
| `operations/*` and `phase_receipts.rs` | Many small phase operation types. | Operation evidence and receipt-bearing phase execution are intentionally explicit and tested. | None found. | Internal-only operation surface. | Install workflow/tests. | None. | `RETAIN WITH OWNER` | Internal but high-risk. | Receipt/order semantics. |
| `config_selection.rs` | Discovery/selection helpers also feed other CLI modules. | Project/fleet config discovery is shared host operator surface. | None found. | Public discovery functions. | CLI build/fleets/scaffold/status/list and `icp_config`. | None. | `RETAIN WITH OWNER` | Medium. | Target selection. |

## Facade / Generated Boundary Review

| Surface | Boundary Type | Generated Consumer Evidence | Could Narrow? | Required Replacement | Deletion Confidence | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `pub mod install_root` from `canic-host` | Host facade | No generated consumer found. | Not safely in this slice; CLI and host modules rely on this facade. | Dedicated narrower install, state, registration, and config APIs would need a broad migration. | Low | `RETAIN WITH OWNER` | Public host install/state contract. |
| `InstallState` fields | Persisted DTO | No generated consumer found. | No; serde state is persisted and denied-unknown-fields. | Versioned migration plan and downstream state/catalog/installed-deployment proof. | Blocked | `BLOCKED` | Persisted local state. |

## Feature / Diagnostics / Test Surface Review

| Surface | Feature/Cfg | Production Consumer? | Test/Diagnostics Consumer? | Visibility Could Narrow? | Action | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Direct owner-module imports in `tests/mod.rs` | test only | No | Yes, focused install-root tests use internals to assert phase ordering, parsing, and state invariants. | Already test-only and no longer routed through module-root convenience imports. | Narrowed in this slice. | `RETAIN WITH OWNER` | Low. |
| Facade child owners | normal production modules | Yes | Yes. | Already narrowed; `InstallRootOptions` remains facade-reexported for callers while identity/clock/capability helpers stay internal. | Moved in this slice. | `RETAIN WITH OWNER` | Medium. |
| Readiness diagnostics | normal production diagnostics | Yes, on bootstrap readiness failure. | Yes. | No safe narrowing found. | None. | `RETAIN WITH OWNER` | Operator diagnosis. |
| Deployment-truth receipt discovery | normal production diagnostics | Yes, deploy resume-report and install output. | Yes. | No safe narrowing found. | None. | `RETAIN WITH OWNER` | Audit evidence. |

## Removal Safety Plan

| Candidate | Action | Disposition | Owner Boundary | Hotness | Required Proof | Focused Validation | Wasm Raw Bytes Relevant? | Follow-Up Trigger |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Test-only module-root convenience imports | Move imports to `tests/mod.rs` with direct owner-module paths. | `NARROW NOW` | `canic-host::install_root` | `test-only` | Focused tests and host clippy. | `cargo test --locked -p canic-host install_root:: -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | No | Complete. |
| Facade-owned options/identity/clock/capabilities | Move to focused child modules while preserving facade re-export for `InstallRootOptions`. | `MOVE NOW` | `canic-host::install_root` | `install-authority` | Focused tests and host clippy. | `cargo test --locked -p canic-host install_root:: -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | No | Complete. |
| Legacy fleet-state rejection | Keep. | `RETAIN WITH OWNER` | `install_root::state` | `cold`, persisted-state authority | Owner-approved removal after all supported local states are deployment-target scoped and no recovery guidance is needed. | State and deployment-catalog/installed-deployment tests. | No | Maintainer declares old fleet-state guard obsolete. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| `InstallState` field/path/schema cleanup | Persisted local state format and deployment-truth observation depend on it. | Owner-approved migration or hard-cut proof plus state/catalog/installed-deployment/deployment-truth validation. |
| Phase operation consolidation | The current shape preserves explicit operation evidence and receipt ordering. | Proof that receipts, phase ordering, and failure-before-mutation behavior remain identical. |
| Registration/verification narrowing | Recovery path is intentionally explicit and evidence-bound. | CLI deploy register/root verification migration plus root-verification tests. |

## Verification

- `cargo fmt --all`: PASS.
- `cargo test --locked -p canic-host install_root:: -- --nocapture`: PASS, 80 focused tests passed after cleanup.
- `cargo check --locked -p canic-host`: not run; focused tests compiled `canic-host`.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; host install/state audit with no runtime wasm payload change.
