# MSH Compact Audit: canic-cli deploy promote

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
| `in_scope_roots` | `crates/canic-cli/src/deploy/promote.rs`, `crates/canic-cli/src/deploy/mod.rs`, `crates/canic-cli/src/deploy/tests/promote.rs` |
| `excluded_roots` | historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | sampled direct promote tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Risk score: `3 / 10`.
- Tier: `Tier 2` because the module builds deployment-truth artifact promotion
  evidence, even though the CLI path is passive and cold.
- Patch mode: read-only.
- Cleanup result: no safe `DELETE NOW`, `NARROW NOW`, or `INLINE NOW` action
  was identified in this run.

The module is large for a leaf command module, but it has a clear current
authority: request-file based passive artifact promotion report building. The
audited path does not install canisters, query live deployment truth, stage
artifacts, call ICP/DFX, or mutate controller/deployment state.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md`; `sed -n '261,560p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for deployment-truth evidence surfaces | terminal output |
| cleanup runner review | `sed -n '1,260p' docs/audits/modular/module-cleanup-runner.md` | PASS: cleanup runner checked; no implementation cleanup requested | terminal output |
| target inventory | `wc -l crates/canic-cli/src/deploy/promote.rs crates/canic-cli/src/deploy/mod.rs crates/canic-cli/src/deploy/tests/promote.rs` | `promote.rs` is `1043` LOC; deploy root is `352` LOC; promote tests are `442` LOC | terminal output |
| surface inventory | `rg -n "pub\\(|pub(crate)|pub(super)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/deploy/promote.rs` | PASS: no dead-code allowances or stale compatibility markers; exported items are `pub(super)` within the private deploy module | terminal output |
| passive-boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|load_deployment_check|check_install_deployment_truth|resolve_current_canic_icp_root|latest_deployment_truth_receipt_path_from_root|icp|dfx|network" crates/canic-cli/src/deploy/promote.rs` | PASS: no live mutation, network, ICP/DFX, or deployment-truth observation primitives found | terminal output |
| consumer check | `rg -n "deploy_promote_|promote_usage|DeployPromoteReportOptions|promote::run|mod promote|use super::super::promote" crates/canic-cli/src -g '*.rs'` | PASS: production entry is `promote::run`; helper/usage surfaces are consumed by the module and focused tests | terminal output |
| stale-signal scan | `rg -n "allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/deploy/promote.rs crates/canic-cli/src/deploy/tests/promote.rs` | PASS: no stale compatibility markers found | terminal output |
| focused tests | `cargo test -p canic-cli deploy::tests::promote -- --nocapture` | PASS: 11 promote tests passed | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `promote::run` dispatch surface | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::deploy::promote`; this is the deploy command-family entry for passive artifact promotion report building. |
| Request-file decode structs | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::deploy::promote`; these are CLI boundary shapes that deserialize operator-provided request files before delegating to host-owned deployment-truth builders. |
| `run_output` generic report runner | `live-authority` | medium | `RETAIN WITH OWNER` | Owner: `canic-cli::deploy::promote`; it preserves one parsing/read/build/render path for all passive promotion leaves without adding live deployment authority. Trigger: revisit if a second report family needs materially different parsing or output behavior. |
| `build_*` adapter functions | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary adapter; conversions from request-file shapes to host request structs belong at this boundary and delegate immediately to `canic-host::deployment_truth`. |
| Command and usage factories | `live-authority` | medium | `DEFER WITH TRIGGER` | Owner: CLI help/test boundary. They are `pub(super)` so sibling deploy tests can assert command shape. Trigger: if promote tests move inline or a narrower test helper appears, make factories private where production no longer needs sibling visibility. |
| Advanced `inspect` namespace | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI operator UX. It intentionally keeps archived/passive promotion internals below `inspect`, and focused tests guard that advanced commands do not become top-level promotion commands. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Promotion report construction | `canic-host::deployment_truth` owns promotion evidence validation and report construction | no | CLI adapters call host builders such as `artifact_promotion_plan`, `check_promotion_readiness`, and report text renderers | yes | CLI stays a boundary adapter and renderer, not the authority for promotion invariants | Low |
| Deployment mutation | install/apply workflows own mutation; promotion command is passive | no | passive-boundary scan found no management-canister, ICP/DFX, network, or live deployment-truth observation calls | yes | no mutation authority drift found | Low |
| Request-file inputs | operator-provided JSON request files are explicit inputs | no | all leaf commands require `--request`; no implicit deployment root or network discovery found | yes | request-file-only contract is intact | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| `promote.rs` command dispatch and report adapters | `cold` | none | CLI-only parsing/rendering; no canister runtime or wasm-sensitive path | focused CLI tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Command/usage factories exposed as `pub(super)` | `cold` | possible future narrowing if tests move inline | no wasm risk; possible test-only visibility cleanup only | focused promote tests after any narrowing | `DEFER WITH TRIGGER` |

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

No required cleanup follow-up was found. Optional future cleanup: if the
promotion tests move inside `promote.rs` or gain a narrower test seam, revisit
the `pub(super)` command/usage factories and make any factory private that no
longer needs sibling-test visibility.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo test -p canic-cli deploy::tests::promote -- --nocapture` | PASS | 11 tests passed |
| `cargo check -p canic-cli` | PASS | owning package compiles |
