# Toko recovery feedback through CANIC-175

Date: 2026-09-14. Canic base: published `v0.110.16`,
`a875c6498721bd89ca98549e38389b8280910d68`, with the existing open .17 draft.
Initial Toko HEAD was `dc58fb253cdd543b0141796c3b9260d1ccf00303`; its working
`docs/upstream/canic.md` advances during the B1 measurement to SHA-256
`05a202c11f25e8863556d68c313372a822985f71049f0003cd2adbcbdf3b1433`.
Sibling files were read only. Earlier .17 work is preserved.

The later read-only source-review refresh retains CANIC-175 as the last issue,
confirms CANIC-174 from the reproduced grant/deadline failure and adds no issue
for Toko's frontend import failures. Its current ledger SHA-256 is
`614cfd58b9b612ca162279b2496a13c42cc98226d6760399558d79142063e9c6`.
Toko HEAD at the final refresh is `b3d6c23991cd740d2a29b678c569f430a2e65857`.
The review agrees with the remaining funding, combined recovery and exact mint
receipt priorities. It treats the corrections as unpublished and keeps the
original encrypted-identity incident's attribution provisional.

## CANIC-173: implemented and tested

The reported duplicate-binding defect is present: terminal publication replaces
topology from desired configuration but previously retained both the configured
name and an earlier inventory name for the same Principal. The subsequent
duplicate-Principal rejection is correct; removing that rejection would weaken
terminal authority and permit double counting.

Publication now delegates an exact projection to the existing host ops owner.
An untyped name is removed only when its Principal also has a current configured
topology binding. Unique dynamic assets remain for fresh terminal inventory.
Two configured owners, or two unconfigured names without configured authority,
remain conflicting and are rejected. That rejection also applies to an empty
terminal inventory, before changing the retained registry projection.

No canister, cycle amount, starting balance, journal receipt, pending effect or
live authority check is removed. This changes local projection, not remote
estate state. The reviewed downstream patch supplied the narrow hypothesis;
Canic places record mutation in ops and additionally qualifies the actual
publication call, replay and empty-inventory boundary.

Validation passes:

- Two new focused cases exercise pool and Component projection, preserved
  unique dynamic assets/cycles, unchanged journal bytes, repeat publication,
  configured/unconfigured conflicts and empty/nonempty inventory rejection.
- Two existing terminal inventory and parent-transition cases pass.
- The generated retained multi-Component apply/replay case passes.
- Host all-target, all-feature Clippy passes with warnings denied.

The new tests exercise host projection. They do not reproduce Toko's live
24-asset estate or claim published downstream adoption. Toko's local convergence
with its own temporary correction remains separate evidence.

## Remaining feedback and implementation sequence

The final native-funding refresh observes ledger SHA-256
`f5ddb5c3911ded515e00a8867e8e38f8814290bad1d61cb941bc75742e1fe859`.
It still ends at CANIC-175 and adds a next-deployment priority handoff. Funding
forecasts/bootstrap/debit timing come first, then exact-release build reuse and
repeated inspection costs, followed by recovery publication, originating
failure visibility and funding-state control. This aligns with prioritizing
CANIC-156/172/174 ahead of B1. Separate build, inspection and CI timing samples
do not establish a combined deployment speedup. Idle-watchdog burn remains an
application attribution candidate; no new Canic defect is established there.

| Issue | Source review | Remaining correction and evidence |
| --- | --- | --- |
| 175 | Implemented and qualified in the open .17 draft. Role-owned dispatch, bounded partial outcomes and pre-await Root timer reconciliation replace the confirmed failing path. | The [state-cascade report](canic-175-state-cascade.md) records native/Candid checks and the passing final Root/Store/Hub/Shard/ordinary-role IC case, authority rejection, partial failure, retry and Readonly/Stopped recovery. Toko's installed Translation workload remains downstream qualification. |
| 172 | The [child reserve diagnostic/recovery slice](canic-172-child-reserve.md) is implemented and qualified: E163, retained bounded failure/backoff, unchanged capacity/receipts and an IC same-claim readiness/replay proof. The [native host review/accounting](canic-172-native-funding.md) is now implemented and passes focused host/CLI checks; its composed issued-operation IC withdrawal proof now passes both response-loss boundaries, conservation and replay. Combining that withdrawal with the E163 retained claim and operator mint receipts remains. | Include bootstrap execution reserve and initial runtime grant demand; extend the existing funding owner with exact native recipient/controller/amount/fee and mint/withdrawal receipts. Preserve original operation, effect prefix, starting balances and conservation. Qualify the governed operator funding workflow independently of disposable test replenishment. |
| 174 | The [IC owner-path proof](canic-174-funding-deadline.md) confirms the unchanged deadline after a real grant crosses the reserve. The corrected cycle owner resamples transfer settlement, advances demand without postponing earlier checks, and excludes transfers from its burn-rate estimate. Final IC proof, replay/budget checks, scoped Clippy and guards pass. | Forecast initial threshold shortfalls and upstream reserve availability without double-counting internal transfers or granting funding authority. The runtime timing correction does not supply missing Coordinator reserves. |

CANIC-175 coordinates command, outcome and timer behavior. No Store alias,
generic compatibility command or automatic rollback was added.
CANIC-172 must use the existing funding owner;
the reported manual staging top-up is not an authorization model for Canic.
CANIC-174's IC owner-path test now establishes the timing defect. Generation
also reports a zero-burn startup-grant scenario and upstream native headroom,
with request/lifetime/window constraints and shared-role counting corrected.
The [funding report](canic-174-funding-deadline.md) records its passing focused
checks and limits. Pending provisioning on observed Roots now binds startup
demand and selected execution reserves into the reviewed native Fund action,
with one Ledger fee, deferred-work rollback and no terminal startup refill.
Fresh continuation also prepays startup funding through its initially reviewed
Create/Fund actions, with bounded Root-local catalogue accounting and no new
successor spending permission. Live grant/reserve accounting remains open.

The follow-through source review identifies these owners and remaining work:

- Host `fleet_ensure::workflow::funding` now validates estate and native reviews
  through the same owner and retained approval history. Native withdrawals use
  the maintained `EnsureAction::Fund` adapter. Recipient/controller authority,
  fixed transfer identity and exact fee/debit conservation have focused host
  coverage. Exact operator mint credits and the combined E163 claim/withdrawal proof remain;
  original balances and the issued effect prefix stay fixed.
- The downstream temporary `native_recovery` correction is bound to one fixed
  plan, operation and receipt file. It is useful evidence for the accounting
  case, not a portable implementation to copy into Canic.
- The direct-child driver now retains its failure through the existing
  allocation record/view/response owners and uses bounded backoff. Metadata
  changes do not count as lifecycle progress; the deployment reserve remains
  unchanged. The new IC proof qualifies the same retained claim after native
  replenishment, but supplies no operator funding authority or Ledger receipts.
- Child-grant settlement now reconciles the existing timer without treating
  transfers as burn or postponing earlier deadlines. The real IC threshold,
  upstream grant, replay and budget sequence passes. The granting parent's
  existing Fleet funding flag check remains authoritative.

These are remaining accepted Canic corrections, not release completion. The
earlier source-bound positive-credit interruption/replay and reserve-preview
evidence also remain due. Funding presentation should combine CANIC-156/172/174
requirements rather than create competing funding authorities.

## B1 and release boundary

[B1 row 6](b1-recovery-dispatch-measurement.md) now has its complete retained
measurement. Rows 8/10/12 and the remaining B1 attribution evidence stay open;
this recovery work does not change that frozen experiment or accept B2/B3.

Package versions remain .16, and the .17 changelog includes the CANIC-172
diagnostic/recovery slice, CANIC-173/175 and
the B1 tooling/measurement result. The complete accepted release batch is not yet
push-ready. No broad suite, version bump, commit, push, deployment, live funding
or sibling mutation ran.
