# Canic 0.110 Implementation Status

## Maintained scope

The .48 package set is published. CANIC-185 target-local bootstrap funding,
role-owned activation persistence, optional observability, passive blob
contracts, artifact reuse/admission and IcyDB composition corrections are
implemented. The [current handoff](../../status/current.md) owns the active
cleanup and CANIC-187/188 correction batch; [release notes](../../changelog/0.110.md) describe shipped behavior.

The normative [design](0.110-design.md) and independent
[size follow-through amendment](2026-09-28-toko-size-follow-through.md) define
accepted scope. The amendment reopened only the bounded .47 activation,
observability and blob-contract work; it did not restart the full B3/B4 matrix.

## Batch dispositions

| Batch | Disposition |
| --- | --- |
| B1 | Accepted by the maintainer on 2026-09-23; retained controlled measurements and source/interface dispositions remain evidence. |
| B2 | Complete with the accepted cold-query tradeoff; role-selected storage/restoration evidence remains retained. |
| B3 | General record/codec restructuring stopped; the bounded .47 activation amendment is implemented. |
| B4 | Remaining general pruning deferred and unscheduled; bounded .47 observability and passive blob-contract changes are implemented. |
| FI1 | Bootstrap and capacity-import implementation/qualification shipped in .43; subsequent reinstall/recovery corrections shipped through .47. |
| B5 | The .42 checkpoint is qualified; final closeout must cover FI1 and subsequent corrections and receive human acceptance. |
| Cleanup | Complete and ready for maintainer review: unused runtime/Host paths, obsolete helpers/tests and status-owned release flows removed, duplicate evidence consolidated, focused checks passed. |
| Reinstall review corrections | Shipped in .48: typed unavailable funding diagnostics and exact-digest unpaid-review cancellation. Toko confirms live cancellation and infrastructure completion. |

| CANIC-187/188 | Implemented and locally qualified in the open .49 batch: whole-operation import budgets, successful-call settlement, seed-byte preservation and an exact issued-import repair. Live repair and downstream convergence remain outstanding; see the [repair decision](issued-import-recovery.md). |

## Current completed-Fleet cleanup — 2026-10-01

The maintainer accepted removal of superseded completed-source preparation,
receipt/interface reconstruction, sealing and reset paths. Current clean
reinstall owns completed-Fleet replacement; shared certified custody observation,
unfinished activation and paid-import reconciliation remain. The
[current handoff](../../status/current.md) owns qualification and complete-batch
readiness alongside the concurrent deployment-reliability corrections.

## Urgent .49 publication checkpoint — 2026-09-30

The maintainer has prioritized publication because CANIC-188 blocks downstream
work. The urgent batch includes the implemented CANIC-187/188 corrections,
qualified cleanup and completed review corrections already in this worktree,
the dependency-gate and Cargo resolver 3 repairs, and same-operation bootstrap
registration budget/funding recovery in Host/CLI.
The remaining R2–R8 findings below stay accepted, sequenced follow-up work;
finishing all 401 review findings is not a prerequisite for this corrective release.
This boundary supersedes earlier handoffs that required the entire expanded
review batch before publication. It does not close those findings or the minor.

At this checkpoint the urgent batch was ready for maintainer review. Later
deployment-audit corrections and the accepted cleanup supersede that readiness;
consult the current handoff before publication.
Qualification passes an explicit public CLI reset journey with nine Workloads and
fifteen Ready spares, 110 targeted import/budget/seed tests and changed Testing-package
all-target/all-feature warning-denied lint. The journey covers all-source completion,
conservation, offline replay and later ordinary Ensure. Ordinary release qualification
retains the smaller estate; the incident-sized journey is opt-in. The
[current handoff](../../status/current.md) records exact results and retained logs.
Bootstrap registration recovery additionally passes 28 targeted native tests,
the recovery and ordinary bootstrap PocketIC journeys, and Host/CLI/Testing lint.
The recovery journey preserves applied effects through lost replies and verifies
pool import only after completion, followed by ordinary Ensure. The external
developer's retained operation has not been executed here.
Package versions remain `.48` and `.49` release notes remain open until the governed
version transaction. No broad gate or Git publication has run; native macOS evidence
remains for CI.

Publishing the corrected packages does not recover an already issued `.48`
import. The [exact staging repair](issued-import-recovery.md) retains its own
qualification and live-authority boundary. Toko's newly reported local import
with an issued uninstall is a different operation and is not covered by that
repair. No live or downstream mutation is part of this publication preparation.

## Accepted code-review corrections — 2026-09-30

The maintainer initially stopped publication and authorized correction of the September 29
code review against the current source. The existing cleanup qualification does
not establish readiness of this expanded batch. Keep the `.49` draft open and
package versions unchanged; no next-minor implementation or live incident
execution is authorized. Blob follow-up and fresh backup enablement remain
outside this batch. macOS Host/CLI support includes both declared architectures.

These outcome groups sequence implementation and evidence, not patch releases.
Their remaining work follows the urgent publication boundary above.
Each includes its diagnostics, affected fixtures, current documentation and
cleanup. Use typed authority and bounded debit evidence; incidental balance or
Registry changes must not prevent safe same-operation recovery.

| Batch | Outcome / owner | Included evidence and focused validation | Status |
| --- | --- | --- | --- |
| R1 | Bounded quota accounting / Core | Sustained quota windows, outstanding reservations, expiry and exact settlement replay; Core intent and cost-guard tests and lint. | Implemented and qualified |
| R2 | Recoverable import and Host operation selection / Host, CLI, Control Plane | Failed prechecks, submitted/unknown outcomes, finite budgets, delayed reviews and a new operation after completed reset; targeted Host tests and exact import/reset PocketIC journeys. | Prechecks, inventory, review-clock recovery and post-reset operation selection qualified; certified request retirement passes native and expired-ingress PocketIC evidence; exhausted budgets and older unknown outcomes pending |
| R3 | Receipt-backed funding and conservation / Host, Core, Control Plane | Signing admission, grant reconciliation, credits, bounded debits, funding fences and exact replay; owning native tests and paid-effect PocketIC journeys. | Signing admission/proof reuse pass native tests; grant replay during rotation passes native and PocketIC evidence; remaining accounting pending |
| R4 | Safe placement and recycling / Core, Control Plane | Concurrent resumers, repeated claims, grant/replay residue and snapshot cleanup; owning native tests and actual lifecycle PocketIC cases. | Placement ownership, capacity, repeated recycling and reset recovery pass native/PocketIC evidence. Allocation-bound routes, counts, delayed replies, recycle targets and completed-removal replay pass 78 Core and 37 Root tests, lint and the owning PocketIC journey. Caller replay, issuer and funding authority isolation remains open |
| R5 | Reliable existing backup/restore / Backup, CLI, Core | Prune locks, complete uploads, consistent capture, release/target authority and restored admission; runner crash tests and affected PocketIC paths. | Manifest/path recovery and creation locking qualified. Prune/restore lifetime coordination, verified retention and partial-deletion reporting pass 125 Backup and 108 CLI tests plus affected-package lint. Complete uploads and remaining authority/capture work pending |
| R6 | Recoverable background drivers / Core, Control Plane | Typed retry decisions, one owned driver, backoff, traps and same-release restoration; timer policy tests and actual PocketIC recovery. | Platform-unavailable retry classification passes native tests; driver ownership/trap recovery pending |
| R7 | Convergence across unrelated Fleet changes / Control Plane, Host | Operation-specific authority, mirror acknowledgements, rotation and activation fences; multi-root publication/retry PocketIC cases. | Pending |
| R8 | Complete retirement/reset accounting / Host, Control Plane | Physical inventory, cycle/ICP evacuation, reserved cycles and reviewed residuals; terminal conservation and effect-free replay. | Pending |
| Qualification | macOS and execution boundaries / Host, CLI, Testing | Native macOS build/filesystem evidence, test selection, message/record admission and artifact identity; focused owner checks. | Native macOS CI configured; linked-directory Host/Backup/restore checks pass on Linux; native results and execution-boundary work pending |

R4 traceability: the placement ownership, admission, quota and point-lookup work
addresses `core-placement-fleet-1` through `core-placement-fleet-5`; delayed receipt
completion addresses `core-placement-fleet-9`. Allocation-bound routing and
Directory fencing address `r2-recycled-principal-authority-4`, while interrupted
reset handling addresses `r2-recycled-principal-authority-6`. The issuer, replay
and funding findings `r2-recycled-principal-authority-1` through
`r2-recycled-principal-authority-3` remain open. The terminal-removal replay
regression additionally exposed permanent-absence checks in storage and workflow;
their correction passes direct-command and RPC replay with the replacement unchanged.

The review's original IDs remain the traceability keys. Recheck findings against
current source before changing behavior; prior reproduction does not establish
that the current source still has the defect. Low/informational findings require
triage, and performance claims require measurement. Final readiness requires the
complete accepted outcome and its direct rejection/recovery evidence, not only
the most recent slice.

R5 filesystem traceability: manifest publication recovery addresses
`backup-persistence-4`; selected-root resolution addresses
`backup-persistence-3` and `backup-persistence-5`; buffered JSON reads address
`backup-persistence-10`; locking CLI layout creation addresses
`backup-persistence-8`. The next continuation closes `backup-persistence-1` using
a parent-side layout lock and durable restore references, including external
journals. Owner death, surviving command descendants, paused/failed work and
terminal replay retain/release the exact source authority. `backup-persistence-9`
is addressed by verified retention, locked retained copies and per-entry deletion
outcomes. Qualification passes 125 owning Backup tests and 108 CLI tests; the
disposable-live-environment restore test remains intentionally ignored. These
corrections do not close upload completion, consistent capture or restore
authority findings, and do not enable fresh live backup execution.

The conservative counted set is 27 distinct original findings after R5
filesystem and retention qualification. Duplicates map to their original primary finding;
partial corrections and unrun native macOS qualification are excluded:

- R1: `core-intent-replay-1`.
- R2: `host-ensure-capacity-6`, `host-ensure-capacity-7`,
  `host-ensure-capacity-9`, `host-ensure-reinstall-1`,
  `host-ensure-reinstall-5`, `r2-wire-mirror-agreement-5`.
- R3: `core-auth-1`.
- R4: `core-placement-fleet-1`, `core-placement-fleet-2`,
  `core-placement-fleet-4`, `core-placement-fleet-9`, `xc-efficiency-1`,
  `r2-recycled-principal-authority-4`, `r2-recycled-principal-authority-6`,
  `xc-async-7`.
- R5: `backup-persistence-1`, `backup-persistence-3`, `backup-persistence-4`,
  `backup-persistence-5`, `backup-persistence-8`, `backup-persistence-9`,
  `backup-persistence-10`.
- R6: `xc-async-1`.
- Filesystem/governance: `host-icp-network-11`, `xc-contracts-5`, `xc-contracts-6`.

This is a tracked minimum, not a completed disposition of all 401 findings.

## Retained evidence and remaining acceptance

- [B1 review packet](../../audits/working/0.110-fleet-runtime-contraction/b1-input-evidence.md#complete-b1-review--accepted-2026-09-23).
- [B2 reachability ledger](../../audits/working/0.110-fleet-runtime-contraction/b2-storage-reachability.md).
- [CANIC-185 funding qualification](../../audits/reports/2026-09/2026-09-29/toko-bootstrap-funding.md).
- [CANIC-185/186 review readiness and cancellation qualification](../../audits/reports/2026-09/2026-09-29/toko-unpaid-review-cancellation.md).
- [Bounded size qualification](../../audits/reports/2026-09/2026-09-28/toko-wasm-size-audit.md#selected-implementation-follow-through).
- [Final closeout audit owner](../../audits/release-lines/0.110-closeout-audit.md).

Downstream live/performance acceptance and complete cycle-attribution evidence
remain separate from the passing Canic implementation checks. Downstream source,
application state and launcher choices are not Canic release gates. Necessary
correctness, security and recovery corrections remain in scope for this line.

The next minor is not authorized by this cleanup or by a passing gate. The
maintainer must explicitly request and accept the final 0.110 closeout audit
before 0.111 implementation begins. Do not restart completed B1 ablations,
stopped B3 work or deferred B4 work without a new scope decision.

## History

The [dated implementation history](../../status/archive/2026-09-29-fleet-runtime-contraction.md)
retains every prior checkpoint, measurement, review and historical queue.
Its old “next action” statements are historical; the dispositions above govern
current work.
