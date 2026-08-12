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

- Workspace package version: `0.101.52`.
- Latest published release: `v0.101.52` at
  `997e0109fae575f84d30b009a1240ef795c719d3`.
- Open changelog draft: `0.101.53` in
  [`docs/changelog/0.101.md`](../changelog/0.101.md).
- Active design and checklist:
  [0.101 Fleet-authoritative service provisioning and publication](../design/0.101-fleet-authoritative-service-provisioning-and-publication/status.md).
- Release boundary: reinstall only. Same-release interruption recovery, exact
  retry, backup, and restore remain required.

## Current Progress

Q4 real-topology qualification is complete. The focused three-application-
Subnet PocketIC journey provisions 15 top-level Components across three roots,
publishes 15 Fleet-service members, exercises packed and spread ActivePool
scale-out with Coordinator restart/replay, creates seven dynamic descendants,
and keeps a second Fleet independent while it shares one physical Subnet. The
measured supported envelope is recorded in
[`qualification.md`](../design/0.101-fleet-authoritative-service-provisioning-and-publication/qualification.md).
Its dedicated root configuration and build target leave the smaller shared
delegation fixture's canonical topology unchanged.

Published 0.101.52 also begins Q5 cleanup by compacting this handoff, reconciling
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

`canic build <app>` now batches the configured Fleet Subnet Root with every
attached Component role, then builds the canonical Fleet Coordinator and Wasm
Store. Supplying the optional role keeps the focused custom-build and role-
provenance surface. Complete builds render bounded TTY-safe activity lines and
padded tables instead of requiring a shell loop or printing repeated Cargo
logs. Components appear under Application Wasm; Coordinator, root and Store
appear under Infrastructure Wasm. The root retains shared Cargo compilation
without receiving a fabricated per-role duration, and both tables show exact
resolved package versions.

Fleet Coordinator now follows the same canonical-Candid model as Wasm Store.
Each published infrastructure source crate owns a checked-in DID; ordinary
local builds copy and embed it without a second debug compile, while one
explicit maintainer refresh path regenerates either contract. Generated
downstream wrappers extract when no canonical source package is available and
do not promote their generated sidecar to source truth.

## Current Decision

Q5 whole-program hard cut and closeout is complete in the existing open
0.101.53 draft. The final
[responsibility report](../design/0.101-fleet-authoritative-service-provisioning-and-publication/closeout.md)
accounts for obsolete authority paths, generated/configuration/Candid
surfaces, stable-memory ownership, retained production size and every permitted
historical-only occurrence. Historical tags and changelogs remain immutable.

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
materializes one configured root, three Component artifacts, Coordinator and
Wasm Store successfully. The retained-cache smoke reported 35.84s for the
application batch, 82.00s for Coordinator, 29.31s for Wasm Store and 147.15s
total before canonical Coordinator Candid. Repeating that smoke after the
change reported 36.43s for Application Wasm, 3.36s for Coordinator, 5.15s for
Wasm Store, 8.51s for Infrastructure Wasm and 44.94s total. The canonical-
infrastructure-Candid slice refreshes and parses the Coordinator contract
through the maintained debug export, and focused host tests prove ordinary
Coordinator and Wasm Store builds use checked-in contracts without a debug
compile. A repeated fast smoke retains one honest shared Application Wasm
total and shows exact resolved package versions for every artifact.

The open 0.101.53 draft moves routine formatting into one repository-owned
pre-commit hook. The hook neither stages files nor duplicates validation;
release preparation remains non-mutating and retains its formatting check. It
also corrects complete-build presentation: the Fleet Subnet Root remains in the
efficient shared configured batch but is reported with Coordinator and Wasm
Store infrastructure, and every artifact shows its resolved package version.
Q5's production cleanup has also separated the complete Coordinator root-
deletion authority from the accumulated Registry/provisioning module while
retaining its exact durable records, hash domains and workflow surface.
Canonical provisioning-plan encoding is now separate from semantic authority
validation, and its frozen plan and root-batch hash vectors remain unchanged.
The feature gate now compiles the Fleet Subnet Root graph independently in
addition to Coordinator, Store, empty and host-consumer graphs.

The exact nine-role Toko fixture built all eight Components plus root,
Coordinator and Store in 186.98s on first fixture-specific materialization and
60.54s on exact retained-cache retry. The root stayed in the shared configured
Cargo batch but rendered under infrastructure with `shared` timing. These are
artifact-build measurements; the Q4 PocketIC journey remains the actual
multi-Subnet installation and runtime evidence.

## Next Action

The open 0.101.53 batch is ready for maintainer-owned complete release
validation. If that gate exposes fallout, keep the correction in this same
batch; otherwise publish 0.101.53 as the final 0.101 closeout release. Do not
start 0.102 or reopen deferred replication, scale-in, cross-Fleet estate or
blob-service work inside this release.
