# July 2026 Audit Summary

## Included Run Days

- [2026-07-01](2026-07-01/summary.md): recurring Wasm footprint baseline and
  reruns; low attributable drift risk.
- [2026-07-11](2026-07-11/summary.md): broad codebase health audit; one high
  recovery finding and two medium ownership findings.
- [2026-07-12](2026-07-12/codebase-health.md): post-0.86 flow audit; prior
  safety findings are closed, scaffold atomicity is corrected, and ICP error
  convergence is now implemented.
- [2026-07-13](2026-07-13/environment-variables.md): product environment-input
  audit; public profile, path/config, cache-retention, target-directory, and
  Candid-refresh shortcuts are hard-cut, three unread child-build values are
  deleted, and required Cargo/build-script handoff values remain private.
- [2026-07-13 codebase health](2026-07-13/codebase-health.md): post-0.87 audit;
  one narrow 0.87 typed-ICP closeout correction and three bounded 0.88
  candidates covering backup durability, CLI file output, and fleet-config
  errors.
- [2026-07-14](2026-07-14/summary.md): 0.92 audit-system inventory and
  hardening, immutable `v0.92.0` method/product admission, and the first Phase C
  results. Six original P1 findings are fixed; dependency v1 is invalid history
  and corrected dependency v2 validly passes declaration/advisory/graph checks
  at risk 3/10, while release integrity fails immutable
  action/tool identity plus required evidence controls. The initial layering
  run remains invalid history because its grouped-import scan missed 25
  production ops-to-policy edges. Corrected v2 detector fixtures pass and the
  immutable baseline validly fails at risk 7/10 while retaining the API/DTO
  authority findings. Build v1 remains invalid history. Corrected v2 confirms
  ordinary artifact and semantic-provenance reproducibility but validly fails
  root reproducibility because absolute builder paths are shipped and remain
  visible as normalized artifact-hash drift. Authentication
  acceptance remains fail-closed in focused unit/PocketIC evidence. The
  original combined v1 report remains invalid history, while corrected
  audience/replay v2 methods validly pass at risk 3/10 and refuse zero-test
  success. Required negative integration proof remains incomplete, typed
  causes are lost, and internal install state leaks into the public DTO
  surface. Mandatory trace admission is also blocked: the design
  requires trace method identity/version/fingerprint fields, but no such
  method was cataloged or frozen. A supporting control-plane publication trace
  also confirms that durable-publish quota/reserve labels are not enforced by
  a runtime cost permit, publication causes collapse to generic internal
  errors, and interrupted convergence lacks executable proof. The independently
  frozen security-ordering method passes across auth/proof/capability/replay
  sequencing, and lifecycle symmetry passes source guards plus PocketIC
  install/upgrade/failure execution; neither pass clears the blocked mandatory
  trace gate or the owner-specific findings. Capability v1 remains invalid
  history because its workspace Clippy requirement conflicts with canonical
  targeted-test authority. Corrected v2 validly passes at risk 4/10 after all
  six retained artifacts, 19 protocol tests, and targeted Clippy pass; its
  global introspection growth is attributable. Publish-surface v1
  validly passes after all eight public packages verify from isolated offline
  archives, with P2 follow-up for incomplete feature docs, conflicting
  `canic-core` layer wording, and old-command anti-resurrection proof residue.
  Module-structure v1 validly fails at risk 7/10 on the upward policy edge; it
  finds no cycle, public record leak, test seam breach, or layout escape.
  DRY-consolidation v1 also validly fails at risk 6/10: shared operator and
  evidence owners remain coherent, but issuer-policy admission duplicates the
  existing ops/policy authority finding and lacks direct rejection proof.
  Complexity-accretion v1 remains invalid history. Corrected v2 maps all 546
  files, reproduces its normalized runner digest, and applies one score. It is
  a valid first baseline failure at risk 8/10; 178 focused test selections
  pass. Delegated-auth/root-proof concentration remains a P2 operational risk,
  not a demonstrated correctness defect.
  Change-friction v1 remains partial/invalid history. Corrected v2 maps all
  546 current files, freezes five exact slices, reproduces normalized output,
  and validly fails at risk 8/10 with 74 focused tests passing. It fixes the
  method defect without creating a separate product finding.
  Instruction-footprint v1 is blocked/invalid before measurement: pinned
  PocketIC starts, but the runner's direct Cargo Wasm fixture path is rejected
  by the authoritative build boundary. Its executable composite is
  root-dependent, the exact checkpoint scan misses 57 namespaced calls, and
  four required flow classes are absent from the retained 11-scenario roster.
  Wasm-footprint v1 is blocked/invalid for the same hard-cut boundary at its
  first `app` build: its required direct Cargo artifact no longer exists as a
  supported path. Its linked-worktree composite is also root-dependent; no
  raw/shrunk/debug or retained-size metric is available. The final retained
  definition, manual-only Module Surface Hardening v2.0, validly fails at risk
  4/10: it confirms the internal proof-install DTO finding and adds one P2
  unnecessary public `canic_core::error` root path. All 22 retained definitions
  have now been attempted. Two invalid results and the partial mandatory
  trace result still prevent a complete product baseline. Mandatory trace v1
  is now cataloged/fingerprinted and all ten IDs have admitted results: six
  pass, two fail on existing findings, and two are partial on existing evidence
  gaps. The Phase C ledger records 20 valid and 2 invalid retained results
  plus 26 unresolved findings. It also corrects the carried-forward Phase B product hash to the
  exact published `v0.92.0` identity. The verdict remains `blocked` and it
  authorizes no product mutation.
- [2026-07-15](2026-07-15/summary.md): instruction-footprint v2 replaces the
  invalid v1 method with authoritative root-harness artifacts, a fixed
  12-scenario update/install roster, exact endpoint-label binding, namespaced
  checkpoint discovery, root-independent identity, and compatible-predecessor
  selection. The valid first baseline is `partial` at risk 6/10: all 12 rows
  execute, 21 checkpoint deltas and 57 static checkpoints are retained, while
  root-proof and delegated-token flows lack internal checkpoints. This fixes
  `CANIC-092-AUDIT-015` and records P2 `CANIC-092-PERF-001`. The live ledger is
  advanced further by Wasm-footprint v2, which removes the unsupported direct
  Cargo/pre-shrink path and measures fresh canonical release/debug artifacts
  for all six roles. The valid first Wasm baseline passes at risk 4/10; leaf
  spread is 1.0526x and root is 1.6227x the largest leaf. This fixes
  `CANIC-092-AUDIT-016`. Evidence-only PocketIC cases then complete the two
  partial mandatory traces: delegated-session bootstrap/replay and generated
  guard parity pass, and a target-committed/root-unmirrored Wasm release
  converges after root upgrade without allocation. The mandatory aggregate is
  now a complete `fail` (6 pass, 4 fail, 0 partial, 0 blocked), fixing
  `CANIC-092-AUTH-001` and `CANIC-092-PUBLICATION-001`. The live ledger is 22
  valid, zero invalid, and 23 unresolved findings (9 P1, 13 P2, one P3). The
  Phase D review maps 19 findings to ten bounded candidate slices, defers three
  P2 watchpoints, and leaves the dedicated scanner limitation proposed but
  unaccepted. D1 then gives durable publication one workflow-owned quota/cycle
  permit and typed failure contract. Focused unit, policy, Clippy, and PocketIC
  validation passes, including quota rejection before mutation, exact
  conflict/capacity codes, authorization, and interrupted recovery. This fixes
  `CANIC-092-COST-001` and `CANIC-092-ERROR-002`, committed and released in
  `v0.92.1`. D2 then preserves typed auth proof/provisioning causes through one
  public mapping boundary. All seven auth methods and the current auth trace
  pass, including invalid-proof rejection without active-state mutation. This
  fixes `CANIC-092-ERROR-001`; D2/D3 are released together in `v0.92.2`. D3
  then resolves the competing
  layer authority and public core architecture wording without runtime change:
  active docs now mirror the strict `AGENTS.md` direction and model/storage
  ownership contract. The same 25 product-code violations remain visible. D3
  fixes `CANIC-092-LAYERING-003` and `CANIC-092-DOCS-001`. D4 then gives
  root-issuer policy/template admission one workflow/policy owner, moves
  persisted state shapes to model, and leaves ops with conversion/persistence.
  Direct rejection and unchanged-state proof fixes `CANIC-092-TEST-001`; the
  live layering guard drops from 25 to 18 while `CANIC-092-LAYERING-005`
  remains open. D4 is released in `v0.92.3`. D5 then moves blob-billing
  Cashier sequencing, reserve/recovery, gateway sync, and readiness out of API
  into one workflow over pure policy and single-step ops. Focused core and
  PocketIC reserve, transient-failure/retry, status, and upgrade proof passes
  without protocol, price, public-shape, or stable-state change. This fixes
  `CANIC-092-LAYERING-001`; the current blob trace passes. D5 is released in
  `v0.92.4`. D6 then makes root RPC DTOs passive, gives one workflow command
  capability-family and replay-identity authority, and leaves mechanical
  signed-payload projection in ops. Focused unit, protocol, Clippy, and exact
  PocketIC identical/conflicting replay proof passes without protocol or
  stable-state change. This fixes `CANIC-092-LAYERING-002` and is released in
  `v0.92.5`. D7 then hard-cuts the duplicate public proof-install
  request/outcome and direct core error root. The internal ops plan, private
  workflow, model failure classification, and deliberate control-plane support
  bridge retain one owner; issuer Candid, typed causes, stored diagnostics, and
  stable state are unchanged. Focused core/control-plane, protocol, package,
  Clippy, and PocketIC provisioning checks pass. This fixes
  `CANIC-092-LAYERING-004` and `CANIC-092-SURFACE-001`; D7 is released in
  `v0.92.6`. D8 then removes absolute build paths from root runtime evidence
  and requires explicit transform identity/outcome in stable provenance. Two
  isolated offline root/bootstrap lanes reproduce raw, gzip, and semantic
  provenance exactly, fixing `CANIC-092-BUILD-001` and
  `CANIC-092-BUILD-002`. D8 is released in `v0.92.7`. D9 then pins all 13
  external Action executions, versions/checksum-binds executable tools,
  removes the npm `ic-wasm` wrapper and remote ICP installer flow, and
  establishes one guarded Ubuntu/Linux/Wasm support matrix. This fixes
  `CANIC-092-RELEASE-001`, `-002`, and `-004`. D10 then aligns published
  feature docs with their manifests, deletes active removed-command negative
  assertions and the obsolete probe breadcrumb, repairs public core rustdoc,
  and makes pre-publish packaged CLI/Wasm-store proof resolve only extracted
  local 0.92.8 packages. This fixes `CANIC-092-PUBLISH-001`,
  `CANIC-092-RESIDUE-001`, and `CANIC-092-DOCS-002`. Five findings remain
  (2 P1 and 3 P2) at the D10 boundary. D11 then moves the remaining shared
  decision values to model, gives root-proof admission to workflow, and
  removes all 18 remaining ops-to-policy dependencies. This fixes
  `CANIC-092-LAYERING-005`; D11 is released in `v0.92.10`.
- [2026-07-16](2026-07-16/summary.md): D12 adds Gitleaks 8.30.1 through the
  repository's version/checksum-bound tool authority and runs a fully redacted
  full-history scan in CI and the patch-release gate. Eleven initial generic
  candidates are confirmed false positives and admitted only by exact
  historical fingerprints; no path or rule is broadly excluded. The rerun
  reports zero findings and fixes `CANIC-092-RELEASE-003` without a waiver.
  Three explicitly deferred P2 watchpoints remain; no P0 or P1 remains.

## Month Status

Partial. The corrected method set and product snapshot are frozen at
`v0.92.0`; Phase C is complete and Phase D is underway. Locked/offline
dependency resolution, the
external declaration-integrity inventory, and the cached advisory scan pass
with zero known vulnerabilities. Dependency v2 supersedes the invalid v1
attempt without introducing legal-family policy. Release integrity
has a valid failed result; layering v2 is now a valid immutable-baseline
failure at risk 7/10, while v1 remains invalid history. Build integrity v2 is
a valid failure: app and bootstrap artifacts plus app semantic provenance
reproduce, while final root raw/gzip and semantic artifact hashes remain
path-dependent. All seven authentication results are valid: audience and
replay v2 pass with current nonempty filters, while their v1 attempts remain
invalid history. No authentication bypass was found; current D2 auth reruns
also pass with typed causes preserved. Mandatory trace v1 is
cataloged, fingerprinted, and complete for all ten IDs: six pass and four fail
on existing product findings. The auth and publication evidence gaps are
fixed, so no trace remains partial or blocked and the Phase C gate is
satisfied. Phase D finding review is complete; D1 through D11 are released
through `v0.92.10`, and D12 passes focused validation. Published package docs
and active proof match the maintained surface, warning-as-error core rustdoc
passes, packaged CLI plus both generated and canonical Wasm-store proofs pass
before registry publication, the layering guard reports zero violations, and
the dedicated redacted secret scan reports zero findings. Three explicitly
deferred P2 watchpoints remain; no P0 or P1 remains. The current trace ledger
is ten pass and zero fail without rewriting the frozen result. Security
ordering and lifecycle
symmetry now have valid frozen-method
passes with watchpoints. Capability v2 is a valid immutable-baseline pass at
risk 4/10; v1 remains invalid history. Publish surface has a valid first
frozen-method baseline at risk 4/10.
Module structure has a valid first frozen-method failure at risk 7/10 and
confirms the ops/policy product finding independently of invalid layering v1.
DRY consolidation has a valid first frozen-method failure at risk 6/10 and
adds `CANIC-092-TEST-001` without duplicating the canonical layering finding.
Complexity v2 is valid and establishes the first comparable baseline at risk
8/10. Its exact scope/counters/manual evidence/score reproduce; the P2
trust-path complexity hotspot remains open. V1 is invalid history.
Change friction v2 is valid and establishes the first comparable baseline at
risk 8/10. Its exhaustive scope/layer map, frozen sample, exact formulas, and
single score reproduce; v1 remains invalid history.
Instruction v1 remains blocked/invalid history. Corrected instruction v2 is a
valid first baseline with aggregate `partial`, risk 6/10, 12 normalized rows,
21 measured checkpoint deltas, and 57 static checkpoint sites. Its auth-flow
checkpoint limitation is explicit and no regression is inferred without a
compatible predecessor.
Wasm footprint v1 remains blocked/invalid history. Corrected v2 uses only the
authoritative host builder, captures all six release/debug role pairs, and is
a valid first baseline pass at risk 4/10. No optimization or product defect is
inferred without a compatible predecessor.
Module Surface Hardening is a valid first frozen-method failure at risk 4/10.
Its generated, replay, state, sibling-support, and test-only surfaces retain
current owners; D7 fixes its proof DTO and direct error-root findings. All 22
retained-method results are valid and the mandatory traces are complete; Phase
C is closeable at a failing product baseline.

## Carry-Forward Follow-up

1. Complete Slice E compatibility accounting and publish one explicit 0.92
   closeout verdict; broad final gates remain maintainer-owned.
2. Keep the dependency, complexity, and instruction-checkpoint P2 watchpoints
   deferred until each has finding-backed evidence for a bounded change.
3. Keep the D4 root-issuer authority singular; do not reintroduce an ops
   validator or policy-owned persisted state shape.
4. Review delegated-auth/root-proof concentration after the complete
   baseline; do not infer a generic abstraction from size alone.
5. Preserve Wasm v2's sole canonical host-builder authority and use only an
   exactly compatible v2 predecessor for future size deltas.
6. Preserve D7's single internal proof-install path and deliberate
   control-plane support bridge; do not restore public mirrors or aliases.
7. Carry the instruction auth-flow checkpoint gap as `CANIC-092-PERF-001`;
   do not add instrumentation until a finding-backed product slice is accepted.
8. Preserve D8's stable-only root runtime evidence and required transform
   provenance; do not add a legacy payload decoder or alternate build path.
