# MSH Module Cleanup: canic-cli cli

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
| `in_scope_roots` | `crates/canic-cli/src/cli/`, `crates/canic-cli/src/lib.rs` |
| `excluded_roots` | historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused CLI tests |
| `audit_tier` | `Tier 1` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `2 / 10`.
- Tier: `Tier 1`, because this module owns internal CLI parsing and help
  helper surface plus the public `top_level_command` re-export path, but no
  deployment mutation, persistence, recovery, wasm, or generated boundary.
- Cleanup result: one one-caller helper was inlined. Numeric parser helpers
  were retained after the consumer check confirmed they are used as clap
  `ValueParser` function items.

The module remains the shared CLI command-construction boundary. It owns common
clap parsing helpers, default ICP/network values, top-level global option
dispatch, and human-readable top-level help rendering. It does not call ICP,
DFX, host install workflows, deployment-truth mutation, stable storage, backup,
or canister update paths.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| runner review | `sed -n '1,240p' docs/audits/modular/module-cleanup-runner.md` | PASS: implementation-requested cleanup runner checked | terminal output |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked | terminal output |
| target inventory | `wc -l crates/canic-cli/src/cli/*.rs` | PASS: module totals `576` LOC across `clap`, `defaults`, `globals`, `help`, and `mod` | terminal output |
| public surface inventory | `rg -n "crate::cli::(clap|defaults|globals|help)|cli::(clap|defaults|globals|help)" crates/canic-cli/src -g '*.rs'` | PASS: helper modules are internal `canic-cli` consumers; external API remains `canic_cli::top_level_command` | terminal output |
| stale-signal scan | `rg -n "allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|doc\\(hidden\\)" crates/canic-cli/src/cli crates/canic-cli/src/lib.rs -g '*.rs'` | PASS: no stale compatibility or dead-code lint markers found | terminal output |
| consumer check | `rg -n "parse_positive_(u64|usize)" crates/canic-cli/src -g '*.rs'` | PASS: positive parsers are live clap value parser helpers for metrics, cycles, and restore options | terminal output |
| cleanup check | `rg -n "render_help\\(" crates/canic-cli/src -g '*.rs'` | PASS after patch: no remaining one-caller wrapper | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| `cli::clap::render_help` | `INLINE NOW` into `render_usage` | It was a one-caller wrapper over `Command::render_help`; inlining preserves behavior and removes one exported internal helper. | `cargo check --locked -p canic-cli`; `cargo test --locked -p canic-cli cli -- --nocapture`; `cargo clippy --locked -p canic-cli --all-targets --all-features -- -D warnings` |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `parse_matches`, passthrough parsing, typed/string/path accessors | `canic-cli::cli::clap` | Shared internal clap adapter used by command modules to keep parse behavior consistent. | Revisit if command modules stop sharing clap parsing conventions. |
| `parse_positive_usize`, `parse_positive_u64` | `canic-cli::cli::clap` | Used as clap `ValueParser` function items by restore, metrics, and cycles options. | Revisit only if those option parsers move to command-local validators. |
| `default_icp`, `local_network` | `canic-cli::cli::defaults` | Central default source for ICP binary and local-network behavior. | Revisit if configuration defaults move to host/operator config. |
| global option dispatch helpers | `canic-cli::cli::globals` | Top-level `--icp` and `--network` forwarding must stay centralized so command-local hidden options are consistent. | Revisit when top-level global option forwarding is redesigned. |
| `top_level_command`, `usage`, help/version helpers | `canic-cli::cli::help` and facade re-export | `top_level_command` is the public command construction API; usage/help helpers own top-level text grouping. | Revisit only with a public CLI help surface change. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| None | No blocked or hot-path cleanup found. | N/A |

## Verification

- `cargo fmt --all -- --check`: PASS.
- `cargo check --locked -p canic-cli`: PASS.
- `cargo test --locked -p canic-cli cli -- --nocapture`: PASS, 3 focused tests passed.
- `cargo clippy --locked -p canic-cli --all-targets --all-features -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; CLI-only helper cleanup.
