# MSH Module Cleanup: canic-cli info

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
| `in_scope_roots` | `crates/canic-cli/src/info.rs` |
| `excluded_roots` | info leaf implementations, deployment truth query helpers, historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused info tests |
| `audit_tier` | `Tier 1` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `2 / 10`.
- Tier: `Tier 1`, because this module owns the `canic info` dispatcher and
  public command error, but delegates all query behavior to leaf modules.
- Cleanup result: removed a duplicate subcommand classification match from
  `parse_info_command` and centralized the clap subcommand list used by the
  parser.

The module remains a read-only dispatcher for installed-deployment information
commands. It parses `canic info <leaf>`, forwards passthrough arguments to leaf
commands, maps leaf errors, and prints group help/version output. It does not
query canisters directly, read deployment state, call ICP/DFX, mutate state, or
own leaf command behavior.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| runner review | `sed -n '1,240p' docs/audits/modular/module-cleanup-runner.md` | PASS: implementation-requested cleanup runner already reviewed for this day | terminal output |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules already reviewed for this day | terminal output |
| target inventory | `wc -l crates/canic-cli/src/info.rs` | PASS: module was `154` LOC before patch | terminal output |
| surface and stale-signal scan | `rg -n "info::|parse_info_command|InfoCommandError|canic info|allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|doc\\(hidden\\)|TODO|FIXME|pub\\(|pub " crates/canic-cli/src/info.rs crates/canic-cli/src -g '*.rs'` | PASS: no stale compatibility or dead-code markers; public surface is command error plus run entry | terminal output |
| focused tests | `cargo test --locked -p canic-cli info -- --nocapture` | PASS: 11 filtered tests passed | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| `parse_info_command` duplicate match | `INLINE NOW` / simplify | The clap command already constrains known subcommands; `parse_info_command` no longer re-maps each accepted command to the same name. Dispatch remains in `run`, where leaf ownership belongs. | focused info tests; package check; clippy |
| repeated clap subcommand construction | `NARROW NOW` into `INFO_SUBCOMMANDS` | Centralizing the parser subcommand names prevents the command builder from repeating the same leaf list by hand. | focused info tests; package check; clippy |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `InfoCommandError` | `canic-cli::info` | Public error boundary for the top-level info command group and leaf error mapping. | Revisit only if command error aggregation moves to a shared CLI dispatcher. |
| `run` dispatcher | `canic-cli::info` | Owns selecting the info leaf module after parsing and forwarding passthrough args. | Revisit if `canic info` leaves move into the main top-level dispatcher. |
| `INFO_USAGE` | `canic-cli::info` | Human-readable group help for installed-deployment info commands. | Revisit with a CLI help surface redesign. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Leaf command cleanup | Out of scope; list, cycles, metrics, endpoints, medic, and env own their own query and rendering behavior. | Dedicated leaf module MSH passes. |

## Verification

- `cargo fmt --all -- --check`: PASS.
- `cargo test --locked -p canic-cli info -- --nocapture`: PASS, 11 filtered tests passed.
- `cargo check --locked -p canic-cli`: PASS.
- `cargo clippy --locked -p canic-cli --all-targets --all-features -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; CLI-only dispatcher cleanup.
