# Canic 0.110 Implementation Status

Date: 2026-08-18

## Status

- State: accepted and scheduled as the production-safety gate after 0.109.
- Outcome: one whole-Fleet, stop-the-world transition from one exact released
  predecessor to one successor, preserving Principals, application stable
  memory and Fleet identity.
- Runtime impact: none from this planning cut.
- Implementation approval: none. B1 requires completed 0.109, an immutable
  released predecessor and explicit maintainer promotion.
- Exception scope: one named predecessor/successor only. No rolling,
  mixed-version, arbitrary-old, downgrade or generic compatibility path.
- Downstream posture: Prequel Wars and IcyDB remain read-only evidence sources;
  stateful managed demo Fleets remain disposable until this line is published
  and adopted downstream.

Design: [Stateful Fleet release adoption](0.110-design.md)

## Release-Batch Tracker

| Batch | Outcome | Included evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Exact predecessor and transition contract | Released schemas/artifacts, role order, operation fences, backup/dry-run contract and ceilings | Reproducible inventory and explicit acceptance | Blocked on 0.109, released predecessor and promotion |
| B2 | Backup and dry-run qualification | Complete snapshot manifest, copied conversion, invariant failures and zero live mutation | Backup, corruption, first-excess and dry-run tests | Blocked on B1 |
| B3 | Role-local stable conversion | Exact Coordinator/root/Store/managed conversions, receipts and atomic traps | State/property and lifecycle rollback tests | Blocked on B2 |
| B4 | Whole-Fleet orchestration | Fence, stop/upgrade order, journal, response-loss recovery and convergence | Host fixtures and multi-role PocketIC interruption matrix | Blocked on B3 |
| B5 | Stateful application composition | Principal/data preservation, 0.104 participant, 0.105 auth policy and 0.109 retirement evidence | Stateful Canic+IcyDB PocketIC journey | Blocked on B4 |
| B6 | Security/performance closeout | Unsupported predecessor denial, bounds, forward recovery, docs and residue cleanup | Targeted repository/security/performance gates | Blocked on B5 |

## Admission Boundary

B1 must freeze one exact released predecessor, every stable owner and encoding,
all nonterminal external-effect fences, the whole-Fleet stop/upgrade ordering,
verified backup/dry-run evidence and the forward-recovery boundary. Any
unknown role, corrupt snapshot or uncertain paid/controller-changing effect
blocks before mutation.

## Next Authorized Action

No 0.110 work is authorized by this scheduling cut. Finish 0.109, select the
exact released predecessor, then request explicit B1 promotion. Stateful
production claims remain blocked until all six batches close.
