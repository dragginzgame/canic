# Canic 0.100 Implementation Status

Date: 2026-07-26

- State: implementation in progress.
- Release boundary: reinstall only.
- Implementation started: yes; intermediate Tree identities were released in
  immutable `v0.100.0`.
- Workspace package version: `0.100.2`.
- Open patch draft: `0.100.3`; no package-version change has been authorized.
- Open design blockers: none.

The 2026-07-26 design amendment removes the proposed Tree layer. The target is
exactly one Fleet Subnet Root per occupied `(FleetKey, SubnetId)`, with each
root managing a flat collection of Component instances. A Component Spec
declares one direct Component role and its direct children. A concrete
Component receives a root-allocated `ComponentInstanceId` only when its
Canister is created.

Different Fleets may each own an independent Fleet Subnet Root on the same
physical Subnet. Root uniqueness, authority, admissions, Stores and limits
remain Fleet-scoped.

The amendment separates immutable Component Topology, root-local Component
admissions, each root's active release set and the root-local Wasm Store
Catalog. The host builds one qualified Component/Component-Child artifact
union per `ReleaseBuildId` and projects an exact release-set manifest for each
root.

Canic infrastructure now has its own exact three-entry artifact manifest for
the Coordinator, Fleet Subnet Root and Wasm Store. The host directly installs
the Coordinator and roots; each root alone bootstraps its local Store from the
verified infrastructure artifact.
The Coordinator is not part of a Component hierarchy and is not installed
through a Wasm Store.

Released 0.100.1 removed the staged Tree declaration layer from
source, generated bootstrap, host/CLI projections, scaffolding, fixtures and
active guidance. It retains no Tree identity or compatibility alias. Runtime
authority is still transitional until the later binding, admission and
Registry slices replace the 0.99 root model.

## Slice 1 — Freeze Current Authorities and Component Topology

- [x] Record the complete live Registry, Directory, cascade, lifecycle,
  bootstrap and role-package
  [authority inventory](0.100-authority-inventory.md).
- [x] Record the intermediate Tree declaration implementation and its bounded
  validation.
- [x] Remove `TreeSpecId`, `TreeGroupId`, `TreeId` and their public exports.
- [x] Replace `[tree_specs.*]` and `[tree_groups.*]` with non-recursive
  `[component_specs.*]`.
- [x] Replace `canic app role attach --tree-spec` with
  `--component-spec <component-spec>` and no alias.
- [x] Require exactly one Component role and a direct child set per Component
  Spec.
- [x] Enforce positive Spec maxima and the 4,096-Component-instance Fleet
  bound.
- [x] Reject `root`, `service` and `component` kinds, `owner_component`,
  nested Components and child-owned children.
- [x] Add `ComponentSpecId` and the canonical `ComponentInstanceId` type.
- [x] Derive and freeze the canonical bounded Component Topology.
- [ ] Replace the temporary environment Component Spec selector with protected
  `ComponentBinding`.
- [x] Freeze `SubnetId`, Coordinator authority, `FleetSubnetRootBinding`,
  Component admissions, root limits and protected Component/child bindings.
- [ ] Hard-cut Fleet Root to Fleet Subnet Root.
- [ ] Hard-cut local `SubnetRegistry` and `SubnetDirectory` to root-owned
  per-Component `ComponentRegistry` and `ComponentDirectory`.
- [ ] Split Fleet and Component Directory provenance.
- [ ] Prove no prior-release transition reader or decoder exists.

## Slice 2 — Topology-Admitted Artifacts and Fresh Root Installation

- [x] Revise config/bootstrap/host projections to Component Specs.
- [x] Canonicalize every Component Spec and freeze its topology hash.
- [x] Distribute positive per-root Component Spec admissions whose sum does
  not exceed the Fleet ceiling.
- [x] Freeze the exact three-role Canic Infrastructure Artifact Manifest
  model, compiler and canonical digest.
- [ ] Populate and persist that manifest from the qualified complete-build
  outputs.
- [ ] Build the exact qualified artifact union once under one release-build
  identity.
- [ ] Construct and freeze one exact Fleet Subnet Root Release-Set Manifest
  per root.
- [ ] Resolve and journal independent Coordinator and Fleet Subnet Root
  placements plus configured limits.
- [ ] Freeze exact creation funding before each external effect.
- [ ] Install the Coordinator from empty state.
- [ ] Install each root and bootstrap its local topology-admitted Wasm Store.
- [ ] Commit the genesis Fleet Registry.

## Slice 3 — Fleet Registry and Root Lifecycle

- [ ] Implement canonical snapshot commits with Fleet Component Spec and root
  rows.
- [ ] Enforce one Fleet Subnet Root per occupied `(FleetKey, SubnetId)`.
- [ ] Prove another Fleet may independently use the same physical Subnet.
- [ ] Implement root `Joining`, `Active`, `Draining` and `Removed`.
- [ ] Install initial roots behind the runtime `Prepared` fence.
- [ ] Enforce Spec, admission, root, topology, limits, active-release-set and
  tombstone rules.

## Slice 4 — Component Lifecycle, Mirrors and Directories

- [ ] Implement durable root-local `ComponentInstanceId` allocation.
- [ ] Implement admitted direct Component creation through the root.
- [ ] Implement authenticated Component-to-root Component Child effects.
- [ ] Make the Fleet Subnet Root the required lifecycle controller and retain
  authoritative idempotent receipts.
- [ ] Resolve lifecycle artifacts only through the active release set.
- [ ] Implement bounded Fleet snapshot synchronization once per root.
- [ ] Atomically activate the Fleet Registry Mirror and Fleet Directory.
- [ ] Store logical Component Registries in one bounded root-local collection
  with independent per-Component heads.
- [ ] Derive ownership-preserving Component Directories.
- [ ] Distribute Directories directly from the root to Components and
  Component Children.

## Slice 5 — Recovery and Closeout

- [ ] Qualify interruption and exact retry.
- [ ] Prove one Component operation cannot block an unrelated Component.
- [ ] Activate initial roots only after active-release-set Store and final
  topology synchronization.
- [ ] Publish the Fleet catalog only after complete terminal evidence.
- [ ] Qualify restore fencing and role-package boundaries.
- [ ] Measure Wasm boundaries and remove stale authority.

## Current Batch

Released 0.100.1 parses only flat
`[component_specs.*]`, compiles all Component roles and direct children into
bootstrap input, builds their union through host projections and exposes
`--component-spec` attachment with no old flag. Root is implicit
infrastructure outside Specs. Component roles are globally unique; the same
declared child artifact may occur on several structural edges without a
role-only lookup choosing an owner.

This batch defines but does not yet allocate durable `ComponentInstanceId`
values. It now compiles each validated config into a bounded canonical
Component Topology, freezes domain-separated golden Spec hashes and root-local
topology digests, and exposes strong `SubnetId`, Coordinator/root authority,
root limits, admission and Component/child binding contracts. Component
aggregate child, Registry-byte and cycles-funding limits compile from finite
config defaults or exact overrides.

The host topology planner accepts resolved root principals, physical Subnets,
limits and positive per-Spec capacities, then derives hashes/digests itself.
It canonicalizes root/admission order, enforces App/Fleet binding, complete
Fleet admission coverage and one root per Fleet/Subnet pair, while independent
Fleets may reuse one physical Subnet. Root bootstrap and managed release-role
projection now consume compiled topology instead of raw Component Spec
configuration.

Released 0.100.2 defines the infrastructure artifact family independently from
application release sets. Its compiler admits exactly one Fleet Coordinator,
Fleet Subnet Root and Wasm Store entry from one `ReleaseBuildId`, derives
raw/gzip lengths and hashes from matching bytes, canonicalizes entry order and
freezes deterministic manifest bytes and digest.

The open 0.100.3 batch adds a qualified infrastructure build-output contract
and immutably persists the canonical manifest under its durable
`ReleaseBuildId`. Loading revalidates canonical bytes and path identity. Exact
same-release retry is idempotent before or after release-build finalization;
conflicting evidence, unsafe artifact paths and first publication after
finalization fail closed. A normal complete install build now requires its
durable release-build identity before Cargo execution.

The current installer does not yet produce all three concrete infrastructure
outputs or invoke the persistence boundary, and it does not journal or install
planned Coordinator or root bindings. Protected environment binding, Fleet
Subnet Root lifecycle, exact root release-set/Wasm Store authority and Fleet
Registry authority remain unimplemented. The temporary Component Spec
selector remains until real allocation supplies the exact protected Component
binding.

## Next Action

Produce the Fleet Coordinator, Fleet Subnet Root and Wasm Store as three
qualified outputs from one complete build and invoke the durable
infrastructure-manifest boundary before release-build finalization. Reject any
build that cannot supply all three exact roles. Then build the qualified
application artifact union once and project exact root-specific release-set
manifests from the validated root admissions.

After artifact finalization, freeze resolved Coordinator/root placement and
funding in the installation journal, install the Coordinator before the roots,
and commit the genesis Fleet Registry.

Replace the temporary Component Spec environment selector only when root-local
allocation and Component Registry commitment can supply a real
`ComponentBinding`; do not fabricate one from role-only configuration.
