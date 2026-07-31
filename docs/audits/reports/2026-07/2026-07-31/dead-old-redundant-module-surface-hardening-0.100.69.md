# Module Surface Hardening: shared utility ownership follow-up

## Run Metadata

| Field | Value |
| ---- | ---- |
| `method_version` | `MSH-2.0` |
| `surface_taxonomy` | `ST-1` |
| `authority_taxonomy` | `AT-1` |
| `deletion_confidence_model` | `DC-1` |
| `compatibility_policy` | `pre-1.0-hard-cut` |
| `code_snapshot` | `e145e3d62` (`v0.100.68`) plus the open `0.100.69` worktree |
| `in_scope_roots` | `canic-host` and `canic-cli` Rust source |
| `excluded_roots` | generated output, target artifacts, canister runtime and stable-state code |
| `audit_tier` | `Tier 1`: host/operator implementation only; no boundary or persistence shape changed |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS` after focused validation.
- Risk score: `1 / 10` after cleanup.
- Cleanup result: 33 local or redundant helper functions were collapsed into
  ten shared functions and three existing owners were reused, removing 23 net
  functions, two net evidence types and 341 net Rust lines.
- Contract result: typed Candid arguments and responses, canister principals,
  methods, update/query selection, exact ICP rendering, digest rendering,
  package validation, path display, timestamp format and operation-clock
  behavior remain unchanged. No endpoint, Candid, stable schema, serialized
  shape or public Rust API changed.

## Evidence Log

| Evidence | Inspection | Result |
| ---- | ---- | ---- |
| install transport duplication | Traced textual-Candid query and call bodies across Coordinator installation, root installation, Registry workflows, Store bootstrap and runtime activation. | Six no-argument query helpers, six argument-call helpers and one no-argument call helper repeated one transport contract. Three install-operation owners now serve every caller; binary argument-file cleanup and typed Fleet closeout errors remain local. |
| live Registry duplication | Compared complete Coordinator Registry reads used by install verification, join, synchronization and activation. | Three identical evidence structs and loaders plus two manually repeated Coordinator reads now use one complete live Registry projection. Expected-state derivation and workflow-specific mismatch errors remain local. |
| Candid argument duplication | Compared Coordinator and Fleet Subnet Root typed install-argument rendering. | Both used the same `IDLValue` projection and tuple formatting; one generic command-boundary renderer now owns it. |
| install rendering duplication | Compared ICP e8s and 32-byte digest formatters used by Coordinator and root installation. | Two ICP renderers now use one command-boundary owner. Two SHA-256 renderers were deleted in favor of the existing exact module-hash renderer. |
| host utility duplication | Compared workspace path display, Cargo package validation and nonempty replica status parsing. | Each pair was byte-identical and now has one existing or parent-boundary owner with all validation limits and fallback behavior retained. |
| CLI utility duplication | Compared evidence timestamps, ICP-refill nanosecond clocks and length-prefixed hash inputs within `canic-cli`. | Three fallible Unix-seconds evidence helpers now share one evidence owner; pending-operation expiry reuses the operation-identity nanosecond clock; pending-key and operation-ID hashing share one exact part encoder. Helpers with different fallback, marker or durable-journal semantics remain separate. |
| retained surfaces | Rechecked binary Candid calls, Fleet closeout queries, typed journal transitions, backup timestamps and generated-service dependencies. | Retained because they own file-safety cleanup, typed errors, durable state invariants, domain-specific marker formats or macro expansion requirements. |

## Candidate Dispositions

| Candidate | Surface Class | Authority / Consumer Result | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- |
| install textual-Candid adapters | `duplicate-surface` | Identical transport and JSON decoding; exact domain authority stays at call sites. | `DELETE NOW` in favor of install operations owner | Low |
| complete live Registry evidence | `duplicate-surface` | Identical three-query projection; workflow-specific validation remains at consumers. | `DELETE NOW` in favor of one install Registry operation | Low |
| typed install Candid renderers | `duplicate-surface` | Coordinator and root arguments use identical tuple rendering. | `DELETE NOW` in favor of one generic command helper | Low |
| Coordinator/root funding and digest formatters | `duplicate-surface` | Byte-identical projections with one command or module-hash owner. | `DELETE NOW` | Low |
| host path, package and nonempty-text helpers | `duplicate-surface` | Identical mechanics inside one crate with an existing or immediate parent owner. | `DELETE NOW` | Low |
| CLI evidence timestamps and refill clocks | `duplicate-surface` | Identical format and error/fallback behavior within one command domain. | `DELETE NOW` | Low |
| ICP-refill hash-part encoders | `duplicate-surface` | Pending-key and generated operation-ID inputs used the same length-prefix framing. | `DELETE NOW` in favor of one conversion-domain helper | Low |
| binary Candid argument helpers | `domain-local-invariant` | Own secure temporary argument files and exact cleanup failure behavior. | `RETAIN WITH OWNER` | Medium if merged |
| Fleet closeout query helper | `domain-local-invariant` | Preserves method/canister-bound typed errors rather than erasing them into a generic box. | `RETAIN WITH OWNER` | Medium if merged |
| typed journal transition helpers | `domain-local-invariant` | Similar mechanics operate on distinct durable records and validation authority. | `RETAIN WITH OWNER` | Medium if merged |

## Verification Readout

- `cargo clippy -p canic-host --all-targets --no-deps -- -D warnings`: PASS.
- `cargo clippy -p canic-cli --all-targets --no-deps -- -D warnings`: PASS.
- Install command, Coordinator creation and config-selection tests: PASS, 17
  focused tests.
- Application and infrastructure release-set tests: PASS, 13 focused tests.
- Replica-query codec/status/transport/wire tests: PASS, 12 focused tests.
- Pending and generated ICP-refill operation tests: PASS, 10 focused tests.
- Targeted formatting and `git diff --check`: PASS.
