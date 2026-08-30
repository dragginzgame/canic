# Canic 0.111 Implementation Status

Date: 2026-08-30

## Status

- State: accepted and scheduled after the reoriented 0.110 line.
- Outcome: one whole-Fleet, stop-the-world transition from one exact released
  predecessor to one successor.
- Runtime impact: none from this planning amendment.
- Implementation approval: none.
- Predecessor: accepted 0.110 closeout, its exact released source/artifacts and
  explicit B1 promotion are mandatory.
- Exception: one predecessor/successor pair only; no rolling, mixed-version,
  downgrade, arbitrary-old or generic compatibility path.
- Downstream steering: repository-owned Toko-shaped stateful fixture plus
  read-only Toko evidence; no downstream production dependency.
- Budget posture: 0.110 Wasm, build and validation ceilings are inherited and
  may not be silently expanded.

Design: [Stateful Fleet release adoption](0.111-design.md)

## Release-Batch Tracker

| Batch | Outcome | Direct evidence | Status |
| --- | --- | --- | --- |
| B1 | Exact predecessor and transition contract | Released schemas/artifacts, multi-Root order, fences, backup/dry-run and inherited budgets | Blocked on 0.110 closeout, predecessor and promotion |
| B2 | Backup and copied-state dry run | Complete manifest, conversion, corruption/first-excess rejection and zero mutation | Blocked on B1 |
| B3 | Role-local exact conversion | Coordinator, Root, Store and managed conversions, receipts and atomic traps | Blocked on B2 |
| B4 | Whole-Fleet transition | Fence, ordered stop/upgrade, journal, response loss and multi-Root convergence | Blocked on B3 |
| B5 | Downstream-shaped stateful composition | Principal/data preservation, lifecycle, admission, local auth and retirement | Blocked on B4 |
| B6 | Security, budgets and closeout | Unsupported predecessor, forward recovery, size/build/test budgets and immutable audit | Blocked on B5 |

## Admission Boundary

B1 freezes one exact released predecessor, all stable owners, the stop/upgrade
order, every nonterminal-effect fence, backup/dry-run evidence and forward-
recovery rules. A stopped prerequisite must already have converged through its
separate reviewed plan; transition admission requires full protected Fleet
authority.

## Next Authorized Action

No implementation is authorized. Finish 0.110, accept its exact closeout,
select the immutable predecessor and explicitly promote B1.
