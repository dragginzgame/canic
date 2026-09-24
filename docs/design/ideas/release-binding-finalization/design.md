# Reusable compilation and release binding

Date: 2026-09-24. Proposal only; not scheduled or implementation authority.
Owner: `canic-host` build/artifact pipeline, with `canic` macro and `canic-core`
activation participation. This is separate from the bounded host deployment
observation improvements and does not begin another minor.

The [first feasibility checkpoint](../../../audits/reports/2026-09/2026-09-24/release-binding-feasibility.md)
passes structural finalization and executable identity/rejection checks on one
optimized synthetic module in PocketIC. It does not qualify production roles,
artifact admission or a build-speed gain; the proposal remains unscheduled.

## Problem and intended result

The [controlled build matrix](../../../audits/reports/2026-09/2026-09-24/toko-performance-followup.md)
found that qualification-only input changes trigger runtime recompilation and
change all eight Wasm hashes. `WorkspaceBuildContext::apply_to_command` supplies
`CANIC_RELEASE_BUILD_ID`; `canic::build!` forwards it and `canic::start!` embeds
it in executable lifecycle and metadata surfaces. Activation checks that binding.

The intended result is to keep a new release's identity out of reusable
compilation, then produce a distinct, exactly bound deployable artifact cheaply.
A new qualification identity must still get a new release record. Neither
ignoring Markdown nor relabelling old Wasm preserves the current contract.

## Recommended feasibility direction

Prototype explicit Wasm finalization against an intentionally declared binding
slot. Do not start by changing cache admission or the source inventory.

1. Compile and optimize a release-neutral template under a stable build context.
   Keep the existing conservative input qualification: sources, build scripts,
   generated inputs, manifests, lockfile, features, target, toolchain, profile,
   network and relevant environment. Non-Rust inputs remain part of the evidence.
   When qualification changes, run Cargo again; with a stable compilation
   environment Cargo may verify that no runtime rebuild is needed. Do not infer
   equivalence from file extensions or reuse across unverified build inputs.
2. Replace compile-time release constants with a deliberately runtime-read
   binding slot, owned by the generated entrypoint. The compiler/optimizer must
   be unable to fold the placeholder into code, duplicate it, or omit a consumer.
   A named structural descriptor identifies the exact slot and its layout in
   the final optimized module. An unconstrained byte/string search is unsuitable.
   The unbound template must fail closed at activation and must never be selected
   as a deployable artifact.
3. Finalize a private copy for the already-durable release plan. Validate the
   exact input hash, supported descriptor, memory/data-segment bounds, unique
   slot, and unbound contents; fill the slot with the plan's canonical release
   identity. Reject already-bound, absent, duplicated or malformed slots.
   Preserve all other executable contents. Validate the resulting Wasm and
   binding, then derive final hashes, deterministic compressed payloads and
   ordinary release artifacts from those bytes.
4. Retain a structured finalization record binding template hash, full input
   qualification digest, finalizer/tool identity, release identity and every
   final artifact hash. Publish atomically under the existing release lock;
   retained artifacts are reusable only after exact verification. Interrupted
   finalization may restart from the verified template or reuse an exact
   completed result; it must not mutate another release's artifacts.

Use the maintained pre-1.0 contract through a hard cut. Any new Canic-owned
descriptor/record begins at v1. Do not add a legacy compiled-constant fallback,
mixed-generation lane, migration or cross-release upgrade promise. Reinstall
and same-release interruption recovery remain the operational boundary.

## Why a proof is needed first

A custom metadata section alone cannot establish the release identity used by
executable startup and activation. A data-slot proposal is viable only if every
consumer reads that slot after the supported Rust/LTO/`ic-wasm` pipeline and the
descriptor remains correct after optimization. Rust volatile loads may be part
of that implementation, but are not by themselves a proof of the whole toolchain.
The prototype must inspect optimized code and exercise the actual lifecycle.

| Alternative | Assessment |
| --- | --- |
| Reuse existing Wasm under a different release record | Reject: executable identity remains stale. |
| Remove qualification inputs from the cache key | Reject: arbitrary build scripts may consume them. |
| Small release-specific linking unit | Retain as fallback research if slot finalization fails; demonstrate that Cargo dependency invalidation and whole-program LTO do not repeat the expensive work. |
| Change optimization profile | Separate tradeoff: requires renewed size and instruction qualification; does not remove release identity from compilation. |
| Structural slot finalization | Preferred feasibility experiment: bounded byte changes after compilation, conditional on executable-binding and optimizer proof. |

## Acceptance and release-batch boundary

First produce a small isolated Canic fixture and an artifact inspection report.
It must demonstrate two finalizations from one verified template: distinct final
hashes and release identities, unchanged non-binding code/data, and deterministic
repeat output. Cover every generated infrastructure role and an application
entrypoint; inventory every executable and metadata release-identity consumer.

PocketIC must prove that the finalized identity is the one observed and enforced
at startup/activation, including rejection of a different reviewed identity and
an unbound template. Test tampered templates, changed tool/config/environment
inputs, corrupted finalization records, output replacement, interruption before
publication and exact retry. Preserve isolated mutable Cargo targets and lock
ownership on relocation/concurrent callers.

Only then propose a complete implementation batch spanning build admission,
macros, activation, artifact extraction/publication, diagnostics and docs.
Compare unchanged, qualification-only, gameplay, dependency and relocated builds
on matched sources/tools. Retain Cargo compilation/link/finalization time,
controlled warm time, peak process resources, Wasm footprint and relevant runtime
instruction costs. No numerical speedup or new timing threshold is accepted by
this proposal; the retained .39 timings identify the opportunity only.

If the structural or executable proof fails, keep the current exact identity
contract and report that result before choosing another implementation.
