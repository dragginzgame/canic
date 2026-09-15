# CANIC-172 child reserve diagnostics and recovery

This is one completed diagnostic/recovery slice in the open 0.110.17 draft,
on the source identities recorded in [the feedback review](toko-recovery-172-175.md).
It does not complete CANIC-172's supplementary operator funding workflow.
The downstream ledger remains unchanged through CANIC-175 and was read only.

## Correction

The deployment owner maps its exact `CycleReserveRejected` variant to E163,
`DEPLOYMENT_CYCLE_RESERVE_REQUIRED`. Pool supply and other capacity failures
keep their current diagnostics. The 1T reserve check is unchanged.

Child allocations retain one bounded failure in their existing Registry row.
The protected allocation response includes its diagnostic, failure time,
consecutive failure count and next retry deadline. Record/view/response
conversion stays with ops. No new stable store or compatibility lane is added.
Admission charges the maximum encoded diagnostic slot up front; later failure
updates do not change Registry capacity counters, reservation authority,
partition heads or paid-effect receipts.

The driver records failures and uses the existing exponential backoff policy,
capped at 60 seconds. Successful observable work or completion clears the
failure. An unchanged result does not clear it; diagnostic metadata itself is
excluded from progress comparison. The original error is still returned to a
direct caller. Retry identity remains the original Component and operation.

## Qualification

Three native cases pass for persistence/restart, unchanged work and capacity,
typed failure projection, clearing after creation progress, preserved effect
records and rejection of wrong-operation/backwards-time/overflow updates
without mutation. Scoped all-target/all-feature Clippy passes for core,
control plane, host, internal testing and the audit Root. All seven selected
child lifecycle/capacity regressions pass, as do the four diagnostic-register
checks, layering and scoped formatting. The pure governed inventory guard
passes with Fleet restore and autonomous Root removal still first.

One registered governed PocketIC case passes on final code. It activates the
existing managed-group fixture, explicitly prepares a sufficient Ready pool
asset, pauses funding and reduces only the disposable audit Root's balance to
800B. A new User Shard allocation exposes E163, stays Reserved with no creation
or installation evidence, and retains one Claimed pool asset. Adding 5T to
that disposable Root lets the same canister reach readiness. Exact command
replay preserves the allocation response and leaves one Workload for the claim.
The final log is `/tmp/canic-172-pocketic-final.log`.

The first probe instead reached E85 in the earlier readiness/maintenance path,
before establishing the deployment reserve rejection; no sufficient Ready
asset had been established. E85 alone does not identify that inner cause.
Explicit import initially raced the
fixture's existing pool reset and returned E132. Setup now reconciles only
that typed conflict within a bound and verifies Ready state before lowering
Root balance. These fixture corrections do not weaken either production guard.

The local-only, controller-authenticated balance control belongs to the
existing `root_probe` audit canister. It is absent from generated production
Roots. The test reuses existing artifact builders and the governed IC runner;
no full suite, live funding, sibling mutation or Git publication ran.

## Initial child origin propagation

Generic parent activation failures now consult the same Prepared Component's
unfinished initial allocations. The first failed child in stable operation
order supplies `ComponentChildAllocation`, its exact operation ID, retained
diagnostic/time and the Root as execution owner. This includes a Reserved
allocation with no child canister yet. Explicit authority failures and existing
origins remain authoritative. Active Components, dynamic children and terminal
allocations do not supply an origin. No record, effect or retry state changes.

Five focused child failure cases pass, including unchanged capacity/receipts,
outer-context preservation and exclusions, in `/tmp/canic-child-origin-tests.log`.
The new registered IC case passes in `/tmp/canic-child-origin-ic-final.log`.
After top-level Components commit, it lowers the disposable Root to 800B.
An initial child retains its claim with E163 and no creation/install evidence;
the Coordinator reports that child operation and Root owner. Adding 5T lets
the same claimed canister become Ready and clears the pending failure. This
uses the existing managed-child fixture, including its other Component roles.

The same case now also reads the retained failure through the production
`IcpEnsurePlatform` observer and the real ICP transport. A selected test
operator is explicitly added to the disposable Coordinator's controllers. The
observer reports the exact child operation, Root target, E163 and retry category
before the same-claim recovery. The existing management-agent PocketIC builder
supplies the HTTP trust root and aligned initial clock; the earlier non-HTTP
fixture lacked that trust root. No host journal or funding receipt is invented
for this observation check. Final IC and affected-package Clippy pass in
`/tmp/canic-child-origin-host-ic.log` and
`/tmp/canic-child-origin-host-clippy.log`.

The first attempt stopped at the test balance endpoint's production activation
fence, before setting up the low reserve. The audit canister now uses raw IC
test instrumentation with an explicit local-build/controller guard, as the
existing fixture hold endpoint does. The passing case rejects a non-controller
before allowing the controller to prepare the balance during partial activation.
Generated production Roots have no such endpoint. No production fence changed.
Its final return contract is the plain cycle amount; guard failures reject the
call. This removes an unnecessary result wrapper and its lint suppression.

The shared deployment floor's startup/planner and seven native funding tests
also pass. Final scoped Clippy, terminal-child exclusion on the existing full
allocation lifecycle fixture, and governed inventory checks pass in
`/tmp/canic-origin-floor-final-clippy.log`, `/tmp/canic-child-terminal-origin.log`
and `/tmp/canic-origin-inventory.log`. The independent Candid equality check
passes in `/tmp/canic-child-origin-candid.log`. Child failure timestamps remain
in child status; the aggregate Root/Coordinator retry record keeps its existing
local observation timestamp semantics.

## Remaining work

The 800B, 5T and pool replenishment values are disposable test inputs, not
operating guidance. PocketIC replenishment does not qualify operator mint,
Ledger fees/withdrawal receipts or terminal conservation in this child journey.
The subsequent [combined host funding proof](canic-172-native-funding.md#same-child-claim-and-native-withdrawal--2026-09-15)
now recovers the same E163 claim through a real native withdrawal, two lost
responses, terminal conservation and replay. It uses a local zero-fee Ledger
stub; the separate synthetic-minimum case covers nonzero fees. Exact operator
mint receipts remain outstanding. Startup prepayment and the CANIC-174 deadline
correction are recorded in the [funding report](canic-174-funding-deadline.md).
The original nine-Workload/24-asset Toko staging estate remains downstream
qualification; the complete accepted release batch is not push-ready.
