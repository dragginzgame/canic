# Canic 0.111 Implementation Status

Date: 2026-08-31

## Status

- State: held at Q0 cycle-disposition platform qualification; implementation
  not promoted and B1 is not promotable from the current evidence.
- Outcome: indexed Root-local estates, one ordinary reserve Fleet and one
  bounded same-Subnet source-disposition/destination-credit operation.
- Predecessor: accepted 0.110 contraction closeout and explicit Q0 promotion.
  B1 separately requires accepted immutable Q0 evidence.
- Hard cut: no stateful retirement, cross-release adoption, migration,
  application-data/stable-memory preservation or Principal preservation.
- Cycle rule: a finalized capsule running temporarily in the source must attach
  the exact controlled destination credit. Source deletion is impossible until
  that accepted credit, bounded observed protocol debit and bounded terminal
  residual discard reconcile under the same operation.
- Authority: both Roots prepare and activate one digest; the destination's
  atomic operation receipt proves the credit. A balance delta alone does not.
- Eligibility: only a concretely unbound, funding-fenced
  `DisposableOrdinaryComponent` may enter the operation.
- Retry: interruption resumes the same source, destination and amount; it does
  not select an alternative or blindly repeat credit.
- Budget posture: every later runtime batch retains 0.110 artifact, build,
  validation and fresh measurement-decision gates, including a refreshed named
  post-`-Oz` report whenever generic/type-instantiation fanout changes.

Design: [Bounded multi-Fleet estates](0.111-design.md)

## Release-Batch Tracker

| Batch | Outcome | Direct evidence | Status |
| --- | --- | --- | --- |
| Q0 | Physical cycle-disposition primitive | Finalized capsule, exact attached credit, atomic receipt, duplicate refund, response-loss recovery, execution slack, uninstall, residual observation and deletion discard | Blocked on 0.110 closeout and explicit Q0 promotion |
| B1 | Indexed Root-local estate | Bounded counters/indexes, corruption, first excess and scan removal | Held until Q0 evidence is accepted and B1 is explicitly promoted |
| B2 | Ordinary reserve Fleet | Empty topology, exact Roots, no-effect plan and authority isolation | Blocked on B1 |
| B3 | One-asset cycle disposition | Two-Root prepare/activate, capsule binding, exact receipt, deletion fence and same-operation retry | Blocked on B2 |
| B4 | Funding and conservation | Funding fence, protocol-debit/residual bounds, internal Root allocation and terminal receipts | Blocked on B3 |
| B5 | Two-Fleet qualification | Two Fleets/two Subnets, one co-located disposition, isolation and replay | Blocked on B4 |
| B6 | Security, budgets and closeout | Wrong authority, no-preservation residue, inherited budgets and immutable audit | Blocked on B5 |

## Scope Boundary

The operation transfers controlled cycle value from one eligible source by
installing a finalized one-shot capsule into the previously empty source,
running its exact bound call, then stopping and uninstalling it before final
reconciliation and deletion. It does not transfer a Canister. Any later
destination creation receives a fresh Principal.

The cancelled stateful-adoption proposal is retained only in
[the archive](../archive/0.111-rescinded-stateful-fleet-release-adoption/status.md)
and has no active authority.

## Next Authorized Action

No implementation is authorized. Complete and accept 0.110, then explicitly
promote Q0. B1 remains held until Q0's immutable platform proof is accepted and
the maintainer explicitly promotes B1.
