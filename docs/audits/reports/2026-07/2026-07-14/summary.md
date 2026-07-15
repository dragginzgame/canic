# 2026-07-14 Audit Summary

## Run Context

- Compatibility anchor: `v0.91.6` at
  `5f7a89f9b966ebf2755d5630ddcba0cdf968ebb1`.
- Frozen Phase C product/method snapshot: `v0.92.0` at
  `91736337fc1cfeb891f17d7d62affb5e671348e2`.
- Product-tree hash:
  `c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e`.
- Scope: Phase A inventory, Phase B method hardening, immutable admission, and
  Phase C dependency, release-integrity, layering, build-integrity, seven
  authentication-invariant results, security-boundary ordering, lifecycle
  symmetry, capability-surface v1, publish-surface v1, module-structure v1,
  DRY-consolidation v1, complexity-accretion v1/v2, change-friction v1/v2,
  instruction-footprint v1, Wasm-footprint v1, canic-core module-surface
  hardening v2.0, mandatory-trace admission, mandatory traces v1, supporting publication evidence,
  the Phase C baseline review, dependency v2 correction/rerun, audience/replay
  v2 correction/rerun, capability-surface v2 correction/rerun, layering v2
  correction/rerun, build-integrity v2 correction/rerun, and exact
  product-baseline identity correction.

Primary reports:

- [audit-system inventory](0.92-audit-system-inventory.md);
- [audit-system hardening](0.92-audit-system-hardening.md);
- [method freeze and product admission](0.92-method-freeze.md);
- [dependency hygiene v1 attempt](0.92-dependency-hygiene-v1.md);
- [dependency hygiene v2](0.92-dependency-hygiene-v2.md);
- [product baseline identity correction](0.92-product-baseline-identity-correction.md); and
- [CI and release integrity](0.92-release-integrity.md);
- [layer boundary](0.92-layer-boundary.md); and
- [layer boundary v2](0.92-layer-boundary-v2.md); and
- [build integrity v1](0.92-build-integrity-v1.md); and
- [build integrity v2](0.92-build-integrity-v2.md); and
- [authentication invariants](0.92-auth-invariants.md); and
- [audience and replay invariant v2 correction](0.92-auth-invariants-v2.md); and
- [mandatory trace admission](0.92-mandatory-trace-admission.md); and
- [mandatory end-to-end traces v1](0.92-mandatory-traces-v1.md); and
- [control-plane publication supporting trace](0.92-control-plane-publication.md);
- [security boundary ordering](0.92-security-boundary-ordering.md); and
- [bootstrap lifecycle symmetry](0.92-bootstrap-lifecycle-symmetry.md); and
- [capability surface v1](0.92-capability-surface-v1.md); and
- [capability surface v2](0.92-capability-surface-v2.md); and
- [publish surface v1](0.92-publish-surface.md); and
- [module structure v1](0.92-module-structure-v1.md); and
- [DRY consolidation v1](0.92-dry-consolidation-v1.md); and
- [complexity accretion v1](0.92-complexity-accretion-v1.md); and
- [complexity accretion v2](0.92-complexity-accretion-v2.md); and
- [change friction v1](0.92-change-friction-v1.md); and
- [change friction v2](0.92-change-friction-v2.md); and
- [instruction footprint v1](instruction-footprint.md); and
- [Wasm footprint v1](wasm-footprint.md).
- [canic-core module surface hardening](canic-core-module-surface-hardening.md).
- [Phase C baseline review](0.92-phase-c-baseline-review.md).

## Risk Summary

Phase A found six confirmed P1 audit-system defects. Phase B corrected them,
and the published `v0.92.0` snapshot contains the exact frozen method content.
The committed product delta is fully classified, so Phase C is admitted.

The first Phase C batch found:

1. one P1 dependency-method defect: v1 had no deterministic license decision;
   v2 now inventories external declarations without claiming legal policy;
2. zero known vulnerabilities in the locked graph using the recorded July 13
   cached advisory database, plus four P2 unmaintained-package pressure paths;
3. mutable tags on all 16 external GitHub Action references;
4. mutable or unverified executable/tool installation paths in CI;
5. no approved dedicated secret scanner; and
6. no canonical supported host/target matrix;
7. blob-storage billing orchestration in the API layer;
8. capability/replay canonicalization behavior in the root RPC DTO;
9. conflicting active rules for direct API-to-ops delegation;
10. a frozen build-method defect around timestamp-bearing provenance;
11. absolute build paths embedded into non-reproducible shipped root Wasm and
    lifecycle diagnostics; and
12. optional artifact-transform identity missing from build provenance;
13. four frozen authentication commands that exit successfully after selecting
    zero tests, invalidating audience v1 and replay v1;
14. missing generated-endpoint negative/parity and delegated-session bootstrap
    integration proof;
15. typed auth proof and root-to-issuer provisioning causes flattened into
    strings or broad outcomes; and
16. internal root proof-install plan/outcome types exported as public DTOs,
    including three production-unreachable outcome variants; and
17. no versioned or fingerprinted mandatory trace method despite ten required
    trace IDs and required method-identity fields in every trace report; the
    v1 protocol and admitted trace run now fix this method defect;
18. durable-publication endpoints labeled with implemented quota/cycle-reserve
    policies whose runtime publication path reserves no cost permit;
19. distinct publication conflict/capacity/recovery failures projected through
    string-backed workflow errors and generic `Internal`; and
20. no executable interruption proof between target promotion/root mirroring
    or across a late multi-manifest failure; and
21. a frozen capability-surface method that requires a broad workspace Clippy
   gate which canonical agent policy forbids without an exact maintainer
   request;
22. six maintained facade features and the control-plane default feature split
   absent from their public package READMEs;
23. public `canic-core` docs teaching a competing layer order and state owner;
   and
24. active installed/packaged CLI proof and operations guidance retaining
   forbidden anti-resurrection checks for removed auth command forms;
25. a frozen layering scan/guard that misses grouped ops-to-policy imports and
   invalidates the earlier layer-boundary result;
26. 25 production ops files depending upward on policy, including policy-owned
   state types and direct policy-decision calls; and
27. one unresolved public `canic-core` rustdoc link to a crate-private type;
   and
28. no direct rejection or unchanged-state proof for the four root-issuer
   policy upsert admission branches;
29. a frozen complexity method whose undefined CAF input, unfrozen search
    identities, overlapping additive modifiers, and unmapped role-contract
    scope prevented a reproducible authoritative score; the deterministic v2
    method and rerun now fix this method defect; and
30. concentrated delegated-auth and chain-key trust-path complexity across six
    large modules, 45 typed error variants, and high flow/call-depth pressure;
    and
31. a frozen change-friction v1 method whose incomplete subsystem/layer map,
    competing score algorithms, and unfrozen sample/counter identities
    prevented a reproducible baseline; corrected v2 now fixes this method
    defect with complete scope, a frozen sample, and one score; and
32. a frozen instruction method whose root-dependent composite fingerprint,
    obsolete direct Cargo Wasm fixture build, incomplete scenario roster, and
    namespaced-checkpoint blind scan prevent measurement; and
33. a frozen Wasm method whose root-dependent composite and required direct
    Cargo artifact model conflict with the authoritative `canic build` hard
    cut, preventing all size and retained-hotspot measurements; and
34. one hidden-but-public `canic_core::error` root path with no current direct
    consumer, while the existing control-plane support bridge already owns the
    required cross-crate contract; and
35. a P1 audit-evidence identity defect: Phase C initially carried the Phase B
    product hash instead of hashing the exact published `v0.92.0` commit. The
    source commit was correct and the derived identity is now corrected.

The dependency v1 attempt remains invalid history. Corrected dependency v2 is
a valid pass at risk 3/10: all 484 external packages identify license metadata,
zero vulnerabilities are known in the cached database, and four unmaintained
packages remain pressure. Release integrity is a valid failure. Layering v1
remains invalid history. Corrected v2 detector fixtures pass and the immutable
product scan validly fails at risk 7/10 on 25 production ops-to-policy
dependencies; the API/DTO authority findings remain confirmed. Build v1
remains invalid history. Corrected v2 semantic provenance passes for
byte-identical app/bootstrap artifacts and validly fails for path-dependent
root raw/gzip artifact hashes. Product code remains unchanged and
product fixes are not yet authorized. The authentication code/execution trace
found no bypass: invalid trust, audience, subject, scope, replay, and
attestation cases reject. The original combined v1 report remains invalid
history, while corrected audience/replay v2 methods now pass validly at risk
3/10 with enforced nonempty test selection. The remaining auth boundary,
equivalence, and subject methods still fail validly on missing integration
proof, and typed causes remain a product finding. The independently frozen cross-stage security
ordering method passes with watchpoints: no inspected handler or mutation runs
before its owning auth, proof, subject/scope, capability, or replay gate. The
frozen lifecycle method also passes with watchpoints: root and non-root
init/upgrade flows restore synchronously and schedule bootstrap/user work
through zero-delay timers. Capability v1 remains invalid history because its
broad Clippy requirement conflicts with canonical execution authority.
Corrected v2 makes the owning 19-test and targeted-Clippy contract normative;
both pass, all six retained artifacts rebuild, and the result validly passes
at risk 4/10 with attributable global controller-query growth.
Publish-surface v1 is a valid pass with risk 4/10: all eight intended public
packages verify from isolated offline archives and their roles remain bounded,
while three P2 documentation/governance findings remain open.
Module-structure v1 is a valid failure at risk 7/10. No cycle, public record
leak, test seam breach, or layout escape was found, but the ops-to-policy edge
is broad and confirmed. DRY-consolidation v1 is a valid failure at risk 6/10:
operator, evidence, backup, and release-proof ownership is generally clear,
while issuer-policy admission duplicates the same authority already indexed
as `CANIC-092-LAYERING-005`; its missing rejection proof is separately indexed.
Complexity-accretion v1 remains invalid history. Corrected v2 owns its scope,
mechanical counters, exact manual evidence, CAF definition, and one
non-overlapping score. Two runner executions reproduce exactly; five focused
filters pass 178 selected executions. The valid first v2 baseline fails at
risk 8/10 on the retained P2 trust-path concentration, not a correctness claim.
Change-friction v1 remains partial/invalid history. Corrected v2 maps all 546
current files, freezes the five exact slices and their flow axes, and produces
the same normalized digest twice. Its valid first baseline fails at risk 8/10:
the 0.90.1 intent slice remains the broad case at 19 files and CAF 48, while 74
focused tests pass. Pressure is deduplicated into existing layering and
complexity findings rather than creating a new product defect.
Instruction v1 is blocked/invalid before producing a perf row. Pinned PocketIC
14.0.0 starts, but the root probe uses the forbidden direct Cargo Wasm path.
Its retained 11-scenario manifest and static 57-checkpoint scan are supporting
evidence only; no instruction regression or score is claimed.
Wasm v1 is also blocked/invalid before producing an artifact. Its required
clean linked-worktree execution reaches the first `app` direct Cargo build,
which the product correctly rejects. All analysis tools are available, but no
raw/shrunk/debug size or `twiggy` evidence exists.
Module Surface Hardening v2.0 is a valid first-method failure at risk 4/10.
It independently confirms the existing internal proof-install DTO finding and
adds one P2 hard-cut surface finding for the unnecessary public core error
path. Generated, replay-policy, state-contract, control-plane support, and
test-only surfaces otherwise retain current named owners.

All 22 frozen retained definitions have now been attempted: 14 system, seven
authentication, and one manual-only module-surface method. That completes the
retained-method run ledger, but not the product baseline: two invalid methods
require versioned correction. The mandatory trace protocol is now valid and
all ten trace IDs have admitted results, but the aggregate is `partial` because
auth integration and interrupted publication evidence remain incomplete.

The [Phase C baseline review](0.92-phase-c-baseline-review.md) therefore records
`blocked`, not ready. After the trace, complexity, and friction corrections,
20 retained results are valid, 2 are invalid, and 26 findings remain unresolved
(13 P1, 12 P2, one P3). No waiver is complete and
four P1 findings are treated as non-waivable. It defines the method-correction
and rerun order but does not accept or authorize product fixes.

## Method and Comparability Notes

- The frozen method manifest is `fa92c4102...`.
- Mandatory traces use `CANIC-MANDATORY-TRACE-001/v1`, fingerprint
  `ea2c06b0...`. All ten traces ran; six pass, deploy/blob fail on existing
  findings, and auth/control are partial on existing evidence gaps. The
  aggregate is `partial`, so the mandatory-trace gate remains incomplete.
- Dependency v1 (`71be0c1d...`) remains invalid history. Corrected
  `CANIC-DEPENDENCY-001/v2` (`ad7b4596...`) is a valid first comparable result
  at risk 3/10 on the immutable `v0.92.0` baseline.
- Release-integrity identity is `CANIC-RELEASE-INTEGRITY-001/v1`, fingerprint
  `3f6b87b3...`; its result is valid and first-of-method for Phase C.
- Build v1 (`57f0a380...`) remains invalid history. Corrected
  `CANIC-BUILD-INTEGRITY-001/v2` (`e75c8fdc...`) validly fails: app and
  bootstrap raw/gzip bytes plus app semantic provenance reproduce, while root
  raw/gzip bytes and semantic artifact hashes remain path-dependent.
- The seven authentication identities and exact fingerprints are recorded in
  the primary auth reports. Boundary, equivalence, capability, subject, and
  trust v1 results are valid. Audience v2 (`bfe780a3...`) and replay v2
  (`743b9fcc...`) are valid passes at risk 3/10; their shared wrapper
  (`8f4a46a2...`) refuses a successful Cargo selection with zero passing
  tests. Audience/replay v1 remain invalid history.
- Security ordering is `CANIC-AUTH-ORDERING-001/v1`, fingerprint
  `bf5e5a5b...`; its first-of-method Phase C result is valid.
- Lifecycle symmetry is `CANIC-LIFECYCLE-001/v1`, fingerprint
  `c3b99716...`; it is partially comparable to the June 22 unversioned report
  and valid as the first frozen-method result.
- Capability v1 (`d7de4f8b...`) remains invalid history. Corrected
  `CANIC-CAPABILITY-SURFACE-001/v2` (`91e61f33...`) validly passes at risk
  4/10 after six retained artifact refreshes, 19 nonempty protocol tests, and
  targeted warning-as-error Clippy.
- Layering v1 (`86270ae4...`) remains invalid history. Corrected
  `CANIC-LAYERING-001/v2` (`a4c71532...`) is a valid failure at risk 7/10;
  its fingerprinted detector fixtures pass and its guard reports all 25
  production ops-to-policy files.
- Publish surface is `CANIC-PUBLISH-001/v1`, fingerprint `8e2eff6a...`; its
  valid result is non-comparable as the first frozen-method baseline.
- Module structure is `CANIC-STRUCTURE-001/v1`, fingerprint `ca370a2c...`;
  its valid failed result is non-comparable as the first frozen-method
  baseline. Its evidence invalidates layering v1 without editing either frozen
  method definition.
- DRY consolidation is `CANIC-DUPLICATION-001/v1`, fingerprint `c4b2b282...`;
  its valid failed result is non-comparable as the first frozen-method
  baseline. No method defect was found.
- Complexity v1 (`47bc0761...`) remains invalid history. Corrected
  `CANIC-COMPLEXITY-001/v2` (`76bb53a5...`) is a valid first baseline at risk
  8/10. Its mechanical runner (`4ff697d1...`) reproduces normalized digest
  `7df0755b...`; `role-contract` is fully mapped and one scoring algorithm owns
  all modifiers.
- Change friction v1 (`00646b25...`) remains invalid history. Corrected
  `CANIC-CHANGE-FRICTION-001/v2` (`5f4377f0...`) is a valid first baseline at
  risk 8/10. Its runner (`7ffa84f7...`) and fixture (`42b440e2...`) produce
  normalized digest `aac8db07...` twice, classify all 546 current files, and
  apply one score.
- Instruction footprint is `CANIC-INSTRUCTION-001/v1`, definition fingerprint
  `f90bbd14...`, expected composite `c79f7027...`. The runner emits
  root-dependent `a5fa45ef...` and the attempted result is blocked/invalid.
- Wasm footprint is `CANIC-WASM-001/v1`, definition fingerprint `1ed32dd3...`,
  expected composite `e8c58213...`. The linked-worktree runner identity is
  root-dependent `8c8e2248...`; the attempted result is blocked/invalid.
- Module Surface Hardening is `CANIC-MODULE-SURFACE-001/v2.0`, fingerprint
  `404a359b...`; its first frozen manual-only run is valid, non-comparable, and
  fails at risk 4/10.

## Findings

| Finding | Severity | Status | Summary |
| --- | --- | --- | --- |
| `CANIC-092-AUDIT-001` through `-006` | P1 | fixed | Phase A audit-system defects closed at `v0.92.0`. |
| `CANIC-092-AUDIT-007` | P1 | fixed | Dependency v2 defines and passes deterministic external declaration inventory. |
| `CANIC-092-DEPENDENCY-001` | P2 | open | Four reachable unmaintained transitive packages. |
| `CANIC-092-RELEASE-001` | P1 | open | External Actions use mutable tags. |
| `CANIC-092-RELEASE-002` | P1 | open | Executable tool identity/integrity is not fixed. |
| `CANIC-092-RELEASE-003` | P1 | blocked | Dedicated secret-scan evidence unavailable. |
| `CANIC-092-RELEASE-004` | P2 | open | Supported host/target matrix is not canonical. |
| `CANIC-092-LAYERING-001` | P2 | open | API orchestration includes blob billing plus runtime/auth projection/sequencing. |
| `CANIC-092-LAYERING-002` | P2 | open | Root RPC DTO owns capability decisions. |
| `CANIC-092-LAYERING-003` | P1 | open | Endpoint dependency authority conflicts. |
| `CANIC-092-AUDIT-008` | P1 | fixed | Build v2 applies the exact timestamp/digest semantic rule and preserves root artifact drift. |
| `CANIC-092-BUILD-001` | P1 | open | Root Wasm embeds absolute builder paths and is not reproducible. |
| `CANIC-092-BUILD-002` | P2 | open | Provenance omits optional artifact-transform identity/status. |
| `CANIC-092-AUDIT-009` | P1 | fixed | Audience/replay v2 use current exact filters and reject zero-test success; both reruns pass. |
| `CANIC-092-AUTH-001` | P1 | open | Negative generated-endpoint and session-bootstrap integration proof is incomplete. |
| `CANIC-092-ERROR-001` | P1 | open | Auth proof and provisioning paths discard typed causes. |
| `CANIC-092-LAYERING-004` | P2 | open | Internal proof-install state is accidental public DTO surface. |
| `CANIC-092-AUDIT-010` | P1 | fixed | Mandatory trace v1 is cataloged/fingerprinted and all ten trace IDs have admitted results. |
| `CANIC-092-COST-001` | P1 | open | Durable publication bypasses its declared cost guard. |
| `CANIC-092-ERROR-002` | P1 | open | Publication failures collapse distinct typed causes. |
| `CANIC-092-PUBLICATION-001` | P1 | open | Interrupted publication convergence lacks executable proof. |
| `CANIC-092-AUDIT-011` | P1 | fixed | Capability v2 uses its owning targeted test/Clippy contract; the corrected baseline passes. |
| `CANIC-092-PUBLISH-001` | P2 | open | Public package docs omit maintained facade features and the control-plane default feature split. |
| `CANIC-092-DOCS-001` | P2 | open | Public `canic-core` docs conflict with the canonical layer contract. |
| `CANIC-092-RESIDUE-001` | P2 | open | Active CLI proof retains forbidden old-command anti-resurrection checks and breadcrumbs. |
| `CANIC-092-AUDIT-012` | P1 | fixed | Layering v2 fixture/guard coverage detects all 25 production ops-to-policy files. |
| `CANIC-092-LAYERING-005` | P1 | open | Ops depends upward on policy and policy owns runtime state-shaped values. |
| `CANIC-092-DOCS-002` | P3 | open | Public core rustdoc links to crate-private `InternalError`. |
| `CANIC-092-TEST-001` | P2 | open | Root-issuer policy upsert rejection and unchanged-state paths lack direct proof. |
| `CANIC-092-AUDIT-013` | P1 | fixed | Complexity v2 has deterministic scope/counters/manual evidence/score and reproduces exactly. |
| `CANIC-092-COMPLEXITY-001` | P2 | open | Delegated-auth and chain-key trust paths concentrate variant, flow, hub, and call-depth pressure. |
| `CANIC-092-AUDIT-014` | P1 | fixed | Change-friction v2 has exhaustive scope/layers, a frozen fixture, exact formulas, and one reproducible score. |
| `CANIC-092-AUDIT-015` | P1 | open | Instruction v1 has a root-dependent composite, obsolete fixture build, incomplete roster, and blind checkpoint scan. |
| `CANIC-092-AUDIT-016` | P1 | open | Wasm v1 has a root-dependent composite and requires the removed direct Cargo Wasm build path. |
| `CANIC-092-SURFACE-001` | P2 | open | The internal error model remains reachable through an unnecessary public core root path. |
| `CANIC-092-AUDIT-017` | P1 | fixed | Exact published product-tree identity replaces the carried-forward Phase B hash. |

## Verification Rollup

- Release/tag/origin, method-path equality, product-tree identity, and product
  delta classification: `PASS`.
- Audit catalog and exact frozen method fingerprints: `PASS`.
- `cargo metadata --locked --offline`: `PASS`.
- `cargo tree --workspace --locked --offline`: `PASS`.
- `cargo-audit 0.22.2` with advisory DB commit `9f3e1380...`: `PASS`, zero
  vulnerabilities and four informational unmaintained warnings.
- Dependency v2 license-declaration inventory: `PASS`; 484 external packages
  declare metadata across 32 observed expression strings. No legal-family
  allow/deny claim is made.
- `actionlint 1.7.12`: `PASS`.
- Workflow permission/trigger/secret reach: `PASS`.
- External action identity and downloaded tool integrity: `FAIL`.
- Local credential-pattern scan: `PASS`; dedicated scan: `BLOCKED`.
- Layering v2: `FAIL`, valid, risk 7/10. Detector fixtures pass; the full guard
  exits 1 and lists 25 production ops-to-policy files. Policy/DTO/model/storage
  purity checks and targeted current tests pass. Layering v1 remains invalid
  history.
- Build v2 four isolated release-profile builds: `PASS` execution. App,
  bootstrap-store, embedded asset, deterministic gzip, and app semantic
  provenance equality: `PASS`.
- Root raw/gzip byte equality: `FAIL`; generated absolute runtime paths are the
  confirmed cause and enter init/upgrade logs.
- Root normalized provenance: `FAIL` only on final raw/gzip artifact hashes;
  this is a valid product failure. Build v1 remains invalid history.
- Authentication unit/PocketIC execution: all selected tests passed, including
  proof-chain, audience, scope, replay, role-attestation, chain-key batch,
  root facade, and unauthorized capability cases.
- Audience/replay v2: `PASS`, valid, risk 3/10. Every current filter selected
  nonzero passing tests, both PocketIC cases passed, and the wrapper's
  intentional zero-selection check returned exit 3. Audience/replay v1 remain
  immutable invalid history.
- Mandatory product traces and instruction measurements: not yet complete.
- Mandatory trace admission: `BLOCKED` and `INVALID`; all ten traces lack the
  frozen trace-method identity required by the accepted design.
- Control-plane publication supporting execution: conflict rejection, direct
  root/store authorization, and completed post-upgrade binding recovery pass.
  Runtime durable-publish permit enforcement fails; the manifest/guard tests
  prove labels and other cost classes only.
- Cross-stage security ordering: `PASS`; focused prepare/lazy-repair,
  replay-abort, recovery-required preservation, and prior auth evidence agree
  with the source ordering. This result does not substitute for missing
  negative integration proof in the separate boundary/equivalence findings.
- Lifecycle symmetry: `PASS`; 2 structural lifecycle boundary tests, 1 trap
  boundary test, and 3 PocketIC install/upgrade/failure tests pass. Root and
  non-root before-bootstrap adapters remain synchronous and all four
  bootstrap helpers retain zero-delay timer boundaries.
- Capability surface v2: `PASS`, valid, risk 4/10. All six retained DIDs
  rebuilt, protocol guards pass 19 tests, and targeted protocol-surface Clippy
  passes. Source endpoints and core constants contract `71 -> 68` and
  `56 -> 53`; three controller-only introspection methods raise global GAF to
  6. Frozen v1 remains invalid history.
- Publish surface: `PASS`, valid, risk 4/10. Cargo metadata, six workspace
  manifest tests, the release package/install definition guard, and isolated
  offline packaging of all eight public crates pass. Public feature guidance,
  `canic-core` architecture wording, and hard-cut proof residue remain open.
- Module structure: `FAIL`, valid, risk 7/10. Isolated rustdoc and direct source
  maps find 25 production ops-to-policy imports, while module layout,
  circularity, public record containment, and fleet/test/audit seams pass.
  Five focused DTO/policy boundary tests pass; warning-as-error core rustdoc
  fails on one unresolved internal link.
- DRY consolidation: `FAIL`, valid, risk 6/10. Registry/query, response,
  output, evidence, backup, and release-proof owners remain distinct. The
  issuer-policy validator duplicates policy authority and lacks direct ops
  rejection proof. Focused tests pass 8 core policy, 19 host registry, 2 host
  response, 30 CLI output-filtered, and 19 backup persistence cases.
- Complexity v2: `FAIL`, valid, risk 8/10. The exact runner reproduces 546
  files, 74,203 logical LOC, 14 non-test files at or above 600 LOC, and five
  strict hubs; every file maps. Five focused filters pass 178 selected
  executions. V1 remains invalid history.
- Change friction v2: `FAIL`, valid, risk 8/10. All 546 current files map; two
  executions reproduce digest `aac8db07...`; five exact core slices average
  8.6 files with nearest-rank p95 19, and the intent slice has CAF 48. Focused
  RPC/capability/delegation execution passes 74 tests. V1 remains invalid
  history.
- Instruction footprint: `BLOCKED`, invalid. The scenario generator retains 11
  identities and PocketIC 14.0.0 starts, but root-probe direct Cargo Wasm
  compilation is rejected by the authoritative build boundary. Zero perf rows
  or checkpoint deltas exist; the exact coverage scan also misses all 57
  namespaced product checkpoints.
- Wasm footprint: `BLOCKED`, invalid. A clean linked-worktree run passes Cargo,
  ICP, `ic-wasm`, and `twiggy` prerequisites, then the first `app` direct Cargo
  Wasm build fails at the authoritative build guard. No byte, shrink, debug,
  section, or retained-size metric exists.
- Module surface hardening: `FAIL`, valid, risk 4/10. All-feature core compile,
  three provisioning tests, and two facade protocol-surface tests pass. The
  public protocol pin confirms the internal batch request is serialized
  unnecessarily; direct error-root consumer scans find none.

## Follow-up Actions

1. Continue correcting instruction and Wasm
   methods against the immutable published product snapshot; product fixes
   remain prohibited.
2. Complete the existing auth generated/session and interrupted-publication
   evidence gaps, then rerun only those two mandatory traces.
3. Keep the durable-publication cost/error/recovery findings read-only until
   the corrected baseline and mandatory traces are complete.
4. Keep issuer-policy admission and its rejection-proof gap in the same
   eventual `CANIC-092-LAYERING-005` fix slice; do not add a wrapper or second
   validation path.
5. Review delegated-auth/root-proof complexity with the complete baseline;
    do not create a generic flow abstraction from file-size pressure alone.
6. Version instruction footprint with a root-independent composite,
    authoritative fixture builds, correct checkpoint scan, and complete flow
    roster; rerun `v0.92.0` before accepting performance claims.
7. Version Wasm footprint around authoritative Canic pre/post-transform
    artifacts and a root-independent composite; rerun `v0.92.0` without
    reintroducing a direct-build compatibility path.
8. Keep the proof-install DTO and direct error-root hard cuts separate and
    read-only until the complete corrected baseline admits product slices.
