# Idea: Language-Neutral Managed-Guest Feasibility

Reviewed: 2026-09-06

## Status

- Deferred and unnumbered. No investigation, production ABI, builder, SDK or
  release is approved by this note.
- Retained need: determine whether an application-owned Motoko Component can
  satisfy the same managed contract as a Rust Component.
- Owners: runtime/role-contract and host build qualification owners.
- Product boundary: Canic infrastructure, IcyDB and existing Rust products
  remain Rust. Motoko is considered only for application-owned guests.
- Repository scope: Canic only; external repositories remain read-only.

## Current Baseline

The [role schema](../../../../crates/canic-core/src/config/schema/role.rs) and
[host artifact builder](../../../../crates/canic-host/src/canister_build/artifact.rs)
qualify Cargo packages. The current init payload and guest implementation
remain Rust-owned.

Ordinary managed roles expose `canic_command` and `canic_status`.
The [role endpoint macros](../../../../crates/canic/src/macros/endpoints/role.rs)
dispatch `ConfigureRuntime` and expose its operation status. Preparation,
Directory synchronization and activation are lifecycle semantics beneath that
surface, not separate public methods to recreate in a Motoko SDK.

Fleet Coordinator, Root and Store retain their distinct role-owned
command/status pairs. A guest implementation must not collapse those role
boundaries or add one method per phase.

Wasm storage, hashes, exact artifact selection and snapshot recovery already
operate on bytes and identities. Installing arbitrary Wasm does not qualify
its lifecycle, protected binding, admission or readiness.

## Proposed Feasibility Boundary

A future bounded investigation should answer three questions:

1. Can Rust and Motoko fixtures decode and enforce the same compact
   Root-issued authority without relying on language-specific re-encoding?
2. Can an exact Mops/`moc` build produce reproducible, qualified artifacts
   without modifying application source or dependency files?
3. Can an explicit Motoko scaffold satisfy current lifecycle, exact retry,
   interruption and application-readiness semantics within the runtime budget?

The investigation would use fixtures only. Passing it would justify a later
production design, not publish managed Motoko support or a replacement ABI.

## Contract Constraints

- Inventory the maintained command/status and init contracts first. Any
  fixture-only alternative must identify its exact production delta and keep
  the same authority boundaries.
- Freeze one current candidate contract with finite payload and collection
  bounds. Use v1 if a Canic-owned generation identifier is needed; do not
  introduce negotiation or parallel compatibility lanes.
- Root-issued frozen payload bytes may be hashed and retained verbatim.
  Validate their bounded semantic contents; do not substitute a digest of a
  Motoko re-encoding.
- Guest-originated requests require their own canonical request binding.
  Root-issued opaque evidence cannot authorize arbitrary guest bytes.
- Bind Fleet, Root, Component, role, caller, Directory and operation identity
  explicitly. Prove stale, conflicting and wrong-authority rejection.
- Preserve synchronous restoration before deferred work, exact hook ordering,
  readiness fencing and same-release interruption recovery. A same-release
  fixture upgrade tests those invariants only.
- Parse Candid structurally and prove the required service-subtyping direction
  with extra-method, missing-method, type and annotation cases.
- Admit only a bounded supported capability profile. Unsupported auth,
  metrics, child provisioning, sharding or scaling must fail qualification.

## Build Proof

Pin exact Mops, compiler, dependency and source evidence. Separate dependency
acquisition from the build, stage generated identity in an isolated directory,
and verify that checked-in manifests, locks and source remain unchanged.

An explicit actor scaffold owns endpoint declarations. Do not silently rewrite
application source or introduce an arbitrary shell builder. Retain exact Wasm,
Candid and relevant compiler evidence; establish clean-build reproducibility
and reject tool, lock, source, output and capability mismatches.

A reproducible artifact is not proof that arbitrary guest code honors the
runtime protocol. The trust model must state what conformance fixtures prove
and what remains an application-maintainer obligation.

## Evidence Before Scheduling Production Work

- One complete bounded candidate contract and shared positive/negative
  frozen-byte vectors.
- Rust and real Motoko fixtures passing the same PocketIC binding,
  configuration, status, readiness, wrong-caller and stale-authority cases.
- Lost-response, exact replay, conflicting request, hook failure, restart and
  same-release fixture-upgrade evidence.
- Isolated build/scaffold proof, structural Candid conformance and measured
  Wasm/build/maintenance cost.
- A go/no-go report identifying every required production change and
  unresolved platform or security limitation.
- A separately accepted release position and complete implementation batch.

## Disposition

Retain a feasibility question, not a promised SDK. The old detailed ABI,
configuration and milestone sketches are removed in favor of the current
command/status baseline.

Production Cargo/Mops configuration, Rust ABI changes, a public Mops SDK,
durable child provisioning, cycles requests and database-service integration
remain outside this proof. Broader application possibilities are retained in
[the managed-guest exploration](exploration.md). Release transitions remain
reinstall-only; no state migration or cross-release decoder is proposed.
