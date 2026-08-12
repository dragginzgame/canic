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

Fresh installation now performs its live authority gate before artifact
compilation and retains the dedicated `target/canic-wasm` Cargo target across
attempts. Interactive installs replace per-finding and per-receipt text streams
with ANSI-aware ASCII activity lines and padded summary tables, while redirected
output remains deterministic and append-only. The complete post-build artifact
and execution gates still re-observe deployment truth before mutation.
Configured artifacts now share one Cargo invocation per Cargo workspace and
profile, and their package validation shares workspace metadata/catalog
evidence while retaining each role's package-selected graph. For the common
local one-workspace topology this reduces configured-role Cargo build processes
from two per role to two total: release and debug/Candid.

Rendered CLI help has also been compacted around routine operator decisions.
Install, App, build, deploy and evidence pages retain their real arguments,
options, representative examples and safety boundary without duplicating
configuration snippets, artifact internals, cache maintenance or policy prose.
The stale reference to an existing-Fleet update flow is removed; `canic
install` now states only its maintained fresh-Fleet boundary.

`canic build <app>` now selects every attached role and reuses the same
workspace/profile Cargo batching as installation, then builds the canonical
Fleet Coordinator and Wasm Store under a separate Infrastructure Wasm section.
Supplying the optional role keeps the focused custom-build and role-provenance
surface. Complete builds render bounded TTY-safe activity lines and padded
Application/Infrastructure tables instead of requiring a shell loop or
printing repeated Cargo logs. Section and total completion lines report elapsed
wall time. App rows now report real per-role artifact-finalization time, while
shared Cargo compilation remains reported once at section level. Each phase-
ready summary is followed by one blank line before the next section.

Fleet Coordinator now follows the same canonical-Candid model as Wasm Store.
Each published infrastructure source crate owns a checked-in DID; ordinary
local builds copy and embed it without a second debug compile, while one
explicit maintainer refresh path regenerates either contract. Generated
downstream wrappers extract when no canonical source package is available and
do not promote their generated sidecar to source truth.

## Current Decision

Continue Q5 whole-program hard-cut and closeout in the existing open draft.
Do not create another release boundary for individual cleanup, documentation,
CI fallout, or focused proof. Historical tags and changelogs stay immutable.

The remaining closeout must account for:

- obsolete authority paths, aliases, fallback decoders, and compatibility
  residue;
- Candid and generated surfaces, configuration guidance, and stable-memory
  ownership;
- module responsibility and retained production size;
- cold and retained-cache Toko install measurement after removing routine
  infrastructure debug-Candid builds; and
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

The installer-efficiency slice passes focused `canic-host` install,
authority-preflight, deployment-truth, cache, table and terminal-output tests;
focused `canic-cli` install-help tests; and package Clippy with warnings denied.
Configured-build grouping, exact multi-package Cargo command and shared
workspace-evidence tests also pass without weakening per-role graph selection.
The rendered-help and whole-App build cleanup passes the complete `canic-cli`
library suite: 563 tests passed and one disposable-environment restore test
remained intentionally ignored.
Focused whole-App build selection, role-provenance rejection and host batch-
command tests pass. A real fast-profile build of the checked-in `demo` App
materializes four attached role artifacts plus Coordinator and Wasm Store
infrastructure successfully. The retained-cache smoke reported 35.84s for the
application batch, 82.00s for Coordinator, 29.31s for Wasm Store and 147.15s
total before canonical Coordinator Candid. Repeating that smoke after the
change reported 36.43s for Application Wasm, 3.36s for Coordinator, 5.15s for
Wasm Store, 8.51s for Infrastructure Wasm and 44.94s total. The canonical-
infrastructure-Candid slice refreshes and parses the Coordinator contract
through the maintained debug export, and focused host tests prove ordinary
Coordinator and Wasm Store builds use checked-in contracts without a debug
compile. A repeated fast smoke reports 4.66s root finalization and roughly
2.3-2.4s for each smaller App role inside one 37.87s shared Application Wasm
section.

## Next Action

Continue Q5 from the active design checklist. Prefer deleting confirmed
sediment and duplicate authority over documenting it, keep changes inside the
existing `0.101.52` draft, and update this handoff only with the current
decision, remaining work, or evidence needed by the next session. For installer
efficiency, next measure cold and retained-cache Toko installs after configured-
build batching and canonical infrastructure Candid.
