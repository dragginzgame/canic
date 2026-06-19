# MSH Module Cleanup: canic-cli evidence

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
| `code_snapshot` | `0.68.24 working tree` |
| `in_scope_roots` | `crates/canic-cli/src/evidence.rs`, `crates/canic-cli/src/evidence/command.rs` |
| `excluded_roots` | evidence compare/gate implementation internals, host policy-gate authority, historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused evidence tests |
| `audit_tier` | `Tier 1` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `3 / 10`.
- Tier: `Tier 1`, because this module owns `canic evidence` dispatch and
  public command errors while leaf modules own evidence comparison and policy
  evaluation behavior.
- Cleanup result: split the inline `compare` and `gate` match arms into
  `run_compare` and `run_gate`, leaving `run` as a thin dispatcher.

The module remains a passive CLI command group boundary. It handles top-level
help/version behavior, selects the evidence leaf command, maps command errors,
and enforces existing non-zero outcomes after leaf reports are written. It does
not own policy evaluation, evidence-envelope comparison semantics, file IO
formatting, host evidence-envelope construction, deployment truth, or mutation.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| runner review | `sed -n '1,240p' docs/audits/modular/module-cleanup-runner.md` | PASS: implementation-requested cleanup runner already reviewed for this day | terminal output |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules already reviewed for this day | terminal output |
| target inventory | `wc -l crates/canic-cli/src/evidence.rs crates/canic-cli/src/evidence/*.rs` | PASS: dispatcher plus evidence child modules inspected for boundaries | terminal output |
| stale-signal scan | `rg -n "evidence::|EvidenceCommandError|parse_evidence|canic evidence|allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|doc\\(hidden\\)|TODO|FIXME|pub\\(|pub " crates/canic-cli/src/evidence.rs crates/canic-cli/src/evidence -g '*.rs'` | PASS: no stale compatibility or dead-code markers found in dispatcher scope | terminal output |
| focused tests | `cargo test --locked -p canic-cli evidence -- --nocapture` | PASS: 23 filtered tests passed | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| Inline `compare` dispatch body | `MOVE OWNER` into `run_compare` | Compare-specific help, option parsing, report writing, and difference error mapping now live in one focused helper while preserving call order. | focused evidence tests; package check; clippy |
| Inline `gate` dispatch body | `MOVE OWNER` into `run_gate` | Gate-specific help, option parsing, report writing, and failure mapping now live in one focused helper while preserving call order. | focused evidence tests; package check; clippy |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `EvidenceCommandError` | `canic-cli::evidence` | Public command error surface for evidence compare/gate command outcomes. | Revisit only if command errors move to a shared CLI error model. |
| `run` dispatcher | `canic-cli::evidence` | Owns selecting the evidence leaf command after top-level evidence parsing. | Revisit if evidence leaves become top-level commands. |
| `evidence::command` builders | `canic-cli::evidence::command` | Passive clap/usage construction for the evidence group and leaf commands. | Revisit with a CLI help or command-shape redesign. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Compare/gate semantic cleanup | Out of scope; leaf modules own evidence comparison, policy gate evaluation, report rendering, and file IO. | Dedicated compare/gate module MSH passes. |

## Verification

- `cargo fmt --all -- --check`: PASS.
- `cargo test --locked -p canic-cli evidence -- --nocapture`: PASS, 23 filtered tests passed.
- `cargo check --locked -p canic-cli`: PASS.
- `cargo clippy --locked -p canic-cli --all-targets --all-features -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; CLI-only dispatcher cleanup.
