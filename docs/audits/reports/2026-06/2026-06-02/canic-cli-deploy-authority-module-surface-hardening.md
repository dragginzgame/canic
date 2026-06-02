# MSH Compact Audit: canic-cli deploy authority

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
| `in_scope_roots` | `crates/canic-cli/src/deploy/authority.rs`, `crates/canic-cli/src/deploy/mod.rs`, `crates/canic-cli/src/deploy/tests/authority.rs` |
| `excluded_roots` | historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | sampled focused authority tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `3 / 10`.
- Tier: `Tier 2` because the module owns controller-authority dry-run evidence
  and deployment-truth-derived authority reports.
- Patch mode: read-only.
- Cleanup result: no safe `DELETE NOW`, `NARROW NOW`, or `INLINE NOW` action
  was identified in this run.

The module is compact and current. It owns the `canic deploy authority` dry-run
command family, loads local deployment truth through the shared deploy loader,
and delegates authority plan/report/evidence/receipt construction to
`canic_host::deployment_truth`. The audited path does not apply controller
changes, install code, mutate deployment state, or call DFX/ICP primitives. The
numeric score stays low because the surface is cold CLI code, all authority
drift rows are low risk, no stale markers were found, and the only deferred item
is test-boundary visibility for command/usage factories.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md`; `sed -n '261,560p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for authority reconciliation surfaces | prior terminal output |
| target inventory | `wc -l crates/canic-cli/src/deploy/authority.rs crates/canic-cli/src/deploy/mod.rs crates/canic-cli/src/deploy/tests/authority.rs` | `authority.rs` is `307` LOC; deploy root is `352` LOC; authority tests are `376` LOC | terminal output |
| source inspection | `sed -n '1,220p' crates/canic-cli/src/deploy/authority.rs`; `sed -n '221,380p' crates/canic-cli/src/deploy/authority.rs` | PASS: module is dry-run command dispatch, parsing, passive report rendering, and host-owned builder delegation | terminal output |
| focused test inspection | `sed -n '1,220p' crates/canic-cli/src/deploy/tests/authority.rs`; `sed -n '221,430p' crates/canic-cli/src/deploy/tests/authority.rs` | PASS: tests cover parsing, help text, dispatch, host-backed local IDs, receipt zero-attempt semantics, and invalid formats | terminal output |
| surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/deploy/authority.rs` | PASS: no dead-code allowances or stale compatibility markers; exported items are `pub(super)` within the private deploy module | terminal output |
| read/mutation boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|load_deployment_check|check_install_deployment_truth|resolve_current_canic_icp_root|latest_deployment_truth_receipt_path_from_root|icp|dfx|network|register_deployment_state|install_root|write_install_state|read_json_file|fs::|metadata" crates/canic-cli/src/deploy/authority.rs` | PASS with expected read signal: `load_deployment_check` and help-text network examples; no mutation primitive found | terminal output |
| consumer check | `rg -n "deploy_authority|DeployAuthorityOptions|build_dry_run|authority::run|mod authority|use super::super::authority" crates/canic-cli/src -g '*.rs'` | PASS: production entry is `authority::run`; helper surfaces are consumed by focused authority tests | terminal output |
| stale-signal scan | `rg -n "allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME|include_str!|source_between" crates/canic-cli/src/deploy/authority.rs crates/canic-cli/src/deploy/tests/authority.rs` | PASS: no stale markers or direct source-file scan tests found | terminal output |
| focused tests | `cargo test -p canic-cli deploy::tests::authority -- --nocapture` | PASS: 14 authority tests passed | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `authority::run` dispatch surface | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::deploy::authority`; this is the deploy command-family entry for dry-run authority reports. |
| Local deployment-check authority leaves: `check`, `evidence`, `report`, `receipt` | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI dry-run report boundary. The leaves derive authority artifacts from local `DeploymentCheckV1` and do not attempt controller mutation. |
| `run_output` generic report runner | `live-authority` | medium | `RETAIN WITH OWNER` | Owner: `canic-cli::deploy::authority`; it preserves one parse/load/build/render path for the dry-run authority leaves. |
| `build_dry_run_evidence` and `build_dry_run_receipt` | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary adapter. These functions add current generated timestamps and delegate evidence/receipt construction to host-owned helpers. |
| Option DTO and parser | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary. Parsing keeps output format and deployment truth selector handling explicit. |
| Command and usage factories | `live-authority` | medium | `DEFER WITH TRIGGER` | Owner: CLI help/test boundary. They are `pub(super)` so sibling deploy tests can assert command shape and dry-run help text. Trigger: if authority tests move inline or a narrower test helper appears, make factories private where production no longer needs sibling visibility. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Authority reconciliation construction | `canic_host::deployment_truth` owns authority plan/report/evidence/receipt construction | no | module imports host DTOs/builders/renderers and delegates construction to them | yes | CLI remains a boundary adapter, timestamp provider, and renderer; no duplicate authority logic found | Low |
| Deployment-check observation | shared deploy loader owns local deployment-truth check loading | no | `run_output` calls `load_deployment_check` before building authority artifacts | yes | local deployment truth remains the input authority | Low |
| Controller mutation | controller mutation belongs to explicit mutation workflows, not dry-run authority reports | no | help text states no controller changes are attempted; boundary scan found no management-canister or DFX primitive | yes | no mutation authority drift found | Low |
| Receipt semantics | host-owned dry-run receipt owns attempted-action reporting | no | focused tests assert receipt local IDs and empty attempted actions | yes | receipt remains evidence-only | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| `authority.rs` command dispatch and report adapters | `cold` | none | CLI-only parsing/rendering; no canister runtime or wasm-sensitive path | focused CLI tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Command/usage factories exposed as `pub(super)` | `cold` | possible future narrowing if tests move inline | no wasm risk; possible test-only visibility cleanup only | focused authority tests after any narrowing | `DEFER WITH TRIGGER` |

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
authority tests move inside `authority.rs` or gain a narrower test seam, revisit
the `pub(super)` command/usage factories and make any factory private that no
longer needs sibling-test visibility.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo test -p canic-cli deploy::tests::authority -- --nocapture` | PASS | 14 tests passed |
| `cargo check -p canic-cli` | PASS | owning package compiles |
