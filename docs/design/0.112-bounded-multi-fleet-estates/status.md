# Canic 0.112 Implementation Status

Date: 2026-08-30

## Status

- State: accepted reorientation; implementation not promoted.
- Outcome: indexed Root-local estates, an ordinary reserve Fleet and one
  same-Subnet single-asset transfer between exact Fleets.
- Runtime impact: none from this planning amendment.
- Predecessors: accepted 0.110 contraction/stateful-retirement closeout and
  accepted 0.111 exact-transition closeout.
- Implementation approval: none. B1 requires explicit maintainer promotion.
- Downstream steering: Toko Miner is read-only evidence; the qualification
  fixture is repository-owned.
- Budget posture: 0.110 size, build, operator and validation budgets remain
  binding.
- Deferred: high-throughput lanes, transfer batches, broad funding automation,
  1,000-canister qualification and the generic Observatory.

Design: [Bounded multi-Fleet estates](0.112-design.md)

## Release-Batch Tracker

| Batch | Outcome | Direct evidence | Status |
| --- | --- | --- | --- |
| B1 | Indexed Root-local estate | Bounded policy, counters/indexes, corruption and scan removal | Blocked on 0.111 closeout and promotion |
| B2 | Ordinary reserve Fleet | Empty topology, explicit Roots, no-effect plan and authority isolation | Blocked on B1 |
| B3 | Single-asset same-Subnet transfer | Two-sided reservation, controller convergence, response loss and tombstones | Blocked on B2 |
| B4 | Funding and conservation | Separate cycle classes, standby/claim bounds, exact debit and replay | Blocked on B3 |
| B5 | Two-Fleet multi-Subnet qualification | Two Roots per Fleet, one co-located transfer, isolation and effect-free replay | Blocked on B4 |
| B6 | Security, budgets and closeout | Wrong-authority rejection, generated surfaces, inherited budgets and immutable audit | Blocked on B5 |

## Scope Boundary

The first transfer cardinality is one. It cannot move an asset across Subnets,
carry application data/Wasm/logical identity or infer Fleet authority from
controllers. Fleet Ensure owns the reviewed host plan; Root records and live
management evidence own recovery.

## Next Authorized Action

No implementation is authorized. Finish and accept the exact 0.110 and 0.111
predecessors, then explicitly promote B1.
