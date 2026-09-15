# CANIC-166/171 source-bound activation reset

Date: 2026-09-15. Published base: `v0.110.16`, commit
`a875c6498721bd89ca98549e38389b8280910d68`. Qualification covers the dirty
0.110.17 development candidate; package versions remain 0.110.16. Toko's
read-only ledger remains unchanged through CANIC-175. No publication, live
deployment, sibling edit or broad gate was performed. The auth/E9 investigation
is parked at the maintainer's request.

## Outcome

The existing production-adapter fixture now qualifies source-bound preparation
and the Root-reset prerequisite together. Production admission and recovery
semantics are unchanged. The case is registered in the governed Fleet catalog:

```text
pic::fleet_registry::baseline::tests::activation_reset::source_bound_activation_reset_recovers_and_replays
```

The fixture uses `canisters/audit/root_probe/activation.toml`: one Workload, one
Ready reserve, and sealed production Coordinator, Root and Store artifacts.
The same production initializer installs the Store under a later provisioning
operation while the Root retains its original initialization. Real publication
and Component activation then reach Published with the Root still Prepared:
the expected Store activation identity does not match. The source journal
contains actual applied protocol receipts and an issued provisioning effect;
no completed host journal is manufactured.

The replacement review selects a distinct sealed build and passes the existing
source, installed-code, controller, inventory and activation-authority checks.
The positive journey proves:

- Review leaves the source plan, journal and state bytes unchanged. Applying
  the wrong review digest returns typed `PlanDigestMismatch` before Stop.
- The first preparation Stop completes on PocketIC but its transport response
  is lost. A newly constructed production adapter reconciles that effect.
- A controlled 10-billion-cycle fixture credit after that Stop creates a
  positive observed credit. Production conservation accounts for it separately
  from operator debit and new funding, which remain zero. Original journal
  starting balances remain unchanged.
- Adoption archives the exact three source documents. Preparation replay
  applies zero effects and preserves the Stop receipts, starting balance and
  credited amount. Fresh observations can consume cycles; replay does not
  promise an identical final live balance.
- Root-reset review retains the preparation's operation identity. The real
  reinstall completes but its response is lost. A new adapter resumes the same
  reset and completes terminal conservation. Exactly one Root reinstall occurs;
  the terminal replay applies zero effects. Operator and Root Ledger balances
  remain unchanged through reset and replay.

## Validation

The single governed PocketIC case passes in
`/tmp/canic-activation-reset-ic.log`. Its test body took 106.65 seconds; this is
test-run context, not a performance comparison. The runner owns the pinned
PocketIC 16.0.0 server and invokes native ICP CLI 1.5.0. Artifacts use the
existing sealed local fixture builds and the workspace lock (IcyDB 0.257.12,
ic-memory 0.13.3, ic-query 0.43.0).

Final all-target/all-feature warning-denied Clippy for
`canic-testing-internal` passes in
`/tmp/canic-activation-reset-final-clippy.log`. The pure governed catalog
ordering/membership regression passes in
`/tmp/canic-activation-reset-inventory.log`. Changed-file formatting, layering,
current-document semantics and whitespace checks also pass.

## Limits and remaining batch

The deliberate credit proves bounded accounting of an observed movement; it
does not attribute Toko's original increase to an IC refund, mint or other
mechanism. Existing native rejection tests cover excessive/incomplete credits
and ineligible scopes. The IC case ends at the Root-reset prerequisite; it does
not claim subsequent full Ensure convergence for this exact conflict, nor live
staging recovery. Other existing journeys own full reinstall convergence.

The complete .17 batch remains open for exact operator ICP mint-credit/fee
receipts, live child-grant usage/reservations in startup/recovery forecasts, and
CANIC-156's whole-recovery funding preview. B1 remains accepted subsequent work.
This evidence is not an immutable release-validation receipt or push approval.
