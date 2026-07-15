# Current Status

Last updated: 2026-07-15

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the source, design, audit, or changelog files needed for the current task.

Historical detail is archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md); and
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md).

## Current Release

- The workspace package version is `0.92.3`.
- `v0.92.3` is published at
  `b7f9aad9265e43def97362457148541f8e787d35`.
- The accepted line design is
  [0.92 holistic audit and audit-system validation](../design/0.92-holistic-audit-and-audit-system-validation/0.92-design.md).
- Detailed release notes are in the
  [0.92 changelog](../changelog/0.92.md).

## Current Decision

0.92 treats Canic as feature complete for this line, but not as 1.0-ready.
The audit machinery has been inventoried, corrected, and frozen. Phase C has
attempted all 22 retained definitions read-only against the published
`v0.92.0` product snapshot. Corrected instruction-footprint v2 is
valid/partial, and corrected Wasm-footprint v2 completes the retained-method
ledger at 22 valid and zero invalid active results. Evidence-only PocketIC
coverage completes the two formerly partial mandatory traces. The aggregate is
now a valid `fail` with six pass and four fail results, zero partial, and zero
blocked. The Phase C product baseline gate is complete; product fixes still
require explicit finding review and bounded slice acceptance. Phase D D1 is
released in `v0.92.1`; D2/D3 are released in `v0.92.2`; D4 is released in
`v0.92.3`. D5 is implemented and validated in the current worktree: blob
billing has one workflow-owned Cashier, reserve, recovery, sync, and readiness
path over pure policy and single-step ops. This fixes
`CANIC-092-LAYERING-001` without changing protocol, prices, public shapes, or
stable state. The live layering guard remains at 18 separately owned
ops-to-policy violations under `CANIC-092-LAYERING-005`.

Pre-1.0 removals remain hard cuts. Do not add aliases, compatibility wrappers,
duplicate command paths, deprecated APIs, anti-resurrection tests, or fallback
behavior unless the maintainer explicitly requests it. Named build
environments resolve through `icp.yaml`; only `local` and `ic` are
implicit, and no staging/mainnet aliases exist.

Toko mint remains downstream-owned. Canic provides generic primitives only;
automated work must not edit the Toko repository or move mint-specific
requests, receipts, evidence, retry, cancellation, or tests into Canic.

## 0.92 Audit-System Outcome

- Phase A's
  [inventory report](../audits/reports/2026-07/2026-07-14/0.92-audit-system-inventory.md)
  found six confirmed P1 audit-system defects.
- Phase B's
  [hardening report](../audits/reports/2026-07/2026-07-14/0.92-audit-system-hardening.md)
  prepared and validated all six corrections.
- The
  [method-freeze report](../audits/reports/2026-07/2026-07-14/0.92-method-freeze.md)
  closes those findings at `v0.92.0` and admits the Phase C baseline.
- One canonical [method catalog](../audits/METHODS.md) owns 22 active
  definitions: 14 system, 7 authentication, and 1 manual-only module-surface
  method.
- The frozen method manifest is
  `fa92c4102efe74391c51f1f829aec7ac9c0b64941da73ee6dad1ebf2b292df07`.
- The frozen source tree is
  `fd31bb8289365a38f2bea7f8ebd6973908ee959f`.
- The frozen product-tree hash is
  `c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e`.
- The compatibility baseline remains `v0.91.6`, product-tree hash
  `8fce43e41ce430d9b505e19f8d596ed440b291d4c6ecb19c4a1cfdf71656a9b6`.
- The committed delta is fully classified as audit-system, operator/CI
  contract, requested documentation, and human-owned release-version changes;
  no runtime/public/serialized/stable/dependency behavior changed.
- At least three months of real-world use remains a separate prerequisite for
  any future 1.0 discussion.

## 0.92 Phase C Baseline and Phase D

- The frozen Phase C baseline remains immutable at
  `91736337fc1cfeb891f17d7d62affb5e671348e2`.
- Phase D changes only accepted finding-backed slices and compares them to the
  immediate parent and frozen baseline.
- D1 is released in `v0.92.1`, D2/D3 in `v0.92.2`, and D4 in `v0.92.3`. D5 is
  implemented with focused validation passing. D6 through D10 remain ordered
  candidates rather than blanket authorization.
- Missing evidence remains partial/blocked, never pass, and historical Phase C
  results are not rewritten by later fixes.

First primary results:

- [dependency hygiene v1](../audits/reports/2026-07/2026-07-14/0.92-dependency-hygiene-v1.md)
  remains invalid history. Corrected
  [dependency hygiene v2](../audits/reports/2026-07/2026-07-14/0.92-dependency-hygiene-v2.md)
  is a valid pass at risk 3/10: all 484 external packages identify license
  metadata, the cached advisory scan finds zero known vulnerabilities, and
  four reachable unmaintained transitive packages remain watchpoints. This is
  metadata hygiene, not legal review.
- [product baseline identity correction](../audits/reports/2026-07/2026-07-14/0.92-product-baseline-identity-correction.md)
  fixes `CANIC-092-AUDIT-017`: the previously cited `cfc49c36...` belonged to
  the Phase B implementation commit; exact published `v0.92.0` product identity
  is `c2b932cf...`. Full source commit identities and product evidence remain
  valid.
- [CI and release integrity](../audits/reports/2026-07/2026-07-14/0.92-release-integrity.md)
  is a valid `fail`: permissions/triggers and `actionlint` pass, but external
  Actions use mutable tags, executable tool downloads/installations lack
  immutable verified identities, the required dedicated secret scanner is
  absent, and no canonical supported host/target matrix exists.
- Findings `CANIC-092-AUDIT-007`, `CANIC-092-DEPENDENCY-001`, and
  `CANIC-092-RELEASE-001` through `-004` are indexed in the 0.92 tracker.
- [layer boundary](../audits/reports/2026-07/2026-07-14/0.92-layer-boundary.md)
  remains invalid v1 history. Corrected
  [layer boundary v2](../audits/reports/2026-07/2026-07-14/0.92-layer-boundary-v2.md)
  uses fingerprinted direct/grouped import fixtures and an executable
  ops-to-policy rule. The guard lists 25 production violations and the valid
  result fails at risk 7/10, fixing `CANIC-092-AUDIT-012`; API/DTO/product
  authority findings remain open.
- [build integrity v1](../audits/reports/2026-07/2026-07-14/0.92-build-integrity-v1.md)
  remains invalid history. Corrected
  [build integrity v2](../audits/reports/2026-07/2026-07-14/0.92-build-integrity-v2.md)
  excludes only observation timestamps and their derived digest from semantic
  provenance comparison. Two isolated lanes reproduce ordinary app and
  bootstrap-store raw/gzip bytes and app semantic provenance; final root
  Wasm/gzip bytes and semantic provenance still differ because absolute build
  paths enter generated runtime records and lifecycle logs. The valid result
  fails, fixes `CANIC-092-AUDIT-008`, and leaves `CANIC-092-BUILD-001` and
  `CANIC-092-BUILD-002` open.
- [authentication invariants](../audits/reports/2026-07/2026-07-14/0.92-auth-invariants.md)
  found no accepting bypass: invalid trust, audience, subject, scope, replay,
  and attestation inputs reject in focused unit/PocketIC evidence. The original
  audience/replay v1 attempts remain invalid history. Corrected
  [audience/replay v2](../audits/reports/2026-07/2026-07-14/0.92-auth-invariants-v2.md)
  methods use current exact filters through a zero-test-refusing runner and
  validly pass at risk 3/10, fixing `CANIC-092-AUDIT-009`. D2 fixes
  `CANIC-092-ERROR-001` by preserving typed proof/provisioning causes;
  `CANIC-092-LAYERING-004` remains for accidental public install DTO surface.
  The generated/session integration gap is fixed by the July 15 evidence slice.
- [mandatory trace admission](../audits/reports/2026-07/2026-07-14/0.92-mandatory-trace-admission.md)
  is `blocked` and `invalid`: the accepted design names ten mandatory trace
  IDs and requires a trace method ID/version/fingerprint, but no trace method
  was cataloged, fingerprinted, or frozen at that attempt. It remains invalid
  history. The superseding
  [mandatory traces v1](../audits/reports/2026-07/2026-07-14/0.92-mandatory-traces-v1.md)
  catalogs/fingerprints the protocol and executes all ten IDs, fixing
  `CANIC-092-AUDIT-010`. The later
  [evidence completion](../audits/reports/2026-07/2026-07-15/0.92-mandatory-trace-evidence-completion.md)
  closes auth/control execution gaps. Six traces pass and deploy/auth/control/
  blob fail on existing product findings; the valid aggregate `fail` completes
  the gate with no partial or blocked trace.
- [control-plane publication](../audits/reports/2026-07/2026-07-14/0.92-control-plane-publication.md)
  supporting evidence passes controller/root authorization, exact conflict
  refusal, and completed post-upgrade binding reconciliation. The later
  [D1 publication-safety slice](../audits/reports/2026-07/2026-07-15/0.92-d1-publication-safety.md)
  fixes `CANIC-092-COST-001` and `CANIC-092-ERROR-002` with one workflow-owned
  quota/cycle permit and typed publication causes. Quota, conflict, capacity,
  authorization, and interrupted-recovery PocketIC cases pass; store-GC
  behavior and public Candid shapes are unchanged.
- [D2 auth typed-cause preservation](../audits/reports/2026-07/2026-07-15/0.92-d2-auth-typed-causes.md)
  fixes `CANIC-092-ERROR-001`. All seven auth methods and the current auth
  trace pass; wrong-issuer, expired, and corrupted proofs reject without
  changing the installed active proof. Candid and stable state are unchanged.
- [D3 canonical layer contract](../audits/reports/2026-07/2026-07-15/0.92-d3-canonical-layer-contract.md)
  fixes `CANIC-092-LAYERING-003` and `CANIC-092-DOCS-001`. `AGENTS.md` is the
  sole active authority; public core docs, module headers, and hygiene guidance
  agree on the strict direction and model/storage ownership. No runtime,
  public, serialized, or stable behavior changes.
- [D4 root-issuer admission ownership](../audits/reports/2026-07/2026-07-15/0.92-d4-root-issuer-admission-ownership.md)
  fixes `CANIC-092-TEST-001` and removes seven root-issuer/model violations
  from `CANIC-092-LAYERING-005`. Workflow invokes pure policy before ops
  mutation; model owns state-shaped values. Public and stable shapes are
  unchanged, and the canonical layering finding remains open.
- [D5 blob-billing workflow ownership](../audits/reports/2026-07/2026-07-15/0.92-d5-blob-billing-workflow-ownership.md)
  fixes `CANIC-092-LAYERING-001`. API now delegates Cashier sequencing,
  reserve/recovery, gateway sync, and readiness to one workflow over pure
  policy and single-step ops. Public DTOs, Candid, billing prices/protocol, and
  stable records are unchanged; the current blob trace passes.
- [security boundary ordering](../audits/reports/2026-07/2026-07-14/0.92-security-boundary-ordering.md)
  is a valid pass with watchpoints: generated endpoint, trust proof,
  subject/scope, capability, replay, and recovery-required paths retain their
  owning gate before handler execution or state mutation. It does not clear
  the owner-specific auth and publication findings.
- [bootstrap lifecycle symmetry](../audits/reports/2026-07/2026-07-14/0.92-bootstrap-lifecycle-symmetry.md)
  is a valid pass with watchpoints: root/non-root init and upgrade restore
  synchronously, failure traps before continuation, and bootstrap/user work is
  scheduled through zero-delay timers. Structural guards and three PocketIC
  lifecycle cases pass.
- [capability surface v1](../audits/reports/2026-07/2026-07-14/0.92-capability-surface-v1.md)
  remains invalid history because its broad workspace Clippy requirement
  conflicts with canonical targeted-test authority. Corrected
  [capability surface v2](../audits/reports/2026-07/2026-07-14/0.92-capability-surface-v2.md)
  uses its owning package test and targeted Clippy contract. Six retained DIDs
  rebuild, 19 protocol tests and Clippy pass, and the valid result passes at
  risk 4/10, fixing `CANIC-092-AUDIT-011`.
- [publish surface v1](../audits/reports/2026-07/2026-07-14/0.92-publish-surface.md)
  is a valid pass and first frozen-method baseline at risk 4/10. All eight
  public packages verify from isolated offline archives. D3 fixes
  `CANIC-092-DOCS-001`; `CANIC-092-PUBLISH-001` and
  `CANIC-092-RESIDUE-001` still record incomplete package feature guidance and
  forbidden old-command anti-resurrection checks in the active CLI proof path.
- [module structure v1](../audits/reports/2026-07/2026-07-14/0.92-module-structure-v1.md)
  is a valid fail and first frozen-method baseline at risk 7/10. It confirms 25
  production ops-to-policy imports, direct policy decisions in ops, and
  policy-owned values used by stable mappers (`CANIC-092-LAYERING-005`). It
  finds no cycle, public record leak, test/fleet seam breach, or module-layout
  escape. Warning-as-error core rustdoc also confirms `CANIC-092-DOCS-002`.
- [DRY consolidation v1](../audits/reports/2026-07/2026-07-14/0.92-dry-consolidation-v1.md)
  is a valid fail and first frozen-method baseline at risk 6/10. Operator,
  evidence, backup, and release-proof responsibilities retain clear owners,
  but root-issuer policy admission duplicates the existing ops/policy
  authority defect. `CANIC-092-TEST-001` records the missing direct rejection
  and unchanged-state proof.
- [complexity accretion v1](../audits/reports/2026-07/2026-07-14/0.92-complexity-accretion-v1.md)
  remains invalid history. Corrected
  [complexity accretion v2](../audits/reports/2026-07/2026-07-14/0.92-complexity-accretion-v2.md)
  maps all 546 files, reproduces its normalized mechanical result, retains
  exact manual evidence, and applies one score. The valid first v2 baseline
  fails at risk 8/10, fixes `CANIC-092-AUDIT-013`, and retains P2
  `CANIC-092-COMPLEXITY-001`; 178 focused test selections pass and no auth
  correctness failure is inferred from the pressure measurements.
- [change friction v1](../audits/reports/2026-07/2026-07-14/0.92-change-friction-v1.md)
  remains invalid history. Corrected
  [change friction v2](../audits/reports/2026-07/2026-07-14/0.92-change-friction-v2.md)
  maps all 546 current files, freezes five exact feature slices, reproduces
  its normalized output twice, and applies one score. The valid first v2
  baseline fails at risk 8/10, fixes `CANIC-092-AUDIT-014`, and creates no
  separate product finding; its pressure remains deduplicated into layering
  and complexity. Seventy-four focused tests pass.
- [instruction footprint v1](../audits/reports/2026-07/2026-07-14/instruction-footprint.md)
  remains blocked/invalid history before producing any perf row. Pinned PocketIC 14.0.0
  starts and 11 scenario tuples are retained, but root-probe direct Cargo Wasm
  compilation is rejected by the authoritative `canic build` boundary. The
  runner composite is root-dependent, the exact scan misses 57 namespaced
  product checkpoints, and four required flow classes are absent
  (`CANIC-092-AUDIT-015`). Corrected
  [instruction footprint v2](../audits/reports/2026-07/2026-07-15/instruction-footprint-v2.md)
  uses only authoritative root-harness artifacts and a fixed 12-scenario
  update/install roster. All rows execute, 21 checkpoint deltas and 57 static
  checkpoints are retained, and the root-independent evidence hashes verify.
  The valid first baseline is `partial` at risk 6/10, fixes
  `CANIC-092-AUDIT-015`, and records P2 `CANIC-092-PERF-001` for missing
  root-proof and delegated-token internal checkpoints.
- [Wasm footprint v1](../audits/reports/2026-07/2026-07-14/wasm-footprint.md)
  is blocked/invalid before producing any artifact metric. The required clean
  linked-worktree run passes its Cargo/ICP/`ic-wasm`/`twiggy` prerequisites,
  then the first `app` direct Cargo Wasm build is rejected by the authoritative
  `canic build` hard cut. Its executable composite is root-dependent
  (`CANIC-092-AUDIT-016`). It remains invalid history. Corrected
  [Wasm footprint v2](../audits/reports/2026-07/2026-07-15/wasm-footprint-v2.md)
  removes the direct-Cargo/pre-shrink flow and builds all six release/debug
  role pairs through the authoritative host builder. Gzip, `ic-wasm`,
  `twiggy`, source-mutation, and evidence-hash checks pass. The valid first v2
  baseline passes at risk 4/10, fixes `CANIC-092-AUDIT-016`, and creates no
  product finding.
- [canic-core module surface hardening](../audits/reports/2026-07/2026-07-14/canic-core-module-surface-hardening.md)
  is a valid first frozen-method failure at risk 4/10. It confirms
  `CANIC-092-LAYERING-004` and adds P2 `CANIC-092-SURFACE-001` for the
  unnecessary hidden-but-public core error path. Generated, replay, state,
  sibling-support, and test-only surfaces otherwise retain current owners.
- [Phase C baseline review](../audits/reports/2026-07/2026-07-14/0.92-phase-c-baseline-review.md)
  remains the original blocked synthesis. The live ledger now has 22 valid
  and zero invalid active results after the instruction and Wasm corrections.
  The final mandatory trace result is valid and complete at aggregate `fail`.
  D1 fixes two non-waivable publication P1 findings, D2 fixes the auth cause
  P1, and D3 fixes one P1 authority conflict plus one P2 documentation drift.
  Sixteen findings remain unresolved (5 P1, 10 P2, one P3). Current control,
  auth, and blob trace reruns pass; the frozen Phase C aggregate remains
  historical.

## Focused Validation

- Clean release/tag/origin identity, method-path equality, method fingerprints,
  product-tree hash, and path classification pass.
- Audit catalog, affected Bash syntax, operator guards, `actionlint`, focused
  instruction identity/baseline tests, changelog governance, formatting, and
  diff hygiene pass.
- Locked/offline Cargo metadata/tree and the dependency-v2 declaration rule
  pass. `cargo-audit 0.22.2` found zero vulnerabilities using advisory DB
  commit `9f3e1380...` from 2026-07-13.
- The repository credential-pattern scan found no match; this does not replace
  the blocked dedicated secret scan.
- Layering v2 detector fixtures pass. Its immutable baseline validly fails on
  25 production ops-to-policy dependencies; D4's affected-scope rerun reduces
  the live set to 18. Policy purity, passive DTO, and root-issuer ownership
  checks pass.
- Build-integrity v2 executes two isolated lanes. Ordinary app and bootstrap
  artifacts reproduce exactly, semantic app provenance matches, and root
  artifacts validly retain the absolute-path reproducibility failure.
  Wasm measurements and broad product suites have not yet run under Phase C.
- Focused authentication tests pass across macro/access, proof-chain, audience,
  scope, replay, role-attestation, chain-key batch, root facade, and capability
  rejection paths. Audience/replay v2 selections are all nonempty and passing;
  their runner rejects a successful Cargo zero-test selection with exit 3.
- The exact delegated-session PocketIC case passes pre-bootstrap rejection,
  successful bootstrap and guarded-call parity, idempotent replay,
  conflicting-wallet rejection, and unchanged authority after rejection.
- D2 focused validation passes 263 all-feature auth library tests, every
  nonempty audience/replay v2 selection, trust/capability selections, strict
  targeted Clippy, and PocketIC facade/session/renewal/attestation/capability
  cases. Wrong-issuer, expired, and corrupt proof installs return typed codes
  without active-state mutation.
- D3 active-contract scans, changed-file Rust formatting, strict `canic-core`
  Clippy, and package verification pass. Layering self-tests pass; the full
  guard retains the same 25 known product-code violations. Warning-as-error
  core rustdoc still fails only on the separately indexed D10 `InternalError`
  link.
- D4 validation passes 881 all-feature `canic-core` library tests, four pure
  policy/passive DTO guards, 19 public protocol tests, strict targeted Clippy,
  and two root proof/renewal PocketIC regressions. Direct workflow tests prove
  positive policy/template admission, every request rejection boundary,
  unchanged state/epoch, and skipped timer start.
- D5 validation passes 878 all-feature `canic-core` library tests, 50 focused
  blob-billing tests, four policy/DTO guards, 19 protocol tests, strict
  all-feature/all-target core Clippy, and four PocketIC billing selections.
  Reserve refusal performs no partial top-up, transient Cashier failure
  releases the guard for retry, status boundaries remain distinct, and billing
  configuration survives upgrade.
- Focused D1 publication validation passes 18 all-feature publication tests,
  30 replay/capability policy tests, four core and one publication-owned
  cost-permit structural checks, strict targeted Clippy, and five root/store
  PocketIC filters. Ten publications are admitted in one quota window; the
  eleventh rejects before fleet mutation. Conflict, capacity, missing release,
  transport, and invalid-state outcomes retain typed public codes.
- The exact interrupted-publication PocketIC case commits target material
  before the root mirror, upgrades root, reuses the same store, and converges
  to exactly one root-projected owner.
- Focused security-ordering additions pass 18 tests across token prepare,
  lazy repair, public prepare, replay abort, and recovery-required receipt
  preservation.
- Lifecycle validation passes 2 structural boundary tests, 1 trap boundary
  test, and 3 PocketIC install/upgrade/failure tests.
- Capability validation rebuilds six retained artifacts, passes 19 protocol
  tests and targeted Clippy, and confirms intended root/non-root/issuer
  placement. Required workspace Clippy is blocked by policy and not fabricated.
- Publish validation passes locked/offline metadata, 6 workspace-manifest
  tests, the release package/install definition guard, and isolated offline
  `cargo package` verification for all 8 public crates.
- Structure validation passes isolated public-surface mapping, module layout,
  crate-cycle, test/fleet seam, and 5 focused DTO/policy boundary tests. Core
  rustdoc with warnings denied fails on one unresolved internal link.
- DRY validation passes 8 core root-policy, 19 host registry-related, 2 host
  response-parser, 30 CLI output-filtered, and 19 backup persistence tests.
  Direct ownership traces find no competing registry, output, evidence,
  backup, or release-proof flow beyond the indexed issuer-policy defect.
- Complexity v2 reproduces its complete scope/counter output, maps every file,
  and applies one deterministic score. Five focused filters pass 178 selected
  executions; the valid result fails at risk 8/10 on the bounded P2 hotspot.
- Change-friction v2 maps all 546 current files, reproduces normalized digest
  `aac8db07...` twice, applies one score, and validly fails at risk 8/10.
  Five exact slices and 74 focused RPC/capability/delegation tests complete.
- Instruction v1 fails closed at the obsolete fixture build path and remains
  invalid history. Corrected v2 passes 37 focused tests plus its ignored live
  generator, executes all 12 isolated PocketIC scenarios, retains 12
  normalized rows and 21 checkpoint deltas, finds 57 static checkpoints, and
  verifies every compact evidence hash. The valid result is partial only on
  the two indexed auth-flow checkpoint gaps.
- Wasm v1 fails closed at the obsolete direct build and remains invalid
  history. V2 completes 12 authoritative role/profile builds in a clean
  detached worktree, verifies every builder gzip pair, analyzes all six
  release artifacts with `ic-wasm` and `twiggy`, preserves tracked product
  source, and verifies the primary report plus ten compact artifacts.
- Module-surface validation passes all-feature `canic-core` check, 3 focused
  provisioning tests, and 2 facade proof-surface tests. The public protocol
  pin confirms the internal batch request remains unnecessarily serialized;
  direct core-error root consumer scans find none.

## Next Action

Phase C is complete and the
[Phase D finding review](../audits/reports/2026-07/2026-07-15/0.92-phase-d-finding-review.md)
maps the original 23 unresolved findings to bounded dispositions. D1 through
D4 are released and D5 is implemented and validated, leaving 16 unresolved
findings. The next ordered candidate is D6 passive RPC DTO ownership. D6
through D10 and the remaining layering subsystems remain separately bounded;
the proposed scanner limitation is not yet a waiver.
