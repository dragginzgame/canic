# Canic 0.101 Implementation Status

## Status

- State: proposed.
- Release boundary: reinstall only.
- Implementation started: no.
- Dependency: 0.100 architecture and qualification.
- Open blockers: none in this design.

0.101 creates a fresh Fleet and provisions its declared services. It does not
consume a 0.100 installation or preserve existing service Canisters.

## Slice 1 — Contracts and Planning

- [ ] Freeze protected target, activation-plan, binding and typed-error shapes.
- [ ] Compile targets during fresh installation.
- [ ] Reject pre-existing local service rows.

## Slice 2 — Tree-Local Provisioning

- [ ] Prepare one exact operation per selected Tree Root.
- [ ] Reuse the canonical local creation lifecycle.
- [ ] Persist and reconcile external effects and receipts.

## Slice 3 — Coordinator Orchestration

- [ ] Resolve and persist the complete plan before mutation.
- [ ] Drive every selected root and verify the complete ready set.
- [ ] Make exact retry deterministic and idempotent.

## Slice 4 — Atomic Publication

- [ ] Commit the complete service set in one Registry revision.
- [ ] Synchronize mirrors and Fleet Directories.
- [ ] Confirm Tree-local publication.

## Slice 5 — Recovery and Qualification

- [ ] Exercise deterministic interruption boundaries.
- [ ] Prove backup/restore cross-document consistency.
- [ ] Prove role-package and authority boundaries.
- [ ] Complete stale-path and design closeout checks.

## Completion

- [ ] All design criteria and required journeys pass.
- [ ] No adoption, prior-release transition or compatibility path survives.
- [ ] Current status and changelog record the final evidence.
