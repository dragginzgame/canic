# Miner pool

**Runnable on the local simulator with `make miner-test`.** A pool canister accepts a target number of toy blocks per round. An explicit tick drives the demonstration. Each miner is a real worker canister running at most two bounded batches per round, with one toy block per completed batch. Use synthetic cycles. The app defines the work, goal and scaling decision. Canic enforces the approved ceiling of four workers and the creation budget, and retains worker identities and management results.

| Goal per round | Expected observation after scaling settles |
| ---: | --- |
| 2 | One miner produces two blocks. |
| 5 | Three miners produce two, two and one. |
| 1 | Two miners drain their issued work and park. One remains active. |
| 5 | The same two parked IDs return to work. |
| 10 | Four miners produce eight blocks and the report identifies the capacity limit. |
| 0 | All miners finish issued work and park. The accumulated total stays visible. |

Show each worker's ID, active/draining/parked state, last batch and production count. Lose a batch reply and recover its original result. The batch counts exactly once. Parking retains the worker's ID and stops new dispatches to it.

Keep this exercise within one fleet. Cross-fleet reuse needs separate evidence that the old fleet relinquished authority and the new fleet receives clean state.

## Run the fleet

From the Canic checkout, prepare the local simulator once and run the installed worker and fleet tests:

```sh
make qualification-setup
make miner-test
```

The test builds the miner through the app config. Testkit supplies separate
Root and pool caller identities. The miner admits work from its retained pool.
Each request names a round and an ordinal (1 or 2). Repeating either completed
batch in the current round returns its original block number. An older round,
a skipped ordinal or a third batch is refused. The pool must settle its pending
round before starting another on that worker. Miner upgrades reopen both the
parent binding and the last round. This test exercises the worker independently
of Root allocation.

The fleet test installs Root and Store, publishes both application Wasms,
and has Root install the pool. It binds the local fleet declaration to those
identities and gives the grant an explicit expiry. It submits the exact capacity
and worker approval bytes produced by the Host planner. The hub then requests
workers and executes the table above. Both hub and Root upgrade after round
two. Worker IDs, grants and production survive. Parked workers still consume
their original slots. The application uses an explicit operator tick for each
step; this example has no background scaling timer.

## Drive an installed pool

In a bound local fleet with the hub installed and its worker grant approved,
use the identity supplied as operator at hub install. From the fleet directory:

```sh
canic fleet call --canister pool --identity operator tick 1 5
```

Each confirmed call advances one step. Repeat the same round and goal after
a confirmed reply until the hub reports `settled` or `capacity limit`. A new
goal takes a higher round number. If Canic reports an uncertain ingress result,
observe the original operation before submitting another tick. The public
`report` query exposes each worker’s ID, dispatch state and cumulative blocks.
The simulator test prints those values after each round.

Tick replies identify the pending work. For example, `worker 0: Root allocation pending`
means Root accepted the first ordinal and the hub is waiting for readiness.
`worker 0: Root call rejected` identifies a rejected placement call; check the
hub's grant. `worker 0, batch 7/1: call rejected` identifies the issued batch
that still needs a receipt. The hub can reopen after upgrade, report that miner
as draining when the goal falls, and finish the original batch when the miner returns.
Its block counts toward the worker's lifetime total and its original round only
if that round is still current.

## Pool ownership

The operator approves capacity and funding at Root. An explicit application
tick requests additional workers through Root. Shrinking the goal drains
issued batches before the pool parks workers; parking preserves the allocation
and principal for reuse in this fleet. A parked worker still consumes Root
capacity. No tick deletes a worker or transfers it elsewhere.

For example, when the operator sets the goal to five blocks per round, the
pool hub needs three active miners. If it already has one active miner and two
parked miners, it resumes those two without a creation request. If it has only
one miner, it requests two additional allocations under its fleet-approved
allowance. Root authenticates the calling hub, checks the approved worker
release, deadline, creation budget and remaining subnet capacity, and retains
each creation result. The hub issues batches only to confirmed ready allocations.
A lost creation reply must resolve to the original allocation before the hub
can count another worker. Lowering the goal changes dispatch at the hub; it
does not refund the original creation allowance.

Legacy at `377fbce` ran the scaling policy before calling the common allocation
workflow. Its regression invalidated a registered worker when the same principal
received another allocation identity. Preserve that check when binding workers
to the pool. The application owns its production goal and block receipts.
[Creation workflow](https://github.com/dragginzgame/canic-legacy/blob/377fbce/crates/canic-core/src/workflow/placement/scaling/mod.rs)
and [allocation reuse regression](https://github.com/dragginzgame/canic-legacy/blob/377fbce/crates/canic-core/src/ops/storage/placement/scaling/tests/mod.rs).
