# Canic 0.100 Implementation Status

## Status

- State: implementation in progress.
- Release boundary: reinstall only.
- Implementation started: yes, from immutable `v0.99.34`.
- Workspace package version: `0.99.34`; no 0.100 package-version change has
  been authorized.
- Open design blockers: none.

The working tree may contain staged 0.99 and 0.100 terminology while a coherent
hard-cut batch is being built. A releasable 0.100 surface may not contain an
alias, decoder or fallback for the removed topology model.

## Slice 1 — Freeze Current Authorities

- [x] Freeze distinct bounded `TreeSpecId` and `TreeGroupId` declaration
  identities.
- [x] Freeze generated 32-byte `TreeId` and its canonical boundary encoding.
- [ ] Record the complete live Registry, Directory, cascade, lifecycle,
  bootstrap and role-package authority inventory.
- [ ] Hard-cut `SubnetSlotId` and `subnet_slot` to Tree declaration, identity
  and physical placement.
- [ ] Hard-cut Fleet Root to Tree Root.
- [ ] Hard-cut local `SubnetRegistry` and `SubnetDirectory` to `TreeRegistry`
  and `TreeDirectory`.
- [ ] Split Fleet and Tree Directory provenance.
- [ ] Freeze `SubnetId`, Coordinator authority and protected `TreeBinding`.
- [ ] Prove no 0.99 transition reader or decoder exists.

## Slice 2 — Fresh Coordinator Installation

- [ ] Compile Tree Specs and Tree Groups.
- [ ] Resolve and journal independent Coordinator and initial Tree placement.
- [ ] Freeze exact creation funding before each external effect.
- [ ] Install the Coordinator from empty state.
- [ ] Commit the genesis Fleet Registry.

## Slice 3 — Fleet Registry and Tree Lifecycle

- [ ] Implement canonical snapshot commits.
- [ ] Implement `Joining`, `Active`, `Draining` and `Removed`.
- [ ] Install initial Tree Roots behind the runtime `Prepared` fence.
- [ ] Enforce group capacity, identity and tombstone rules.

## Slice 4 — Fleet Registry Mirror and Directories

- [ ] Implement bounded snapshot synchronization.
- [ ] Atomically activate the Fleet Registry Mirror and Fleet Directory.
- [ ] Derive and cascade the separate Tree Directory.
- [ ] Expose bounded consumer topology.

## Slice 5 — Recovery and Closeout

- [ ] Qualify interruption and exact retry.
- [ ] Activate initial runtimes only after final topology synchronization.
- [ ] Publish the Fleet catalog only after complete terminal evidence.
- [ ] Qualify restore fencing and role-package boundaries.
- [ ] Measure Wasm boundaries and remove stale authority.
- [ ] Complete the 0.100 design criteria and current-surface terminology scan.

## Current Batch

The first batch adds only invariant-preserving Tree identity contracts. It does
not yet introduce `TreeBinding`, `SubnetId`, Tree-aware configuration,
Coordinator state or a partial runtime rename.

## Next Action

Record the exact 0.99 topology authority map, then hard-cut App configuration
from `[subnets.*]` and `[services.fleet]` to `[tree_specs.*]` and
`[tree_groups.*]` as one coherent parser, validation, bootstrap, host-mutation,
fixture and active-document batch.
