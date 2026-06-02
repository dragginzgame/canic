# MSH Compact Audit: canic-cli deploy root

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
| `in_scope_roots` | `crates/canic-cli/src/deploy/root.rs`, `crates/canic-cli/src/deploy/mod.rs`, `crates/canic-cli/src/deploy/tests/root.rs` |
| `excluded_roots` | historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | sampled focused root tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `4 / 10`.
- Tier: `Tier 2` because the module owns deployment-root evidence inspection
  and an explicit verified-root state transition.
- Patch mode: read-only.
- Cleanup result: no safe `DELETE NOW`, `NARROW NOW`, or `INLINE NOW` action
  was identified in this run.

The module is current and intentionally narrow. `inspect` reads an explicit
`DeploymentRootVerificationRequestV1` file and builds a passive report through
host-owned deployment-truth helpers. `verify` reads an explicit
`DeploymentCheckV1` artifact and delegates the registered-root state transition
to `canic_host::install_root::verify_registered_deployment_root`. The audited
path does not install code, mutate canisters/controllers, or call DFX/ICP
primitives. The score is higher than the fully passive deploy leaves because
`verify` records verified-root state by design.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md`; `sed -n '261,560p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for deployment-truth evidence and install/root state surfaces | prior terminal output |
| target inventory | `wc -l crates/canic-cli/src/deploy/root.rs crates/canic-cli/src/deploy/mod.rs crates/canic-cli/src/deploy/tests/root.rs` | `root.rs` is `270` LOC; deploy root is `352` LOC; root tests are `163` LOC | terminal output |
| source inspection | `sed -n '1,220p' crates/canic-cli/src/deploy/root.rs`; `sed -n '221,340p' crates/canic-cli/src/deploy/root.rs` | PASS: module is root inspect/verify command dispatch, request-file parsing, passive report rendering, and host-owned state-transition delegation | terminal output |
| focused test inspection | `sed -n '1,220p' crates/canic-cli/src/deploy/tests/root.rs` | PASS: tests cover inspect/verify parsing, help boundaries, dispatch, and host-backed report construction | terminal output |
| surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/deploy/root.rs` | PASS: no dead-code allowances or stale compatibility markers; exported items are `pub(super)` within the private deploy module | terminal output |
| authority boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|load_deployment_check|check_install_deployment_truth|resolve_current_canic_icp_root|latest_deployment_truth_receipt_path_from_root|icp|dfx|network|register_deployment_state|install_root|write_install_state|verify_registered_deployment_root|read_json_file|fs::|metadata" crates/canic-cli/src/deploy/root.rs` | PASS with expected state signals: `read_json_file`, `verify_registered_deployment_root`, `resolve_current_canic_icp_root`, and network parsing; no canister/controller mutation primitive found | terminal output |
| consumer check | `rg -n "deploy_root|DeployRoot|build_verification_report|root::run|mod root|use super::super::root" crates/canic-cli/src -g '*.rs'` | PASS: production entry is `root::run`; helper surfaces are consumed by focused root tests | terminal output |
| stale-signal scan | `rg -n "allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME|include_str!|source_between" crates/canic-cli/src/deploy/root.rs crates/canic-cli/src/deploy/tests/root.rs` | PASS: no stale markers or direct source-file scan tests found | terminal output |
| focused tests | `cargo test -p canic-cli deploy::tests::root -- --nocapture` | PASS: 8 root tests passed | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `root::run` dispatch surface | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::deploy::root`; this is the deploy command-family entry for deployment-root inspect and verify flows. |
| `inspect` request-file flow | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI passive evidence boundary. It reads an explicit verification request file and delegates report construction/validation to host-owned deployment-truth helpers. |
| `verify` registered-root flow | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary for the explicit root state transition. It reads an explicit deployment-check artifact and delegates the state write to `canic_host::install_root`. |
| `build_verification_report` | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary adapter. It wraps host report construction and validation without owning deployment-root invariants. |
| Option DTOs and parsers | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary. They keep passive inspect and stateful verify inputs explicit, including `--request`, `--from-check`, and internal network selection. |
| Command and usage factories | `live-authority` | medium | `DEFER WITH TRIGGER` | Owner: CLI help/test boundary. They are `pub(super)` so sibling deploy tests can assert command shape and boundary wording. Trigger: if root tests move inline or a narrower test helper appears, make factories private where production no longer needs sibling visibility. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Deployment-root report construction | `canic_host::deployment_truth` owns verification report construction and validation | no | `build_verification_report` calls host report construction and validation helpers | yes | CLI remains a request adapter and renderer, not the report authority | Low |
| Verified-root state transition | `canic_host::install_root::verify_registered_deployment_root` owns the registered-root state write | no | `run_verify` constructs `VerifyDeploymentRootOptions` and delegates immediately | yes | state mutation authority remains in host install-root logic | Medium |
| Canister/controller mutation | install/controller workflows own canister and controller mutation | no | boundary scan found no management-canister, DFX, install-code, or controller mutation primitives | yes | no canister/controller mutation drift found | Low |
| Input provenance | operator-provided JSON artifacts are explicit inputs | no | `inspect` requires `--request`; `verify` requires `<deployment>` and `--from-check` | yes | root evidence source boundary is explicit | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| `root.rs` command dispatch and adapters | `cold` | none | CLI-only parsing/rendering/request-file reading; no canister runtime or wasm-sensitive path | focused CLI tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Verify state-transition adapter | `cold` | none | state authority is delegated to host; inlining would blur CLI/host ownership | focused root tests and host install-root tests for behavior changes | `RETAIN WITH OWNER` |
| Command/usage factories exposed as `pub(super)` | `cold` | possible future narrowing if tests move inline | no wasm risk; possible test-only visibility cleanup only | focused root tests after any narrowing | `DEFER WITH TRIGGER` |

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

No required cleanup follow-up was found. Optional future cleanup: if the root
tests move inside `root.rs` or gain a narrower test seam, revisit the
`pub(super)` command/usage factories and make any factory private that no longer
needs sibling-test visibility.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo test -p canic-cli deploy::tests::root -- --nocapture` | PASS | 8 tests passed |
| `cargo check -p canic-cli` | PASS | owning package compiles |
