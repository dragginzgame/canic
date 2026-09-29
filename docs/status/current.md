# Current handoff — 2026-09-29

## Baseline and accepted cleanup batch

The source baseline is tagged `v0.110.48`; package versions remain `0.110.48`.
The maintainer accepted the original nine findings and eight additional cleanup
findings, including explicit documentation of the unscheduled backup gap, then
accepted the remaining non-blob findings from the next audit. Blob-storage
follow-up is excluded for its planned extraction to `../ic-blob-storage`; this
pass makes no changes to that repository or the retained blob surfaces. The
[0.110.49 draft](../changelog/0.110.md#011049---unreleased) groups this work;
no version transaction, commit, publication or deployment has been performed.

Implementation removes the prose-based blob inventory gates, unread metrics,
test-only runtime flows and physical topology planner, obsolete manifest variants,
and timer anti-resurrection assertions. Capacity admission now uses its tested
Root selection policy. Evidence manifests and reports consistently name the local
workspace. Cleanup guides follow reinstall-only release policy while preserving
same-release recovery and cycle conservation.

The blob gate consumes a structured JSON protocol evidence record, preserving
exact upstream method/source identifiers and retained historical inventories.
Operator evidence manifests remain TOML, now using `[workspace]`; reports expose
`workspace_name` and workspace schema IDs. Rust and canonical Store Candid admit
only the maintained chunked/approved manifest variants. No compatibility aliases
were added. The separate CANIC-188 repair below has an explicit, operation-specific
state-preservation exception; ordinary release transitions remain reinstall-only.

Keep this accepted cleanup in 0.110 to finish its hard-cut and release-authority
corrections before the human-owned minor closeout. This reassesses the soft
12-release guideline without authorizing another minor. The final 0.110 closeout
must include FI1 and subsequent corrections and receive explicit maintainer
acceptance before any 0.111 implementation.

## Qualification

Targeted qualification passes:

- 77 Core metrics, lifecycle, management and blob/Cashier boundary tests.
- 109 Host topology, release projection, capacity-import and evidence tests.
- 76 CLI metrics, cycles and evidence tests.
- 60 default-feature Control Plane template/bootstrap/state-contract tests;
  37 isolated Root tests and 40 isolated Store tests also pass.
- Three structured protocol-evidence gate regressions, 53 protocol/Candid checks
  and 15 current timer ownership checks.
- Warning-denied Clippy for all targets/features of `canic`, `canic-core`,
  `canic-control-plane`, `canic-host` and `canic-cli`; Control Plane was rerun
  after the final isolation and test-flow cleanup.
- Release-integrity, structured evidence, shell lint, layering, document semantics,
  changed-document local links, scoped Rust formatting and whitespace checks.

The exact governed PocketIC case
`pic::fleet_registry::baseline::tests::completed_reset::completed_estate_reset_recovers_and_replays`
passes in 479.93 seconds. Its 703-second governed run includes compilation and
fresh Wasm size/Candid/artifact qualification. It exercises unpaid review
cancellation, replacement build selection, issued-work rejection, lost-response
recovery, cycle conservation and effect-free replay through the current CLI.
The retained log is
`target/test-runs/20260929T143011Z-49183.YxmmgV/1.log`;
selection logs are `/tmp/canic-cleanup-*.log`. No broad workspace gate ran.

Isolated Store qualification also removed three disconnected test-only chunk
flows and two orphaned errors. Corruption, hash rejection and recovery assertions
now use the maintained Store operations. Role-specific shared fixtures and state
contract expectations are gated with their actual owners.

The additional cleanup removes the disconnected physical-binding validator,
standalone Fleet-service compiler, unused backup discovery and authority-request
surfaces, hash alias and release-build error. Release guards use structured
cadence and executable outcomes instead of diagnostic prose.

Additional focused validation passes: 49 Core tests, 68 Backup tests including
manifest-publication process-death recovery, 11 Host tests and 19 release-flow
tests. Warning-denied all-target/all-feature Clippy passes for Core, Backup,
Host and the facade. Release-integrity, shell lint, document semantics, changed
guide/handoff local links, scoped formatting and whitespace checks pass. Logs
are `/tmp/canic-followup-*.log`. No broad gate or additional PocketIC run was
needed for these disconnected API removals and host guard changes.

The final non-blob cleanup removes unused replay reasons, the unused durable-file
creation mode, default-only restore preview wrappers and the duplicate subnet
catalog failure projection. Production retains detailed upstream typed failures;
regressions assert them directly. Durability race coverage uses the maintained
parent-creating path, and restore tests use configured previews. Core layering
and schema-1 runtime status documentation now match the maintained contracts.

Final focused tests pass: 24 Host durability and subnet-catalog tests, 52 Core
replay tests, 69 Backup restore journal/runner tests and 15 CLI restore tests.
Warning-denied all-target/all-feature Clippy passes for Core, Host, Backup and
CLI. Current-document semantics, scoped Rust formatting and whitespace checks
pass. Logs are `/tmp/canic-cleanup3-*.log`; no broad workspace gate ran.

The accepted cleanup batch, with blob follow-up excluded as directed, is qualified
and ready for maintainer review. The `.49` changelog draft covers all three cleanup
passes and the CANIC-187/188 corrections below. The complete accepted local batch
is qualified and ready for maintainer review; live recovery remains outstanding.
Package versions remain `.48`; changes are uncommitted, with `.49` notes ready
for the maintainer-owned version/publication flow.

## Release gate follow-up

The reported audit-catalog, release-integrity and validation-runner failures are
corrected. The CANIC-188 example uses testkit's bounded explicit startup; changed
DRY and module-hardening methods have new versions and current fingerprints,
with previous identities retained. The [method correction report](../audits/reports/2026-09/2026-09-29/audit-method-correction.md)
limits affected historical audit conclusions; comparative minor closeout still
requires the corrected paired audits.

All three exact Make gates pass, as do warning-denied Clippy for the changed
example and its local PocketIC repair/restoration qualification. Logs are
`/tmp/canic-gate-repair.log` and `/tmp/canic188-startup-*.log`. The retained repair
Wasm is unchanged. These gate fixes and updated `.49` notes are ready for review;
no broad validation, version transaction, commit or publication was run by the
agent in this follow-up.

The later governed-suite failure was a stale supplied-infrastructure import
fixture: its two-source request allowed 32 calls, below the maintained minimum
of 35. It now derives the recommended call count and debit budget from the shared
policy and destination Root quote. The exact
`pic::fleet_registry::baseline::tests::infrastructure_bootstrap::supplied_infrastructure_initializes_and_recovers`
case passes in 77.55 seconds, including recovery and effect-free replay.
Warning-denied library and test-target Clippy also pass. Logs are
`/tmp/canic-bootstrap-import-*.log`; the exact test output is
`target/test-runs/20260929T200342Z-61806.ofa9Iw/1.log`. The `.49` notes include
this fixture correction; no broad suite or release command was rerun.

## Unscheduled backup execution gap

Fresh CLI backup execution fails closed because Coordinator-backed Component
Registry topology preflight is unimplemented. Dry-run planning does not prove
live authority or create a backup. This known gap has no accepted slice in .110
or the scheduled .111 extraction; OC-5 is only an unscheduled proposal. The
[backup guide](../features/backup-and-restore/README.md#current-availability)
owns availability, scope and the required follow-up. Keep existing backup and
same-release recovery machinery; this cleanup does not implement the preflight.

## Independent blob-service feedback

Reviewed the clean local ic-blob-storage 0.3.0 source snapshot and recorded
[upstream feedback](../upstream/ic-blob-storage.md). A bounded existing-binary
probe reproduces a FIFO cursor blocking before the CLI's regular-file check.
Trusted completion, operational restore recovery and managed/operator parity
remain acknowledged adoption blockers. The sibling repository was read-only;
no provider request, sibling build or external message ran. This review does
not change the accepted cleanup batch or authorize Canic blob removal.

## Downstream and history

The September 29 CANIC-187/188 feedback is implemented locally. New imports
use a Root call-cost quote and a complete-operation budget checked before handoff.
Successful callbacks release their exact unused allowance; lost or uncertain
callbacks retain reserves and consumed call counts. Typed diagnostics expose the
protected phase and budget. Unchanged seed projections retain original bytes;
real identity changes keep the existing publication owner.

Targeted Core, Control Plane and Host tests and warning-denied all-target/all-feature
Clippy for Core, Control Plane, Host and CLI pass. The actual Root import and
public CLI completed-estate recovery/replay PocketIC journeys pass after these
changes. Logs are `/tmp/canic-import-*.log`; no broad workspace gate ran.

The maintainer accepted a narrow CANIC-188 repair exception for local implementation
and qualification. The [repair decision](../design/0.110-fleet-runtime-contraction/issued-import-recovery.md)
and [incident instructions](../../scripts/dev/canic188/README.md) bind the exact
issued import. A frozen `.48` repair releases only the exact retained completed
allowances, preserves operation/progress and original ceilings, and restores the
original Root before settlement. Native exact-record tests and actual PocketIC
repair/restoration pass, including lost-install-reply reconciliation, replay
rejection and the original Root debit ceiling. The fixture seeds public evidence;
it does not replay all 24 historical source effects.

The reproduced, tested candidate and original artifact are retained under
`.canic/incident-repairs/canic188/ed3084b6908a04b28effa21e00ec425aaf382d1423849fcbb4f3b12414d61f48/`,
with a hash manifest, structured qualification and logs. Fixture Wasm is excluded.
No live calls or sibling mutations occurred. Live execution still requires fresh
exact status, controller/module and cycle-margin checks; the original `.48` CLI
must finish the original approved import after Root restoration. Full-Fleet
convergence, terminal replay and frontend acceptance remain outstanding. Do not
retry the unchanged exhausted apply, overwrite retained authority, or fund the
superseded `.16` review. The [feedback triage](../audits/reports/2026-09/2026-09-29/toko-pool-import-feedback.md)
retains the original diagnosis and the qualification boundary.
The [readiness qualification](../audits/reports/2026-09/2026-09-29/toko-unpaid-review-readiness.md)
and [cancellation qualification](../audits/reports/2026-09/2026-09-29/toko-unpaid-review-cancellation.md)
record the accepted current review/recovery behavior.

- [Maintained scope and batch dispositions](../design/0.110-fleet-runtime-contraction/status.md).
- [Cleanup through 0.110.48 and prior validation](archive/2026-09-29-cleanup-through-0.110.48.md).
- [Publication recovery and earlier .47 handoff](archive/2026-09-29-publication.md).
- [Dated .110 implementation history](archive/2026-09-29-fleet-runtime-contraction.md).
