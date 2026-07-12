# 0.85 Status: Operational Safety Hardening

Last updated: 2026-07-12

## Purpose

This file is the permanent implementation tracker for the 0.85 design line.
The design captures the intended contract; this file records what lands, what
is rejected, and what remains open so the line is not reconstructed from chat
history.

Design: [0.85-design.md](0.85-design.md)

Audit source:
`docs/audits/reports/2026-07/2026-07-11/codebase-health.md`

## Current State

Slices A, B, and C are complete and published as `v0.85.0`. The stdin transport
cleanup is published as `v0.85.1`, and typed binary staging is published as
`v0.85.2`. Release artifact path containment is published as `v0.85.3`, and
canonical manifest identity admission as `v0.85.4`, which is the current package
version.

The next focused cleanup is changelog-finalized for `0.85.5`, while package
versions remain `0.85.4` until the human-owned release flow runs. The same
manifest-owned validator now rejects invalid static artifact shapes across
writing, loading, and staging, and the workspace adopts `ic-query 0.10.0`
without a Canic source migration. It does not reopen the completed restore,
build-authority, or stable-CBOR contracts.

## Locked Scope

1. Durable restore plan/journal replacement owned by `canic-backup`.
2. One explicit host-owned canister build context with no production global
   environment mutation.
3. CBOR byte proof and removal of Canic-direct `serde_cbor` ownership where
   exact compatibility is demonstrated.

The line does not include hub-module splitting, a general IO/process
framework, codec negotiation, stable migrations, or compatibility shims.

## Baseline Evidence

- Restore runner journal persistence currently uses truncating `fs::write` at
  one owner function called after seven runner/preview transitions.
- CLI restore preparation also writes backup-owned plan/journal types through
  generic truncating output helpers.
- CLI build and host install each own a production `BuildEnvGuard` with unsafe
  process-global environment mutation.
- `canic-core` and `canic-host` directly depend on `serde_cbor 0.11.2`.
- Default Wasm `canic-core` selects `serde_cbor` directly.
- `ciborium 0.2.2` is already selected through `ic-memory 0.9.0`.
- All-feature auth and host graphs may retain transitive `serde_cbor` through
  upstream IC crates even after Canic's direct edges are removed.
- The core stable macros currently cover 37 Canic-owned macro-backed types;
  replay receipts also use direct CBOR storage encoding.

## Implementation Checklist

### Slice A - Durable recovery persistence

- [x] Add one private unique-sibling durable replacement primitive in
      `canic-backup`.
- [x] Serialize complete recovery documents before opening a staging file.
- [x] Sync staging bytes, rename, and sync the parent directory.
- [x] Clean staging files on pre-rename errors.
- [x] Route backup persistence documents through the same primitive.
- [x] Route every restore runner journal transition through the primitive.
- [x] Expose only typed restore plan/journal persistence to `canic-cli`.
- [x] Remove direct truncating recovery-document writes.
- [x] Add focused durability and runner transition tests.

### Slice B - Explicit build authority

- [x] Extend or replace `WorkspaceBuildContext` as the one host-owned build
      authority value.
- [x] Resolve workspace, ICP root, config, role, profile, environment, build
      network, and local replica inputs once.
- [x] Apply required values directly to child Cargo/ICP commands.
- [x] Make display and provenance consume the typed value.
- [x] Delete CLI `BuildEnvGuard`.
- [x] Delete install `BuildEnvGuard`.
- [x] Remove production unsafe environment mutation from build/install paths.
- [x] Add focused context parity, no-leakage, and child-environment tests.

### Slice C - CBOR proof and hard cut

- [x] Prove exact legacy/current bytes across the active macro-backed owner
      suites and retain one rich stable-shape golden.
- [x] Freeze exact replay receipt bytes.
- [x] Preserve every existing bounded record size declaration.
- [x] Add exact local replica query/status CBOR wire fixtures.
- [x] Replace host direct codec use behind one private adapter.
- [x] Compare the candidate core codec across the stable owner suites.
- [x] Since all comparisons match, hard-cut the core adapter and remove the direct
      core dependency.
- [x] Confirm no fixture differs; add no fallback or migration.
- [x] Record default Wasm, all-feature Wasm, host, and workspace dependency
      trees after the decision.
- [x] Ensure release notes distinguish direct removal from upstream transitive
      retention.

## Validation Status

Slice A targeted validation:

- `cargo check -p canic-backup --lib`: pass.
- `cargo check -p canic-cli --lib`: pass.
- durable JSON replacement, serialization-failure preservation, and
  rename-failure cleanup: 3 passed.
- backup layout manifest, download journal, backup plan, and execution journal
  round trips: 4 passed.
- restore runner terminal-receipt validation: 3 passed.
- CLI default restore preparation: 1 passed.
- CLI completed mutating restore operation: 1 passed.
- CLI failed-operation, retry-failed, and unclaim-pending journal transitions:
  3 passed.
- `cargo clippy -p canic-backup --lib -- -D warnings`: pass.
- `cargo clippy -p canic-cli --lib -- -D warnings`: pass.

Slice B targeted validation:

- `cargo check -p canic-host -p canic-cli`: pass.
- `cargo check -p canic-host --example build_artifact`: pass.
- explicit child build environments, direct-local target application, and
  sequential no-leakage: 6 passed.
- install ICP command targeting: 9 passed.
- named-environment install context: 1 passed.
- install operation authority wiring: 10 passed.
- CLI build context, config, environment, and provenance behavior: 13 passed.
- targeted host and CLI Clippy with warnings denied: pass.
- production build/install scans contain neither `BuildEnvGuard` nor unsafe
  process-global environment mutation.

Slice C host-wire validation:

- anonymous query envelope, replied response, rejected response, byte-valued
  status root key, and malformed/unsupported CBOR goldens: 5 passed unchanged
  before and after the codec switch.
- existing JSON/CBOR status root-key behavior: 3 passed.
- `canic-host` has no direct dependency on or source reference to
  `serde_cbor`; its private adapter declares and uses `ciborium 0.2.2`.
- the host graph still selects transitive `serde_cbor 0.11.2` through
  `ic-agent` and `ic-transport-types`, as expected.
- targeted host Clippy with warnings denied: pass.

Slice C stable-byte validation:

- temporary dual-codec differential proof across default core storage: 62
  passed with byte equality enforced at the adapter.
- temporary dual-codec differential proof across all-feature core storage: 115
  passed with byte equality enforced at the adapter.
- temporary dual-codec differential proof across control-plane storage: 38
  passed with byte equality enforced at the adapter.
- permanent rich serde-shape and replay receipt byte goldens: 2 passed.
- default, all-feature, and Wasm core checks: pass.
- `canic-core` and `canic-host` manifests and source contain no direct
  `serde_cbor` dependency or codec call.
- default `canic-core` selects neither `serde_cbor` nor `half 1.8`; all-feature,
  host, and workspace graphs retain upstream copies through published IC
  signature, agent, transport, and PocketIC crates.
- current published upstream releases still depend on `serde_cbor`:
  `ic-canister-sig-creation 1.3.1`, `ic-signature-verification 0.3.0`,
  `ic-agent 0.48.1`, and `pocket-ic 14.0.0`.
- targeted core and control-plane Clippy with warnings denied: pass.

Current release-set transport validation:

- binary Candid mode selects `--args-format bin` and preserves arbitrary stdin
  bytes, including zero bytes.
- one local artifact preflight verifies size, canonical chunk size, payload
  hash, chunk count, and every chunk hash before the first root mutation.
- manifest emission and staging reject empty, absolute, parent-traversal, and
  symlink-escaping artifact paths before reading bytes.
- staging rejects empty versions and roles, duplicate roles, and template IDs
  that do not exactly match `embedded:<role>` before the first root mutation.
- manifest admission rejects zero payload sizes, noncanonical chunk sizes,
  impossible chunk counts, and malformed or non-SHA-256-length hex digests.
- a maximum-size chunk request round-trips through the canonical endpoint DTO
  below the configured payload limit.
- 62 focused release-set tests and targeted host Clippy pass.
- `ic-query 0.10.0` cached subnet-catalog integration: 1 passed.
- targeted `canic-host` check against `ic-query 0.10.0`: pass.

Current `ic-memory 0.10` hard-cut validation:

- default, all-feature, and `wasm32-unknown-unknown` core checks: pass.
- exact empty, valid, and invalid commit-slot projection: 1 passed.
- invalid-slot recovery classification: 1 passed.
- memory diagnostic Candid enum round trip: 1 passed.
- targeted core Clippy with warnings denied: pass.

Required validation is targeted by slice. Do not run the full test suite as
part of normal development; the maintainer performs deployment/release
validation.

## Decisions And Drift Log

- 2026-07-11: initial design created from the post-0.84 codebase audit.
- 2026-07-11: IC CBOR wire requirements were explicitly separated from use of
  the `serde_cbor` crate.
- 2026-07-11: `ciborium` selected as the first candidate because it is already
  in the default Wasm graph through `ic-memory`; exact stable bytes remain the
  decision gate.
- 2026-07-11: stable mismatch policy is stop-and-revise, not dual decoding or
  automatic migration.
- 2026-07-11: Slice A hard-cut the fixed `.tmp` backup writer and truncating
  restore journal writer to one unique sibling durable replacement primitive.
  Backup layout documents and typed restore plan/journal entrypoints now share
  it; generic presentation output remains unchanged.
- 2026-07-11: Slice B hard-cut both process-global build environment guards.
  `WorkspaceBuildContext` now carries the exact role, profile, roots, config,
  selected environment, resolved build network, and optional direct-local
  replica target. Cargo receives command-local build authority; install ICP
  commands receive the typed local target explicitly. The retired internal
  build-environment override and local-target environment fallback are removed.
- 2026-07-11: Slice C's independent host wire gate passed. Exact IC request,
  reply, rejection, status root-key, and invalid-input fixtures are identical
  under `ciborium`; the direct host `serde_cbor` edge and codec-specific public
  error type are hard-cut. Transitive upstream copies remain accurately
  reported.
- 2026-07-11: the stable proof used one temporary dual encoder at the canonical
  adapter across the maintained core and control-plane owner suites instead of
  adding 37 duplicate fixture frameworks. Every exercised legacy/current byte
  comparison matched. The legacy encoder was then removed; a rich serde-shape
  golden and an exact replay receipt golden remain as permanent adapter guards.
  Existing per-owner round-trip and bound tests remain authoritative for their
  record values and sizes.
- 2026-07-11: the workspace advanced to `ic-memory 0.10.0`. Canic now consumes
  enum-based commit-slot diagnostics, the combined recovery result, and
  retirement generations carried by `AllocationState::Retired`. The newly
  meaningful invalid-slot recovery case is exposed exactly rather than folded
  into `Unknown`; no adapter for the 0.9 shape remains.
- 2026-07-11: slices A, B, and C were published as `v0.85.0`. The next focused
  cleanup hard-cuts release-set Candid argument files to piped child stdin so
  large call payloads are neither persisted nor placed in process arguments.
- 2026-07-11: the stdin transport cleanup was published as `v0.85.1`. The next
  cleanup encodes the canonical template request DTOs as binary Candid, deletes
  the manual release-set text encoders, validates the complete artifact
  manifest before root mutation, and keeps a maximum chunk request under the
  endpoint payload bound.
- 2026-07-12: the typed binary staging cleanup was published as `v0.85.2`. The
  next focused cleanup constrains every release artifact read to a canonical
  target inside the canonical ICP project root.
- 2026-07-12: release artifact path containment was published as `v0.85.3`. The
  next focused cleanup validates release-set version, role, uniqueness, and
  exact embedded template identity before staging begins.
- 2026-07-12: canonical manifest identity admission was published as
  `v0.85.4`. The next focused cleanup extends the same validator to the static
  payload, chunk, and SHA-256 shape before artifact access or root mutation and
  adopts `ic-query 0.10.0` after its canonical-library-path review passed.

## Next Action

Run the human-owned `0.85.5` release flow after reviewing the finalized patch.
Do not claim that `serde_cbor` left the workspace lock: current published IC
signature, agent, transport, and PocketIC dependencies still select the
upstream crate.
