# Canic 0.101 Implementation Status

Last updated: 2026-07-25

## Status

- Design state: proposed.
- Implementation state: not started.
- Dependency: 0.99 and 0.100 must be implemented and qualified before Slice 1.
- Scope: Tree-owned Fleet service adoption/provisioning and complete
  Coordinator publication.
- 0.100 input: preserve the exact Tree Groups, group-bound Trees,
  Coordinator-outside-Trees model and empty `authority_services` field.
- Excluded: ownership transfer to the Coordinator, relocation, replacement,
  replication, promotion, failover and application-data migration.

No item below is complete until its implementation and focused validation
evidence exist.

## Slice 1 — Contracts and Target Compilation

- [ ] Hard-cut the declaration to `[services.fleet.targets]`.
- [ ] Compile every role to one singleton `TreeGroupId`, active `TreeId`, Tree
  Root, Tree Spec hash and physical `SubnetId`.
- [ ] Require the selected role in exactly the target Tree Spec.
- [ ] Add `TreeId` to the existing Registry and Directory service binding.
- [ ] Freeze canonical declaration/plan encodings, domains and bounds.
- [ ] Preserve the 0.100 `tree_groups` and `trees` fields unchanged.

## Slice 2 — Tree Root Preparation

- [ ] Add local `Preparing` and `FleetPublished` publication states without
  changing local Registry ownership.
- [ ] Add exact authenticated prepare/status endpoints.
- [ ] Fence duplicate local creation, removal, role reassignment and
  controller-policy mutation while preparing.
- [ ] Qualify an existing candidate without moving it, releasing it or
  changing its controllers.
- [ ] Qualify the root-first/Coordinator-last current-schema upgrade.

## Slice 3 — Complete Planning and Inventory Qualification

- [ ] Collect inventory from every `Joining`, `Active` and `Draining` Tree
  Root.
- [ ] Load every qualified removed-Tree final inventory and fail closed on
  unqualified removal.
- [ ] Bind evidence to the exact Tree, group, Tree Spec hash, root and
  placement.
- [ ] Resolve the complete sorted target set to exact `Adopt` or `Provision`
  work before mutation.
- [ ] Reject wrong-Tree, duplicate or possibly live removed-Tree candidates.
- [ ] Persist one bounded activation plan and Tree-lifecycle mutation fence.

## Slice 4 — Tree-Local Provisioning

- [ ] Reuse the target Tree Root's existing allocation and installer path.
- [ ] Journal one exact principal-returning create request.
- [ ] Never repeat an unknown create without recovering the exact principal.
- [ ] Verify protected runtime binding, package/module and controller policy.
- [ ] Commit one normal local Registry row and canonical ready receipt.

## Slice 5 — Atomic Registry and Directory Publication

- [ ] Verify that the complete ready set equals the declaration.
- [ ] Publish one non-empty complete set in one checked Registry revision.
- [ ] Treat an empty declaration and empty Registry as idempotently current.
- [ ] Populate Directory entries as `(role, TreeId, canister_id)`.
- [ ] Preserve the exact group, Tree and provenance projections.
- [ ] Confirm target-root `FleetPublished` state through normal Registry
  synchronization.
- [ ] Resume Fleet Tree lifecycle mutation after publication.

## Slice 6 — Recovery, Backup and Operational Qualification

- [ ] Resume from every durable activation boundary.
- [ ] Reconcile lost preparation, ready, create and publication responses.
- [ ] Prove old Coordinator state cannot lower Registry authority.
- [ ] Prove old Tree Root state cannot clear publication fences.
- [ ] Add bounded inspection, metrics and typed reports.
- [ ] Record raw/compressed release-Wasm evidence for host, Coordinator,
  Tree-Root and service artifacts.
- [ ] Complete focused PocketIC or disposable-environment qualification.

## Completion Criteria

- [ ] The Coordinator has no Tree identity and is not a service parent,
  local Registry owner or controller.
- [ ] `[services.fleet.targets]` is the sole Fleet service declaration.
- [ ] Every target role resolves to one active Tree in one singleton group.
- [ ] The complete service set is resolved before mutation.
- [ ] Existing services retain principal, data, local Registry row and Tree
  Root authority.
- [ ] Missing services are created only by the selected Tree Root.
- [ ] Live and removed Tree evidence prevents duplicate candidates.
- [ ] A complete non-empty set is published in one checked revision.
- [ ] Registry and Directory entries retain `(role, TreeId, canister_id)`.
- [ ] The one current 0.100 Registry schema and Tree topology are reused.
- [ ] Tree lifecycle mutation resumes after publication.
- [ ] Co-locating a Primary Data Tree with the Coordinator changes no logical
  authority and is reported as a shared capacity/outage trade-off.
- [ ] A Primary Data Tree on another Subnet has the same publication
  semantics.
- [ ] Unknown management outcomes reconcile or fail closed.
- [ ] Old backups cannot roll authority or publication backward.
- [ ] Writable-primary replication, promotion and failover remain absent.
- [ ] Per-role Wasm evidence attributes growth without adding 0.101 workflows
  to unrelated ordinary Canisters.
- [ ] Existing-Fleet and fresh-Fleet journeys pass.
- [ ] Closeout finds no Coordinator-owned service Registry, ownership-transfer
  machinery or superseded distinguished-Subnet terminology.
