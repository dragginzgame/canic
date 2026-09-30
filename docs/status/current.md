# Current handoff — 2026-09-30

## Cargo resolver 3 continuation

The maintainer requested Cargo resolver 3. The workspace, generated Fleet
packages, Host role admission, CLI medic fixtures, maintained isolated manifests
and generated downstream smoke-test workspaces now select it. The old test that
rejected resolver 3 is removed. Historical audit snapshots and the frozen CANIC-188
repair inputs remain evidence for their original source state.

Locked offline metadata succeeds and its package/dependency/feature graph exactly
matches the captured pre-change graph. Resolver selection adds no lockfile changes;
the prior `yoke-derive 0.8.4` repair remains retained. Focused qualification passes
45 Host role/package tests, 41 CLI medic tests and two generated Fleet wrapper
admission tests: **88 tests**. Host/CLI all-target/all-feature warning-denied Clippy
passes, as do shell syntax, targeted ShellCheck, formatting and document semantics
(two existing advisory layout warnings). Logs are under
`target/review-validation/`: `resolver3-targeted.log`, `resolver3-generated.log`
and `resolver3-clippy.log`. Invocation-owned scratch is removed.

The urgent `.49` batch remains ready for maintainer commit and the governed release
flow, with these resolver and dependency-gate corrections included. Package
versions remain `.48`, and the existing open `.49` notes are extended. No broad
validation, version bump, commit or push ran.

## Dependency gate repair after maintainer validation

The maintainer committed the urgent batch at `239a49173` and started the governed
release validation. Its dependency gate found newly yanked `yoke-derive 0.8.3`.
The warning has no advisory ID; Bash collapsed the empty first TSV field and
misreported the package as `0.8.3` and the warning kind as `yoke-derive`.

Only that locked package and its checksum now advance to compatible `0.8.4`.
The gate uses a nonempty placeholder for absent advisory IDs. Focused classification
regressions accept advisory-free informational warnings and still reject yanked
dependencies, known vulnerabilities and direct unmaintained dependencies.
`--classification-only` runs these cases without creating the separate Git
database fixture; that unchanged offline-isolation scenario was not rerun.
Shell syntax and targeted ShellCheck pass. The real `make dependency-risk-gate`
passes with zero vulnerabilities and the two existing transitive warnings;
log: `target/review-validation/dependency-risk-yoke-final.log`.
Locked offline Host/CLI compilation passes in
`target/review-validation/dependency-risk-yoke-compile.log`. The corrective slice
is ready for maintainer commit and retry of the governed release command. No broad
validation was rerun; package versions remain `.48`, and changes extend the open
`.49` notes. The failed release gate has not produced a completed validation receipt.

## Urgent CANIC-188 publication preparation

The maintainer now wants to publish because CANIC-188 is blocking downstream
work. The [urgent .49 boundary](../design/0.110-fleet-runtime-contraction/status.md#urgent-49-publication-boundary--2026-09-30)
includes the implemented incident corrections and completed worktree fixes.
Remaining R2–R8 outcomes stay accepted follow-ups; earlier statements requiring
the whole expanded review before any push are superseded. The tracked review
minimum remains 27 of 401, and the minor is not closed.

The urgent implementation batch is **ready for maintainer commit and the governed
release flow**. Final qualification passes the explicit 24-source public CLI case
`pic::fleet_registry::baseline::tests::completed_reset::incident_estate_reset_recovers_and_replays`.
It uses nine installed Workloads and fifteen empty spares; ordinary governed
release cases keep the smaller estate. All 24 imports, exact conservation,
offline unchanged-journal replay, full Fleet convergence and a later ordinary
Ensure pass. Import used 220 of 784 calls and 51,096,447,856 observed Root debit
cycles within the 33,008,458,000,000-cycle ceiling. The case took 322.32 seconds;
the governed invocation took 341 seconds. This is local-network execution;
mainnet quote/call bounds are qualified separately by native tests.

The first attempt exposed an assumption
in the new capacity-negative fixture; the second completed all 24 imports and
conservation but used a Fleet-only field in its import replay assertion. Both
fixture assertions are corrected. Logs are under `target/review-validation/`:
`canic188-24-source-pocketic.log`, `canic188-24-source-rerun.log` and the passing
`canic188-24-source-final.log`.

Targeted native qualification passes 29 Control Plane import tests, two Core
budget tests and 78 Host import tests, plus the exact Host generation journey
that owns no-op seed-byte preservation and actual ID publication: **110 tests**.
The initial generator-module filter did not select that helper; the explicit
owning test did. Logs are `canic188-native-final.log` and `canic187-seed-final.log`.
Changed Testing-package all-target/all-feature warning-denied Clippy passes in
`canic188-clippy-final.log`; its dependencies also compile/lint. Earlier completed
review slices retain their owning test/lint evidence below. Formatting, whitespace,
layering, document semantics and `.49` release-note preflight pass. The two known
document-layout warnings remain advisory. Scratch is back to 48 KiB.

The retained frozen `.48` repair bundle passed all 17 checksum checks. Its
limited state-preservation qualification remains valid; this current-runtime
journey does not replay the entire historical incident. Publishing `.49` does
not resume the old import. The exact staging repair still needs its own live
preconditions and authority; the new local `.48` operation with an issued
uninstall is outside that repair. No live or sibling mutation was performed.
Versions remain `.48`, with open `.49` notes ready for the governed version
transaction. Native macOS execution remains for CI. No full suite, version bump,
commit, tag or push ran. The next publication step is the maintainer's source
commit, then `make patch` and the governed release targets; agents must not create
commits. Status text supplies no release authority. The remaining review queue
does not block this bounded corrective batch or become closed by its publication.

## Latest continuation — R5 restore retention

Two more original findings are qualified: `backup-persistence-1` and
`backup-persistence-9`. The conservative tracked minimum is now **27 of 401**;
partial fixes and native macOS qualification remain excluded.

Backup creation/execution, restore preparation/run and prune share a stable
parent-side layout lock. Restore publication durably retains the source before
publishing its journal, including custom external journal paths. Paused/failed
restores, owner death and surviving command descendants retain artifacts.
Terminal receipt replay releases only that journal's exact reference after
command quiescence. Completed external journals remain replayable after pruning;
new journal writes cannot recreate an already pruned source.

Prune verifies complete backup artifacts before counting retained copies, locks
those copies through deletion, and reports busy/referenced/invalid layouts as
skipped. Partial failures preserve successful deletion outcomes in the printed
report and return failure. `--keep 0` requires no extra confirmation.

Final targeted qualification passes 125 Backup persistence/runner/publication
tests and 108 CLI backup/restore tests (one live-environment test intentionally
ignored). Affected-package all-target/all-feature warning-denied Clippy passes.
Process-death tests cover initial journal publication, terminal receipt publication
and orphaned command trees; public CLI tests cover default/external preparation,
concurrent prune, corrupt newest copies and partial deletion failures. Logs:
`target/review-validation/backup-lifetime-final.log`,
`backup-lifetime-cli-final.log` and `backup-lifetime-clippy-final.log`.
Document semantics and layering pass; the document guard retains its two known
advisory layout warnings. Invocation-owned scratch is removed.

The accepted R1–R8 batch remains **not ready to push**. Upload completion,
consistent capture, restore authority and the other pending R2–R8 outcomes still
need work. Fresh live backup execution remains outside scope. Package versions
remain `.48`; `.49` notes and operator documentation are extended. Changes remain
uncommitted; no release or live deployment ran. Native macOS results remain for CI.

## Latest continuation — R5 filesystem recovery

The broader review continued with five original findings:
`backup-persistence-3`, `backup-persistence-4`, `backup-persistence-5`,
`backup-persistence-8` and `backup-persistence-10`.
Manifest retry preserves the original publication provenance while rebuilding
all authority and artifact fields. It rejects conflicting manifests and checks
for a direct regular file before adoption. The backup runner resolves the selected
root once, so relative/dot/parent paths and directory links work through download,
pause/resume and verification; artifact-tree links still reject. JSON reads are
buffered. CLI layout creation now takes the runner's execution lock before
reading or publishing the plan and journal.

Final native filesystem qualification passes 120 targeted Backup persistence,
backup-runner and restore-runner tests, including process-death recovery. The CLI
library's 45 backup tests pass, including competing creation with unchanged files.
Affected-package all-target/all-feature warning-denied Clippy passes.
The first CLI command mistakenly selected its testless binary target; its zero-test
result is excluded. Logs are under `target/review-validation/`:
`backup-filesystem-final.log`, `backup-cli-final.log` and
`backup-filesystem-clippy.log`.

The five fixes extend the prior conservative count of 20 distinct original
findings to 25. This is a tracked minimum,
not a disposition of all 401 findings. Partial import/funding work and native
macOS qualification remain excluded. Allocation-scoped replay authority was
inspected but not edited in this continuation; it still needs a coordinated
change to generated operation IDs, receipt actors, issuer and funding authority.
Prune/restore lifetime coordination was still open at this checkpoint and is
qualified above. Upload completion, consistent capture and the remaining R2–R8
outcomes remain open. Fresh live backup execution remains
outside the batch. Package versions remain `.48`, `.49` notes are extended,
and the complete accepted batch is **not ready to push**.

## Resumed after maintainer reboot

The maintainer resumed the accepted correction batch after reboot. All changes
remain uncommitted. Shared target ownership was checked before targeted tests.
IcyDB lockfile refresh to `0.261.19` is complete.

Latest completed checks: 76 Host import tests; affected Host/Backup/Testing
warning-denied Clippy; expired-ingress recovery PocketIC (35.00 seconds,
140-second governed invocation); reset/ordinary Ensure and funding-rotation
PocketIC; 15 Host durability, 70 Backup persistence and 32 restore-runner tests
with a linked temporary directory. Mac PocketIC archive/binary hashes, cached
installation and corrupt-cache rejection pass through platform-selection probes;
native Mac execution remains for CI. Document semantics and layering passed
before the latest placement edits.

Resumed placement qualification passes: 53 feature-enabled Core placement tests,
31 intent-store tests and warning-denied Core/Control Plane all-target/all-feature
Clippy. This covers late index completion, safe shard ownership, failed local
admission cleanup and index quota reclamation after more than 1,000 completed keys.
Five fixture-grant tests passed before the build-directory cleanup. macOS workflow
syntax, shell lint and the revised release-integrity guard pass.

At the maintainer’s request, repository `.tmp` was reduced from about 34 GiB to
48 KiB; only the compiler-cache runtime directory remains. Earlier external cleanup
removed `target/` and `/tmp/canic-review-*.log` while compilation was running.
New validation logs are under `target/review-validation/`.

The repeated-recycling R4 slice is qualified: registered child
records retain allocation operation IDs; recycling admission/completion binds the
whole allocation claim; subtree capacity remains fenced until Directory convergence.
A PocketIC fixture now recycles the same canister twice and replays the older
removal. All 43 owning pool tests and 26 isolated Root registry tests pass, as do
warning-denied CP/Testing lint and the isolated Root lint check. The exact
PocketIC case passes in 459.44 seconds (652-second governed run with rebuilding):
`pic::fleet_registry::baseline::tests::prepared_root_initial_shard_bootstrap_reaches_terminal_component_membership`.
The reset slice is also qualified. It leaves interrupted management effects pending, deletes retained
snapshots before publishing empty capacity and admits reconciliation of unclaimed
created/recycled inventory without changing its provenance. It also accepts a
concurrent resumer's exact archived Directory completion.
Manual/background reset exclusion now follows the existing capacity-import
execution-guard pattern, with durable pending state surviving cancellation. Native
qualification passes 45 pool tests and five maintenance-policy tests. Core/Control
Plane/Testing warning-denied all-target/all-feature lint and isolated Root lint
pass. The same PocketIC case passes in 52.41 seconds (71-second governed rerun),
including snapshot deletion, rejected/retried recycled-asset reinspection and two
concurrent reset requests. The fixture now compares exact workload claims during
terminal provisioning replay while permitting unrelated pending pool maintenance
to finish. Targeted logs are `target/review-validation/reset-*.log`; the failed
whole-pool fixture assertion is retained in `reset-pocketic.log`, and the passing
run is `reset-pocketic-rerun.log`. Layering, document semantics and whitespace
checks pass. Invocation-owned scratch was removed; `.tmp` remains 48 KiB.
Allocation-scoped caller authority and the remaining R2–R8 items stay open.
The complete batch remains
**not ready to push**. No release, commit, push or live deployment was performed.

The R4 routing and terminal-replay slice is qualified. Directory children and
their local cache retain the existing Root allocation operation ID. Index bindings, shard
assignments and scaling workers use that identity when routing or counting live
capacity; delayed placement replies must still match it. Retired successful
replies settle their own allocation accounting and release only their original
index claim; an already disposed outcome stays disposed. Explicit recovery frees
an unavailable index binding, and shard release preserves a reused allocation's
count. Recovery also rechecks routing after asynchronous cleanup. Recycle RPC
payloads include the expected target allocation. Child records require explicit
allocation-field presence even when the infrastructure value is null.

Completed removals now replay while a later allocation uses the same physical ID.
Storage proves retirement of the original allocation; workflow consumes that
proof instead of requiring permanent physical absence. Workflow distinguishes
removal of a descendant from removal of the actual target, and the driver returns
terminal completion without retrying it as unfinished work.

Qualification passes 78 focused Core tests and 37 isolated Root registry/workflow
tests, plus four Root state-contract checks. Warning-denied affected-package
all-target/all-feature Clippy and final isolated Root library/test lint pass.
The exact owning PocketIC case named above passes in 102.85 seconds (233-second
governed proof, 234-second runner). It rejects old routes and new recycle requests
for retired allocations, replays both direct removal and recycle RPC twice while
the replacement remains unchanged, then completes repeated recycling and reset
recovery. Final logs are `routing-final-native.log`, `routing-root-workflow.log`,
`routing-final-clippy.log`, `routing-root-workflow-clippy.log` and
`routing-pocketic-completion.log`, under `target/review-validation/`.
Keep `routing-pocketic-rerun.log` and `routing-pocketic-final.log`: they reproduce
the separate storage and workflow permanent-absence failures before correction.
The final Root artifact is 8.89 MiB, with 1.11 MiB headroom. Invocation scratch was
removed and `.tmp` is 48 KiB. Root replay actors, issuer policies and per-child
funding totals still require allocation-scoped authority work. The full accepted
R1–R8 batch remains **not ready to push**; package versions remain `.48` with the
open `.49` changelog updated. Native macOS results remain outstanding.

Remaining review notes: caller reuse also regenerates placement and cycle-topup
operation IDs from reset local counters. Scope those IDs and replay actors/keys
to the managed allocation together; changing only the replay key still leaves
Root allocation-operation collisions. Do not purge uncertain paid receipts as a
shortcut. The prune lifetime and retention findings recorded at this checkpoint are
now qualified by the latest R5 continuation above. No separate `--keep 0`
confirmation was introduced. Upload completion, consistent capture and restore
authority remain open.

R6 source recheck confirms that top-up watchdog recovery discards the recovered
timer directive, and allocation-driver deduplication survives only the ordinary
completion path. The locked timer provider is still `ic-timers 0.8.0`. Re-arming
the same stuck `Once` claim is insufficient; qualify a pre-armed watchdog with
separately spawned, fenced work using the existing fixture-import owner as the
local pattern. Keep lifecycle-driver concurrency bounded without allocating a
new timer registration on every operation replay. Real post-await trap recovery
and absence of duplicate paid effects remain required evidence, not completed checks.

## Active code-review correction batch

The maintainer stopped the push and authorized implementation of the reviewed
correctness and deployment-recovery corrections. The
[accepted correction sequence](../design/0.110-fleet-runtime-contraction/status.md#accepted-code-review-corrections--2026-09-30)
owns R1–R8 and qualification. The maintainer explicitly confirmed editing authority
after the initial read-only review. R1 quota accounting and the first R2 import
preflight/operation-selection corrections are implemented, with qualification in
progress. The earlier cleanup evidence below remains
valid for that narrower work, but the expanded batch is not ready to push.
Keep package versions `.48` and extend the open `.49` notes. Preserve the
maintainer's dependency changes in `Cargo.toml` and `Cargo.lock`. No broad gate, commit, release,
live incident execution or sibling mutation is authorized by this continuation.

Current targeted results: 86 Core cost-guard, intent, stable-bound and runtime
status checks; 16 Host operation-selection checks; 15 native durability checks;
73 import checks; and the generated-estate fixture with failed-bootstrap-review
recovery. Reopened-journal regressions prove failed free submission and inspection
prechecks leave allowances intact. Inspection preparation also preserves Root
reserve diagnostics and allows unrelated Roots to remain active. macOS has two
native CI lanes, but no native macOS result has run in this Linux session.

The expanded completed-reset PocketIC journey now passes, including ordinary
Ensure review and apply under the retained completed protocol owner, recovery and
effect-free replay (537.22 seconds; governed invocation 739 seconds). The changed
signed-handoff case passes in 154.91 seconds, including fixture builds. Exact
funding receipt replay during rotation passes on PocketIC in 29.15 seconds;
its assertion permits bounded ingress execution costs while requiring unchanged
funding records. An additional 24 signing, 24 funding/retry and six CLI output
tests pass. Warning-denied Clippy passes for all targets/features of Core,
Control Plane, Host, CLI and Testing at that source state.

Linked-temporary-directory qualification passes on Linux: 15 Host durability,
70 Backup persistence and 32 restore-runner checks. This exercises macOS's path
shape; it is not native macOS qualification. Import journal sizing includes all
maintained stop and confirmation phases, with all 73 import tests passing before
the next recovery change.

Current implementation adds durable certified handoff retirement: a pruned done
response or certified absence within the specified five-minute expiry window
permits exact custody reconciliation or bounded renewal. Never-issued envelopes
can refresh, and issuance and its submission charge publish atomically. All 76 focused
import tests and affected-package warning-denied Clippy pass. The expired-ingress
PocketIC extension passes in 35.00 seconds (140-second governed invocation). Older
unknown outcomes, exhausted import budgets and the remaining R2–R8 work are still
open. The maintainer selected IcyDB `0.261.19` and explicitly requested the matching
lockfile refresh; all six IcyDB packages agree, with unrelated versions unchanged.
The PocketIC installer now selects checksum-bound binaries for both Mac hosts;
CI installs/runs the native tools and finalizes a demo canister artifact. Installer
platform/hash qualification is in progress. Logs are `/tmp/canic-review-*.log`. Do not infer complete-batch readiness from these
results.

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

The September 30 failure was a second stale fixture composition: the bootstrap
hold case combined two source reviews while retaining one source's 24-call
budget. The shared review helper now takes the complete source slice and derives
its call/debit bounds from that count and the Root quote. Bootstrap and all
signed-handoff callers use it directly; no merged single-source budget remains.
The exact bootstrap-hold case passes in 17.05 seconds, and the signed-handoff
recovery case passes in 27.51 seconds. Warning-denied test-target Clippy passes.
Logs are `/tmp/canic-bootstrap-capacity-regression.log`,
`/tmp/canic-import-transport-regression.log` and
`/tmp/canic-capacity-fixture-clippy.log`. The `.49` changelog includes this fix.
The accepted local batch is ready for maintainer review; these targeted checks
do not claim a full-suite rerun or change the outstanding live recovery boundary.

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
