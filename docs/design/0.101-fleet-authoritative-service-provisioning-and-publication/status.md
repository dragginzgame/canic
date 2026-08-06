# Canic 0.101 Implementation Status

Date: 2026-08-06

## Status

- State: implementation has the bounded layer-neutral identity vocabulary,
  strict configuration-only Component Group declarations, independent
  deployment flattening with bounded placement envelopes and worst-case Spec
  demand, exact deployment/include/leaf Fleet-service purpose resolution and
  strict canonical Fleet-service target validation and bounded inert labels
  inherited across deployment/include/leaf paths, plus bounded reduction-only
  limits resolved against exact flattened members and immutable Component
  Spec ceilings and one schema-v1 semantic digest over the canonical group,
  flattened-deployment and Fleet-service-target sections. The managed runtime
  ABI now validates and retains an immutable ordinary-or-group-member context
  and exposes it to application policy. One bounded canonical fresh
  provisioning plan now binds the exact Registry version, active roots,
  release sets, placement IDs, complete flattened members and effective
  limits; the strict Fleet input and Registry also carry the immutable
  per-root group-placement ceiling. A selected root now validates and durably
  accepts its exact Coordinator-authenticated batch, retains permanent
  placement reservations and an exact restart-safe acceptance receipt, and
  fences unrelated allocation and ordinary removal while that aggregate
  authority is live. The root now advances that accepted batch one canonical
  placement member at a time through the existing root-local
  `ComponentInstanceId` allocator, retaining a hash-bound O(1) cursor and
  exact response-loss reconciliation with the Component Registry. After every
  identity is reserved, the same aggregate command claims one
  oldest-sufficient Ready prepaid Canister per canonical member through the
  ordinary Component creation journal and derives the exact protected
  `GroupMember` runtime context from the accepted plan. A second hash-bound
  O(1) cursor reconciles response loss without another claim. After every
  Canister is claimed, the root installs one canonical member at a time through
  the ordinary Store-backed install journal, retaining and independently
  observing its exact grouped context behind `Prepared`; a third hash-bound
  cursor reconciles response loss. After every member is installed, the root
  commits one exact `Prepared` Component Registry partition at a time through
  the ordinary commitment authority; a fourth hash-bound cursor reconciles
  response loss only after the allocation receipt, grouped Registry-byte
  ceiling and current Fleet Directory agree. A focused PocketIC root proves
  the complete accept/reserve/claim/install/commit/replay path against one
  canonical Project Hub group placement, including the absence of a partition
  before commitment and the continued Directory/runtime fence afterward. The
  durable aggregate `Provisioned` result, Directory publication and runtime
  activation remain unimplemented.
- Release boundary: reinstall only.
- Implementation started: yes; `0.101.12` is released and `0.101.13` is open.
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
reuses the ordinary prepaid-Canister claim plus the root-owned 0.100 Cycles
Ledger refill. A Component request cannot create a physical Canister or use a
paid fallback when no `Ready` asset exists; root maintenance independently
restores the configured pool minimum.

The current complete Fleet-service member vectors and affected-root
confirmation barrier are retained only for the measured initial envelope. A
later reinstall-only large-Fleet design may hard-cut them to versioned
partitions and proof-carrying root-local projections, optionally distributed
through bounded Coordinator Workers, while keeping the Coordinator the sole
Fleet policy writer.

## Slice 1 — Composition and Purpose Contracts

- [x] Add bounded `ComponentGroupSpecId`, `ComponentGroupDeploymentId`,
  `ComponentGroupPlacementId`, `FleetServiceId`, member IDs and canonical
  member paths.
- [x] Compile nested Component Group declarations as a bounded acyclic graph.
- [x] Flatten every deployment completely before planning.
- [x] Preserve each distinct member-path occurrence and distinguish inclusion
  from independent deployment.
- [x] Resolve every Fleet-service leaf through exactly one typed purpose
  assignment on its deployment/include/leaf path.
- [x] Reject unused purpose assignments and orphan service occurrences or
  targets.
- [x] Add typed `Ordinary` and `FleetServiceMember` purpose with Authority,
  Replica and PoolMember variants.
- [x] Validate AuthorityReplica and ActivePool target/member invariants.
- [x] Validate service-wide member density/spread independently of deployment
  placement policy, including concrete fresh-plan and selected-root
  assignments.
- [x] Add bounded inert deployment labels that cannot alter authority.
- [x] Compile bounded reduction-only deployment-member limits against exact
  flattened paths and immutable Component Spec envelopes.
- [x] Add the reinstall-only protected Component deployment runtime contract,
  hard-cut managed init/status to retain it, validate exact compiled grouped
  projections and expose the retained purpose to application policy. Current
  ordinary provisioning emits only `UngroupedOrdinary`.
- [ ] Derive each `GroupMember` context from the accepted root plan, verify it
  against the Component Group Directory and enforce its exact effective limits
  throughout root allocation and descendant lifecycle. Accepted-plan context
  derivation, compiled-configuration validation and exact retained-context
  installation are complete; Component Group Directory confirmation and
  descendant-limit enforcement remain.
- [x] Derive one semantic protected configuration digest over groups,
  deployments and service targets.
- [ ] Remove singleton-Spec and sole-root-admission service assumptions.
- [x] Validate worst-case Spec demand, placement density/spread and the
  zero-placement/non-Authority versus singleton-Authority count rules.
- [ ] Measure and freeze the initial supported root, Component, placement,
  service-member, plan, Registry and Directory envelope.

## Slice 2 — Root Plans and Provisioning

- [x] Freeze one canonical root/group-placement/member plan shape.
- [ ] Carry every member's canonical effective limits through plan hashing,
  root acceptance, protected runtime context and durable receipts. Plan
  hashing, root acceptance, the acceptance receipt and grouped runtime-context
  installation are complete; Registry receipts and complete descendant-effect
  enforcement remain.
- [ ] Reserve monotonically increasing, never-reused placement ordinals before
  root calls.
- [x] Bind every placement to one exact eligible Fleet-owned root while
  permitting repeated roots within placement policy.
- [x] Require every flattened Spec in that root's immutable admissions,
  Component Topology, active release set and Wasm Store Catalog before durable
  acceptance.
- [ ] Enforce each root's immutable aggregate group-placement ceiling across
  accepted and committed state. Permanent exact-retry-safe accepted
  reservations now count once; later provisioning/publication transitions do
  not yet exist.
- [x] Extend the complete current root-limit contract without dropping
  Registry, Store, prepaid-pool or cycles-funding authority.
- [ ] Reuse canonical root-local `ComponentInstanceId` allocation,
  prepaid-Canister claim and platform lifecycle, failing closed when no Ready
  imported, recycled or automatically created asset exists. Root-local
  identity allocation, ordinary prepaid-Canister claim and exact Store-backed
  install plus Registry-commit recovery are complete; later lifecycle phases
  remain.
- [ ] Reuse 0.100's bounded root-owned Cycles Ledger refill and permanent
  uncertain-expiry fence; keep raw management `create_canister` and Component
  paid fallback absent.
- [ ] Add authenticated minimum-balance/fixed-top-up policy and bounded
  overfunding warnings without inventing an absolute maximum balance.
- [ ] Add non-IC ledger funding/configuration only with the same exact-Subnet,
  guaranteed-response and expired-uncertainty fences as the 0.100 mainnet path.
- [ ] Derive a cross-root top-level requester from the raw caller's exact
  current Fleet Registry service binding and matching Fleet Directory, then
  independently require the compiled peer-Component grant.
- [ ] Accept same-root child requests from any exact registered
  Component-tree node through an exact role-to-role spawn grant and without a
  Coordinator operation.
- [ ] Bind every descendant to its exact immediate parent while retaining the
  owning top-level Component binding.
- [x] Keep new Components runtime `Prepared`.
- [ ] Persist group-partitioned Component Registry evidence and one aggregate
  idempotent root receipt. The aggregate acceptance record, receipt and
  hash-bound reservation/claim/install/Registry cursors plus committed
  partitions are complete; the aggregate `Provisioned` binding/Registry result
  remains.

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
  Accepted placement authority, retained grouped origins and grouped Components
  are fenced; the aggregate grouped removal protocol remains unimplemented.
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
