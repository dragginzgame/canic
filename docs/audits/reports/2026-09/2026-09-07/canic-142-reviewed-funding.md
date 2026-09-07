# CANIC-142: Reviewed Funding Resume

Date: 2026-09-07. Scope: the open 0.110.9 Canic source batch.

The maintainer chose resumable funding and deferred CANIC-141. This correction
keeps Fleet Ensure as the sole owner. It does not add a runtime mode, custody
canister, compatibility path or another deployment identity.

## Contract

An `EstateFundingRequired` journal remains `InProgress`. Ordinary planning
retains its original plan and exposes an additional `funding_review` containing
the exact pause, Root account, Ledger, transfer, fee and review digest. Planning
does not submit a remote effect. Applying that digest durably accepts the
transfer before invoking the existing `FundEstate` adapter.

The original operation ID, plan digest, effect ordering, issued protocol
commands, pending creation identity and initial conservation baseline remain
unchanged. The original digest cannot approve a new transfer. After an intent
has been persisted, response-loss recovery uses its same timestamp, destination,
amount and fee and accepts the Ledger's duplicate receipt. A retained receipt
prevents another transfer call when only its source observation was interrupted.
The review digest cannot authorize a later plan, including after completion of
the operation that originally retained it.

Funding review verifies the live Root's exact controllers, account, fee and
pending creation against the retained pause. A response-loss retry checks Root
control again before repeating the same Ledger request. Current Ledger balances
must reconcile with initial funds,
applied transfers and exact creation receipts before another debit is accepted.
An uncertain creation result is not authority to pay again. One review per
Root/pending-creation identity and a creation-count-derived bound prevent an
unbounded sequence of reviews for the same pause.

Terminal verification adds approved transfer amounts and fees to the original
funding bounds. It preserves the original creation count, creation debit/fee
bounds and execution-burn allowance. Root credits may already have been consumed
by autonomous creation; transfer receipts and exact creation receipts reconcile
that balance without pretending an intermediate credit remains observable.

The current host journal schema gains required `funding_reviews`; it remains
v1 and changes through the pre-1.0 reinstall hard cut. No Candid/runtime schema
changes are required. This release does not adopt predecessor host journals.

## Evidence and limits

Four focused host regressions pass: immutable operation/review binding,
fee/balance/controller/review corruption rejection, lost-transfer-response
recovery with exact conservation and effect-free funding replay, and rejection
of a completed operation's review digest against a later plan. The host fixture models
evidence and deliberately lacks a complete runtime topology; it does not claim
Fleet readiness. Log: `/tmp/canic-142-host-tests.log`.

The bounded PocketIC qualification uses the existing production-adapter setup
and a retained underforecast fixture. It creates no fabricated paid receipt or
completed protocol effect. This is a recovery-path injection, not evidence that
the normal generator or current downstream mainnet operation enters the pause.
The fixture exposed two additional dropped observations: reserve maintenance did
not project the existing Root funding pause, and the outer ICP adapter discarded
the typed protocol adapter's funding field. Both now preserve that same exact
evidence. The maintained positive Ready reserve is unchanged.

The final focused PocketIC case passes in 190.30s (221s runner), including a
32.79s artifact preparation. It uses one Workload and one Ready reserve. The
ordinary Ensure planning command exposes the new review while preserving issued
effects. Fee rejection precedes payment; lost transfer and creation responses
retain one transfer and the exact original Root creation identity. The complete
Fleet reaches readiness, reconciles its controlled cycles and replays without
another effect. Log: `/tmp/canic-142-pocketic.log`.

The affected Fleet host selection passes 147 tests (two PocketIC tests remain
ignored in that unit-test invocation), and all 10 selected Fleet CLI tests pass.
The test-registration check passes. Warning-denied Clippy passes across every
target and feature of `canic-host`, `canic-cli` and `canic-testing-internal`.
After the runtime run, the Root comparison was expressed as named authority and
review matching was tightened against later plans; the four focused funding
tests and scoped Clippy cover those final changes. Formatting and diff checks
pass. Logs: `/tmp/canic-142-fleet-host.log`, `/tmp/canic-142-cli.log`,
`/tmp/canic-142-catalogue.log` and `/tmp/canic-142-clippy.log`.

CANIC-140 and CANIC-142 are complete in the accepted open batch, which is ready
for release approval with the 0.110.9 changelog draft. Package versions remain
0.110.8. No broad validation, Git publication, version transaction or downstream
mutation was performed.

The operator guide and open changelog describe the review command. Publication,
adoption and downstream mainnet qualification remain separate. Toko Miner must
adopt the published correction, preserve the current same-release operation if
it pauses, review the reported exact shortfall, and verify terminal growth and
application readiness. CANIC-141's mixed-subnet fee authority remains deferred.
