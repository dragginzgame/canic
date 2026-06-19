# Audit Summary - 2026-06-19

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `canic-cli-cli-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/cli/` | PASS |
| `canic-cli-evidence-support-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/evidence_support.rs` | PASS |
| `canic-cli-info-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/info.rs` | PASS |
| `canic-cli-output-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/output/` | PASS |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `canic-cli-cli-module-surface-hardening.md` | 2 / 10 | Internal CLI helper surface is retained with owner; one one-caller help-rendering wrapper was inlined. |
| `canic-cli-evidence-support-module-surface-hardening.md` | 2 / 10 | Evidence command-provenance helper is retained with owner; optional path input was narrowed to `Option<&Path>`. |
| `canic-cli-info-module-surface-hardening.md` | 2 / 10 | Read-only info dispatcher is retained with owner; duplicate subcommand remapping was removed. |
| `canic-cli-output-module-surface-hardening.md` | 2 / 10 | Shared CLI output helpers are retained with owner; one plain-filename parent-directory edge case was hardened. |

## Method / Comparability Notes

- `canic-cli-cli-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-evidence-support-module-surface-hardening.md` uses `MSH-2.0` and
  is non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-info-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-output-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.

## Key Findings

- The `cli/` module remains the shared command-construction boundary for
  `canic-cli` parsing, defaults, global option forwarding, and top-level help.
- No ICP, DFX, deployment mutation, stable storage, backup/recovery, wasm, or
  generated-boundary authority was found in the inspected module.
- The `render_help` wrapper had one caller and was inlined into `render_usage`.
- The positive-number parser helpers are retained because they are consumed as
  clap `ValueParser` function items by cycles, metrics, and restore options.
- The `evidence_support.rs` helper remains a passive command-provenance adapter.
- `push_optional_path_arg` now takes `Option<&Path>` and its focused tests cover
  absent optional paths plus outside-root redaction.
- The `info.rs` dispatcher no longer reclassifies already-validated clap
  subcommands in `parse_info_command`; dispatch remains in `run`.
- The `output/` module remains a passive file/stdout IO helper boundary.
- Plain relative output filenames now skip empty parent-directory creation,
  while nested output paths still create parents.
- Output helper visibility was left as `pub` because the parent module is
  private and clippy treats `pub(crate)` as redundant there.

## Verification Readout Rollup

| Report | PASS | FAIL | BLOCKED |
| ---- | ----: | ----: | ----: |
| `canic-cli-cli-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-evidence-support-module-surface-hardening.md` | 6 | 0 | 0 |
| `canic-cli-info-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-output-module-surface-hardening.md` | 4 | 0 | 0 |

## Follow-up Actions

- Continue the CLI tree with the next focused module, avoiding backup/restore
  recovery surfaces unless a Tier 2 pass is explicitly desired.
- Defer broad `&Path` cleanup around restore IO to a dedicated restore/io pass,
  because it crosses into backup/recovery-adjacent authority.
