# Canic 0.107 Implementation Status

Date: 2026-08-16

## Status

- State: accepted and scheduled as reserve-Fleet critical-path step 5.
- Runtime impact: none from this planning cut.
- Predecessors: completed 0.103 and 0.104, accepted current 0.105 qualification
  and completed 0.106 Coordinator-backed root funding.
- Implementation approval: none. B1 additionally requires explicit
  maintainer promotion after both predecessor reports are current.
- Successor: accepted closeout unlocks 0.108 Skynet T2 observatory B1 review;
  it does not authorize 0.108 mutation by itself.
- Surface posture: estate policy, maintenance, transfer and operation
  observation add Root command/status variants only. Creation, reset,
  reservation, controller and reconciliation phases add no methods. Each
  status variant authorizes before protected estate or operation state is read.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Indexed estate authority | Hard-cut policy, counters, indexes, pagination, transitions and scan removal | State/property, first-excess and corrupted-index tests | Blocked on 0.103, 0.104, 0.105, 0.106 and promotion |
| B2 | High-throughput maintenance | Durable creation/reset lanes, reservations, scheduling, pause/resume and recovery | Policy/replay and bounded PocketIC concurrency | Pending |
| B3 | Reserve Fleet composition | Empty topology, explicit roots, reference profile, dry-run cost and operator flow | Plan/help/config fixtures and disposable install | Pending |
| B4 | Same-Subnet cross-Fleet transfer | Two-sided reservation, controller saga, retry, tombstones and fresh logical identity | Authority/interruption two-Fleet PocketIC journeys | Pending |
| B5 | Balance and claim funding | Standby minimum, fixed claim top-up, balance separation and diagnostics | Funding/replay and balance journeys | Pending |
| B6 | Large-estate qualification and hard cut | 10/100/1,000 measurements, cross-App reuse, generated surfaces and cleanup | Targeted repository gates plus approved real journey | Pending |

Six batches fit the normal minor-line planning range. They are not preassigned
patch releases.

## Next Authorized Action

No 0.107 mutation is authorized. Keep the design current while 0.103 through
0.106 complete; then review the measured limits before promoting B1.
