# Canic 0.100 Implementation Status

## Status

- State: implementation in progress.
- Release boundary: reinstall only.
- Implementation started: yes; foundational identities are released in
  immutable `v0.100.0`.
- Workspace package version: `0.100.0`.
- Open patch draft: `0.100.1`; no package-version change has been authorized.
- Open design blockers: none.

The working tree may contain staged 0.99 and 0.100 terminology while a coherent
hard-cut batch is being built. A releasable 0.100 surface may not contain an
alias, decoder or fallback for the removed topology model.

## Slice 1 — Freeze Current Authorities

- [x] Freeze distinct bounded `TreeSpecId` and `TreeGroupId` declaration
  identities.
- [x] Freeze generated 32-byte `TreeId` and its canonical boundary encoding.
- [x] Record the complete live Registry, Directory, cascade, lifecycle,
  bootstrap and role-package
  [authority inventory](0.100-authority-inventory.md).
- [x] Hard-cut App configuration, compiled bootstrap input, host projections,
  CLI mutation and active fixtures from `SubnetSlotId`, `[subnets.*]` and
  `[services.fleet]` to Tree Specs and Tree Groups.
- [ ] Replace the temporary environment `TreeSpecId` selector with protected
  `TreeBinding`, concrete `TreeId`, Tree Root and physical `SubnetId`.
- [ ] Hard-cut Fleet Root to Tree Root.
- [ ] Hard-cut local `SubnetRegistry` and `SubnetDirectory` to `TreeRegistry`
  and `TreeDirectory`.
- [ ] Split Fleet and Tree Directory provenance.
- [ ] Freeze `SubnetId`, Coordinator authority and protected `TreeBinding`.
- [ ] Prove no 0.99 transition reader or decoder exists.

## Slice 2 — Fresh Coordinator Installation

- [x] Parse, validate and embed bounded Tree Specs and Tree Groups.
- [ ] Canonicalize each Tree Spec and freeze its binding hash.
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

The open 0.100.1 batch hard-cuts the App configuration language to
`[tree_specs.*]` and `[tree_groups.*]`. Every Spec has one root, group counts
are positive and ordered, and total declared capacity is bounded at 4,096
Trees. Compiled bootstrap config, active fixtures, host projections,
scaffolding and `canic app role attach --tree-spec` use the same surface.

The existing installer still supports only one initial Tree and now fails
closed for a larger declaration. This is an explicit staging boundary:
Coordinator installation, concrete Tree identity, placement and Registry
authority are not implemented by this batch.

## Next Action

Freeze `SubnetId`, `FleetCoordinatorBinding` and protected `TreeBinding`, then
hard-cut Fleet Root and the remaining environment contract to distinct
Coordinator and Tree Root authority. Do not let the temporary Tree Spec
selector become runtime identity.
