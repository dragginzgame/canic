# Toko recovery follow-ups

Date: 2026-09-14. Canic source base is published `v0.110.16`,
`a875c6498721bd89ca98549e38389b8280910d68`; changes extend the existing
unpublished .17 draft. Prior dependency, CANIC-139 and B1 work is preserved.

The read-only downstream refresh still ends at CANIC-171. Toko HEAD is
`dc58fb253cdd543b0141796c3b9260d1ccf00303`; its working ledger SHA-256 is
`d8eaf8c6dfae43757ff84095047b297300a0b705605b7909967163ce2e7026b4`.
The newer ledger expands CANIC-156 beyond the earlier review.

## Implemented

| Issue | Current Canic change | Evidence boundary |
| --- | --- | --- |
| 139 | Prior absent-Cargo-input correction remains in .17. | Existing real Cargo reproducer and reuse evidence remain in its separate report. |
| 166 | The public exact-apply selector prioritizes the validated staged review, retaining all identity/digest checks. | Four source-parser fixtures exercise selection, wrong-digest rejection, preserved source bytes and interrupted durable handoff. No old executable-plan decoder was added. |
| 171 | A required report field accounts for bounded credits in exact Applied Stop receipts, only in source-bound preparation and Root reset. | Six tests cover both scopes, receipt hashes/states/completeness, bounds, full terminal arithmetic, overflow, unchanged journal bytes and repeat verification. Repeat verification is not a new complete remote interruption journey. |
| 170 | Fleet contexts bind one selected identity before the Principal fence; clones and subsequent subprocesses use the same name, including direct-agent export. | A fake ICP executable changes its default between observation and a later call. A separate platform test changes the selected identity's Principal and proves no effect is issued. These do not establish the cause of Toko's original prompt or exercise real encrypted keys. |
| 156 | Exact SDK liquid-cycle admission failures retain a distinct public code and actionable host guidance. | Typed conversion/Candid round-trip, unspecified-failure and bootstrap retry tests pass. A focused Root PocketIC case proves E162 with positive native cycles and recovery without funding; this does not identify Toko's original SDK error variant. |

The 166/171 starting point was the reviewed downstream
`docs/audits/reports/2026/09/14/staging-recovery/01/cli-staging-recovery.patch`.
Canic adds explicit imports, lint cleanup and terminal-conservation regressions.
The recorded downstream recovery with its temporary patch is supporting
evidence, not qualification of this complete dirty Canic candidate.

## Root reserve qualification

The maintained `root_probe` and generated Wasm Store fixtures were built from
this candidate. The new integration case uses the existing Root
`InspectCanister` endpoint and real management calls to a blank Root-controlled
target. It searches the fixture's measured freezing-reserve interval; code,
target and controllers remain fixed.

The observed failure had `599945111099266` native cycles and a freezing threshold
of `19793138865` seconds. The public result was E162. Resetting the disposable
fixture's threshold to zero made the same inspection succeed, and the observed
balance did not increase. The unusually large threshold follows from the
fixture's large initial balance; neither number is a production recommendation
or an exact Toko workload measurement. No instruction or wall-clock performance
claim is made.

The passing entry point was:

```sh
CARGO_NET_OFFLINE=true make test-pocketic-case CASE=pic_root_inspection_reserve
```

Earlier direct Cargo attempts stopped at environment setup: generated metadata
could not reach crates.io, then the fixture correctly required the governed
server URL. The final targeted Make entry point owned its private scratch and
PocketIC server and used cached dependencies. Only the named case ran. The
inventory now classifies this test in the serial PocketIC runtime lane.

## Remaining accepted work

- CANIC-156's typed failure path is qualified. Ahead-of-inspection host reserve
  evidence and protected numerical details remain work. A
  generic call-perform failure cannot safely be relabelled as insufficient
  funding. The code-only public envelope intentionally does not expose raw
  rejection text or numerical SDK details; richer protected inspection evidence
  remains work.
- CANIC-156's whole-recovery preview is a separate candidate: distinguish
  current authoritative requirements from later estimates and unknown values.
  Reconcile later pool/readiness/reset/observation requirements without granting
  transfer authority from a preview or changing existing reserve policy.
- CANIC-171: qualify the complete source-bound preparation/Root-reset
  interruption and effect-free terminal replay on this exact candidate. Native
  receipt and conservation tests do not establish that complete IC journey.
- B1 remains open for matched measurements and remaining attribution evidence.
  None of these recovery changes accepts B1 or supplies a size-saving result.

CANIC-161 remains a candidate application incident: the reported Toko nested
search was removed downstream, and funding configuration guidance is already
recorded. There is no confirmed additional Canic algorithm defect to fix.

CANIC-002/003/008/010/017/135/165 are Available, with implementations already
shipped and downstream adoption/qualification outstanding. This work does not
duplicate those implementations or change Toko's acceptance statuses.

## Validation and release boundary

Focused settlement tests (6), retained-source tests (4), identity-filter tests
(15, including the two new regressions), infra mapping tests (2), and the
typed-liquid-failure/bootstrap-retry filter (2) pass. The existing generated
retained-estate apply/replay journey (1), CLI Fleet tests (15), diagnostic
register (4), and focused Root PocketIC case (1) also pass. Filters overlap;
these are command results, not an aggregate count of new cases.

Host/CLI all-target, all-feature Clippy and Clippy for the new integration
target pass with warnings denied. Changed Rust formatting, test inventory,
document semantics and whitespace checks pass.

The changelog describes the current required report field and new diagnostic.
The complete accepted batch is still open for the qualification above; this is
not a full-batch push-readiness claim. No broad suite, version bump, commit,
push, deployment, funding or sibling mutation ran.


## CANIC-156 protected numerical inspection — 2026-09-15

The current Root `InspectCanister` response now includes
`InspectionReserveRequired`. It carries the exact calling Root and requested
target, native cycles sampled before the management attempt, and the SDK's
available/required liquid-cycle amounts at call admission. The SDK rejects this
failure before issuing the outbound management request. The existing transport,
metrics and controller guard remain; no extra query, stable record, reserve
reduction or automatic top-up is introduced. Successful inspection retains its
existing response payload. Other SDK/transport failures retain their original
classification and do not receive invented numerical funding evidence.

The host pool/reset, terminal inventory and cycle-observation paths recognize
this current response. They reject mismatched identities and impossible values,
retain typed E162 evidence and print exact cycle amounts. The production pool
inspection adapter's existing fake-ICP fixture proves that a numerical failure
survives decoding and that a successful retry reads fresh status in the same
observation scope. Failed inspections are not cached as target state.

Validation:

- Focused core/host inspection regressions pass in
  `/tmp/canic-inspection-numeric-tests.log`, including exact Candid evidence,
  generic-failure preservation, invalid-evidence rejection, production host
  propagation/retry and existing pool inspection authority/cache checks.
- Scoped all-target/all-feature warning-denied Clippy passes for `canic-core`,
  `canic-host`, `canic` and `root_probe` in
  `/tmp/canic-inspection-numeric-clippy.log`. The changed `canic-tests`
  integration target's Clippy passes in `/tmp/canic-inspection-numeric-ic-clippy.log`.
- `CARGO_NET_OFFLINE=true make test-pocketic-case CASE=pic_root_inspection_reserve`
  passes in `/tmp/canic-inspection-numeric-ic.log`. It checks controller-only
  access, a genuine positive-native-balance SDK reserve rejection, exact Root
  and target evidence, and successful retry without adding cycles after changing
  only the disposable fixture's freezing threshold. The selected ten source
  files stayed unchanged across the run, checked with
  `/tmp/canic-inspection-numeric-source.sha256`.

The observed evidence was native cycles `599904661596158`, available liquid
cycles `32857985623` and required liquid cycles `42102454000`, at a fixture
freezing threshold of `19779149118` seconds. These values follow from the
fixture's large initial balance and measured reserve interval; they are not
production settings, funding recommendations or an exact Toko workload match.
Only the selected IC case ran, not the full workspace or recovery suite.

This completes the protected numerical diagnostic gap, not CANIC-156 as a
whole. The Root must first admit the inspection update, so an already frozen
Root may still fail before this evidence can be returned. Earliest host reserve
preflight and whole-recovery forecasting remain open. The SDK's one-call reserve
is not a complete execution budget or an authorized funding debit. Source-bound
CANIC-166/171 interruption/replay, live child-grant accounting and exact operator
mint receipts also remain part of the accepted .17 batch. No package version,
commit, push, live deployment, funding or sibling repository change occurred.

## CANIC-156 ahead-of-inspection reserve query — 2026-09-15 follow-up

The current Root contract adds controller-only
`canic_observability(InspectionReserve(target))`. It reads native/liquid balances
and quotes the same encoded `canister_status` call builder used by the actual
inspection, through the pinned SDK's cost API. No outbound management call,
stable record, automatic funding or reserve-policy change is introduced. Only
this additional observability variant is admitted while the Root is prepared;
the other variants retain their existing fence.

Host pool inspection, retained-asset preparation/reset, terminal inventory and
cycle observations query before each uncached inspection. They bind the exact
Root/target through the selected Candid contract, reject impossible samples and
report `InspectionPreflightReserve` before the update when liquid cycles are
insufficient. This is distinct from the actual SDK's E162 admission failure.
Neither failed query nor failed inspection becomes cached target state. The host
does not impose a minimum quoted fee; zero is accepted if the cost API reports it.
One query is added per uncached inspection. Bounded pool concurrency and reuse
of successful observations remain covered by the native regressions.

Focused qualification passes:

- Sixteen host inspection cases, three pool concurrency/error-precedence/retry
  cases and generated retained-estate planning/apply/replay:
  `/tmp/canic-inspection-preflight-host.log`,
  `/tmp/canic-inspection-preflight-batch.log` and
  `/tmp/canic-inspection-preflight-generation.log`.
- Final host minimum-fee validation and scoped all-target/all-feature Clippy:
  `/tmp/canic-inspection-preflight-final-host.log` and
  `/tmp/canic-inspection-preflight-final-clippy.log`. Earlier owning-package
  Clippy for core, host, facade and Root probe passes in
  `/tmp/canic-inspection-preflight-clippy.log`; the changed integration target
  passes in `/tmp/canic-inspection-preflight-ic-clippy.log`.
- All twenty host regressions pass again after the final validator cleanup and
  concurrent ic-query 0.43 integration, in
  `/tmp/canic-inspection-preflight-final-regressions.log`. Formatting, layering,
  document semantics and whitespace checks pass.
- The existing `pic_root_inspection_reserve` case passes in
  `/tmp/canic-inspection-preflight-ic.log`, using PocketIC 16.0.0 and the Fast
  Root probe. It proves controller rejection, prepared-Root query availability,
  the remaining E46 observability fence, encoded-target-dependent cost, exact
  agreement with the SDK's required reserve, a query-observed shortfall without
  changing native balance, and recovery without adding cycles. The thirteen
  selected source files stayed unchanged during this run
  (`/tmp/canic-inspection-preflight-source.sha256`). The later host-only
  minimum-fee cleanup is qualified separately above; the IC code is unchanged.

Observed raw cycles in the disposable fixture:

| Observation | Native | Available liquid | Required outbound reserve |
| --- | ---: | ---: | ---: |
| Actual SDK rejection | 599904645833710 | 32858017243 | 42102454000 |
| Subsequent query, same freezing threshold | 599944631533531 | 72843717064 | 42102454000 |
| Query after another fixture threshold adjustment | 599944608045531 | 36595702272 | 42102454000 |

These samples also demonstrate the limitation: a sufficient query does not
guarantee update admission. The query does not reserve the later update's
execution cycles, and balances can change between calls. The SDK remains the
admission authority and its exact failure survives a successful preflight.
The large fixture balances and searched freezing thresholds are not production
settings, sizing recommendations or an exact Toko workload match.

This completes the known outbound-reserve preflight path where the Root can
serve the query. A frozen/unreachable Root still retains its query transport
failure. The one-call quote is not a complete ingress/execution budget or
whole-recovery funding preview. Exact operator mint receipts, live child usage
and recovery-quote integration, and CANIC-166/171 source-bound IC interruption
evidence remain required before the accepted .17 batch is push-ready.

## Bound inspection contract correction — 2026-09-15

Review of CANIC-166 found that the new unconditional preflight requested a selector
absent from retained source Root contracts. Host inspection now checks the exact
bound Candid's service, request variant and selector structurally, resolving named
types and checking the query mode and target payload. A contract without the
selector keeps the protected inspection; its missing quote supplies no reserve
estimate. Invalid contracts and failed declared queries still reject. There is
no version negotiation, error-triggered bypass, source-plan execution or runtime
change. Existing artifact/controller/conservation admission remains authoritative.

The focused host regression covers actual adapter call selection, shortfall,
malformed response, invalid contract and fresh successful retry. It also covers
named Candid types and unrelated selector names. Qualification is native host
transport evidence; it does not close the CANIC-166/171 IC recovery journey.

Final qualification passes: twenty-four inspection, bounded pool and Candid
binding cases in `/tmp/canic-source-inspection-final-tests.log`; generated
retained-estate planning/apply/replay in
`/tmp/canic-source-inspection-generation.log`; owning host package
all-target/all-feature warning-denied Clippy in
`/tmp/canic-source-inspection-final-clippy.log`. Formatting, layering, document
semantics and whitespace checks also pass. Published .16's Root observability
definition confirms the selector is absent and its existing variants are fenced
while prepared. This review used the same published base identified above;
the Toko ledger remains unchanged through CANIC-175.

The initial-child funding fixture is not an admissible substitute for that
journey: source-bound activation reset requires a complete pool, published and
activated Components, a still-Prepared Root and the exact retained Root/Store
activation-identity conflict. Its source journal must retain issued provisioning.
No production admission was loosened and no completed journal was manufactured
to make a different fixture pass. This IC proof, whole-recovery funding preview,
operator mint accounting and child-ledger quote integration remain open.
