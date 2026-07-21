# 0.97 Role-Owned Runtime Dependencies And CDK Surface - Status

Last updated: 2026-07-22

## Current State

0.97 is accepted and active after completing audit-only Slice A against
released `v0.96.8` and implementing bounded Slice B in the open `0.97.0`
batch. The required inventories and relocation decisions are frozen in the
[Slice A report](../../audits/reports/2026-07/2026-07-22/0.97-slice-a-surface-and-graph-inventory.md).
Slice B gives every authoritative role-contract caller one canonical Cargo
evidence producer and hard-cuts the declaration and protected-graph
contradictions identified by Slice A. No package-version change, downstream
edit, public CDK removal, or refill hard cut is part of this graph slice.

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
- The current public `canic::cdk` facade mixes upstream conveniences,
  Canic-owned helpers and types, and macro-expansion plumbing.
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
- Active `canic::cdk` consumers require only direct upstream IC imports or
  existing Canic-internal helpers. The only justified semantic relocations are
  Canic cycle values under `dto::cycles` and public ICRC-21 protocol DTOs under
  `dto::icrc21`.
- ICP refill has no current dedicated timer and no in-repository role enables
  its generic feature. Its remaining generic feature/config/API/emitter,
  arbitrary target/CLI fabrication, unconditional lifecycle/metrics access,
  and child-funding integration are all inventoried for Slice E. The three
  removable stable error codes have no maintained producer.

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

## Next Action

Finish the open `0.97.0` Slice B validation and publish boundary. After the
maintainer pushes it, begin Slice C at the frozen hidden macro-plumbing and
public `canic::cdk` boundary. Do not start the refill hard cut, change Cargo
package versions, or edit Toko in the graph batch.
