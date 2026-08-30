# Canic 0.110 Implementation Status

Date: 2026-08-30

## Status

- State: accepted reorientation; implementation not promoted.
- Outcome: contract the Fleet runtime, release builder, operator path and
  validation burden, then add one bounded stateful-retirement safety boundary.
- Runtime impact: none from this planning amendment.
- Downstream steering: Toko Miner is the primary read-only real-application
  evidence source. Canic retains repository-owned fixtures and no downstream
  production dependency.
- Predecessor: 0.109 is still open. Its downstream adoption, accepted
  complexity findings, B9/B10 work, passing immutable audit and human closeout
  remain mandatory.
- Promotion: none. Only a later explicit maintainer decision after accepted
  0.109 closeout may start B1.
- Successors: accepted 0.110 closeout admits inventory for the exact 0.111
  transition; 0.112 remains separately blocked.

Design: [Fleet runtime contraction and stateful safety](0.110-design.md)

## Feedback Incorporated

- The design's [upstream feedback disposition](0.110-design.md#upstream-feedback-disposition)
  maps the retained, sequenced and deferred `CANIC-*` requirements through
  `CANIC-091`.
- `CANIC-014`: structured publication truth and a sub-250-line handoff.
- `CANIC-087`: remove redundant declaration LTO and serial compatible runtime
  links while preserving one canonical production artifact.
- `CANIC-090`/`CANIC-091`: exact prerequisite effects short-circuit unavailable
  protected observation and require full revalidation afterwards.
- Endpoint-heavy downstream size evidence: shared non-generic wrappers,
  capability attribution and at least 350 KiB useful current-profile headroom.
- Fleet application-version inventory: managed roles already expose package,
  Canic and IC canister versions through their protected Overview, while the
  Fleet list currently retains only Canic version and module hash. B4 owns one
  host-only verified aggregate that groups semantic versions while preserving
  exact module hashes as deployment authority.

`CANIC-091` remains a 0.109 blocker; recording its invariant here does not
defer its implementation.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence | Status |
| --- | --- | --- | --- |
| B1 | Immutable baseline and hard-cut scope | Capability/build/operator/test budgets and retained/deferred/removed map | Blocked on 0.109 closeout and promotion |
| B2 | Release-build contraction | Non-LTO declarations, build-context map, batched compatible roles, deterministic LTO A/B | Blocked on B1 |
| B3 | Runtime and endpoint contraction | Thin trampolines, role pruning, capability sizes and endpoint-heavy headroom | Blocked on B2 |
| B4 | Control-plane/operator/validation contraction | Ownership ratchets, Fleet Ensure vocabulary, verified per-role application/Canic/IC/module version inventory and bounded test envelope | Blocked on B3 |
| B5 | Stateful retirement safety | Opt-in receipt, exact removal binding, Draining restriction, retry and tombstone | Blocked on B4 |
| B6 | Downstream-shaped qualification and closeout | Multi-Root, two-Fleet, endpoint-heavy, public recovery and immutable audit evidence | Blocked on B5 |

## Deferred From Former 0.110

- Indexed estates and a bounded reserve Fleet move to 0.112.
- Same-Subnet cross-Fleet transfer moves to 0.112.
- Adaptive high-throughput lanes, broad automatic funding, transfer batches
  and 1,000-canister qualification are unscheduled.
- The generic runtime Observatory is an unnumbered idea.

## Next Authorized Action

Complete 0.109 and its human-owned closeout. Do not implement or measure a
0.110 candidate until the maintainer explicitly promotes B1 against the exact
accepted predecessor.
