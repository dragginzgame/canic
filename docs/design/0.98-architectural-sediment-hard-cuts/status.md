# 0.98 Architectural Sediment Hard Cuts - Status

Last updated: 2026-07-22

## Current State

0.98 is accepted and active against immutable `v0.97.3`, but that anchor is
not a passing 0.97 closeout. Its post-0.97 deletion/reachability inventory and
final-0.96 receipt assertion manifest remain valid and are frozen in the
[baseline report](../../audits/reports/2026-07/2026-07-22/0.98-immutable-baseline-and-inventory.md).
Slice A is complete in the open `0.98.0` batch: the dead project-protocol test
package, workspace member, and lock row are deleted without replacement. The
explicit legacy refill anti-resurrection test found during 0.97 closeout is
also removed under current repository policy. No package-version or downstream
repository change is part of the current batch.

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

- Immutable implementation baseline: released `v0.97.3` at
  `4f4397cd58b648759307b51d98033c7c21538345`.
- Source tree: `0efda05cd46e94c1c45d6a37f6a0270fa8b7bd0c`.
- Product-tree hash:
  `e540970e5aad935a2f4c5aff5ff43c790beb1958d6e33fb5f801ba6c050cc03d`.
- Cargo.lock SHA-256:
  `bcf041e99d7ead0f1d4419251f4fe5cd24d11604dbb15002330562e37dc547bd`.
- Final receipt contract: released `v0.96.8`.
- Preceding line: released 0.97 role-owned runtime dependencies and CDK
  surface at `v0.97.3`; rigorous closeout corrections are pending release.
- Canonical 0.98 design: [0.98-design.md](0.98-design.md).
- Canonical evidence: the
  [architectural sediment audit](../../audits/reports/2026-07-21/architectural-sediment.md).

## Finding Index

| Finding | Severity | State | 0.98 disposition |
| --- | --- | --- | --- |
| CANIC-098-CONFIG-001 randomness is accepted but never executed | P1 | proposed | Delete schema, render, projection, adapter, metrics, docs, and tests |
| CANIC-098-PACKAGE-002 project-protocol-stub has no consumer | P2 | fixed | Package/member/lock row deleted without replacement |
| CANIC-098-TEST-003 LocalIntent external race contradicts receipt contract | P2 | proposed | Delete client/external/buy path and retain focused receipt conformance |

## Slice Order

| Slice | State | Outcome |
| --- | --- | --- |
| A — dead project protocol package | completed | Workspace contains no consumerless protocol placeholder |
| B — receipt-backed test narrowing | pending | One focused current intent conformance; no LocalIntent external await |
| C — randomness contract hard cut | pending | No accepted no-op config or unreachable raw_rand runtime path |
| Closeout — contract accounting | pending | Exact product/test/config/metric/package impact recorded |

## Implementation Prerequisite

The deletion inventory prerequisite is satisfied: the exact released 0.97
identity is recorded above, the complete inventory was rerun, and the final
released 0.96 receipt assertions are frozen. Release closeout is blocked until
the bounded 0.97 corrections have a validated immutable anchor.

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

Review and release the bounded 0.97 closeout corrections before treating the
0.98 predecessor as passing. Keep the open `0.98.0` Slice A package deletion
distinct, then begin the separately bounded receipt-backed intent-fixture
Slice B. Do not change versions outside the maintainer release flow or combine
the independent randomness deletion into this batch.

## Slice A Validation

- Locked Cargo metadata resolves 39 workspace packages and contains no
  `project-protocol-stub` package or member.
- `project_hub_stub`, `project_instance_stub`, and `delegation_root_stub`
  compile together with their role values owned beside the actual consumer.
- All seven workspace-manifest governance tests pass.
- All 51 subnet-schema tests pass after removing the legacy-path-only test.
- Strict all-target test Clippy for `canic-core` passes with warnings denied.
- The retained package deletion changes no product Rust, Candid, stable state,
  config, CLI, JSON, metric, or artifact-format contract.

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
