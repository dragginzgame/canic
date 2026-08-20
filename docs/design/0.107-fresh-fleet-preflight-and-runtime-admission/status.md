# Canic 0.107 Implementation Status

Date: 2026-08-20

## Status

- State: accepted and scheduled as application-safety/estate step 5.
- Runtime impact: B2 changes only the host CLI planning boundary; no Canister
  runtime, stable state, Candid or external effect changes.
- Predecessor: accepted repository-local 0.106 B1 baseline. The separately
  authorized 0.106 B2 external evidence does not gate this line.
- Successors: 0.108 Coordinator-backed root funding retains its passing B1
  evidence but begins production work only after this line; 0.109 estates and
  later scheduled lines move one minor number without semantic change.
- Repository boundary: Toko remains read-only and supplies requirements plus
  final acceptance evidence only.
- Estimate: seven release batches and approximately 10-15 engineering days,
  excluding upstream release latency and separately approved live-IC work.
- Implementation approval: the maintainer accepted B1 on 2026-08-20. B2 is
  complete; the sequenced B3-B7 work is authorized within this design.

Design: [Fresh-Fleet preflight and runtime admission](0.107-design.md)

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Exact baseline and contract | Toko traceability, current planner/install/whitelist/upstream inventories, bounds and exact surface contract | source/fixture inventory and explicit acceptance | Accepted 2026-08-20 |
| B2 | Target-correct planning | direct plan-leaf environment forwarding and mismatch rejection | CLI parse/forwarding/help tests | Complete |
| B3 | Fleet-input-complete pure preflight | shared compiler, no-effect ordering and fresh-Fleet blockers | host plan/install ordering and fixture tests | Pending |
| B4 | Complete evidence and digest binding | placement, counts, funding, balance, output and install-receipt parity | parity, insufficient-funds and receipt tests | Pending |
| B5 | Structured catalog inconsistency | Registry version/provenance/cache/subject/retry/effect propagation and upstream update if needed | typed collector/host/CLI tests | Pending |
| B6 | Durable runtime whitelist | seed/restore, bounds, add/remove/revision/digest/replay and config hard cut | core/facade/restoration tests | Pending |
| B7 | Operator proof and closeout | command/status UX, adversarial/recovery journeys, generic fixture, downstream read-only rerun and residue cleanup | targeted package and bounded PocketIC checks | Pending |

These are coherent outcome batches, not preassigned patch releases.

## B1 Result

The repository-local B1 capture is complete under
[`docs/audits/working/0.107-fresh-fleet-preflight-and-runtime-admission/`](../../audits/working/0.107-fresh-fleet-preflight-and-runtime-admission/README.md).
The maintainer accepted it on 2026-08-20 as production and stable-state
implementation authority for the sequenced B2-B7 batches.

- At B1 capture, the planner, installer, whitelist access path, stable-
  allocation registry and managed-role macros were byte-for-byte `v0.105.0`
  source. The retained hashes remain that accepted predecessor baseline.
- The maintained direct plan leaf now has one frozen grammar shared with
  install: required App, Fleet and Fleet input; common profile and optional
  finalized release-build identity; one forwarded global environment; and an
  optional install-only expected plan digest.
- One pure host compiler and domain-separated schema-1 plan digest cover all
  decision-bearing target, Fleet-input, catalog, placement, funding, balance,
  release-source and no-effect evidence before durable build preparation.
- Runtime admission is frozen for managed non-root roles beneath the existing
  role methods. Memory ID 61 owns schema 1, at most 256 sorted principals, a
  128-entry page and one retained operation under Root-or-controller
  administration. Test-only maximum encodings measure 8,417 stable bytes,
  4,072 status Candid bytes and 101 mutation Candid bytes.
- Read-only Toko commit
  `bf14a5d3d89be4335d3da2601e8a60128fde04df` contains 175 compiled
  whitelist principals, leaving 81 entries beneath the hard maximum. It has no
  current Canic integration or CANIC-011/012/013 identifiers, so B7 must use a
  newer read-only acceptance source or record that exact external blocker.
- Exact `ic-query 0.40.1` loses a Registry version already known after its
  initial fetch and does not retain failed cache-stage context or unknown
  retryability. B1 freezes the smallest additive typed upstream result; B5 may
  not claim full provenance through string parsing or a fork.

No production source, stable implementation, Candid or CLI was changed by B1.
No 0.106 B2 effect or sibling-repository mutation occurred.

## B2 Result

- The top-level environment is forwarded to the direct `deploy plan` leaf.
- A hidden/internal environment that disagrees with the selected top-level
  environment rejects before dispatch instead of silently winning.
- Planning resolves the selected ICP environment to one canonical network.
  Missing authority and contradictory environment profiles are blockers, and
  a contradiction cannot fall through to Fleet-catalog lookup.
- Direct plan help now identifies the top-level environment placement. The
  leaf still accepts no ICP executable because planning performs no ICP
  command or IC effect.
- Focused plan/forwarding tests and warning-denied `canic-cli` Clippy pass. No
  Canister runtime, stable state, Candid, external Canister or sibling
  repository changed.

## Feedback Traceability

| Feedback | Owning batches | Closeout proof |
| --- | --- | --- |
| CANIC-011 runtime whitelist evolution | B1, B6, B7 | authorized bounded mutation plus denial, retry and restoration journeys |
| CANIC-012 target/Fleet-input-complete plan | B1-B4, B7 | plan/install input and digest parity with pre-effect failures |
| CANIC-013 catalog inconsistency diagnostics | B1, B5, B7 | structured provenance and truthful retry/effect rendering |

## Next Authorized Action

Begin B3's Fleet-input-complete pure preflight from the accepted B1 contract
and completed target-correct B2 boundary. Keep 0.106 B2's external effects
held pending their separate exact authorization.
