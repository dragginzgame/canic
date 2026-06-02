# MSH Compact Audit: canic-cli deploy external

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
| `in_scope_roots` | `crates/canic-cli/src/deploy/external/`, `crates/canic-cli/src/deploy/mod.rs`, `crates/canic-cli/src/deploy/tests/external_builders.rs`, `crates/canic-cli/src/deploy/tests/external_commands.rs` |
| `excluded_roots` | historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | sampled split external builder and command tests from current working tree |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `4 / 10`.
- Tier: `Tier 2` because the module packages deployment-truth and external
  lifecycle evidence, including verification inputs derived from supplied
  observations or embedded `DeploymentCheckV1` inventory artifacts.
- Patch mode: read-only.
- Cleanup result: no safe `DELETE NOW`, `NARROW NOW`, or `INLINE NOW` action
  was identified in this run.

The module is broad but current. It owns the `canic deploy external` passive
command family, dispatches local deployment-check derived reports, reads
explicit request files for archived/passive external lifecycle internals, and
delegates lifecycle/evidence construction to `canic_host::deployment_truth`.
The audited path does not request consent, execute external upgrades, install
code, query live inventory, mutate deployment state, or call DFX/ICP primitives.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md`; `sed -n '261,560p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for deployment-truth evidence surfaces | prior terminal output |
| current directory inventory | `rg --files crates/canic-cli/src/deploy/external \| sort` | PASS: current external module root contains `mod.rs`, `builders.rs`, `command.rs`, and `options.rs` | terminal output |
| source inspection | `sed -n '1,220p' crates/canic-cli/src/deploy/external/mod.rs`; `sed -n '1,220p' crates/canic-cli/src/deploy/external/builders.rs` | PASS: module is command dispatch, parsing, passive report rendering, request-file read adapters, and host-owned builder delegation | terminal output |
| source-scan cleanup | `rg -n "include_str!\|source_between" crates/canic-cli/src/deploy` | PASS: no deploy test relies on direct production source-file paths after cleanup | terminal output |
| focused tests | `cargo test -p canic-cli deploy::tests::external -- --nocapture` | PASS: 20 external tests passed after removing brittle source-scan test | terminal output |
| deploy test suite | `cargo test -p canic-cli deploy::tests -- --nocapture` | PASS: 84 deploy tests passed | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `external::run` dispatch surface | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::deploy::external`; this is the deploy command-family entry for passive external lifecycle report construction. |
| Local deployment-check report leaves | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI passive report boundary. These leaves derive local external lifecycle evidence from `DeploymentCheckV1` and host-owned report builders; they do not execute lifecycle work. |
| Request-file leaves | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI request-file boundary. These leaves deserialize explicit operator-provided request files and delegate validation/construction to host-owned external lifecycle helpers. |
| `build_*` adapter functions | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary adapter. The functions assign local artifact IDs and call `canic_host::deployment_truth`; lifecycle invariants remain host-owned. |
| Verification-check source arbitration | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI request adapter. Verification check construction rejects ambiguous observation sources and validates supplied observations or deployment-truth inventory-backed checks through host validators. |
| Command and usage factories | `live-authority` | medium | `DEFER WITH TRIGGER` | Owner: CLI help/test boundary. They remain `pub(super)` for sibling deploy tests. Trigger: if tests move inline or a narrower test helper appears, make factories private where production no longer needs sibling visibility. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| External lifecycle construction | `canic_host::deployment_truth` owns lifecycle report, proposal, consent, verification, and completion construction | no | module imports host DTOs/builders/validators and delegates each `build_*` adapter to them | yes | CLI remains a boundary adapter, local ID assigner, and renderer; no duplicate lifecycle authority found | Low |
| Deployment-check observation | shared deploy loader owns local deployment-truth check loading for deployment-derived leaves | no | deployment-derived leaves call `load_deployment_check`; request-file inspect leaves require explicit `--request` | yes | deployment-derived and request-file-derived paths stay explicit | Low |
| External execution and consent | external controllers/operators own actual external execution and consent delivery | no | help text and focused tests state commands do not request consent, execute external upgrades, or mutate state | yes | no execution authority drift found | Low |
| Deployment mutation | install/register workflows own mutation; external command is passive | no | command tests assert passive dispatch/help, and package validation passes | yes | no mutation authority drift found | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| `deploy/external/` command and builders | `cold` | none | CLI-only parsing/rendering/request-file reading; no canister runtime or wasm-sensitive path | focused CLI tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Command/usage factories exposed as `pub(super)` | `cold` | possible future narrowing if tests move inline | no wasm risk; possible test-only visibility cleanup only | focused external command tests after any narrowing | `DEFER WITH TRIGGER` |

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
external tests move inside the module or gain a narrower test seam, revisit the
`pub(super)` command/usage factories and make any factory private that no longer
needs sibling-test visibility.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo test -p canic-cli deploy::tests::external -- --nocapture` | PASS | 20 external tests passed |
| `cargo test -p canic-cli deploy::tests -- --nocapture` | PASS | 84 deploy tests passed |
| `cargo check -p canic-cli` | PASS | owning package compiles |
