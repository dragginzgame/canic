# Canic 0.101 Implementation Status

Date: 2026-08-04

## Status

- State: proposed.
- Release boundary: reinstall only.
- Implementation started: no.
- Dependency: completed 0.100 qualified independently host-installed
  Coordinator/root/Store infrastructure, Fleet Subnet Root, Component Spec,
  root-local Component identity, topology-admitted sibling Wasm Store,
  prepaid-Canister inventory and Registry architecture, including flat
  potential-Wasm catalogs and multi-level dynamic Component trees plus
  separate runtime and Registry membership activation, revision-bound
  current-Directory convergence and inventory-bound Fleet Subnet Root runtime
  activation.
- Open design gate: implementation must measure and freeze the exact initial
  root, Component, placement, service-member, plan, Registry and Directory
  envelope. The first implementation does not claim ten-thousand-Subnet
  qualification. Application-data replication remains a separate later design
  and is not an implementation blocker for 0.101 topology, purpose or
  discovery contracts.

0.101 creates a fresh Fleet with composable compile-time Component Groups.
Nested group declarations flatten to direct Components under exact Fleet
Subnet Roots. Inclusion emits one occurrence per member path and does not
implicitly execute another deployment or deduplicate equal Specs. The same
Component Spec may provide one Fleet service Authority and several
cross-Subnet Replicas or several members of an ActivePool. Bounded
`FleetServiceId` is separate from role, and typed mode/member purpose is
protected independently of inert labels. One reusable service-bearing group
may receive an Authority assignment at its singleton deployment and a Replica
assignment where another group includes it.

Fresh installation provisions configured initial group placements and
gives each concrete copy a stable never-reused
`ComponentGroupPlacementId`, and atomically publishes every service's complete
mode-compatible member set. Placement policy can pack several copies of one
deployment on one root or spread them across roots, subject to per-deployment
density/spread and immutable aggregate root limits. After installation, an
authenticated administrator may monotonically add exact placements on
pre-installed, pre-admitted roots from the same fresh Fleet installation.
0.101 does not add a root after the initial all-root activation. Toko's
one-cell-per-root choice and maximum of ten are example policy values, not
protocol limits.

Separate deployment IDs may reference the same Component Group while applying
different reduction-only limits to exact flattened member paths. The
Component Spec remains the absolute envelope. A deployment may narrow
`maximum_descendants`, `maximum_registry_bytes` and exact role-to-role
spawn-grant ceilings, but may not raise them, add grants or replace component
configuration. Every placement and later scale-out of one deployment inherits
the same protected effective limits. This permits one Project Hub deployment
with a 10,000-instance Hub-to-Instance grant and another on a different root
and physical Subnet with a 2,000-instance effective grant without duplicating
its group, Spec, role or Wasm.

The design now also carries the corrected high-cardinality Toko path. Every
project-data-cell placement contains one Project Hub PoolMember beside the
database Replicas. The Hub asks its own root to create Project Instance direct
children, and a Project Instance asks the same root to create its Ledger and
optional Machine children. Every child binding records the exact immediate
parent while the root remains sole lifecycle authority. The Coordinator is not
on this path. The Hub's `project_id -> ComponentChildBinding` map is
an application-owned Placement Index that agrees with, but does not replace,
protected Canic parentage or the root-derived Component Directory. The Hub and
Project Instance use distinct explicit spawn grants for their respective
child roles.

0.101 also closes 0.100's deferred cross-root peer-Component requester proof.
The target root derives one exact top-level Fleet-service member from the
IC-authenticated raw caller and its current Fleet Registry Mirror, requires
the matching Fleet Directory projection, then independently enforces the
compiled requester-Spec-to-target-Spec Component Provisioning Grant.
Membership proves identity, not permission. Children, ungrouped Components,
forwarded callers and caller-supplied bindings cannot use this path.

0.101 does not consume a 0.100 installation, preserve existing Canisters,
synchronize application data, choose load-balancer health, scale in, promote a
Replica or create roots during scale-out. Grouped Components and their roots
remain fenced from the ordinary 0.100 removal paths while placement or service
references exist.

Fresh 0.101 installation inherits the 0.100 infrastructure manifest,
independent host installation of the Coordinator, every root and every sibling
Store, reciprocal root/Store verification and sole-root Store adoption.
Component Group placement does not change that installation ownership and
reuses the ordinary prepaid-Canister claim. It cannot create a physical
Canister or use a paid fallback when the root's operator-supplied inventory is
empty.

The current complete Fleet-service member vectors and affected-root
confirmation barrier are retained only for the measured initial envelope. A
later reinstall-only large-Fleet design may hard-cut them to versioned
partitions and proof-carrying root-local projections, optionally distributed
through bounded Coordinator Workers, while keeping the Coordinator the sole
Fleet policy writer.

## Slice 1 — Composition and Purpose Contracts

- [ ] Add bounded `ComponentGroupSpecId`, `ComponentGroupDeploymentId`,
  `ComponentGroupPlacementId`, `FleetServiceId`, member IDs and canonical
  member paths.
- [ ] Compile nested Component Group declarations as a bounded acyclic graph.
- [ ] Flatten every deployment completely before planning.
- [ ] Preserve each distinct member-path occurrence and distinguish inclusion
  from independent deployment.
- [ ] Resolve every Fleet-service leaf through exactly one typed purpose
  assignment on its deployment/include/leaf path.
- [ ] Reject unused purpose assignments and orphan service occurrences or
  targets.
- [ ] Add typed `Ordinary` and `FleetServiceMember` purpose with Authority,
  Replica and PoolMember variants.
- [ ] Validate AuthorityReplica and ActivePool target/member invariants.
- [ ] Validate service-wide member density/spread independently of deployment
  placement policy.
- [ ] Add bounded inert deployment labels that cannot alter authority.
- [ ] Compile bounded reduction-only deployment-member limits against exact
  flattened paths and immutable Component Spec envelopes.
- [ ] Persist one plan-derived protected Component deployment context so
  application policy can enforce Authority/Replica purpose and the root can
  enforce exact effective limits.
- [ ] Derive one semantic protected configuration digest over groups,
  deployments and service targets.
- [ ] Remove singleton-Spec and sole-root-admission service assumptions.
- [ ] Validate worst-case Spec demand, placement density/spread and the
  zero-placement/non-Authority versus singleton-Authority count rules.
- [ ] Measure and freeze the initial supported root, Component, placement,
  service-member, plan, Registry and Directory envelope.

## Slice 2 — Root Plans and Provisioning

- [ ] Freeze one canonical root/group-placement/member plan shape.
- [ ] Carry every member's canonical effective limits through plan hashing,
  root acceptance, protected runtime context and durable receipts.
- [ ] Reserve monotonically increasing, never-reused placement ordinals before
  root calls.
- [ ] Bind every placement to one exact eligible Fleet-owned root while
  permitting repeated roots within placement policy.
- [ ] Require every flattened Spec in that root's immutable admissions,
  Component Topology, active release set and Wasm Store Catalog.
- [ ] Enforce each root's immutable aggregate group-placement ceiling.
- [ ] Extend the complete current root-limit contract without dropping
  Registry, Store, prepaid-pool or cycles-funding authority.
- [ ] Reuse canonical root-local `ComponentInstanceId` allocation,
  prepaid-Canister claim and platform lifecycle, failing closed when no Ready
  imported or recycled asset exists.
- [ ] Add a first-class operator pool-replenishment command whose host-side
  creation intent, selected Subnet, ingress/result evidence and exact
  principal survive response loss before authenticated root import; the root
  must never issue raw `create_canister` or a paid fallback.
- [ ] Add authenticated minimum-balance/fixed-top-up policy and bounded
  overfunding warnings without inventing an absolute maximum balance.
- [ ] Keep Cycles Ledger creation disabled unless recovery is proved across
  `TooOld`, its finite deduplication horizon and duplicate responses without a
  Canister ID; otherwise retain external operator creation plus exact import.
- [ ] Derive a cross-root top-level requester from the raw caller's exact
  current Fleet Registry service binding and matching Fleet Directory, then
  independently require the compiled peer-Component grant.
- [ ] Accept same-root child requests from any exact registered
  Component-tree node through an exact role-to-role spawn grant and without a
  Coordinator operation.
- [ ] Bind every descendant to its exact immediate parent while retaining the
  owning top-level Component binding.
- [ ] Keep new Components runtime `Prepared`.
- [ ] Persist group-partitioned Component Registry evidence and one aggregate
  idempotent root receipt.

## Slice 3 — Service Topology and Directories

- [ ] Resolve exactly one Authority plus zero or more same-Spec Replicas for
  AuthorityReplica, and one or more same-Spec Pool members for ActivePool.
- [ ] Publish each service's complete initial mode-compatible member set in one
  Fleet Registry revision.
- [ ] Project exact service ID, mode and purpose-bearing member bindings
  through Fleet Directory.
- [ ] Derive one root-local Component Group Directory per placement without
  introducing group parentage or lifecycle authority.
- [ ] Send exact Fleet, Component and Group Directories before activation.
- [ ] Activate each Component runtime under its exact prepared Directory,
  then promote its Component Registry partition to `Active` and synchronize
  the resulting revision-bound current Directory before root activation.
- [ ] Freeze the exact Directory-confirmation roots: all initial roots for
  fresh install, and selected plus every affected existing service-member root
  for scale-out.
- [ ] Require Replica purpose to fail application database write-authority
  checks.
- [ ] Require PoolMember purpose to grant no implicit leadership, health or
  consistency.
- [ ] Preserve that write fence for service-sensitive descendants through
  their exact owning top-level Component without making descendants group
  members or Fleet services.

## Slice 4 — Explicit Group Scale-Out

- [ ] Persist each deployment's placement IDs, exact root assignments, current
  count, protected maximum and next ordinal.
- [ ] Accept only monotonic desired-count increases with exact unused
  placement IDs on active roots within density, spread and aggregate limits.
- [ ] Require every eligible scale-out root to belong to the complete root set
  installed and activated by the same fresh Fleet installation.
- [ ] Enforce each affected service's complete member density/spread policy
  after every addition.
- [ ] Provision only new placements and retain exact retry identity.
- [ ] Append all Replica and PoolMember bindings from one scale operation
  atomically.
- [ ] Fence grouped Components and their roots from ordinary drain/removal.
- [ ] Reject scale-down, placement reuse, Authority-group scaling, live root
  creation and admission expansion.

## Slice 5 — Recovery and Qualification

- [ ] Exercise deterministic interruption boundaries for fresh provisioning
  and scale-out.
- [ ] Prove backup/restore cross-document consistency.
- [ ] Prove Component Topology, group flattening, admission,
  active-release-set, Wasm Store, effective member limits, placement,
  service-mode, purpose, label and authority boundaries.
- [ ] Prove several placements of one deployment may share a root without
  identity or Component Group Directory ambiguity.
- [ ] Prove two Fleets remain isolated when their roots share one physical
  Subnet.
- [ ] Prove cross-root peer provisioning requires the exact current raw-caller
  Registry member, matching Directory projection and independent
  requester-Spec-to-target-Spec grant, while stale, forwarded, child and
  caller-supplied identities reject.
- [ ] Prove independent host Store installation/adoption and ordinary
  prepaid-Canister claim/retry remain the sole infrastructure and Component
  effect paths used by grouped provisioning.
- [ ] Prove the first excess value for every frozen initial scale bound rejects
  before mutation or network effects.
- [ ] Prove configured Replica discovery never claims data readiness,
  promotion or failover and configured PoolMember discovery never claims
  health, load-balancer eligibility or consistency.
- [ ] Prove local Project Hub -> Project Instance -> Ledger/Machine
  provisioning, exact immediate-parent bindings and Coordinator-free retry.
- [ ] Prove two deployments reuse one Project Hub group with distinct
  reduction-only 10,000/2,000 Hub-to-Instance ceilings on different roots.
- [ ] Complete stale-path and design closeout checks.

## Completion

- [ ] The Toko journey provisions database A, B and C Authorities on one root.
- [ ] The same database group is reused inside a nested project-data-cell
  group to provision same-Spec database A, B and C Replicas plus one Project
  Hub PoolMember on at least two other roots.
- [ ] One project-data-cell scale-out resumes exactly across forced
  interruption.
- [ ] The local Project Hubs provision at least three Project Instance
  children across their project roots; every Project Instance creates one
  Ledger and exactly one creates its optional Machine.
- [ ] Two deployments reuse one Project Hub group on different roots with
  distinct protected effective spawn-grant ceilings and no duplicated Spec.
- [ ] A same-Spec ActivePool packs multiple stable placements on one root,
  spans at least two roots and publishes one atomic scale-out addition.
- [ ] The initial supported Fleet/service envelope is measured and does not
  claim ten-thousand-Subnet qualification.
- [ ] All design criteria and required journeys pass.
- [ ] No Tree identity, runtime Group Canister, nested Component declaration,
  Component Child group/service target, delegated lifecycle authority,
  singleton-Spec restriction, adoption, prior-release transition or
  compatibility path survives.
- [ ] Current status and changelog record the final evidence.
