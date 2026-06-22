# Module Surface Hardening: canic-host format

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
| `code_snapshot` | `4bcad983` |
| `in_scope_roots` | `crates/canic-host/src/format/` |
| `excluded_roots` | install/deploy mutation, backup/recovery workflows, stable storage, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module tests |
| `audit_tier` | `Tier 1` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `1 / 10`.
- Tier: `Tier 1`, because this is public host formatting surface consumed by
  operator-facing CLI and host modules. It does not own install mutation,
  deployment truth, recovery, stable formats, generated boundaries, or wasm
  payload shape.
- Cleanup result: no high-confidence delete, narrow, inline, or move action was
  found. The retained helpers have current owners and focused tests.

The module remains the host-owned formatting boundary for cycle labels, byte
size labels, compact durations, and local wasm artifact size summaries. It only
formats passive values for operator output.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| cleanup runner fallback review | `sed -n '1,280p' docs/audits/modular/module-cleanup-runner.md` | PASS: cleanup runner checked as the implementation fallback; no patch was needed | terminal output |
| MSH definition review | `sed -n '1,320p' docs/audits/modular/module-surface-hardening.md`; `sed -n '321,620p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked for this run | terminal output |
| target inventory | `wc -l crates/canic-host/src/format/mod.rs crates/canic-host/src/format/tests.rs` | PASS: module totals `79` LOC across implementation and focused tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/format -g '*.rs'` | PASS: public surface is `byte_size`, `cycles_tc`, `wasm_size_label`, and `compact_duration`; no stale or dead-code markers found | terminal output |
| consumer check | `rg -n "canic_host::format|crate::format|format::\\{[^}]*compact_duration|format::\\{[^}]*wasm_size_label|format::\\{[^}]*cycles_tc|format::\\{[^}]*byte_size|compact_duration\\(|wasm_size_label\\(|cycles_tc\\(|byte_size\\(" crates/canic-host crates/canic-cli -g '*.rs'` | PASS: helpers are consumed by CLI live list/cycles output and host install/release-set rendering | terminal output |
| focused tests | `cargo test --locked -p canic-host format:: -- --nocapture` | PASS: 2 focused tests passed | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| None | `REJECT CLEANUP` | No stale compatibility, orphaned helper, overexposed internal helper, or one-caller abstraction without an invariant was found. | Focused format tests passed. |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `byte_size` re-export | `canic-host::format` and `canic-core::shared_support::format` | Host and CLI operator output share the canonical byte-size renderer while core retains the underlying formatting implementation. | Revisit only if host stops acting as the shared operator-format facade. |
| `cycles_tc` re-export | `canic-host::format` and `canic-core::shared_support::format` | Host install, release-set, and CLI cycles/list output need the canonical cycle label without duplicating core formatting. | Revisit if cycle output moves to a dedicated operator-report API. |
| `wasm_size_label` | `canic-host::format` | Operator output must prefer raw uncompressed wasm bytes and keep gzip as secondary context, matching the raw-wasm-primary audit rule. | Revisit if artifact-size reporting moves into a structured build-artifact report type. |
| `compact_duration` | `canic-host::format` | CLI cycles output uses a compact largest-unit duration label for table summaries, with focused coverage for edge cases. | Revisit if duration rendering becomes a shared table/report formatting policy. |
| `compact_duration_pair` | `canic-host::format` | Private helper keeps unit-pair formatting centralized behind `compact_duration`; it is not exported. | Revisit only with a behavior-preserving duration formatter rewrite. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| None | No hot-path, wasm-sensitive, generated-boundary, storage, recovery, or authority-reconciliation cleanup candidate was found. | N/A |

## Verification

- `cargo fmt --all -- --check`: not run; no source edits.
- `cargo test --locked -p canic-host format:: -- --nocapture`: PASS, 2 focused tests passed.
- `cargo check --locked -p canic-host`: not run; focused tests compiled `canic-host`.
- `cargo clippy --locked -p canic-host --all-targets --all-features -- -D warnings`: not run; no source edits.
- wasm/raw-size check: not applicable; host/CLI formatting audit with no runtime wasm payload change.
