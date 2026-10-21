# Platform dependency boundary

Canic selects published **ic-platform 0.1.12**, verified with the crates.io registry
and its downloaded archive on 2026-10-18. The three Cargo lockfiles retain the
registry checksum. No sibling checkout, path override or unpublished API is used.
The published crate requires Rust 1.88; Canic's maintained MSRV remains 1.91.

## Supported ownership

Use `ic_platform::Principal` without features for principal-only code. Select
`candid` for `ic_platform::candid::contract` types, traits and encoding. Derives
must select their generated paths explicitly:

```rust
#[derive(ic_platform::candid::contract::CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
struct Reply {
    owner: ic_platform::Principal,
}
```

A workspace catalog declares `ic-platform = { version = "0.1.12",
default-features = false }`; members inherit it and select the needed features.
Consumers expanding the exported contract macros need the `candid` feature too.
Generated Fleet Root, coordinator and store manifests select the exact Platform
version from Core's resolved normal dependency with its `cdk` feature; generated
Fleet packages declare neither Candid nor CDK directly. Application protocols owned by IC Wire stay on IC Wire.

- Contracts, Core, Host, CLI and both internal test packages have no direct
  Candid dependency. Default decoders and structural subtype qualification use
  Platform 0.1.7’s exact upstream functions and error identities.
- Backup selects no Platform features. This removes its direct `ic_principal`
  declaration, but Backup's upstream dependencies still include Candid.
- Core and Host use `certification` for the existing upstream certificate/tree
  identities. Core retains its conditional auth feature selection.
- Host's six release decoders use Platform's bounded decoding with unchanged
  byte/work/skipping/type/header limits, collection debits and domain failures.
  Testkit 0.54 callers use the same Platform limits identity.
- Configured and staged Candid decoders preserve the upstream `DecoderConfig`
  and `IDLDeserialize` identities through Platform. Service registration/export,
  reflection, type equality, macros and Principal errors use the new 0.1.5 exports.
  Service generation passes the explicit `ic_platform::candid::contract` path.
- Host parsing and JavaScript/TypeScript generation use Platform's native-only
  `candid-parser` feature. Host and internal tests select it normally; facade
  surface tests select it as a development dependency. No direct parser remains.
- Generated method contexts use Platform's `executor` exports, which are the
  exact executor functions previously called through CDK's forwarding functions.
  Core's broad Candid reexport and the contract macros' Candid reexport are gone.
  Hidden Platform exports support macro hygiene, not a competing implementation.

Host and native delegation tests select Platform `agent`; the runtime probe
selects `management-canister-types`. Native Agent retains 0.49.2 with PEM support;
management HTTP types retain 0.11.0. Certificates use Platform certification with
the same upstream identity. Direct declarations of both packages are removed and
forbidden in all three policies. They remain transitive dependencies.

Host now selects `icrc-ledger-types` for upstream 0.2.0 account, transfer,
block/certificate and archive types. The facade selects `ic0` for its unchanged
1.2.0 raw argument functions. Direct ledger/IC0 declarations are removed and
forbidden in all three policies; these packages remain transitive. No ledger
storage integration is enabled. Existing partial management-response projections
retain their decoder shape rather than requiring unused history fields.

Core selects `management-canister-types` for `CanisterIdRecord`,
`CanisterInfoArgs`, `Snapshot`, `ChunkHash` and `DeleteCanisterSnapshotArgs`;
all five local mirrors are deleted. Intentional partial response projections remain.
This adds no storage integration or lifecycle owner.

## Direct dependency enforcement

All three maintained workspaces forbid direct Candid and CDK-family declarations.
Platform 0.1.12 supplies the remaining system, signing-cost, certificate/stable
and typed call-error identities requested in [#22](https://github.com/dragginzgame/ic-platform/issues/22).
Canic uses the original upstream implementations; no local substitute APIs or
storage migration were added. Boundary guards recognize the Platform paths.

The exact forbidden package policy lives in each workspace's
`ci/cargo-dependency-boundaries.json`, covering aliases and all dependency kinds.
Transitive upstream packages remain, as described below.

Do not replace CDK `futures::spawn` with Platform `spawn_protected`: CDK wraps the
future with an incomplete-task drop panic outside trap recovery. The raw executor
operation corresponds to CDK `spawn_weak`. Use Platform `cdk::futures::spawn`, which forwards the original CDK behavior. Configured decoders now use Platform’s explicit upstream configuration/staging
exports. They do not substitute mandatory finite limits or compact diagnostics
for existing policy. Structural subtyping is still distinct from type equality;
the existing projection tests use Platform’s exact upstream subtype export. Default
decoders use `candid::decoder`, not the parent’s finite-limit helpers. The hidden
facade Candid reexport is removed; exported endpoint macros use Platform directly.

## Enforcement and resolved graphs

`make cargo-dependency-boundaries-gate` invokes the adopted Shared Tooling 0.24
checker for all three workspace roots. Both `check-invariants` and `ci-preflight`
require it. Cargo metadata supplies canonical package names, so renamed aliases,
normal/build/development dependencies, optional entries and target-specific
entries are covered. Shared fixtures exercise these cases and failed/malformed
metadata observations.

The common policy is a package allowlist. It does **not** constrain feature names,
dependency kind or target within an allowed package, inspect transitive edges,
or attest the installed Wasm. Feature checks and graph inspection are separate
qualification evidence; automatic deployment-role exclusion/artifact binding is
still missing, as recorded in [Canic #602](https://github.com/dragginzgame/canic/issues/602).

The resolved graphs retain Candid 0.10.38 and its derive implementation, CDK 0.20.3,
CDK macros 0.20.3 and one executor identity, 2.0.0. Platform itself forwards to the
upstream owners; removing direct declarations does not remove these packages.
Native Host retains candid_parser 0.4.1 transitively through Platform. The selected Canic Wasm normal-dependency
closure (default and all features) and both independent consumers (all features)
have no Candid parser and resolve the same executor identity. Contracts selects only Platform `candid`; Backup's
Platform selection is featureless. Platform's executor-only capability is checked
separately from Canic's combined runtime selection.

Lock resolution also reconciles the maintainer's pending Build 0.17, Blob 0.33,
Query 0.62 and Metrics 0.14 catalog changes. Those constraints are preserved;
this Platform update does not migrate their APIs or any storage/memory state.

## Qualification limits

Focused evidence is recorded in the current handoff. Contracts' wire/projection
corpus, Backup tests and macro tests exercise the migrated interfaces. Native
Linux and Wasm cross-compilation are not native macOS or installed-IC evidence.
Core's existing Memory/Storage compilation failures prevent runtime-dependent
Host, generated-package, lifecycle and installation qualification. Retaining the
necessary direct dependencies is intentional; it is not evidence that the whole
0.112 release batch is ready to push.

Testkit 0.54 callers use Platform `DecodeLimits` with their existing finite budgets.
Direct `ic-testkit-candid` dependencies are forbidden in all three workspaces.
The selected Query graph may still retain it transitively until its owner migrates.

Platform 0.1.12 entrypoint generation uses its maintained macro fork. Apps, probes,
scaffolds and Fleet generators select `cdk`; custom Canic raw adapters retain their
existing decoder/guard order. Host BLS fixture keys select only Platform `bls`.
Direct dependency removal does not remove upstream Candid/CDK or BLS transitively.

Platform 0.1.12 also owns Core call-context instruction reads and incoming-cycle
observation/acceptance, including the funding probe. The remaining CDK exports are now supplied in 0.1.12; this does not change cycle policy.
