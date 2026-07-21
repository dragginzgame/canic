# 0.97 Role-Owned Runtime Dependencies And CDK Surface - Status

Last updated: 2026-07-21

## Current State

0.97 is proposed with its Cargo contract complete. The active product line
remains 0.96 at the released `v0.96.7` anchor with an open `0.96.8` timing
hard cut. No 0.97 product implementation,
package-version change, downstream edit, or public API removal is authorized
before maintainer acceptance and the 0.96 boundary permits the next line.

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
- The validator rejects a second normal path to Canic but currently renders a
  generic reason rather than the offending package chain.
- Cargo resolver 2 does not prevent feature union across two normal runtime
  paths to one package.
- Cargo workspace dependency features are additive, and Canic defaults remain
  enabled unless every contributing declaration disables them.
- The current validator permits a renamed Canic dependency even though
  procedural macro output uses the source-level crate name `canic`.
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

## Slice A Evidence Required Before Product Changes

1. Exact classification of every active `canic::cdk` export and consumer.
2. Exact inventory of workspace, normal, build, and feature declarations for
   every current role, including each `canic::build!` owner.
3. Protected and allowed-implementation package classifications across every
   supported Canic feature fixture.
4. Approved semantic owner or removal decision for every Canic-owned public
   CDK item.
5. Exact inventory of build, deploy, Medic, state, internal build, and release
   callers that must consume the one dependency checker.
6. Frozen normalized source/path semantics for structured dependency evidence.
7. Focused graph, macro, rustdoc, and isolated consumer validation matrix.

## Next Action

Complete only the read-only Slice A inventory after 0.96 permits the next
release line. Do not remove `canic::cdk`, widen the validator, allocate a 0.97
package version, or edit Toko before the design is accepted.
