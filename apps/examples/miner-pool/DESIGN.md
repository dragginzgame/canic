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
If a batch needs a new worker, its round stays bound while Root is unavailable
or the allocation is pending. When the worker becomes ready, the hub records
that original batch before it processes a later goal. A request to park every
miner can therefore finish one outstanding attempt.
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
by reading the original hub's receipt for either a directly created worker or one originally claimed from the reserve. A confirmed release stays quarantined
and consumes capacity. A controller can issue `reset-worker` with that exact target and a deadline
within ten minutes. Repeat the same call to observe stop, issue uninstall and
verify the empty module. Retained snapshots block reset. Reset progress survives
Root upgrade. After observing the empty canister, Root checks the configured
balance floor and zero snapshot memory, starts it and checks again before
returning it to inventory. Repeating the original reset returns its receipt
even after a new installation. A new worker can claim the same principal
with a fresh generation. The installed tests exercise both origins with Root and Store. A directly created worker still consumes its original creation budget after release, reset and reuse.

The hub uses `canic::worker_pool!("miner")` for assignment and lookup. Each additional worker component
gets its own `[[apps.miner-pool.fleets.local.worker_pools]]` table. A second
pool under the same parent starts at ordinal zero with its own grant. Root
charges both pools against the same subnet limit.

The local worker grant binds a reuse floor of 500 billion cycles. A change
of even one cycle conflicts with the existing approval. Omitting
`minimum_reuse_cycles` preserves the prior worker install and leaves recycling
unapproved. Both forms deliver only the exact allocation generation to the miner.

## Keeping spare canisters ready

A sharding Root can maintain a fleet reserve even when the worker hub’s own
creation budget is zero. In a fleet with a bound Root and an approved subnet
ceiling, add the optional table below to `canic.toml`. Select an absolute
spending deadline in Unix nanoseconds. The example timestamp is illustrative.

```toml
[apps.miner-pool.fleets.local.standby_pool]
root = "root"
min_available_spares = 2
maximum_creations = 3
expires_at_ns = 1_800_000_000_000_000_000
minimum_import_cycles = 500_000_000_000

[[apps.miner-pool.fleets.local.standby_pool.imports]]
network = "local"
principals = ["<first-canister-principal>", "<second-canister-principal>"]
```

Replace the placeholders with surplus canisters you control. Omit the imports
table when there are no candidates. Each group requires `network = "ic"` for
mainnet or `network = "local"` for a local fleet. When the previous fleet is
known, put its `previous_root` on the group; use separate groups for different
former Roots.
Review the resolved principals, spending ceiling and deadline before submission.
Set `IDENTITY` to the controller signer name and run these commands from the configured fleet directory.

```sh
canic --json fleet plan --identity "$IDENTITY" > reserve-plan.json
canic fleet call --canister root --identity "$IDENTITY" configure-subnet-capacity "$(jq -c '.capacity.approval' reserve-plan.json)"
canic fleet call --canister root --identity "$IDENTITY" configure-standby "$(jq -c '.standby.approval' reserve-plan.json)"
canic fleet call --canister root --identity "$IDENTITY" observe-standby
```

The target counts ready spares once across the whole subnet fleet. Permanent
children, shared shards, dedicated instances and workers all claim from it at
install admission. A hit skips canister creation; Wasm installation and readiness
still run. `canic fleet show` reads Root’s aggregate counts without signing.
A missing Root or unavailable query leaves the ready count unknown. Root retains one charge for each automatic
creation, including an uncertain one, and stops dispatching at its creation
allowance, spending deadline or physical subnet ceiling. Manual refills have
their own approvals. A fleet can import with `maximum_creations = 0`.

After approval, an existing controller must hand the **running, uninstalled**
candidate to Root as its sole controller. Both subnet Registry lookups must
be available. A declared former Root must also send its release to the
destination. The controller then runs the following command twice, once to
begin quarantine and again after two hours to verify and admit the canister.

```sh
canic fleet call --canister root --identity "$IDENTITY" import-standby "$PRINCIPAL"
```

A stopped or underfunded candidate stays quarantined. For a low balance, top it
up and repeat the same import. Restarting a stopped candidate requires its IC
controller, which at this point is Root. The import command only observes it.

Canic validates each import group against the fleet network before planning.
The selected local fleet supplies the replica and trust key.

Append additional candidates to the original list and review and submit the
updated approval. Exact retries preserve earlier handoffs and spending.
`make miner-test` exercises this workflow with a local simulator.

### Moving an empty spare between managed fleets

Approve the destination import with the source principal as `previous_root`.
From the source fleet, call `export-standby SPARE DESTINATION_ROOT DEADLINE_NS`
as its controller. Choose a deadline within ten minutes. The source checks the
destination reservation and subnet, reserves the spare, and issues one
sole-controller change. While pending the spare still consumes source capacity
and cannot be claimed by a source worker.

Call `import-standby SPARE` on the destination to verify sole empty custody and
start quarantine. Repeat the original export on the source to record confirmation
and free its slot. The destination still waits two hours before admission.
An uncertain controller change stays issued. If the deadline passes before
dispatch, a reserved spare stays blocked for operator investigation.
