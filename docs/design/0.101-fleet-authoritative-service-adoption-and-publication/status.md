# Canic 0.101 Implementation Status

Last updated: 2026-07-23

## Status

- Design state: proposed.
- Implementation state: not started.
- Dependency: 0.99 and 0.100 must be implemented and qualified before Slice 1.
- Scope: in-place authority-service adoption, missing-service provisioning,
  one-way ownership handoff and complete Fleet Directory publication.
- Excluded: relocation, replacement, replication, promotion, failover and
  application-data migration.

No item below is complete until its implementation and focused validation
evidence exist.

## Slice 1 — Contracts, Declaration and Current Registry Schema

- [ ] Add `[services.fleet]` singleton/eligibility validation.
- [ ] Compile one canonical host declaration and install its protected
  prepared/committed bytes and hash in the Coordinator.
- [ ] Freeze and implement the declaration/plan encodings, domains and bounds.
- [ ] Reuse the 0.100 `FleetRegistry` and
  `FleetAuthorityServiceBinding` without another schema or Fleet binding.
- [ ] Perform only required one-time local stable-state migrations.
- [ ] Remove any Directory-owned authority declaration.

## Slice 2 — Service-Local Authority State and Deployment Fence

- [ ] Add `Local`, `Preparing`, `Provisioning` and `Fleet` protected states.
- [ ] Migrate existing Fleet-bound services to `Local`.
- [ ] Add exact structural prepare/commit endpoints.
- [ ] Fence Canic control-plane mutation while preparing or provisioning.
- [ ] Qualify the root-first/Coordinator-last current-schema upgrade and
  `AwaitingCoordinatorUpgrade` state.

## Slice 3 — Complete Planning and Inventory Qualification

- [ ] Collect current inventory from every `Joining`, `Active` and `Draining`
  root.
- [ ] Load and verify qualified final inventory/fence evidence for every
  `Removed` member.
- [ ] Fail closed on unqualified removal or a selected unresolved
  removed-member candidate.
- [ ] Build one canonical bounded `Adopt`/`Provision` activation plan.
- [ ] Validate qualified placement, package, controller, registry and binding
  evidence before mutation.
- [ ] Persist the plan and membership-mutation fence.

## Slice 4 — Missing-Service Provisioning

- [ ] Reuse the existing allocation/pool and installer path.
- [ ] Require a durably known Canister principal.
- [ ] Journal an explicitly Authority-Subnet-targeted creation request and
  successful result.
- [ ] Verify protected installation and qualified placement evidence.
- [ ] Commit unpublished authority-registry ownership.

## Slice 5 — Existing-Service Ownership Handoff

- [ ] Add source `MovingOut` and `Released` records.
- [ ] Add destination `PreparedIn` and `Committed` records.
- [ ] Implement exact controller-set transfer and unknown-outcome
  reconciliation.
- [ ] Retain source tombstones and all committed per-role evidence.
- [ ] Make every phase idempotent or explicitly outcome-unknown.

## Slice 6 — Canonical Publication and Directory Cascade

- [ ] Verify that the complete committed service set equals the declaration.
- [ ] Publish a non-empty complete set in one checked Registry revision.
- [ ] Commit an empty declaration idempotently without a revision.
- [ ] Populate existing Directory `entries` from `authority_services`.
- [ ] Preserve the exact `subnets` projection and `DirectoryProvenance`.
- [ ] Set `synchronized_at` only on atomic mirror-and-Directory activation.
- [ ] Resume Fleet membership mutation after publication.

## Slice 7 — Recovery, Backup and Operational Qualification

- [ ] Resume from every durable boundary.
- [ ] Prove old source backups cannot resurrect ownership.
- [ ] Prove old Coordinator backups cannot lower canonical state.
- [ ] Prove removed-member evidence cannot be omitted or treated as empty.
- [ ] Prove declaration drift cannot start a second activation.
- [ ] Add bounded inspection, metrics and typed reports.
- [ ] Complete focused PocketIC or disposable-environment qualification.

## Completion Criteria

- [ ] Service authority is declared only through `[services.fleet]`.
- [ ] Every selected role is eligible and singleton.
- [ ] The complete service set is resolved before mutation.
- [ ] Every removed member is qualified and has no selected unresolved
  candidate.
- [ ] Existing mutable services retain their principals and application data.
- [ ] Missing services have complete qualified Authority-Subnet placement
  evidence.
- [ ] The Coordinator owns one separate authority-service registry.
- [ ] Every handoff is durable, one-way and restart-safe.
- [ ] No execution trace admits two committed owners.
- [ ] Unknown management outcomes reconcile or fail closed.
- [ ] A complete non-empty set is published in one checked revision; an empty
  set is idempotently already current.
- [ ] The one current 0.100 Registry schema and public binding type are reused.
- [ ] Fleet Directory `entries`, `subnets`, provenance and `synchronized_at`
  match the design exactly.
- [ ] Membership mutation resumes after publication.
- [ ] Old backups cannot roll authority backward.
- [ ] No relocation, replacement, replication, promotion, failover or
  application-data migration code is added.
- [ ] Existing-Fleet and fresh-Fleet journeys pass.
- [ ] Closeout finds no second declaration, ownership, publication or
  Directory authority.
