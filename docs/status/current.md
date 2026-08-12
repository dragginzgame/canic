# Current Status

Last updated: 2026-08-12

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the active design, source, validation, or changelog material needed for the
current task.

Historical handoffs are archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md);
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md); and
- [status through the 0.101.52 Q4 qualification](archive/2026-08-12-precompact.md).

## Current Release

- Workspace package version: `0.101.51`.
- Latest published release: `v0.101.51` at
  `c20ed1a57148e860e46742c991de872de9edefc8`.
- Open changelog draft: `0.101.52` in
  [`docs/changelog/0.101.md`](../changelog/0.101.md).
- Active design and checklist:
  [0.101 Fleet-authoritative service provisioning and publication](../design/0.101-fleet-authoritative-service-provisioning-and-publication/status.md).
- Release boundary: reinstall only. Same-release interruption recovery, exact
  retry, backup, and restore remain required.

## Completed In The Open Draft

Q4 real-topology qualification is complete. The focused three-application-
Subnet PocketIC journey provisions 15 top-level Components across three roots,
publishes 15 Fleet-service members, exercises packed and spread ActivePool
scale-out with Coordinator restart/replay, creates seven dynamic descendants,
and keeps a second Fleet independent while it shares one physical Subnet. The
measured supported envelope is recorded in
[`qualification.md`](../design/0.101-fleet-authoritative-service-provisioning-and-publication/qualification.md).
Its dedicated root configuration and build target leave the smaller shared
delegation fixture's canonical topology unchanged.

The open draft also begins Q5 cleanup by compacting this handoff, reconciling
current installation and recovery guidance, clarifying layer and release
authority, and making documentation/CI guards protect maintained semantics
instead of incidental wording or workflow shape.

## Current Decision

Continue Q5 whole-program hard-cut and closeout in the existing open draft.
Do not create another release boundary for individual cleanup, documentation,
CI fallout, or focused proof. Historical tags and changelogs stay immutable.

The remaining closeout must account for:

- obsolete authority paths, aliases, fallback decoders, and compatibility
  residue;
- Candid and generated surfaces, configuration guidance, and stable-memory
  ownership;
- module responsibility and retained production size; and
- the final targeted evidence required by the active design checklist.

Application-data replication, grouped removal, scale-in, replacement, and
relocation remain later designs rather than 0.101 blockers.

## Validation

Q4 qualification command:

```text
cargo test --locked -p canic-testing-internal pic::fleet_registry::baseline::tests::toko_topology_qualifies_scale_out_descendants_packing_and_fleet_isolation --lib -- --exact --nocapture
```

For the current semantic-cleanup slice, run only the directly affected
documentation, recovery, release-validation, layering, workflow, and changelog
checks plus `git diff --check`. Full deployment and publish validation remains
maintainer-owned.

The targeted cleanup checks pass: Actionlint; ShellCheck for the changed guard
scripts; documentation-example `rustfmt --check`; current-document, release-
validation, package/install, recovery, layering, and release-integrity guards;
changelog governance and reference-surface tests; and `git diff --check`.

## Next Action

Continue Q5 from the active design checklist. Prefer deleting confirmed
sediment and duplicate authority over documenting it, keep changes inside the
existing `0.101.52` draft, and update this handoff only with the current
decision, remaining work, or evidence needed by the next session.
