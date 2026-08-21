# Canic 0.108 Implementation Status

Date: 2026-08-21

## Status

- State: the 0.108.0 checkpoint is source-ready for the maintainer-owned
  release flow. It combines accepted M0 evidence, complete B2/M1 protected
  policy and the urgent fresh-Fleet deployment corrections. B3/M2 and later
  runtime-funding batches remain pending and have no staged production source
  in this checkpoint.
- Runtime impact: B1/M0 remains test-only. B2 hard-cuts protected funding
  policy into Fleet input, plan/init/root/Registry authority and removes the
  unreachable generic application-configured refill path. It adds no grant,
  timer, treasury ledger or public command/status implementation.
- Predecessors: completed 0.103 and 0.104, accepted 0.106 B1 baseline and
  completed 0.107 fresh-Fleet preflight/runtime admission, plus the
  current root cycle/external-call ownership inventory and proposed cost
  envelope are available. The 0.107 closeout and complete M0 evidence are
  accepted. The held 0.106 B2 external work does not gate this line.
- Successor: 0.109 estate implementation remains blocked until this line is
  complete.
- Surface posture: funding adds no Root/Coordinator command or status variant,
  request, acceptance, acknowledgement, retry, admin or reclamation method in
  0.108.0. Protected policy is nested authority data only.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | M0 recovery and admission proof | Minimal Coordinator/root atomicity proof, current/last-result model, offline break-glass authority and measured request/refill floors | Focused PocketIC value-transfer plus bounded pure/host proof | Accepted 2026-08-21 |
| B2 | Protected policy hard cut | Fleet-input schema-1 policy, validation, hashing, propagation and generic refill sediment removal | Host/config/hash, Candid-containment and final payload-bound tests | Complete; included in the 0.108.0 checkpoint |
| B3 | Coordinator grant authority | Registry-bound decisions, treasury windows, reserve, intents, receipts and attached-cycles call | Policy, authority and replay tests | Pending; no production source staged in 0.108.0 |
| B4 | Root acceptance and request journal | Exact acceptance, zero-accept replay and monotonic current/last-result replacement | Root state/restart and PocketIC response-loss tests | Pending |
| B5 | Root timer integration | Low-balance request ownership, nonterminal resumption and unchanged descendant funding | Timer/policy/restart tests | Pending |
| B6 | Manual and automatic ICP refill | Protected policy, cumulative budget, floor, terminal fallback and mutual exclusion | Ledger/CMC PocketIC journeys | Pending |
| B7 | Operator and lifecycle completion | Direct top-up resolution, status, metrics, Medic, runbooks, draining fences and generated surfaces | CLI/host/lifecycle/snapshot checks | Pending |
| B8 | Qualification and closeout | Real value-transfer and fallback journeys, measured guidance, sediment audit and closeout | Targeted repository gates and PocketIC evidence | Pending |

Eight batches match the design's M0-M7 dependency boundaries. They are not
preassigned patch releases.

## Next Authorized Action

Run the maintainer-owned validation, version, tag and push flow for the
0.108.0 checkpoint, then rerun the downstream fresh Toko Fleet installation
against the published release. The combined
[M0/M1 evidence](../../audits/working/0.108-coordinator-backed-root-funding/README.md)
records the protected policy and measured 42.2B-cycle request floor. B3/M2
starts only after this checkpoint; the held 0.106 B2 work remains independent.
