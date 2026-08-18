# Canic 0.107 Implementation Status

Date: 2026-08-18

## Status

- State: accepted and scheduled as application-safety/estate step 5.
- Runtime impact: none from this planning cut.
- Predecessors: completed 0.103 and 0.104 plus the accepted 0.106 B1 baseline,
  current root cycle/external-call ownership inventory and current-cost
  assumptions must be available before B1 begins. Those exact outputs and a
  passing B1 proof gate every mutating batch; 0.106 B2 does not.
- Successor: 0.108 estate implementation remains blocked until this line is
  complete.
- Surface posture: funding adds Root/Coordinator command and status variants
  only. It adds no request, acceptance, acknowledgement, retry, admin or
  reclamation method. Each status variant authorizes before treasury, policy,
  receipt or operation state is read.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Attached-cycles recovery proof | Minimal Coordinator/root proof, intent/call/receipt interruption, replay refund and measured headroom | Focused PocketIC value-transfer proof | Pending completed 0.103/0.104 and accepted 0.106 B1 baseline/ownership/cost evidence |
| B2 | Protected policy hard cut | Fleet-input schema-1 policy, validation, hashing, propagation and generic refill sediment removal | Host/config/hash tests | Blocked on accepted 0.106 B1 outputs and own B1 |
| B3 | Coordinator grant authority | Registry-bound decisions, treasury windows, reserve, intents, receipts and attached-cycles call | Policy, authority and replay tests | Pending |
| B4 | Root acceptance and request journal | Exact acceptance, zero-accept replay, acknowledgement and reclamation | Root state/restart and PocketIC response-loss tests | Pending |
| B5 | Root timer integration | Low-balance request ownership, nonterminal resumption and unchanged descendant funding | Timer/policy/restart tests | Pending |
| B6 | Manual and automatic ICP refill | Protected policy, cumulative budget, floor, terminal fallback and mutual exclusion | Ledger/CMC PocketIC journeys | Pending |
| B7 | Operator and lifecycle completion | Direct top-up resolution, status, metrics, Medic, runbooks, draining fences and generated surfaces | CLI/host/lifecycle/snapshot checks | Pending |
| B8 | Qualification and closeout | Real value-transfer and fallback journeys, measured guidance, sediment audit and closeout | Targeted repository gates and PocketIC evidence | Pending |

Eight batches match the design's M0-M7 dependency boundaries. They are not
preassigned patch releases.

## Next Authorized Action

Complete 0.103 and 0.104 and finish and accept 0.106 B1's baseline, current
root ownership inventory and current-cost assumptions. Then run only 0.107
B1's bounded PocketIC proof; do not begin runtime or stable-state mutation
until those exact outputs are accepted and 0.107 B1 passes. 0.106 B2 remains
independent of this gate.
