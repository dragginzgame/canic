# 0.97 Role-Owned Runtime Dependencies And CDK Surface - Status

Last updated: 2026-07-22

## Current State

0.97 is accepted and active after completing audit-only Slice A against
released `v0.96.8`, releasing bounded Slice B as `v0.97.0`, hidden
macro-boundary Slice C as `v0.97.1`, and public-facade Slice D as `v0.97.2`.
Slice E is implemented in the open `0.97.3` batch: manual ICP conversion is
owned by the root role, its standard controller endpoint, root-only config and
state, and the deployment-root CLI. The required
relocation decisions are frozen in the
[Slice A report](../../audits/reports/2026-07/2026-07-22/0.97-slice-a-surface-and-graph-inventory.md).
Slice B gives every authoritative role-contract caller one canonical Cargo
evidence producer and hard-cuts the declaration and protected-graph
contradictions identified by Slice A. No package-version change, downstream
edit is part of the current slice.

The proposal gives each configured role package sole external authority over
Canic-owned framework packages and directly selected Canic capability
features. It freezes explicit Cargo resolver 2, disabled workspace Canic
defaults, no workspace Canic features, canonical dependency key `canic`, one
direct normal edge, and a separately purpose-bound build edge. One canonical
`RoleCargoGraphEvidence` producer supplies every validator caller.

The design retains the package-graph validator, extends it to protected Canic
framework packages, carries exact dependency paths as structured evidence,
separates macro plumbing into the existing hidden `canic::__internal`
boundary, and unconditionally hard-cuts the human-facing `canic::cdk` facade.
The surface inventory decides semantic destinations and migration
instructions; it cannot retain the facade.

## Current Evidence

- Canic already requires one direct, unconditional, non-optional normal
  dependency from each role package to package `canic`.
- The validator rejects every sibling path to a protected Canic package and
  renders the exact normalized dependency chain without raw Cargo IDs or
  absolute paths.
- Cargo resolver 2 does not prevent feature union across two normal runtime
  paths to one package.
- Cargo workspace dependency features are additive, and Canic defaults remain
  enabled unless every contributing declaration disables them.
- A renamed Canic dependency is rejected because procedural macro output uses
  the source-level crate name `canic`.
- Cargo metadata must be filtered and feature-selected to match the exact role
  build; otherwise callers can validate different graphs.
- The former public `canic::cdk` facade mixed upstream conveniences,
  Canic-owned helpers and types, and macro-expansion plumbing; Slice D deletes
  it after separating those owners.
- Canic already has a hidden `canic::__internal` macro boundary.
- The standalone `canic-cdk` package was removed in 0.43.3 and is not restored
  by this proposal.
- A separate Canic-dependent `toko-canic` helper would recreate the forbidden
  transitive path; Canic-specific integration therefore belongs inside the
  final role package.
- The exact inventory covers 26 role-shaped packages. Every maintained role
  now has the canonical direct dependency key, an explicit normal feature
  list, disabled workspace Canic defaults, and a feature-empty
  purpose-bearing build edge. Previously implicit `metrics` activation is
  explicit, and `delegation_root_stub` reaches control-plane DTOs through the
  semantic Canic facade rather than a sibling protected-package edge.
- The all-feature Canic normal closure contains exactly the four protected
  framework packages; no allowed non-protected Canic-owned implementation
  package is needed.
- `cargo metadata --manifest-path <member>` still reports workspace-union
  activated features in this virtual workspace, while Cargo 1.97 tree output
  can retain target-inactive packages. Canonical evidence therefore
  intersects complete package identity, target-filtered metadata edges, and
  one package-selected tree per exact role. No single Cargo view is treated as
  the selected Wasm graph.
- Isolated real Cargo workspaces prove the supported shape, renamed-Canic
  rejection, protected sibling-path rejection, resolver hard cut, explicit
  feature requirements, and purpose-bound build edge. The source fixtures do
  not retain generated lockfiles.
- The published Slice A lexical inventory missed grouped imports such as
  `use canic::{cdk::...}` in maintained test fleets, stubs, and integration
  support. Direct Slice D inspection corrected that evidence before deletion;
  all grouped and fully qualified consumers now use direct upstream IC imports
  or existing Canic-internal helpers.
- Canic cycle values now live publicly under `dto::cycles`, and public ICRC-21
  protocol DTOs live under `dto::icrc21`. The former internal ICRC-21 path is
  removed rather than retained as an alias.
- ICP refill has no dedicated timer. Slice E deletes its generic feature,
  capability, facade API/emitter, arbitrary target and fabrication paths,
  non-root lifecycle/metrics access, child-funding integration, and three
  unproduced stable error variants. The executing root is the derived source
  and target, and memory ID 33 is root-owned and lazily opened only by root
  lifecycle or metrics.
- Declarative lifecycle and endpoint macros now use definition-owned hidden
  CDK paths; procedural endpoint expansion uses the canonical hidden Canic
  path. The compiler boundary exposes only the frozen six macros, `Principal`,
  and five required API functions.
- Generic IC caller/self and Candid derive conveniences no longer arrive
  through `canic::prelude`; maintained demo callers use their upstream owners.
- Locked internal PocketIC builds now request locked online role evidence,
  distinct from unlocked development builds and locked-offline passive
  inspection. Focused validation proves the repository lockfile does not
  change.

## Slice A Evidence

All seven required evidence groups are complete in the canonical Slice A
report: public surface/consumer classification, 26-role declarations and build
owners, protected closure and feature fixtures, semantic relocations, graph
callers, normalized path semantics, isolated validation matrix, and the exact
refill baseline/deletion set.

## Slice B Validation

- 31 host role-contract tests pass, including three real isolated Cargo
  workspaces, resolver/default/feature/build-edge hard cuts, target filtering,
  deterministic protected paths, bounded parsing/traversal, the repository
  protected closure, the built-in Wasm store, and internal PocketIC packages.
- The generated Wasm-store wrapper contract and exact blob-role state-manifest
  regressions pass against the canonical feature/declaration evidence.
- 51 CLI Medic role/config tests, 14 core role-contract tests, all 21 host
  state-manifest tests, three generated-bootstrap-store tests, seven
  workspace-manifest tests, and changelog governance pass.
- Strict all-target Clippy passes for `canic-core`, `canic-host`, and
  `canic-cli`; the delegation-root and built-in Wasm-store packages check with
  their semantic facade and canonical feature declarations.
- Formatting and diff hygiene pass. Repository-root project Medic retains the
  separately existing cross-fleet role-name ambiguity in state-manifest
  discovery; it reports no role dependency-shape finding.

## Slice C Validation

- All 36 procedural-macro tests and 25 focused Canic endpoint, protocol, and
  reference-surface tests pass.
- All 22 focused role-package tests pass, including byte-for-byte lockfile
  preservation for locked internal PocketIC evidence.
- Strict all-target Clippy passes for the three changed packages. The
  delegation-root, blob-storage probe, demo hub/shard, and built-in Wasm-store
  packages compile with the hidden boundary.
- Formatting and diff hygiene pass. Candid, stable records, CLI output, and
  runtime behavior are unchanged.

## Slice D Validation

- Every migrated role, facade, runtime, macro, internal-support, and
  integration-test target checks successfully. Strict all-target Clippy passes
  for the same affected package set.
- All 36 procedural-macro tests, 26 focused endpoint/protocol/reference tests,
  and the cycle DTO test pass. The semantic ICRC-21 and cycle public imports
  compile and Candid-roundtrip without changing the generated wire contract.
- Focused Cargo Machete reports no unused dependency across all migrated
  packages. Obsolete direct-dependency exceptions are removed where imports
  are now visible.
- Fresh isolated rustdoc contains public `dto::icrc21` and `dto::cycles`
  modules and no public `canic::cdk` module. The doc-hidden compiler module is
  not listed on the public crate index.

## Slice E Validation

- All 981 `canic-core` library tests pass with one ignored. The focused
  contract covers root-only configuration, direct root allocation, stable
  index rebuild, replay and recovery binding, non-self rejection, notification
  overflow, and root upgrade refusal while any refill remains resumable.
- Focused facade protocol tests pass for the standard root bundle's controller
  guard and the narrowed named request and dry-run Candid records.
- All 80 focused refill tests, 18 CLI conversion tests, 21 host state-manifest
  tests, and two host role-capability tests pass. The generated root package
  checks successfully.
- Strict all-target Clippy for core, facade, CLI, and host passes. Targeted
  formatting, layering, changelog governance, and diff hygiene also pass.

## Next Action

Finish the targeted Slice E gate, publish `0.97.3`, and close 0.97 against the
accepted design. Do not change Cargo package versions outside the maintainer's
release flow or edit the downstream Toko repository.
