# CANIC-175 live state cascade

This work belongs to Canic's open 0.110.17 draft on the 0.110.16 source
identity recorded in [the feedback review](toko-recovery-172-175.md).
The Toko ledger still ends at CANIC-175. Toko remains read-only.

## Correction

Root preserves each selected recipient's inventory-owned command surface:
Store inventory uses `canic_wasm_store_command`, while Component Registry
recipients use `canic_command`. Managed Components now accept the maintained
`SynchronizeState` command with exact immediate-parent authentication. Their
local direct-child directory determines further recipients. No Store endpoint
alias, generic compatibility endpoint or rollback is added.

State recovery commands remain reachable in Readonly and Stopped modes. The
three command dispatchers exempt only their state commands from the ordinary
Fleet update guard, then retain the existing controller or parent check.
Other commands keep that mode guard and its normalized denial diagnostics.
The first IC run reached and passed
propagation, partial failure, timer and retry assertions, then exposed the old
Readonly restoration barrier; the repeated case also checks unrelated commands
remain rejected in both fenced modes.

Root returns the local previous/current/change receipt separately from its
propagation report and local timer-reconciliation error. It reconciles the
funding timer after local mutation and before awaiting fanout. A failed child
therefore cannot skip this reconciliation or hide other successful calls.
Store and Component replies include their own application result and the
observations returned by their descendants. An unconfirmed call means the
response did not establish application; it does not assert that the recipient
remained unchanged.

The report's detail budget derives from the existing 16 KiB cascade budget.
This bounds the report itself; a complete command reply also carries its
command union's Candid type envelope. Full counts
survive detail truncation, including failures whose individual rows are omitted.
The bound never limits execution. Received reports must identify their called
recipient first, contain consistent counts and distinct retained identities,
and aggregate without overflow. Bootstrap continues to require a complete
report before recording its cascade evidence.

An unchanged-state command retries propagation. After each await, a sender
checks that its snapshot still matches local state before sending it to another
child. This prevents an older invocation from continuing obsolete fanout after
a newer local change. It is not a distributed atomic transaction or a rollback
promise; an interrupted or overlapping operation can still need a new retry.

## Qualification

Four focused native report tests pass: bounded detail/full failure counts,
encoded report budget, recipient/application binding and rejection of malformed
or overflowing aggregation without mutation. The two Root workflow tests pass
with ordinary package features. All 46 protocol-surface tests pass, including
structural equality between the complete canonical Store response and its Rust
type. All-target/all-feature Clippy passes for `canic-core`,
`canic-control-plane`, `canic` and `canic-testing-internal`; layering and scoped
formatting checks pass. The corrected governed IC regression passes, including
Readonly/Stopped restoration and denial of unrelated commands in both modes.
The final guard-reporting cleanup passes that same case too. Its retained log
is `/tmp/canic-175-pocketic-diagnostics.log` (one selected case passed). All 36
ordinary-feature protocol tests also pass; the complete Store response test
is gated on the optional control-plane dependency's owning features.

The new registered IC case reuses the existing active multi-role provisioning
fixture. It checks Root/Store/Hub/Shard and another managed branch, controller
and parent rejection, a stopped Shard's partial result, local timer state,
restart/retry, unchanged replay and Fleet status restoration. Its ordinary
managed role shares the maintained Component command surface; it is not Toko's
Translation application or an exact reproduction of its staging estate.

CANIC-172 native supplementary funding, CANIC-174 debit timing/forecasting,
source-bound positive-credit interruption evidence and reserve previews remain
separate accepted work. B1 measurement expansion is parked behind those fixes.
No broad validation, version bump, commit, push, deployment or live funding ran.
