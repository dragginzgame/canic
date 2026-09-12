# CANIC-165 automatic application consumer

Canic now delivers an installed fixture through one registered synchronous
application importer and a retained native watchdog. The application owns its
checkpoint and durable completion receipt. Receipt-gated placement and the
funding/retention lifecycle remain unfinished; fixture-bearing Fleet generation
stays disabled.

## Maintained behavior

Registration runs from the existing synchronous lifecycle participant after
restoration. Runtime activation and same-release restart reconstruct the timer
without invoking import callbacks during lifecycle. The facade exposes
registration and observation; delivery has no manual-step API.

One attempt initializes an absent checkpoint, fetches and applies one exact Store
chunk, or performs one bounded stored-data validation step. The source await
retains a heap lease and durable attempt fence. Before applying bytes, Canic
rechecks the attempt, lease, runtime, exact installed assignment and unchanged
application progress, then verifies chunk length and digest.

The native watchdog pre-arms recovery in a separate IC message. A callback trap
therefore cannot strand scheduling behind a lost running callback. The actual
first attempt with an ordinary once registration exposed that failure; the
watchdog correction passes the same recovery journey. Successful steps request
immediate continuation. Transient readiness/source failures respect the existing
provisioning backoff and are revisited on Canic's recovery cadence. Expired
attempts invalidate the old heap lease before a successor can fetch.

Mutating callback errors and invalid postconditions trap, rolling back rows and
checkpoint together. Read-only application progress may return `NotReady` while
its database recovers. Permanent returned failures retain typed application,
authority, source or runtime diagnostics in the existing async recovery record
and stop automatic work across restart. The complete record's measured maximum
is 751 bytes. No application row cursor or provider state is persisted there.
A mutating callback's trapped error code remains a trap diagnostic, not a durable
failure record; that work is uncertain and watchdog-recoverable.

Receiving all chunks remains Pending until bounded application validation commits
the exact receipt binding and expected summary. Completed replay observes the
receipt without another Store read. Application code owns schema semantics and
bounded validation of actual stored data.

## Qualification

- [Twenty native checks](canic165-consumer-evidence/native-tests.log) pass for
  exact progress/receipt binding, single registration, heap/durable attempt
  fencing, restart, stale completion, permanent failure retention and the
  measured stable encoding bound.
- [Scoped Clippy](canic165-consumer-evidence/clippy.log) passes with warnings
  denied for core, the IcyDB probe and the changed integration/guard targets.
- All five [Store/lifecycle PocketIC cases](canic165-consumer-evidence/pocketic.log)
  pass in 121.29s; the runner takes 164s including compilation and setup. Three
  automatic targets cover returned-error traps, skipped checkpoints and invalid
  final receipts, each with a stopped Store followed by recovery. Same-heap
  recovery and fresh-heap restart complete without manual delivery calls.
  Completed replay survives grant revocation and another restart. A fourth target
  retains its permanent authority failure after restart clears the injected fault.
  The four existing commit, transport, Store and lifecycle cases still pass.

All eighteen [timer/memory guard cases](canic165-consumer-evidence/guards.log)
and final guard-only Clippy pass. Reconciliation
owns both declaration and scheduling, so direct method actions need only remain
inside registered owners. The memory ABI guard now checks actual Rust tokens,
avoiding false positives from constants, comments and example strings. Its native
VectorMemory fixture is explicitly test-only; raw runtime memory calls remain
rejected.

[Commands](canic165-consumer-evidence/commands.json) and the
[1,631-input snapshot](canic165-consumer-evidence/source.sha256) record the runtime
boundary. All recorded hashes and the source inventory stayed unchanged during
final PocketIC qualification. The subsequent timer-guard-only correction has its
own targeted check; runtime source is unchanged. These are working-tree checks,
not a complete release gate or production-capacity measurement.

## Remaining batch

Connect receipt observation to placement, application dispatch and deployment
completion without blocking initial Hub/Shard bootstrap. Qualify later Shards,
interrupted grants, reviewed retry funding, revocation before reuse, source
reference release and terminal Store retirement. The current conservative GC
block is not a completed retention contract. Held-response/reinstall fencing
through the automatic owner still needs its combined acceptance evidence; earlier
probe transport evidence alone does not establish that complete integration.

The combined worktree is not push-ready. CANIC-165 remains in the unassigned
Unreleased note, while the separate .15 operator draft includes the guard repair.
No broad gate, version transaction, publication, deployment or sibling mutation
ran.
