# Module Surface Hardening: canic-host response_parse

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
| `code_snapshot` | `5bc5a458` |
| `in_scope_roots` | `crates/canic-host/src/response_parse/` |
| `excluded_roots` | generated output, target artifacts, downstream parser implementation outside direct consumers |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module tests plus CLI metrics/cycles consumers selected by test filter |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `2 / 10`.
- Tier: `Tier 2`, because `response_parse` is a public `canic-host` facade
  consumed by `canic-cli` for operator response parsing.
- Cleanup result: four host-only helpers were narrowed from public API to
  crate-visible API; CLI-shared parsing primitives remain public.

`response_parse` is a small shared parser utility for ICP/CLI response shapes:
recursive JSON field lookup, `response_candid` extraction, Candid text helpers,
numeric parsing, quoted-string extraction, and Candid record block scanning.
It supports host metadata/cycle-balance/install-root readiness parsing and CLI
metrics/cycles parsing. The residual risk is low because the public surface now
tracks cross-crate consumers more closely, there are no stale compatibility
markers or lint suppressions, and the remaining public helpers are directly
used by CLI parsers.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md`; `sed -n '1,220p' docs/audits/modular/module-cleanup-runner.md` | PASS: `MSH-2.0` and cleanup-runner rules checked for this run | terminal output |
| target inventory | `find crates/canic-host/src/response_parse -type f -name '*.rs'`; `wc -l crates/canic-host/src/response_parse/mod.rs crates/canic-host/src/response_parse/tests.rs` | PASS: `255` LOC across module and tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/response_parse -g '*.rs'` | PASS: parser facade exports identified; no stale markers or lint suppressions found | terminal output |
| consumer check | `rg -n "RECORD_MARKER|find_field\\(|find_string_field\\(|response_candid\\(|parse_candid_text_field\\(|parse_candid_text_like_field\\(|parse_cycle_balance_response\\(|parse_json_u64\\(|parse_json_u128\\(|field_value_after_equals\\(|text_after\\(|parse_u64_digits\\(|parse_u128_digits\\(|quoted_strings\\(|candid_record_blocks\\(" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: host-only and CLI-shared helpers separated | terminal output |
| cleanup patch | source inspection and diff review | PASS: host-only helpers narrowed to `pub(crate)` while CLI consumers retain required public helpers | source diff |
| focused host tests | `cargo test --locked -p canic-host response_parse -- --nocapture` | PASS: 3 response-parse-filtered tests passed | terminal output |
| CLI consumer tests | `cargo test --locked -p canic-cli metrics -- --nocapture`; `cargo test --locked -p canic-cli cycles -- --nocapture` | PASS: 11 metrics-filtered tests and 43 cycles-filtered tests passed | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings`; `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | PASS | terminal output |

## Reachable Surface Inventory

| Item | Kind | Visibility | Consumer Evidence | Consumer Should Exist? | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `RECORD_MARKER` and `candid_record_blocks` | Candid text parser support | `pub` | CLI metrics Candid-text parsing. | Yes | Shared parser support for operator response shapes. | `live-authority` | `response_parse` facade | `RETAIN WITH OWNER` | Low. |
| Recursive JSON lookup helpers | JSON response parser support | `pub` / `pub(crate)` | CLI metrics/cycles use `find_field`; host metadata uses `find_string_field`. | Yes | Centralizes tolerant ICP JSON response traversal. | `live-authority` | `response_parse` facade | `NARROWED` for host-only string helper | Low. |
| `response_candid` | ICP JSON response adapter | `pub` | CLI metrics/cycles and host metadata/install-root parsing. | Yes | Extracts direct Candid text from ICP JSON output. | `live-authority` | `response_parse` facade | `RETAIN WITH OWNER` | Low. |
| Candid text field helpers | Candid text parser support | `pub(crate)` | Host metadata and install-root readiness parsing. | Yes | Parses string and optional-string fields from Candid text fallback output. | `live-authority` | `response_parse` facade | `NARROWED` | Low. |
| Cycle-balance response parser | host query parser | `pub(crate)` | Host `cycle_balance` fallback after direct replica query fails or is unavailable. | Yes | Parses `canic_cycle_balance` ICP CLI response shapes. | `live-authority` | `cycle_balance` via `response_parse` | `NARROWED` | Low. |
| Numeric/text helpers | parser primitives | `pub` | CLI metrics/cycles parse JSON and Candid text numbers/labels. | Yes | Avoids duplicated tolerant numeric/text response parsing in CLI. | `live-authority` | `response_parse` facade | `RETAIN WITH OWNER` | Low. |

## Dead / Stale Surface Signals

| Candidate | File | Signal | Current Consumers | Consumer Should Exist? | Authority Reason | Surface Class | Deletion Confidence | Disposition | Risk If Removed |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `find_string_field` public visibility | `mod.rs` | Used only by host metadata parsing. | `canic_metadata`. | Yes, but not cross-crate. | Metadata parsing is host-internal. | `overexposed-internal` | High | `NARROWED` | None after host tests/clippy. |
| `parse_candid_text_field` public visibility | `mod.rs` | Used only by host metadata parsing. | `canic_metadata`. | Yes, but not cross-crate. | Metadata Candid fallback parsing is host-internal. | `overexposed-internal` | High | `NARROWED` | None after host tests/clippy. |
| `parse_candid_text_like_field` public visibility | `mod.rs` | Used only by install-root readiness parsing. | `install_root::readiness::parsing`. | Yes, but not cross-crate. | Bootstrap status fallback parsing is host-internal. | `overexposed-internal` | High | `NARROWED` | None after host tests/clippy. |
| `parse_cycle_balance_response` public visibility | `mod.rs` | Used only by host cycle-balance query fallback and module tests. | `cycle_balance`. | Yes, but not cross-crate. | Cycle-balance fallback parsing is host-internal. | `overexposed-internal` | High | `NARROWED` | None after host tests/clippy. |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| `find_string_field` | `NARROW NOW`: `pub` to `pub(crate)` | Only same-crate metadata parser consumes it. | `cargo test --locked -p canic-host response_parse -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |
| `parse_candid_text_field` | `NARROW NOW`: `pub` to `pub(crate)` | Only same-crate metadata parser consumes it. | `cargo test --locked -p canic-host response_parse -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |
| `parse_candid_text_like_field` | `NARROW NOW`: `pub` to `pub(crate)` | Only same-crate install-root readiness parser consumes it. | `cargo test --locked -p canic-host response_parse -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |
| `parse_cycle_balance_response` | `NARROW NOW`: `pub` to `pub(crate)` | Only same-crate cycle-balance query fallback consumes it. | `cargo test --locked -p canic-host response_parse -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `find_field`, `parse_json_u64`, `parse_json_u128` | `response_parse` facade | CLI metrics/cycles need tolerant JSON traversal and numeric parsing across ICP response shapes. | If CLI parsers move into `canic-host` or stop consuming these helpers. |
| `response_candid`, `field_value_after_equals` | `response_parse` facade | Host and CLI response parsers share Candid fallback extraction. | If ICP response parsing is replaced by typed Candid decode for these command paths. |
| `text_after`, `parse_u64_digits`, `parse_u128_digits`, `quoted_strings` | `response_parse` facade | CLI text parsers share tolerant Candid-text primitive parsing. | If CLI metrics/cycles Candid text fallback is removed. |
| `RECORD_MARKER`, `candid_record_blocks` | `response_parse` facade | CLI metrics parser scans Candid record blocks without owning the shared marker. | If metrics parsing no longer accepts Candid text output. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Removing Candid text fallback parsing | ICP command output still reaches host/CLI parsers in Candid text and JSON-with-`response_candid` shapes. | Proof all relevant commands use typed Candid decode or stable JSON only, plus focused metadata/readiness/cycles/metrics validation. |
| Moving CLI parser primitives into `canic-cli` | The helpers are intentionally shared by host and CLI response parsers. | Ownership decision and duplication/migration proof across metadata, readiness, cycle-balance, metrics, and cycles parsers. |

## Verification

- `cargo fmt --all`: PASS.
- `cargo test --locked -p canic-host response_parse -- --nocapture`: PASS, 3 response-parse-filtered tests passed.
- `cargo test --locked -p canic-cli metrics -- --nocapture`: PASS, 11 metrics-filtered tests passed.
- `cargo test --locked -p canic-cli cycles -- --nocapture`: PASS, 43 cycles-filtered tests passed.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- `cargo clippy --locked -p canic-cli --all-targets -- -D warnings`: PASS.
- `git diff --check`: PASS.
- trailing whitespace scan over touched response-parse and report files: PASS.
- wasm/raw-size check: not applicable; host/CLI parser visibility cleanup with no runtime wasm payload change.
