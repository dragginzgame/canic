# MODULAR AUDIT - Module Surface Hardening

## Method Contract

- Audit ID: `CANIC-MODULE-SURFACE-001`
- Method version: `2.0`
- Disposition: `manual_only`
- Owner: requested module-level reachability, exposure, deletion pressure, and
  runtime-shape review
- Kind/profile: `manual`
- Trace mode: `code_trace`; measurement gates may execute only in a disposable
  environment
- Cost/runtime: medium; 30-90 minutes per module
- Prerequisites: named module, source snapshot, reviewer, ripgrep/Cargo
  evidence, and measurement baseline when runtime shape may change
- False-positive boundary: syntactic reachability, generated consumers, tests,
  macros, and runtime registration are classified before deletion pressure
- Shared contract: [AUDIT-HOWTO.md](../AUDIT-HOWTO.md)

Use this audit for Canic modules whose exposed surface, compatibility residue,
generated-boundary support, or retained helper layers need current authority
review.

`canic-core` is the default first target. Facade, CDK, macros, control-plane,
wasm-store, host, CLI, backup, testkit, and canister crates are in scope only
where they own reachable runtime surface, generated-code wiring, deployment
truth, persisted formats, operator output, or code whose retention keeps
complexity alive.

Use `docs/audits/modular/module-cleanup-runner.md` for implementation slices
that should patch safe findings. This document is the policy and classification
framework; the cleanup runner is the shorter per-module workflow.

## Audit Name

Use **Module Surface Hardening**, not **Code Hygiene**.

Reason: `code hygiene` already means formatting, imports, docs, ordering, and
small readability standards in `docs/governance/code-hygiene/README.md`. This audit is
narrower and more consequential: remove dead or stale exposed surface, collapse
unnecessary complexity lanes, and verify that every retained in-scope code unit
has a current Canic authority reason without regressing hot-path or
wasm-sensitive runtime shape.

Historical name: **Module Code Hygiene / Redundancy**.

Recommended slug for new reports: `module-surface-hardening`.

Old reports using `code-hygiene` remain valid historical reports, but they are
non-comparable with `MSH-2.0` unless the authority, generated-boundary,
hot-path, and wasm regression gates are backfilled.

## Audit Tier

Use the lowest tier that can honestly classify the module.

| Tier | Use when | Output |
| ---- | ---- | ---- |
| `Tier 0` | Cold/warm internal modules with no facade, generated-boundary, storage, recovery, stable format, query-hot, or wasm-sensitive involvement. | Cleanup runner only. |
| `Tier 1` | Some public/internal exposure risk, test-only production surface, duplicate helpers, or stale compatibility, but no persisted/runtime authority change. | Compact MSH report. |
| `Tier 2` | Facade API, generated-boundary uncertainty, stable-memory/storage records, backup/recovery, lifecycle/install/upgrade, deployment-truth, authority reconciliation, query/encode hot paths, or wasm-sensitive changes with unclear runtime shape. | Full MSH report. |

Escalate tiers as soon as evidence shows the current tier cannot classify a
candidate without weakening authority, generated-boundary, persisted-state, or
runtime-shape proof.

## Purpose

Force retained code to justify its current role in Canic's runtime,
deployment, generated-code, and operator architecture. Then find code that can
be deleted, narrowed, inlined, or moved because that justification is missing,
obsolete, duplicated, or too broad.

This audit targets:

* dead public or hidden surface
* stale compatibility paths
* generated-code fallback lanes that should be hidden behind macro boundaries
* facade re-exports that expose implementation internals by convenience
* orphaned helpers, DTOs, cfg branches, diagnostics hooks, and test-only
  production surfaces
* deployment-truth, backup, install, or authority evidence code that preserves
  obsolete routes or old vocabulary
* abstractions whose vocabulary or indirection costs more than their invariant
* cleanup candidates whose simpler shape risks worse wasm size, instruction
  count, allocation behavior, dispatch, or monomorphization

This is NOT:

* a style audit
* a general DRY audit
* a correctness audit
* a general performance audit
* a line-count reduction contest
* a redesign proposal exercise
* a module-boundary audit unless ownership or exposure is the reason code cannot
  justify itself
* a request for line-by-line prose when adjacent implementation lines share one
  clear authority reason

The goal is simplification with authority and runtime shape intact: delete,
narrow, inline, or move code when that makes the current architecture smaller
and clearer. Do not reduce lines by removing useful invariants, diagnostics,
generated-boundary proofs, or intentionally optimized hot-path structure.

## Canic Authority Rules

Apply these before classifying any finding.

* Dependency direction is strict:
  `endpoints -> workflow -> policy/domain -> ops -> model/storage`.
* DTOs are passive boundary data. They do not own validation, normalization,
  uniqueness, ordering, replacement semantics, or storage invariants.
* Stable storage records, runtime state restored from them, and explicit
  config/build inputs own canister-runtime authority.
* `view/` owns internal read-only projections. Views do not become mutation
  authority.
* Conversions between DTOs, views, records, and external protocol shapes belong
  in `ops::*` or the owning boundary adapter, not in workflow orchestration.
* `export()` and `import()` are reserved for canonical `*Data` snapshots.
* `canic::start!` and `build!` must stay thin macro entry points.
* Generated and macro-expanded code may use narrow hidden boundaries such as
  `canic::__internal`, `canic::__build`, `canic_core::__reexports`, and
  `canic_core::api`. Normal public facade surface must not be widened only for
  generated code convenience.
* Auth is enforced at endpoints. Workflow and ops assume authenticated input.
* Capability proof, replay metadata, subject/caller, audience, subnet, epoch,
  parent, and controller bindings must remain explicit.
* Lifecycle adapters restore synchronously and schedule async work. They do not
  await. User hooks run after Canic invariants are restored.
* Deployment-truth plans, inventories, checks, reports, receipts, and dry-run
  evidence are passive evidence unless a specific install/apply workflow
  consumes them by contract. Dry-run authority evidence must not mutate
  controllers.
* Controller mutation, canister install/upgrade, wasm-store chunking, backup,
  and recovery paths are high-risk authority surfaces.
* Pre-`1.0.0` hard-cut applies to unsupported internal protocols by default. It
  does not by itself authorize deletion of stable-state, backup, exported
  snapshot, deployment-truth, or live operator data compatibility without an
  owner decision and migration/recovery proof.
* For wasm-related deletion decisions, raw non-gzipped `.wasm` bytes are the
  primary size signal; gzip is secondary context.

## Scope

Default in-scope roots:

* `crates/canic-core/src`
* `crates/canic/src`
* `crates/canic-macros/src`
* `crates/canic-control-plane/src`
* `crates/canic-wasm-store/src`
* `crates/canic-host/src`
* `crates/canic-cli/src`
* `crates/canic-backup/src`

Conditional roots:

* sibling `../ic-testkit/src` and `crates/canic-testing-internal/src` when test
  support widens production visibility or acts as the real owner of a helper.
* `crates/canic-tests`, `canisters/test`, `canisters/audit`, and `fleets/*`
  when they are direct consumers of production surface or generated-boundary
  evidence.
* `docs/` when current design or governance files define the owner decision.

Default exclusions:

* historical docs and changelogs
* generated build output such as `.icp/`
* target artifacts
* tests, except when they are the only consumer of production surface
* examples, except when they explain why facade surface must remain stable

Test-only code is not dead just because production code does not call it. Mark
it separately as `test-only retained`, `test-only stale`, or `test-only masking
production dead surface`.

## Method Contract

Include this manifest in each report:

* `method_version = MSH-2.0`
* `surface_taxonomy = ST-1`
* `authority_taxonomy = AT-1`
* `deletion_confidence_model = DC-1`
* `compatibility_policy = pre-1.0-hard-cut`
* `wasm_signal_rule = raw-wasm-primary`
* `hot_path_risk_model = HP-1`
* `proof_policy = read-only-first`

Mark the run `non-comparable` if any manifest item changes, if in-scope roots
change, or if test/generated-code inclusion rules change.

`MSH-2.0` supersedes the retired modular code-hygiene audit. It keeps the
deletion-pressure standard and adds two release-quality gates:

* cleanup in hot or wasm-sensitive code must include an optimization-risk
  classification before a patch is recommended.
* audits are read-only by default; they produce findings first. Code changes
  require an explicit implementation request or an already-approved cleanup
  slice.

## Evidence Classes

Column classes:

* `[M]` Mechanical: direct code, compiler, or generated artifact signal.
* `[C]` Classified: analyst judgment over inspected code.
* `[D]` Derived: formula over mechanical fields.

Evidence modes:

* `mechanical`: generated from commands or compiler output.
* `semi-mechanical`: mechanical seed plus inspected context.
* `classified`: judgment from code reading.

Mention counts are weak signals. Do not classify a surface as dead from `rg`
counts alone.

## Retention Standard

Every retained in-scope code unit must have a current Canic authority reason.
The audit question is:

> If this code unit disappeared or became narrower, what current Canic
> authority would fail?

Then ask the deletion-pressure follow-up:

> Is that failure desirable because the caller is stale, test-only,
> generated-only, overexposed, or preserving old vocabulary?

Then ask the MSH-2.0 runtime-shape follow-up:

> Would the simpler shape add allocation, cloning, formatting, dynamic dispatch,
> generic monomorphization, or wasm size/instruction risk in a hot path?

A code unit can be a module, function, type, trait impl, enum variant, match
arm, DTO field, re-export, cfg branch, diagnostics hook, generated-boundary
helper, or tightly related implementation block. Adjacent implementation lines
may be justified together when they serve one clear authority reason.

Acceptable authority reasons:

* current runtime authority
* stable storage, backup, recovery, or deployment-truth contract
* generated-boundary requirement
* stable facade contract
* test or diagnostics ownership that does not widen production visibility
* narrow implementation support for one of the above

Owner means the smallest Canic boundary that owns the invariant or contract:
module, crate, facade boundary, generated boundary, storage/recovery contract,
deployment-truth workflow, diagnostics/test support, or named maintainer when
an architectural decision is required.

Non-reasons:

* historical compatibility before `1.0.0`
* "it is currently used" without explaining why the consumer should still exist
* convenience re-exporting
* avoiding churn
* possible future use without an owner decision
* test-only use that keeps production surface wider than necessary

If a code unit has no current authority reason, do not classify it as live.
Classify it as `stale-compatibility`, `stale-generated-fallback`,
`orphaned-helper`, `overexposed-internal`, `duplicate-surface`, or `unclear`.
Use `unclear` only when the missing authority reason needs an owner decision,
not as a way to retain code by default.

## Deletion Pressure Standard

Every retained item must answer all of these:

* What breaks if this is deleted?
* Is that break desirable because the caller is stale, test-only,
  generated-only, overexposed, or preserving old vocabulary?
* Is the consumer production code, generated code, diagnostics, tests, or
  historical compatibility?
* Can the consumer be changed more simply than retaining the surface?
* Is the item public because users need it, or because it was convenient?
* Does the item protect a real invariant, or does it only add vocabulary?

Reference reachability is evidence only. It is not a retention reason.

Special pressure rules:

* If only tests use production surface, move the surface to test support or
  delete it unless it guards a production invariant.
* If only generated code uses the surface, it belongs behind `__internal`,
  `__build`, `__reexports`, or another generated boundary, not normal public
  API.
* If an abstraction has one caller and does not protect a meaningful invariant,
  inline it or mark `INLINE NOW`.
* If a crate exists only for a tiny helper, identify the real owner and mark
  `MOVE OWNER`, unless the crate deliberately preserves a workspace boundary.
* If a huge module is live, the audit must name the owner and either a concrete
  shrink trigger or an explicit reason the module should stay whole.

## Hot-Path / Wasm Regression Gate

Evidence mode: `classified`

MSH is allowed to flag optimization risk, but it is not a broad optimizer pass.
The gate exists to stop cleanup from reintroducing shapes the project has
already worked to avoid.

Hotness classes:

* `cold`: setup, CLI, diagnostics, docs, or rare error path.
* `warm`: normal runtime path but not known to dominate canister execution.
* `hot-runtime`: storage, stable-memory, lifecycle, endpoint, scheduler,
  deployment apply, backup, recovery, or repeated canister runtime path.
* `encode-decode-hot`: Candid, CBOR, snapshot, manifest, stable record, wasm
  chunk, or canonical encode/decode loops.
* `query-executor-hot`: query endpoint, projection, pagination, topology scan,
  pool scan, metrics materialization, or repeated read-only execution loop.
* `wasm-sensitive`: code shape likely to affect canister raw `.wasm` bytes or
  instruction count.
* `test-only`: no production/runtime reachability.

Optimization-risk signals:

* new closure-based generic visitors in encode/decode, stable-memory, query, or
  scheduler loops
* broad generic helpers likely to increase monomorphization
* trait objects or dynamic dispatch on repeated runtime paths
* extra `Vec`, `String`, `format!`, clone, allocation, or staging buffers on
  the success path
* replacing callback/state walkers with closures or iterator adapters without
  proof
* moving from direct field access or direct calls to layered helper chains in a
  hot loop
* success-path diagnostics or rendering work
* widening public APIs in a way that forces retained generic implementations

Required dispositions for optimization-risk findings:

* `RETAIN HOT PATH`: keep the current shape because the optimized structure is
  intentional and the cleanup does not justify the risk.
* `MEASURE FIRST`: do not patch until focused wasm, instruction, or benchmark
  evidence exists.
* `PATCH WITH PROOF`: cleanup is acceptable only with the named proof.
* `REJECT CLEANUP`: the simpler shape is structurally nice but worse for the
  current runtime goal.

Produce:

| Code Unit [M] | Hotness [C] | Proposed Cleanup [C] | Optimization Risk [C] | Required Proof [C] | Disposition [C] |
| ---- | ---- | ---- | ---- | ---- | ---- |

## Disposition Taxonomy

Every retained or candidate item gets exactly one disposition:

* `DELETE NOW`: remove the item in the current slice.
* `NARROW NOW`: reduce visibility or move behind the correct hidden/generated
  boundary in the current slice.
* `INLINE NOW`: inline a one-caller or vocabulary-only abstraction in the
  current slice.
* `MOVE OWNER`: move the item to the crate/module that owns the invariant.
* `MOVE TO TEST`: move the item into test support.
* `RETAIN WITH OWNER`: keep the item; report the owner and invariant it
  protects.
* `DEFER WITH TRIGGER`: keep temporarily; report the exact future event that
  should force deletion, narrowing, inlining, or movement.
* `RETAIN HOT PATH`: keep because a cleanup would risk a known hot or
  wasm-sensitive shape without enough proof.
* `MEASURE FIRST`: require wasm, instruction, or benchmark evidence before any
  code change.
* `PATCH WITH PROOF`: perform the cleanup only with the named focused
  validation, measurement, or regression test.
* `REJECT CLEANUP`: reject the cleanup because it simplifies code at the expense
  of a current runtime or wasm goal.
* `BLOCKED`: owner decision, generated artifact, or release policy evidence is
  required before changing it.

Avoid bare "defer". A deferral without a trigger is just retention by default.

## Read-Only-First Rule

MSH reports are read-only by default. The auditor should produce findings,
classifications, dispositions, and proof requirements before changing code.

Code changes are allowed only when the user explicitly asks to implement a
cleanup, or when the current task is already an implementation slice. Even then,
the patch should be the smallest change that satisfies the report disposition
and required proof.

When the task says to "run the audit", use this read-only mode. When the task
says to "clean up" or "run MSH cleanup", use the module cleanup runner and patch
only the safe dispositions allowed there.

## Surface Taxonomy

Classify every candidate as exactly one:

* `live-authority`: current runtime authority, stable storage contract,
  deployment/backup authority, or stable facade contract.
* `live-generated-boundary`: required by macro/generated code wiring.
* `live-diagnostics`: feature-gated or test-retained diagnostics surface.
* `live-test-support`: production-adjacent helper intentionally owned for
  tests.
* `stale-compatibility`: retained for obsolete internal format/protocol support.
* `stale-generated-fallback`: generated or macro fallback path that should be
  retired or hidden under the generated boundary.
* `orphaned-helper`: helper, type, enum variant, or module with no current
  owner.
* `overexposed-internal`: reachable surface wider than its actual consumers
  need.
* `duplicate-surface`: parallel API paths where one canonical owner is enough.
* `unclear`: insufficient evidence; requires owner decision.

## Deletion Confidence

Use this scale:

* `high`: no non-test consumers, no facade/generated boundary role, compile and
  focused tests can prove removal.
* `medium`: consumers exist but can be rewired to a canonical owner without
  changing behavior.
* `low`: public/facade/generated/runtime authority surface, or consumer evidence
  is incomplete.
* `blocked`: cannot decide without external owner, release policy, generated
  artifact evidence, or wasm/runtime proof.

Do not recommend deletion for `low` or `blocked`; recommend
`DEFER WITH TRIGGER` or `BLOCKED` instead. `medium` confidence should normally
produce a concrete `NARROW NOW`, `INLINE NOW`, `MOVE OWNER`, or `MOVE TO TEST`
action unless the blast radius is too broad for the slice.

## STEP 0 - Run Metadata

Evidence mode: `semi-mechanical`

Capture:

| Field [M/C] | Value |
| ---- | ---- |
| `method_version` | `MSH-2.0` |
| `baseline_report` | path or `N/A` |
| `comparability_status` | `comparable` / `non-comparable` |
| `code_snapshot` | git short SHA or `N/A` |
| `in_scope_roots` | roots inspected |
| `excluded_roots` | roots excluded |
| `generated_code_inclusion` | included / excluded / sampled |
| `test_surface_inclusion` | included / excluded / sampled |
| `audit_tier` | `Tier 0` / `Tier 1` / `Tier 2` |
| `patch_mode` | `read-only` / `implementation-requested` |

## STEP 1 - Reachable Surface And Retention Inventory

Evidence mode: `mechanical`

Inventory surface and retained code units that can create dead complexity:

* crate-root `pub mod` and `pub use`
* facade `pub mod`, `pub use`, and stable prelude exports
* `#[doc(hidden)]` exports
* `__internal`, `__build`, `__reexports`, and macro/build support exports
* `pub(crate)`, `pub(in ...)`, and `pub(super)` items in hub modules
* cfg-gated diagnostics/test exports
* public error variants, DTOs, views, and records with internal representation
  payloads
* private helpers, branch families, DTO fields, and module-local impl blocks in
  hub or hotspot modules when they materially retain complexity

Produce:

| Item [M] | Kind [M] | Path [M] | Visibility [M] | Feature/Cfg [M] | Consumer Evidence [M/C] | Consumer Should Exist? [C] | Authority Reason [C] | Surface Class [C] | Owner [C] | Disposition [C] | Risk [C] |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |

Consumer evidence should prefer compiler reachability, direct imports,
generated output references, macro expansion evidence, or focused code
inspection over text counts.

## STEP 2 - Dead / Stale Surface Signals

Evidence mode: `semi-mechanical`

Scan for:

* `#[allow(dead_code)]`, `#[expect(dead_code)]`, `#[expect(unused_imports)]`
* `legacy`, `compat`, `compatibility`, `fallback`, `shim`, `deprecated`, `old`,
  and `temporary`
* generated-boundary fallback paths that expose broad internals instead of
  using narrow hidden exports
* runtime reconstruction from DTOs, snapshots, deployment evidence, or generated
  metadata after a stronger authority is available
* duplicate endpoint, route, lifecycle, install, deployment, backup, cursor,
  storage, metrics, or diagnostics entrypoints
* enum variants whose only remaining purpose is old transition handling
* public DTO fields that expose implementation representation without a stable
  facade reason
* private helpers or match arms retained only because a broad module still uses
  them after its authority moved elsewhere

Produce:

| Candidate [M] | File [M] | Lines [M] | Signal [M] | Current Consumers [M/C] | Consumer Should Exist? [C] | Authority Reason [C] | Surface Class [C] | Deletion Confidence [C] | Disposition [C] | Risk If Removed [C] |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |

## STEP 3 - Runtime Authority Drift Check

Evidence mode: `classified`

Check that runtime behavior does not keep obsolete authorities alive.

Required Canic checks:

* stable records, stable-memory registries, and restored runtime state are
  authoritative for canister runtime state.
* DTOs and views remain passive boundary/read-only shapes.
* config/build inputs own build planning; deployment-truth reports and receipts
  are evidence, not mutation authority.
* authority reconciliation dry runs remain read-only; controller mutation is
  explicit and separately gated.
* generated/macro surfaces are limited to endpoint, lifecycle, build, and
  hidden support wiring.
* facade modules expose stable user-facing concepts, not core internals by
  convenience.
* capability, replay, subject/caller, audience, subnet, parent, and controller
  bindings are explicit at the correct boundary.
* persisted decoding remains bounded and fallible; removing dead surface must
  not replace a typed contract with ad hoc reconstruction.

Produce:

| Area [C] | Runtime Authority [C] | Alternate Authority Found? [C] | Evidence [M/C] | Allowed Role? [C] | Finding [C] | Risk [C] |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |

## STEP 4 - Complexity That Exists Only For Dead Surface

Evidence mode: `semi-mechanical`

Focus on complexity that can shrink by deletion, visibility narrowing,
inlining, or ownership movement. General design complexity is out of scope only
when each inspected code unit has a current authority reason.

Measure:

* branch sites protecting obsolete compatibility modes
* enum variants used only to route to old behavior
* match arms for no-longer-supported formats or route shapes
* modules whose public API count is high while consumer count is low
* feature-gated branches whose feature no longer has a reachable caller
* facade re-export chains that widen internal implementation surface
* helper blocks whose only justification is that a nearby live module still
  happens to call them

Produce:

| Module [M] | Complexity Signal [M] | Retention Justification [C] | Dead-Surface Link [C] | Public/Hidden Items [M] | Current Consumers [M/C] | Shrink Action [C] | Disposition [C] | Expected Blast Radius [C] | Risk [C] |
| ---- | ---- | ---- | ---- | ----: | ---- | ---- | ---- | ---- | ---- |

## STEP 5 - Facade / Generated Boundary Review

Evidence mode: `classified`

Review `canic` facade, `canic-macros`, generated endpoint support,
`canic::__internal`, `canic::__build`, `canic_core::__reexports`, and
`canic_core::api` separately from normal internal API.

Do not classify generated-boundary surface as dead until generated output,
macro expansion, or endpoint/build tests prove it is unused.

Check:

* facade modules expose stable user-facing concepts, not core internals by
  convenience.
* `#[doc(hidden)]` exports are either generated-boundary requirements or
  explicitly temporary.
* macro/build support does not keep broad runtime internals public when a
  narrower hidden helper would work.
* generated endpoint metadata, authenticated endpoint wiring, lifecycle
  wiring, and build-time config embedding do not preserve obsolete route names
  or fallback shapes.

Produce:

| Surface [M] | Boundary Type [C] | Generated Consumer Evidence [M/C] | Could Narrow? [C] | Required Replacement [C] | Deletion Confidence [C] | Disposition [C] | Risk [C] |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |

## STEP 6 - Feature / Diagnostics / Test Surface Review

Evidence mode: `semi-mechanical`

Check feature-gated and test-only surfaces:

* `control-plane`
* build-support cfgs such as non-wasm/test-only helpers
* metrics, perf, and diagnostics hooks
* generated endpoint switches
* host/CLI text rendering and report fixtures
* test-only re-exports, seams, and fixtures

Classify test and diagnostics surface by current owner. Test-only surface may be
valid, but it should not force production visibility wider than necessary.

Produce:

| Surface [M] | Feature/Cfg [M] | Production Consumer? [M/C] | Test/Diagnostics Consumer? [M/C] | Visibility Could Narrow? [C] | Action [C] | Disposition [C] | Risk [C] |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |

## STEP 7 - Removal Safety Plan

Evidence mode: `classified`

For every `high` or `medium` confidence candidate, and every inspected code unit
whose retention justification is missing or weak, define the smallest safe
change.

Allowed actions:

* delete
* narrow visibility
* inline one-caller or vocabulary-only helpers
* collapse duplicate export to canonical owner
* move owner
* move to test-only module
* replace stale compatibility branch with current-format hard cut
* retain hot-path shape
* measure before changing
* reject cleanup on optimization grounds
* defer with a specific trigger
* block on owner decision before touching

Produce:

| Candidate [M] | Action [C] | Disposition [C] | Owner Boundary [C] | Hotness [C] | Required Proof [C] | Focused Validation [C] | Wasm Raw Bytes Relevant? [C] | Follow-Up Trigger [C] |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |

## STEP 8 - Runtime Shape / Optimization Risk Review

Evidence mode: `classified`

For every candidate that touches hot or wasm-sensitive code, classify the
runtime-shape risk before recommending a patch.

Do not require wasm or instruction measurement for every cleanup. Require it
when the code sits in a hotness class and the cleanup changes allocation,
dispatch, generic expansion, data movement, encode/decode flow, stable-memory
flow, controller mutation flow, or endpoint success-path work.

Produce:

| Candidate [M] | Hotness [C] | Runtime Shape Today [C] | Proposed Shape [C] | Risk Signal [C] | Required Proof [C] | Disposition [C] |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |

## STEP 9 - Risk Scoring

Evidence mode: `classified`

Score only removable/narrowable surface, not the whole architecture.

Risk index:

* `0-2`: low dead-surface pressure
* `3-5`: moderate cleanup queue; track follow-ups
* `6-8`: high complexity retained by stale surface
* `9-10`: critical; obsolete authority or fallback path can distort runtime
  behavior or block pre-`1.0.0` hard cuts

Produce:

| Bucket [C] | Count [D] | Highest Risk [C] | Notes [C] |
| ---- | ----: | ---- | ---- |
| stale compatibility |  |  |  |
| stale generated fallback |  |  |  |
| orphaned helper |  |  |  |
| overexposed internal |  |  |  |
| duplicate surface |  |  |  |
| unclear |  |  |  |
| optimization-risk cleanup |  |  |  |

## Required Report Sections

Tier 2 full reports must include:

1. run metadata
2. step status table
3. evidence log
4. reachable surface and retention inventory summary
5. dead/stale candidate table
6. runtime authority drift findings
7. facade/generated-boundary findings
8. removal safety plan
9. runtime shape / optimization risk findings
10. risk score
11. verification readout
12. disposition summary
13. follow-up actions or explicit "none"

Step status table:

| Step [M] | Status [C] | Evidence Artifact [M/C] | Comparability Impact [C] |
| ---- | ---- | ---- | ---- |
| STEP 0 |  |  |  |
| STEP 1 |  |  |  |
| STEP 2 |  |  |  |
| STEP 3 |  |  |  |
| STEP 4 |  |  |  |
| STEP 5 |  |  |  |
| STEP 6 |  |  |  |
| STEP 7 |  |  |  |
| STEP 8 |  |  |  |
| STEP 9 |  |  |  |

Allowed statuses:

* `PASS`: evidence and table are present.
* `N/A`: method explicitly allows no candidates or no relevant surface.
* `BLOCKED`: evidence could not be produced; include a concrete reason.

## Evidence Log

Every Tier 1 or Tier 2 report must include the exact commands or inspections
used to classify the surface. Tier 0 cleanup summaries should include the same
log when there was any ambiguity or blocked validation.

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| public surface inventory |  |  |  |
| stale-signal scan |  |  |  |
| consumer check |  |  |  |
| generated/facade boundary check, if relevant |  |  |  |
| hot/wasm check, if relevant |  |  |  |
| validation |  |  |  |

## Tier 1 Compact Report

Use this shape when the tier selector chooses a compact MSH report:

```markdown
# MSH Compact Audit: <module>

## Verdict
- Risk score:
- Tier:
- Patch mode:
- Main decision:

## Scope / Evidence
| Area | Evidence | Result |
| ---- | ---- | ---- |

## Evidence Log
| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| public surface inventory |  |  |  |
| stale-signal scan |  |  |  |
| consumer check |  |  |  |
| validation |  |  |  |

## Findings
| Item | Class | Confidence | Disposition | Authority / reason |
| ---- | ---- | ---- | ---- | ---- |

## Hot / Wasm Risk
| Item | Hotness | Risk | Required proof |
| ---- | ---- | ---- | ---- |

## Disposition Ledger
| Disposition | Count |
| ---- | ----: |
| DELETE NOW |  |
| NARROW NOW |  |
| INLINE NOW |  |
| MOVE TO TEST |  |
| RETAIN WITH OWNER |  |
| MEASURE FIRST |  |
| BLOCKED |  |

## Validation / Follow-up
- Required validation:
- Blocked decisions:
- Triggers:
```

## Disposition Ledger

Track disposition counts during pilots and repeat runs to prove the audit is
producing cleanup decisions rather than only documentation.

| Disposition | Count |
| ---- | ----: |
| DELETE NOW |  |
| NARROW NOW |  |
| INLINE NOW |  |
| MOVE TO TEST |  |
| RETAIN WITH OWNER |  |
| MEASURE FIRST |  |
| BLOCKED |  |

## Suggested Evidence Commands

These are prompts for the auditor, not mandatory exact commands:

* enumerate public exports and hidden exports from crate roots
* search for `dead_code`, `unused_imports`, `legacy`, `compat`, `fallback`,
  `shim`, `deprecated`, `temporary`, broad generated-boundary names, and
  snapshot/reconstruction vocabulary
* inspect facade, `__internal`, `__build`, `__reexports`, and macro support
  re-export chains
* inspect direct imports and focused compiler failures before classifying
  reachability
* run focused compile/tests after proposed deletions or visibility narrowing
* compare raw non-gzipped wasm bytes only when the candidate affects canister
  runtime payload
* run existing instruction/perf tests when a cleanup changes a measured hot path
* inspect generated wasm-impacting code for added closures, generic helpers,
  allocations, clones, formatting, or dynamic dispatch on success paths

Suggested deletion-pressure prompts:

* list items with exactly one production caller and inspect whether the helper
  protects an invariant or just adds vocabulary
* list public or hidden exports consumed only by generated code and check
  whether they belong under a hidden generated boundary
* list production items consumed only by tests and check whether they can move
  to test support
* list largest modules and require either a retained-owner explanation or a
  concrete split/deletion trigger
* inspect crate boundaries where a crate remains for one or two helpers and ask
  whether ownership should move back to the real caller
* reject or measure any cleanup that makes hot code prettier by replacing an
  explicit state/callback loop with a generic closure, iterator stack, or
  allocation-heavy helper

Do not start or stop the local ICP/DFX network for this audit.

## Pilot Rollout

Start using this process in three passes:

1. Pilot on one cold or test-only non-core module with Tier 0 or Tier 1. Confirm
   the report stays short and validation is not noisy.
2. Pilot on one generated/facade-adjacent module with Tier 2. Confirm the
   generated-boundary and facade rules guide decisions instead of creating table
   work without cleanup decisions.
3. Review the disposition ledger. If most findings are `RETAIN WITH OWNER`,
   `MEASURE FIRST`, or `BLOCKED`, tighten target selection before making MSH
   the standard cleanup path.
