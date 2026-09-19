# Canic 0.110 Implementation Status

## Published-line ordinary CI repair — 2026-09-19

Keep this correction on .110 despite the soft release-count guideline: the
published workflow omits prerequisites required by its maintained host tests.
The bounded .29 repair supplies the Wasm Rust target and checksum-bound
`ic-wasm`, guards each owning job and preserves fixture compiler diagnostics.
All 18 affected/regression tests, workflow/shell lint and the release-integrity
contract pass. The completed chunk-upload work below joins the same open .29
draft; both changelog surfaces are ready for the maintainer-selected release
flow. Packages stay .28 and no publication is implied. The pool and test-setup
work below completes the accepted Canic-only .29 throughput scope.

## CANIC-160 publication and import throughput — 2026-09-19

The maintainer accepts the new Toko effect-count feedback after publishing .27.
Keep this operator-latency work on .110 despite the soft release-count guideline;
no minor closeout or package bump is implied.

| Sequence | Owner | Scope and required evidence | Status |
| --- | --- | --- | --- |
| Store publication consolidation | Control plane and host | Atomic metadata/first-chunk admission through the bounded byte lane, exact retained content, lost-reply recovery and matched Store journey | Published in .28; fewer calls proven, elapsed improvement unproven |
| Requested dependency updates | Host and test/audit fixtures | IcyDB 0.259.6 ownership/admission and HMAC 0.13.0 with host SHA-2 0.11.0; stable tag/hash encoding | Complete; 46 focused tests and scoped fixture/host Clippy pass |
| Independent chunk uploads | Host execution/journal | Bounded issuance after preparation, exact per-chunk authority, drain issued work on failure and reconcile every outstanding result | Complete in the open .29 batch; six native tests, the extended existing Root/Store PocketIC case and scoped warning-denied Clippy pass; whole-deployment savings unmeasured |
| Pool reconciliation batches | Host execution/journal | Existing per-asset checks, funding admission, ordered failures, interruption recovery and conservation; matched production-adapter timing | Complete in .29; 18 import effects take 32.858 seconds serial versus 21.731 seconds concurrent in three phases; both paths pass recovery and exact replay |
| PocketIC setup and Canic verification | Testing | Bounded pending-asset imports, unchanged runtime artifacts, warm reuse and existing lifecycle journeys | Complete in .29; setup saves 0.173 seconds in one pair, all 21 warm outputs reused, two 30-file release sets match between control and candidate |

The complete .29 batch is ready for the maintainer-selected release flow:
369 focused Fleet Ensure native tests, the existing Root/Store and four-Shard
cases, both matched production-adapter recovery paths, scoped warning-denied
host/internal Clippy and formatting pass. Both changelog surfaces are current;
packages stay .28. Acceptance is entirely within Canic, with no consumer-specific
verification or live operation required. No full-gate speedup is claimed.

Store publication now combines manifest, metadata and chunk zero in one update,
within the existing byte envelope. Native rejection/replay/content-reference
checks, scoped Clippy, canonical Candid and real Root/Store lost-response proof
pass. The paired disposable Store-bootstrap journey reduces host updates from
11 to eight at identical bytes; 559 versus 563 ms establishes no elapsed gain.
Its temporary instrumentation is removed, so normal validation does not add a
second Fleet setup. No whole-deployment improvement is claimed. .28 changelog
surfaces include the completed dependency update and are ready; Canic packages
remain .27. No broad gate or publication ran.
[Evidence](../../audits/working/0.110-validation-throughput/report.md).

## Post-.26 validation throughput — 2026-09-19

The maintainer confirms .26 publication and continues Toko-first speed work.
The corrected handoff scan confirms downstream .26 library/CLI adoption and 282
passing native tests. Concurrent-source rejection and CANIC-168's sandbox-cache
diagnostic worked as intended; managed qualification and live timing remain
downstream work. No new Canic defect is established by these reports.
Installed .26 CLI acceptance passes on the exact-tag four-role fixture: unchanged
and session-only repeats take 4.639/4.637 seconds, with four complete cache hits
and all 18 release-file hashes unchanged. Two actual competing builds expose the
correct owner and serialize safely; eleven focused cache/lock regressions pass,
including interruption recovery. CANIC-176's Canic-side released-CLI evidence is
complete; exact Toko workload and live acceptance are still downstream work.
Keep these latency follow-ups on the affected .110 line despite the soft release
count guideline; no minor closeout or new package version is authorized.

| Outcome | Owner | Included evidence | Validation | Status |
| --- | --- | --- | --- | --- |
| Reduce validation preparation while retaining security and recovery coverage | CI/test infrastructure | Deterministic dependency-gate graph/database, real stale-advisory isolation, .26 timing attribution; further fixture-build cost qualification | Gate classification/rejection tests, scoped shell lint and CI contract; focused IC proof for any retained runtime change | Active; preflight slice qualified, larger artifact-build work next |

The gate regression takes 1.869 seconds versus the prior 134-second record;
the production security audit remains unchanged. Both changelog surfaces open
.27 with packages/pins still .26. A native inspection-query experiment passed
the real recovery case but did not improve execution time and was removed.
The complete speed batch remains open; this is not a push recommendation for
one preflight change. [Evidence](../../audits/working/0.110-validation-throughput/report.md).

## Further .26 fixture setup qualification — 2026-09-19

The bounded .26 batch remains ready for the maintainer-selected release gate.
Fresh recovery-fixture infrastructure now installs directly through PocketIC
with the existing production initializer compiler and exact selected Wasm digest.
Each case keeps independent replica/host state; paid creation and recovery
assertions remain. Five focused IC cases, initializer/estate/registry regressions
and scoped warning-denied host/internal Clippy pass. Warm-pair installation falls
24.126 to 11.933 seconds; complete cases fall 165.233 to 154.977 seconds. Server
history differs, so no complete release or Toko deployment gain is established.
The earlier checkpoint prototype changed prepared accounting and was discarded.
Both changelogs are current; packages/pins remain unchanged. No broad gate,
version bump, publication or sibling edit ran. Full RF3 and further terminal-read
work remain separate. [Evidence](../../audits/working/0.110-validation-throughput/report.md).

## Expanded .26 acceptance — 2026-09-19

Ready for the maintainer-selected release gate, superseding the narrower scope
below. All four current Toko items are addressed: safe planning/replay/successor
evidence reuse, inclusive nested attribution, non-authorizing continuation
forecasts and published/installed CLI warm/session/competing-lock qualification.
The exact installed .24 CLI already contains CANIC-176's released diagnostics.
Both real competing builds retain complete cache reuse and immutable outputs.

Both complete reinstall subjects pass unchanged safety assertions. Common-stage
observation calls fall 1,044 to 1,017; retained-estate reinstall takes 277.737
versus 275.936 seconds. This single pair establishes only modest observed
end-to-end improvement. Distinct embedded source paths change Wasm hashes and
isolated cache warmth prevents a whole-runner timing claim. Native regressions,
focused IC recovery and final scoped warning-denied Clippy pass. Both changelog
surfaces are ready; versions/pins remain unchanged and no broad gate ran.
Full RF3 live funding and further terminal-read optimization remain separate.
[Evidence and limitations](../../audits/working/0.110-validation-throughput/report.md).

## Post-.25 speed batch — 2026-09-19

The maintainer confirms .25 publication and selects more speed work before
continuing full RF3. Keep accepted operator/validation latency corrections on
the affected .110 line despite the soft twelve-release guideline; no minor
closeout is implied. The .26 draft starts with per-call release-input sharing
for Root authority and init compilation, retaining fresh validation across calls.
Multi-Root parity, initializer authority, evidence mutation/retry and generated
planning/replay checks pass; scoped host Clippy passes. Packages remain .25.

The measured setup slice isolates the fixture artifact builder from journey
assertions/timing edits. All 17 funding-fixture artifacts remain byte-identical;
the exact recovery journey passes after extraction and after a journey-only edit
that reuses the complete cache without building. Build-helper mutation/reuse,
release identity, fixture restoration and governed catalogue checks pass, as does
scoped internal Clippy. The maintainer extended the same batch with CANIC-160:
per-batch pool authority preparation, identity/cache observation attribution and
typed transient provisioning-status retries. Eighty-five focused native tests
and the exact funded-estate recovery/replay journey pass. Scoped Clippy,
including the final fixture network-wiring correction, and formatting checks pass.
The query transport keeps signature verification and exact operation/plan checks;
no mutation replay or across-effect cache was introduced. Both changelog surfaces
and the operator runbook are current; the complete bounded .26 batch is ready for
the maintainer-selected release gate. The .25 runner baseline remains 2,674
seconds; no controlled whole-release saving is claimed. The expanded acceptance
above adds safe reuse and continuation forecasts; full RF3 remains follow-up
work. Packages stay .25.
[Current evidence](../../audits/working/0.110-validation-throughput/report.md).

## .25 release-test correction — 2026-09-18

The full gate exposed one stale mixed-topology assertion after CANIC-166/172:
an interrupted reset now names its original operation and digest through
`RetainedOperationRecoveryRequired`. The test asserts that exact identity.
The complete targeted mixed-topology case passes, including both wipes,
lost-response recovery, conservation and replay; warning-denied internal-test
Clippy passes. No production behavior changed. The selected .25 batch is ready
to retry its release flow; the full gate was not rerun. Full RF3 remains next.
See the [current handoff](../../status/current.md) for the focused logs.

## Selected .25 release scope — 2026-09-18

The maintainer selected the completed recovery fix, speed improvements and
funding diagnostics for .25, keeping full RF3 as the next accepted batch. The
selected outcome is implemented and qualified; both changelog surfaces are
ready for the governed release flow. Earlier statements that full RF3 is not
push-ready do not block this explicitly bounded release.

Included: CANIC-166/172 retirement admission and original-operation guidance;
bounded Root/Store reads, stable generated build inputs, direct audit-Root
builds and duplicate PocketIC removal; local funding deficits and complete
allocation-binding diagnostics. Existing native, transport, focused PocketIC
and scoped Clippy evidence is retained in the
[release handoff](../../status/current.md#selected-25-release-handoff--2026-09-18).
No full-suite or whole-release speed result is claimed. Package versions remain
.24; no version/publication/live effect ran.

Full RF3 remains open for CANIC-156/174: reviewed-budget descendant collection,
complete live inventory and current-registry qualification, recursive demand,
recovery-quote integration and transport/IC recovery proof. Minimum native Root
recovery stays independent of telemetry. Actual Toko staging recovery follows
publication/adoption under its original authority and is not established by
the local proof. RF3 remains ahead of B1; no minor closeout is implied.

## RF3 funding allocation bindings — 2026-09-18

The .25 draft now preserves complete allocation identity, checks selected Fleet/
Coordinator/Root placement and Spec admission, and validates observed parent
chains against declared role edges and shared Component/release authority.
Missing parents are unknown; duplicates, cycles and conflicting joins reject.
An iterative walk shares completed results without extra observation calls.
25 host funding tests, two CLI tests and generated-estate/replay qualification
pass, as does scoped warning-denied Clippy. Changelogs and the funding runbook are updated.

RF3 is not complete or push-ready: budgeted descendant ledgers, complete live
inventory/current-registry proof, recursive demand, full recovery quotes and IC
evidence remain. Minimum native recovery stays independent of telemetry. No
newer confirmed Toko blocker was found; earlier recovery/speed evidence remains
valid separately. No version/publication/live effects or sibling edits ran.
[Evidence](../../audits/reports/2026-09/2026-09-18/rf3-funding-bindings.md).

## RF3 observed child shortfalls — 2026-09-18

RF3 implementation has started under the maintainer's continuation. The existing
.25 draft now includes local child deficit diagnostics derived from observed
balances and settled policy-bound ledger charges. Unknown/pending evidence is
never zero demand. Runtime policy owns allowance/cooldown limits; generation
adds no update calls or payment authority. The 21 focused host tests, native
generation/replay fixture, both CLI tests and scoped warning-denied Clippy pass.

The complete RF3 batch remains unfinished: budgeted descendant relay, exact live
placements, recursive demand, complete recovery reserves and IC evidence are
next. Earlier recovery/speed readiness remains independently valid; this does
not close CANIC-156/174 or establish live Toko recovery. The latest upstream
scan contains no newer confirmed blocker. Both changelogs and the operations
runbook are updated; no version/publication/live effects ran.
[Evidence and boundaries](../../audits/reports/2026-09/2026-09-18/rf3-local-demand.md).

## CANIC-166/172 retirement evidence — 2026-09-18

The new .24 staging admission blocker is corrected in the open .25 batch.
Immutable completed retirement accounting preserves its recorded vocabulary
and canonical encoding, so Toko's otherwise-current retained plan verifies
against its original digest. No executable predecessor schema, field defaults,
payment substitution or plan rewrite was introduced. Fresh accounting remains
strict. An unfinished operation receives an explicit recovery diagnostic when
a new reinstall is requested; original funding/recovery must complete before
the selected-release reset review.

348 focused native tests, one exact production-Ledger mint/withdrawal and
lost-reply proof, and all-target/all-feature warning-denied host/CLI Clippy pass.
The exact retained source also passes local journal/mint admission on an unchanged
private copy. Both changelog surfaces cover this correction and the already
qualified speed work; the selected .25 batch is ready for its release flow.
Live staging recovery is still downstream qualification after publication,
including exact retry/receipt status, source authority and cycle accounting.
No version bump, publication or live effect ran.
[Evidence](../../audits/reports/2026-09/2026-09-18/retirement-recovery.md).

## Fixture build throughput — 2026-09-18

The selected .25 planning/fixture speed batch is complete. Audit-Root fixtures
no longer compile and finalize an unused canonical Root. Generated infrastructure
packages are prepared before enclosing Cargo input snapshots, fixing the late
cache rejection exposed during qualification without weakening mutation guards.
The cold-directory regression, exact child-funding proof, retained-estate reinstall
proof and scoped warning-denied Clippy pass; all 21 child-funding artifacts match
the reference byte for byte. Observed warm artifact construction is 38.56 versus
13.93 seconds, with no full-release speed claim.

Both changelog surfaces are ready for the maintainer-selected release flow;
packages remain .24. Shared host/runtime snapshot isolation and RF3 remain
separate work, not prerequisites for these qualified fixes. No publication ran.
[Evidence](../../audits/reports/2026-09/2026-09-18/fixture-build-throughput.md).

## PocketIC redundancy cleanup — 2026-09-18

The open .25 speed batch removes the repeated invalid-state lifecycle upgrade
and the one-asset uncertain refill case subsumed by the four-asset proof. All six
lifecycle tests, the surviving four-asset case, the dynamic inventory check and
scoped warning-denied Clippy pass. Required qualification warm-ups and distinct
reset/funding journeys remain. Sharing host/runtime fixtures requires additional
isolation design; repeated artifact/setup work remains the larger speed target.
This completes the deduplication review, not the entire accepted throughput batch.

[Dispositions and focused evidence](../../audits/reports/2026-09/2026-09-18/pocketic-redundancy.md).

## Post-.24 planning throughput — 2026-09-18

The maintainer confirms .24 is pushed and explicitly continues .110 operator
latency corrections. Keep these accepted regressions on the affected minor
despite the soft twelve-release guideline; do not infer minor closeout.

The bounded Root/Store observation slice is complete in the open .25 speed batch.
Canonical owner: host current-protocol planning. Independent pairs share the
existing four-read collector; exact Root/Store agreement, configured ordering,
drained failures and fresh retry remain. Native transport fixtures prove overlap,
partial batches, missing authority, disagreement and corrected retry. The opt-in
same-executable measurement shows a benefit for multiple Roots, with no claim
for single-Root or complete release performance. No paid effect changed.

Latest Toko feedback records .24 adoption; retained staging recovery and actual
deployment/cache measurements remain downstream evidence, not new confirmed
Canic defects. Next accepted speed work is attribution and reduction of repeated
build/setup work in the long PocketIC journeys, retaining their full safety
coverage. Keep further slices in .25 rather than recommending a patch for this
one change. RF3 remains a separate accepted recovery-forecast batch.

[Current evidence](../../audits/reports/2026-09/2026-09-18/root-authority-throughput.md).

## Accepted Toko feedback follow-up — 2026-09-18

The additional CANIC-176, CANIC-172 and CANIC-160 work is complete in the same
open .24 batch. Lock-owner diagnostics remain advisory; pre-build readiness is
effect-free and keeps caller estimates separate from spending approval. A real
Ledger mint now joins original native withdrawal, lost-reply recovery, terminal
accounting and replay in one focused host proof. This extends the monetary
qualification below but does not claim the full managed Fleet or live staging.

The synthetic readiness comparison reduces median status-read time by about
66% for three and nine owners, preserving exact reads and call counts. It does
not measure total downstream deployment or compilation. Thirty native tests,
two CLI help checks, both exact Ledger cases, the isolated comparison, scoped
warning-denied Clippy and format/layering/script checks pass.

The complete selected batch is ready for the maintainer-selected release flow.
[Evidence and limits](../../audits/reports/2026-09/2026-09-18/toko-feedback-follow-up.md)
record the source identities and remaining downstream adoption, readiness-wrapper
integration, actual retained staging recovery and complete timing qualification.
Packages remain .23; .24 changelog surfaces are ready. No publication or live
effect occurred.

## Native donation tolerance — 2026-09-18

The maintainer requests correction of native canister balance increases that
currently reject recovery. This extends the selected .24 correction batch.
Creation, funding, duplicate idle observations and terminal accounting must
accept native donations without creating new payment authority. Net accounting
replaces the narrow Stop-credit inference described in older entries below.
Original balances, receipts, action budgets and deletion residual limits remain
binding; the retained successor net-debit watermark cannot decrease. Deposits
can mask simultaneous execution consumption, so net balance evidence does not
prove a gross-burn ceiling. Ledger-account credits remain a separate contract.

| Field | Donation follow-up |
| --- | --- |
| Owner | Host Fleet observation/funding/conservation; runtime pool first observation; CLI reports |
| Evidence | Native success, excessive loss, authority/receipt rejection, retry, terminal replay and retained successor allowance; focused real-canister creation/recovery proof |
| Validation | Focused native tests, exact host PocketIC case, affected-package warning-denied Clippy and formatting |
| Status | Complete; 359 selected native tests, exact host PocketIC recovery/replay proof, scoped warning-denied Clippy, formatting and layering pass; .24 batch ready for the maintainer-selected release flow |

Current qualification and remaining limits are recorded in the
[current handoff](../../status/current.md). No change to deletion safeguards or
Root/operator Ledger credit authority is included.

## CANIC-172 operator admission and retained recovery — 2026-09-18

Published .23 accepted a full reinstall with 693,932,575,589 operator cycles
against an 81,375,002,534,785-cycle reviewed debit. Full reinstall verification
returned before the ordinary balance check; its rejected withdrawal retained
one original Intent. CANIC-166's two preparation seals had completed.

The .24 correction checks maximum debit at the common fresh-journal boundary
before any replacement, adoption or funding intent. The selected-plan quote
reports balance, exact shortfall, observed CMC rate and separate transfer/deposit
fees without payment. It is not the requested pre-compilation readiness check.

RF2 is restored from the preserved snapshot and integrated into this correction.
Original first-withdrawal recovery has its own exact funding-review binding;
supplementary Native/Estate reviews can also proceed when the operator needs
conversion. The host retains amount/fee/time/account/network authority before
payment, authenticates the exact ICP transfer and memo-bound Cycles Ledger mint,
and adds actual net credit once. Baselines and source seals remain immutable.
Conversion approval is separate from original/supplementary withdrawal approval.
No journal reset, independent top-up, duplicate timestamp or new withdrawal is
used to escape ambiguous payment state.

| Evidence | Result and boundary |
| --- | --- |
| Native host/CLI | 80 host and 19 CLI cases pass; original underfunded Intent reaches convergence with lost-reply recovery and effect-free replay; negative crypto/archive/outcome cases use native fixtures |
| Production Ledgers in PocketIC | Normal conversion plus lost transfer/notification replies and pre-credit interruption pass; actual gross/fee/net and receipt authentication, once-only admission and conversion replay checked |
| Managed mixed-topology PocketIC | Earlier fresh-admission rejection and full reinstall/replay pass against the operator Ledger stub |
| Remaining live evidence | Exact staging authority/request age/available receipts, complete retained Fleet apply and terminal conservation; no live operation was changed |

The production-Ledger proof's later Fleet funding uses a host model with no
Coordinator topology; it is not a combined real-Ledger managed-Fleet recovery.
Certificate acquisition now handles owned pruned trees under existing budgets
and Agent verification. Receipt discovery is bounded to one certified ICP archive
hop and 1,024 Cycles Ledger blocks. Missing/expired evidence remains unresolved.
The [upstream Ledger source](https://github.com/dfinity/cycles-ledger/blob/main/cycles-ledger/src/storage.rs)
shows a prunable duplicate index and a balance check before later timestamp
validation. Therefore a later insufficient-funds response alone is not proof
of nonpayment; this source inference does not qualify a deployed Ledger module.

The selected correction and preserved speed batch have updated operator docs
and the open .24 changelog. Scoped final checks and exact logs are in the
[current handoff](../../status/current.md). No broad gate was pre-run. Complete
live recovery forecasts (RF3), B1 and early pre-build readiness remain follow-ups.
Packages remain .23. The saved RF2 source is untouched; no publication, version
bump, live funding or sibling mutation occurred. This necessary operator recovery
fix stays on the affected .110 line beyond the soft twelve-release guideline.

## Post-.23 host speed batch — 2026-09-18

The maintainer confirms .23 published and requests Toko feedback first, then speed.
The latest feedback records .23 adoption with 255 native tests, strict Clippy,
Wasm/Candid checks and both managed tests passing. CANIC-166 live retained-source
review/apply remains separate, with no new confirmed Canic defect.

| Field | Selected bounded batch |
| --- | --- |
| Outcome | Avoid session-only rebuilds, repeated release-test compile graphs, undersized fixture-cache retention and sequential protocol-owner status waiting |
| Owner | Host build environment/caches, Fleet status collector, workspace runner and internal fixture artifact owner |
| Evidence | Session-only control misses with identical compiler Wasm; candidate hits with equal input/release/output identities; build-script/compiler/extractor absence checks; real inputs and unknown launcher keys still invalidate |
| Validation | 70 build/cache tests, 46 platform tests, exact host PocketIC proof, controlled Cargo graph experiment, runner contract and shell lint pass; scoped strict Clippy/formatting pass; one existing build benchmark ignored |
| Status | Complete bounded batch; .24 draft ready for the maintainer-selected release flow |

The next requested extension overlaps protocol-owner status with the existing
four-read bound. Native transport tests prove partial batches, complete draining,
configured-order errors, stopped-owner precedence and fresh retries. Runtime
module checks remain after the status scan; no mutation order changes.

The requested release-test extension reuses the ordinary workspace graph for
full validation's serial host proof, avoiding the observed package-graph switch.
The exact proof remains selected and passes. The controlled Cargo fixture retains
the original executable with `fresh=true`; full-run timing remains unmeasured.
Ordinary unit/binary tests and all registered ordinary integrations now share
one workspace Cargo invocation as well. The controlled runner fixture proves
selection, feature unification, continued failures, the PocketIC barrier and
fresh later host reuse. Inventory checks reject cross-class target-name collisions.
The 81-minute release baseline and distinct artifact recipe findings are recorded
in the throughput report. Larger fixture/build attribution remains follow-up;
the accepted nineteen-Workload/five-Ready capacity proof is not reduced.

The fixture compiler cache's measured Local footprint is 4.36 GiB, exceeding its
former 4 GiB whole-target cleanup threshold. The threshold is now 8 GiB per
network, with seven-day expiry and hourly locked maintenance retained. Clearances
and maintenance failures are visible without verbose output. Six artifact tests
and strict internal library/test Clippy pass, including actual retention of a
5 GiB sparse fixture and safe clearance at 9 GiB. No historical eviction or
whole-run saving is claimed; the old normal logs do not establish either.

This consolidates build environment, cache identities, status reads, release-test
runner reuse, regressions and documentation in one speed batch. No new cache or runtime protocol is added.
No external effects need interruption qualification. Direct/Make/CI/Toko launcher
comparisons and end-to-end latency remain wider performance work; RF2/RF3/B1
remain separate. The minor already exceeds the 12-release guideline: this stays
on the affected .110 line as an explicitly requested operator-performance
follow-up, without opening another minor or allocating one patch per proof.
See the [throughput report](../../audits/working/0.110-validation-throughput/report.md)
and [current handoff](../../status/current.md) for evidence and limits.


## Post-.22 completed-source retirement extension — 2026-09-17

CANIC-166's new completed staging source is distinct from the earlier issued
partial activation. The selected .23 extension admits a separate review only after
exact Applied receipts, immutable phases, fresh complete inventory and bounded
cycle conservation reconcile. It reuses the existing preparation, effect journal
and atomic handoff; source forecasts stay opaque and source actions never become
executable replacement authority. Original phase files are archived with the source
plan, journal and state. Review includes measured source accounting.

Local source inspection passes for the reported 61-effect/two-phase source.
Nineteen focused host regressions, the CLI retained-input regression and scoped
warning-denied Clippy pass. The selected .23 local batch is ready for its release
flow. Actual staging review, interrupted remote deployment and terminal effect-free
replay are not established. This is a
host-tool correction with no new runtime protocol or game rebuild requirement.
The previous selected-batch readiness statement below predates this extension.


## ICYDB-029 runtime composition completed — 2026-09-18

The requested published IcyDB 0.258.0 update now completes this seam in the open
.23 batch. Both maintained locks resolve its six packages and one ic-memory
0.14.3. One artifact-owned admission callback is sealed before bootstrap, called
over the complete snapshot before allocation commitment and covered by policy
identity. Typed rejection, Canic range ownership and the single lifecycle owner
remain intact. Both fixtures use logical keys and explicit host grants.

Five native admission/grant regressions, 24 memory tests, 26 boundary guards,
the existing seven PocketIC lifecycle/import journeys and scoped warning-denied
Clippy pass. Both standalone audit feature variants compile under strict lint.
Native omitted-journal qualification covers selection, original bytes/placement
and grant rejection, not all IcyDB debt/retirement states. The current handoff
records evidence and limits. This does not qualify Toko's live estate or add
cross-release migration. No package publication or external mutation occurred.

The selected .23 batch and changelogs are ready for the maintainer release flow;
packages remain .22 and no full validation was run. RF2/B1 and wider speed work
remain separate follow-ups. Test-only IcyDB remains outside normal release
composition admission.


## Post-.22 Toko build-admission and diagnostic extension — 2026-09-17

The same .23 batch now addresses ICYDB-029's application memory-runtime split
and CANIC-176's frozen-checkout output warning. Selected-role validation uses
exact Cargo feature options and one reachable ic-memory package identity; whole-
App preflight precedes infrastructure compilation. Output diagnostics report
resolved runtime/declaration roots and warn about another workspace's target,
while preserving explicit dedicated paths and existing cache/lock authority.
Test-only IcyDB remains separate from Canic release admission.

Fifty role-contract and seventy build/cache tests pass (one existing benchmark
ignored), together with scoped core/host/CLI/internal-test warning-denied Clippy.
The selected dependency/diagnostics batch and changelog are ready for the
maintainer's release flow. Packages remain .22; no broad gate or publication ran.
No new deployment-speed measurement or downstream adoption is claimed. See the
[current handoff](../../status/current.md) for exact feedback identities, evidence,
limitations and the separately retained RF2/B1/performance work.

## Post-.22 testkit dependency extension — 2026-09-17

The same .23 dependency batch adopts ic-testkit 0.10.0 and its retained exact
artifact ownership contract. Internal builders return retained records; fixture
reads, extraction and Root staging keep those owners alive. Complete release
fixtures stage from retained inputs into private invocation directories, and
blob-storage upgrades reuse their installation bytes. The earlier ic-memory
.14.3 changes are preserved. Production runtime behavior is unchanged.

Seven focused cache tests and the exact managed-projection PocketIC lifecycle
case pass, together with scoped warning-denied Clippy including internal test
code. The selected dependency batch and both changelog surfaces are prepared for
the maintainer's release flow; package versions remain .22. No broad gate or
publication ran. External IcyDB alignment remains separate. See the
[current handoff](../../status/current.md) for evidence and limitations.

## Post-.22 memory dependency batch — 2026-09-17

The maintainer confirms .22 is published and requests current ic-memory. The
selected .23 draft updates Canic and both lockfiles to published .14.3, adopting
the upstream ledger encoding hard cut through the existing reinstall-only rule.
Fixed allocation IDs, bucket sizes and host policy are unchanged. Test-only
IcyDB's separate dependency schedule remains outside this release boundary.

Twenty-three memory tests, four production ABI guards and the all-feature core
Wasm check pass. Locked standalone graph inspection also resolves Canic to .14.3.
The dependency batch and changelog are prepared for the maintainer's release
flow; package versions remain .22 and no publication or broad validation ran.
This is an existing-line dependency follow-up, not a next-minor decision. See
the [current handoff](../../status/current.md) for evidence and limitations.

## Additional .22 native build reduction — 2026-09-17

The next accepted speed attempt reduces native debug records to line tables.
The matched host-harness compile/link pair improves 68.415s → 54.320s; the final
native graph yields a 296 MB harness versus 834 MB with full debug information.
This retains file/line backtraces and all test membership, with full debugger
variables/types available through Cargo overrides. Fast/Release Wasm settings
and runtime behavior are unchanged.

Sixty-eight build/cache tests, six reinstall guards and a focused debug/overflow
probe pass. The selected extended .22 batch and changelog are ready; package
versions remain .21 and publication is not performed. The result is a scoped
native-build improvement, not a measured 90-minute release reduction. See the
[current handoff](../../status/current.md) and
[throughput report](../../audits/working/0.110-validation-throughput/report.md).

## Extended .22 speed batch — 2026-09-17

The maintainer kept .22 open for Toko launcher-cache diagnosis and repeated
artifact/reinstall cost. The completed extension normalizes shell depth in both
build execution and cache identity, and overlaps retained-asset verification in
existing four-read batches. Fresh exact authority, deterministic rejection and
interrupted-operation safety remain required. The shared Cargo intermediate
experiment showed no benefit and was discarded.

Sixty-eight build tests, six reinstall host tests, scoped warning-denied Clippy
and the exact mixed-topology PocketIC recovery case pass. Both deliberate resets,
conservation and effect-free replays are qualified. The selected expanded batch
and changelog are ready for the maintainer's release flow; versions remain .21.
No broad gate or publication ran. The candidate selected-build reinstall takes
407.24s versus the earlier unmatched 411.44s; this does not establish a meaningful
whole-deployment gain. Downstream matched measurements remain open. See the
[current handoff](../../status/current.md) and
[throughput report](../../audits/working/0.110-validation-throughput/report.md)
for the controlled launcher result, timings, identities and limitations.

## Post-.21 selected diagnostic/dependency batch — 2026-09-17

The published base is .21. The open .22 batch includes complete CI dependency
prefetch, CANIC-179 retained-plan guidance, ic-memory .14.1, test IcyDB .257.22
and the maintainer-accepted CANIC-160/176 build diagnostics. It stays on the
existing .110 line as downstream operator/dependency follow-up; RF2 and broader
recovery forecasts remain separate. Test-only IcyDB alignment does not block
Canic's production graph or release.

Heartbeats identify roles/batches and distinguish child time from phase time.
Optional private keyed comparisons name changed environment inputs without
retaining raw values or weakening cache identity. The build guide records
comparison limits and key privacy. Sixty-eight build and seventeen durable-file
tests pass, along with scoped warning-denied Clippy and the changed-file secret
scan. The complete selected .22 batch and both changelog surfaces are ready for
the maintainer's release flow; versions remain .21. No broad gate or publication
ran. See the [current handoff](../../status/current.md) for logs, downstream
limits and the earlier dependency qualification. Toko still needs new comparison
evidence to identify its actual environment-only miss; no whole-deployment
speedup is claimed.

## Post-.20 selected correction batch — 2026-09-17

The maintainer requests CANIC-178 first, then new Toko feedback and speed work.
The open .21 batch contains caller-scoped automatic funding identities, retained
protected failure evidence, a real two-Shard qualification, bounded terminal
partition reads and Cargo-phase heartbeats. The .20 release is published; package
versions remain .20; this draft has completed its focused qualification. This is a correctness and
operator follow-up on the affected .110 line, within the release-count exception.
No new minor, closeout audit, full validation or publication is authorized.

The requested IcyDB follow-up aligns both maintained fixtures and their lockfiles
on published 0.257.21. Locked offline lifecycle-probe and composed-participant
checks pass; the .21 draft includes this dependency update. Historical Wasm
measurements retain their original source identities.

The next requested speed round extends four-wide terminal reads to each parent's
independent child allocation receipts, with validation before management inspection.
Twenty focused host tests, scoped warning-denied Clippy and the existing exact
mixed-topology recovery case pass, including both deliberate reinstalls and
effect-free replay. The transport test proves sibling overlap and failed-batch
draining; the live fixture proves integration with one child per parent. This
does not establish a whole-deployment speedup. Fresh Toko working-tree review
finds no additional Canic funding defect; publication/adoption remains pending.
See the [throughput report](../../audits/working/0.110-validation-throughput/report.md)
for evidence and limits. This continuation stays in the same .21 draft.

The subsequent requested speed slice bounds independent Candid extraction to
four workers. It preserves exact cache binding, drains failures and derives no
partial runtime profiles. Ten focused tests and scoped warning-denied Clippy
pass. The same six retained Wasms yield a controlled median extraction reduction
from 5.245s to 2.732s with identical interfaces. This bounded build-phase gain
joins the existing .21 batch; broader compile/deployment latency remains open.

The identity fix preserves the parent actor boundary and same-release durable
retry. Existing cycle telemetry owns the diagnostic; existing bounded observation
and process owners own the speed/progress changes. RF2 and complete recovery
forecasts remain separate. Dedicated downstream acceptance, environment changed-
value attribution and matched deployment measurements remain open. See the
[current handoff](../../status/current.md) for the passing 27 core, 21 host and
one CLI regressions, the two-Shard PocketIC case and scoped lint. The selected
correction/progress batch is ready for the maintainer-directed release flow;
publication and live Toko recovery remain separate.


## Selected .20 release-test correction — 2026-09-16

The maintainer selected the completed fixes for release; RF2, complete live
forecasts and broader speed qualification remain follow-up work. The full release
run exposed loss of the retained initial-child failure after parent runtime
acknowledgement. Membership retries now use the same diagnostic lookup as runtime
activation, preserving the exact child failure through Coordinator and host.
Five focused unit cases, scoped warning-denied Clippy and the exact failed IC
case pass. The selected correction and .20 changelog are ready for the release
retry; the agent did not repeat the full gate or perform publication. See the
[current handoff](../../status/current.md#release-test-correction-child-failure-through-membership-retries--2026-09-16)
for exact evidence and limits.

## Management-only funding protection complete locally — 2026-09-16

The open .20 batch now includes Stop → optional Fund → Reinstall → Start,
with initial operator/fee coverage, per-attempt Stopped/source-authority checks,
immutable payment identity and retained-receipt conservation. Source-bound
activation resets preserve funding budgets; preparation remains unfunded.
A reset inspection also removes one duplicate Root status read (five → four
remote calls in the matched native fixture), without reuse across passes.

Twenty-three focused host tests, scoped warning-denied Clippy and the final
nineteen-Workload/five-Ready disposable PocketIC journey pass. The latter covers
lost funding/install replies, restart/controller rejection, one withdrawal,
full recovery and effect-free replay. It uses a local audit Root, not a production
Root artifact or mainnet. See the [qualification report](../../audits/working/0.110-validation-throughput/report.md#stopped-root-recovery-funding-and-reset-authority-reads--2026-09-16)
for logs and the retained earlier high-bound continuation rejection.

Toko feedback advanced to `5fb157bf1dd1c6fa34bbd42341af541c44894b0174fe9c828e0829561d4c17ce`.
Complete live recovery forecasts and representative deployment-speed evidence
remain open. RF2 remains preserved and sequenced before RF3. Persisted Registry
prefixes need the existing upstream collector's API; Canic must not duplicate
that authority owner. The complete batch is not yet push-ready; the .20 draft
is updated and packages remain .19. No broad gate or publication ran.

## CANIC-139 deployment credential exclusion — 2026-09-16

Toko feedback is unchanged at SHA-256
`62e20bfa84b2586971051f279b51221edef50414c42c06487190568722065f92`.
Build subprocesses now remove inherited `CANIC_ICP_IDENTITY_PASSWORD_FILE`;
complete-build and Candid-extraction identities and diagnostics exclude the same
key. Cargo, compiler/cache probes, Wasm tools, provenance and tool acquisition
share the boundary. Every other inherited environment key remains bound.
Deployment identity unlocking, exact release/output checks and source-drift
rejection are unchanged. This is an environment contract, not a build sandbox.

Isolated child invocations qualify changing, removing and restoring the credential:
real build.rs/rustc and a native extractor see no inherited credential; compiler
Wasm stays identical; the same synthetic sealed release is found before Cargo.
A genuine build-script input, dependency source and configuration each invalidate
reuse. This uses a minimal synthetic dependency workspace and release manifests;
it does not measure production Fleet or Toko build time. The affected build/tool/
provenance/identity selection passes 115 tests, with one existing real-extractor
qualification ignored. One cache-probe assertion failed in the initial selection;
its diagnostic now preserves the actual typed error, and the same selection
passes on rerun. The initial cause remains unconfirmed. Host all-target/all-feature
Clippy with warnings denied passes after private-module visibility cleanup.
Logs: `/tmp/canic139-build-environment-regression-retry.log` and
`/tmp/canic139-build-environment-clippy-retry.log`; the initial failure is retained
in `/tmp/canic139-build-environment-regression.log`.

The .20 changelog and build documentation are updated; packages remain .19.
The accepted batch remains open for management-only funding protection, complete
recovery forecasts and measured remote-call reductions. RF2 remains preserved;
downstream adoption and complete deployment-speed qualification remain separate.
No broad validation, version bump, commit, push, deployment or sibling mutation ran.

## CANIC-156 funding fence and CANIC-160 wait output — 2026-09-16

Fresh read-only Toko feedback advanced to SHA-256
`62e20bfa84b2586971051f279b51221edef50414c42c06487190568722065f92`.
The new request is unchanged provisioning-output suppression; admission remains
fixed locally as recorded below.

Before issuing a native or estate credit to a Root/Coordinator that has a reviewed
reinstall ahead of it, the host now verifies source management authority and the
exact operation's existing seal. A changed module, controller set, recipient,
subnet, status or seal rejects before credit. The plan includes the additional
management/status observation allowance. Successor funding after replacement
skips the old seal; this guard does not add protocol authority to management-only
recovery, pause an unverified installed runtime or protect external top-ups.

The CLI now emits meaningful progress changes immediately and unchanged waits
only on the first observation at least 30 seconds after the previous output.
Elapsed time alone is excluded from change detection. Text and JSON preserve
current elapsed/pending phase details; funding, review, terminal and error output
remain immediate. No host polling or recovery behavior changed, and this is not
a deployment-duration improvement.

Five new funding-boundary tests, 17 existing reinstall regressions and 26 Fleet
CLI tests pass. The existing five-Workload/one-Ready PocketIC journey now also
proves a durable pre-payment intent, zero credit after explicit Root seal removal,
resealing the same operation, one native withdrawal after losing its successful
response, and rejection of child grants while sealed. It then completes the
existing interrupted installs, separately reviewed descendant funding, two
deliberate wipes, row reset, conservation and effect-free replays. The fixture
counts the pre-reset credit in the operation's full operator debit. An initial
run correctly rejected an invalid fixture minimum above its initial allocation;
the corrected configured pair passes. This is one current-protocol Root native
credit, not live Coordinator/estate-credit coverage, management-only protection,
mainnet qualification or a deployment-speed measurement.

Host/CLI and internal-testing all-target/all-feature Clippy with warnings denied
pass. Logs: `/tmp/canic156-funding-seal-unit.log`,
`/tmp/canic156-reinstall-regression.log`, `/tmp/canic160-progress-cli.log`,
`/tmp/canic156-160-clippy.log`, `/tmp/canic156-sealed-funding-pocketic-retry.log`
and `/tmp/canic156-sealed-funding-clippy.log`. The exact selected PocketIC case
passed in 637.07 seconds (656 seconds for its runner, including setup/builds).
Targeted runs disabled the compiler wrapper after the local sccache socket
returned `Operation not permitted`.

The .20 draft includes both fixes; packages remain .19. The full accepted batch
remains open: complete live forecasts, management-only funding protection,
controlled build subprocess environment and measured remote-call reductions
remain. RF2 stays separately preserved. No broad validation, version bump,
commit, push, live deployment or sibling mutation ran.

## CANIC-108 admission interface and response correction — 2026-09-16

New Toko feedback SHA-256
`5ac3702fdf5fd18efc2b19cc2ed5b9f5780518d6c02985a91ba6a0521e9beac6`
reproduced admission's mutable-sidecar lookup on released .19. The shared
admission connection now uses the terminal selected release, with finalized
manifest and participant module/role/profile verification. A real CLI probe
then exposed redundant `Result` decoding in five admission calls; those now
consume the shared transport's already-unwrapped response.

Ten binding regressions, six admission/Medic tests, scoped Clippy and the existing
four-Workload/four-Failed PocketIC recovery case pass. That case additionally
proves ordinary admission plan/apply/status/replay, one generation advance and
the added Principal, with no environment-local Candid copies. Its admission
catalogue has one Root and zero managed participants, so application ingress,
multi-Root and downstream release acceptance remain separate. The .20 draft
includes the fix; packages stay .19 and the full batch remains open.

Continue with current-source recovery headroom protection through the existing
seal owner, then the accepted build-environment and remote-call speed candidates.
Retain management-only authority boundaries for an unreadable installed module;
do not add predecessor-protocol negotiation. RF2 remains preserved.

## Native top-up observation contraction — 2026-09-16

CANIC-160's duplicate pre-withdrawal read is removed through the existing effect
executor. Ops captures a new intent and its first native-funding observation;
workflow persists intent before payment and consumes that observation once.
No balance cache crosses an effect or restart. All 58 selected native workflow
tests pass. The matched four-Workload/four-Failed PocketIC case preserves exact
withdrawal counts, lost-response recovery, conservation and replay with identical
Wasms. Funding reads fall 13 to 9 (52 to 36 remote calls); measured read time is
10.69s / 7.14s, and the live journey excluding artifacts is 78.19s / 75.86s.
One local pair does not establish mainnet or whole-release improvement.

The baseline also exposed direct targeted runs bypassing the stable cache wrapper.
The scratch runner now selects it when no wrapper is explicit; its existing
executable contract tests cover selection and explicit/empty overrides. The .20
draft remains open for the previously recorded recovery and throughput scope;
RF2 is preserved and package versions remain .19.

The final downstream scan changed to SHA-256
`af360d5009c2be2693edac2fd0e35c594f4a3c09dfc58ef655e1dcfa6adf5406`,
Toko commit `3e2d16c8cf77b30407ce7893a401a24f2eeef753`. Its staging
latency receipt adds reasonable CANIC-139/160 candidates, not proven removable
time. Sequence the next bounded assessment around a controlled build environment:
deployment credentials may leave the fingerprint only when they also leave every
relevant build subprocess, with unchanged-repeat and real-input invalidation
proof. Investigate Registry restart reuse at the existing `ic_query` source
boundary, with corrupt-prefix and disagreeing-head rejection; no sibling mutation
is authorized and no duplicate Canic collector should be introduced. Attribute
per-effect issue/await/receipt costs before considering mutation batches. Preserve
dependency order, current authority, individual receipts and interruption recovery.
Typed terminal projection remains Canic-owned evidence with downstream alias
application; do not create a second executor. These larger candidates remain
separate from the completed native top-up slice.

## Rejection diagnostics and authority funding admission — 2026-09-16

CANIC-139 now retains optional bounded post-build rejection evidence under the
existing build-reuse owner. Three complete-build tests qualify path/fingerprint
evidence, successful-retry retention, invalid diagnostic isolation and original
typed-error preservation when persistence is unavailable. This does not prove
the historical Toko Translation cause.

CANIC-156's seal review now requires each Root and Coordinator to cover its own
unchanged conservative allowance. Native cases reject either deficient authority
despite aggregate surplus and admit exact individual headroom. The existing Root
restore PocketIC case proves ordinary grants are fenced before/after a top-up,
explicit live resume permits funding, repeated requests do not credit twice and
snapshot restoration preserves the fence. Automatic management-only recovery
protection and complete live successor forecasts remain open. Reuse the existing
authority owner for current-source preparation; do not add predecessor-protocol
fallbacks or assume a read-only review pauses funding.

Scoped host/internal-testing all-target/all-feature Clippy passes. The .20
changelog is updated; versions remain .19, RF2 is preserved, and the accepted
throughput/recovery batch is not yet complete. No broad gate or publication ran.

## Startup forecast and build-input continuation — 2026-09-16

CANIC-156 now reports configuration-bound startup minimum, continuation steps and
allowance, configured minimum and explicit reuse assumptions before Root reinstall.
The same helper feeds prepayment. This completes the requested attributable
forecast slice; protection from concurrent child grants and a complete live quote
remain separate accepted work. The existing nineteen-Workload/five-Ready PocketIC
reinstall journey passes with added pre-mutation forecast checks and unchanged
authority rejection, lost-response recovery, conservation and terminal replay.

CANIC-139 observes retained generated-source exports before compilation and
classifies existing paths against resolved output roots. Twenty-one reuse tests
cover cold/warm generated catalogue changes, missing dependency records, foreign
inputs and parent traversal. Latest Toko feedback SHA-256 is
`e314310837501adce7d84925504e217c0c80540f73891325b1082e323a3401ef`;
its new Translation rejection is not reproduced by current source/input evidence.
Do not claim the historical first-build cause is established or weaken drift checks.

The same live journey attributes configured inspection latency: six 52-call local
observations take 4.06–4.61s, with median protected-update latency 401ms versus
78ms reserve preflight across 163 pairs. Temporary probes were removed. Further
speed work should investigate the protected-call path while preserving its safety
bounds; these timings are neither mainnet qualification nor a matched speedup.
The .20 draft remains the current batch; RF2 stays preserved and versions stay .19.

## Release-test throughput follow-up — 2026-09-16

The next bounded transport change consolidates three temporary argument writers
under ICP and removes per-call disk flushes from disposable invocation files.
Private creation, complete child reads, cleanup, Fleet argument bounds and
durable journal writes remain qualified. Fifty-six selected native tests,
host all-target/all-feature Clippy and the matched four-Workload/four-Failed
PocketIC journey pass. Wasms and all 224 observation calls match; the small
21.522s / 21.202s observation difference does not establish a deployment speedup.
The isolated file probe shows the intended I/O reduction. Configured inspection
already runs four-wide; larger gains need further cost attribution, without
skipping per-target reserve checks. The throughput batch and .20 draft stay open.

Fresh Toko feedback at SHA-256
`79d5292f7ca93d0f79de8d77ce399dc2feb04e2ab4a27e067ae34e8b397eb233`
adds CANIC-175 mainnet restoration acceptance: ten successful targets, none
omitted/unconfirmed and no reconciliation failures. This is released-.19
evidence, not a new defect or injected partial-failure qualification.

Fresh CANIC-160 mainnet evidence prioritizes retained pool observation cost.
PendingReset/Failed balances now use the existing four-wide collector while
retaining every reserve/authority check and fresh observation scope. The same
four-Workload/four-Failed PocketIC recovery journey passes before/after with
identical Wasms and call counts. Pool-balance time falls 8.112s to 2.579s; the
live journey excluding artifacts falls 84.337s to 79.303s (6.0%, one pair).
Configured-canister mainnet latency and total release duration remain separate
acceptance. The two new native cases cover bounded execution, failed-batch
drainage, error ordering, unchanged failed inventory and fresh retry balances.

CANIC-139's controlled linked-output probe reproduces foreign-path emission and
overwriting another copy's generated file. Generation now rejects that link
early, and fixture persistence names the exact linked path, including `.canic`.
This does not establish the historical downstream copy sequence. Normal output
repair/reuse remains qualified. Dependency records also reuse a file's first
observation only within the current snapshot; a collector-only comparison keeps
all 780 paths/hashes and lowers median time 348ms to 163ms. Nine fixture tests,
13 build-support tests, two build-macro integration tests and 19 reuse tests pass.
The .20 draft includes these bounded changes; RF2 stays preserved and the larger
throughput objective remains open. No new patch version or publication occurred.

The .20 request-process continuation is qualified: two default Tokio workers
reduce measured observation time from 17.654s to 15.432s across the same 169
calls in the existing four-Component/four-Ready recovery journey. Explicit
settings and replica-start contexts are preserved. Wasms, recovery, conservation
and replay match; full live-journey time is essentially unchanged. A preceding
concurrent partition-read experiment was discarded after no measured gain.
Forty ICP tests, 242 Fleet native tests, focused PocketIC and host/internal
all-target/all-feature Clippy pass. The throughput report retains exact inputs.

Fresh CANIC-156 feedback also exposed misleading Root-reinstall headroom
diagnostics and cross-Root aggregation. Per-Root admission and typed diagnostics
now retain the original conservative bound and actual effect count. Recovery
funding protection, startup/continuation breakdowns and complete forecasts remain
separate accepted follow-ups; RF2 is preserved. CANIC-139's new isolated-build
report names generated inputs in the original checkout. Normal first-build
runtime/declaration output is qualified, foreign inputs remain checked, and
cache-isolation guidance is documented. The precise foreign-path producer still
needs controlled reproduction. These completed slices extend the .20 draft;
the full throughput batch remains open. No full-gate speedup is claimed.

CANIC-160's activation progress request is now implemented in the same .20 batch.
The existing typed observation supplies phase and bounded Root/Component counts;
existing waiting events include monotonic elapsed seconds for the current effect
or terminal check in this invocation. No additional polling, durable authority
or completion/retry semantics changed. The 242-test Fleet host selection,
18-test Fleet CLI selection, eight post-cleanup provisioning regressions and
host/CLI all-target/all-feature Clippy pass. The changelog documents the current
Rust/JSON progress-shape change. Downstream live acceptance and matched latency
evidence remain separate. This slice is complete; the full throughput batch stays
open for terminal-inventory and representative subprocess-cost investigation.

The .20 continuation removes repeated Store template-status queries within one
protocol compilation, retaining exact query identity, per-action Candid checking
and fresh evidence across plans, retries and execution. Nineteen native protocol
tests, scoped host Clippy and the existing mixed-topology PocketIC case pass.
Both selected build sets retain all sixteen raw Wasm hashes. In one matched pair,
the same 27 planning observations use 430 calls instead of 690, taking 42.305s
instead of 61.232s. Other observation counts and calls remain identical; the live
journey excluding artifacts is 4.7% shorter. The throughput report records cold
build differences and CPU-accounting limitations. The slice and .20 changelog
are complete, while the broader speed batch remains open. Toko's later live
reinstall verifies CANIC-150 as well as CANIC-014. Its CANIC-160 activation detail
is addressed above. Matched downstream latency evidence remains separate;
no new release blocker was found.

Retained-asset inspection during activation reset now uses the existing four-wide
observation runner with fresh reserve/controller/module/cycle checks, drained
failures and deterministic input ordering. Four native regressions and scoped
host Clippy pass; the existing source-bound reset recovery/replay case passes
with serial and bounded scheduling. One two-asset live pair improves the phase
excluding nested artifact resolution from 84.28s to 78.89s (6.4%); nine native
fixture assets prove the full concurrency bound. This is qualified in .19, not a
full-release speed claim. The concurrent IcyDB update occurred after measurement
and is preserved without delaying this slice for repeat qualification. Exact
inputs and limitations are in the throughput report; the broader batch stays open.

The latest Toko recheck has no new feedback beyond the locally corrected
CANIC-150/014 below. Complete Fleet journeys now use the existing production
App build pipeline, keeping Cargo serial while captured infrastructure artifacts
finalize. Five-role artifact parity, the focused four-shard activation/replay
case, both release-cache regressions and scoped internal-fixture Clippy pass.
The cache binds its fixture producer source; sealing and exact release authority
are unchanged. This slice is qualified and included in .19. Its 61.17s live case
uses warm Cargo, so it is not a before/after speedup claim. The broader speed
outcome still needs the remaining compiler/link and live-journey work.

The latest maintainer instruction prioritizes fresh Toko feedback first.
CANIC-150's phase-only progress gap is reproduced and corrected: emit once
after each durable applied transition, preserving receipt order, exact counts,
lost-response recovery and terminal replay. Host/CLI regressions and scoped
Clippy pass. CANIC-014's duplicate root draft claim is removed; detailed headings
remain the version-transaction owner. Publication/downstream live acceptance
remain separate. RF2 stays preserved.

The subsequent speed correction preserves unchanged generated-source timestamps
and narrows Cargo watches to exact config/manifest inputs. Real-Cargo tests
reproduce both prior invalidations and pass settled reuse, source/metadata
invalidation and generated-output repair. Build-support tests and scoped Canic
Clippy pass. The current handoff and throughput report retain exact logs and
the possible one-time output-watch settling rerun. No whole-gate timing claim
or required recovery-coverage reduction is introduced.

The published .18 run still spends 94 minutes in tests, including 78 minutes in
the internal ordered PocketIC stage. The maintainer prioritizes throughput ahead
of restoring RF2 or continuing B1. Start by removing repeated Cargo catalog
resolution within role reports and terminal inventories; preserve isolated role
feature checks and reacquire evidence on every subsequent operation. This
reduction is implemented, including internal test-Wasm preflight. Sixty-six
focused regressions and scoped warning-denied Clippy pass. Role resolution takes
5.950s / 2.541s and eight-role preflight 8.32s / 3.43s. Qualification and measured
limits belong in the current handoff and the existing
[throughput report](../../audits/working/0.110-validation-throughput/report.md).
No concurrent PocketIC lanes or coverage reductions are introduced. The broader
throughput outcome remains open; this is not a new minor or a release command.

The shared Coordinator and Store fixture now calls the linked production builder
instead of compiling a second native host executable. Artifact parity (raw/gzip/
Candid), Coordinator cache reuse, four helper tests, two release/cache-authority
regressions, scoped host/internal Clippy and the focused Fleet restore pass.
The observed removable native compilation is 69s; no complete-gate speedup is
claimed. The current handoff/report records differing warm-cache conditions and
the final 68s focused runner. Continue attribution of remaining compiler/link
and complete-journey costs after the macro correction above, retaining features,
config and release identity. The qualified changes remain in the open .19 batch, whose
larger throughput outcome is not yet push-ready.

## Selected .18 dependency and build correction batch — 2026-09-15

The maintainer selected the completed dependency/cache/CI fixes for release and
preserved RF2 for the next batch. The release tree now contains CANIC-177's
ic-timers 0.7.1 and CDK/macros 0.20.3/ic0 1.2.0 alignment, CANIC-176's cache and
lock diagnostics, cold-runner CI repair and CANIC-014 changelog clarification.
RF2's incomplete source, dependency additions and journal changes are absent.
The exact 33-file RF2 patch and byte-verified backup are preserved locally under
`.tmp/rf2-preserved-20260915T180303Z/`; the current handoff owns restoration details.
The existing RF2 design remains accepted after .18, before RF3 and B1.

This necessary dependency/operator correction stays in 0.110 despite the
12-release guideline: published Canic's exact timer pin blocks downstream CDK
adoption. RF2 is not being declared complete or split into a published partial
payment flow. No minor closeout, version change or publication is implied.

The selected .18 batch is ready for the maintainer-directed release flow, with
its root/detailed changelog ready. The 31 host and 27 CLI regressions, 22
dependency/macro/timer tests, scoped warning-denied Clippy, main Canic Wasm check,
standalone fixture native check and cheap changed-surface guards pass. The
complete release gate/version/publication flow remains maintainer-directed.
The current handoff records exact logs, preservation/restore instructions and
qualification limits. The development checkpoints below describe preserved RF2
work and earlier qualification, rather than the selected .18 source.

## Post-release continuation — 2026-09-15

The maintainer confirms 0.110.17 is pushed; local HEAD is release commit
`5693d9c31`, and the generated receipt records complete validation for source
`035bcd8dd`. The selected .17 release is published, not an open readiness task.
Earlier checkpoints below retain their historical scope.

RF2 is now active, ahead of RF3 and B1. The
[RF2 implementation contract](0.110-design.md#rf2-receipt-safe-operator-icp-conversion)
records exact approval/intent bindings, ICP/CMC/deposit receipt reconciliation,
net-credit conservation, refund/expiry handling and the focused qualification
sequence. Protocol review confirmed that cached CMC results need independent
deposit binding and that gross minted cycles differ from net Ledger credit.
The first implementation portion adds named intent/receipt/outcome records,
pure checked accounting and deterministic ICP/CMC requests. It preserves exact
transfer identity across serialized restart, binds the deposit memo to reviewed
inputs, and retains duplicate, expiry, Processing/refund and mint-result evidence.
The existing journal now retains one conversion review and separately approved
exact transfer bytes. Restart/repeated approval is byte-stable; approved intents
cannot be replaced or cancelled, and ordinary funding effects remain fenced
while conversion is unresolved. Transfer replies and exact notification intent/
outcomes now survive restart too. Known transaction locators cannot be replaced;
mint/refund outcomes stay uncredited until receipt verification. Forty-five
selected host mint/funding tests and the CLI funding consumer pass, including
unchanged starting balances and already-funded recovery regressions. Wire and
journal checks do not authenticate receipts.

The Cycles Ledger deposit verifier now authenticates the reviewed network root,
Ledger certificate and tip witness, contiguous hash chain and exact mint account,
memo and net/fee/gross accounting. It bounds verification before hashing and
rejects unsupported signed integers before the upstream hasher can panic. Its
wire projection matches the reviewed Ledger's `Value` contract, including Nat64,
and hashes borrowed blocks. The private verified result cannot be deserialized
or independently admit a credit. All 46 selected mint tests pass, including 14
native receipt cases; host all-target/all-feature Clippy and layering pass.
Exact logs, dependency identities and native-evidence limitations are in the
handoff.

ICP receipt ops now signs a one-block replicated read and authenticates its exact
request-status reply, then validates the original protobuf transfer fields without
default fees or substituted timestamps. It rejects foreign read identities,
changed bindings, unsupported encodings and exhausted verification budgets.
The Ledger can now authorize one exact archive callback through its certified
reply. Only that private authorization permits signing an archive read; a separate
certificate must admit the archive's canister range and authenticate the exact
request and transfer. Unsupported callbacks, widened ranges, recursive redirects,
missing/extra blocks and typed archive errors stay unresolved. Shared certificate
verification and native test helpers serve both Ledgers; locked prost 0.14.4 is
explicit in the host. All 61 selected mint tests pass, including six archive cases;
host all-target/all-feature Clippy, layering and current-document guards pass.
Bounded acquisition and durable read/retry ownership,
once-only credit integration, timestamp uniqueness, CLI propagation and PocketIC
qualification remain. No payment or receipt read was submitted.

New Confirmed CANIC-176 takes priority over the next RF2 portion. The code now
explains cache misses with optional redacted input comparison and separates
lock waiting from verification without changing identity or admission. All 18
focused cache/lock regressions pass, including finalized hits, tampering,
isolated environment changes and independent-process contention. Host/CLI
Clippy passes; Toko launcher acceptance follows publication. Operator build
documentation and the existing .18 draft include the completed correction.
Toko's CANIC-172 mint-recovery acceptance cases fit the existing RF2 contract.

Toko's new .17 adoption assessment retains CANIC-156/174 for RF2/RF3 and reopens
CANIC-014 for contradictory changelog release-state prose. The maintained notes
and ownership guidance are corrected in the open .18 draft; immutable tags are
unchanged and release/adoption remains due. Other .17 fixes are now Adopted,
with downstream acceptance limitations retained. The current handoff records
the updated feedback hash. Concurrent cold-runner CI and parked idea work remain
intact. Toko's final update reports passing .17 CI, eight unchanged-build cache
hits and application state/partial-retry qualification, with cold discovery and
production Root/Store evidence still separate. These downstream results were
read, not rerun here. The scan adds no confirmed defect. RF2 and the combined
worktree are not push-ready; this continuation does
not close an upstream issue or the minor. CANIC-177's dependency conflict is now
resolved by published ic-timers 0.7.1: Canic resolves ic0 1.2.0 and ic-cdk/macros
0.20.3 with one timer provider. The current handoff owns focused qualification
status; downstream publication/adoption remains separate.

## Selected release scope — 2026-09-15

The maintainer selected **prepare the completed fixes for release**, using an
already-funded operator Cycles Ledger account for native recovery. Receipt-safe
ICP conversion and complete live recovery forecasts stay as accepted follow-up
work. This decision supersedes the earlier checkpoint readiness statements below.

The selected .17 batch is **ready for the maintainer-directed release flow**:
same-operation native withdrawal and receipt recovery, startup reserves and
grant-timer correction, source-bound activation preparation/reset, role-correct
state fanout, inspection diagnostics/preflight, child usage/allowance diagnostics,
unchanged-release reuse and the completed ICP/IcyDB/ic-query integrations.
Direct positive, rejection and interruption/replay evidence is recorded in the
[current handoff](../../status/current.md#latest-focused-validation) and linked
reports. The root and detailed changelogs describe the complete selected scope.
Packages remain .16; the .17 draft is untagged. The complete release gate has
not been run by this readiness pass. No version or Git publication occurred.

The subsequent release-test failures are corrected: startup prepayment is
included in fixture conservation and mock Ledger backing; a fully funded
continuation may reach its ceiling; and both selected-build wipe paths resume
typed pending initialization against the same reviewed plan with bounded retries.
All three failed cases pass individually, and owning-package Clippy passes.
The [current handoff](../../status/current.md#release-test-corrections--2026-09-15)
records logs and the concurrent dependency-update boundary. Runtime semantics
and the selected release scope are unchanged; the complete gate remains due.

Both maintained IcyDB fixtures now select published 0.257.15, with six aligned
packages in each lockfile, SQL disabled and ic-memory 0.13.3 retained. Seven
focused PocketIC cases and fixture/integration/composed declaration-mode Clippy
pass without Rust API edits. The [dependency handoff](../../status/current.md#icydb-025715-update--2026-09-15)
records the qualification; historical Wasm measurements are unchanged.

The line already exceeds the advisory twelve-release guideline. This boundary
keeps necessary recovery and operator corrections on their affected 0.110 line
without delaying them for a new payment state machine or full forecasting.
It does not close 0.110 or authorize implementation in another minor.

| Batch | Outcome and owner | Required evidence | Status |
| --- | --- | --- | --- |
| RF2 | Receipt-safe ICP conversion within retained recovery; host funding/journal owner | Exact transfer and mint identities, credits, fees and receipts; unchanged original balances; interrupted/lost-response recovery, conservation and effect-free replay; focused host and PocketIC qualification | Preserved for the next batch after selected .18; exact requests, durable review/approval/replies and certified deposit/ICP transfer/archive verification pass native regressions in the saved patch; bounded acquisition/read recovery, credit/CLI integration, timestamp uniqueness and IC qualification remain |
| RF3 | Complete live recovery funding forecasts; host observation/planning owners using runtime policy | Budgeted descendant relay, exact role/placement and policy binding, live usage/reservations in demand quotes, unavailable/underfunded telemetry, full recovery reserve scope; focused policy, transport and IC evidence | Next accepted batch after selected .25 fixes/diagnostics, ahead of B1; full RF3 remains open |

The current diagnostics are allowances and individual inspection reserves, not
complete recovery demand or new spending authority. Minimum native recovery
must remain possible before a Root can afford descendant telemetry. RF2/RF3
retain CANIC-156/172/174's uncompleted criteria; publication/adoption and actual
Toko recovery remain separate. The original password incident is unproven and
auth/E9 remains parked. No upstream issue is declared fully closed by this
scope decision.

## Earlier implementation checkpoints

The dated entries below preserve their original scope and evidence. Use the
selected release scope above for current readiness and follow-up sequencing.

## CANIC-172/174 child usage checkpoint — 2026-09-15

Protected child-funding observations now expose charged totals, unresolved
operations and exact retained transfer reservations, with unknown evidence
explicit. Generation binds selected Root code/operator/participants and displays
seeded direct-child observations alongside its fresh-ledger startup scenario.
The [report](../../audits/reports/2026-09/2026-09-15/canic-172-child-funding-usage.md)
records native expiry/settlement tests, host binding and generation/replay checks,
CLI coverage, scoped Clippy and the real IC grant/replay/controller proof.
No new state or paid effect is introduced. Recursive descendants, exact live
placement matching, recovery quotes and operator mint receipts remain open.
The parent-attribution follow-up now joins each current seeded Workload claim
to its exact committed allocation. It reports the immediate funding parent and
role even for nested children. Only Root-funded Workloads use Root's ledger;
descendant ledger reads explicitly require the existing update relay. Ready,
unresolved and conflicting assets never become zero usage. Generation stays
query-only; further collection belongs in budgeted recovery observation. Native
parent/transport tests, CLI rendering, generation/replay and scoped Clippy pass.
The live allowance follow-up now shares runtime policy for charged lifetime
headroom, cooldown and the next-request cap. It binds allocation release-build
identity and Spec hash to the selected configuration; pending operations retain
an unknown cap and reservations are not deducted twice. Native policy, host/CLI
and generation/replay tests pass. This is policy allowance, not live demand or
spending authority. Minimum native recovery remains independent of optional
descendant telemetry, which may require a funded Root to relay. Full descendant
collection and remaining-demand quotes are still open.
The existing .17 draft remains the release target and is not yet push-ready.

## Accepted ICP 1.5 integration work — 2026-09-15

The maintainer accepted the [ICP 1.5 audit](../../audits/reports/2026-09/2026-09-15/icp-1.5.0-integration.md)
fixes and improvements. Delivery remains in the open 0.110.17 draft:

1. Host build/config correctness: bind script builds to ICP's selected environment,
   parse inline YAML structurally, check effective App role membership, and qualify
   current CLI contracts and selective builds. Preserve complete Canic build closure.
   Positive cases and conflicting/malformed/omitted/empty selection cases belong together.
2. Host/operator inspection: retain typed visibility and query statistics without
   treating viewers as controllers, and reuse successful local version qualification
   within one transport context. Fresh contexts and failed probes must recheck;
   network state, controller authority and balances remain live observations.
3. Frontend: integrate read-only post-sync verification with the existing exact-digest
   handoff. Bind the selected environment, asset Principal and uploaded file identities;
   test wrong target, changed content and immediate repeat verification. Asset upload
   ownership and a new mutation/recovery protocol are outside this delivery.

Each outcome includes focused tests, diagnostics, active documentation and cleanup.
No publication, deployment or broad validation is authorized by this acceptance.
Existing native recovery evidence and B1 work retain their independent readiness gates.

Completed: the [implementation report](../../audits/reports/2026-09/2026-09-15/icp-1.5.0-implementation.md)
retains the focused host/CLI, Fleet fixture, helper, help-surface and native ICP
evidence. Scoped host/CLI and internal-testing Clippy pass. The frontend adapter
uses a documented host script with bounded query readback; portable WASI plugin
distribution remains optional future work. The open patch draft is updated;
these completed ICP outcomes do not close the independent release-batch gates.

## CANIC-156 inspection preflight and diagnostics — 2026-09-15

Host pool/reset, terminal inventory and cycle-observation paths now query the
exact next inspection target's reserve through controller-only Root observability.
The query shares the actual encoded status-call builder, schedules no outbound
call and admits only this additional variant through the prepared-Root fence.
The host rejects mismatched or impossible evidence and observed shortfalls before
the update, without caching a failed preflight or assigning a minimum fee.
One query is added per uncached inspection; bounded pool concurrency and reuse
of successful inspection state remain qualified.

The host first checks the exact bound Candid structurally. Retained source
contracts without the reserve selector keep their protected inspection path and
supply no quote. Declared selectors must pass preflight; invalid contracts and
query failures cannot fall through to inspection. This fixes the new preflight's
unconditional request against retained Roots without changing source authority,
reset admission, or executable plan contracts.

The controller-owned Root inspection now preserves the SDK's exact liquid-cycle
admission failure as `InspectionReserveRequired`. It binds caller and target,
reports native balance sampled before the attempt and the SDK's available/required
liquid cycles, and issues no additional observation call. Only this exact SDK
variant is classified as insufficient funding; other failures retain their cause.
Host pool/reset/inventory/telemetry adapters validate the evidence and keep failed
inspections out of their observation cache. A later retry reads fresh status.

Focused core/host cases, scoped Clippy and the positive-balance PocketIC case
pass. The existing [recovery report](../../audits/reports/2026-09/2026-09-14/toko-recovery-followups.md)
records qualification and limitations, including the prepared-Root query and
shortfall proof. A sufficient query cannot guarantee admission of the later
update, which reserves execution cycles and observes later state. Whole-recovery
funding preview remains open; one outbound-call reserve does not authorize a
funding amount or cover ingress, reset, readiness and later pool demand.

## CANIC-172/174 startup funding — 2026-09-14

Desired-state generation now exposes the zero-burn initial-grant scenario,
request/lifetime/window constraints and Coordinator headroom without changing
funding authority. A shared initial-role counting correction includes every
parent path and rejects cyclic initial demand within a fixed graph bound.
Four pure cases, four generation/pool/replay regressions, the CLI evidence-label
and raw-shortfall case, scoped Clippy and source guards pass. See the
[funding report](../../audits/reports/2026-09/2026-09-14/canic-174-funding-deadline.md).
Pending provisioning on observed Roots now adds configuration-bound startup
demand and selected execution reserves to reviewed native funding. Existing
top-ups retain one Ledger fee; deferred provisioning drops its increment and
terminal replay does not refill a startup allowance. Focused planning evidence
also covers source mismatch, typed demand rejection and exact plan identity.
Fresh continuation now prepays startup funding through its initial Create/Fund
actions, using Root-local artifact/import/retry bounds. The generated-estate
and ten continuation regressions pass, followed by the exact IC withdrawal,
response-loss, conservation and replay case. Runtime admission, startup planning
and supplementary native reviews now share the unchanged 1T deployment floor;
five startup and seven native funding cases pass. The protected initial-child
origin also reaches the Coordinator in a passing IC E163/same-claim recovery
case, now including the production host observer and selected-operator transport.
Recovery quotes also preserve the selected Root's bootstrap request threshold,
with missing/duplicate authority and overflow rejection. The generated-estate
case, seven withdrawal regressions, host-observer IC case and scoped host/internal
Clippy pass. See the [child report](../../audits/reports/2026-09/2026-09-14/canic-172-child-reserve.md).
The combined E163 initial-child/native-withdrawal journey also passes, retaining
the same claim through two lost responses, terminal conservation and replay
(`/tmp/canic-native-child-ic.log`). Its local Ledger stub uses zero fees.
Generation now projects live Coordinator window spend/reservations, successful
automatic usage and pending Root operations through its existing protected query.
Exact selected code, operator, Root set and policy are required; unavailable
observations never imply an unused budget. Runtime window admission also rejects
accounting overflow. Seeded direct-child observations are now qualified above;
recursive child coverage, recovery-quote integration and exact operator mint
receipts remain open before B1.
The four usage/transport cases, six runtime policy cases, generated-estate replay
and CLI rendering regression pass, as does scoped all-target/all-feature Clippy
for core, host and CLI. The funding report records qualification limits.
Toko's latest follow-up confirms CANIC-174, agrees with these priorities and
adds no issue ID.
The .17 draft is updated; the complete accepted batch is not push-ready.

## CANIC-174 grant deadline checkpoint — 2026-09-14

The IC owner-path regression confirms the unchanged deadline after a real
child grant crosses the Root reserve. The cycle owner now resamples transfer
settlement, advances the existing timer and preserves earlier safety checks
without interpreting transfers as computation burn. Final IC qualification,
scoped Clippy, formatting, layering and inventory/source guards pass. The [report](../../audits/reports/2026-09/2026-09-14/canic-174-funding-deadline.md)
records the baseline failure, behavior and limits. Bootstrap forecasts and the
remaining CANIC-172 funding/recovery work remain part of the open .17 batch.

## CANIC-172 native funding checkpoint — 2026-09-14

The current funding owner now reviews native Root supplementation against an
issued provisioning action, with exact management/Ledger authority, separate
approval, fixed withdrawal identity, receipt recovery and original-balance
conservation. Unapproved quotes can refresh; approved intents cannot. Eleven
host tests, one CLI report case, scoped Clippy and source guards pass. The
[report](../../audits/reports/2026-09/2026-09-14/canic-172-native-funding.md)
records the current tagged pause shape and evidence limits.

The composed issued-operation IC withdrawal case now passes both lost-response
boundaries, exact deposit/debit accounting, terminal conservation and replay.
Its synthetic high host minimum does not reproduce the E163 low-reserve child
claim; the subsequent combined claim/withdrawal case above now closes that gap.
Exact operator mint-credit receipts and live grant/reservation forecasts remain;
CANIC-174 owner-path qualification is recorded above. The shared estate-pause IC regression, final scoped
Clippy and inventory/source guards also pass. Earlier interruption/preview
evidence and B1 are still open.
The .17 changelog reflects this step; packages remain .16 and the whole batch
is not push-ready. No publication, deployment or live funding ran.

## CANIC-172 child reserve checkpoint — 2026-09-14

The open .17 draft distinguishes the deployment reserve rejection as E163 and
retains bounded child-allocation failure evidence with capped retry backoff.
Native persistence/capacity/progress cases, scoped Clippy and the governed
low-reserve IC claim/readiness/replay proof pass. The
[report](../../audits/reports/2026-09/2026-09-14/canic-172-child-reserve.md)
separates this correction from outstanding operator funding review, exact
mint/withdrawal/fee accounting and bootstrap demand forecasting. CANIC-174 and
the earlier interruption/preview evidence remain ahead of B1. The .17
changelog is updated; the complete batch remains open and unpublished.

## CANIC-175 state cascade — 2026-09-14

The open .17 draft now routes snapshots through each recipient's owned command,
reports bounded partial outcomes and reconciles Root funding before fanout.
The final focused IC case passes role propagation, authority rejection, stopped
Shard failure, retry and Readonly/Stopped restoration while unrelated commands
remain fenced. Native report/Root tests, Candid equality, scoped Clippy, default
feature protocol tests and layering pass. The
[report](../../audits/reports/2026-09/2026-09-14/canic-175-state-cascade.md)
records the hard cut and synthetic-fixture limits.

Prioritize CANIC-172 native funding and CANIC-174 timing/forecast evidence next;
positive-credit interruption and richer reserve previews remain. B1 expansion
stays behind this feedback. The full release batch remains open, with .16
packages and the updated .17 draft; no Git publication or deployment ran.

## B1 recovery-dispatch measurement — 2026-09-14

Row 6 passes the complete twelve-artifact matched measurement on immutable
`v0.110.5`: two byte-identical baseline builds and two byte-identical candidate
builds per artifact. Canonical artifact sums fall by 1,242,984 code bytes and
2,297 functions in the destructive audit variant. The
[report](../../audits/reports/2026-09/2026-09-14/b1-recovery-dispatch-measurement.md)
retains identities, vectors, tooling correction and limitations. This supplies
attribution, not recovery parity or a production deletion. Rows 8/10/12 and
remaining B1 evidence are still open. The post-run feedback refresh advances
to CANIC-175. CANIC-173's configured inventory projection is fixed and passes
focused publication/replay/authority regressions and host Clippy. The
[new feedback report](../../audits/reports/2026-09/2026-09-14/toko-recovery-172-175.md)
sequences CANIC-175 state-cascade correction, CANIC-172 native funding review
and CANIC-174 timing qualification ahead of further contraction measurements.

## Source-bound recovery qualification — 2026-09-15

CANIC-166/171's [focused IC proof](../../audits/reports/2026-09/2026-09-15/canic-166-171-activation-reset.md)
now passes: real Published Components/Prepared Root/Store identity conflict,
source-bound review and exact archives, wrong-digest rejection, positive Stop
credit with unchanged starting balances, lost Stop and Root reinstall responses,
and both effect-free replays. Exactly one Root reset occurs. Scoped internal
Clippy and catalog qualification pass. The proof ends at the reset prerequisite;
it does not establish live staging recovery or subsequent full convergence for
that exact conflict. Exact mint accounting, child-grant forecasts and the
whole-recovery funding preview still block complete .17 readiness. Auth/E9 is
parked at the maintainer's request; no runtime authority was relaxed.

## Recovery feedback implementation — 2026-09-14

The open .17 draft now corrects CANIC-166 staged-review apply selection,
CANIC-171 bounded observed Stop-credit accounting and CANIC-170 operation
identity binding. CANIC-156 gains an exact liquid-cycle admission diagnostic,
qualified through the real Root inspection path in one focused PocketIC case.
Native recovery/identity/conservation, CLI, diagnostic-register and scoped lint
checks pass. The [follow-up report](../../audits/reports/2026-09/2026-09-14/toko-recovery-followups.md)
records scope, source identities, tests and limitations.

The source-bound positive-credit interruption proof is now qualified above.
The full batch remains open for richer funding accounting/preview work; B1
matched measurements remain subsequent accepted work. Available downstream
items already have shipped Canic implementations.
Package versions remain .16; the .17 changelog includes the required actual
conservation field and E162. No Git publication, deployment or sibling edit ran.

## Post-release continuation — 2026-09-14

`v0.110.16` is tagged at `a875c6498721bd89ca98549e38389b8280910d68`;
the maintainer reports publication complete and explicitly continues 0.110.
The completed OP1–OP4, memory and recovery checkpoints below are historical
development evidence for that release. Their former open-.16 wording does not
describe a new draft. The
[initial feedback review](../../audits/reports/2026-09/2026-09-14/toko-upstream-feedback.md)
initially reached CANIC-169 and separated shipped implementation from
remaining downstream qualification. Toko's working lock already selects .16.

No patch is allocated just for this reconciliation. Remaining accepted B1
attribution continues within 0.110; necessary correctness follow-ups stay on
the affected line despite the advisory twelve-release guideline. This does not
open 0.111, accept B1, or unpark role-specific stable initialization.

## B1 qualification checkpoint — 2026-09-14

The subsequent requested dependency update pins both maintained IcyDB fixture
graphs to 0.257.12, with SQL disabled and shared ic-memory 0.13.3. All seven
focused lifecycle/import PocketIC cases and both fixture/schema Clippy groups
pass without Rust source changes. The open .17 draft includes this update;
historical ablation identities and verdicts remain unchanged.

Row 8 now passes the complete fourteen-artifact selector on immutable
`v0.110.5`, with its original patch and restored source/lock. Historical patch
preflight is repaired to use the frozen tree instead of the evolving method
checkout. The [report](../../audits/reports/2026-09/2026-09-14/b1-row8-qualification.md)
preserves exact evidence and limits. The .17 changelog draft records the audit
tooling change; package versions remain .16. The later row 10/12 qualifications
below pass; B1's matched measurements remain due. This is not a full-batch
push-readiness handoff.

## CANIC-139 follow-up — 2026-09-14

The later downstream refresh identifies a confirmed reuse-verifier defect in
published .16: newly recorded absent Cargo inputs beneath scanned directories
were reported as changed source. The open .17 correction admits only absence
proved by the pre-build scan, sharing its exact directory exclusions. Real
changes and unobserved inputs still reject reuse. All 12 scoped reuse cases
and host all-target/all-feature Clippy pass; the
[report](../../audits/reports/2026-09/2026-09-14/canic-139-absent-inputs.md)
records the real first-release Cargo reproducer and limits. This completes the
bounded correction without accepting B1 or claiming downstream staging success.

## Promoted operator batches — 2026-09-13

The maintainer explicitly requests CANIC-010/008/002/017 in 0.110 now. Their
former idea deferrals are superseded by the
[promoted design](0.110-design.md#promoted-downstream-operator-batches).
No further promotion ceremony or minor closeout is required to implement this
accepted sequence. The previously completed .16 corrections remain preserved;
the expanded delivery request is complete and ready for the governed release
flow with its direct evidence and propagation recorded below. No new patch
number is allocated; versions remain .15 and the open draft is .16.

| Batch | Outcome | Owner | Included evidence / validation | Status |
| --- | --- | --- | --- | --- |
| OP1 / 010 | Reviewed ordinary Component lifecycle | Host workflow and CLI; existing Root operation | Exact authority, crash persistence, lost replies, activation, capacity/caller rejection, terminal replay and packaged CLI; focused native/PocketIC/Clippy | Ready in open .16 |
| OP2 / 008 | Verified frontend environment/binding handoff | Host export and CLI | Nonterminal/stale/tampered input rejection, bounded public output, local trust and independent frontend consumer | Ready in open .16 |
| OP3 / 002 | Supported bounded Fleet observatory | Host protected-status projection and downstream adapter | Freshness, provenance, partial/failing roles, bounds and measured runtime/host cost | Ready in open .16 |
| OP4 / 017 | Persistent multi-subnet local Fleet | Host developer process and public test primitives | Two Roots, browser gateway/discovery, restart/time/reset/shutdown, resource bounds, packaged consumer | Ready in open .16 |

The maintainer explicitly chooses this existing minor despite the advisory
12-release guideline. Implementation batches are not individual patch releases.
Existing 0.110 contraction work and the eventual human-owned minor closeout
remain separate; this promotion does not start 0.111.

### OP4 implementation checkpoint

The subsequent integration audit is fixed and qualified: additional imported
Root-owned assets participate in terminal discovery without becoming named local
allocations. The extended two-Root case passes with ordinary Component retry,
explicit simulated Ledger creation/Root import, export and another restart
(215.67s; 231s runner). Local automatic refill remains disabled. Exact scope and
limits are in the [local Fleet guide](../../features/operations/local-development-fleet.md#qualification).

The optional public host `local-fleet` feature and packaged foreground consumer
are implemented. Preparation binds exact local release/workspace/session authority,
preallocates through the bundled Ledger, initializes Roots with the maintained
initializer and delegates convergence to ordinary Fleet Ensure. Current role/subnet
discovery and sealed Candid feed the existing frontend handoff. Exact environment
binding fixes the direct ICP adapter's former literal-`local` restriction.

The two-Root acceptance case passes in 190.34s (238s runner), with two application
placements, authenticated SDK calls before/after restart and immutable frontend
bundle reuse. All 4,126 recorded source files stayed unchanged. Four lifecycle
cases pass in 22.32s, including lost Ledger responses, capacity/ownership rejection,
time/trust, interrupted/terminal reset and late orphan-write isolation between
session directories. Logs: `/tmp/canic-op4-public-fleet-final.log`,
`/tmp/canic-op4-lifecycle-generation-final.log`. The direct ICP environment
regression and all 20 generator cases pass; final host/library/test/example and
internal governed-fixture Clippy pass (`/tmp/canic-op4-transport-native.log`,
`/tmp/canic-op4-generator-native.log`, `/tmp/canic-op4-final-targets-clippy.log`).
The host library also compiles without the optional feature, and the internal
library/tests compile in their ordinary default feature selection
(`/tmp/canic-op4-host-default-check.log`, `/tmp/canic-op4-fixture-default-check.log`).
Layering, current-document semantics (zero layout warnings), formatting and
whitespace checks pass (`/tmp/canic-op4-layering.log`, `/tmp/canic-op4-docs-final.log`).

The actual Cargo host archive contains 280 Rust files identical to workspace
source. Its extracted consumer builds against exact current local Canic path
dependencies and unchanged external lock identities, then passes allocation,
duplicate requests, time advance, restart, shutdown/reopen, terminal reset and
old-reset rejection through public JSON commands in 21.12s. Evidence:
`/tmp/canic-op4-package-evidence.json`, `/tmp/canic-op4-package-build.log`,
`/tmp/canic-op4-consumer-result.json`. The archive/startup and full Fleet proofs
are separate; no published adoption or mainnet fidelity is claimed. After the full Fleet run, source changes are limited to a pure invalid-Fleet-label
rejection and formatting. Final scoped lint and archive compilation cover those
changes; native and lifecycle evidence remains recorded separately above. OP4 is ready in
open .16. See the
[local Fleet guide](../../features/operations/local-development-fleet.md).

### OP3 implementation checkpoint

The host snapshot, role-specific transport, bounded process runner, public
projection/HTML/HTTP adapter and CLI are implemented. Current Store metadata
counts supplement protected byte accounting. The shipped Store Candid was regenerated
and the structural runtime/status equality regression passes
(`/tmp/canic-op4-canonical-test.log`). Local journal progress remains
visible without treating an interrupted Fleet as terminal authority. The
[observatory guide](../../features/operations/fleet-observatory.md) owns the
contract, budgets and limits. The live case passes in 116.10s (132s runner), with eight queries, 7,670 private
bytes and 1.59s for private/public/partial collections. Exact funding/Store
responses, privacy and independent Store failure all pass. Store inventory,
CLI help, layering and documentation checks pass. Final seven native cases, scoped Clippy, formatting and package inventory pass
(`/tmp/canic-op3-native-complete.log`, `/tmp/canic-op3-clippy-complete.log`,
`/tmp/canic-op3-canic-host-package.txt`, `/tmp/canic-op3-canic-cli-package.txt`).
The package lists include ten host observatory files and the CLI adapter, with
no example dependency installation directory. OP3 is ready in open .16. Those
OP3 package lists are inventory evidence; the later OP4 host archive/startup
qualification above remains distinct from publication/adoption. Logs: `/tmp/canic-op3-pocketic-qualified.log`,
`/tmp/canic-op3-inventory.log`, `/tmp/canic-op3-help.log`. The guide records
Store artifact cost and the exact limitations.
No runtime rendering, global selector, controller bypass or sibling edit is added.

### OP2 implementation checkpoint

The host/CLI now generate and verify exact browser manifests and bindings,
retain selected admission origins in Fleet generation, export enrolled local
trust and inspect native asset cycles against explicit payload/funding bounds.
The [frontend guide](../../features/operations/frontend-handoff.md) owns the
contract and external asset/identity responsibilities. The current II limit is
100 alternative origins, correcting the older feedback's ten-origin assumption.

Twelve focused native/Node cases pass, including independent SDK digest parity,
tampering and trust isolation (`/tmp/canic-frontend-tests-final.log`). Generated
TypeScript now also compiles against SDK core 5.4.0/auth 8.0.3 and TypeScript
6.0.3 (`/tmp/canic-frontend-sdk-types.log`). Admission-origin generation and CLI
help pass (`/tmp/canic-frontend-generation.log`, `/tmp/canic-frontend-cli-help.log`).

The exact public CLI/SDK Fleet case passes in 148.27s (174s runner). It covers
fresh convergence, local trust, generated declarations, the exact admitted user,
a denied user, missing-journal/stale-sidecar/origin rejection and typed native
capacity failure (`/tmp/canic-frontend-pocketic.log`). Its frontend phase takes
4.01s and its complete one-role bundle is 136,858 bytes. No Internet Identity UI
ceremony, real asset upload, Toko adoption or published-package adoption is
claimed. Initial attempts corrected fixture TOML/discovery and a transparent
error assertion; they did not justify weakening runtime checks. Final host/CLI/internal-fixture/leaf Clippy, layering, documentation and package
inventories pass (`/tmp/canic-frontend-clippy-final.log`,
`/tmp/canic-frontend-docs.log`, `/tmp/canic-frontend-canic-host-package.txt`,
`/tmp/canic-frontend-canic-cli-package.txt`). OP2 is ready in open .16. All four promoted batches are now complete; their separate evidence is recorded above.

### OP1 implementation checkpoint

The subsequent integration correction retains original Component provenance
across a new no-op Ensure review and includes ordinary allocations in terminal
Fleet inventory. Exact live authority still rejects drift. The extended OP4 case
proves this after fresh convergence, including a lost accepted reply and one
submission. Ten Component, six export and 200 Fleet Ensure native cases pass
(two existing ignored); affected host/CLI/internal-fixture Clippy passes.

The host and public CLI implement `component plan`, `apply` and `status`, with
one durable operation ID, exact selected network/review/controller/release/Spec
bindings, atomic intent before submission, bounded polling and terminal local
replay. `info env --component-operation <name>` refreshes completion and replaces
an old Ready-pool export row. The [operator guide](../../features/operations/component-operations.md)
owns usage and recovery semantics. Ten native recovery/authority cases, six
export cases and recursive CLI help pass at their recorded checkpoints.

The first runtime case found and corrected resume-after-restoration: apply now
replays the same Root command once, then polls; Root coalesces scheduling for
that operation. It passes in 100.01s (159s runner), including denied caller, lost
reply, same-release Root restoration, duplicate resume, one additional Component
and terminal replay without calls or cycle changes (`/tmp/canic-op1-pocketic.log`).

The public CLI/production ICP case passes in 23.40s (38s runner), using an
isolated identity, a real management gateway, current endpoint-checked Candid,
lost accepted response, terminal binding and exact JSON export. Tampered Candid
rejects and completed replay succeeds without a transport executable. Log:
`/tmp/canic-op1-public-cli-pocketic.log`. Its explicit terminal starting fixture
and synthetic IC identity do not claim a full Ensure convergence or immutable
published-package adoption. Earlier failed attempts were corrected fixture
extraction/gateway/discovery setup, separate from the first case's runtime fix.

Final native recovery (10), artifact-sidecar (2), governed catalogue, scoped
Clippy, layering and document checks pass. Package file inventories include the
complete host owner and CLI command. Logs: `/tmp/canic-op1-native-final.log`,
`/tmp/canic-op1-artifact-tests.log`, `/tmp/canic-op1-catalogue.log`,
`/tmp/canic-op1-final-clippy.log` and `/tmp/canic-op1-docs-final.log`. OP1 is ready
in the open .16 draft. All four promoted batches are now complete and ready
for the governed release flow. Existing package versions and external
repositories are unchanged.

## Current upstream follow-up

CANIC-168 is a completed, separate host build-diagnostic batch in the open .16
draft: compiler startup is checked before using an implicit cache; explicit
wrappers, original failures and one-shot Cargo execution remain intact. Forty-
five build-owner and 14 bootstrap-related tests plus scoped host Clippy pass.
The memory batch below remains complete. Role-specific memory initialization
is [parked under ideas](../ideas/role-specific-stable-initialization/design.md).

CANIC-166's journal-entry correction uses the existing source inspector before
explicit reset admission and diagnoses ordinary journal failures through that
same owner. A private read-only projection verifies an already Applied bootstrap
receipt without fixture metadata; it cannot create an executable action. All
192 native Fleet Ensure tests pass, and a copy of Toko's exact 30-row source
reaches the recovery diagnostic with all 17 source files unchanged. The current-
release PocketIC reinstall journey passes in 666.84s before that final receipt
projection; the native/copy proofs qualify the latter. The obsolete `.12` executable harness is now retired; its original immutable
proof report remains historical. Current-runtime recovery and source/adoption
qualification stay maintained. Live staging effects remain separate.

CANIC-014's visible version/source/date/gate summary is now generated by the
governed bump, with the preserved human handoff explicitly predating that
transaction. All 16 release-flow tests, scoped Clippy and ShellCheck pass.
Both corrections extend the existing open .16 draft; see the
[current handoff](../../status/current.md).

Before OP1–OP4 were added, the completed in-repository corrections and
changelog were ready for the governed release flow. Final host Clippy, source-entry regressions and lightweight
ownership/document checks pass. Package versions remain .15; no version or
publication action ran. CANIC-141 now rejects ambiguous mixed-subnet creation
through the feedback's explicit fail-closed alternative, including retained
apply and mixed-Root funding. All 197 native Fleet Ensure cases and affected
host/testing Clippy and native governed-catalogue validation pass. Toko's live recovery/conversion and the four larger
product requests remain separate; see the current triage for their owners and
release-boundary constraints. No new minor is started.

## Stable-memory follow-up after 0.110.15

The maintainer authorized allocation reduction and the reviewed consolidations
on 2026-09-13 after the .15 push completed. The single open .16 batch is owned by
Canic core/control-plane storage, with host state-manifest and runtime-fixture
propagation. It adopts ic-memory 0.13.3, configurable 1 MiB buckets, four bounded
singleton cells, small provisioning-map pages, and receipt/shard consolidation.
Template payload and metadata merges are rejected on measured access costs and
independent lifetimes. Native checks, scoped Clippy and all nine cases in the
three focused PocketIC targets pass with unchanged source inputs. The complete
.16 batch and changelog are ready for the maintainer's normal release flow. See [layout and inventory](../../features/runtime/stable-memory-layout.md)
and [current handoff](../../status/current.md). This necessary allocation
follow-up remains on the affected minor despite the advisory release-count
threshold; no new minor, closeout, version or publication is authorized.

The same batch also avoids loading template chunk bodies for GC counts, using
reference slots and vector length. Eleven focused native chunk/publication
tests and scoped control-plane Clippy qualify this later metadata-only change;
the preceding PocketIC evidence retains its recorded snapshot. Stable layout
and the 1 MiB default remain unchanged, and the complete batch stays ready.

## Earlier validation throughput before the .14 push

The maintainer's pre-push VS1 outcome is qualified at the recorded ic-memory
0.13.1 checkpoint. BF2-BF4 and the other feedback corrections retain their scope.
Published IcyDB 0.257.4 now shares ic-memory 0.13.2, so the maintainer requested
adoption and CANIC-162 is restored. Fresh aligned lifecycle/allocation PocketIC
proofs, eight memory cases, three ABI guards, 15 timer guards, the catalogue
regression and affected-target warning-denied Clippy pass. The complete scoped
.14 batch and changelog are ready for release review; package versions remain
0.110.13.

| Batch | Outcome | Owner | Included evidence | Validation | Status |
| --- | --- | --- | --- | --- | --- |
| VS1 | Reduce dominant release-test work while preserving distributed invariants | Internal fixture/runner and host observation owners | Phase attribution, exact artifact reuse, native CLI selection, authority/retry/conservation and final-source timing | Both dominant exact PocketIC journeys, 23 host cases, timing/catalogue regressions, scoped lint and runner guards | Qualified; same open 0.110.14; complete release gate remains maintainer-directed |

The [final report](../../audits/working/0.110-validation-throughput/report.md)
records 1,153.72s for mixed topology plus two resets and 575.12s for retained-estate
recovery. Both run with an unchanged 1,686-file source inventory. All 144 duplicate
balance calls disappear; fresh authority, recovery and replay checks remain.
Native regressions and warning-denied all-target/all-feature host/internal-fixture
Clippy pass. Both test invocations clean their owned servers and scratch.

The [VS1 amendment](0.110-design.md#validation-throughput-amendment-vs1)
retains serial Cargo and PocketIC ownership. Existing verified artifact reuse
already works across invocation roots; sub-second replica setup does not justify
new mutable fixture pooling or catalogue splitting. This measured disposition
completes the artifact/setup slice without a speculative runtime-identity change.

The [historical baseline](../../audits/working/0.110-validation-throughput/baseline.json)
records a 6,322s older runner. Earlier attribution and final runs differ in
memory dependencies and cache warmth, so no isolated whole-journey percentage,
new complete-gate duration or sub-hour result is claimed. The open changelog
records the completed scope; no version or publication ran.

## Build feedback 087 and 139

The maintainer accepted these build-throughput follow-ups on 2026-09-09.
BF1 shipped in 0.110.13. The host build owner now separates
declarations from runtime linking, batches package-bound protocol contexts and
verifies complete unchanged-build reuse before allocating a release identity.
Changed inputs still rebuild the runtime set because every artifact embeds the
complete release identity; cross-identity role reuse is outside this scope.

| Batch | Outcome | Owner | Included evidence | Validation | Status |
| --- | --- | --- | --- | --- | --- |
| BF1 | Faster governed builds and verified no-op reuse | Host artifact/release owners and build macro | Input/output invalidation, exact role binding, Candid/features, managed lifecycle and measured cost | Focused native checks, controlled release comparison and scoped PocketIC evidence | Complete within recorded scope |
| BF2 | Overlap infrastructure finalization with subsequent compilation | Host artifact owner and CLI build orchestration | Exact captured inputs, output/provenance parity, failed-build cleanup and valid retry | 53 host cases, 25 CLI cases, affected-package Clippy and controlled two-artifact comparison | Complete; open 0.110.14 |
| BF3 | Admit unchanged first builds without confusing Cargo inventory changes with source drift | Host build reuse and generated-package owners | Infrastructure source coverage before compilation, stale-record replacement, real source changes and unknown-input refusal | 58 combined focused host cases and affected-package Clippy | Complete; same open 0.110.14 |
| BF4 | Reuse unchanged compiled declaration extraction across backend edits | Host artifact and Candid extraction owners | Role/shared-input changes, corrupted records, exact native tool identity, failed extraction and real-tool byte parity | Focused build regressions, six-role extraction comparison and affected-package Clippy | Complete within recorded scope; same open 0.110.14 |

The maintainer selected changed-input reuse on 2026-09-10. Every runtime's
embedded complete release identity is itself a changed dependency, so BF4
preserves that contract and reuses the independent declaration extraction
stage. Cargo remains the dependency owner; there is no second source inventory
or per-role cache claiming runtime reuse across identities. A six-role real-tool
comparison reports 1.922s fresh extraction versus 0.087s verified reuse with exact
Candid parity (2.234s first cache population). Complete-build speedup and
cross-identity runtime reuse remain unclaimed.

The [build feedback report](../../audits/reports/2026-09/2026-09-09/build-feedback.md)
owns BF1 qualification and its limits. Its unchanged replay is 2.83s; a warm
one-role edit improves 477.10s to 291.03s; cold time is roughly unchanged.
The managed/standalone lifecycle case passes. Mixed-topology startup, conservation
and replay assertions completed before the extra reset exercises were stopped;
the complete case is not marked passed. Precise per-role/cross-identity reuse,
material cold improvement, full clean determinism and downstream qualification
remain outside this completion claim. Earlier evidence below retains its scope.

After publishing 0.110.13, the maintainer prioritized another bounded build-speed
batch. BF2 retains serial Cargo and existing release settings while captured
Coordinator and Store outputs finalize in scoped workers. The controlled
comparison improves two-artifact wall time from 112.89s to 85.03s (24.68%);
Wasm, gzip, Candid, protocol hashes and transform provenance match exactly.
Configured-admission failure drains workers without staging leftovers, and a
following valid request with the same release identity reproduces the artifacts.
The [scheduling report](../../audits/reports/2026-09/2026-09-09/build-pipeline.md)
owns the evidence and its limits. BF2 and its open 0.110.14 notes are complete;
VS1 is qualified; restored CANIC-162 shares the published IcyDB memory
runtime and passes fresh qualification. The .14 batch is ready for release review.
Package versions remain 0.110.13. No full downstream, broad
workspace or aggregate-memory qualification is claimed. CANIC-087/139 stay
partial and CANIC-141 stays deferred. Continue this maintainer-selected feedback
on the published 0.110 line despite the soft release-count guideline, rather
than allocate a new minor for the bounded follow-up.

The maintainer then selected CANIC-139's first-build refusal for BF3. Current
downstream dependency records name 143 control-plane files outside the initial
App package scan. Reuse now includes the exact generated-infrastructure family
roots before compiling, retains file-level evidence through Cargo record
replacement, and records the verified final inventory key. Real mutations and
unobserved external inputs still fail, with distinct path-specific diagnostics.
The [first-build report](../../audits/reports/2026-09/2026-09-09/build-reuse-first-build.md)
records the tiny real Cargo/Wasm regression and its limits. BF2/BF3 remain one
completed 0.110.14 batch; full downstream qualification and changed-input
per-role reuse remain outside its completion claim.

## Sampling cost reference clarification

On 2026-09-10 the maintainer directed that the unexplained 20M instruction
threshold must not become a future blocker. Canic now reports it as an advisory
reference and keeps structural, relative source-growth and recovery checks.
The [cost policy record](../../audits/reports/2026-09/2026-09-10/metrics-cost-policy.md)
links the maintained evidence requirements for any future absolute threshold.
This is an open 0.110.14 test/documentation correction, with no runtime budget
or protocol change. A matching Toko test patch is prepared but unapplied.

## Additional downstream qualification

CANIC-159's corrected runtime now passes the existing five-component PocketIC
journey with a Store outage during Root Accepted. Exact origin fields reach
Coordinator; same-operation recovery clears failures and immediate replay
requires no updates. The [runtime report](../../audits/reports/2026-09/2026-09-10/canic159-runtime.md)
retains the candidate evidence separately from downstream adoption.

The maintainer's four-item Toko Miner request extends open 0.110.14 with
corrections for Root Accepted-phase provisioning origins, optional public
sampling after rejected producer data, and metric history lookup cost. The
[four-item report](../../audits/reports/2026-09/2026-09-09/toko-feedback.md)
records disposable retained reset/data/replay, a controlled downstream first
build, and an unapplied staging preparation review. The corrected history index
passes the real metrics ceiling through 300 periods and rejection/recovery, with
19,963,567 peak instructions against 20M (0.18% headroom). Downstream runtime
adoption and staging recovery remain explicit acceptance boundaries. The
approved wrapper cleanup is applied. No protocol generation,
compatibility layer, version bump or new minor is introduced.

## Activation feedback 157–160

The maintainer accepted these follow-ups with an explicit sanity and drift
review. Existing Ensure, provisioning, protected-status and observation owners
contain the changes. Root prerequisite identity and dependent Store admission
now reject incompatible reuse; bounded retry state and originating failures
remain in the existing operation. Observation timing and bounded independent
reads retain the existing snapshot lifetime.

Installed-source queries on 2026-09-09 confirm a sealed initial inventory and
all 24 pool assets. Source evidence inspection does not make the old plan
executable or supersede its journal. Refill-intent admission now rechecks full
physical capacity after fee discovery, with 24 targeted pool cases passing.
Lease takeover prevents using one successful maintenance pass as settlement
proof. The single-Root preparation, Root reset and Full Ensure reviews now use
the existing effect driver. Exact source archival and crash recovery precede
the first remote effect; admission binds source modules/controllers, complete
assets, unchanged Ledger accounts and bounded source debit. The immutable
`v0.110.12` PocketIC journey passes controller-drift rejection without source
replacement, stop/restart preparation, lost Root install response recovery,
Full Ensure readiness, exact retained assets, conservation and effect-free
replay. The proof also corrected changed-Wasm history reconciliation by binding
both the prior and requested module. Three history regressions, 167 direct
Fleet Ensure cases and focused host/CLI/fixture Clippy pass.

The [activation feedback report](../../audits/reports/2026-09/2026-09-08/activation-feedback.md)
owns the exact evidence and single-Root applicability boundary. The complete
accepted 0.110.13 source batch and changelog are ready for push/release review;
package versions remain 0.110.12. Live downstream adoption remains separate.
Earlier OD1/RI1/RF2 results retain their completed scope.

## RF2: Accepted upstream follow-ups 151–156

The maintainer accepted all six follow-ups on 2026-09-08, extending the same
0.110.13 batch after OD1/RI1. The launcher is aligned and qualified. Retained
estate omission rejection, affordable continuation reserves/prefixes and typed
dependent-funding review are implemented. The compiled-configuration fixture
boundary and matching-build recipe are explicit. Bounded public process, timer
and memory projections use the existing five-family sampler and history.

The complete accepted OD1/RI1/RF2 batch is ready for push/release review.
Qualification passes: 167 host and 51 CLI cases, 18 all-feature metrics cases,
all eight timer PocketIC cases and the expanded 27-canister changed-release
journey. The latter retains all 24 pool assets, rejects sixteen omitted imports
before Root reset, reviews six low-balance top-ups, recovers lost install and
funding responses, reaches 19 Workloads plus five Ready reserves, checks exact
debit/controllers/conservation, and replays without effects. First/repeat/full
scheduled public sampling remains below 20 million instructions. The open
0.110.13 changelog is ready; package versions remain 0.110.12.

CANIC-157–160 appeared in a later tracker refresh and are accepted separately
above; RF2 does not claim their completion.

## RI1: Explicit Same-Release Fleet Reinstall

Accepted by the maintainer on 2026-09-08 after OD1. CANIC-149 adds one explicit
operation-scoped `fleet ensure --reinstall` intent. Preparation durably seals
Coordinator/Root allocation through the existing authority-snapshot owner;
the following reviewed full plan resets the same-Wasm infrastructure and the
complete sealed Root-owned pool, then reconstructs application fixtures and
full Fleet readiness. Both phases retain the same operation identity. The
network, controlled Principal set and cycle accounts remain controlled;
logical pool role assignments may change. No permanent desired-state flag,
new scheduler, cross-release compatibility or downstream executor is added.

Host policy/ops/workflow, CLI, the existing authority inspection boundary and
governed PocketIC fixtures own one complete batch. Required evidence covers
controller/closure rejection, real Hub/Shard user-data erasure, fixture restore,
interruption before/after effects, a second deliberate wipe, conservation and
effect-free replay. Includes Candid/fixture/docs propagation and cleanup.
Implementation, recovery evidence, propagation and cleanup are complete.
The production-adapter journey passed two real database wipes, Coordinator
before/after-response interruptions, a Root interruption after ordinary version
advancement, retained estate/controller/cycle-account checks, conservation and
both terminal/ordinary-ensure replays. Targeted native/interface checks and
warning-denied affected-package Clippy pass. The combined OD1/RI1 batch is ready
to push and the open 0.110.13 changelog is ready for release approval; package
versions remain 0.110.12. Publication and deployment are separate.

## OD1: Downstream Operator Diagnostics

The 2026-09-08 request to work on Toko Miner upstream feedback selects reopened
CANIC-042 and CANIC-150 as one bounded diagnostic correction. Host role evidence
owns actionable Cargo causes; CLI Medic and Fleet rendering own advice, named
budget terms, compact cycle values and action descriptions. Positive and invalid
evidence includes nested offline failures, source/credential sanitization,
bounded Unicode, pending/unknown funding observations, initial creation versus
Root-funded capacity, reinstall and no-op presentation, and unchanged JSON and
progress behavior. Targeted host/CLI tests and affected-target Clippy own
qualification. OD1 is complete; the maintainer subsequently extended the
0.110.13 batch with RI1 above. Its completed evidence is: five host evidence cases,
thirteen Fleet CLI cases in the combined batch, the Medic advice case and all-target/all-feature Clippy
for both affected packages pass. Downstream live-output adoption remains separate.
Propagation includes the Fleet operations contract, changelogs and
[feedback handoff](../../audits/reports/2026-09/2026-09-08/operator-feedback.md).

Extend the affected 0.110 line with the 0.110.13 draft despite the soft release
count guideline: these correct published diagnostics and complete the accepted
reinstall operator workflow. CANIC-149 is complete as RI1 above. CANIC-141 stays
deferred.

## PM1: Bounded Public Sampling

Complete and ready for release approval after maintainer-pushed 0.110.10.
CANIC-148 corrects optional public
collection through bounded owner projections, independently attempted families
and unchanged cycle-tracking ownership. Includes native bound/identity/failure
regressions, a small IC sampling-cost and cycle-tracking proof, documentation
and the open 0.110.11 changelog. No history task or new scheduler. CANIC-147
is a separate capability request awaiting contract acceptance; CANIC-141 remains
deferred. Native regressions (112 all-feature, 10 default), 17 storage/timer guards,
affected-target Clippy and seven PocketIC cases pass. The focused PocketIC runner
took 96 seconds including artifact rebuilds. Sampling 256 versus 4,096 recorded
checkpoints took 4,449,546 versus 4,647,992 instructions. The
[sampling handoff](../../audits/reports/2026-09/2026-09-08/public-sampling-bounds.md)
records bounds, evidence and downstream qualification. Package versions remain
0.110.10; the open 0.110.11 changelog is ready. Release execution remains
maintainer-selected.

## RF1: Retained Fleet Feedback

Ready for release approval, selected by the maintainer after CR1 on 2026-09-07. Extends the open
0.110.10 draft with CANIC-143, CANIC-144 and the CANIC-125 observation follow-up.
Host policy, transport and the existing journal remain the owners. The outcome
is ordinary convergence of installed underfunded imports, symbolic retained
Root restart and bounded initial-child activation without application retries.
Required evidence: exact lifecycle/controller rejection before funding, retained
withdrawal receipts across lost replies, generated symbolic restart through
reviewed reinstall, conservation/replay, initial-child demand bounds and no
duplicate issuance. Reuse the existing focused production-adapter journeys.
The implementation, propagation and focused qualification are complete:
150 Fleet host tests, affected-package Clippy, installed-import recovery
(310.66s) and generated symbolic-seed reinstall/recovery (393.03s) pass.
Both runtime cases include conservation and effect-free replay. The
[feedback handoff](../../audits/reports/2026-09/2026-09-07/retained-fleet-feedback.md)
records the final boundaries. CANIC-141 remains deferred. The combined CR1/RF1
draft is ready; release execution and downstream live timing remain separate.

## CR1: Canonical Fleet Subnet Root

Ready for release approval, explicitly selected by the maintainer on 2026-09-07
after 0.110.9.
This is the next accepted batch in the current minor, ahead of further downstream
feedback; CANIC-141 remains deferred. The canonical Root build, package rename,
configuration and lifecycle hard cut, fixture/documentation propagation and
focused fresh/reinstall/recovery qualification are one release batch.
Owners: facade/control plane, host build/role contract and test infrastructure.
See [the CR1 contract](0.110-design.md#canonical-fleet-subnet-root-batch-cr1).
The complete hard cut and propagation are implemented in the open 0.110.10
draft. Packaged Root construction, fresh generated Fleet recovery/replay
(720.46s, including cold builds), generated reinstall/recovery/conservation/
replay (496.36s), focused source/interface tests and affected-target Clippy pass.
See [the CR1 handoff](../../audits/reports/2026-09/2026-09-07/canonical-fleet-root.md).
Package versions remain 0.110.9; the maintainer-selected release gate and
publication remain. CR1 does not close the independent runtime-contraction line.


Date: 2026-09-06

## Status

- Authorized correction batch (2026-09-05): resolve `CANIC-007` and
  `CANIC-132` through `CANIC-138` within Canic. Runtime/auth owns synchronous
  selected-store validation and auth-free Root reachability; host/Fleet Ensure
  owns shared policy admission, complete capacity, reviewed native funding and
  pool reconciliation, and management-bound same-module Start. Required
  evidence includes typed rejection before effects, corrupted issuer restore,
  exact enabled/disabled role surfaces, lost responses, process reconstruction,
  terminal cycle conservation and immediate effect-free replay. Status: ready
  for release approval;
  publication and downstream effects remain separate boundaries.
  The focused four-Shard and direct nineteen-Workload activation/replay cases
  pass. Automatic five-Workload/five-Ready convergence passes in 548.96s;
  four-Workload reserve refill passes in 377.41s; the full four-Workload/four-
  Failed repair passes in 349.17s without Root-account funding or creation.
  All 138 focused Fleet host tests and scoped Clippy pass. Seven native auth
  cases, auth-free Root activation, and the controlled Toko Root comparison
  pass; the latter removes 333,288 code-section bytes (5.15%). Named optimized
  symbol evidence remains separate from canonical artifact measurements.
  Generated nineteen-plus-five passes in 990.33s, including all 104 effects,
  terminal conservation and both completed-plan and newly planned effect-free
  replay. The corrected fixture preserves source-relative package authority
  and checks it before host effects.
  CANIC-134 follows the maintainer's 2026-09-06 correction: explicit reinstall
  discards state while preserving cycle and asset control. Full evacuation is
  conditional on deleting the Fleet. The separate custody implementation and
  pinned predecessor owner have been removed. The Root Ledger deletion slice
  passes its lost-reply checkpoint; complete whole-Fleet deletion is optional
  follow-up. Pool-policy/status errors no longer select implicit resets.
  Start retains only exact installed-module authority. The Root reinstall
  prerequisite now binds stop/reinstall/start to exact management authority;
  generation avoids protected calls on a different Root module. All 138 focused
  host cases pass after this change. The generated changed-release PocketIC
  journey passes in 522.04s, including a working Fleet after lost-response
  recovery, conservation and both effect-free replays. Live Toko applicability
  remains downstream qualification. All 47 selected Fleet CLI tests and five
  canonical Coordinator Candid tests pass. The final Coordinator preflight and
  Ledger lost-reply checkpoint passes in 142.74s. The correction batch is ready
  for release approval; the governed release gate and publication remain.
  See the [feedback readiness report](../../audits/reports/2026-09/2026-09-05/fleet-feedback-readiness.md).

- State: B1 promoted and active after accepted 0.109 closeout.
- Review verdict: the frozen replica-validator-equivalent function counter,
  whole generated-surface and repository-owned capability-fixture gates are
  binding. Consumer artifacts remain non-binding routing evidence. Predecessor
  closeout and explicit promotion remain mandatory.
- Outcome: create durable absolute Wasm code-section and replica-limited
  defined-function headroom through storage, codec and whole generated-surface
  endpoint/provider/recovery reachability cuts.
- Capability boundary: the contraction work adds no runtime capability.
  The separately authorized CANIC-134 correction permits explicit hard-cut
  reinstall and cycle-safe Fleet deletion. General estates, host-only version inventory and
  status redesign remain outside this line.
- Provenance: immutable predecessor `3185dc45b` (`v0.109.35`), superseding the older
  tracked 0.110 revision last changed at `ef3acc17c`.
- Compatibility: reinstall-only. Selected codec cuts retain no predecessor
  decoder and require complete destroyed-state/reconstruction evidence.
- Predecessor: 0.109 B8-B10, its immutable complexity evidence and the human
  minor-closeout acceptance are complete.
- Promotion: the human maintainer explicitly promoted B1 on 2026-09-01.
- Successor: 0.111 owns bounded cycle-safe multi-Fleet estates after 0.110
  closeout.
- Downstream: 0.110 cannot delay the published 0.109 Toko Miner unblock.
- First release: immutable `v0.110.0` at `d29a7cc72` carries the initial
  `CANIC-119` through `CANIC-121` fresh-estate correction and corrected
  canonical-role baseline; it does not claim B1 completion or authorize B2.
- Latest release: immutable `v0.110.7` at `2cd3588b6` completes the endpoint,
  validation, terminal-authority, estate-funding, cryptography and retained
  B1 attribution work recorded in the detailed changelog. It does not accept
  B1 or authorize the remaining B2/B3 families.
- Published `v0.110.6` repairs active child
  response completion, terminal Fleet discovery, backup and descendant
  authority, and controller-protects exact observability while preserving
  operator access through Root relays. The earlier public host-only CANIC-124
  managed Component-tree qualification fixture, bounded CANIC-126 Root-owned
  balance re-observation and CANIC-127 Prepared-Root initial-child convergence
  remain current. The
  fixture derives exact `ComponentChild` authority for configured and on-demand
  sharding, scaling and index children. An exact active runtime-configuration
  replay can also reclaim an exhausted transient non-Root init bootstrap without
  rerunning application init or reopening a non-retryable failure. These
  corrections do not promote B2. One governed literal-zero-estate proof drives
  the current plan, durable journal, concrete production adapter, lost-response
  recovery and real Coordinator/Root/Store protocol through terminal cycle
  conservation and immediate zero-effect replay.
- Published `v0.110.6` correction: an Active Component parent receives its newly
  provisioned child ID from the original capability call, while Prepared
  initial bootstrap alone retains detached completion. The managed Component
  Group PocketIC journey proves both branches. This correction does not change
  the immutable `v0.110.5` B1 baseline; candidate measurements include the
  corrected working overlay.
- Published `v0.110.6` terminal-inventory correction: Root pool Workloads are matched to
  the complete protected top-level and descendant Component tree by exact
  Component and allocation-operation authority. Only an observed,
  protocol-bound Pool-to-Component promotion may change logical parent, and
  `info subnets` retains the selected ICP executable and environment. This
  operator correction does not promote B2 or expand runtime capability.
- Published `v0.110.7` hardening: terminal Components additionally require exact
  running state, Root-only controllers and current management/Directory module
  identity; cycle reporting is capability-aware; Fleet Ensure treats every
  Root Cycles Ledger account as a separately observed and reviewed funding
  domain, pausing without a protocol effect when its exact creation forecast is
  underfunded, executing only a reviewed plan-owned Ledger transfer and
  reconciling exact creation receipts and fees at terminal;
  observability authority is
  proven through exact diagnostic codes and response variants across fresh and
  restored PocketIC fixtures; and B1 consumes typed transform metrics from a
  method-owned harness that compiles against the immutable product worktree
  instead of optimizer log prose. Its separate offline-resolved lock and every
  runnable patch hash are machine-bound. A retained all-role row-3 measurement
  against immutable `v0.110.5` attributes 3,001,136 optimized code bytes and
  2,025 defined functions to the inclusive activation-persistence family while
  preserving every role's Candid hash. It proves no activation parity and does
  not promote B2. Role-contract validation also rejects auth-crypto features
  without matching role authority, target-Wasm dependency closure rejects
  duplicate crypto package generations, and capability reports expose retained
  cryptography symbols separately. Aligning the existing chain-key
  implementation with the IC client stack's `k256 0.13.4` dependency is a
  non-protocol contraction; chain-key replacement remains unauthorized.
- Current open 0.110.8 slice: the maintainer explicitly authorized the bounded
  authorization-persistence split before complete B1 acceptance. Local
  application sessions, delegated-token issuer proof and Root delegation
  policy now use separate records, stable cells, operations and allocation
  keys. Verification alone, Coordinator and Store select no auth persistence;
  Root selects only Root delegation state. Typed contract tests and targeted
  optimized Coordinator, Root, verifier-only and combined issuer/local builds
  prove the selected declaration set. This slice neither accepts B1 nor opens
  unrelated storage families.
- Current open 0.110.8 dependency alignment advances only the isolated
  lifecycle-composition fixture to the exact IcyDB 0.253.0 family with default
  features disabled. It adds no IcyDB edge to a production Canic crate and
  changes no Fleet runtime capability or B1/B2 verdict.
- Row 8 is `ready` after the September 14 complete fourteen-artifact
  qualification on frozen `v0.110.5`. The existing audit-only Store Candid
  accommodation remains confined to the experiment. Exact source/lock and
  artifact evidence is in the
  [qualification report](../../audits/reports/2026-09/2026-09-14/b1-row8-qualification.md).
  No retained paired measurement or runtime parity is claimed.
- Rows 10 and 12 are `ready` after complete fourteen- and eleven-artifact
  qualifications on September 14, using their original patches and frozen
  source/lock. The [report](../../audits/reports/2026-09/2026-09-14/b1-row10-row12-qualification.md)
  preserves exact evidence. Immutable paired measurements remain open.

Design: [Fleet runtime contraction](0.110-design.md)

Planning evidence: [B1 Wasm input evidence](../../audits/working/0.110-fleet-runtime-contraction/b1-input-evidence.md)

Generated-surface source trace:
[B1 generated surface inventory](../../audits/working/0.110-fleet-runtime-contraction/b1-generated-surface-inventory.md)

Destroyed-state source trace:
[B1 destroyed-state and reconstruction inventory](../../audits/working/0.110-fleet-runtime-contraction/b1-destroyed-state-inventory.md)

Pool Ledger recovery hard-cut trace:
[B1 source-family absence ledger](../../audits/working/0.110-fleet-runtime-contraction/b1-pool-ledger-recovery-hard-cut.md)

Controlled-ablation trace:
[B1 experiment manifest](../../audits/working/0.110-fleet-runtime-contraction/b1-controlled-ablation-manifest.md)

Current canonical-role evidence:
[CANIC-WASM-001/v6 baseline](../../audits/reports/2026-09/2026-09-03/wasm-footprint-v6.md)

## Binding Corrections

- B1 freezes dated code-section, total-module and defined-function limits.
  Completion requires at least 5% of both the code-section and replica-limited
  defined-function limits free under frozen tools: 512 KiB code headroom under
  the retained 10 MiB limit, with 1 MiB preferred. The frozen IC source proves
  that imports do not consume the 50,000-function limit; the optimizer-defined
  metric is an independent cross-check, while `ic-wasm`'s total is attribution
  only.
- B1/B5 bind the canonical eleven-role roster and Canic-owned `runtime_probe`,
  `payload_limit_probe`, `blob_storage_probe` and `leaf_probe` fixtures. Their
  matrix covers both authentication paths, blob storage/billing, lifecycle,
  payload adapters, status, metrics, timers, applicable recovery and the
  generic cohort without importing a consumer protocol.
- The supplied 10,275,629-byte/268-method downstream observation remains useful
  pressure evidence only. A Toko/TokoMiner commit, lockfile, application state
  policy or release procedure is not a Canic gate.
- Baseline attribution uses controlled storage, activation, authorization,
  CBOR, recovery, macro projection, Candid construction/documentation/
  serialization, async-adapter, provider, timer and status ablations. Every
  experiment records code, total, compressed, function, table and instruction
  evidence.
- B1 adds a controlled `1..=N` Canic generic-instantiation cohort and a named
  post-`-Oz` report mapping remaining Canic-owned generic families to concrete
  type arguments and specialized bodies. Per-instantiation deltas are measured,
  not inferred from downstream generator slopes.
- B2 emits direct role-specific initialization, restore and recovery calls.
  Function-pointer, trait-object and erased-callback registries are excluded.
- B1 inventories every runtime-contributing macro expansion. B2 contracts its
  storage/lifecycle portion; B4 closes endpoint, authentication/admission,
  status, Candid, metrics, command, configuration, provisioning, timer,
  recovery and role-dispatch reachability.
- Macro imports, macro splitting and source shape receive no size credit.
  Canonical optimized artifacts must prove unselected machinery absent.
- Every B2-B4 batch ends with canonical roles, each affected Canic-owned
  capability fixture, fresh marginal attribution and an explicit
  stop/continue/retarget decision.
- B3 records/codecs proceed only where the latest residual evidence justifies
  them. B4 remains mandatory while any known role-inapplicable generated
  endpoint/provider/serializer/timer/recovery/control-plane surface remains;
  reaching the size reserve cannot waive final artifact absence.
- Endpoint extraction is accepted from optimized artifacts, not source shape.
- The temporary pool Ledger recovery helper is hard-deleted in published
  `0.110.3`. B1 records the current/predecessor artifact delta;
  B4 verifies that no helper/Store/Root/DTO/host/test/CI reachability returned.
  Current Fleet funding targets native canister balances and retains no helper
  compatibility path or fallback.
- Chain-key ECDSA replacement is not authorized. Missing the safe contraction
  gates stops the line for a separate protocol design and threat-model audit.
- Release-build acceleration is parallel supporting work. It is not a batch
  and cannot delay the first evidenced runtime cut.
- Governed release metadata, planning and qualification enforce the
  reinstall-only hard cut structurally. An existing stateful Fleet's ordinary
  upgrade/adoption attempt must reject with no install, stop, controller,
  funding or workflow effect. Only an explicit authorized reinstall may then
  proceed.
- If safe Canic-owned cuts do not place the canonical roster and fixture matrix
  above both 5% reserves, B5 stops with an exact Canic-owned residual handoff.
  Optional consumer observations do not authorize downstream mutation or
  weaken authentication.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence | Status |
| --- | --- | --- | --- |
| B1 | Immutable baseline, differential attribution and absolute budgets | Dated limits, repository-owned capability fixture matrix, replica-validator-equivalent local-function count, generated-surface inventory, complete artifact vector, current/predecessor delta for the deleted temporary pool Ledger recovery family, `1..=N` generic-instantiation cohort, named post-`-Oz` report, destroyed-state/reconstruction inventory and accepted allowances | Active from immutable `v0.110.5`; valid `CANIC-WASM-001/v6` size/determinism evidence, generated-surface/destruction traces, pool-Ledger source absence, the machine-checked eighteen-row ablation harness and repository-owned frozen function counter, immutable all-role row 2 attribution supporting role-selected storage wiring without lifecycle parity, immutable all-role row 3 inclusive activation-persistence attribution supporting role-selected separation without activation parity, immutable canonical-plus-runtime-fixture row 4 authorization-persistence attribution without persistence or authorization parity, immutable canonical-plus-runtime/blob-fixture row 5 shared-CBOR-helper attribution without codec or persistence parity, immutable canonical-plus-runtime-fixture row 6 recovery-dispatch attribution without recovery parity, selected-artifact qualification for rows 8, 10 and 12, immutable row 11 payload-adapter attribution retaining the safety path, the `Page<T>`/`N = 5` generic fixture and hash-bound downstream routing observation are retained, while counter-backed immutable role/fixture measurements, remaining source-ablation patches and measurements, optimized-artifact absence, generic measurements/post-`-Oz` mapping, accepted allowances and compatible predecessor artifact evidence remain open |
| B2 | Role-selected storage reachability | Lazy TLS, direct generated wiring, storage/lifecycle inventory contraction, data-only reservations, symbol absence and full remeasurement | Blocked overall on B1; bounded auth stable-declaration sub-slice explicitly active and targeted role evidence passes |
| B3 | Capability-owned activation/auth records and only still-justified codecs | Concrete records, phase cache, bounded codec evidence and full remeasurement | Blocked overall on B2 decision; bounded auth-record split explicitly active without selecting another codec cut |
| B4 | Endpoint, recovery and role-capability pruning | Complete generated-surface inventory, exact Candid/provider reachability, optimized body/function evidence, direct dispatch, continued absence of the hard-deleted temporary pool Ledger recovery family, role pruning and full remeasurement | Mandatory after the B3 decision while known role-inapplicable reachability remains |
| B5 | Canic-owned qualification and closeout | Canonical and fixture 5% byte/function reserves, capability matrix, per-role generated-surface absence, total-module limit, instructions, determinism, structured reinstall-only guard, optional consumer observations and immutable audit | Blocked on final B2-B4 decision |

## Deferred From 0.110

- Broader host semantic-version inventory and status redesign return to later
  operator planning. CANIC-014's generated release snapshot shipped in .16.
- Indexed estates, a bounded reserve Fleet and cycle-safe source disposition
  remain 0.111 work.
- Adaptive lanes, broad automatic funding, batches and 1,000-canister
  qualification are unscheduled.
- The host-owned generic observatory shipped in .16 as OP3; renderer and
  application profiles remain outside canonical canister runtime roles.

## Next Authorized Action

The .16 release is complete. The later September 14 CANIC-139 correction above
is complete in the open .17 draft; downstream adoption remains outstanding.
Keep downstream adoption, installed-state measurements and authorized staging
recovery separate from Canic source completion. The maintainer's continuation
keeps the contraction sequence below active in 0.110; faster tests and the
published memory consolidation do not complete its attribution evidence.

The focused authorization-persistence source review confirms separate
feature-selected stores and restore paths, with no new defect found. It adds
no runtime parity or B2 remeasurement evidence and authorizes no other state
family. Row 8 endpoint-declaration construction is now fully build-qualified.

The latest downstream refresh remains CANIC-175. The selected .17 correction
batch is ready as defined above, including positive-credit reset interruption,
native withdrawal and grant/deadline qualification. Release execution remains
maintainer-directed. Follow-up RF2/RF3 preserve the remaining receipt-conversion
and complete live funding-preview criteria. The original identity-prompt cause
and exact staging rejection remain unproven; auth/E9 stays parked.

After RF2/RF3, continue B1 from immutable `v0.110.5` with retained matched measurements for
ready rows 8, 10 and 12; row 6's retained measurement is complete. Then
complete the remaining controlled ablations,
optimized generated-
surface absence, generic cohort and accepted allowances and obtain compatible
predecessor evidence where required. The source generated-surface and complete
allocation/destruction inventories are retained. Do not begin the remaining B2
or B3 scope until the maintainer accepts the complete B1 evidence.
