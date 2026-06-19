# MSH Module Cleanup: canic-cli evidence_support

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
| `code_snapshot` | `3d503513` |
| `in_scope_roots` | `crates/canic-cli/src/evidence_support.rs` |
| `excluded_roots` | host evidence-envelope authority, deployment truth construction, historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused evidence-support tests plus consuming evidence command filters |
| `audit_tier` | `Tier 1` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `2 / 10`.
- Tier: `Tier 1`, because this helper participates in evidence command
  provenance but owns only passive argument rendering/redaction support.
- Cleanup result: the optional path helper now accepts `Option<&Path>` instead
  of `Option<&PathBuf>`, and focused tests cover absent inputs plus absolute
  path redaction.

The module remains a small evidence-support adapter. It appends optional
path-valued flags to normalized command provenance and records redactions when
host-owned `command_path_for_root` reports an absolute path outside the config
root. It does not build evidence envelopes, choose schemas, read files, write
files, inspect deployment truth, call ICP/DFX, or mutate state.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| runner review | `sed -n '1,240p' docs/audits/modular/module-cleanup-runner.md` | PASS: implementation-requested cleanup runner already reviewed for this day | terminal output |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules already reviewed for this day | terminal output |
| target inventory | `wc -l crates/canic-cli/src/evidence_support.rs` | PASS: helper was `19` LOC before patch | terminal output |
| consumer inventory | `rg -n "push_optional_path_arg\\(" crates/canic-cli/src -g '*.rs'` | PASS: seven callers in deploy-check and fleet adoption evidence provenance | terminal output |
| stale-signal scan | `rg -n "allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|doc\\(hidden\\)|TODO|FIXME" crates/canic-cli/src/evidence_support.rs -g '*.rs'` | PASS: no stale compatibility or dead-code markers found | terminal output |
| focused tests | `cargo test --locked -p canic-cli evidence_support -- --nocapture` | PASS: 2 tests passed | terminal output |
| consuming tests | `cargo test --locked -p canic-cli adoption_report -- --nocapture`; `cargo test --locked -p canic-cli deploy_check -- --nocapture` | PASS: adoption and deploy-check evidence command filters passed | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| `push_optional_path_arg` path parameter | `NARROW NOW` from `Option<&PathBuf>` to `Option<&Path>` | The helper only needs path semantics for host-owned display/redaction logic. Callers pass `.as_deref()`, preserving all existing normalized argv behavior while avoiding an unnecessary concrete buffer requirement. | focused evidence-support, adoption-report, deploy-check tests; package check; clippy |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `push_optional_path_arg` | `canic-cli::evidence_support` | Shared command-provenance helper for optional evidence input paths and redaction tracking. | Revisit if command provenance assembly moves fully into host evidence-envelope helpers. |
| Redaction string shape | `canic-cli::evidence_support` with host path display authority | It records which flag used an absolute path outside the config root without exposing the path. | Revisit only with an evidence-envelope redaction schema change. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Host evidence path rendering | Owned by `canic-host::evidence_envelope::command_path_for_root`, not this helper. | Separate host evidence-envelope MSH pass. |

## Verification

- `cargo fmt --all -- --check`: PASS.
- `cargo test --locked -p canic-cli evidence_support -- --nocapture`: PASS, 2 tests passed.
- `cargo test --locked -p canic-cli adoption_report -- --nocapture`: PASS, 18 tests passed.
- `cargo test --locked -p canic-cli deploy_check -- --nocapture`: PASS, 14 tests passed.
- `cargo check --locked -p canic-cli`: PASS.
- `cargo clippy --locked -p canic-cli --all-targets --all-features -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; CLI-only evidence provenance helper cleanup.
