# Canic 0.110 Implementation Status

## Maintainer-requested .52 checkpoint — 2026-10-03

The maintainer needs an earlier push after dependency edits finish. Include the
implemented FR1 discovery/assessment APIs and production corrections in the open
.52 draft, alongside the completed Host/Backup and documentation work. This is a
checkpoint boundary under delivery cadence; it does not close FR1 or expose its
unfinished release command. Keep the remaining execution, quiescence, custody and
conservation work in the accepted FR1 batch. Final-graph targeted lint, native,
inventory, Candid, embedded-peer and four exact PocketIC proofs pass. The .52
checkpoint is ready for the maintainer-selected release flow; the current handoff
owns exact qualification evidence and remaining FR1 scope. No version or Git
action was performed.

## Fresh-shard authentication simplification — complete for .51, 2026-10-02

The maintainer selected restoration of automatic proof fetching for fresh shards
and simpler issuer setup in the open .51 batch. AF1 belongs to Core auth and the
facade Root endpoint contract. Configure policy and renewal together from one
explicit Fleet audience/grant request, preserve authority on identical setup
retries, fetch missing proofs through caller-bound Root authority, and distinguish
missing/disabled configuration from pending signing before paid work.

Qualification covers first preparation and verifier acceptance in PocketIC,
configuration rejection and retry stability, replay-owner revalidation, signing
reuse/backoff, affected DTOs/macros/fixtures, scoped lint and active documentation.
The [current handoff](../../status/current.md) owns completion and evidence.
Toko adoption and staging effects remain downstream work; this batch changes no
sibling repository or deployed installation. FR1 remains independently unfinished.

## Integrated CANIC-183 import diagnostics — 2026-10-02

The maintainer selected separate Toko feedback work while another session owns
Fleet release/reservation and authority-restoration changes. Direct import HTTP
timings are implemented and qualified against published .50 in the independent
source/target bundle `.canic/local-work/toko-feedback-20261002/`.
The maintainer requested integration; the patch is applied to the primary worktree
after its active build finished, preserving the other session's edits.
The [current handoff](../../status/current.md#integrated-toko-import-diagnostics-canic-183--2026-10-02)
owns the exact source, evidence and integration boundary.

Root/Coordinator queries, signed handoff, certified request-status reconciliation
and Root commands share the existing bounded invocation receipt. Imported-child
attribution survives client cloning and Host reopen; async cancellation/panic
retain incomplete pairs without leaking parents or worker counts. Native import,
timing and CLI receipt tests, affected-package lint and the exact real-HTTP
PocketIC handoff/recovery/replay case pass against the isolated source. Patch
integrity and document checks pass after integration; combined compile/test
qualification remains with the other session. The .51 draft describes this
diagnostic batch without changing package versions.

This fills the direct-HTTP receipt gap only. Full import/IC child attribution,
matched 24-source deployment performance and Toko live acceptance remain open.
FR1/CS1 and the other session's qualification retain their separate owners.

## Maintained scope

The maintainer reports 0.110.51 published and selected continuation of the
accepted FR1 whole-Fleet release-to-capacity batch. Published corrections,
including RD1/CANIC-191, CANIC-190 and CANIC-192, are not reopened. Packages remain
.51; the open .52 checkpoint draft includes completed Host/Backup corrections
and implemented FR1 discovery. Remaining FR1 execution stays in root `Unreleased`. Sibling blob implementation is
outside this batch; the independent service must have no Canic dependency.

The parked FR1 source has been restored and reconciled against .50. Keep
`.canic/local-work/fr1-separated-20261001T200203Z/` as recovery evidence; do not
apply its restoration patch again. Durable read reservations, authenticated
registered inventory and the internal Core release fence are requalified. Spent
read reservations alone no longer prevent explicit reset; unresolved executable
effects retain their reconciliation requirement. No whole-Fleet release command
is exposed and no retirement implementation has yet been removed.

The [current handoff](../../status/current.md) records 74 passing native tests,
four exact PocketIC cases, affected-package Clippy and the refreshed embedded
peer. These qualify the restored foundations, not complete FR1 execution. Next
complete paid-obligation/account collection, role quiescence, bounded handoff,
existing-journal execution/recovery, CLI, whole-Fleet proof and contraction.
Declared-account observation now reads real ICRC balances with bounded signed
queries and canonical default-subaccount identities; 30 selected native tests,
Host Clippy and the exact account PocketIC proof pass. This does not yet discover
every role/application account, qualify recovery artifacts or settle external effects.
Root funding evidence now has a controller-only stable census, bounded to 32
returned refills plus one lookahead. Exhausted notifications, historical
accounts, refunds/CMC expiry, pending requests, accepted grants and rotations
remain visible. Core's 84 refill tests, 10 Root funding tests, affected-package
Clippy, fixture refresh and the real Ledger/CMC PocketIC case pass (226.87s;
`fr1-funding-census-*`). This is evidence collection, not paid settlement or a
producer fence, and adds no original-review closure count.
The Host collection boundary now consumes those pages for every selected Root,
bracketing bounded signed reads with certified owner custody and matching
Registry observations. It retains exhausted/history evidence, checks page/header
continuity and returns no partial census on refusal. Five native tests, Host
Clippy and the extended ownership PocketIC case pass (0.88s; 70s runner;
`fr1-host-funding-*`). Execution/CLI, settlement and quiescence remain separate
unfinished FR1 requirements.
Host receipt assessment now separates known no-transfer outcomes, completed
conversions/refunds, residual review and unresolved Ledger/CMC work. Durable
transfer uncertainty fixes reservation clearing and changed-fee replay after a
lost Ledger reply. The required current-v1 record field is reinstall-only.
Core/Host/Root selected native tests pass (86/42/17), along with affected-package
Clippy, embedded refresh and the exact Ledger/CMC and Host PocketIC cases
(229.71s/0.87s). Evidence: `fr1-funding-assessment-*` and
`fr1-transfer-uncertainty-native.log`. This does not complete settlement or FR1.
Coordinator treasury evidence now joins the bounded signed Host census through
its existing funding query. Complete Root membership and policy/lifecycle binding
are checked independently of order. Pending grants/rotations remain visible,
while terminal history alone creates no pending work. All 44 selected Host tests,
Host library/test Clippy and the exact signed-query PocketIC case pass (1.29s;
72s runner; `fr1-coordinator-evidence-*`). Other obligation owners, quiescence
and execution remain incomplete.
Root pool evidence now has a controller-only bounded singleton query preserving
bootstrap Store/source holds, import progress and exhausted budgets, uncertain
creation and pending handoff without new-work admission or mutation. All 62 pool
native tests, affected-package Clippy and fixture refresh/verification pass. The
exact import PocketIC case passes in 88.42s (170s runner), including both custody
paths, authorization, progress/replay and unchanged balances (`fr1-pool-census-*`).
Host pool collection is implemented with bounded signed reads, custody/Registry
bracketing and exact Root/subnet checks. After space was freed, its 22 selected
release-ops native tests and Host library/test Clippy pass. The exact signed-query
PocketIC proof passes in 1.77s (4s runner), including two Roots, late refusal,
replay and unchanged balances (`fr1-host-pool-*`). Provisioning/Directory journal
discovery now reads one bounded stable operation per page with key-only lookahead,
independently of active pointers. Its 24 provisioning and three Directory native
tests and affected-package/governed-journey Clippy pass. Host's bounded page
collector passes all 27 selected release tests, Clippy and the extended signed-query
PocketIC proof (2.17s, 4s runner). Fixture refresh/verification pass; the exact
interrupted-to-terminal Root journey passes (30.46s, 51s runner;
`fr1-provisioning-census-*` and
`fr1-host-provisioning-*`). This is not complete paid settlement.
Shared replay discovery now covers Root/Coordinator receipt metadata, retaining
expired uncertainty and original effect/accounting identities. Affected-package
and governed-journey Clippy, three discovery tests and 150 replay regressions pass;
fixture refresh/verification pass. Host's expanded-envelope regression and all
28 selected release tests pass after correcting its type/header limits. The
extended real paid-grant/Coordinator journeys pass (325.14s/142.87s;
`fr1-replay-census-*`). Host receipt collection and production decoder integration
pass 33 selected Host tests, affected lint, signed-query proof (2.78s) and real
Root decoder proof (36.09s). The public Coordinator request/Candid contract now
includes the selector; five contract tests, isolated DTO round trips and affected
lint pass. Final fixture verification and the public-request Coordinator journey
pass (98.39s, 222s runner), as do format/document/whitespace checks
(`fr1-host-receipts-*`, `fr1-replay-canonical-*`).
Complete paid-owner integration, quiescence and execution remain unfinished.
CS1 follows FR1. The normative [design](0.110-design.md) and independent
[size follow-through amendment](2026-09-28-toko-size-follow-through.md) retain
accepted scope; B3/B4 remain stopped/deferred and the human closeout gate remains.

## Batch dispositions

| Batch | Disposition |
| --- | --- |
| AF1 | Complete for open .51: automatic missing-proof fetching and one idempotent issuer configuration command. Direct rejection/retry, replay ownership, signing reuse/backoff, real-canister proof delivery to `Valid` and verifier acceptance, protocol/fixture/docs propagation and embedded-peer qualification pass. Toko adoption and staging acceptance remain downstream; no deployed authentication is claimed. |
| B1 | Accepted by the maintainer on 2026-09-23; retained controlled measurements and source/interface dispositions remain evidence. |
| B2 | Complete with the accepted cold-query tradeoff; role-selected storage/restoration evidence remains retained. |
| B3 | General record/codec restructuring stopped; the bounded .47 activation amendment is implemented. |
| B4 | Remaining general pruning deferred and unscheduled; bounded .47 observability and passive blob-contract changes are implemented. |
| FI1 | Bootstrap and capacity-import implementation/qualification shipped in .43; subsequent reinstall/recovery corrections shipped through .47. |
| RD1 | CANIC-191 admission correction is locally qualified: current artifacts, explicit physical custody and existing archive/reset owners replace predecessor-completion gates. 135 focused native tests, targeted lint and the partial-bootstrap/activation/import public-CLI PocketIC proof pass (249.78s). In-flight provisioning reconciliation remains; remove its preparation machinery only when those uncertain-effect obligations are covered. No live Toko recovery. FR1 continues as a separate batch after .50. |
| FR1 | Active after .50: reservation, inventory and release-fence foundations are restored and requalified against the released corrections. Read-budget bookkeeping no longer blocks explicit reset. Complete paid-obligation/account collection, role quiescence wiring, execution/recovery, CLI, whole-Fleet qualification and retirement contraction remain. |
| CS1 | Accepted and pending after FR1: all seven audited simplifications, including direct qualification, propagation and cleanup, must complete before B5/human closeout and 0.111 blob removal/extraction. |
| B5 | The .42 checkpoint is qualified; final closeout must cover FI1, subsequent corrections, FR1 and CS1 and receive human acceptance. |
| Cleanup | Complete and ready for maintainer review: unused runtime/Host paths, obsolete helpers/tests and status-owned release flows removed, duplicate evidence consolidated, focused checks passed. |
| Reinstall review corrections | Shipped in .48: typed unavailable funding diagnostics and exact-digest unpaid-review cancellation. Toko confirms live cancellation and infrastructure completion. |

| CANIC-187/188 | Maintained import budget, settlement, seed-byte and native-credit corrections are implemented and locally qualified. The maintainer withdrew the frozen `.48` repair exception on 2026-10-01; hard cut plus reinstall applies to malformed/unfinished installations. Historical repair evidence is [archived](../../audits/release-lines/supporting/0.110-fleet-runtime-contraction/canic188-issued-import-recovery.md). |

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

The issued `.48` repair route was subsequently withdrawn by the maintainer on
2026-10-01. Corrected current packages are selected for hard cut plus reinstall;
predecessor state and clients are not repaired to continue an old installation.
Cycle-safe disposition of physical canisters and paid effects remains required.
No live or downstream mutation is part of this publication preparation.

## FR1 sequence and acceptance — 2026-10-01

The maintainer accepts [FR1](0.110-design.md#fr1-fleet-release-to-reusable-capacity--accepted-2026-10-01)
as one complete 0.110 batch before final closeout and blob extraction. After
reporting the urgent `.49` push, the maintainer selected FR1 next. Toko subsequently
reported its separate eight-source import recovered; evidence review and preventive
R2 follow-up are tracked below. No downstream source/state dependency or live-effect
authority is introduced. Package/release decisions
remain maintainer-owned; no per-slice patch versions are allocated.

Host/CLI own the reviewed physical inventory and single operation, Core/Control
Plane own quiescence and bounded effects, and Testing owns management/recovery
proof. Complete whole-Fleet ownership release, Ledger/native/reserved accounting,
controller handoff, empty retained IDs, same-subnet pool or operator-held capacity,
fresh bootstrap, interruptions and effect-free replay. Contract exclusive
retirement commands/records/resumers and duplicate deletion phases only once
the maintained operation owns their safety obligations. Preserve shared current
reset/import, funding, recycling, Store adoption, backup/restore and CANIC-188.

Status: active; Host review admission is the first implementation slice. Its
typed review binds complete ownership, snapshot identity, budgets, account recovery
and same-subnet independent destinations. It exposes no execution command. Runtime
observation provenance, journal integration and end-to-end management evidence are
still required; no retirement path is removed by this slice. Ten native tests,
Host library/test warning-denied Clippy and scoped formatting pass; evidence is
in `target/review-validation/fleet-release-admission-*.log`. Supporting
[usefulness evidence](../../audits/working/0.110-surface-contraction/root-retirement-usefulness.md)
is descriptive; the design owns the exact contract. The shared executor now also
supports exact snapshot deletion under stopped sole-operator custody, with
before/after inventory reconciliation. The focused production-adapter PocketIC
case passes wrong custody/inventory refusal, retained-intent lost-response recovery
and replay without another deletion (`fleet-release-snapshots-pocketic.log`). This
does not qualify whole-Fleet execution or replace release quiescence. FR1 overlaps R8 accounting
but does not close any original finding without its named proof. Latest focused
qualification also passes fourteen native tests, Host/CLI all-feature library/test
Clippy, formatting and runner/shell checks (`fleet-release-snapshots-*.log`).
The authenticated physical reader after operator handoff also passes its focused
PocketIC proof (`fleet-release-observation-pocketic.log`). It binds signer/network
and certified custody, bounds each stable sample to four management reads and
shares the bounded snapshot decoder with deletion. Wrong authority and changed
custody refuse. Complete role/account collection remains required before
whole-Fleet execution; the reader alone grants no reset
authority. The deletion lost-reply regression passes against the shared reader
(`fleet-release-observation-snapshot-regression.log`). Seventeen focused native
tests, warning-denied Host library/test all-feature Clippy, runner regressions,
scoped formatting and shell checks also pass (`fleet-release-observation-*.log`).
The following reservation, inventory and fence checkpoints describe the preserved
FR1 bundle, not the corrective release tree.

In that bundle, durable read reservations live inside the existing Fleet journal. The sealed
review freezes the per-call quote; a non-cloneable token binds operation, canister
and subnet and borrows the journal lock through the reads. Reservations survive
lost results, consume finite call/debit allowance, reject mismatched retained
authority and prevent ordinary Ensure/import takeover while release is unfinished.
The executable plan retains its own identity. Operation creation, complete
quiescence/account recovery and the effect compiler remain pending; this adds no
operator command or second journal. Final focused reservation validation passes
31 native Host/CLI tests, the reserved-observation PocketIC proof, warning-denied
Host/CLI/Testing library/test all-feature Clippy and scoped formatting/diff checks
(`target/review-validation/fleet-release-reservation-*.log`). FR1 remains unfinished.
The registered-ownership collector now shares import's complete pagination and
bounded query decoder. It binds signer/network, exact Registry/Root placement,
Store/child membership and certified custody, without duplicate preliminary child
reads. Its result deliberately carries no quiescence assertion: unfinished
creation/import may retain physical IDs outside the registered pool and must be
reconciled before destructive inventory is complete. Twenty-three focused native
tests and the signed-query/certificate PocketIC wire fixture pass
(`fleet-release-inventory-{native,pocketic}.log`). This qualifies collection, not
real Fleet role release or account recovery. The current quiescence/handoff owners
still require integration; no retirement path has been removed. Final Host
library/test all-feature Clippy with warnings denied, runner regressions, scoped
formatting/diff checks and shell lint pass (`fleet-release-inventory-*.log`).

The Core fence now retains a distinct release purpose with exact operation,
review digest and recipient bindings. Its synchronous internal workflow hook
checks role-owned settlement before suspending producers and committing the seal;
exact replay skips those callbacks. Snapshot recovery cannot reopen it, and all
existing ordinary/recovery commands remain fenced while release-sealed. Seventeen
focused Core/facade tests pass, covering typed refusals, native retained-record
roundtrip/replay, maximum-width encoding and exact canonical Candid status
(`fleet-release-fence-native.log`). This does not qualify live role settlement,
IC restoration or release handoff. No release endpoint is exposed; the actual
paid-obligation census, role integration and bounded command paths remain next.
The changed current `v1` record/status follows reinstall-only release transitions.
Core/facade library/test all-feature warning-denied Clippy, scoped formatting
and diff hygiene also pass (`fleet-release-fence-clippy.log`).

Remaining R2–R8
corrections keep their owners; only FR1's direct safety dependencies join this
batch. The advisory line estimate is not a release gate or deletion quota.

## Pre-blob simplification — accepted 2026-10-01

The maintainer accepts all seven read-only audit candidates into
[CS1](0.110-design.md#cs1-pre-blob-code-simplification--accepted-2026-10-01).
RD1 now takes priority. CS1 follows FR1, coordinating with remaining R2–R8 owner
corrections, and must complete before final B5/human 0.110 closeout and 0.111
blob-storage removal/extraction. The audit traced `34751fc09`; implementation
rechecks current source and retains final qualification. These are planned
simplifications, not implemented cuts or a new patch allocation.

| Slice | Outcome / owner | Included evidence | Focused validation | Status |
| --- | --- | --- | --- | --- |
| CS1-1 | Shared Fleet transitions / Host, CLI, Testing | Exact replan publication, counters, scope-specific receipt validation and offline completion replay | Owning native transition tests, interruption/recovery PocketIC journey and affected-package lint | Pending |
| CS1-2 | Shared chain-key decoding / Core auth/config | Required/invalid input, exact key material, purpose-specific checks and feature selection | Owning native parsing/auth tests and affected-target lint | Pending |
| CS1-3 | Shared nonroot retries / Core lifecycle/workflow | Service ordering, phase/metric identity, scheduling admission, bounded retries and distinct init exhaustion | Native retry/state tests, focused lifecycle/recovery PocketIC proof and affected-target lint | Pending |
| CS1-4 | Shared bootstrap survey loop / Host | Retained samples, exact IDs, reservation before observation, interruption and finite allowances | Owning survey tests, focused observation/recovery proof and affected-target lint | Pending |
| CS1-5 | Shared role overview assembly / facade, Core, Host | Exact role/capability metadata, profile digest, Candid and cfg-pruned generated consumers | Owning native/generated consumer checks, focused endpoint proof and affected-target lint | Pending |
| CS1-6 | Shared Backup staging / Backup | Create-only versus replacement publication, write/sync failures, cleanup and preserved barriers | Owning persistence/crash tests and affected-target lint | Pending |
| CS1-7 | Shared CI installers / tooling, CI | Exact platform/member selection, checksum/version rejection, cleanup and executable identity | Focused installer fixtures, shell syntax and ShellCheck | Pending |

Delivery order: CS1-5, CS1-4, CS1-6, CS1-7, CS1-2, CS1-3, CS1-1. Complete
consumer/docs/changelog propagation with each owning outcome; retain affected
artifact evidence before runtime savings claims. All seven must close. General
B3/B4 pruning stays stopped/deferred; retirement contraction stays with FR1,
fixture-data removal needs its own scope decision, and fresh backup enablement
is outside CS1. Preserve unfinished paid recovery and the human minor gate.

## Toko staging follow-up — accepted future work 2026-10-01

The maintainer requests all recovery-report feedback and import-call reduction
work retained in the [design follow-up](0.110-design.md#toko-staging-import-follow-up--accepted-future-work-2026-10-01).
Toko reports successful eight-source recovery, backend convergence and original
Root restoration. Its repair source and live evidence are unavailable here;
this historical report does not close the remaining R2 queue. The withdrawn
CANIC-188 repair is archived; subsequent replacement is reinstall-only.
RD1 now takes priority over FR1; these follow-ups join the existing 0.110 owner sequence
before final closeout, without a new patch allocation or live/downstream authority.

| Outcome / owner | Required completion evidence | Status |
| --- | --- | --- |
| Import efficiency and bounded authority / R2, Core, Control Plane, Host, CLI | Reject the eight-source 72-call envelope before effects; qualify workflow-derived call/debit budgets; reduce repeated subnet/status observations only with validity and drift proof; preserve exact history, consumed/unknown allowances, interruption recovery, conservation and effect-free replay; measure call/debit/time changes. | Locally qualified. PocketIC measures 105 calls for eight running sources and 106 with placement-drift refusal/recovery; recommended allowance is 224. Native admission, preserved consumption, conservation and effect-free replay pass. Cross-advance placement caching remains conditional. |
| Controller membership comparison / R2, Host | Reordered lists accepted, changed/duplicate members rejected, sealed review and authority hashes preserved. | Both comparisons corrected; eleven owning native tests, Host library/test all-feature warning-denied Clippy and scoped formatting/diff checks pass (`toko-controller-order-*` logs); not yet published |
| Live observation diagnostics / CLI, Host | Diagnose readiness/version/cycles errors; retain per-canister observation/endpoint causes; targeted success/failure and installed-client proof, current help/output/docs. | Pending; role/ID mapping is reported working |
| Historical incident evidence and current artifact identity / downstream, Host | Retain reported source/client/raw/gzip manifests, ceilings and receipt/snapshot limits as historical evidence; qualify current reinstall artifact identity, cycle-safe disposition and uncertain-effect reconciliation. | Historical bundle not supplied here; old-state repair withdrawn, current replacement is reinstall-only |
| Application and dependency provenance / downstream Toko owner | Login, project creation, collections, uploads and tokens against the deployed release; record local Cargo-patch source identity and recovery-client retention; distinguish reserved allowance, observed debit and funding. | Outstanding downstream acceptance; no sibling mutation authorized |
| Unfinished-installation reset / RD1, Host, CLI | Current qualified artifacts and physical inventory before predecessor decoding; partial bootstrap/activation/import and malformed-state reset, changed-controller refusal, uncertain-effect reconciliation, conservation and replay. | CANIC-191 admission corrected and locally qualified; live Toko adoption remains downstream-owned; uncertain provisioning contraction remains |
| Exhausted attempts / R2, Host, CLI | Concrete bounded continuation or cycle-safe reset after spent survey/submission/inspection allowances; no counter reset, duplicate paid effect or repair-first requirement. | CANIC-190 locally qualified: exact-digest, effect-free grants of two attempts per exhausted Host resource, including retired envelopes. Native exact-owner/replay/partial-publication proofs and exhausted signed-handoff PocketIC recovery pass. Root spending caps remain unchanged. |

These items are future implementation/qualification and evidence work, not claims
that this checkout contains the Toko repair or that a smaller call count is proven.
Use native owner checks and focused PocketIC proofs for Canic behavior; downstream
application acceptance remains separate from Canic release gates.

## Accepted code-review corrections — 2026-09-30

Finding disposition is owned by the
[GitHub review catalogue](https://github.com/dragginzgame/canic/issues/40) and its
linked issues. This section retains the accepted sequence and qualification
evidence. Record new finding triage and completion decisions in GitHub.

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
| R2 | Recoverable import and Host operation selection / Host, CLI, Control Plane | Failed prechecks, submitted/unknown outcomes, finite budgets, delayed reviews and a new operation after completed reset; targeted Host tests and exact import/reset PocketIC journeys. | Prechecks, inventory, review-clock recovery and post-reset operation selection qualified; certified request retirement passes native and expired-ingress PocketIC evidence; CANIC-190 Host attempt recovery and import-call reduction qualified; Root-cap disposition, older unknown outcomes and remaining R2 work pending |
| R3 | Receipt-backed funding and conservation / Host, Core, Control Plane | Signing admission, grant reconciliation, credits, bounded debits, funding fences and exact replay; owning native tests and paid-effect PocketIC journeys. | Signing admission/proof reuse pass native tests; grant replay during rotation passes native and PocketIC evidence; remaining accounting pending |
| R4 | Safe placement and recycling / Core, Control Plane | Concurrent resumers, repeated claims, grant/replay residue and snapshot cleanup; owning native tests and actual lifecycle PocketIC cases. | Placement ownership, capacity, repeated recycling and reset recovery pass native/PocketIC evidence. Allocation-bound routes, counts, delayed replies, recycle targets and completed-removal replay pass 78 Core and 37 Root tests, lint and the owning PocketIC journey. Caller replay, issuer and funding authority isolation remains open |
| R5 | Reliable existing backup/restore / Backup, CLI, Core | Prune locks, complete uploads, consistent capture, release/target authority and restored admission; runner crash tests and affected PocketIC paths. | Manifest/path recovery and creation locking qualified. Prune/restore lifetime coordination, verified retention and partial-deletion reporting pass 125 Backup and 108 CLI tests plus affected-package lint. Complete uploads and remaining authority/capture work pending |
| R6 | Recoverable background drivers / Core, Control Plane | Typed retry decisions, one owned driver, backoff, traps and same-release restoration; timer policy tests and actual PocketIC recovery. | Platform-unavailable retry classification passes native tests; driver ownership/trap recovery pending |
| R7 | Convergence across unrelated Fleet changes / Control Plane, Host | Operation-specific authority, mirror acknowledgements, rotation and activation fences; multi-root publication/retry PocketIC cases. | Pending |
| R8 | Complete retirement/reset accounting / Host, Control Plane | Physical inventory, cycle/ICP evacuation, reserved cycles and reviewed residuals; terminal conservation and effect-free replay. | Pending |
| Qualification | macOS and execution boundaries / Host, CLI, Testing | Native macOS build/filesystem evidence, test selection, message/record admission and artifact identity; focused owner checks. | Native macOS CI configured; linked-directory Host/Backup/restore checks pass on Linux; native results and execution-boundary work pending |

R4 traceability: the placement ownership, admission, quota and point-lookup work
addresses [core-placement-fleet-1](https://github.com/dragginzgame/canic/issues/76) through [core-placement-fleet-5](https://github.com/dragginzgame/canic/issues/376); delayed receipt
completion addresses [core-placement-fleet-9](https://github.com/dragginzgame/canic/issues/80). Allocation-bound routing and
Directory fencing address [r2-recycled-principal-authority-4](https://github.com/dragginzgame/canic/issues/429), while interrupted
reset handling addresses [r2-recycled-principal-authority-6](https://github.com/dragginzgame/canic/issues/430). The issuer, replay
and funding findings [r2-recycled-principal-authority-1](https://github.com/dragginzgame/canic/issues/426) through
[r2-recycled-principal-authority-3](https://github.com/dragginzgame/canic/issues/428) remain open. The terminal-removal replay
regression additionally exposed permanent-absence checks in storage and workflow;
their correction passes direct-command and RPC replay with the replacement unchanged.

The review's original IDs remain the traceability keys. Recheck findings against
current source before changing behavior; prior reproduction does not establish
that the current source still has the defect. Low/informational findings require
triage, and performance claims require measurement. Final readiness requires the
complete accepted outcome and its direct rejection/recovery evidence, not only
the most recent slice.

R5 filesystem traceability: manifest publication recovery addresses
[backup-persistence-4](https://github.com/dragginzgame/canic/issues/310); selected-root resolution addresses
[backup-persistence-3](https://github.com/dragginzgame/canic/issues/309) and [backup-persistence-5](https://github.com/dragginzgame/canic/issues/311); buffered JSON reads address
[backup-persistence-10](https://github.com/dragginzgame/canic/issues/316); locking CLI layout creation addresses
[backup-persistence-8](https://github.com/dragginzgame/canic/issues/314). The next continuation closes [backup-persistence-1](https://github.com/dragginzgame/canic/issues/307) using
a parent-side layout lock and durable restore references, including external
journals. Owner death, surviving command descendants, paused/failed work and
terminal replay retain/release the exact source authority. [backup-persistence-9](https://github.com/dragginzgame/canic/issues/315)
is addressed by verified retention, locked retained copies and per-entry deletion
outcomes. Qualification passes 125 owning Backup tests and 108 CLI tests; the
disposable-live-environment restore test remains intentionally ignored. These
corrections do not close upload completion, consistent capture or restore
authority findings, and do not enable fresh live backup execution.

The [GitHub review catalogue](https://github.com/dragginzgame/canic/issues/40) owns
the original-ID qualification set and duplicate mapping. Its linked issues record
individual disposition; partial corrections and unrun native macOS qualification
remain separate from completed targeted evidence.

The simple corrections accept additive fields in ICP-owned balance, snapshot
inventory and known visibility output, and compare valid backup artifact hex
digests independent of letter case, including restore-preview projections.
Missing/malformed consumed fields, unknown visibility variants and different
artifact bytes still reject. Focused native evidence lives under
`target/review-validation/simple-review-*`; the wider ICP selection encountered
two sandbox-denied HTTP listener tests, which are outside these parsing changes.
All 66 selected tests and affected-package library/test lint pass; Host lint uses
`--no-deps` to exclude a separate in-progress Core refill panic-doc warning.

The low export/reset corrections reject duplicate shell variables before output
and discard only the exact simulator directory without applying load traversal
bounds to its contents. Three filesystem regressions qualify deep-tree reset,
interior-link target preservation, terminal replay and instance-root link refusal
with recovery. Seven CLI export tests qualify deterministic collision refusal and
maintained normal exports. Evidence: `target/review-validation/low-review-*.log`.
Host/CLI all-feature library/test Clippy passes with `--no-deps` and warnings
denied; scoped formatting and whitespace checks pass.

Individual finding disposition continues in the linked GitHub issues.

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
