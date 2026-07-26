# Canic 0.101 Implementation Status

Date: 2026-07-26

## Status

- State: proposed.
- Release boundary: reinstall only.
- Implementation started: no.
- Dependency: completed 0.100 Fleet Subnet Root, Component Spec,
  root-local Component identity, topology-admitted Wasm Store and Registry
  architecture.
- Open design blockers: none. Application-data replication remains a separate
  later design and is not an implementation blocker for 0.101 topology,
  purpose or discovery contracts.

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
pre-admitted roots, including a root that completed the separate ordinary
root-registration lifecycle before scale-out. Toko's one-cell-per-root choice
and maximum of ten are example policy values, not protocol limits.

0.101 does not consume a 0.100 installation, preserve existing Canisters,
synchronize application data, choose load-balancer health, scale in, promote a
Replica or create roots during scale-out. Grouped Components and their roots
remain fenced from the ordinary 0.100 removal paths while placement or service
references exist.

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
- [ ] Persist one plan-derived protected Component deployment context so
  application policy can enforce Authority/Replica purpose.
- [ ] Derive one semantic protected configuration digest over groups,
  deployments and service targets.
- [ ] Remove singleton-Spec and sole-root-admission service assumptions.
- [ ] Validate worst-case Spec demand, placement density/spread and the
  zero-initial/non-Authority versus singleton-Authority count rules.

## Slice 2 — Root Plans and Provisioning

- [ ] Freeze one canonical root/group-placement/member plan shape.
- [ ] Reserve monotonically increasing, never-reused placement ordinals before
  root calls.
- [ ] Bind every placement to one exact eligible Fleet-owned root while
  permitting repeated roots within placement policy.
- [ ] Require every flattened Spec in that root's immutable admissions,
  Component Topology, active release set and Wasm Store Catalog.
- [ ] Enforce each root's immutable aggregate group-placement ceiling.
- [ ] Reuse canonical root-local `ComponentInstanceId` allocation and
  platform lifecycle.
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
- [ ] Freeze the exact Directory-confirmation roots: all initial roots for
  fresh install, and selected plus every affected existing service-member root
  for scale-out.
- [ ] Require Replica purpose to fail application database write-authority
  checks.
- [ ] Require PoolMember purpose to grant no implicit leadership, health or
  consistency.
- [ ] Preserve that write fence for service-sensitive direct children through
  their exact owning Component without making children group members or Fleet
  services.

## Slice 4 — Explicit Group Scale-Out

- [ ] Persist each deployment's placement IDs, exact root assignments, current
  count, protected maximum and next ordinal.
- [ ] Accept only monotonic desired-count increases with exact unused
  placement IDs on active roots within density, spread and aggregate limits.
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
  active-release-set, Wasm Store, placement, service-mode, purpose, label and
  authority boundaries.
- [ ] Prove several placements of one deployment may share a root without
  identity or Component Group Directory ambiguity.
- [ ] Prove two Fleets remain isolated when their roots share one physical
  Subnet.
- [ ] Prove configured Replica discovery never claims data readiness,
  promotion or failover and configured PoolMember discovery never claims
  health, load-balancer eligibility or consistency.
- [ ] Complete stale-path and design closeout checks.

## Completion

- [ ] The Toko journey provisions database A, B and C Authorities on one root.
- [ ] The same database group is reused inside a nested project-cell group to
  provision a project hub plus same-Spec database A, B and C Replicas on at
  least two other roots.
- [ ] One project-cell scale-out resumes exactly across forced interruption.
- [ ] A same-Spec ActivePool packs multiple stable placements on one root,
  spans at least two roots and publishes one atomic scale-out addition.
- [ ] All design criteria and required journeys pass.
- [ ] No Tree identity, runtime Group Canister, nested Component, child target,
  singleton-Spec restriction, adoption, prior-release transition or
  compatibility path survives.
- [ ] Current status and changelog record the final evidence.
