# Module Surface Hardening: canic-host registry

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
| `in_scope_roots` | `crates/canic-host/src/registry/` |
| `excluded_roots` | subnet-registry query transport, replica-query Candid adapters, CLI rendering/tree helpers, backup planner internals, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module and direct registry-consumer tests selected by test filter |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `4 / 10`.
- Tier: `Tier 2`, because this public parser turns root registry observations
  into `RegistryEntry` rows consumed by installed-deployment resolution, list,
  status, metrics, cycles, token, backup/snapshot planning, and deployment-truth
  observation.
- Cleanup result: no safe delete, narrow, inline, or move candidate was found
  in this read-only pass.

The module is compact and intentionally parser-shaped. It owns the host-facing
registry row projection and accepts the currently observed shapes from root
registry queries: raw arrays, `{ "Ok": [...] }` JSON, and ICP
`response_bytes` Candid envelopes. It does not query the replica or ICP CLI
itself and does not mutate state. The residual risk is parser authority:
mis-parsing `parent_pid`, `role`, `kind`, or `module_hash` can affect operator
targeting, backup/snapshot plans, and deployment-truth observation.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,220p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked for this run | terminal output |
| target inventory | `wc -l crates/canic-host/src/registry/mod.rs crates/canic-host/src/registry/tests.rs` | PASS: `428` total LOC across parser and focused tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/registry -g '*.rs'` | PASS: public row DTO, error enum, and parser function identified; no stale markers or lint suppressions found | terminal output |
| consumer check | `rg -n "registry::|parse_registry_entries|RegistryEntry|RegistryParseError|canister registry|root registry|Registry" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: registry rows are live across installed deployment, CLI list/status/info-env/metrics/cycles/token/backup/snapshot/endpoints, deployment-truth observation, and backup crate tests/planning | terminal output |
| authority boundary scan | `rg -n "Decode!|CandidType|Principal|serde_json::from|Value|response_bytes|hex_to_bytes|hex_bytes|parse_optional_principal|parse_module_hash|debug_assert|TODO|FIXME|legacy|fallback|compat" crates/canic-host/src/registry -g '*.rs'` | PASS: parser owns JSON/Candid shape conversion only; no live query, file IO, or mutation authority found | terminal output |
| focused tests | `cargo test --locked -p canic-host registry -- --nocapture` | PASS: 15 registry-filtered tests passed, including direct parser tests and registry consumers | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | PASS | terminal output |

## Reachable Surface Inventory

| Item | Kind | Visibility | Consumer Evidence | Consumer Should Exist? | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `RegistryEntry` | host registry row DTO | `pub` | CLI commands, installed-deployment, deployment-truth observation, and backup/snapshot adapters consume row fields. | Yes | Canonical host projection of a root registry canister row for operator targeting and topology planning. | `live-authority` | `canic-host::registry` | `RETAIN WITH OWNER` | Medium to high; row fields affect target selection. |
| `RegistryParseError` | parser error | `pub` | CLI list/metrics/cycles/token/snapshot/backup and installed-deployment map or expose parse failures. | Yes | Preserves JSON shape, response-bytes hex, Candid decode, and root rejection failure modes. | `live-diagnostics` | `canic-host::registry` | `RETAIN WITH OWNER` | Medium; weak diagnostics can hide bad registry evidence. |
| `parse_registry_entries` | parser facade | `pub` | Installed-deployment, CLI list tests, backup preflight, snapshot download, and deployment-truth observation use it. | Yes | Single public conversion point from root registry query output into host registry rows. | `live-authority` | `canic-host::registry` | `RETAIN WITH OWNER` | High for backup/snapshot/deployment-truth correctness. |
| Candid wire structs and hex helpers | internal parser support | private | Used only by `response_bytes` parsing and tests. | Yes | Supports ICP response envelope shape without exposing wire details. | `live-authority` | `canic-host::registry` | `RETAIN WITH OWNER` | Medium; decode drift would break ICP query parsing. |

## Dead / Stale Surface Signals

| Candidate | File | Signal | Current Consumers | Consumer Should Exist? | Authority Reason | Surface Class | Deletion Confidence | Disposition | Risk If Removed |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `response_bytes` Candid parser lane | `mod.rs` | Alternate input shape in addition to JSON arrays and `{ "Ok": [...] }`. | Direct parser tests and ICP response consumers. | Yes | ICP command/replica responses can arrive as encoded Candid response bytes, and the parser surfaces rejection/hex/decode errors. | `live-authority` | Low | `RETAIN WITH OWNER` | Removing it would break valid root registry query output. |
| JSON entry `filter_map` behavior | `mod.rs` | Malformed individual JSON rows are skipped rather than making the whole parse fail. | Existing consumers rely on parser output shape; no explicit rejection contract found. | Unclear without compatibility proof. | Current behavior may tolerate partial registry rows from older/root outputs, but bad rows can also hide evidence. | `unclear` | Blocked | `MEASURE FIRST` | Tightening this could change CLI/backup/deployment-truth behavior and needs targeted compatibility tests. |

## Removed / Narrowed / Inlined / Moved

No changes were made in this read-only run.

## Runtime Authority Drift Check

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Registry row schema | `canic-host::registry::RegistryEntry` owns host row projection. | `canic-backup` has its own backup registry row type, converted from host rows by CLI backup planning. | Consumer scan and backup adapter. | Yes | Host row stays the source projection for live root registry data; backup owns its domain row after conversion. | Schema drift can break backup/snapshot target planning. |
| Registry JSON shape parsing | `parse_registry_entries_json` owns array and `{ "Ok": [...] }` conversion. | Replica query wire can emit CLI JSON, but does not own host projection. | Parser tests and replica-query wire tests. | Yes | Host parser centralizes the operator-facing conversion. | Silent row drops remain a watchpoint. |
| ICP `response_bytes` parsing | Private Candid wire structs and hex helpers own this parser lane. | `replica_query::wire` has a separate decode path for local replica transport. | Parser tests and direct code inspection. | Yes | Duplication is bounded by different input boundary responsibilities. | Wire shape drift can break direct ICP output parsing. |
| Query transport | `subnet_registry` and `replica_query` own querying. | No query logic in `registry`. | Authority scan. | Yes | Parser has no live-state or mutation authority. | Low mutation risk. |

## Complexity And Runtime Shape

| Module | Complexity Signal | Retention Justification | Dead-Surface Link | Public/Hidden Items | Current Consumers | Shrink Action | Disposition | Expected Blast Radius | Risk |
| ---- | ---- | ---- | ---- | ----: | ---- | ---- | ---- | ---- | ---- |
| `mod.rs` | Compact parser with JSON, optional-principal/module-hash conversion, hex conversion, and Candid response envelope support. | Host commands and backup/deployment-truth need one registry projection rather than per-command parsing. | `response_bytes` retained; JSON row strictness deferred. | Public DTO/error/parser; private wire and helpers. | Broad host/CLI/backup consumers. | None. | `RETAIN WITH OWNER` | Medium to broad. | Parser authority. |
| `tests.rs` | Four focused parser tests. | Covers wrapped JSON, Candid response bytes, invalid hex, and rejected response bytes. | None. | Test-only. | Host unit tests. | None. | `RETAIN WITH OWNER` | Low. | Parser coverage. |

## Facade / Generated Boundary Review

| Surface | Boundary Type | Generated Consumer Evidence | Could Narrow? | Required Replacement | Deletion Confidence | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `pub mod registry` from `canic-host` | Host facade | No generated consumer found. | Not safely in this slice; CLI and backup adapters consume it directly. | Dedicated narrower parser modules plus CLI/backup/deployment-truth migration proof. | Low | `RETAIN WITH OWNER` | Public host registry projection. |
| `RegistryEntry` public fields | Host DTO | No generated consumer found. | Not safely; downstream command/planning code reads fields directly. | Accessor or domain-specific row migration across CLI/backup/deployment-truth. | Low | `RETAIN WITH OWNER` | Target selection and topology planning. |

## Feature / Diagnostics / Test Surface Review

| Surface | Feature/Cfg | Production Consumer? | Test/Diagnostics Consumer? | Visibility Could Narrow? | Action | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `#[cfg(test)] mod tests` | test only | No | Yes. | Already test-only. | None. | `RETAIN WITH OWNER` | Low. |
| Parse errors | normal production diagnostics | Yes | Yes. | No safe narrowing found. | None. | `RETAIN WITH OWNER` | Medium. |

## Removal Safety Plan

| Candidate | Action | Disposition | Owner Boundary | Hotness | Required Proof | Focused Validation | Wasm Raw Bytes Relevant? | Follow-Up Trigger |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `response_bytes` parser lane | Keep. | `RETAIN WITH OWNER` | `canic-host::registry` | `cold` operator/parser path | Proof all production registry queries use JSON array or `{ "Ok": [...] }` only. | Registry parser tests plus CLI list/snapshot/backup tests. | No | ICP/replica query output contract removes response-bytes shape. |
| JSON row `filter_map` tolerance | Keep for now. | `MEASURE FIRST` | `canic-host::registry` | `cold` operator/parser path | Compatibility review and tests for malformed rows across list/status/backup/snapshot/deployment-truth. | Focused parser and command tests. | No | User requests stricter registry parsing or backup/deployment-truth hardening. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Strict rejection of malformed individual JSON rows | Current parser silently skips rows lacking `pid`; tightening could change behavior for existing operator outputs. | Compatibility decision plus CLI/backup/deployment-truth tests proving intended failures and messages. |
| Hiding `RegistryEntry` fields | Public row fields are consumed across command rendering, topology selection, backup planning, and tests. | Broad consumer migration to accessors or domain-specific row conversions. |
| Consolidating Candid wire decode with `replica_query::wire` | The modules sit at different input boundaries: this parser decodes ICP `response_bytes` envelopes, while `replica_query` decodes local query bytes. | Shared abstraction that does not blur transport ownership and preserves parser tests. |

## Verification

- `cargo fmt --all`: not run; no code edits were made for this module.
- `cargo test --locked -p canic-host registry -- --nocapture`: PASS, 15 registry-filtered tests passed.
- `cargo check --locked -p canic-host`: not run; focused tests compiled `canic-host`.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; host parser audit with no runtime wasm payload change.
