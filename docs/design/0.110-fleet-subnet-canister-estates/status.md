# Canic 0.110 Implementation Status

Date: 2026-08-18
Last updated: 2026-08-23

## Status

- State: accepted and scheduled as application-safety/estate step 8.
- Runtime impact: none from this planning cut.
- Predecessors: completed 0.103 and 0.104, accepted current 0.106
  qualification, completed 0.107 deployment readiness, completed 0.108
  Coordinator-backed root funding and completed 0.109 Fleet-wide ingress
  admission.
- Implementation approval: none. B1 additionally requires explicit
  maintainer promotion after both predecessor reports are current.
- Successors: accepted closeout feeds the 0.111 stateful adoption inventory and
  later 0.112 generic observatory; it authorizes neither successor by itself.
- Surface posture: estate policy, maintenance, transfer and operation
  observation add Root command/status variants only. Creation, reset,
  reservation, controller and reconciliation phases add no methods. Each
  status variant authorizes before protected estate or operation state is read.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Indexed estate authority | Hard-cut policy, counters, indexes, pagination, transitions and scan removal | State/property, first-excess and corrupted-index tests | Blocked on 0.103, 0.104, 0.106, 0.107, 0.108, 0.109 and promotion |
| B2 | Stateful application retirement evidence | Opt-in acknowledgement, immutable removal binding, bounded opaque receipt, restricted Draining access, forced-removal tombstone and exact retry | State, authority, interruption, upgrade and PocketIC stateful-retirement journeys | Pending |
| B3 | High-throughput maintenance | Durable creation/reset lanes, reservations, scheduling, pause/resume and recovery | Policy/replay and bounded PocketIC concurrency | Pending |
| B4 | Reserve Fleet composition | Empty topology, explicit roots, reference profile, dry-run cost and operator flow | Plan/help/config fixtures and disposable install | Pending |
| B5 | Same-Subnet cross-Fleet transfer | Two-sided reservation, controller saga, retry, tombstones and fresh logical identity | Authority/interruption two-Fleet PocketIC journeys | Pending |
| B6 | Balance and claim funding | Standby minimum, fixed claim top-up, balance separation and diagnostics | Funding/replay and balance journeys | Pending |
| B7 | Large-estate qualification and hard cut | 10/100/1,000 measurements, qualified/forced recycling, cross-App reuse, generated surfaces and cleanup | Targeted repository gates plus approved real journey | Pending |

Seven batches fit the normal minor-line planning range. They are not preassigned
patch releases.

## Next Authorized Action

No 0.110 mutation is authorized. Keep the design current while 0.103 through
0.109 complete; then review the measured limits before promoting B1.
