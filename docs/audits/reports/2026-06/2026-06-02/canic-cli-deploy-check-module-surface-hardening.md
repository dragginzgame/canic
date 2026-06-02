# MSH Compact Audit: canic-cli deploy check

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
| `in_scope_roots` | `crates/canic-cli/src/deploy/check.rs`, `crates/canic-cli/src/deploy/mod.rs`, `crates/canic-cli/src/deploy/tests/deploy_check.rs` |
| `excluded_roots` | historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | sampled direct deploy-check tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Risk score: `3 / 10`.
- Tier: `Tier 2` because the module packages deployment-check evidence and
  calls the shared local deployment-truth loader.
- Patch mode: read-only.
- Cleanup result: no safe `DELETE NOW`, `NARROW NOW`, or `INLINE NOW` action
  was identified in this run.

The module is a current CLI evidence surface. It parses `canic deploy check`,
loads a local `DeploymentCheckV1`, optionally wraps it in the stable evidence
envelope, and enforces blocked deployment-check status. The audited path has
read authority for local deployment truth and input fingerprints, but no install,
state registration, management-canister mutation, ICP/DFX call, or live network
operation was found in `check.rs`. The numeric score is low because every
authority-drift row is `Low`, no stale surface was found, the module is cold,
and the only deferred item is test-boundary visibility for command/usage
factories.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md`; `sed -n '261,560p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for deployment-truth evidence surfaces | prior terminal output |
| cleanup runner review | `sed -n '1,260p' docs/audits/modular/module-cleanup-runner.md` | PASS: cleanup runner checked; no implementation cleanup requested | prior terminal output |
| target inventory | `wc -l crates/canic-cli/src/deploy/check.rs crates/canic-cli/src/deploy/mod.rs crates/canic-cli/src/deploy/tests/deploy_check.rs` | `check.rs` is `385` LOC; deploy root is `352` LOC; deploy-check tests are `312` LOC | terminal output |
| surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/deploy/check.rs` | PASS: no dead-code allowances or stale compatibility markers; exported items are `pub(super)` within the private deploy module | terminal output |
| read/mutation boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|load_deployment_check|check_install_deployment_truth|resolve_current_canic_icp_root|latest_deployment_truth_receipt_path_from_root|icp|dfx|network|fs::|File|write|remove|metadata" crates/canic-cli/src/deploy/check.rs` | PASS with expected read signals: `load_deployment_check`, network value rendering, and `fs::metadata` for source-config fingerprinting; no write or mutation primitive found | terminal output |
| shared loader review | `sed -n '120,190p' crates/canic-cli/src/deploy/mod.rs`; `sed -n '190,280p' crates/canic-cli/src/deploy/mod.rs` | PASS: dispatch routes to `check::run`; shared loader calls `check_install_deployment_truth` with current ICP root resolution and no local install/register write path in inspected slice | terminal output |
| consumer check | `rg -n "deploy_check|DeployCheckOptions|build_deployment_check_envelope|enforce_deployment_check_status|check::run|mod check|use super::super::check" crates/canic-cli/src -g '*.rs'` | PASS: production entry is `check::run`; direct helper surfaces are consumed by focused deploy-check tests | terminal output |
| stale-signal scan | `rg -n "allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/deploy/check.rs crates/canic-cli/src/deploy/tests/deploy_check.rs` | PASS: no stale compatibility markers found | terminal output |
| focused tests | `cargo test -p canic-cli deploy::tests::deploy_check -- --nocapture` | PASS: 13 deploy-check tests passed | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `check::run` dispatch surface | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::deploy::check`; this is the deploy command-family entry for local deployment-check evidence output and blocked-status enforcement. |
| `build_deployment_check_envelope` | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI evidence-envelope boundary. It preserves the stable envelope contract for CI/GitOps without adding deployment mutation authority. |
| Source config and build-provenance fingerprint helpers | `live-authority` | medium | `RETAIN WITH OWNER` | Owner: evidence-envelope boundary. These helpers read file metadata/content fingerprints only; they do not write local deployment state. |
| `enforce_deployment_check_status` | `live-authority` | high | `RETAIN WITH OWNER` | Owner: deploy-check CLI status boundary. It converts a blocked local safety report into the CLI blocked error and allows warning/safe reports. |
| `DeployCheckOptions::parse` and option DTO | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary. Parsing enforces that `--build-provenance` is only valid with `--format envelope-json`, matching the evidence-envelope contract. |
| Command and usage factories | `live-authority` | medium | `DEFER WITH TRIGGER` | Owner: CLI help/test boundary. They are `pub(super)` so sibling deploy tests can assert command shape. Trigger: if deploy-check tests move inline or a narrower test helper appears, make factories private where production no longer needs sibling visibility. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Deployment-check construction | shared deploy loader delegates to `canic-host` deployment-truth checking | no | `check::run` calls `load_deployment_check`; deploy root loader calls `check_install_deployment_truth` | yes | CLI remains the boundary adapter, renderer, and status enforcer; host logic remains the check authority | Low |
| Evidence-envelope construction | `canic_host::evidence_envelope` owns envelope DTOs, schemas, fingerprints, and exit-class helpers | no | module imports schema/fingerprint helpers and constructs the envelope with host DTOs | yes | no parallel envelope schema or ad hoc hash contract found | Low |
| Deployment mutation | install/register workflows own mutation; deploy check is evidence-only | no | boundary scan and focused test `deploy_check_path_has_no_local_state_write_primitives` cover install/register/write tokens | yes | no mutation authority drift found | Low |
| Input file reads | operator-provided build provenance and recorded config path are fingerprinted | no | `fs::metadata` and `file_input_fingerprint` are the only file read signals in the module | yes | read-only evidence collection is explicit and bounded | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| `check.rs` command dispatch and envelope construction | `cold` | none | CLI-only parsing/rendering/fingerprinting; no canister runtime or wasm-sensitive path | focused CLI tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Command/usage factories exposed as `pub(super)` | `cold` | possible future narrowing if tests move inline | no wasm risk; possible test-only visibility cleanup only | focused deploy-check tests after any narrowing | `DEFER WITH TRIGGER` |

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
deploy-check tests move inside `check.rs` or gain a narrower test seam, revisit
the `pub(super)` command/usage factories and make any factory private that no
longer needs sibling-test visibility.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo test -p canic-cli deploy::tests::deploy_check -- --nocapture` | PASS | 13 tests passed |
| `cargo check -p canic-cli` | PASS | owning package compiles |
