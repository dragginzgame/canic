# Module Surface Hardening: canic-host tests

## Verdict

- Status: `PASS`.
- Risk score: `1 / 10`.
- Tier: `Tier 0`.
- Patch mode: `implementation-requested`.
- Cleanup result: no source changes; the module is a single crate-root
  invariant test and does not widen production visibility.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| target inventory | `find crates/canic-host/src/tests -type f -name '*.rs'`; `wc -l crates/canic-host/src/tests/mod.rs` | PASS: one 8-line test module | terminal output |
| stale/public surface scan | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/tests -g '*.rs'` | PASS: no public surface, stale markers, or lint suppressions found | terminal output |
| wiring inspection | `sed -n '1,80p' crates/canic-host/src/tests/mod.rs`; `sed -n '1,100p' crates/canic-host/src/lib.rs` | PASS: test covers `should_export_candid_artifacts` through crate-root `#[cfg(test)] mod tests` | source inspection |
| focused test | `cargo test --locked -p canic-host candid_artifact_export_is_dev_only -- --nocapture` | PASS: crate-root Candid export invariant test passed | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | PASS | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| None | N/A | No stale or overexposed test-only surface was found. | Focused test and host clippy passed. |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `candid_artifact_export_is_dev_only` | `canic-host` crate root | Guards that public Candid artifact export remains restricted to local/development environments. | If `should_export_candid_artifacts` moves into a narrower module with colocated tests. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Moving the crate-root test elsewhere | The tested helper currently lives in `lib.rs`; colocating this invariant in `src/tests/mod.rs` avoids adding test-only clutter to the crate root. | Helper ownership migration or a deliberate crate-root test layout change. |

## Verification

- `cargo test --locked -p canic-host candid_artifact_export_is_dev_only -- --nocapture`: PASS, 1 focused test passed.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; test-only audit with no production source change.
