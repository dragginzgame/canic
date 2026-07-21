# 0.98 Architectural Sediment Hard Cuts - Status

Last updated: 2026-07-21

## Current State

0.98 is proposed, baseline-blocked, and has no product implementation. It
follows the active 0.96 receipt line and the proposed 0.97 role-owned
dependency/CDK line. No package version, manifest, runtime, config, public API,
test package, changelog, or downstream repository change is authorized by the
proposal alone.

The line removes exactly three findings proven by the repository-wide
architectural sediment audit:

1. accepted/generated/displayed randomness configuration with no runtime
   consumer, plus the unreachable raw_rand adapter and metric variants;
2. the dead project-protocol-stub package left by the 0.65 protected
   descriptor/client hard cut; and
3. the obsolete LocalIntent external-call race path, intent_client, and
   intent_external, while retaining the current receipt-backed intent
   conformance in intent_authority.

Compatibility posture is a pre-1.0 hard cut. No aliases, deprecated wrappers,
ignored config fields, fallback parsers, placeholder packages, legacy test
targets, or compatibility modes are permitted.

## Release Boundary

- Provisional evidence anchor: released v0.96.5. This is not the 0.98
  implementation baseline.
- Open current work: the unfinished 0.96 receipt/reclamation line.
- Preceding proposed line: 0.97 role-owned runtime dependencies and CDK
  surface.
- Canonical 0.98 design: [0.98-design.md](0.98-design.md).
- Canonical evidence: the
  [architectural sediment audit](../../audits/reports/2026-07-21/architectural-sediment.md).

## Finding Index

| Finding | Severity | State | 0.98 disposition |
| --- | --- | --- | --- |
| CANIC-098-CONFIG-001 randomness is accepted but never executed | P1 | proposed | Delete schema, render, projection, adapter, metrics, docs, and tests |
| CANIC-098-PACKAGE-002 project-protocol-stub has no consumer | P2 | proposed | Delete package/member/lock row without replacement |
| CANIC-098-TEST-003 LocalIntent external race contradicts receipt contract | P2 | proposed | Delete client/external/buy path and retain focused receipt conformance |

## Slice Order

| Slice | State | Outcome |
| --- | --- | --- |
| A — dead project protocol package | pending | Workspace contains no consumerless protocol placeholder |
| B — receipt-backed test narrowing | pending | One focused current intent conformance; no LocalIntent external await |
| C — randomness contract hard cut | pending | No accepted no-op config or unreachable raw_rand runtime path |
| Closeout — contract accounting | pending | Exact product/test/config/metric/package impact recorded |

## Implementation Prerequisite

Implementation may begin only after 0.96 and 0.97 are both closed. At that
point the 0.98 design and this page must name the exact released 0.97 tag and
commit, the deletion/reachability inventory must be rerun against that
immutable tree, and Slice B must contain the exact assertion manifest from the
final released 0.96 receipt contract. Until all three actions are complete,
0.98 is not implementation-ready.

## Explicit Non-Goals

0.98 does not:

- change 0.96 receipt storage, replay, eligibility, reclamation, timers, or
  downstream conformance;
- implement any 0.97 dependency/CDK work;
- add replay receipt observation or RecoveryRequired reconciliation;
- delete or redesign icp-refill;
- delete or redesign LocalIntent;
- add a PRNG, entropy, nonce, randomness, reseed, transaction, or recovery
  framework;
- create a replacement shared project protocol crate;
- edit downstream repositories; or
- become a general cleanup line.

## Next Action

Wait for the preceding release boundaries. When they close, freeze the exact
0.97 baseline, rerun the complete consumer/contract inventory, and pin the
final 0.96 receipt assertions before implementing Slice A. Do not begin by
changing versions or by combining all deletions into one unreviewable patch.

## Completion Gate

0.98 closes only when:

- all three findings are deleted;
- current config without randomness still builds;
- explicit randomness input fails through
  ConfigInvalid(path) -> CoreConfig(Project) -> CannotParseToml with typed
  logical-path and unknown-field evidence, without raw Serde-text matching;
- the three deleted packages are absent from Cargo metadata and lock data;
- the focused receipt-backed PocketIC test preserves every exact assertion in
  the frozen final-0.96 manifest;
- legitimate LocalIntent tests remain, and maintained code, fixtures,
  examples, and docs contain no rollback based solely on an externally
  uncertain result;
- product stable IDs/schemas and generic LocalIntent/ReceiptBackedIntent
  formats are unchanged, obsolete intent_authority test-only state is
  accounted, and surviving receipt state remains upgrade-compatible;
- product Candid, backup/restore, and deployment contracts have no
  unclassified change;
- no compatibility surface is added; and
- one 0.98 closeout report records the exact contract and deletion evidence.
