# 0.92 Holistic Audit and Audit-System Validation - Status

Last updated: 2026-07-16

## Current State

Phases A and B are complete. The six P1 audit-system findings are fixed at the
published `v0.92.0` snapshot, and the improved method set is frozen.
Phase C is complete at the immutable product snapshot. All 22 retained
definitions have valid active results. Corrected instruction-footprint v2 is
valid/partial, and corrected Wasm-footprint v2 brings the retained-method
ledger to 22 valid and zero invalid results. Evidence-only PocketIC slices now
complete the auth and control mandatory traces: the aggregate is a valid
`fail` with six passing and four failing traces, zero partial, and zero
blocked. The product baseline gate and finding review are complete, and Phase D
is underway. D1 is released in `v0.92.1`, D2/D3 in `v0.92.2`, and D4 in
`v0.92.3`; D5 is released in `v0.92.4`; D6 in `v0.92.5`; and D7 in
`v0.92.6`. D8 is released in `v0.92.7`. Absolute build paths are removed from
root runtime records and
diagnostics, isolated root/bootstrap artifacts reproduce byte for byte, and
stable build provenance requires explicit transform identity and outcome.
`CANIC-092-BUILD-001` and `CANIC-092-BUILD-002` are fixed. D9 is released in
`v0.92.8`: 13 external Action executions are immutable,
downloaded executable tools are version/checksum bound, and one matrix owns
the release-supported host/target cell. `CANIC-092-RELEASE-001`, `-002`, and
`-004` are fixed; the dedicated scanner gap remained blocked after D9 and is
fixed by D12. D10 is
released in `v0.92.9`: published feature docs match their
manifests, active CLI proof contains no removed-command anti-resurrection
checks, public core rustdoc passes with warnings denied, and installed plus
packaged downstream proofs pass before registry publication. This fixes
`CANIC-092-PUBLISH-001`, `CANIC-092-RESIDUE-001`, and
`CANIC-092-DOCS-002`. D11 is released in `v0.92.10`: shared
decision inputs now belong to model, root-proof admission belongs to workflow,
and ops no longer imports policy. This fixes `CANIC-092-LAYERING-005`; the
layering guard passes with zero violations. D12 is released in `v0.92.11`:
Gitleaks 8.30.1 is version/checksum bound, scans complete history with full
redaction, and reports zero unreviewed findings after 11 false positives were
admitted only by exact fingerprints. This fixes `CANIC-092-RELEASE-003`
without a waiver. Slice E then found that the version-only 0.92.11 release
transaction re-resolved six unrelated external packages. D13 fixes
`CANIC-092-RELEASE-005` in the current candidate by synchronizing only
workspace lock entries offline. Three P2 findings remain unresolved with
explicit deferred dispositions. The current trace ledger is ten pass and zero
fail, and no P0 or P1 remains.

No runtime product code, public contract, stable state, or generated product
behavior changed during method hardening. The final 0.92.11 version transaction
did advance six semver-compatible transitive lockfile entries; current
locked/offline metadata and advisory checks pass, and D13 prevents later
version-only bumps from repeating that unrelated dependency resolution.

0.92 treats Canic as feature complete for this line but does not claim 1.0
readiness. At least three months of real-world use remains a separate
maintainer prerequisite for any future 1.0 discussion.

## Immutable Identities

- Compatibility/review anchor: `v0.91.6` at
  `5f7a89f9b966ebf2755d5630ddcba0cdf968ebb1`.
- Compatibility product-tree hash:
  `8fce43e41ce430d9b505e19f8d596ed440b291d4c6ecb19c4a1cfdf71656a9b6`.
- Frozen method and Phase C product snapshot: `v0.92.0` at
  `91736337fc1cfeb891f17d7d62affb5e671348e2`.
- Frozen source tree:
  `fd31bb8289365a38f2bea7f8ebd6973908ee959f`.
- Frozen product-tree hash:
  `c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e`.
- Prepared/frozen method content manifest:
  `fa92c4102efe74391c51f1f829aec7ac9c0b64941da73ee6dad1ebf2b292df07`.
- Phase A method fingerprint:
  `ab47f96a4ca388d0c61f01280e2a47bb37930b1ce863d675ea8427bf08b229e6`.
- Freeze admission method fingerprint:
  `8188a7e08d9551efbad79e56c20cdd2213ed54758fc07b0bd0120b61e0dba82b`.
- Latest published Phase D anchor and D13 parent: `v0.92.11` at
  `fdf3bd6f2a20e2fc2da50398771660eae59fca94`, source tree
  `6e67e6700c290501869242c9fbcf35bfff68ec57`, product-tree hash
  `3cc60e4a86373cae61b26bccf4cdb29fc0a07869ef8ab3db817563a379853ca5`.
- Workspace package version: `0.92.11`.

## Slices

### Slice A - Audit-system inventory

- [x] Record the release anchor, full tree identity, audit-tree identity,
      lockfile/toolchain hashes, dirty state, method identity, and script
      hashes.
- [x] Inventory recurring, authentication, modular, operational, executable,
      release-line, and retained-evidence candidates.
- [x] Map owners, triggers, method tags, executable sinks, comparability limits,
      overlap groups, holistic coverage, execution safety, and evidence
      integrity.
- [x] Publish the dated primary report and required summaries.

Primary evidence:
[0.92 audit-system inventory](../../audits/reports/2026-07/2026-07-14/0.92-audit-system-inventory.md).

### Slice B - Meta-audit and method hardening

- [x] Disposition every candidate and establish one canonical owner per
      invariant/topic.
- [x] Resolve findings `CANIC-092-AUDIT-001` through `-006`.
- [x] Expand meta-audit, run-state, comparison, safety, evidence, and retention
      governance.
- [x] Correct and test the definitions/runners without runtime product changes.
- [x] Freeze method/script/fixture fingerprints at `v0.92.0`.
- [x] Review the committed product-tree delta and admit the Phase C snapshot.

Primary evidence:

- [0.92 audit-system hardening](../../audits/reports/2026-07/2026-07-14/0.92-audit-system-hardening.md);
- [0.92 method freeze](../../audits/reports/2026-07/2026-07-14/0.92-method-freeze.md).

### Slice C - Holistic read-only baseline

- [x] Prove and review the method-preparation/release product-tree delta.
- [x] Run the complete retained improved suite against
      `91736337fc1cfeb891f17d7d62affb5e671348e2`.
- [x] Execute every mandatory versioned trace in its permitted mode.
- [x] Produce dated reports and required day/month summaries for every
      retained method attempt.
- [x] Deduplicate retained-method findings into the `CANIC-092-*` index.
- [x] Publish the frozen method/product admission report.
- [x] Run the first dependency and release-integrity batch; dependency v1 is
      invalid/blocked by a post-freeze method defect, while release integrity
      is a valid failure.
- [x] Correct dependency hygiene to `CANIC-DEPENDENCY-001/v2` with a
      deterministic external license-declaration inventory rule and rerun the
      immutable baseline. The valid result passes at risk 3/10 with zero known
      vulnerabilities and four unmaintained-package watchpoints.
- [x] Recompute the canonical product-tree identity from the published commit.
      `CANIC-092-AUDIT-017` corrects the carried-forward Phase B hash to the
      actual `v0.92.0` product hash without changing or invalidating product
      evidence.
- [x] Run the executable layering guards and representative responsibility
      traces. The original result found API/DTO ownership and authority
      conflicts but is now invalid because its grouped-import scan missed the
      ops-to-policy edge.
- [x] Correct layering to v2 with fingerprinted direct/grouped import fixtures
      and an executable ops-to-policy rule. The corrected guard reports 25
      production violations and the valid result fails at risk 7/10; v1
      remains invalid history.
- [x] Run build-integrity v1 code and execution traces; ordinary artifacts are
      reproducible, while root artifacts embed absolute build paths. The v1
      result is invalid under the post-freeze method-defect protocol because
      its raw provenance comparison contradicts the canonical timestamp rule.
- [x] Correct build integrity to v2 by excluding only observation timestamps
      and their derived digest from semantic provenance comparison, then rerun
      two isolated lanes. Ordinary app/bootstrap artifacts reproduce and the
      valid result retains the root absolute-path failure.
- [x] Run all seven authentication invariant traces and focused execution. No
      accepting bypass was found; five methods are valid, while audience and
      replay v1 are invalid because four frozen filters execute zero tests.
- [x] Correct audience and replay to v2 with current exact filters and a shared
      nonempty-test runner, then rerun the immutable baseline. Both corrected
      methods validly pass at risk 3/10; v1 remains invalid history.
- [x] Attempt mandatory trace admission; all ten are blocked because no
      versioned/fingerprinted trace method was cataloged or frozen. Supporting
      code traces remain evidence but do not satisfy the mandatory gate.
- [x] Correct the mandatory trace system to
      `CANIC-MANDATORY-TRACE-001/v1`, fingerprint it, and execute all ten
      trace IDs. Six pass, deploy/blob fail on existing findings, and
      auth/control are partial on existing evidence gaps. The aggregate
      `partial` result keeps the trace gate incomplete while fixing
      `CANIC-092-AUDIT-010`.
- [x] Trace control-plane/Wasm-store publication as supporting evidence.
      Authorization, conflict refusal, exact reuse, and completed upgrade
      reconciliation pass; declared durable-publish cost enforcement and typed
      failure projection fail, and interrupted convergence proof is absent.
- [x] Run frozen cross-stage security-boundary ordering. No inspected handler
      or mutation precedes its auth, proof, subject/scope, capability, or
      replay owner; the valid result passes with watchpoints.
- [x] Run frozen lifecycle symmetry. Root/non-root init and upgrade restore
      synchronously, failure traps before continuation, and all four bootstrap
      paths cross zero-delay timers; structural and PocketIC evidence passes.
- [x] Attempt frozen capability-surface v1. Hard placement and focused protocol
      checks pass, but the result is partial/invalid because its mandatory
      workspace Clippy command conflicts with canonical targeted-test policy.
- [x] Correct capability surface to v2 around its owning package test and
      targeted warning-as-error Clippy target, then rerun the immutable
      baseline. Six retained artifacts rebuild and the valid result passes at
      risk 4/10; v1 remains invalid history.
- [x] Run frozen publish-surface v1. All eight intended public packages verify
      from isolated offline archives and the valid result passes at risk 4/10;
      three P2 package-doc, architecture-doc, and hard-cut residue findings
      remain open.
- [x] Run frozen module-structure v1. The valid result fails at risk 7/10 after
      confirming 25 production ops-to-policy imports. It also proves the
      layering v1 detector/guard defect; cycle, public-record, test-seam, and
      module-layout checks pass.
- [x] Run frozen DRY-consolidation v1. The valid result fails at risk 6/10:
      operator-side owners remain bounded, while root-issuer policy admission
      duplicates the existing ops/policy authority defect and lacks direct
      rejection/unchanged-state proof.
- [x] Attempt frozen complexity-accretion v1. Immutable measurements and 97
      focused tests complete, but the result is partial/invalid: undefined CAF
      input, unfrozen search identities, overlapping score modifiers, and the
      unmapped role-contract scope block an authoritative risk baseline.
- [x] Correct complexity to `CANIC-COMPLEXITY-001/v2` with one fingerprinted
      mechanical runner, complete scope, exact manual evidence, informational
      CAF, and one score. Two executions reproduce exactly and 178 focused
      selections pass; the valid first baseline fails at risk 8/10 on the
      retained P2 hotspot, fixing `CANIC-092-AUDIT-013`.
- [x] Attempt frozen change-friction v1. Five exact slices and 74 focused tests
      complete, but the result is partial/invalid: the mandatory map leaves
      163 core files unclassified, API has no fixed CAF layer, and competing
      score plus unfrozen sample/counter rules block a velocity baseline.
- [x] Correct change friction to `CANIC-CHANGE-FRICTION-001/v2` with an
      exhaustive 23-subsystem map, frozen five-commit fixture, exact formulas,
      reused layering/complexity authorities, and one score. Two executions
      reproduce exactly and 74 focused tests pass; the valid first baseline
      fails at risk 8/10 and fixes `CANIC-092-AUDIT-014` without adding a
      product finding.
- [x] Attempt frozen instruction-footprint v1. Pinned PocketIC and an 11-entry
      manifest start, but the result is blocked/invalid before any perf row:
      the runner uses a forbidden direct Cargo Wasm fixture build, emits a
      root-dependent composite, and misses namespaced checkpoints/flow classes.
- [x] Correct instruction footprint to `CANIC-INSTRUCTION-001/v2` with the
      authoritative root harness, a fixed 12-scenario update/install roster,
      exact endpoint labels, root-independent composite identity, compatible
      predecessor discovery, and namespaced checkpoint scanning. The valid
      first v2 baseline is `partial` at risk 6/10: all required rows execute,
      while root-proof and delegated-token flows lack internal checkpoints.
- [x] Attempt frozen Wasm-footprint v1 in the required clean linked worktree.
      The result is blocked/invalid before any artifact metric: its first
      direct Cargo `app` build violates the authoritative `canic build` hard
      cut, and its executable composite is also root-dependent.
- [x] Correct Wasm footprint to `CANIC-WASM-001/v2` with one authoritative
      host-builder artifact path, fresh release/debug builds for the fixed
      six-role roster, a root-independent executable composite, exact
      compatible-predecessor keys, and compact hashed evidence. The valid first
      v2 baseline passes at risk 4/10 and fixes `CANIC-092-AUDIT-016` without
      adding a product finding.
- [x] Complete `TRACE-AUTH-001` and `TRACE-CONTROL-001` with evidence-only
      PocketIC cases. Generated auth now proves pre-session rejection,
      bootstrap success, guarded-call parity, replay idempotence/conflict, and
      unchanged authority after rejection. Publication now proves recovery
      from a committed target release before the root mirror. The mandatory
      aggregate becomes `fail` (6 pass, 4 fail, 0 partial, 0 blocked), fixing
      `CANIC-092-AUTH-001` and `CANIC-092-PUBLICATION-001` without changing
      production implementation.
- [x] Run manual-only `CANIC-MODULE-SURFACE-001/v2.0` against `canic-core`.
      The valid first run fails at risk 4/10, confirms the existing internal
      proof-install DTO finding, and adds one bounded public error-root hard
      cut. Generated/facade, replay, state, sibling-support, and test-only
      surfaces otherwise retain current owners.
- [x] Produce the read-only Phase C technical baseline review. Subsequent
      corrections bring the live ledger to 22 valid and zero
      invalid active results. After fixing `CANIC-092-AUDIT-015` and
      `CANIC-092-AUDIT-016` and recording `CANIC-092-PERF-001`, the unresolved
      index contains 25 findings. The verdict remains `blocked` on the partial
      mandatory traces; no product slice is accepted by that original review.
      The July 15 evidence completion supersedes the trace state and reduces
      the live unresolved index to 23.

### Slice D - Finding-backed hardening

- [x] Review severity, confidence, and recommended dispositions before
      mutation. Nineteen findings map to ten bounded candidate slices, three P2
      pressure findings remain deferred watchpoints, and the dedicated scanner
      P1 has a proposed but unaccepted limitation record.
- [ ] Implement only accepted bounded fix slices.
- [ ] Add targeted positive, rejection, boundary, and regression proof.
- [ ] Compare each slice causally to its parent and cumulatively to the frozen
      product baseline.
- [x] Implement and validate D1 publication safety and typed failures. Ten
      publications are admitted per window, the eleventh rejects before fleet
      mutation, conflict/capacity remain distinct, and interrupted recovery
      converges after upgrade. Store-GC behavior remains outside the slice.
- [x] Record D1 implementation `daa67913...`, validation `d9dc6304...`, and
      released `v0.92.1` source/product identities.
- [x] Implement and validate D2 auth typed-cause preservation. All seven auth
      methods and `TRACE-AUTH-001` pass; wrong-issuer, expired, and corrupted
      proofs reject without replacing active state.
- [x] Implement and validate D3 canonical layer documentation. Active public
      docs and module headers now mirror the strict `AGENTS.md` contract; the
      25 product-code ops-to-policy violations remain visible for later
      bounded slices.
- [x] Record D2/D3's shared fix and validation identity in released `v0.92.2`.
- [x] Implement and validate D4 root-issuer admission ownership. Workflow now
      owns admission orchestration, policy owns pure decisions, model owns
      state-shaped values, and ops owns conversion/persistence. Direct
      positive, rejection, unchanged-state, timer-order, stable-state, and
      PocketIC regression evidence passes.
- [x] Record D4's fix and validation identity in released `v0.92.3`.
- [x] Implement and validate D5 blob-billing workflow ownership. API delegates
      Cashier sequencing, reserve/recovery, gateway synchronization, and
      readiness to one workflow; pure policy owns deterministic decisions.
      Reserve, transient failure/retry, status, and upgrade PocketIC evidence
      passes without protocol or stable-state change.
- [x] Record D5's fix and validation identity in released `v0.92.4`.
- [x] Implement and validate D6 passive RPC DTO ownership. One workflow command
      owns request family, capability descriptors, replay identity, and
      admitted metadata; ops owns mechanical signed-payload projection. Exact
      identical replay and cross-family conflict PocketIC evidence passes.
- [x] Record D6's fix and validation identity in released `v0.92.5`.
- [x] Implement and validate D7 internal surface hard cuts. The duplicate
      public proof-install request/outcome and direct core error root are
      removed without aliases; the internal plan, model failure classification,
      and control-plane support bridge retain their canonical owners.
- [x] Record D7's fix and validation identity in released `v0.92.6`.
- [x] Implement and validate D8 reproducible root artifacts. Runtime bootstrap
      evidence contains no build paths, two isolated root/bootstrap lanes are
      byte-identical, semantic provenance matches, and optional transform
      identity/outcome is required and fail-closed.
- [x] Record D8's fix and validation identity in released `v0.92.7`.
- [x] Implement and validate D9 release execution integrity. External Actions
      use full commit identities, executable archives are exact and verified
      before extraction, package tools and IC prerequisite checks consume one
      repository pin authority, the guard discovers that maintained surface,
      deterministic local synchronization cannot mutate unrelated dependencies,
      and one guarded matrix owns the supported Ubuntu/Linux/Wasm cell.
- [x] Implement and validate D10 active documentation and hard-cut residue.
      Published feature docs derive complete/default expectations from their
      manifests, active renewal proof asserts only the maintained status
      command, warning-as-error core rustdoc passes, and installed plus
      packaged downstream proofs pass from isolated artifacts.
- [x] Implement and validate D11 canonical layering closure. Shared
      environment, funding, topology, placement, and metrics values belong to
      model; root delegation-proof admission belongs to workflow; and the
      executable guard reports zero ops-to-policy dependencies.
- [x] Record D11's fix and validation identity in released `v0.92.10`.
- [x] Implement and validate D12 dedicated secret scanning. Gitleaks 8.30.1
      and its platform archives are version/checksum bound, the complete
      history scan is fully redacted, reviewed false positives use only exact
      fingerprints, and CI plus patch-release gates invoke the canonical scan.
- [x] Record D12's fix and validation identity in released `v0.92.11`.
- [x] Implement and validate D13 workspace-only release lock synchronization.
      Version bumps update workspace package entries offline without
      re-resolving unrelated external packages; the existing release-surface
      rollback remains fail-closed.

### Slice E - Closeout

- [x] Resolve or explicitly disposition every finding. Three P2 watchpoints
      remain explicitly deferred with bounded revisit conditions; every other
      reviewed finding is fixed or otherwise closed.
- [x] Confirm no required run is partial, blocked, or unjustifiably not
      applicable. All 22 retained definitions have valid current results and
      all mandatory traces have complete current evidence.
- [x] Confirm no P0 or non-waivable P1 remains unresolved.
- [x] Execute the `v0.91.6` compatibility contract or document each accepted
      hard cut and migration boundary. Generated Candid, CLI/config,
      stable-state upgrade, backup/restore, package features, Rust source cuts,
      and the provenance regeneration boundary are accounted directly.
- [ ] Write `docs/audits/release-lines/0.92-closeout.md` with one explicit
      closeout verdict.

## Finding Index

| Finding | Class | Severity | Confidence | Status | Fix / validation |
| --- | --- | --- | --- | --- | --- |
| `CANIC-092-AUDIT-001` | audit method defect | P1 | confirmed | fixed | `cdcd1487...` / `91736337...` |
| `CANIC-092-AUDIT-002` | governance conflict | P1 | confirmed | fixed | `cdcd1487...` / `91736337...` |
| `CANIC-092-AUDIT-003` | audit method defect | P1 | confirmed | fixed | `cdcd1487...` / `91736337...` |
| `CANIC-092-AUDIT-004` | governance conflict | P1 | confirmed | fixed | `cdcd1487...` / `91736337...` |
| `CANIC-092-AUDIT-005` | evidence gap | P1 | confirmed | fixed | `cdcd1487...` / `91736337...` |
| `CANIC-092-AUDIT-006` | operational risk | P1 | confirmed | fixed | `cdcd1487...` / `91736337...` |
| `CANIC-092-AUDIT-007` | audit method defect | P1 | confirmed | fixed | Dependency v2 defines and passes a deterministic declaration-integrity rule; commit pending. |
| `CANIC-092-DEPENDENCY-001` | operational risk | P2 | confirmed | deferred | Four reachable transitive packages are unmaintained, but no known vulnerability or bounded safe replacement is established; reconsider on advisory, dependency update, or 1.0 preparation. |
| `CANIC-092-RELEASE-001` | operational risk | P1 | confirmed | fixed | D9 pins all 13 active external Action executions to reviewed full commits and guards future references. |
| `CANIC-092-RELEASE-002` | operational risk | P1 | confirmed | fixed | D9 versions package tools and checksum-verifies all downloaded executable archives before extraction/execution. |
| `CANIC-092-RELEASE-003` | evidence gap | P1 | confirmed | fixed | D12 installs pinned Gitleaks through a checksum-bound path and records a redacted full-history pass with zero unreviewed findings. |
| `CANIC-092-RELEASE-004` | evidence gap | P2 | confirmed | fixed | D9 establishes one guarded Ubuntu 24.04 x86_64/native/Wasm support matrix and classifies other installer branches as unvalidated. |
| `CANIC-092-RELEASE-005` | operational risk | P2 | confirmed | fixed | D13 replaces full lockfile regeneration with a workspace-only offline update; disposable proof changes only workspace package versions and locks zero external packages. |
| `CANIC-092-LAYERING-001` | product defect | P2 | confirmed | fixed | D5 moves blob Cashier, reserve/recovery, sync, and readiness orchestration behind one workflow; API delegates and public/stable shapes are unchanged. |
| `CANIC-092-LAYERING-002` | product defect | P2 | confirmed | fixed | D6 makes the DTO passive; one workflow command owns capability/replay identity and ops owns mechanical wire projection, with protocol and stable state unchanged. |
| `CANIC-092-LAYERING-003` | governance conflict | P1 | high | fixed | D3 makes `AGENTS.md` the sole active authority and removes wording that permits endpoint-to-ops delegation; released in `v0.92.2`. |
| `CANIC-092-AUDIT-008` | audit method defect | P1 | confirmed | fixed | Build v2 applies the exact semantic exclusion and preserves the root artifact drift; commit pending. |
| `CANIC-092-BUILD-001` | product defect | P1 | confirmed | fixed | D8 removes runtime build paths; two isolated root/bootstrap raw and gzip lanes plus semantic provenance reproduce exactly. |
| `CANIC-092-BUILD-002` | operational risk | P2 | confirmed | fixed | D8 requires role/kind/mode/tool/version/outcome transform evidence and rejects inconsistent provenance. |
| `CANIC-092-AUDIT-009` | audit method defect | P1 | confirmed | fixed | Audience/replay v2 use current exact filters through a zero-test-refusing runner; both immutable-baseline reruns pass; commit pending. |
| `CANIC-092-AUTH-001` | evidence gap | P1 | confirmed | fixed | Exact PocketIC evidence proves pre-session rejection, bootstrap, generated-guard parity, replay conflict/idempotence, and unchanged authority; commit pending. |
| `CANIC-092-ERROR-001` | product defect | P1 | confirmed | fixed | D2 preserves proof/provisioning causes through one existing public-code boundary; all seven auth methods and no-mutation PocketIC proof pass; released in `v0.92.2`. |
| `CANIC-092-LAYERING-004` | product defect | P2 | confirmed | fixed | D7 removes the duplicate public install request/outcome; workflow consumes the internal ops plan and typed issuer result directly. |
| `CANIC-092-AUDIT-010` | audit method defect | P1 | confirmed | fixed | Mandatory trace v1 is cataloged/fingerprinted and all ten IDs have admitted results; commit pending. |
| `CANIC-092-COST-001` | product defect | P1 | confirmed | fixed | D1 gives publication one workflow-owned quota/cycle permit and settlement path; fixed in `daa67913...`, validated in `d9dc6304...`, released in `v0.92.1`. |
| `CANIC-092-ERROR-002` | product defect | P1 | confirmed | fixed | D1 preserves publication conflict/capacity/missing/hash/state/store/transport causes; fixed in `daa67913...`, validated in `d9dc6304...`, released in `v0.92.1`. |
| `CANIC-092-PUBLICATION-001` | evidence gap | P1 | confirmed | fixed | Exact PocketIC evidence commits the target before the root mirror and proves post-upgrade convergence without allocation; commit pending. |
| `CANIC-092-AUDIT-011` | audit method defect | P1 | confirmed | fixed | Capability v2 uses its owning targeted test/Clippy contract; six artifact refreshes and the corrected baseline pass; commit pending. |
| `CANIC-092-PUBLISH-001` | documentation drift | P2 | confirmed | fixed | D10 documents every maintained facade/control-plane feature and exact default set, guarded by a manifest-derived package-doc test. |
| `CANIC-092-DOCS-001` | documentation drift | P2 | confirmed | fixed | D3 aligns public core docs with `endpoints -> workflow -> policy -> ops -> model` and model-owned state/storage invariants; released in `v0.92.2`. |
| `CANIC-092-RESIDUE-001` | governance conflict | P2 | confirmed | fixed | D10 deletes removed-command negative assertions and the obsolete active probe breadcrumb while retaining current positive CLI proof. |
| `CANIC-092-AUDIT-012` | audit method defect | P1 | confirmed | fixed | Layering v2 fixtures and guard detect all 25 production ops-to-policy files; corrected baseline validly fails; commit pending. |
| `CANIC-092-LAYERING-005` | product defect | P1 | confirmed | fixed | D4 removes seven root-issuer/model ownership violations; D11 moves the remaining shared values to model and root-proof admission to workflow, leaving zero ops-to-policy dependencies. |
| `CANIC-092-DOCS-002` | documentation drift | P3 | confirmed | fixed | D10 describes typed internal failure preservation without linking the crate-private type; warning-as-error core rustdoc passes. |
| `CANIC-092-TEST-001` | evidence gap | P2 | confirmed | fixed | D4 directly proves valid policy/template admission, every request rejection boundary, unchanged state/epoch, skipped timers, and renewal/provisioning regressions. |
| `CANIC-092-AUDIT-013` | audit method defect | P1 | confirmed | fixed | Complexity v2 has one fingerprinted runner, complete scope/manual evidence, and one reproducible score; commit pending. |
| `CANIC-092-COMPLEXITY-001` | operational risk | P2 | confirmed | deferred | Delegated-auth and chain-key trust paths remain concentrated, but no correctness defect or justified replacement abstraction is established; reconsider with operational evidence. |
| `CANIC-092-AUDIT-014` | audit method defect | P1 | confirmed | fixed | Change-friction v2 has exhaustive scope/layers, a frozen fixture, exact counters/formulas, and one reproducible score; commit pending. |
| `CANIC-092-AUDIT-015` | audit method defect | P1 | confirmed | fixed | Instruction v2 uses authoritative root-harness artifacts, a complete fixed roster, exact endpoint labels, root-independent identity, compatible-predecessor discovery, and namespaced checkpoint scanning; commit pending. |
| `CANIC-092-AUDIT-016` | audit method defect | P1 | confirmed | fixed | Wasm v2 uses only canonical host-builder release/debug artifacts, root-independent identity, exact comparison keys, and verified compact evidence; commit pending. |
| `CANIC-092-PERF-001` | evidence gap | P2 | confirmed | deferred | Root proof provisioning and issuer token paths lack internal checkpoints, but the baseline is valid and no regression comparison exists; reconsider with a compatible predecessor or measured incident. |
| `CANIC-092-SURFACE-001` | product defect | P2 | confirmed | fixed | D7 hard-cuts the direct core error root; the deliberate control-plane support bridge remains the sole sibling surface. |
| `CANIC-092-AUDIT-017` | audit method defect | P1 | confirmed | fixed | Published product-tree identity corrected from the Phase B tree to exact `v0.92.0`; commit pending. |

Finding detail and exact evidence live in the dated primary reports. The index
assigns identity by canonical owner/invariant rather than discovery order.

## Validation State

- Clean `HEAD`, tag, and `origin/main` identity at `v0.92.0`: pass.
- Frozen method-path equality from Phase B implementation to release: pass.
- Active method catalog, exact fingerprints, runner controls, and current
  operator guards: pass.
- Committed product-tree hash and exact path classification: pass.
- Release-only Cargo/lockfile/install-helper delta: reviewed. The 0.92.11
  version bump advanced six compatible transitive entries despite unchanged
  direct dependency declarations; D13 prevents future version-only bumps from
  repeating that unrelated resolution.
- Current locked/offline Cargo metadata and cached advisory scan: pass; 524
  packages, 484 external registry packages, zero Git packages, zero missing
  external license declarations, zero known vulnerabilities, and four
  unmaintained transitive packages.
- Dependency v1 remains invalid history. Corrected dependency v2 is a valid
  pass at risk 3/10: all 484 external packages identify license metadata, zero
  known vulnerabilities are present, and four unmaintained packages remain
  bounded watchpoints. No legal-family policy is claimed.
- Release integrity: D9's action identity, executable integrity, matrix,
  `actionlint`, permissions, and triggers pass. D12 adds the required pinned
  dedicated scanner; its fully redacted full-history run passes with zero
  unreviewed findings. The current affected-method rerun is pass without
  rewriting the frozen Phase C failure. D13 adds bounded workspace-only
  offline lock synchronization and a release-integrity regression guard.
- Layering v2: the immutable baseline remains a valid fail at risk 7/10 with
  25 production ops-to-policy files. D4's affected-scope rerun passes detector
  fixtures and reduces the live violation set to 18; D5 and D6 add no upward
  edge.
  The canonical `CANIC-092-LAYERING-005` finding remains open. V1 remains
  invalid history.
- Build integrity v1 remains invalid history. Corrected build integrity v2 is
  a valid fail: two isolated lanes reproduce app/bootstrap-store raw and gzip
  bytes and app semantic provenance after excluding only observation time and
  its derived digest. Root raw/gzip bytes and semantic provenance differ
  because generated runtime records contain absolute build paths.
- Authentication invariants: focused fail-closed unit and PocketIC coverage
  passes and no bypass was found. The later evidence slice closes the
  generated-endpoint/session proof gap with positive, rejection, replay, and
  unchanged-authority execution. Corrected audience/replay v2 pass validly at
  risk 3/10 with current nonempty selections; their v1 attempts remain invalid
  history. D2 now preserves typed proof/provisioning causes and passes all
  seven current method selections. D7 subsequently fixes the separate internal
  install DTO ownership finding.
- Mandatory trace admission remains invalid history. Corrected
  `CANIC-MANDATORY-TRACE-001/v1` is cataloged and fingerprinted; all ten trace
  IDs are complete. Six pass; deploy/auth/control/blob fail on existing
  product findings. The valid aggregate `fail` has no partial or blocked trace
  and completes the gate. Current D1/D2/D5 reruns fix control/auth/blob without
  rewriting that frozen result.
- Control-plane publication: D1 focused validation passes. Admin, bootstrap,
  and reconciliation publication reserve distinct workflow-owned quota/cycle
  permits before effects; ten same-window admin publications pass and the
  eleventh rejects before mutation. Conflict, capacity, missing release, hash,
  invalid-state, missing-store, and transport causes retain typed public codes.
  Authorization and interrupted target/root recovery continue to pass.
- Security boundary ordering: valid pass with watchpoints under
  `CANIC-AUTH-ORDERING-001/v1`; source and focused execution preserve auth,
  proof, subject/scope, capability, replay, and recovery-required sequencing.
- Bootstrap lifecycle symmetry: valid pass with watchpoints under
  `CANIC-LIFECYCLE-001/v1`; 2 structural guards, 1 trap guard, and 3 PocketIC
  lifecycle cases pass. The result remains only partially comparable to the
  prior unversioned June report.
- Capability surface v2: valid pass at risk 4/10. Six retained artifacts
  rebuild, 19 protocol tests pass, and targeted Clippy passes. Source
  endpoints/core constants contract by three while global controller-only
  introspection adds three retained methods with GAF 6. V1 remains invalid
  history.
- Publish surface v1: valid pass and first frozen-method baseline, risk 4/10.
  D10's current-tree rerun passes seven manifest-policy tests, installed and
  packaged CLI proof, and generated plus canonical packaged Wasm-store proof.
  Published facade/control-plane feature docs match their manifests and active
  proof contains only maintained positive command assertions.
- Module structure v1: valid fail and first frozen-method baseline, risk 7/10.
  Direct source inspection finds 25 production ops-to-policy imports, direct
  policy calls in ops, and policy-owned values round-tripped through stable
  mappers. D10 fixes the separate P3 public-doc link and warning-as-error core
  rustdoc now passes; the immutable structural failure remains historical.
- DRY consolidation v1: valid fail and immutable first baseline, risk 6/10.
  D4 removes the duplicate root-issuer admission owner and supplies the direct
  rejection/unchanged-state proof, fixing `CANIC-092-TEST-001`; unrelated
  ownership areas are unchanged.
- D5 blob-billing validation passes 878 all-feature core library tests, 50
  focused blob-storage tests, four policy/DTO guards, 19 protocol tests,
  strict core Clippy, and four PocketIC billing selections. Reserve refusal,
  transient failure/guard release/retry, readiness boundaries, and configured
  or missing-config upgrade persistence retain parent behavior.
- D6 passive-RPC validation passes 51 workflow RPC tests, eight RPC ops tests,
  four replay-manifest tests, four policy/DTO guards, 19 protocol tests, strict
  core Clippy, and exact PocketIC identical-replay and cross-family-conflict
  cases. The current capability trace remains pass.
- D7 internal-surface validation passes core/control-plane checks and strict
  Clippy, four provisioning tests, 20 chain-key batch tests, three
  DTO/serialization guards, 19 protocol tests, all-feature facade check,
  offline core/facade package verification, the control-plane facade test, and
  exact PocketIC new-issuer provisioning. The current trace ledger remains
  nine pass and one fail.
  D10's later current-tree rerun fixes the separately indexed rustdoc link.
- D8 reproducible-root validation passes seven transform tests, 15 provenance
  and policy tests, 12 release-set manifest tests, targeted checks and strict
  Clippy, and two fresh isolated offline root builds. Root/bootstrap raw and
  gzip artifacts plus semantic provenance match, no lane path reaches final
  Wasm, and the current deploy trace passes.
- D10 active-documentation validation passes seven manifest-derived package
  tests, positive CLI unit proof, warning-as-error core rustdoc, strict
  targeted Clippy, installed CLI proof, packaged CLI proof, and generated plus
  canonical packaged Wasm-store builds. Shell syntax, ShellCheck, targeted
  formatting, and diff hygiene pass.
- Complexity v1 remains invalid history. Corrected v2 is a valid first
  baseline failure at risk 8/10. It maps all 546 files, reproduces its runner
  digest, retains exact manual evidence, applies one score, and passes 178
  focused selections.
- Change friction v1 remains invalid history. Corrected v2 is a valid first
  baseline failure at risk 8/10. It maps all 546 current files, freezes five
  exact slices, reproduces normalized output twice, applies one score, and
  passes 74 focused tests without creating a new product finding.
- Instruction footprint v1 remains blocked/invalid history. Corrected v2 is a
  valid first baseline with aggregate `partial`, risk 6/10. All 12 isolated
  update/install scenarios execute through authoritative root-harness
  artifacts, producing 12 normalized rows and 21 checkpoint deltas; 57 static
  checkpoints are found. Root-proof and delegated-token flows still lack
  internal checkpoints (`CANIC-092-PERF-001`).
- Wasm footprint v1 remains blocked/invalid history. Corrected v2 completes 12
  authoritative role/profile builds in a clean linked worktree. Builder gzip,
  `ic-wasm`, `twiggy`, source-mutation, method identity, and retained-hash
  checks pass. The valid first v2 baseline passes at risk 4/10; leaf spread is
  1.0526x and root is 1.6227x the largest leaf.
- Module Surface Hardening v2.0: valid fail, first frozen-method baseline,
  risk 4/10. Targeted all-feature core check and five focused proof-surface
  tests pass. D7's current-tree rerun fixes both surface findings without
  rewriting the frozen result.
- Improved holistic suite: all 22 retained definitions have valid active
  results. Mandatory traces are complete with a frozen aggregate `fail`: six
  pass and four fail. Current D1/D2/D5/D6/D7/D8 affected-trace reruns pass, so
  current trace state is ten pass and zero fail without rewriting the Phase C
  baseline.
- Phase C technical baseline: complete. The original review remains preserved
  as the pre-correction blocked synthesis; the live index contains 23
  unresolved findings (9 P1, 13 P2, one P3) at that snapshot.
- Product fix slices: D1 is released in `v0.92.1`, D2/D3 in `v0.92.2`, D4 in
  `v0.92.3`, D5 in `v0.92.4`, D6 in `v0.92.5`, D7 in `v0.92.6`, and D8 in
  `v0.92.7`, D9 in `v0.92.8`, D10 in `v0.92.9`, and D11 in `v0.92.10`.
  D12 is released in `v0.92.11`; D13 fixes the newly observed release-lock
  expansion in the current candidate. The live unresolved index is 3 (0 P1
  and 3 P2), all explicitly deferred.
- Broad workspace, deployment, publish, and release gates: not run as Phase C
  audit evidence unless a frozen method specifically requires them.

## Next Action

D12 dedicated secret scanning is released in `v0.92.11`. Slice E compatibility
accounting found and D13 fixes one bounded release-lock defect in the current
candidate. The executable `v0.91.6` contract and all three deferred P2
dispositions are now complete. The next step is to assign D13 and the
compatibility report an immutable release identity, then publish one explicit
release-line verdict. Broad release validation remains maintainer-owned.

Primary review evidence:
[0.92 Phase C baseline review](../../audits/reports/2026-07/2026-07-14/0.92-phase-c-baseline-review.md).

Correction evidence:
[dependency hygiene v2](../../audits/reports/2026-07/2026-07-14/0.92-dependency-hygiene-v2.md)
and
[product baseline identity correction](../../audits/reports/2026-07/2026-07-14/0.92-product-baseline-identity-correction.md),
plus
[build integrity v2](../../audits/reports/2026-07/2026-07-14/0.92-build-integrity-v2.md)
and
[instruction footprint v2](../../audits/reports/2026-07/2026-07-15/instruction-footprint-v2.md),
plus
[Wasm footprint v2](../../audits/reports/2026-07/2026-07-15/wasm-footprint-v2.md)
and
[mandatory trace evidence completion](../../audits/reports/2026-07/2026-07-15/0.92-mandatory-trace-evidence-completion.md).

Phase D review evidence:
[finding review](../../audits/reports/2026-07/2026-07-15/0.92-phase-d-finding-review.md).

D1 implementation evidence:
[publication safety and typed failure contract](../../audits/reports/2026-07/2026-07-15/0.92-d1-publication-safety.md).

D2 implementation evidence:
[auth typed-cause preservation](../../audits/reports/2026-07/2026-07-15/0.92-d2-auth-typed-causes.md).

D3 implementation evidence:
[canonical layer contract](../../audits/reports/2026-07/2026-07-15/0.92-d3-canonical-layer-contract.md).

D4 implementation evidence:
[root-issuer admission ownership](../../audits/reports/2026-07/2026-07-15/0.92-d4-root-issuer-admission-ownership.md).

D5 implementation evidence:
[blob-billing workflow ownership](../../audits/reports/2026-07/2026-07-15/0.92-d5-blob-billing-workflow-ownership.md).

D6 implementation evidence:
[passive RPC DTO ownership](../../audits/reports/2026-07/2026-07-15/0.92-d6-passive-rpc-dto-ownership.md).

D7 implementation evidence:
[internal surface hard cuts](../../audits/reports/2026-07/2026-07-15/0.92-d7-internal-surface-hard-cuts.md).

D8 implementation evidence:
[reproducible root artifacts](../../audits/reports/2026-07/2026-07-15/0.92-d8-reproducible-root-artifacts.md).

D9 implementation evidence:
[release execution integrity](../../audits/reports/2026-07/2026-07-15/0.92-d9-release-execution-integrity.md).

D10 implementation evidence:
[active documentation and hard-cut residue](../../audits/reports/2026-07/2026-07-15/0.92-d10-active-documentation-and-hard-cut-residue.md).

D11 implementation evidence:
[canonical layering closure](../../audits/reports/2026-07/2026-07-15/0.92-d11-canonical-layering-closure.md).

D12 implementation evidence:
[dedicated secret scan](../../audits/reports/2026-07/2026-07-16/0.92-d12-dedicated-secret-scan.md).

D13 implementation evidence:
[workspace-only release lock synchronization](../../audits/reports/2026-07/2026-07-16/0.92-d13-workspace-lock-sync.md).

Slice E compatibility evidence:
[`v0.91.6` compatibility accounting](../../audits/reports/2026-07/2026-07-16/0.92-v0916-compatibility-accounting.md).
