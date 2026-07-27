# Canic 0.100 Implementation Status

Date: 2026-07-27

- State: implementation in progress.
- Release boundary: reinstall only.
- Implementation started: yes; intermediate Tree identities were released in
  immutable `v0.100.0`.
- Workspace package version: `0.100.8`.
- Open patch draft: `0.100.9`; no package-version change has been authorized.
- Open design blockers: none.

The 2026-07-26 design amendment removes the proposed Tree layer. The target is
exactly one Fleet Subnet Root per occupied `(FleetKey, SubnetId)`, with each
root managing multiple Component instances as dynamic multi-level trees. A
Component Spec declares one direct Component role and a flat catalog of every
potential descendant role/Wasm. Concrete parentage is root-owned Registry
state. A Component receives a root-allocated `ComponentInstanceId` only when
its Canister is created.

Different Fleets may each own an independent Fleet Subnet Root on the same
physical Subnet. Root uniqueness, authority, admissions, Stores and limits
remain Fleet-scoped.

The amendment separates immutable Component Topology, root-local Component
admissions, each root's active release set and the root-local Wasm Store
Catalog. The host builds one qualified Component/Component-Child artifact
union per `ReleaseBuildId` and projects an exact release-set manifest for each
root.

The latest design amendment separates flat artifact admission from dynamic
runtime parentage. A Project Hub creates a Project Instance as its direct child
on the same root/Subnet; that Project Instance may create its Ledger, Machine
and further children. Each child records its immediate parent, but the Fleet
Subnet Root remains sole controller, Registry owner and lifecycle executor.
Every root admitted for the Project Hub Spec stores the complete
Hub/Instance/Ledger/Machine potential-Wasm catalog once. The open 0.100.9
hard-cuts Component Spec/Topology canonical encoding to schema/domain version
3 with no v2 decoder.

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
- [x] Replace `[tree_specs.*]` and `[tree_groups.*]` with flat
  `[component_specs.*]`.
- [x] Replace `canic app role attach --tree-spec` with
  `--component-spec <component-spec>` and no alias.
- [x] Require exactly one Component role and a flat potential child-role/Wasm
  catalog per Component Spec.
- [x] Enforce positive Spec maxima and the 4,096-Component-instance Fleet
  bound.
- [x] Reject `root`, `service` and `component` child kinds,
  `owner_component` and nested Component declarations.
- [x] Add `ComponentSpecId` and the canonical `ComponentInstanceId` type.
- [x] Derive and freeze the canonical bounded Component Topology.
- [x] Hard-cut the canonical Component Spec/Topology encoding to version 2
  with initial child cardinalities and non-parent provisioning grants.
- [x] Hard-cut version 2 to canonical version 3, removing initial/direct-depth
  authority and compiling the flat potential-Wasm catalog plus exact
  role-to-role spawn grants.
- [x] Validate bounded spawn-grant parents, targets, completeness and
  per-parent ceilings while allowing recursive role capabilities.
- [x] Validate bounded peer-Component provisioning-grant targets, cycles and
  per-requester/root ceilings.
- [ ] Replace the temporary environment Component Spec selector with protected
  `ComponentBinding`.
- [x] Freeze `SubnetId`, Coordinator authority, `FleetSubnetRootBinding`,
  Component admissions, root limits and protected Component/child bindings.
- [x] Bind every Component Child to its immediate Component-tree parent so
  protected identity can represent arbitrary runtime depth.
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
- [x] Freeze the exact qualified application artifact-union model, compiler
  and canonical digest under one release-build and Fleet-wide topology
  identity.
- [x] Populate and persist that application artifact union once from the
  qualified complete-build outputs.
- [x] Freeze exact Spec-scoped Fleet Subnet Root Release-Set Manifest
  projection and canonical digests.
- [x] Materialize and persist one projected release-set manifest per planned
  root.
- [x] Freeze one immutable pre-effect Fleet install-plan boundary from exact
  resolved Coordinator/root placement, admissions, limits and funding input.
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
- [ ] Implement same-root grant-checked peer Component provisioning while
  retaining causal origin without parentage.
- [ ] Implement authenticated parent-to-root child effects at arbitrary depth.
- [ ] Make the Fleet Subnet Root the required lifecycle controller and retain
  authoritative idempotent receipts.
- [ ] Resolve lifecycle artifacts only through the active release set.
- [ ] Implement bounded Fleet snapshot synchronization once per root.
- [ ] Atomically activate the Fleet Registry Mirror and Fleet Directory.
- [ ] Store logical Component Registries in one bounded root-local collection
  with independent per-Component heads.
- [ ] Store normalized Component Registry rows with principal, parent/role,
  count and operation-journal indexes.
- [ ] Derive ownership-preserving Component Directories with compact heads and
  revision-bound pagination.
- [ ] Run subtree removal as durable post-order traversal and partition
  mutation serialization by Component instance.
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
`[component_specs.*]`, compiles all Component and potential child roles into
bootstrap input, builds their union through host projections and exposes
`--component-spec` attachment with no old flag. Root is implicit
infrastructure outside Specs. Top-level Component roles are unique as
Component declarations, but a role may also appear in another Spec's flat
potential-descendant catalog. The same declared descendant artifact may occur
in several catalogs without a role-only lookup choosing an owner.

This batch defines but does not yet allocate durable `ComponentInstanceId`
values. It now compiles each validated config into a bounded canonical
Component Topology, freezes domain-separated golden Spec hashes and root-local
topology digests, and exposes strong `SubnetId`, Coordinator/root authority,
root limits, admission and Component/child binding contracts. Component
aggregate descendant, Registry-byte and cycles-funding limits compile from finite
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

Released 0.100.3 adds a qualified infrastructure build-output contract and
immutably persists the canonical manifest under its durable
`ReleaseBuildId`. Loading revalidates canonical bytes and path identity. Exact
same-release retry is idempotent before or after release-build finalization;
conflicting evidence, unsafe artifact paths and first publication after
finalization fail closed. A normal complete install build now requires its
durable release-build identity before Cargo execution.

Released 0.100.4 freezes the separate application artifact-union
compiler. It requires the exact topology role set in both pre-build targets
and qualified build outputs, binds canonical evidence to one
`ReleaseBuildId` and the Fleet-wide Component Topology digest, and rejects
package, path, representation, build or topology drift. Exact per-root
release-set projection retains every Component Spec and child-role
authorization entry. Repeated or byte-identical artifacts count only once
against the root's Wasm Store byte limit without deduplicating those
authorization entries.

Released 0.100.5 derives the exact application targets from the
validated complete-build snapshot, qualifies the current raw/gzip outputs
against them and immutably persists the canonical union under its durable
`ReleaseBuildId` before finalization. Loading revalidates canonical bytes,
release-build path identity and the Fleet-wide topology. Exact retry remains
idempotent before or after finalization, while conflicts, unsafe artifact
paths and first publication after finalization fail closed.

Released 0.100.6 adds the immutable pre-effect `FleetInstallPlan`.
It accepts already-resolved Coordinator/root Subnets, positive creation
funding, admissions and limits; derives canonical pre-creation root topology
without fabricated Canister principals; and projects every root manifest from
the finalized application union. Root manifests publish first and the
Fleet/network/release-build-bound plan publishes last. Exact and interrupted
retry is idempotent, while conflicts, noncanonical documents, unsafe files,
identity drift and topology drift fail closed. Different Fleets retain
independent plan paths and may use the same physical Subnets.

Released 0.100.7 hard-cuts Component Spec and Component Topology encoding to
version 2, compiles exact initial-child cardinality and bounded non-parent
provisioning grants, and projects grants incoming to admitted target Specs
without importing requester admission or artifacts.

Released 0.100.8 freezes passive Fleet Registry snapshot, manifest and
version contracts plus a bounded domain-separated canonical encoding.
Epoch-one/revision-one genesis contains the complete immutable Component Spec
set and zero roots. Snapshot validation admits incremental partial `Joining`
rows while enforcing the configured App, exact protected Coordinator
authority, canonical physical-Subnet order, unique root principals, one active
release build, root topology/admissions/limits and aggregate Fleet admission
ceilings. Durable commits and root transitions remain pending.

The open 0.100.9 batch hard-cuts fixed-depth Component Topology v2 to v3.
Specs now compile one Component role plus a flat catalog of every potential
descendant Wasm. `initial_instances` is removed, aggregate
`maximum_children` becomes `maximum_descendants`, and every protected child
binding records its immediate parent. Child-role `maximum_instances` is
replaced by an explicit role-to-role spawn grant with a positive
`maximum_instances_per_parent`; `maximum_descendants` caps the whole tree and
defaults to 20,000. A child may request another admitted child in the same
Component tree/Subnet only through its exact grant, while the root retains
every controller, funding, Registry, artifact-selection and lifecycle effect.
Descendant roles may own scaling, sharding and Placement Index pools. The
default Registry allowance is 16 MiB, and the design requires normalized
indexed storage, compact revision-bound Directory pages, durable post-order
subtree removal and per-Component concurrency.

The current installer does not yet produce all three concrete infrastructure
outputs or invoke the infrastructure persistence boundary. It does not yet
resolve placement/funding input or invoke the new Fleet install-plan boundary
before its legacy single-root creation path. It also does not install planned
Coordinator/root bindings. The repository
does not yet contain a genuine Fleet Coordinator runtime/export authority, so
a correctly qualified Coordinator artifact cannot be produced by relabelling
the Fleet Subnet Root runtime. Protected environment binding, Fleet Subnet
Root lifecycle, active root release-set/Wasm Store authority and Fleet
Registry storage and runtime authority remain unimplemented. The temporary
Component Spec selector remains until real allocation supplies the exact
protected Component binding.

## Next Action

Add the trusted/operator placement and funding input boundary, resolve it to
exact Coordinator/root Subnets and invoke the immutable Fleet install planner
before any creation effect. The current legacy single-root creation path must
not bypass that authority.

Add the genuine Fleet Coordinator runtime and export authority before
producing the Fleet Coordinator, Fleet Subnet Root and Wasm Store as three
qualified infrastructure outputs. Do not satisfy the three-role manifest by
relabelling an existing root runtime or emitting a placeholder.

After artifact finalization, freeze resolved Coordinator/root placement and
funding in the installation journal, install the Coordinator before the roots,
and commit the genesis Fleet Registry.

Replace the temporary Component Spec environment selector only when root-local
allocation and Component Registry commitment can supply a real
`ComponentBinding`; do not fabricate one from role-only configuration.
