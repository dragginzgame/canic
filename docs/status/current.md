# Current Status

Last updated: 2026-07-16

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the source, design, audit, or changelog files needed for the current task.

Historical detail is archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md); and
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md).

## Current Release

- The workspace package version is `0.92.10`.
- The latest published release is `v0.92.10` at
  `35de57b53a5c331977e3f7ac49e8190355b1d9f4`. D12 was initially committed at
  `2e4131571aeb6ca13f050b012db30602d8e20b1b`; its focused scan-contract
  hardening remains an Unreleased candidate until the maintainer assigns the
  final commit and tag.
- The `v0.92.10` source tree is
  `b6b7541e697c264b1b40cd60a8a6fc72f497e9cd`; its product-tree hash is
  `ad5421cac98f605266e55af9e55e7a1fd1845f56f774082c8f27b6714b25d5bb`.
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
`v0.92.3`; D5 is released in `v0.92.4`; D6 in `v0.92.5`; and D7 in
`v0.92.6`. D8 is released in `v0.92.7`. Root runtime records and lifecycle
diagnostics no longer contain
absolute build paths, isolated root/bootstrap artifacts reproduce byte for
byte, and build provenance requires explicit optional-transform identity and
outcome. This fixes `CANIC-092-BUILD-001` and `CANIC-092-BUILD-002`; the
current mandatory-trace ledger is ten pass and zero fail. D9 is released in
`v0.92.8`: 13 external Action
executions use immutable commits, downloaded tools are version/checksum bound,
and one governance matrix owns the Ubuntu 24.04 x86_64 native/Wasm support
cell. This fixes `CANIC-092-RELEASE-001`, `-002`, and `-004`; the dedicated
scanner gap remained blocked after D9 and is fixed by D12. D10 is released in
`v0.92.9` and
focused
validation passes: published feature documentation matches the owning
manifests, active CLI proof asserts only maintained commands, warning-as-error
core rustdoc passes, and installed plus packaged downstream artifact proofs
pass before registry publication. This fixes `CANIC-092-PUBLISH-001`,
`CANIC-092-RESIDUE-001`, and `CANIC-092-DOCS-002`. D11 is released in
`v0.92.10` and
focused validation passes: shared decision inputs now belong to model,
root-proof admission belongs to workflow, and ops no longer imports policy.
This fixes `CANIC-092-LAYERING-005`, and the live layering guard passes with
zero violations. D12 is implemented and its focused current-tree rerun passes:
Gitleaks 8.30.1 is version/checksum bound, scans complete history with full
redaction, and reports zero unreviewed findings. This fixes
`CANIC-092-RELEASE-003` without a waiver. Three P2 findings remain unresolved,
all with explicit deferred dispositions; no P0 or P1 remains.

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
- D1 is released in `v0.92.1`, D2/D3 in `v0.92.2`, D4 in `v0.92.3`, D5 in
  `v0.92.4`, D6 in `v0.92.5`, D7 in `v0.92.6`, D8 in `v0.92.7`, D9 in
  `v0.92.8`, D10 in `v0.92.9`, and D11 in `v0.92.10`. D12 is implemented and
  validated against the `v0.92.10` parent.
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
- [D9 release execution integrity](../audits/reports/2026-07/2026-07-15/0.92-d9-release-execution-integrity.md)
  fixes the action, executable-tool, and support-matrix findings. Its affected
  subchecks pass. The dedicated-scanner gap that remained after D9 is fixed by
  D12 below, so the current affected release-integrity rerun passes.
- [D10 active documentation and hard-cut residue](../audits/reports/2026-07/2026-07-15/0.92-d10-active-documentation-and-hard-cut-residue.md)
  fixes the package-feature, active legacy-proof residue, and public-rustdoc
  findings. Manifest-derived docs proof, installed/packaged CLI proof, and
  generated/canonical packaged Wasm-store proof pass without product behavior
  or serialized-surface changes.
- [D11 canonical layering closure](../audits/reports/2026-07/2026-07-15/0.92-d11-canonical-layering-closure.md)
  fixes the remaining 18 ops-to-policy dependencies. Shared data belongs to
  model, root delegation-proof admission belongs to workflow, and the current
  executable layering guard passes with zero violations.
- [D12 dedicated secret scan](../audits/reports/2026-07/2026-07-16/0.92-d12-dedicated-secret-scan.md)
  fixes the final P1 evidence gap. The version/checksum-bound scanner runs over
  complete history with full redaction; 11 reviewed false positives are
  admitted only by exact fingerprints, and the rerun reports zero findings.
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
  fails, fixes `CANIC-092-AUDIT-008`, and left `CANIC-092-BUILD-001` and
  `CANIC-092-BUILD-002` open at the frozen baseline.
- [D8 reproducible root artifacts](../audits/reports/2026-07/2026-07-15/0.92-d8-reproducible-root-artifacts.md)
  fixes both build findings. Two isolated offline lanes now produce identical
  root/bootstrap raw and gzip artifacts plus identical semantic provenance;
  final root Wasm contains neither temporary root. The host builder records
  transform role, kind, optional mode, tool, version, and applied, unavailable,
  or unrequested outcome through the required hard-cut provenance shape.
- [authentication invariants](../audits/reports/2026-07/2026-07-14/0.92-auth-invariants.md)
  found no accepting bypass: invalid trust, audience, subject, scope, replay,
  and attestation inputs reject in focused unit/PocketIC evidence. The original
  audience/replay v1 attempts remain invalid history. Corrected
  [audience/replay v2](../audits/reports/2026-07/2026-07-14/0.92-auth-invariants-v2.md)
  methods use current exact filters through a zero-test-refusing runner and
  validly pass at risk 3/10, fixing `CANIC-092-AUDIT-009`. D2 fixes
  `CANIC-092-ERROR-001` by preserving typed proof/provisioning causes. D7
  subsequently fixes `CANIC-092-LAYERING-004` by deleting the accidental
  public install-state DTO surface.
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
  unchanged. The canonical layering finding remained open after D4 and is
  fixed by D11.
- [D5 blob-billing workflow ownership](../audits/reports/2026-07/2026-07-15/0.92-d5-blob-billing-workflow-ownership.md)
  fixes `CANIC-092-LAYERING-001`. API now delegates Cashier sequencing,
  reserve/recovery, gateway sync, and readiness to one workflow over pure
  policy and single-step ops. Public DTOs, Candid, billing prices/protocol, and
  stable records are unchanged; the current blob trace passes.
- [D6 passive RPC DTO ownership](../audits/reports/2026-07/2026-07-15/0.92-d6-passive-rpc-dto-ownership.md)
  fixes `CANIC-092-LAYERING-002`. Request DTOs retain only boundary data and
  neutral constructors; one workflow command owns capability family, replay
  identity, and metadata attachment, while ops owns signed-payload projection.
  Public protocol, replay behavior, and stable state are unchanged.
- [D7 internal surface hard cuts](../audits/reports/2026-07/2026-07-15/0.92-d7-internal-surface-hard-cuts.md)
  fix `CANIC-092-LAYERING-004` and `CANIC-092-SURFACE-001`. The duplicate
  public proof-install request/outcome and direct core error root are deleted;
  the internal ops plan, model failure classification, and deliberate
  control-plane support bridge retain one owner. No Candid endpoint or stable
  schema changes.
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
  `CANIC-092-DOCS-001`; D10's current-tree rerun fixes
  `CANIC-092-PUBLISH-001` and `CANIC-092-RESIDUE-001` with complete
  manifest-derived feature docs and maintained-command-only proof.
- [module structure v1](../audits/reports/2026-07/2026-07-14/0.92-module-structure-v1.md)
  is a valid fail and first frozen-method baseline at risk 7/10. It confirms 25
  production ops-to-policy imports, direct policy decisions in ops, and
  policy-owned values used by stable mappers (`CANIC-092-LAYERING-005`). It
  finds no cycle, public record leak, test/fleet seam breach, or module-layout
  escape. D10's current-tree warning-as-error rustdoc rerun fixes the separate
  `CANIC-092-DOCS-002` link without changing the frozen result.
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
  sibling-support, and test-only surfaces otherwise retain current owners. D7
  fixes both product findings without rewriting the frozen result.
- [Phase C baseline review](../audits/reports/2026-07/2026-07-14/0.92-phase-c-baseline-review.md)
  remains the original blocked synthesis. The live ledger now has 22 valid
  and zero invalid active results after the instruction and Wasm corrections.
  The final mandatory trace result is valid and complete at aggregate `fail`.
  D1 fixes two non-waivable publication P1 findings, D2 fixes the auth cause
  P1, D3 fixes one P1 authority conflict plus one P2 documentation drift, D7
  fixes two P2 accidental public surfaces, and D8 fixes the root-build P1 plus
  transform-provenance P2. D9 fixes the two release-execution P1s plus the
  support-matrix P2. D10 fixes two P2 active package/proof findings and the P3
  rustdoc drift. D11 fixes the remaining layering P1, and D12 fixes the
  dedicated-scanner P1 without a waiver. Three P2 findings remain unresolved
  with explicit deferred dispositions.
  All current trace reruns pass; the frozen Phase C aggregate remains
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
- The original credential-pattern scan found no match. D12's dedicated
  Gitleaks 8.30.1 full-history scan also passes after 11 false positives were
  classified and admitted only by exact historical fingerprints.
- Layering v2 detector fixtures pass. Its immutable baseline validly fails on
  25 production ops-to-policy dependencies; D4's affected-scope rerun reduces
  the live set to 18. Policy purity, passive DTO, and root-issuer ownership
  checks pass.
- Build-integrity v2's frozen baseline retains the root-path failure. D8's
  affected-scope rerun executes two new isolated root lanes: root/bootstrap
  raw and gzip artifacts plus semantic provenance reproduce exactly, neither
  final root contains a temporary lane path, and all four local transforms
  record `ic-wasm 0.9.11`.
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
  guard retains the same 25 known product-code violations. D10 later fixes the
  separately indexed public rustdoc link.
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
- D6 validation passes 51 workflow RPC tests, eight RPC ops tests, four replay
  manifest tests, four policy/DTO guards, 19 protocol tests, strict
  all-feature/all-target core Clippy, and exact PocketIC identical-replay and
  cross-family-conflict cases. The capability trace remains pass and the live
  layering set at the D6 boundary remains 18.
- D7 validation passes all-feature/all-target core and control-plane checks,
  four provisioning tests, 20 chain-key batch tests, three DTO/serialization
  guards, 19 protocol tests, all-feature facade check, strict
  core/control-plane Clippy, offline core/facade package verification, the
  feature-owned control-plane facade test, and exact PocketIC new-issuer
  provisioning. The live layering set at the D7 boundary remains 18. D10's
  later current-tree rustdoc rerun passes with warnings denied.
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
- Publish validation passes locked/offline metadata, seven workspace-manifest
  tests, the release package/install definition guard, installed and packaged
  CLI proof, and generated plus canonical packaged Wasm-store builds.
- Structure validation passes isolated public-surface mapping, module layout,
  crate-cycle, test/fleet seam, and five focused DTO/policy boundary tests.
  D10's current-tree core rustdoc passes with warnings denied.
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
- Module-surface D7 validation passes all-feature `canic-core` check, focused
  provisioning and facade proof-surface tests, protocol and package checks,
  and direct consumer scans. The duplicate serialized batch request/outcome
  and direct core-error root are removed; the maintained control-plane support
  bridge remains.
- D8 validation passes 7 artifact-transform tests, 15 build-provenance/policy
  tests, 12 release-set manifest tests, targeted checks and strict Clippy, and
  two isolated offline root builds. The current deploy trace now passes; the
  full clean-worktree Wasm v2 retained rerun waits for the immutable
  maintainer commit rather than fabricating a dirty-tree result.
- D9 validation passes the 13-action immutable-pin guard, positive and
  rejection checksum proof, all five external executable install/version
  probes, canonical caller-override rejection and exact IC tool checks,
  workflow and shell lint, 22 focused Rust tests, targeted strict Clippy, the
  host package probe, and matrix/changelog/diff guards. The integrity guard
  discovers the maintained version/checksum surface instead of duplicating a
  variable ledger, and `update-dev` no longer updates unrelated toolchains or
  workspace dependencies. Release mutation is transactional on failure,
  one-shot phases are sequential under parallel Make, and push rejects unless
  the clean release commit and annotated tag match before an atomic ref update.
- D10 validation passes seven manifest-derived package tests, positive CLI
  unit proof, warning-as-error core rustdoc, strict targeted Clippy, installed
  CLI proof, packaged CLI proof, and generated plus canonical packaged
  Wasm-store builds. Bash syntax, ShellCheck, targeted formatting, and diff
  hygiene pass.
- D11 validation passes focused environment, funding, topology, placement,
  metrics, pool, and auth tests; default and sharding core checks; strict
  all-target/all-feature core Clippy; and the executable layering guard with
  zero production ops-to-policy dependencies. Public, serialized, stable,
  configuration, and dependency surfaces are unchanged.
- D12 validation passes the checksum-bound Gitleaks install and exact-version
  probe, redacted full-history scan, unavailable/near-match version,
  rule-configuration override, shallow-history, and argument rejection probes,
  release-integrity and validation-matrix guards, `actionlint`, Bash syntax,
  changed-script ShellCheck, and `make gitleaks-scan`.
  The admitted result has zero unreviewed findings and retains no raw report.

## Next Action

Phase C is complete and the
[Phase D finding review](../audits/reports/2026-07/2026-07-15/0.92-phase-d-finding-review.md)
maps the original 23 unresolved findings to bounded dispositions. D1 through
D11 are released through `v0.92.10`, and D12 is implemented and validated.
No P0 or P1 remains. The next step is Slice E closeout: confirm the three
deferred P2 dispositions, execute the `v0.91.6` compatibility accounting, and
publish one explicit release-line verdict. Broad release validation remains
maintainer-owned.
