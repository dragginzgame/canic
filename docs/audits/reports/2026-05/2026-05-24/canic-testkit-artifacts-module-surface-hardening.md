# MSH Compact Audit: canic-testkit artifacts

## Preamble

- Scope: `crates/canic-testkit/src/artifacts`
- Compared baseline report path: `N/A`
- Code snapshot identifier: `0eaad7dc`
- Method tag/version: `MSH-2.0`
- Comparability status: `non-comparable`: first Canic Tier 1 compact Module
  Surface Hardening pilot for this module

## Verdict

- Risk score: `2 / 10`
- Tier: `Tier 1`
- Patch mode: `implementation-requested`
- Main decision: the public artifact helper surface pays rent as test-support
  API; the useful cleanup was in a consumer that duplicated the target module's
  wasm path/read ownership.

## Scope / Evidence

| Area | Evidence | Result |
| ---- | ---- | ---- |
| Target files | `find crates/canic-testkit/src/artifacts -maxdepth 1 -type f -print` | 4 files: `mod.rs`, `workspace.rs`, `wasm.rs`, `icp.rs` |
| Size | `wc -l crates/canic-testkit/src/artifacts/*.rs` | 487 total LOC |
| Public surface | focused `rg` for `pub`, hidden docs, dead-code allowances, stale terms | Public test-support exports are active; no `dead_code` allowances or compatibility shims found |
| Consumers | focused `rg` across `crates`, `canisters`, and `fleets` | Exports are consumed by `canic-testing-internal`, `canic-tests`, and local testkit tests |
| Cleanup candidate | inspected `crates/canic-testing-internal/src/pic/attestation/build.rs` | Consumer duplicated wasm path/read helpers already owned by `canic-testkit::artifacts` |

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| public surface inventory | `rg -n "pub \|doc\(hidden\)\|allow\(dead_code\)\|expect\(dead_code\)\|expect\(unused_imports\)\|legacy\|compat\|fallback\|shim\|deprecated\|temporary\|TODO\|FIXME" crates/canic-testkit/src/artifacts` | No hidden exports, stale compatibility hooks, or lint allowances; public helpers are explicit test-support API | terminal output |
| stale-signal scan | same scan plus source inspection of `icp.rs`, `wasm.rs`, `workspace.rs`, `mod.rs` | No stale compatibility or fallback surface in target | terminal output/source inspection |
| consumer check | `rg` for exported helper names across `crates`, `canisters`, and `fleets` | Public exports have current consumers; `icp_artifact_ready_for_build` is direct-test-only but paired with public snapshot API | terminal output |
| duplicate-surface check | inspection of `crates/canic-testing-internal/src/pic/attestation/build.rs` | Found duplicated fast-profile wasm path/read helpers and target-dir construction | source inspection |
| validation | `cargo fmt --all`; `cargo check -p canic-testkit`; `cargo check -p canic-testing-internal`; `cargo test -p canic-testkit --test artifact_helpers`; `git diff --check` | All pass | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / reason |
| ---- | ---- | ---- | ---- | ---- |
| `WasmBuildProfile`, `wasm_path`, `wasm_artifacts_ready`, `read_wasm`, wasm build helpers | `live-test-support` | high | `RETAIN WITH OWNER` | Owner: `canic-testkit::artifacts`; these define the public test-support contract for host-side wasm builds and reads. |
| `WatchedInputSnapshot`, `icp_artifact_ready_with_snapshot`, `build_icp_all_with_env` | `live-test-support` | high | `RETAIN WITH OWNER` | Owner: `canic-testkit::artifacts`; these support root baseline artifact freshness and local `.icp` artifact builds in `canic-testing-internal`. |
| `icp_artifact_ready_for_build` | `live-test-support` | medium | `RETAIN WITH OWNER` | Owner: `canic-testkit::artifacts`; direct production consumers were not found, but it is the simple public wrapper tested by `artifact_helpers`/inline tests and keeps snapshot capture optional for callers. Trigger: revisit if no external testkit consumers appear after the next testkit surface audit. |
| Duplicate wasm path/read helpers in `canic-testing-internal/src/pic/attestation/build.rs` | `duplicate-surface` | high | `MOVE OWNER` | Owner: `canic-testkit::artifacts`; the consumer duplicated `read_wasm`, `wasm_path`, and fixed `fast` layout behavior that the testkit helper already owns. Patched by routing reads and target-dir construction through the canonical helper. |

## Hot / Wasm Risk

| Item | Hotness | Risk | Required proof |
| ---- | ---- | ---- | ---- |
| `canic-testkit::artifacts` helpers | `test-only` | Host-side test artifact paths/build commands; no canister runtime or raw wasm payload shape changes. | `cargo check -p canic-testkit`, `cargo check -p canic-testing-internal`, and focused testkit artifact tests. |
| `canic-testing-internal` consumer cleanup | `test-only` | Path construction now delegates to the same helper shape; no build command or profile behavior changed. | Same focused validation; no raw wasm byte comparison required. |

## Disposition Ledger

| Disposition | Count |
| ---- | ----: |
| DELETE NOW | 0 |
| NARROW NOW | 0 |
| INLINE NOW | 0 |
| MOVE TO TEST | 0 |
| MOVE OWNER | 1 |
| RETAIN WITH OWNER | 3 |
| MEASURE FIRST | 0 |
| BLOCKED | 0 |

## Validation / Follow-up

- Required validation: completed focused package checks and testkit artifact
  integration tests.
- Blocked decisions: none.
- Triggers: revisit `icp_artifact_ready_for_build` only if a future testkit
  surface audit shows it still has no consumers outside its own tests.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo fmt --all` | PASS | formatted the consumer cleanup |
| `cargo check -p canic-testkit` | PASS | target crate compiles |
| `cargo check -p canic-testing-internal` | PASS | patched consumer compiles |
| `cargo test -p canic-testkit --test artifact_helpers` | PASS | 5 artifact helper tests passed |
| `git diff --check` | PASS | whitespace check passed |

## Pilot Readout

This pilot did produce real cleanup, but the result was small. The Tier 1 shape
was useful because it separated "public but owned test-support API" from the
actual duplicate surface in a consumer. For this kind of module, the process is
worth using when there is visible consumer duplication or public testkit surface
pressure. It would be too heavy as a mandatory step for every tiny test helper.
