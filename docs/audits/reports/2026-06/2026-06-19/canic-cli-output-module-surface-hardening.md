# MSH Module Cleanup: canic-cli output

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
| `in_scope_roots` | `crates/canic-cli/src/output/` |
| `excluded_roots` | backup/restore recovery workflows, historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused output tests |
| `audit_tier` | `Tier 1` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `2 / 10`.
- Tier: `Tier 1`, because this module owns shared CLI file/stdout output and
  JSON decoding helpers, but does not own backup/recovery authority,
  deployment mutation, stable storage, wasm, or generated code.
- Cleanup result: one parent-directory edge case was hardened and covered by a
  focused regression test. Visibility and `&Path` narrowing were rejected after
  clippy and scope review showed they either added no value inside a private
  module or pulled restore IO cleanup into this slice.

The module remains a passive CLI IO helper. It writes pretty JSON or text to a
requested output file or stdout, creates non-empty parent directories for output
files, and reads JSON files for callers that own the command authority. It does
not choose what to write, mutate deployment state, call ICP/DFX, or own
backup/restore semantics.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| runner review | `sed -n '1,240p' docs/audits/modular/module-cleanup-runner.md` | PASS: implementation-requested cleanup runner already reviewed for this day | terminal output |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules already reviewed for this day | terminal output |
| target inventory | `wc -l crates/canic-cli/src/output/*.rs` | PASS: module totals `93` LOC across helper and tests before patch | terminal output |
| public surface inventory | `rg -n "crate::output|output::|write_pretty_json|write_pretty_json_file|write_text|read_json_file" crates/canic-cli/src -g '*.rs'` | PASS: consumers are internal CLI modules; output does not cross the public facade because `mod output` is private | terminal output |
| stale-signal scan | `rg -n "allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|doc\\(hidden\\)|TODO|FIXME" crates/canic-cli/src/output -g '*.rs'` | PASS: no stale compatibility or dead-code markers found | terminal output |
| authority scan | `rg -n "fs::|File::|write\\(|writeln!|println!|stdout|stderr|create_dir|remove|rename|copy" crates/canic-cli/src/output crates/canic-cli/src -g '*.rs'` | PASS: output owns file/stdout IO only; no canister, deployment, or ICP mutation found in this module | terminal output |
| focused tests | `cargo test --locked -p canic-cli output -- --nocapture` | PASS: 22 filtered tests passed, including the new plain filename regression | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| `ensure_parent_dir` empty-parent handling | `PATCH WITH PROOF` | Plain relative output names such as `summary.json` have no useful parent directory to create. Skipping an empty parent preserves nested-directory behavior and avoids treating current-directory outputs as directory creation requests. | `cargo test --locked -p canic-cli output -- --nocapture`; `cargo check --locked -p canic-cli`; `cargo clippy --locked -p canic-cli --all-targets --all-features -- -D warnings` |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `write_pretty_json` | `canic-cli::output` | Shared CLI output helper for JSON to stdout or `--out` files. | Revisit if command modules adopt command-local output writers. |
| `write_pretty_json_file` | `canic-cli::output` | Shared artifact writer for JSON files where callers own the artifact semantics. | Revisit if artifact writers move to command-owned IO modules. |
| `write_text` | `canic-cli::output` | Shared CLI output helper for text to stdout or `--out` files. | Revisit if stdout/file output behavior becomes command-specific. |
| `read_json_file` | `canic-cli::output` | Shared JSON decode helper; callers own validation and command authority. | Revisit if JSON input readers become domain-specific. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Visibility narrowing to `pub(crate)` | Rejected because `output` is already a private module and clippy flags `pub(crate)` as redundant there. | N/A |
| Broad `&Path` signature cleanup | Deferred because it pulls restore IO `ptr_arg` cleanup into this output slice. | Separate restore/io MSH pass. |

## Verification

- `cargo fmt --all -- --check`: PASS.
- `cargo test --locked -p canic-cli output -- --nocapture`: PASS, 22 filtered tests passed.
- `cargo check --locked -p canic-cli`: PASS.
- `cargo clippy --locked -p canic-cli --all-targets --all-features -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; CLI-only IO helper cleanup.
