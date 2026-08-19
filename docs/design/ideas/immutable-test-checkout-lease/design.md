# Idea: Immutable Cross-Process Test Checkout Lease

Date: 2026-08-19

## Status

- Classification: deferred, unnumbered idea. It is not a scheduled release or
  implementation authority.
- Need: remove repeated exact Wasm-input resolution across the governed Cargo
  test processes without asserting source immutability that Canic cannot
  enforce.
- Current safety: ordinary `ic-testkit` batches continue to resolve and
  validate their own inputs. This idea is a performance opportunity, not a
  correctness or 0.104 release blocker.
- Ownership: Canic owns the checkout and runner enforcement boundary;
  `ic-testkit` may own a generic prepared-input service or cross-process
  snapshot format only after that boundary exists.
- Repository scope: Canic only. The sibling `ic-testkit` repository remains
  read-only unless the maintainer separately authorizes work there.

## Decision Direction

A future governed test runner may prepare one exact source snapshot and expose
it read-only to the complete Cargo/test process tree. Every reusable Wasm input
snapshot must be tied to that real runner-held lease. A lock file, unrelated
lifetime token, clean-tree check, process-local `OnceLock`, permission-only
checkout or read-only view of the live worktree is insufficient.

The first implementation should be limited to clean committed CI and release
runs:

1. record the exact Git tree and materialize it into invocation-private
   scratch;
2. prefetch the locked dependency graph and resolve the admitted toolchain;
3. launch the complete workspace-test runner in one kernel-enforced sandbox
   with the source snapshot mounted read-only;
4. expose Cargo targets, exact artifact caches, PocketIC scratch and other
   generated state through separate writable roots outside the snapshot;
5. freeze every declared build argument, relevant environment value,
   additional input and Cargo/rustc executable identity for the lease;
6. admit only a predeclared set of exact `WasmBuildSpec` values; and
7. share invalidation across every Cargo test process so one detected mismatch
   rejects later readers and the complete run.

Dirty development runs remain supported through ordinary independently
validated batches. They must not receive the immutable-source fast path merely
because the runner can compute a starting digest.

## Required Canic Preparation

Before the checkout can be honestly mounted read-only, Canic must:

- inventory every source, manifest, lockfile, Cargo configuration, toolchain
  selector, build configuration and explicit additional input;
- externalize workspace-relative `.canic`, `.icp`, `target`, test scratch and
  other generated outputs so the source view needs no writable holes;
- define one canonical inventory of every Wasm build specification used by the
  governed PocketIC lane;
- prove that every Cargo invocation remains a descendant of the runner that
  owns the lease; and
- retain the existing exact-cache coordination, source-race rejection and
  artifact-content verification beneath the new optimization.

The preferred Linux mechanism is a private frozen source snapshot plus one
read-only mount namespace around the complete runner process tree. A read-only
bind of the live checkout alone is rejected because an editor outside that
namespace can still mutate its backing files.

## Possible `ic-testkit` Boundary

The current process-local prepared snapshot is correctly conservative and
should remain unused by Canic until enforcement exists. Afterward, a generic
upstream primitive could use either:

- one runner-owned artifact service that retains the prepared resolution and
  answers exact specification requests over a private local channel; or
- a bounded cross-process prepared-resolution manifest tied to the Canic lease
  identity and one shared invalidation record.

Either form should:

- reject an undeclared or non-identical specification before cache or build
  work;
- distinguish preparation, reader reuse and fallback resolution in metrics;
- never infer immutability from an environment boolean or arbitrary borrowed
  token;
- preserve before/after mutation detection and atomic publication; and
- let Canic supply the enforcement boundary without teaching `ic-testkit`
  Canic-specific checkout, Git or CI policy.

## Non-Goals

This idea does not:

- change application, canister, Candid or stable-state behavior;
- make 0.104 depend on a test-performance optimization;
- weaken ordinary input validation or exact artifact fingerprints;
- promise a portable hostile-code sandbox;
- treat advisory locks as authority over editors or arbitrary same-user
  processes;
- merge the governed Cargo test processes into one oversized test binary; or
- authorize a change in `ic-testkit` or another repository.

## Evidence Required Before Promotion

- measured input-resolution cost across the complete governed PocketIC lane;
- a complete protected-input and externalized-output inventory;
- a test proving every Cargo/test child observes the same exact source tree;
- rejected source writes from inside the process tree;
- proof that mutation of the live checkout cannot change the frozen test
  source;
- exact rejection of undeclared specifications, tool/environment mismatch and
  stale lease identity;
- shared invalidation after a deliberately detected race;
- successful local and CI operation on the admitted runner platforms; and
- a benchmark showing material wall-time or resource improvement without
  changing artifact identity or failure attribution.

Promotion requires a concrete performance need, an accepted release position,
a complete batch plan and explicit maintainer approval. Until then, Canic
keeps the safer independent-resolution path.
