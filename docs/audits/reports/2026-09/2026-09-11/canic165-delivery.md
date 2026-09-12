# CANIC-165: retained delivery, failures and grant lifecycle

Date: 2026-09-11; updated 2026-09-12. This retains focused FP2 implementation
and qualification evidence, not a minor closeout or broad release-validation
verdict. Package versions remain 0.110.14. The completed Canic feature now joins
the existing .15 draft. The [latest generated apply checkpoint](#complete-generated-fleet-apply-and-funding-2026-09-12)
completes the earlier runtime, funding and recovery evidence below.

## Qualified behavior

The existing initial-Shard Fleet journey now continues through a later
application allocation. The disposable Shard importer runs automatically;
only the initial Hub retains the test-controlled hold used to prove that a
Prepared Root can provision its initial child before parent data readiness.
No production readiness override or manual delivery API was added.

After the initial Fleet reaches terminal membership, the test fills the first
Shard to its configured capacity, removes the Store publication controller and
stops Store. A new account cannot obtain a ready assignment during the outage.
After restoring Store, retries of that same account produce one additional
Shard. Root issues its exact revision-1 grant from the retained release; the
application imports and validates the retained bytes without an operator release.
The receipt matches the grant, target, release, content and completion summary.
Account replay returns the same Shard and preserves the complete pool observation.
This extends the existing case rather than adding another Fleet setup to release
validation. Capacity comes from the qualification config, not a new product cap.

The automatic consumer previously classified argument encoding and response
decoding errors as retryable transport failures. It now returns `Codec` and
persists the originating diagnostic code as a permanent import failure.
Transport outages and pending sources remain retryable. No new failure journal,
application cursor, instruction limit or protocol generation was introduced.

A deliberately incompatible PocketIC peer returns a Candid byte where the Store
result is required. The automatic consumer records `CODEC_INVALID`, commits no
rows or chunk progress and keeps application calls blocked. After a fresh-heap
target restart, the same failure remains and the source read count is still one.
The real Store outage, callback rollback and completed-receipt recovery cases
continue to pass.

## Verification

The retained [commands and results](canic165-delivery-evidence/commands.json)
identify exact targeted commands, timings and evidence hashes. All 17 native fixture
and recovery regressions pass, including the unchanged 751-byte maximum stable
recovery record. Scoped Clippy covers changed production and test owners with
warnings denied. The five Store/IcyDB cases and the exact Fleet case pass against
the same 1,633 unchanged source inputs and inventory. The consumer cases take
130.45s (202s runner); the final Fleet case takes 467.42s (497s runner), including
uncached artifact builds. No broad gate ran.

Source snapshots are members of `source-snapshots.tar.gz`; the command record
identifies the archive, each run's member and the member hashes. Consolidation
preserves the earlier snapshot bytes and qualification boundaries.

The neutral eight-row probe measured 21,643,079 validation instructions. This is
measurement evidence only; it does not establish a production fixture ceiling.

## Remaining boundary

The earlier controlled Store outage occurs before later allocation completes.
The new receipt-loss proof below reconciles actual Store grant/revocation effects
after the test caller discards their replies. It does not interrupt Root inside
its Store await. That boundary and reinstall with an actual Store fetch response
still suspended remain unqualified.
Reviewed retry funding remains. Direct fixture-bearing Root retirement and the
existing grouped-removal rejection are qualified below. The bounded collector
and explicit GC intents described below replace
the earlier conservative GC block. Then fixture-bearing generation and complete
generated apply must be enabled and qualified together.

CANIC-165 and the combined worktree are not push-ready. Changelog and active
design/status documentation describe this progress without allocating a version.
No sibling repository, package version, Git publication or live Fleet changed
through this work. Concurrent lockfile updates were preserved and requalified.

## Revocation before recycling

Root now revokes the allocation's exact Store grant before the pool owner can
uninstall its code. The existing pending recycling claim supplies the component
and installation fence; Root rechecks that claim and its Store/release authority
after Store calls. A source-call failure leaves recycling pending. An already
revoked matching grant reconciles a lost response without another mutation.
Reset work checks the retained claim after controller updates and before recording
its result, so a delayed callback cannot overwrite a completed or replaced claim.
No stable-state schema or parallel reset journal was added.

The extended Fleet case removes the later Shard through the public subtree
removal command. Store outage at admission must return `PLATFORM_UNAVAILABLE`
with the target's pool entry and installed code unchanged. After Store returns, removal must
reach Completed, leave the physical canister Ready with no module, and retain
the disabled exact grant at the next revision. Terminal removal replay preserves
that pool entry and grant; replaying the old grant request returns
`FixtureStoreError::Conflict`. Unrelated background pool preparation remains
allowed; the test does not freeze the whole pool while maintenance runs.

This outage is at removal admission, not inside the revocation call. The existing
removal owner checks live Store authority before accepting the operation. Exact
interruption during revocation remains unqualified; the native reconciliation
check alone does not substitute for that runtime proof.

All 16 focused control-plane grant/pool tests and affected-target warning-denied
Clippy pass. The final extended Fleet case passes in 52.34s (67s runner with
cached artifacts) against 1,634 unchanged source inputs and inventory. Its
snapshot is `revocation-source.sha256`; the earlier consumer and delivery runs
retain their original snapshot in the command record. The native check preceded
only the final baseline assertion cleanup; its production and native test inputs
are unchanged. This checkpoint adds no full-suite or production-load claim.

## Retained replacement selection

Parent and child install effects now retain the selected fixture-grant revision.
Their plan/effect comparisons, renewal and charged-size calculations include it.
The current nullable field is required on deserialization; an absent field is not
silently defaulted. This is a pre-1.0 reinstall-only stable-schema change.

A fresh allocation may select the successor of a disabled grant only for the
same Root, release and physical target, with a different installation operation.
Once intent exists, retries reconstruct that exact selection rather than choosing
again from Store. Issuance checks the active workload claim across Store calls;
retired or replaced claims cannot issue access. The target accepts a positive
selected revision and Root checks it against the retained installation payload.

The Fleet journey continues after recycling through a controller-only disposable
Hub endpoint that calls the public application RPC with a retained operation ID.
It requires reuse of the physical Shard, a different installation, the successor
grant after revocation, and a fresh matching completed import. Exact RPC replay
and a delayed previous-installation revocation must preserve the replacement grant.
This adds neither a production provisioning endpoint nor a new progress journal.

The reuse investigation also found that the target reached Installed without
progressing to runtime verification. Recycling stops the physical canister;
installation did not contain a start step. The existing Installed owner now
observes the selected module, Root controller and workload claim, starts a
non-running target, then rechecks the claim. An already-running observation skips
that effect. The regression explicitly stops the recycled target before requesting
its new installation. Startup uses the normal observed management-call boundary
and does not raise a funding allowance.

The extended real Fleet case passes in 436.53s (498s runner including native
compilation and uncached artifact builds). It proves the stopped Shard is reused
with a new installation, an enabled revision-3 grant and its own exact completed
receipt. Repeating the public RPC returns that same Shard, and a stale revision-1
revocation rejects without changing its replacement grant. All 1,634 recorded
source inputs and their inventory stayed unchanged through the final runtime run.
The snapshot member is `replacement-source.sha256`; previous checkpoints retain
their original members and scope. This adds no production ceiling or complete
release-time claim.

All 39 focused control-plane and four core native regressions pass, including
retained parent/child revisions through renewal and restore, missing-field
rejection, charged-size accounting, stale workload claims and positive target
assignment revisions. Scoped core/control-plane/internal-testing/Hub Clippy
passes with warnings denied. The final control-plane rerun confirms the same
unchanged source snapshot. Formatting, local document links, diff whitespace,
document semantics and retained evidence hashes pass.

## Partial-import reinstall and discarded Store receipts (2026-09-12)

The automatic importer now has a disposable application fault that pauses after
one committed chunk. The test observes the exact old installation, one stored row
and no completion receipt. Actual same-Principal reinstall with a new protected
installation clears the checkpoint and row. This is same-release operational
replacement; it adds no cross-release compatibility path.

While the replacement remains Prepared, the test submits revocation and issuance
to the actual Store as its Root, drains transport and discards both command
receipts without decoding them. Store queries and exact command replay reconcile
the disabled revision 2 and enabled revision 3. Same-Wasm Store restart retains
the replacement grant. Old issuance, revocation and reads return typed refusal;
they cannot alter the new grant or complete the new import.

The new runtime begins with an empty application while Store is unavailable.
When Store returns, the registered consumer completes automatically and produces
its own exact receipt. Same-release target restart preserves that receipt. The
test owns only fault selection and ordinary canister calls; Canic owns scheduling
and the application retains the sole rows/checkpoint/receipt transaction.

All five Store/IcyDB integration cases pass in 109.13s (155s runner including
compilation and artifact builds); affected probe, internal-testing and integration
Clippy passes with warnings denied. All 1,635 final source inputs and inventory
stayed unchanged. The snapshot member is `pending-reinstall-source.sha256`.
After an earlier pass, an external lockfile update required a focused rerun.
Its first compile exhausted disk space; reclaiming 49.6 GiB of stale Canic
incremental caches allowed the same check to finish. No source or retained
release/evidence artifact was removed.

This qualifies partial durable-import interruption and discarded replies at the
Store test caller. It does not claim Root's suspended-await interruption or an
actual Store fetch reply held across reinstall. No production fault hook,
fixture-generation enablement, funding increase or broad gate was added.


## Durable outage backoff (2026-09-12)

Source-outage retry pressure now survives a same-release heap restart. The
existing async-job recovery record retains a failure streak and earliest retry
time; its exact maximum encoding is now 810 bytes. The shared completion owner
updates those fields and releases the current lease atomically. Claim admission
rejects early work, successful progress resets the delay, and late completions
cannot alter the current attempt's retry pressure. Other recovery owners retain
their existing behavior. No new allocation, journal or application cursor exists.

The focused native restore proof covers the complete existing delay sequence,
one admitted retry at its deadline, stale callbacks, independent recovery owners,
and saturation of both the failure count and clock arithmetic. It restores the
exact stable record between attempts; this is native persistence qualification,
not an assertion that PocketIC intercepted a Store response. The existing
Store/IcyDB journey supplies the separate runtime recovery regression.

The existing one-to-60-second delay policy bounds retry frequency. It is not a
lifetime spend allowance and does not close reviewed funding acceptance. The
remaining interrupted-effect, reference-retirement and generated-apply boundaries
above remain open. IcyDB is pinned to published 0.257.5 for this checkpoint;
earlier source snapshots retain their original dependency versions.


Thirteen focused core recovery tests pass. Core/control-plane library and test
Clippy passes with warnings denied. All five Store/IcyDB runtime tests pass in
126.15s (200s runner including compilation and Wasm builds). All 1,635 final
source hashes and their inventory remained unchanged. Native recovery tests
preceded only explicit default-constructor/import lint cleanup; their exact
snapshot is retained separately from final Clippy and runtime inputs.

All 18 focused memory/timer contract tests also pass on the final source.


## Explicit Store retirement and bounded fixture clearing (2026-09-12)

The existing GC command previously inferred its target from the current phase:
an initial call prepared Store, while repeating the same call could start
collection. It now carries an explicit `Prepared` or `Complete` target through
the Root client, Store endpoint, passive DTO and canonical Candid. This changes
the maintained command through a hard cut; no decoder fallback or second GC
operation exists. Preparation retry retains its exact phase and bytes, including
when the retained operation is already clearing or complete. Collection requires
that same prepared operation; missing and conflicting owners cannot authorize it.

Root's final-inventory operation now revalidates its exact terminal intent and
absence of workload assets/pending lifecycle effects around Store preparation.
Those existing owners close active-release and issued-import references. Initial
import completion alone never closes them. The Prepared Store fences fixture
reads and writes, retaining bytes until Root explicitly requests collection after
logical removal. Each collection pass deletes one admitted fixture entry and its
exact byte charge. Same-release restart resumes the retained Clearing phase;
empty fixture state then permits existing template cleanup and cycle reclamation.
Pending clearing reports unavailable to the existing Root retry owner.

Native checks cover exact operation/phase replay, terminal inventory admission,
source-read fencing, the 1 MiB chunk, byte-ledger conservation and restoration
between bounded cleanup passes. The actual Store journey rejects premature
collection and a publication-controller request, replays preparation before and
after restart with all bytes retained, restarts during nonempty Clearing, resumes
to zero bytes and verifies terminal/conflicting-operation replay. All five
Store/IcyDB tests pass in 76.18s (137s runner including compilation and Wasm builds).
This Store test submits retirement commands as Root; the full Root driver is
qualified separately and this alone does not establish a complete fixture-bearing
Fleet retirement or reviewed funding allowance.


All 20 final native checks and 15 timer guards pass, as does scoped production
and integration Clippy. The final existing full Root-retirement case passes in
122.35s (158s runner), including cycle reclamation and external deletion readiness.
Its baseline has no fixture imports. Root now explicitly acknowledges the exact
preparation operation even when the observed phase is already Prepared; observing
the phase alone is insufficient. The final Root snapshot includes that refinement;
the earlier Store snapshot retains its own unchanged runtime inputs. All 1,637
recorded inputs and inventory stayed unchanged within those runs. The canonical
Store Candid and current command change together without a compatibility reader.

The remaining acceptance is a full fixture-bearing Root retirement, reviewed
funding, the two previously identified exact suspended-effect cases and generated
Fleet apply. The combined batch remains unassigned and not push-ready.

## Fixture-bearing Root retirement and settled accounting (2026-09-12)

The accepted retirement qualification exposed two production defects that the
separate Store and no-descendant Root journeys did not cover. After deleting the
last child membership, the driver tested the descendant count and attempted
final inventory before the subtree's directory/terminal journal steps finished.
It now follows the existing durable `DescendantsEmpty` result and its exact
Registry head. The same retained operation finishes without another lifecycle
owner, cursor, memory allocation or journal.

The next boundary falsely treated an installation reservation as retained child
history. Child commit already settles that precharge into actual record/index
sizes; retirement must count the remaining child records after live indexes are
removed. Temporary runtime tracing measured 13,603 calculated bytes against the
correct 12,660-byte ledger, a 943-byte overcount. Final inventory now uses the
same settled-record accounting, retaining the exact equality guard. All temporary
production tracing is removed. The final test retains read-only Store diagnostics
for future failures.

The complete disposable direct Component journey installs a Hub and initial Shard,
imports the release-selected fixture, validates the actual application receipt
and source retention, and then retires through Root and Coordinator. It verifies
all imported pool assets remain controlled with application code removed, Store
is deleted only after reclamation, Root deletion is explicitly reviewed, lost
Ledger transfer replies reconcile, and terminal replay retains the exact result
and two total transfers. The shared existing retirement assertions now inspect
every pool asset rather than only two selected Components.

An initial attempt to retire the grouped initial/later-Shard Fleet correctly
returned `STATE_CONFLICT`: Coordinator operation, placement and service references
fence standalone Root removal. That guard remains intact. The final grouped
regression proves rejection with unchanged Registry and retained fixture sources;
a separate direct Component case qualifies supported retirement. This does not
claim grouped application retirement, reviewed funding or generated apply.

Final focused qualification: all 21 Component Registry native tests pass in
0.38s. Control-plane and internal-testing library/test Clippy passes with
warnings denied and the governed PocketIC test feature enabled. The fixture-bearing
retirement passes in 192.55s (237s runner); grouped import/reuse and
retirement rejection pass in 53.08s (55s runner); existing no-fixture
Root retirement passes in 25.20s (27s runner). These are focused
test timings, not deployment-speed measurements.

The existing evidence bundle preserves the original failures and the temporary
trace snapshot separately from the final source. All 1,637 final source inputs
and inventory remained unchanged during qualification. Changelog, feature,
design and status surfaces are updated without allocating or bumping a version.
Reviewed funding, the two exact suspended-effect cases and fixture-bearing
generated apply remain. The complete batch is not push-ready; no broad validation,
Git publication, live deployment or sibling mutation ran.

## Funding admission during initial fixture readiness (2026-09-12)

A real registered initial Hub requested cycles while its fixture kept Component
membership Prepared. The original Root capability resolver returned typed
`AUTHORITY_UNAUTHORIZED` before the request could reach the configured funding
owner. The exact Prepared Root now admits cycles requests alongside the existing
initial-child allocation capability. The redundant second capability restriction
is removed; the lifecycle match still rejects all other Prepared capabilities.
Core funding authority, structural parent checks, role limits, reserve, cost guard,
transfer execution and durable receipts are unchanged.

The final initial/later-Shard test derives funding limits from its compiled App
configuration, requests more than its total allowance, and checks each transfer
is clamped to the per-request and remaining-child limits. It verifies exact
response replay during cooldown pays once, a distinct request during cooldown
transfers nothing, and total allowance exhaustion returns the typed rejection.
Unregistered callers and Prepared recycling reject before any transfer. These
are real Root/management effects; the recipient is stopped during exact balance
assertions to exclude its timer execution. An initial test of the default allowance
ran across a storage-rent charge and correctly exposed that stopped canisters
still incur rent. The final disposable policy uses a 2T request cap, 3T total and
one-second cooldown to exercise both a full and partial grant with minimal steps;
production defaults are unchanged. Balances are checked per effect at fixed time.
The test restarts it, confirms fixture readiness and placement remain pending, then completes the same import and full
initial/later-Shard/recycling journey. It does not claim an automatic caller-timer
proof or a suspended Root funding callback.

The focused PocketIC case passes in 85.57s (100s runner).
Both Component RPC native tests pass; scoped control-plane/internal-testing
library/test Clippy passes with warnings denied. The existing evidence bundle
retains the initial rejection and separate source snapshot plus final logs and
snapshot; all 1,637 source inputs and inventory remain unchanged. These timings
include the local qualification workflow and are not deployment benchmarks.

Reviewed generated publication/retry cost accounting and automatic target-funding
integration remain open, alongside the two exact interrupted-effect cases and
generated apply. The batch remains Unreleased and not push-ready. No broad gate,
version change, Git publication, live staging effect or sibling mutation ran.

## Automatic funding while import remains pending (2026-09-12)

The new disposable configuration keeps the initial Hub running while its
application importer holds completion. The target's actual funding timer requests
2T, receives 2T and then a final 1T, and reaches the configured 3T child allowance.
All successful transfers originate from that timer. The protected target top-up
history reports exhaustion; an independent ordinary request confirms the typed
`ChildBudgetExhausted` result. Advancing five additional one-minute intervals
leaves the history unchanged: the exhausted timer stops issuing requests.
Root remains Prepared and the fixture remains NotReady. Releasing the application
hold completes the same Hub import with selected targets and Store sources retained.
This case qualifies target import readiness, not complete generated Fleet apply.

The fixture uses a 200T demand threshold solely to trigger requests on an already
funded disposable target without burning its balance. It preserves the 5T pool
admission floor and uses a 2T request cap, 3T total and one-second cooldown to
exercise both full and partial grants. Production funding defaults are unchanged.
The shared installer now accepts the selected Root/config/component artifacts;
its prior caller retains the same initial/later-Shard configuration. A helper
extracts the initial-Hub wait and typed failure diagnostics without suppressing
Clippy's function-size check.

Final source-bound qualification: the automatic-funding case passes in 42.04s
(59s runner), and the existing initial/later-Shard funding and recovery case
passes in 55.25s (57s runner). Internal-testing library/test Clippy passes with
warnings denied and governed PocketIC tests enabled. The existing evidence
bundle appends both runtime logs and lint results and retains the final
`automatic-funding-source.sha256` archive member. All 1,638 source inputs and
inventory remain unchanged. These are focused qualification timings, not
measurements of release or deployment speed.

Read-only host review identifies the next remaining funding correction:
`bind_action` retains the reviewed execution-burn allowance for each fixture
preparation/upload, but failed calls may remain at Intent without a persisted
paid-attempt count. `maximum_stalled_observations` and terminal burn comparison
are not evidence of intent-before-effect retry accounting. Pool maintenance's
existing durable attempt path provides the relevant local precedent. Publication
retry qualification remains open and must preserve exact review authority and
lost-reply reconciliation through the existing journal owner.

The automatic target-timer acceptance is now covered. Reviewed publication/retry
funding, the two exact suspended-effect cases and generated apply remain. The
changelog and active contract/design/status are updated; CANIC-165 remains
Unreleased and the complete batch is not push-ready. No broad gate, version
change, Git publication, live staging effect or sibling mutation ran.

## Durable reviewed publication attempts (2026-09-12)

Fixture preparation/upload previously retained an Intent after a failed call but
had no paid-attempt counter. Repeated resumes could issue further calls despite
having reserved only one update's burn. A stalled-observation counter and final
conservation comparison did not establish intent-before-effect retry accounting.

The current Store-action binder now copies the configured base retry bound into
`maximum_attempts`, reserves its checked product with per-update burn, and includes
repeated observations in the reviewed conservation reservation. Exact action and
plan hashes bind this allowance. A required `publication_attempts` field in the
existing journal consumes authority before the external call. Local persistence
failure prevents issue; uncertain persistence conservatively retains consumption.
Failed or lost replies and fresh process reconstruction do not reset the counter.
Source reconciliation remains before the limit check: final-attempt success with
a lost reply can complete without another upload. Otherwise exhaustion returns
`FixturePublicationBound`; an impossible retained counter rejects as journal
integrity failure before an effect. No compatibility default or second journal is
introduced. Production funding defaults remain unchanged and the full possible
retry cost is visible in the reviewed plan.

The native tests use the existing deterministic platform to isolate host
persistence. The platform reads the journal inside each publication call and
compares the exact effect record passed to it. Fresh platform reconstruction
retains only observed external state and disk records; preparation and a later
chunk both exhaust their two-attempt allowance without another call. A separate
one-attempt-per-action journey loses each successful reply and still completes,
then replays with zero further publication calls. Other checks cover budget/hash
binding, multiplication overflow, zero attempts and a counter above its limit.
Existing plan-content, protocol compilation, maintenance and interruption tests
also pass: 24 distinct checks, 25 executions across the six selected filters.

Host and internal-testing library/test Clippy pass with warnings denied. The real
Root/Store publication and immediate zero-effect replay case passes in 48.63s
(93s runner). It uses actual retained source bytes, exact prefix observation
and lost upload replies. This complements the host journal tests; it is not a
complete generated fixture Fleet apply or proof of either suspended Root/Store
callback boundary. The existing evidence bundle appends native, lint and runtime
logs and `publication-attempts-source.sha256`; all 1,638 source inputs and inventory
stay unchanged. These are focused test timings, not release-speed measurements.

Changelog, operator documentation, feature contract, design and status are updated.
The local publication retry accounting gap is closed. The exact grant/revoke
interruption, Store-fetch/reinstall interruption and complete generated Fleet
apply/funding remain open. CANIC-165 remains Unreleased and the combined batch is
not push-ready. No broad gate, version, Git publication, staging effect or sibling
mutation ran.


## Reinstall submitted during a pending real Store fetch (2026-09-12)

The new Store/IcyDB integration case uses the canonical Store and an ordinary
fixture consumer on separate disposable application subnets. An application
pause lets startup settle before the automatic import is released. A query
observes the existing active fetch lease before reinstall is submitted, with
zero rows imported at that point. The query requires the probe's test controller;
its runtime observation is read-only and available only with the existing
`internal-test-fixtures` feature. No Store behavior or transport is replaced.

The replacement reports NotBegun immediately after reinstall and after ten more
rounds with fifty seconds of simulated time. Its rows remain empty, application
admission is fenced and no fetch lease survives. The test revokes the old grant,
authorizes the new installation and starts that replacement. It completes with
its exact new receipt binding and completion summary. A read using the former
grant returns `FixtureStoreError::Authority`.

This proves an actual fetch is pending at reinstall submission. It does not
establish that Store's reply arrives after reinstall executes: the fixture does
not deterministically hold that reply. The evidence keeps
`actual_store_fetch_reply_held_across_reinstall` false and separately records
`actual_store_fetch_pending_at_reinstall_submission` as true. Root interruption
inside an actual Store grant/revoke await remains unqualified. Neither this
cross-subnet fixture nor the existing test-source held-reply proof establishes
those missing canonical-effect boundaries or the generated Fleet's topology.

All six cases in `icydb_lifecycle_composition` pass in 131.54s (163s runner).
Scoped core, lifecycle probe, internal-testing and integration Clippy passes with
warnings denied. The existing evidence bundle appends runtime and lint results
and `pending-real-fetch-reinstall-source.sha256`; all 1,639 source inputs and
inventory remain unchanged. These timings describe focused qualification, not
deployment performance.

The current feature contract, design, status and Unreleased changelog are updated.
The exact held-reply cases and complete generated Fleet apply/funding remain
open. CANIC-165 is still unassigned and the complete batch is not push-ready.
No broad gate, version, Git publication, staging effect or sibling mutation ran.


## Held Store replies and Root worker restoration (2026-09-12)

A controller-guarded barrier compiled only under `internal-test-fixtures` holds
successful canonical Store grant, revocation and chunk responses after their
real authorization and storage work. The host-generated Store package supplies
the test Wasm; its exact bytes are bound at initial Root installation. Ordinary
Store builds emit no barrier endpoints. This instrumented artifact is neither
production-finalized nor byte-identical to shipping Store. It adds no production
transport, stable record, scheduler or tuning limit. Bounded test rounds use real
management consensus calls; the Store barrier's own reply survives the simulated
caller-timeout advance.

The Root case begins stop while a real Store reply remains held, lets Root's
bounded call drain, then restarts the same Root Wasm. Store remains held through
restart. Release and recovery preserve the exact grant revision and target set;
the Fleet reaches terminal activation. The subsequent revocation case observes
the grant disabled before target reset, repeats the same interruption boundary,
then finishes removal and proves exact effect-free replay.

This exposed a real defect: Root's aggregate provisioning journal survived
restart while Prepared, but its heap worker was not rescheduled. The synchronous
control-plane restore adapter now asks the existing provisioning workflow to
resume its retained active operation. The existing dispatcher retains backoff
and review-required failure handling; completed operations have no active owner.
The correction adds no readiness override or alternate lifecycle.

The initial live-upgrade setup was replaced with stop/drain before heap
replacement, following the [IC management contract](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/).
No claim is made that Rust can interpret outstanding callbacks after its heap
has been replaced. The final consumer case separately holds a real Store reply
past reinstall and proves empty replacement state, fenced admission, exact new
receipt completion and old-grant rejection.

The Root case passes in 98.26s (113s runner); seven Store/IcyDB cases pass in
78.07s (88s runner). Twenty-three provisioning tests and scoped Clippy
pass. The existing [command record](canic165-delivery-evidence/commands.json)
retains source snapshots and failures as well as passing runs. The final consumer
check follows a test-helper extraction after the Root proof; Root/runtime inputs
are unchanged. Complete generated Fleet apply and reviewed funding remain FP2
work. Fixture-bearing generation stays disabled and the batch stays Unreleased.


## Generated release cache authority correction (2026-09-12)

The generated Fleet test recipe omitted `fixture-artifact-manifest.json` from
its cached output inventory, even though the complete release manifest binds its
digest. Cache reuse could therefore restore a selected release without the child
authority required by generation. The cache now retains that manifest alongside
the existing release plan and artifact manifests.

A native regression commits a real release plan and empty fixture manifest,
removes the release directory, restores the cache and loads the exact original
fixture authority by release, topology and digest. Other output files in this
native proof are inert; it does not qualify a finalized fixture-bearing release.
The existing distinct-release cache regression also passes: two tests in 10.94s.
Scoped internal-testing library/test Clippy passes with warnings denied.

Logs and a source snapshot are retained in the existing delivery evidence.
Only `baseline.rs` changed after the held-reply runtime qualification; its later
change adds the cache output and native regression. No runtime source changed.
Fixture-bearing cached payloads, generated apply and reviewed funding remain
open; this correction does not enable fixture-bearing generation.


## Complete generated Fleet apply and funding (2026-09-12)

The existing mixed-topology production-adapter journey now supplies a neutral
Shard fixture through the host manifest compiler before sealing. It builds the
ordinary generated Coordinator, Root, Store and configured application packages,
generates desired Fleet state and applies the real reviewed host plan against
a disposable PocketIC Fleet. Store carries no internal response-barrier feature.

The generated cache retains the selected fixture manifest and content chunks,
and fingerprints authored selection/source inputs separately from its outputs.
Native qualification removes the release directory, restores exact authority and
payloads, verifies retained bytes without authored source files, and proves that
changed source bytes invalidate the cache. Distinct release cache identities
remain separate. The final journey reuses 30 sealed artifacts: total artifact
resolution is 7.73s, including a 1.69s cache restore. Its separately sealed second
build takes 57.15s to resolve. These are local observations, not a complete
release-time benchmark or cross-release reuse of finalized Wasms.

A production gap was found in fresh continuation: its finite successor count
and funding preview omitted fixture preparation/chunks and their paid retries.
Review now counts one preparation and every chunk for each distinct content
object in the per-Root upper bound. Required
`fixture_publication_retry_attempts` separately binds additional permitted calls
into the current v1 review record and plan digest. The preview includes their
configured update/observation costs. Funding defaults and exact release binding
are unchanged; source verification precedes admission. The provisional
fixture-unavailable rejection is removed from generation and initialization.

The final runtime proof loses actual controller, reset and fixture upload replies.
A fresh adapter resumes the same reviewed operation, observes the committed
upload and does not publish twice. Before terminal replay, assertions verify
retained source bytes, exact target/release/content receipt, reviewed
publication attempt bounds and the original retry reserve. The complete Fleet
converges, its exact and newly planned replays apply no effects, and cycle
conservation holds. Changed-build and identical-build deliberate reinstalls also
converge and replay; the existing public allocation qualification passes.

The first attempt rejected overlapping cache input/output paths before building.
The second reached terminal deployment, but an outdated mirrored Candid type
prevented the intended reset response loss. Its test fragment now includes the
current fixture command. The final run exercises every intended fault. Both
failed runs remain in the same evidence bundle.

The final case passes in **824.01s** (838s target; 839s runner). Nineteen focused
host tests and two cache regressions pass; the corrected cache-input regression
passes separately in 0.10s. Scoped host/internal-testing library/test Clippy
passes with warnings denied. All 1,641 recorded source inputs remain unchanged
during the final runtime run. Each run names its own source snapshot; the final
source is based on pre-existing development commit `9e1becc58`, with package
versions still 0.110.14. No Git, version or publication action was performed.

This completes Canic FP2 and its .15 draft propagation. It does not qualify
Toko's actual Translation/Game Shard conversion, matched payload/import-cost
measurements, production fixture ceilings or live staging recovery. Those remain
downstream work. Earlier checkpoint statements about disabled generation or open
FP2 describe their historical evidence scope. No broad gate or minor closeout
audit was run.
