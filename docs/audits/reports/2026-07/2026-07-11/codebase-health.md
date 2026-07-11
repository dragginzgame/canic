# Codebase Health Audit - 2026-07-11

## Report Preamble

- Scope: all Rust workspace crates, retained fleet and canister packages,
  workspace manifests, CI/build helpers, persistence boundaries, and current
  architecture governance.
- Exclusions: no full test suite, PocketIC matrix, deployment, network fetch,
  or full Wasm rebuild. The most recent Wasm audit remains the comparable
  2026-07-01 run.
- Compared baseline report path: `N/A`.
- Code snapshot identifier: `588b9f0c` plus the documented 0.84.14 release
  corrections in the current worktree.
- Method tag/version: `Codebase Health V1`.
- Comparability status: `non-comparable`; this is the first broad health run
  using this combined method.
- Auditor: `codex`.
- Branch: `main`.
- Worktree: dirty only for audit and release-documentation changes when the
  scan began.

## Executive Summary

- Risk score: **6 / 10**.
- One high-risk recovery issue exists: restore execution journals are
  truncated and rewritten in place between external mutations.
- Two medium-risk ownership issues should lead the next line: duplicated
  process-global unsafe build environments, and a direct unmaintained CBOR
  dependency that participates in stable-state and replica wire encoding.
- The strict runtime layering guard, memory-allocation authority, package
  publication boundaries, and role/package manifest guards pass.
- No known security vulnerability was reported by the local RustSec database;
  four unmaintained dependencies were reported, only one of which is directly
  owned by Canic.

## Inventory

| Measure | Current |
| --- | ---: |
| Rust files under `crates`, `canisters`, and `fleets` | 1,568 |
| Rust lines under `crates`, `canisters`, and `fleets` | 268,460 |
| Reviewed Rust, shell, TOML, and Markdown lines under implementation roots | 277,201 |
| Locked dependency packages scanned by RustSec | 524 |
| Workspace packages with inherited 0.84.14 versions | all checked packages |

## Findings

### High - Restore journals are not crash-safe

Evidence:

- `crates/canic-backup/src/restore/runner/io.rs` serializes a changed
  `RestoreApplyJournal` and persists it with truncating `fs::write`.
- The runner calls that write after seven state transitions across
  `execute.rs` and `preview.rs`.
- The journal is the recovery authority for operations that stop canisters,
  load snapshots, restart canisters, and verify restored state.
- Initial restore plans and journals in `crates/canic-cli/src/restore/io.rs`
  also use the generic truncating CLI output helper.
- Canic already has the required narrow primitive shape in
  `canic-host::durable_io`: unique sibling staging, file sync, rename, cleanup
  on failure, and parent-directory sync. Backup persistence has a second,
  weaker fixed-`.tmp` implementation.

Impact:

A process or machine failure during an in-place journal rewrite can leave an
empty or partial JSON document after an external mutation has completed. The
next restore invocation then loses the evidence needed to decide whether to
resume, recover, or refuse the operation. The sidecar lock prevents concurrent
writers but does not protect against interrupted writes.

Recommended hard cut:

1. Give `canic-backup` one internal durable byte-replacement function with a
   unique sibling temporary file, cleanup on error, file sync, rename, and
   directory sync.
2. Route the restore runner journal and backup persistence documents through
   it.
3. Expose only the narrow restore plan/journal persistence operation needed by
   `canic-cli`; do not make general host filesystem internals public.
4. Remove the truncating stateful paths. Ordinary `--out` presentation files
   do not need to become transactional.

This is one small persistence primitive, not a transaction framework.

### Medium - Build authority is carried through duplicated unsafe global environment mutation

Evidence:

- `crates/canic-cli/src/build.rs` owns one `BuildEnvGuard` and unsafe
  `set_var`/`remove_var` implementation.
- `crates/canic-host/src/install_root/build_environment/mod.rs` owns a second
  guard and unsafe implementation for substantially the same build context.
- Both safety comments depend on the surrounding process remaining
  single-threaded, but that invariant is not represented by the type system or
  enforced at the process boundary.
- Child-process APIs already support command-local `env` and `env_remove`
  values.

Impact:

Rust 2024 marks process environment mutation unsafe because other threads may
read the environment concurrently. Future progress output, background
inspection, or parallel artifact work could invalidate the current comments.
The duplicate guards can also drift in which Canic/ICP variables they save,
set, clear, and restore.

Recommended hard cut:

Create one host-owned typed build environment value and apply it directly to
each spawned Cargo/ICP command. Pass that value through CLI and install build
orchestration. Delete both global guards and all production unsafe environment
mutation; do not retain a fallback global path.

### Medium - Unmaintained CBOR owns stable and network encodings

Evidence:

- `cargo audit --no-fetch` reports `serde_cbor 0.11.2` as unmaintained under
  `RUSTSEC-2021-0127`.
- Canic directly declares it for `canic-core` and `canic-host`.
- Runtime serialization uses it through `canic-core::cdk::serialize`.
- `ReplayReceiptRecord` stable bytes are encoded and decoded with it.
- Host replica query envelopes and status responses use it for IC CBOR wire
  data.
- The scan found nine source modules with direct references across runtime,
  storage, tests, and host transport.

Impact:

This is not a known vulnerability, but Canic owns a persistent-schema and
network-protocol dependency that has no maintained upstream release path.
Replacing it casually could change stable bytes or canonical wire behavior;
leaving it indefinitely makes future Rust/dependency upgrades harder.

Recommended hard cut:

Make this a bounded 0.85 design decision. Select one maintained CBOR owner,
pin byte-for-byte fixtures for every stable record and protocol envelope, and
either prove compatible decoding/encoding or explicitly declare a destructive
pre-1.0 state cut. Do not carry two runtime CBOR implementations or a legacy
fallback decoder.

### Medium - Three new production hubs concentrate 0.84 change risk

Evidence:

| Module | File lines | Production boundary | Changed lines since 2026-06-28 |
| --- | ---: | ---: | ---: |
| `canic-cli/src/medic/mod.rs` | 1,573 | entire file; tests are separate | 3,498 |
| `canic-cli/src/deploy/plan.rs` | 1,915 | about 1,477 lines before inline tests | 2,379 |
| `canic-host/src/state_manifest/mod.rs` | 1,738 | about 1,132 lines before inline tests | 2,432 |

The modules mix several otherwise separable concerns:

- medic mixes project, deployment, package-contract, blob-storage, auth, and
  rendering-facing check construction;
- deploy plan mixes option definitions, evidence collection, comparison,
  diagnostics, rendering, output creation, and exit classification;
- state manifest mixes resolution, descriptor joins, all audit categories,
  aggregation, and next-action rendering data.

Impact:

The modules are not currently incorrect, but they are the highest edit-blast
radius in the current tree. Adding another check or state rule requires
reviewing large mixed-purpose files and increases merge/change friction.

Recommended hard cut:

Use ordinary directory modules and split only by existing responsibility. Do
not introduce a check framework or generic rule engine. Preserve the current
public facade and move private functions into focused children.

### Low - External diagnostic classification remains scattered

External string matching is correctly limited to tool/replica compatibility
boundaries, but classifiers are spread across CLI install, metrics, replica,
live-list, and host install-readiness/command modules. Keep the original
diagnostic and fail closed, but place each external tool's maintained wording
in one adapter owned by that tool boundary. Do not centralize unrelated tools
into one global error parser.

### Low - Dependency duplication is mostly upstream pressure

The normal/build graph contains parallel generations of crypto, HTTP, TOML,
and IC transport dependencies, including `k256 0.13/0.14`,
`reqwest 0.12/0.13`, and `ic-transport-types 0.45/0.47`. Current reverse trees
show these are largely contributed by `ic-agent`, `pocket-ic`, Candid, and
Canic's new direct crypto generation. Do not force workspace shims or patches
solely to reduce duplicate counts. Re-evaluate during normal upstream upgrades.

## Passing Boundaries

| Boundary | Result | Evidence |
| --- | --- | --- |
| Runtime layer direction | PASS | `scripts/ci/run-layering-guards.sh` exits successfully with no suppression. |
| Stable-memory allocation authority | PASS | Numeric active IDs resolve through `role_contract::allocation`; storage declarations use named authorities and canonical constants. |
| Published package direction | PASS | `workspace_manifest` package/publication tests: 6 passed. |
| Changelog structure | PASS | `changelog_governance`: 1 passed. |
| Security vulnerability scan | PASS with warnings | No known vulnerabilities; four unmaintained crates reported. |
| Direct unsafe review | PASS with finding | Production unsafe is limited to constructor macro machinery and the two environment guards recorded above. |
| DTO default review | PASS | Defaults found on empty/summary/container values; no unsafe command default was identified. |
| Hard-cut posture | PASS | No compatibility shim or retired memory-ID path was found in the reviewed 0.84 surfaces. |

## Unmaintained Dependency Readout

| Crate | Ownership | Action |
| --- | --- | --- |
| `serde_cbor 0.11.2` | direct Canic runtime/host dependency | 0.85 design and hard cut |
| `backoff 0.4.0` | transitive through `ic-agent` and `pocket-ic` | track upstream |
| `instant 0.1.13` | transitive through `backoff` | track upstream |
| `paste 1.0.15` | transitive through Candid | track upstream |

## Priority Order

1. Make restore journals durable before any further restore feature work.
2. Replace process-global build environment mutation with command-local typed
   environment propagation.
3. Decide and execute the CBOR hard cut with explicit stable/wire fixtures.

The hub splits should follow these safety changes and remain mechanical.

## Risk Matrix

| Area | Risk | Notes |
| --- | ---: | --- |
| Recovery and persistence | 8 / 10 | Interrupted restore journal writes can destroy recovery authority. |
| Build/process safety | 6 / 10 | Unsafe global environment mutation has a comment-only single-thread invariant. |
| Dependency/protocol ownership | 6 / 10 | Direct unmaintained CBOR participates in stable and IC wire encoding. |
| Layering and state allocation | 2 / 10 | Executable guard and canonical allocation scan pass. |
| Structural maintainability | 5 / 10 | Three recently changed modules are now 1,500-1,900 lines. |
| Security advisories | 3 / 10 | No vulnerability; four unmaintained dependencies. |
| Overall | 6 / 10 | Moderate health risk led by one concrete recovery defect. |

## Verification Readout

| Command/check | Status | Notes |
| --- | --- | --- |
| `bash scripts/ci/run-layering-guards.sh` | PASS | No runtime/control-plane workflow layering violations. |
| `cargo test -p canic --test workspace_manifest` | PASS | 6 passed. |
| `cargo test -p canic --test changelog_governance` | PASS | 1 passed. |
| `cargo audit --no-fetch` | PASS | No vulnerabilities; 4 allowed unmaintained warnings. |
| `cargo tree --workspace --locked --duplicates` | PASS | Duplicate generations inventoried and reverse ownership reviewed. |
| Stable-memory declaration scan | PASS | No unowned production numeric allocation was identified. |
| Production unsafe and text-classification scans | PASS with findings | Findings recorded above. |
| `git diff --check` | PASS | Documentation changes contain no whitespace errors. |
| Full test/PocketIC/Wasm matrix | BLOCKED by scope | Explicitly omitted under the targeted-test-only repository policy. |

## Follow-up

- Owner boundary: `canic-backup`; replace journal and persistence rewrites
  before the next restore behavior change.
- Owner boundary: `canic-host` with CLI consumers; make build environment
  command-local and remove both unsafe guards.
- Owner boundary: `canic-core` plus `canic-host`; write the CBOR hard-cut design
  before changing stable or wire bytes.
- Target run: the first 0.85 implementation audit.
