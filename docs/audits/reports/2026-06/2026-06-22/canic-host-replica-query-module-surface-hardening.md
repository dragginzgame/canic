# Module Surface Hardening: canic-host replica_query

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
| `in_scope_roots` | `crates/canic-host/src/replica_query/` |
| `excluded_roots` | lower-level replica implementation, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module tests included to prove direct wire/status parsing helpers |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `3 / 10`.
- Tier: `Tier 2`, because this module is a public host facade over direct local
  replica HTTP query transport, Candid/CBOR wire decoding, local status/root-key
  discovery, and operator fallback behavior for local networks.
- Cleanup result: one transport-only endpoint helper was narrowed from
  `pub(super)` to private; the public facade and parent-consumed wire/transport
  helpers were retained with owner.

`replica_query` remains a live local-network authority adapter. It decides when
Canic may bypass CLI calls for direct local replica queries, builds replica
query envelopes, parses HTTP/CBOR/Candid responses, renders subnet registry
JSON in the existing CLI-compatible shape, and feeds install-root readiness,
build environment setup, status, list, cycles, token, metrics, snapshot, and
subnet-registry flows. The residual risk is low-to-moderate because the module
is small, has no stale compatibility branches or lint suppressions, and its
only overexposed helper was narrowed.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md`; `sed -n '1,220p' docs/audits/modular/module-cleanup-runner.md` | PASS: `MSH-2.0` and cleanup-runner rules checked for this run | terminal output |
| target inventory | `find crates/canic-host/src/replica_query -type f -name '*.rs'`; `wc -l crates/canic-host/src/replica_query/*.rs crates/canic-host/src/replica_query/*/*.rs` | PASS: `772` LOC across root, status, transport, wire, and tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/replica_query -g '*.rs'` | PASS: facade, parent-consumed internals, and one overexposed transport helper identified; no stale markers or lint suppressions found | terminal output |
| consumer check | `rg -n "replica_query|ReplicaStatus|query_canister_status|canister_status_from_replica|query_canister_module_hash|ModuleHashProof|Transport|ReplicaQuery" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: public facade is consumed by CLI status/replica/list/metrics/cycles/token/snapshot and host canister-ready, cycle-balance, subnet-registry, install-root readiness/build-environment flows | terminal output |
| cleanup patch | source inspection and diff review | PASS: `local_replica_endpoint_with_port` is only used by `transport` and child tests, so sibling visibility was unnecessary | source diff |
| focused tests | `cargo test --locked -p canic-host replica_query -- --nocapture` | PASS: 9 replica-query-filtered tests passed | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | PASS | terminal output |

## Reachable Surface Inventory

| Item | Kind | Visibility | Consumer Evidence | Consumer Should Exist? | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Local-query routing predicate | network selector | `pub` | CLI list/status and host canister-ready/cycle-balance/subnet-registry/install-root gate direct local queries behind local or explicit HTTP networks. | Yes | Prevents direct replica transport from being used for non-local networks. | `live-authority` | `replica_query` facade | `RETAIN WITH OWNER` | Medium; wrong routing changes operator target behavior. |
| Ready/bootstrap/cycle/subnet query functions | direct replica query API | `pub` | Canister-ready, install-root readiness, cycle-balance, subnet-registry, and CLI live-list flows consume these. | Yes | Encapsulates local replica HTTP query transport and Candid result decoding. | `live-authority` | `replica_query` facade | `RETAIN WITH OWNER` | Medium to high; wrong decode/query changes install/readiness evidence. |
| Local replica status and root-key helpers | local status API | `pub` | CLI status/replica and install-root build environment consume status reachability, root key, and endpoint selection. | Yes | Reads local replica `/api/v2/status` and extracts the root key needed for local environment setup. | `live-authority` | `replica_query::status` | `RETAIN WITH OWNER` | Medium. |
| CLI-compatible subnet registry JSON renderer | adapter surface | `pub` | Subnet registry and live list flows reuse discovery parsing that expects CLI-shaped JSON. | Yes | Bridges direct Candid query output into existing operator parser shape without duplicating parser ownership. | `live-authority` | `replica_query::wire` | `RETAIN WITH OWNER` | Medium. |
| Transport socket helpers | implementation support | `pub(super)` / private | Root facade and status child module call direct local query/status helpers. | Yes | Builds local HTTP requests and parses minimal HTTP responses for direct local replica calls. | `live-authority` | `replica_query::transport` | `RETAIN WITH OWNER` | Medium. |
| Wire decode structs/functions | implementation support | `pub(super)` | Root facade decodes bootstrap, cycle-balance, and subnet-registry Candid responses. | Yes | Keeps response wire contract local to the query adapter. | `live-authority` | `replica_query::wire` | `RETAIN WITH OWNER` | Medium. |

## Dead / Stale Surface Signals

| Candidate | File | Signal | Current Consumers | Consumer Should Exist? | Authority Reason | Surface Class | Deletion Confidence | Disposition | Risk If Removed |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `local_replica_endpoint_with_port` sibling visibility | `transport/mod.rs` | Function was `pub(super)` but only used inside `transport` and child tests. | `local_replica_endpoint`, `local_replica_endpoint_from_root`, transport tests. | Yes, but not outside `transport`. | Endpoint formatting is transport-owned and does not need parent-module access. | `overexposed-internal` | High | `NARROWED` | None after focused tests. |
| HTTP endpoint fallback to configured/default local port | `transport/mod.rs` | Fallback behavior. | Local query/status/root-key flows. | Yes. | Local replica query needs project-configured port or ICP CLI default port. | `live-authority` | Low | `RETAIN WITH OWNER` | Removing fallback would break local operator flows. |
| CLI-shaped registry JSON conversion | `wire/mod.rs` | Adapter vocabulary. | Subnet-registry/list flows. | Yes. | Existing discovery parser consumes the CLI response shape. | `live-authority` | Low | `RETAIN WITH OWNER` | Removing it requires parser ownership migration proof. |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| `transport::local_replica_endpoint_with_port` | `NARROW NOW`: `pub(super)` to private | The function is only called inside `transport`; child tests can still exercise private parent items. | `cargo test --locked -p canic-host replica_query -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |

## Runtime Authority Drift Check

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Local network routing | `should_use_local_replica_query` | CLI commands also know network names but delegate the direct-query decision here. | Consumer scan. | Yes | Direct local transport is centrally gated. | Wrong target selection. |
| Direct query envelope and HTTP transport | `replica_query::transport` | No alternate direct local HTTP query transport found. | Source inspection. | Yes | Transport code owns socket request/response shape. | Query rejection or malformed response risk. |
| Root status/root-key discovery | `replica_query::status` | CLI status wraps the helper but does not parse status itself. | Consumer scan and status tests. | Yes | Status parsing remains centralized. | Local install environment risk. |
| Candid response decoding | `replica_query::wire` | No duplicate direct-query decoder found for these calls. | Source inspection and tests. | Yes | Wire types stay private to the adapter boundary. | Decode drift risk. |

## Complexity And Runtime Shape

| Module | Complexity Signal | Retention Justification | Dead-Surface Link | Public/Hidden Items | Current Consumers | Shrink Action | Disposition | Expected Blast Radius | Risk |
| ---- | ---- | ---- | ---- | ----: | ---- | ---- | ---- | ---- | ---- |
| `mod.rs` | Public facade over direct local query operations. | Current CLI and host flows rely on it to avoid CLI process parsing for local networks. | None. | Public query functions and private response helpers. | Broad host/CLI local-network flows. | None. | `RETAIN WITH OWNER` | Broad. | Medium. |
| `status` | Local replica status/root-key parser. | Install-root environment setup and CLI status need status reachability/root key. | None. | Public status helpers; private parsers. | CLI replica/status and install-root. | None. | `RETAIN WITH OWNER` | Medium. | Medium. |
| `transport` | Socket-level HTTP/CBOR query implementation. | Owns direct local replica transport details. | Endpoint helper visibility narrowed. | Parent-visible query/status helpers; private endpoint formatting. | `replica_query` root and `status`. | Completed. | `RETAIN WITH OWNER` | Medium. | Medium. |
| `wire` | Candid decode and CLI JSON adapter. | Keeps direct-query wire contract and CLI-shape adapter local. | None. | Parent-visible decode/types; private entry conversion. | `replica_query` root. | None. | `RETAIN WITH OWNER` | Medium. | Medium. |

## Feature / Diagnostics / Test Surface Review

| Surface | Feature/Cfg | Production Consumer? | Test/Diagnostics Consumer? | Visibility Could Narrow? | Action | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `parse_ready_json_value` | production parser plus tests | Yes, canister-ready CLI fallback parsing. | Yes. | No. | None. | `RETAIN WITH OWNER` | Low. |
| `parse_local_replica_root_key` | status parser plus tests | Yes, through `local_replica_root_key_from_root`. | Yes. | Already `pub(super)` for module-local access. | None. | `RETAIN WITH OWNER` | Low. |
| Wire structs | Candid decode plus tests | Yes, through parent response decoder. | Yes. | No safe narrowing; parent names response type. | None. | `RETAIN WITH OWNER` | Low. |

## Removal Safety Plan

| Candidate | Action | Disposition | Owner Boundary | Hotness | Required Proof | Focused Validation | Wasm Raw Bytes Relevant? | Follow-Up Trigger |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Endpoint formatter sibling visibility | Make private. | `NARROW NOW` | `replica_query::transport` | `cold/warm` operator path | Focused tests and host clippy. | `cargo test --locked -p canic-host replica_query -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | No | Complete. |
| CLI-shaped subnet registry JSON adapter | Keep. | `DEFER WITH TRIGGER` | `replica_query::wire` and subnet registry parser | `cold/warm` operator path | Parser ownership migration proof. | Subnet-registry/list tests after migration. | No | If subnet registry parsing stops consuming CLI-shaped JSON. |
| Direct socket transport | Keep. | `RETAIN WITH OWNER` | `replica_query::transport` | `local query path` | Replacement transport proof against local replica HTTP API. | Replica/local integration coverage. | No | If transport moves to a shared host HTTP adapter. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Replacing manual HTTP parsing | The current code is minimal and avoids adding a runtime HTTP dependency to the host query path. | Dependency/runtime-shape decision plus local replica query integration proof. |
| Removing CLI-shaped registry JSON conversion | Existing subnet-registry/list parser expects that shape. | Parser ownership migration and focused list/subnet-registry validation. |

## Verification

- `cargo fmt --all`: PASS.
- `cargo test --locked -p canic-host replica_query -- --nocapture`: PASS, 9 replica-query-filtered tests passed.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- `git diff --check`: PASS.
- trailing whitespace scan over touched replica-query and report files: PASS.
- wasm/raw-size check: not applicable; host-only local replica query adapter change with no runtime wasm payload change.
