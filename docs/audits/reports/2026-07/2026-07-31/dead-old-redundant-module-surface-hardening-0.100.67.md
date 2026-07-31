# Module Surface Hardening: post-0.100.66 follow-up

## Run Metadata

| Field | Value |
| ---- | ---- |
| `method_version` | `MSH-2.0` |
| `surface_taxonomy` | `ST-1` |
| `authority_taxonomy` | `AT-1` |
| `deletion_confidence_model` | `DC-1` |
| `compatibility_policy` | `pre-1.0-hard-cut` |
| `code_snapshot` | `bc2f75c48` (`v0.100.66`) plus the shared in-progress `0.100.67` worktree |
| `in_scope_roots` | Rust source under `crates/`, `canisters/`, and `apps/` |
| `excluded_roots` | generated output, target artifacts, and concurrently edited Fleet lifecycle files |
| `audit_tier` | `Tier 1`: host-only helpers and internal visibility; generated Candid dependency ownership was traced but not changed |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS` after focused validation.
- Risk score: `1 / 10` after cleanup.
- Cleanup result: two one-consumer path-normalization helpers were inlined,
  removing 10 net production lines, and five helpers beneath public host
  modules were restricted to their owning parent module.
- Contract result: no endpoint, Candid, stable record, stable-memory ID,
  serialized value, feature gate, runtime authority or public host API changed.

## Evidence Log

| Evidence | Inspection | Result |
| ---- | ---- | ---- |
| dead-code suppression scan | Searched all repository Rust source for `allow(dead_code)`, `expect(dead_code)` and mixed lint lists. | Only the auth snapshot and feature-off sharding schema expectations remain. Both have live state-contract descriptor owners and were retained. |
| declaration reachability scan | Compared public function and type declarations with exact repository-wide identifier consumers, then inspected declaration-only and one-consumer candidates. | No additional zero-consumer function, type or enum variant was safe to remove. Two trivial host path helpers each had one consumer and no independent invariant. |
| module ownership scan | Compared Rust module declarations and compiled dependency inputs with repository source files. | No orphan production module or uncompiled host Rust source remains. Test-only modules found by the textual scan own executable test harnesses. |
| stale-surface scan | Searched active source for legacy, obsolete, deprecated, fallback, adoption, import, TODO and FIXME vocabulary and traced matches to owners. | Remaining matches describe current validation, observation, recovery or generated contracts; none is a compatibility path safe to hard-cut. |
| dependency scan | Ran the direct-dependency inventory and removed its two apparent unused dependencies from the Fleet Coordinator stub as a proof test. | Compilation then failed in `start_fleet_coordinator!` and `finish!` expansion because direct `candid` and `ic-cdk` crates were missing. Both dependencies were restored; this is a confirmed macro-expansion false positive. |
| unreachable-public scan | Audited `canic-host` with Rust's `unreachable_pub` lint and strict Clippy. | Five helpers under public modules accepted narrower `pub(super)` scope. Top-level private-module declarations were retained as `pub`: their module already bounds reachability and Clippy rejects narrower crate-equivalent spellings as `redundant_pub_crate`. |

## Candidate Dispositions

| Candidate | Surface Class | Authority / Consumer Result | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- |
| `canister_build::cache::absolute_from` | `duplicate-surface` | One caller; only repeated `Path::is_absolute` and workspace joining. | `DELETE NOW` | Low |
| `workspace_discovery::normalize_workspace_path` | `duplicate-surface` | One caller; no discovery, canonicalization or IO invariant. | `DELETE NOW` | Low |
| ICP output/compatibility runners | `overexposed-internal` | Consumed only by sibling modules below the public `icp` owner. | `NARROW NOW` | Low |
| network digest Serde adapters | `overexposed-internal` | Named only by the parent profile's Serde attribute. | `NARROW NOW` | Low |
| auth and sharding `dead_code` expectations | `live-schema-contract` | State-contract descriptors and focused snapshot validation own the otherwise feature/test-dependent shapes. | `RETAIN WITH OWNER` | Low |
| Fleet Coordinator stub `candid` and `ic-cdk` dependencies | `live-generated-boundary` | Required by exported-service and endpoint macro expansion despite no direct source reference. | `RETAIN WITH OWNER` | High if removed |
| top-level private host-module `pub` declarations | `effectively-private` | The private module caps external reachability; strict Clippy rejects `pub(crate)` and `pub(super)` as redundant. | `RETAIN WITH OWNER` | Low |
| concurrent Component Registry feature warnings | `concurrent-worktree` | Produced only while compiling the Coordinator-only stub against the separate in-progress `0.100.67` feature-gating slice. | `BLOCKED` from this pass; owned by the concurrent session | Unknown until that slice closes |

## Verification Readout

- `cargo check -p canic-host --all-targets`: PASS.
- `cargo clippy -p canic-host --all-targets --no-deps -- -D warnings`: PASS.
  Dependency linting was disabled because the concurrent control-plane slice
  currently owns separate Clippy findings.
- `cargo test -p canic-host canister_build::cache::tests --lib`: PASS, 1 test.
- `cargo test -p canic-host workspace_discovery::tests --lib`: PASS, 5 tests.
- `cargo test -p canic-host install_root::tests::config_selection --lib`: PASS,
  10 tests.
- `cargo test -p canic-host icp::tests --lib`: PASS, 11 tests.
- `cargo test -p canic-host network::tests --lib`: PASS, 14 tests.
- `cargo check -p fleet_coordinator_stub`: PASS with the direct macro
  dependencies restored. Its 73 control-plane dead-code warnings belong to
  the concurrently edited role-feature boundary and were not suppressed or
  changed here.
- Targeted formatting and `git diff --check`: PASS.

## Follow-up Trigger

Re-run the Coordinator-only warning inventory after the concurrent
`0.100.67` feature-gating work closes. Do not add broad dead-code suppression:
each remaining warning must either compile only for its root role or retain an
explicit state-contract owner.
