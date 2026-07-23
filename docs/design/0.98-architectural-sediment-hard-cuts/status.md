# 0.98 Architectural Sediment Hard Cuts - Status

Last updated: 2026-07-22

## Current State

0.98 is closed at immutable `v0.98.2`. The original implementation inventory remains
anchored to immutable `v0.97.3`, which is not retroactively treated as a
passing 0.97 closeout. Its post-0.97 deletion/reachability inventory and
final-0.96 receipt assertion manifest remain valid and are frozen in the
[baseline report](../../audits/reports/2026-07/2026-07-22/0.98-immutable-baseline-and-inventory.md).
Published `v0.98.0` supplies the corrected immutable predecessor and completes
Slice A. Published `v0.98.1` completes Slice B: the obsolete external race
fixture and its two auxiliary packages are removed, while one focused
receipt-backed PocketIC authority retains every frozen final-0.96 assertion.
Slice C is published in `v0.98.2`. The false
randomness schema, render, projection, adapter, metrics, tests, and active docs
are removed without a replacement or compatibility path. Before publication,
the maintainer approved adding the bounded consolidation amendment to the same
release. Its 42-candidate repository audit fixes one P1 and 11 P2 findings,
proves 30 notes, and leaves no deferred or unresolved item. No package-version
change beyond the maintainer-owned release synchronization, and no downstream
repository change, is part of the batch.

The original line removes three findings proven by the repository-wide
architectural sediment audit:

1. accepted/generated/displayed randomness configuration with no runtime
   consumer, plus the unreachable raw_rand adapter and metric variants;
2. the dead project-protocol-stub package left by the 0.65 protected
   descriptor/client hard cut; and
3. the obsolete LocalIntent external-call race path, intent_client, and
   intent_external, while retaining the current receipt-backed intent
   conformance in intent_authority.

The accepted [consolidation design amendment](consolidation-design-amendment.md)
extends only the released 0.98.2 patch. Its
[implementation tracker](consolidation-implementation-tracker.md),
[disposition ledger](consolidation-ledger.md), and
[validation evidence](consolidation-validation-evidence.md) support the one
canonical [0.98 closeout audit](0.98-closeout-audit.md).

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
- Corrected predecessor and released Slice A: `v0.98.0` at
  `f6aef15ffd03d0b6cb573330ac0cc7a348ee3caf`, source tree
  `fd1ef47e7b3a6e4cd3ad7f9e88262a2a1a2335d4`, product-tree hash
  `902883e318cf6fdb88cabb9f4195cfbdda13d18c80094ad622f45d0eb2a70524`,
  and Cargo.lock SHA-256
  `f7d26cf21ea029a4a76fbb7cbd2ba402e9b46d221c78a1af56a81968bf7d9550`.
- Released Slice B: `v0.98.1` at
  `e0dcd0cbb8f550e4c0366d9e1007ca32dceb2aa7`, source tree
  `ae154b0deb862702d48fed4dd235caf76089f7a2`, product-tree hash
  `b190d1f163cf8f2099290fb429a7e9f84c693cd15ae1b446e72165214622a042`,
  and Cargo.lock SHA-256
  `801ad42f9b2a733e925d3c4b0b66cae1922b60b3b7b2cc0166a9a52cfd2092e2`.
- Released Slice C, consolidation amendment, and immutable closeout:
  `v0.98.2` at commit
  `73973fc24c407b1732de1a142d4990b5cb6becf6`, source tree
  `a53cc20e4533f7c7277e2fda3c594ecba8eb99ac`, product-tree hash
  `961b30a138f55e4644b34372b51bd595abbca933234ed39b07b598113d90c0d3`,
  and Cargo.lock SHA-256
  `dc6355881a2dc3856cb8a991b03b5b368e73dc5398a9da599fcb68be63721458`.
- Final receipt contract: released `v0.96.8`.
- Preceding line: released 0.97 role-owned runtime dependencies and CDK
  surface at `v0.97.3`; its rigorous closeout corrections are published in
  `v0.98.0` without changing the frozen 0.98 deletion inventory.
- Canonical 0.98 design: [0.98-design.md](0.98-design.md).
- Canonical closeout evidence:
  [0.98-closeout-audit.md](0.98-closeout-audit.md).
- Canonical evidence: the
  [architectural sediment audit](../../audits/reports/2026-07-21/architectural-sediment.md).

## Finding Index

| Finding | Severity | State | 0.98 disposition |
| --- | --- | --- | --- |
| CANIC-098-CONFIG-001 randomness is accepted but never executed | P1 | fixed | Schema, render, projection, adapter, metrics, docs, and tests deleted without replacement |
| CANIC-098-PACKAGE-002 project-protocol-stub has no consumer | P2 | fixed | Package/member/lock row deleted without replacement |
| CANIC-098-TEST-003 LocalIntent external race contradicts receipt contract | P2 | fixed | Client/external/buy path deleted; focused receipt conformance retained |
| CANIC-098-CLOSE-* consolidation ledger | 1 P1, 11 P2, 30 notes | resolved | All findings fixed and every candidate disposition proved |

## Slice Order

| Slice | State | Outcome |
| --- | --- | --- |
| A — dead project protocol package | completed | Workspace contains no consumerless protocol placeholder |
| B — receipt-backed test narrowing | completed | One focused current intent conformance; no LocalIntent external await |
| C — randomness contract hard cut | completed | No accepted no-op config or unreachable raw_rand runtime path |
| D — build and config authority | completed | Syntax-aware role build contract, one parsed config, executable current guide |
| E — runtime and state authority | completed | Active CycleTracker metadata; dead duration/snapshot/capability layers removed |
| F — authentication consolidation | completed | Fixed seed/domain families remain without singleton kind taxonomies |
| G — host/operator and dependency surface | completed | Orphan helpers removed, visibility narrowed, intentional fixture edges recorded |
| H — repository closure | completed | All 42 candidates resolved and cumulative validation recorded |
| Closeout — contract accounting | completed | Complete impact and immutable `v0.98.2` identity recorded |

## Implementation Prerequisite

The deletion inventory prerequisite is satisfied: the exact released 0.97
identity is recorded above, the complete inventory was rerun, and the final
released 0.96 receipt assertions are frozen. The bounded 0.97 corrections now
have the validated immutable `v0.98.0` anchor.

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

The immutable `v0.98.2` closeout remains closed. The maintainer subsequently
accepted bounded post-closeout hard-cut tails through released `v0.98.15`.
Open `0.98.16` removes the consumerless generic ICRC-2 allowance and
`transfer_from` stack, its internal bindings and metadata, and its unreachable
platform metric surface. The separate root-only manual ICP-to-cycles workflow
remains. This does not reopen the immutable 42-candidate accounting, the
excluded ICP-refill state machine, or the app-registry work deferred to 0.100.
After targeted validation, exercise the resulting release in real
deployments.

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

## Slice B Validation

- Locked Cargo metadata resolves 37 workspace packages and contains neither
  `intent_client` nor `intent_external`.
- The one-canister receipt-backed PocketIC target passes every frozen
  admission, replay, rejection, capacity, settlement, terminal, upgrade, and
  reclamation assertion.
- All three lifecycle-boundary PocketIC tests pass with the narrowed unit-init
  `intent_authority` fixture.
- All seven focused LocalIntent workflow tests and both receipt-reclamation
  inventory guards pass. The retained `runtime_probe` LocalIntent consumer
  checks, strict Clippy passes for all changed Rust targets, and the layering
  guard passes. The generic product LocalIntent and ReceiptBackedIntent
  authorities remain unchanged.
- Product Candid and stable formats are unchanged; only the unpublished test
  canister's obsolete init argument and `buy` endpoint are removed.

## Slice C Validation

- All 15 checked-in active `canic.toml` files parse and validate after the
  field/type/default deletion. The generated standalone minimal profile and
  configured root audit profile both compile without a randomness field.
- Direct core parsing and the complete host config projection reject an
  explicit retired table through
  `ConfigInvalid -> CoreConfig(Project) -> CannotParseToml`, with typed
  `logical_path` and `unknown_field` values. The tests do not match rendered
  TOML/Serde wording.
- Focused core config/bootstrap, host projection, Canic reference, canonical
  configuration-guide, and protocol-surface tests pass. The protocol suite
  includes the checked-in Wasm-store Candid contract.
- Strict all-target Clippy passes for `canic-core`, `canic-host`, and `canic`
  with warnings denied. Formatting, layering, changelog governance, and diff
  hygiene pass.
- Active product/config/operator source contains no randomness schema,
  generated field, projection, adapter, error, or metric path. The remaining
  active mentions are two strict-schema rejection fixtures and accurate auth
  documentation stating that auth does not call `raw_rand`.
- Product Candid, stable memory, backup/restore, deployment, CLI command/JSON,
  and package-version surfaces are unchanged. The host parser gains one
  host-only dependency for structured path evidence.

## Consolidation Amendment Validation

- All 42 candidates have a final disposition: zero P0, one fixed P1, 11 fixed
  P2 findings, and 30 proved notes.
- Focused role-contract, state-manifest, config-guide, state-contract,
  capability, auth, host ICP, CLI state, protocol, and reference checks pass.
- The recorded cumulative unit/PocketIC selection, strict workspace Clippy,
  formatting, layering, feature/dependency, audit-catalog, release-policy,
  changelog-governance, and diff-hygiene gates pass.
- No stable encoding or memory-ID, Candid, maintained JSON, CLI, workspace
  package-set, or package-version change is introduced. Public Rust and TOML
  hard cuts are explicitly recorded in the closeout contract table.

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
- one 0.98 closeout report records the exact contract, deletion, consolidation,
  and validation evidence.
