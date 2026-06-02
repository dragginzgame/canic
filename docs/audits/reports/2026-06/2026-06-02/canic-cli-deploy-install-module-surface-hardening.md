# MSH Compact Audit: canic-cli deploy install

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
| `in_scope_roots` | `crates/canic-cli/src/deploy/install.rs`, `crates/canic-cli/src/deploy/mod.rs`, `crates/canic-cli/src/deploy/tests/install.rs` |
| `excluded_roots` | historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | sampled focused install tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `5 / 10`.
- Tier: `Tier 2` because the module is the CLI boundary into the active install
  runner.
- Patch mode: read-only.
- Cleanup result: no safe `DELETE NOW`, `NARROW NOW`, or `INLINE NOW` action
  was identified in this run.

The module is small and intentionally active. It parses
`canic deploy install <deployment> --plan <file>`, reads either a
`DeploymentPlanV1` or ready `ArtifactPromotionPlanV1`, converts that explicit
plan artifact into `InstallRootOptions`, and delegates mutation to
`canic_host::install_root::install_root`. The CLI module does not directly call
management-canister primitives, DFX/ICP commands, registration writes, or
promotion execution paths. The score is higher than passive deploy modules
because successful execution intentionally installs through the current runner.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md`; `sed -n '261,560p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for install/upgrade surfaces | prior terminal output |
| target inventory | `wc -l crates/canic-cli/src/deploy/install.rs crates/canic-cli/src/deploy/mod.rs crates/canic-cli/src/deploy/tests/install.rs` | `install.rs` is `195` LOC; deploy root is `352` LOC; install tests are `112` LOC | terminal output |
| source inspection | `sed -n '1,240p' crates/canic-cli/src/deploy/install.rs` | PASS: module reads explicit plan input, validates promotion readiness, builds install options, and delegates to host install root | terminal output |
| focused test inspection | `sed -n '1,180p' crates/canic-cli/src/deploy/tests/install.rs` | PASS: tests cover dispatch, install option construction, raw deployment plan decode, ready promotion plan decode, and blocked promotion rejection | terminal output |
| dispatch inspection | `sed -n '120,170p' crates/canic-cli/src/deploy/mod.rs` | PASS: deploy root dispatch routes `install` to `install::run` | terminal output |
| surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/deploy/install.rs` | PASS: no dead-code allowances or stale compatibility markers; exported items are `pub(super)` within the private deploy module | terminal output |
| authority boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|load_deployment_check|check_install_deployment_truth|resolve_current_canic_icp_root|latest_deployment_truth_receipt_path_from_root|icp|dfx|network|register_deployment_state|install_root|write_install_state|verify_registered_deployment_root|read_json_file|fs::|metadata|ArtifactPromotion|DeploymentPlan" crates/canic-cli/src/deploy/install.rs` | PASS with expected active signals: `install_root`, `resolve_current_canic_icp_root`, `fs::read`, plan DTOs, and network parsing; no direct canister/controller primitive or DFX/ICP command found | terminal output |
| consumer check | `rg -n "deploy_install|DeployInstall|read_plan|install::run|mod install|use super::super::install" crates/canic-cli/src -g '*.rs'` | PASS: production entry is `install::run`; helper surfaces are consumed by focused install tests | terminal output |
| stale-signal scan | `rg -n "allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME|include_str!|source_between" crates/canic-cli/src/deploy/install.rs crates/canic-cli/src/deploy/tests/install.rs` | PASS: no stale markers or direct source-file scan tests found | terminal output |
| focused tests | `cargo test -p canic-cli deploy::tests::install -- --nocapture` | PASS: 5 install tests passed | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `install::run` dispatch surface | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::deploy::install`; this is the explicit deploy command-family entry for current-runner install execution. |
| Plan-file reader | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI input boundary. It decodes explicit `DeploymentPlanV1` or ready `ArtifactPromotionPlanV1` artifacts and rejects blocked promotion plans before mutation. |
| `DeployInstallPlanOptions::into_install_root_options` | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI-to-host adapter. It maps explicit deployment, network, profile, ICP root, and plan overrides into host `InstallRootOptions`. |
| Root canister selection from plan | `live-authority` | medium | `RETAIN WITH OWNER` | Owner: CLI adapter. The fallback chain uses trust-domain root anchor, deployment root principal, expected root canister, then default target. |
| `install_root` delegation | `live-authority` | high | `RETAIN WITH OWNER` | Owner: host install runner. The CLI does not own install mutation; it delegates to `canic_host::install_root::install_root`. |
| Command and usage factories | `live-authority` | medium | `DEFER WITH TRIGGER` | Owner: CLI help/test boundary. They are `pub(super)` so sibling deploy tests can assert command shape. Trigger: if install tests move inline or a narrower test helper appears, make factories private where production no longer needs sibling visibility. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Install mutation | `canic_host::install_root::install_root` owns install/preflight/activation mutation | no | `run` reads plan and delegates immediately to `install_root` with `InstallRootOptions` | yes | CLI remains the input adapter, not a parallel install authority | Medium |
| Promotion readiness | host deployment-truth validation owns promotion plan validation; CLI gates non-ready plans before install | no | `read_plan` calls `validate_artifact_promotion_plan` and rejects non-`Ready` status | yes | promotion artifacts are not executed when blocked | Low |
| Direct canister/controller mutation | host install runner owns management-canister and controller side effects | no | boundary scan found no direct `install_code`, `create_canister`, controller update, or DFX primitive in the CLI module | yes | no direct mutation drift found | Low |
| Input provenance | operator-provided plan artifact is explicit input | no | command requires `<deployment>` and `--plan <file>` | yes | install source boundary is explicit | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| `install.rs` command dispatch and plan adapter | `cold` | none | CLI-only parsing/read/decode before host delegation; no canister runtime or wasm-sensitive path | focused CLI tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Plan decode fallback | `cold` | none | order matters because ready promotion envelopes carry both promoted plan and promotion context | focused install plan-reader tests | `RETAIN WITH OWNER` |
| Command/usage factories exposed as `pub(super)` | `cold` | possible future narrowing if tests move inline | no wasm risk; possible test-only visibility cleanup only | focused install tests after any narrowing | `DEFER WITH TRIGGER` |

## Disposition Ledger

| Disposition | Count |
| ---- | ----: |
| DELETE NOW | 0 |
| NARROW NOW | 0 |
| INLINE NOW | 0 |
| MOVE OWNER | 0 |
| MOVE TO TEST | 0 |
| RETAIN WITH OWNER | 5 |
| DEFER WITH TRIGGER | 1 |
| MEASURE FIRST | 0 |
| BLOCKED | 0 |

## Follow-up

No required cleanup follow-up was found. Optional future cleanup: if the install
tests move inside `install.rs` or gain a narrower test seam, revisit the
`pub(super)` command/usage factories and make any factory private that no longer
needs sibling-test visibility.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo test -p canic-cli deploy::tests::install -- --nocapture` | PASS | 5 tests passed |
| `cargo check -p canic-cli` | PASS | owning package compiles |
