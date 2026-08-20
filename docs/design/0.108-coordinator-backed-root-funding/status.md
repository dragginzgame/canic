# Canic 0.108 Implementation Status

Date: 2026-08-20

## Status

- State: accepted and scheduled as application-safety/estate step 6.
- Runtime impact: none. B1 adds only a test-only attached-cycles probe and
  PocketIC evidence.
- Predecessors: completed 0.103 and 0.104, accepted 0.106 B1 baseline and
  completed 0.107 fresh-Fleet preflight/runtime admission, plus the
  current root cycle/external-call ownership inventory and proposed cost
  envelope are available. A passing B1 proof still gates every mutating batch;
  0.106 B2 does not. The passing B1 evidence is retained while production
  work waits for 0.107 closeout.
- Successor: 0.109 estate implementation remains blocked until this line is
  complete.
- Surface posture: funding adds Root/Coordinator command and status variants
  only. It adds no request, acceptance, acknowledgement, retry, admin or
  reclamation method. Each status variant authorizes before treasury, policy,
  receipt or operation state is read.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Attached-cycles recovery proof | Minimal Coordinator/root proof, intent/call/receipt interruption, replay refund and measured headroom | Focused PocketIC value-transfer proof | Ready for maintainer acceptance: focused proof passes |
| B2 | Protected policy hard cut | Fleet-input schema-1 policy, validation, hashing, propagation and generic refill sediment removal | Host/config/hash tests | Blocked on 0.107 closeout, accepted 0.106 B1 outputs and own B1 |
| B3 | Coordinator grant authority | Registry-bound decisions, treasury windows, reserve, intents, receipts and attached-cycles call | Policy, authority and replay tests | Pending |
| B4 | Root acceptance and request journal | Exact acceptance, zero-accept replay, acknowledgement and reclamation | Root state/restart and PocketIC response-loss tests | Pending |
| B5 | Root timer integration | Low-balance request ownership, nonterminal resumption and unchanged descendant funding | Timer/policy/restart tests | Pending |
| B6 | Manual and automatic ICP refill | Protected policy, cumulative budget, floor, terminal fallback and mutual exclusion | Ledger/CMC PocketIC journeys | Pending |
| B7 | Operator and lifecycle completion | Direct top-up resolution, status, metrics, Medic, runbooks, draining fences and generated surfaces | CLI/host/lifecycle/snapshot checks | Pending |
| B8 | Qualification and closeout | Real value-transfer and fallback journeys, measured guidance, sediment audit and closeout | Targeted repository gates and PocketIC evidence | Pending |

Eight batches match the design's M0-M7 dependency boundaries. They are not
preassigned patch releases.

## Next Authorized Action

Review and accept the
[B1 attached-cycles evidence](../../audits/working/0.108-coordinator-backed-root-funding/README.md).
The bounded proof passes exact intent/call/receipt interruption, foreign-caller
denial and zero-accept replay with automatic refund. It observes an exact
42,102,499,000-cycle call cost for the 1T fixture request, sub-13M Coordinator
execution overhead and sub-6M root overhead, and proposes separate 100M-rounded
execution allowances while requiring `cost_call` to be recomputed for the
final payload. Do not begin runtime or stable-state mutation until B1 is
accepted and 0.107 is complete. 0.106 B2 remains held and independent of this
gate.
