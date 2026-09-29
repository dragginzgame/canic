# CANIC-185 bootstrap funding correction

The 2026-09-28 Toko Miner feedback reports an unapproved infrastructure review
requiring 6,145.060424619911T of operator debit for the qualified 0.3.16 release.
The three directly controlled infrastructure canisters already held
677.939875380089T. No downstream apply or spending is part of this correction.
Toko Miner remains read-only.

## Cause and correction

The bootstrap planner deposited its entire hypothetical continuation ceiling
on every infrastructure target, in addition to that target's local allowance.
With unchanged generated 1T observation and update bounds, this multiplied a
2,033T continuation allowance across Coordinator, Root and Store.

Funding now covers each target's configured floor, local installation effects
and one complete observation window. That window covers the eight permitted
effect rounds, three observations per round, and two rounds of each of the four
whole-estate inspection phases. Funding and durable inspection use the same
attempt constants. The existing target funding margin and exact Ledger fees
still apply. No observation/update allowance or retry limit is lowered.

The plan's shared execution ceiling is capped by controlled native surplus
above configured floors; reserved cycles are excluded from spendable surplus.
Before another write, the ordinary effect driver checks the current target's
native balance against its floor, the complete observation window and one update.
This does not promise prepaid exhaustion of every possible later retry catalogue.
Insufficient native headroom or continuation budget produces a typed failure
without authorising additional funding. Original intents, receipts and approved
payment amounts remain authoritative. Success still requires terminal
conservation, configured floors and exact replay.

## Pre-build operator surface

For a completed Fleet, `fleet readiness --source --seed` projects the current
physical Coordinator/Root/Store inputs and observes their exact controllers and
native balances without artifacts or historical executable contracts.
`funding.clean_reinstall_infrastructure` reports per-owner funding, the sum and
maximum possible Ledger transfer count. Missing balances or mismatched authority
remain unknown. A known shortfall becomes an early readiness blocker.

The forecast uses five possible retained-infrastructure effects and the same
funding policy as exact review. Applied to the reported three balances and floors,
its upper funding estimate is 81.075619955792T before Ledger fees. This is a
calculation over recorded inputs, not a fresh mainnet observation or approved
plan. Fees, import, pool and workload funding retain explicit separate review;
this surface does not claim a complete whole-Fleet quote before its inputs exist.

## Qualification

The native generated-estate fixture now
retains the production observation default; its synthetic 27-ID variant keeps
24 children held by Root and checks that they do not inflate infrastructure
funding. Current native coverage also checks overflow, missing/headroom failures,
forecast unknowns, immutable review, inspection exhaustion and continuation.

Targeted host checks pass: 29 passed, two ignored. The selected filters cover
readiness, generation preflight, bootstrap funding policy, continuation and the
generated multi-component retained estate. They include rechecking the forecast's
parsed source against the selected operator and Ledger.

Both targeted PocketIC cases pass with the generated observation allowance:

- `completed_estate_reset_recovers_and_replays`: 369.26s test body, 532s runner.
  Covers lost install responses, same-digest continuation, retained IDs, clearing,
  exact Ledger debit, terminal conservation and offline plain/JSON replay.
- `supplied_infrastructure_initializes_and_recovers`: 161.24s test body, 162s
  runner. Covers initialization, lost install response, publication and replay
  after removing the fixture's reduced observation allowance.

Logs are retained locally at `/tmp/canic-185-final-native.log`,
`/tmp/canic-185-reset-pocketic.log` and `/tmp/canic-185-bootstrap-pocketic.log`.
The subsequent readiness-only authority check does not change the runtime
qualified by those PocketIC cases. No broad workspace gate was run.

Three focused CLI tests pass, covering readiness input handling and command
ordering. Warning-denied Clippy passes for `canic-host`, `canic-cli` and
`canic-testing-internal` with all features and library/test targets (14.92s final
run). Scoped formatting and diff checks pass. CLI and lint logs are
`/tmp/canic-185-cli.log` and `/tmp/canic-185-final-clippy.log`. The existing .47
batch and both changelog views are ready for maintainer review and the selected
release flow; package versions remain .46 and changes remain uncommitted.

No package version, Git commit, publication, live reset or downstream dependency
change is authorised by this implementation task. Toko Miner's existing tag
remains bound to its selected Canic release; adopting the correction requires
the maintainer-selected matching CLI/runtime release and qualification.
