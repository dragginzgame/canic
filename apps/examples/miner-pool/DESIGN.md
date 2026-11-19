# Miner pool

**Planned, local simulator only.** A pool canister accepts a target number of toy blocks per round. An explicit tick drives the demonstration. Each miner is a real worker canister running at most two bounded batches per round, with one toy block per completed batch. Use synthetic cycles. The app defines the work, goal and scaling decision. Canic enforces the approved ceiling of four workers and the creation budget, and retains worker identities and management results.

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
