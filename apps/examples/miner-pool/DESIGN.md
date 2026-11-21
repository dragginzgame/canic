# Miner pool

Run `make demo DEMO_APP=miner-pool` after `make qualification-setup`.
Set the active count with `miners 3`. Each second the terminal drives one
operator tick. Root allocates genuine worker canisters up to the configured
four-worker grant. Watch their principals appear and see individual attempts
miss or find a block. `report` reads the installed hub. Type `miners 1` to
park two, `miners 3` to reuse them, and `miners 0` to drain and park all.
Parked miners keep their principals and continue to consume allocated slots.
Exit stops the disposable simulator.

## Mining and retries

The miner hashes its principal, the round (big-endian u64) and batch ordinal
with SHA-256. A first digest byte below 64 finds a toy block. The 25% rate is
reproducible; a short session can have any number of misses. This local example
does not perform a proof-of-work search or require an entropy service.

The hub gives each active miner up to two attempts per round. A miner returns its
lifetime attempt number and hit/miss. It retains the outcomes of both ordinals
for its latest round. An exact retry after a lost reply or upgrade recovers the
original receipt. The hub confirms the expected attempt number before counting
a find. A changed target starts another round, draining any retained batch first.
The terminal starts new rounds while the target is positive. If the terminal
stops, it issues no further ticks.

The low-level `tick ROUND ATTEMPTS` call takes an attempt budget for one round.
The live prompt selects two attempts per requested worker. Installed tests use
odd budgets as well, and verify grant refusal, replay, upgrade, drain and reuse.
Run `make miner-test` to exercise these cases. This version uses fresh miner and
hub layouts and a changed toy batch reply. Start a fresh local demo.

## Pool ownership

The operator approves capacity and funding at Root. An explicit application
tick requests additional workers through Root. Shrinking the goal drains
issued batches before the pool parks workers; parking preserves the allocation
and principal for reuse in this fleet. A parked worker still consumes Root
capacity. No tick deletes a worker or transfers it elsewhere.

Legacy at `377fbce` ran the scaling policy before calling the common allocation
workflow. Its regression invalidated a registered worker when the same principal
received another allocation identity. Preserve that check when binding workers
to the pool. The application owns its production goal and block receipts.
[Creation workflow](https://github.com/dragginzgame/canic-legacy/blob/377fbce/crates/canic-core/src/workflow/placement/scaling/mod.rs)
and [allocation reuse regression](https://github.com/dragginzgame/canic-legacy/blob/377fbce/crates/canic-core/src/ops/storage/placement/scaling/tests/mod.rs).

### Worker release and standby

The pool operator can park a worker and later resume its existing batches.
Explicit release is permanent for that allocation. Release parked workers
from the end of the list, after all pending dispatch has settled. The pool
retains the exact principal, slot and generation. Its four approved lifetime
ordinals include released workers.

On a sharding Root, an exact controller refill approval creates one empty
canister and consumes one physical subnet slot. The next admitted worker
can claim it and install its approved Store release. Each mining request
includes the generation Root delivered at install. Root can confirm a release
by reading the original hub's receipt. A confirmed release stays quarantined
and consumes capacity. A controller can issue `reset-worker` with that exact target and a deadline
within ten minutes. Repeat the same call to observe stop, issue uninstall and
verify the empty module. Retained snapshots block reset. Reset progress survives
Root upgrade. After observing the empty canister, Root checks the configured
balance floor and zero snapshot memory, starts it and checks again before
returning it to inventory. Repeating the original reset returns its receipt
even after a new installation. A new worker can claim the same principal
with a fresh generation. The installed test gives the hub a zero creation
budget and exercises both generations through the real Root and Store.

The local worker grant binds a reuse floor of 500 billion cycles. A change
of even one cycle conflicts with the existing approval. Omitting
`minimum_reuse_cycles` preserves the prior worker install and leaves recycling
unapproved. Both forms deliver only the exact allocation generation to the miner.
