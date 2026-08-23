# Canic 0.109 Implementation Status

Date: 2026-08-23

## Status

- State: accepted and scheduled as application-safety step 7, immediately
  after 0.108, because direct standalone application ingress blocks Toko Miner
  staging.
- Outcome: one Coordinator-owned, bounded Fleet admission policy projected and
  enforced locally by every participating managed or declared standalone
  application canister.
- Runtime impact: none from this planning cut.
- Predecessors: published 0.107 supplies bounded local whitelist primitives;
  completed and human-accepted 0.108 is required before implementation.
- Implementation approval: none. B1 requires 0.108 closeout and explicit
  maintainer promotion.
- Successors: 0.110 estates, 0.111 stateful adoption and 0.112 observatory are
  renumbered but otherwise retain their accepted dependency order.
- Surface posture: the design hard-cuts `[app.whitelist]`,
  `caller::is_whitelisted()` and independent per-canister mutation into
  protected Fleet input, one Coordinator command/status authority, local
  projections and `caller::is_fleet_admitted()`. Exact names remain a B1
  boundary decision before implementation.
- Downstream posture: Toko Miner remains read-only from Canic. Its standalone
  IcyDB App consumes the future framework-neutral protocol in downstream-owned
  work after publication.

Design: [Fleet-wide ingress admission](0.109-design.md)

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Exact authority and baseline | 0.107 whitelist inventory, Toko direct-ingress trace, selector/participant contract, bounds and hard-cut acceptance | Source/config/Candid inventory and explicit maintainer review | Blocked on 0.108 closeout and promotion |
| B2 | Protected policy compilation | Fleet-input schema, selectors, canonical digest, effective projections, plan/Registry/install authority and config hard cut | Config/hash/plan/first-excess tests | Pending |
| B3 | Coordinator policy authority | Stable policy, add/remove generation, replay, paged status and diagnostics | Model/policy/storage/replay/capacity tests | Pending |
| B4 | Managed-role projections | Root distribution, local storage/predicate, endpoint manifests, fresh activation and whitelist removal | Role builds, access/macro, restart and multi-Root PocketIC tests | Pending |
| B5 | Standalone consumer contract | Passive DTO/hash/validation package, one-time binding and generic consumer fixture | Native parity and direct-ingress PocketIC journey | Pending |
| B6 | Runtime convergence | Prepare/fence/activate/open journal, participant fences, exact retry and forward recovery | Interruption, unavailable-target, add/remove and new-Component PocketIC matrix | Pending |
| B7 | Security closeout and propagation | Docs/generated surfaces, residue cleanup, measurements and read-only Toko adoption review | Targeted repository gates and adversarial multi-Root journey | Pending |

Seven batches fit the normal minor-line guideline. They are not preassigned
patch releases.

## Blocking Application Evidence

Toko Miner's Core checks Canic's local runtime whitelist before browser login,
while its standalone IcyDB App accepts direct authenticated ingress under its
own lifecycle. An arbitrary non-anonymous Principal can therefore bypass Core
and reach any App method whose application policy admits that caller. Canister
topology and controllership do not intercept the call.

The accepted 0.109 outcome gives Core and the App projections of one Fleet
policy while preserving direct caller identity and application-owned
`Principal -> UserPrincipal -> UserId` resource authority.

## Next Authorized Action

No 0.109 implementation is authorized by this scheduling cut. Complete the
0.108 closeout, then request B1 promotion and freeze the exact authority,
selector, participant, bound and hard-cut evidence before mutation. Toko
staging remains blocked on a released admission boundary.
