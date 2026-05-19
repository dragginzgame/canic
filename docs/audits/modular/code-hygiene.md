# Audit: Module Code Hygiene / Redundancy

Use this modular audit when a Canic module looks larger than its behavior
contract requires, or when recent churn may have left wrappers, shims,
compatibility residue, duplicated checks, or implementation-detail tests behind.

This is a deletion-first hygiene audit. It is not LoC reduction for its own
sake. Line count is only a measurement tool for finding and validating
redundancy removal.

## Prompt

```text
You are auditing a Canic module for code hygiene, redundancy, and boundary
discipline.

Do not edit code in this pass.

Target module:

    <MODULE_PATH>

Goal:

Determine how much of this module is essential behavior versus post-churn
scaffolding, wrappers, duplicated checks, stale abstractions, overly defensive
error mapping, compatibility residue, DTO mirroring, or tests that assert
implementation details instead of behavior.

This is not a feature redesign.
This is a code-hygiene and boundary audit.

Core question:

What is the minimum code Canic needs in this module to preserve externally
meaningful behavior?

The burden of proof is on keeping code, not deleting it.

Do not say "keep for safety" unless you can name the specific behavior,
invariant, call site, protocol surface, or test that requires it.
```

## Audit Scope

Inspect:

```text
<MODULE_PATH>/**
```

Also inspect direct call sites and tests needed to understand behavior,
including likely references in:

```text
crates/canic-core/src/ops/**
crates/canic-core/src/api/**
crates/canic-core/src/dispatch/**
crates/canic-core/src/storage/**
crates/canic-core/src/dto/**
crates/canic-core/src/workflow/**
crates/canic-core/tests/**
crates/canic-tests/**
docs/**
```

Do not broaden beyond what is needed to classify the target module.

## Behavior Contract

Before recommending deletions, identify the externally meaningful behavior
contract.

Classify every behavior as one of:

- `externally meaningful and must preserve`
- `internal implementation detail`
- `duplicate of another module`
- `stale / no longer used`
- `unclear and needs call-site confirmation`

The behavior contract should include only things that matter to callers, tests,
protocol surfaces, safety invariants, or runtime correctness.

Everything else is fair game.

## Classification Labels

Classify every non-test type, function, and module in the target module as one
of:

- `KEEP`: directly preserves the behavior contract.
- `DELETE`: obsolete, unused, duplicated, stale, or only supports removed
  internals.
- `INLINE`: one-use wrapper/helper that does not improve clarity.
- `COLLAPSE`: duplicate DTO/error/helper/module can merge into caller.
- `MOVE`: belongs in a different existing module.
- `TEST-ONLY`: should exist only in tests or fixtures.
- `INVESTIGATE`: insufficient evidence; name exact call sites or facts needed.

## Size Map

Produce a precise size map.

For every file under the target module:

| File | Raw LoC | Non-test LoC | Test LoC | Main responsibility | Initial verdict |
| --- | ---: | ---: | ---: | --- | --- |
| `<path>` | `<n>` | `<n>` | `<n>` | `<responsibility>` | `<verdict>` |

Also produce totals:

| Category | LoC |
| --- | ---: |
| Non-test | `<n>` |
| Inline tests | `<n>` |
| Total | `<n>` |

## Responsibility Map

For each file/module, classify responsibility using domain-specific terms.

Examples:

- policy decision
- runtime guard
- storage access
- stable persistence
- DTO/API shaping
- endpoint integration
- lifecycle orchestration
- diagnostic facade
- error mapping
- compatibility residue
- generic helper
- test fixture
- unclear

Then state whether that responsibility belongs in this module or somewhere
else.

## Rent Table

For every non-test type, function, and module:

| Item | Classification | Rationale | Recommended action |
| --- | --- | --- | --- |
| `<item>` | `<KEEP/DELETE/...>` | `<why it pays rent or does not>` | `<exact action>` |

Rules:

- If it has one call site and does not clarify a real invariant, recommend
  `INLINE`.
- If it mirrors another DTO/type without adding meaning, recommend `COLLAPSE`
  or `DELETE`.
- If it exists only to preserve an old name, recommend `DELETE` or `INLINE`.
- If it is generic and not module-specific, recommend `MOVE`.
- If it exists only for tests, recommend `TEST-ONLY` or `DELETE`.

## Redundancy Audit

Look for duplicated or near-duplicated logic, including:

- repeated policy checks
- repeated guards
- repeated storage lookups
- repeated validation wrappers
- repeated error conversions
- repeated DTO conversion code
- repeated metrics/logging facades
- repeated test fixtures
- wrapper functions that only call another wrapper
- old helper names preserved after refactors

For each duplication:

| Duplicated logic | Locations | Risk | Recommended action |
| --- | --- | --- | --- |
| `<logic>` | `<files/items>` | `<risk>` | `<concrete action>` |

Recommended actions must be concrete:

- delete one copy
- inline wrapper
- collapse error variants
- move shared helper
- keep duplication because contexts differ

Avoid vague recommendations.

## Dead / Low-Value Code Audit

Search for likely residue:

```bash
rg "pub fn|pub struct|pub enum|pub trait|fn " <MODULE_PATH>
rg "TODO|FIXME|legacy|compat|deprecated|old|temporary|shim|adapter|wrapper" <MODULE_PATH>
rg "BTreeMap|HashMap|OnceCell|thread_local|REGISTRY|Runtime|State|Record|Entry|Descriptor" <MODULE_PATH>
```

Also search domain-specific terms relevant to the module.

For each public item:

| Public item | Call sites | Keep/delete/inline/move | Reason |
| --- | --- | --- | --- |
| `<item>` | `<sites>` | `<action>` | `<reason>` |

If an item is public only within the crate and has one call site, recommend
inlining unless it materially clarifies the domain.

## Error Model Audit

Inspect module-related error types.

Check:

- Are there too many variants?
- Do variants add actionable context?
- Are wrappers merely translating another error with no added meaning?
- Are user-facing/API errors separated from internal failures?
- Are tests asserting overly specific internal error shapes?
- Are old compatibility errors still present after the compatibility path was
  removed?

Recommend collapsing only when it does not weaken externally meaningful
diagnostics.

## Test Audit

Classify tests touching this module.

For each test or test group:

| Test | What it asserts | Keep/rewrite/delete | Reason |
| --- | --- | --- | --- |
| `<test>` | `<assertion>` | `<action>` | `<reason>` |

Classification:

- behavior/invariant test: keep
- implementation-detail test: rewrite or delete
- duplicate coverage: collapse
- obsolete compatibility test: delete
- fixture/helper bloat: move/collapse

The ideal test suite should assert externally meaningful behavior and safety
invariants.

It should not preserve:

- old helper names
- wrapper layering
- private struct layout
- source-text ordering
- internal dispatch mechanisms
- deleted compatibility paths

## Boundary Audit

Answer directly:

- What should this module own?
- What should this module not own?
- Which code should move to existing modules?
- Which code should be deleted outright?
- Which code should be compressed but kept?
- Which tests protect real behavior?
- Which tests protect scaffolding?

Do not propose a new crate unless explicitly asked.

Do not propose broad architecture churn unless the module is clearly misplaced.

## Footprint Target

Estimate realistic cleanup.

Provide:

| Area | Current non-test LoC | Realistic target | How to get there |
| --- | ---: | ---: | --- |
| `<area>` | `<n>` | `<n>` | `<specific cleanup>` |

Be aggressive but credible.

If the module is already lean, say so.

If it is bloated, name the exact sources of bloat.

## Output Format

Produce:

```text
<MODULE_NAME> Code Hygiene Audit

Verdict
- Is this module bloated?
- Boundary cleanliness rating: X/10
- Current non-test LoC
- Realistic target non-test LoC
- Highest-value deletion
- Highest-risk deletion
- First safe cleanup pass

Size Map
- Include the file table.

Behavior Contract
- List what must be preserved.

Rent Table
- Include the item table.

Duplications
- Include the duplication table.

Dead / Low-Value Code
- List delete/inline/collapse candidates.

Error Model
- List error cleanup opportunities.

Test Cleanup
- Include the test table.

Boundary Recommendations
- State what belongs in this module and what does not.

Proposed Cleanup Plan
- Phase 1: Safe deletion/inlining. Exact items only.
- Phase 2: Error/test collapse. Exact items only.
- Phase 3: Boundary moves, if any. Exact items only.

Implementation Prompt for Phase 1
- Include a ready-to-use implementation prompt.

Validation Commands for a Future Implementation Pass
- Include module-appropriate commands.
```

## Implementation Prompt Requirement

At the end of the audit report, write a ready-to-use implementation prompt for
the first cleanup phase.

That prompt must be narrow, behavior-preserving, and scoped to the
highest-value / lowest-risk cleanup.

## Validation Commands

Recommend module-appropriate commands, starting with:

```bash
cargo fmt --all --check
cargo test -p canic-core <MODULE_OR_RELEVANT_FILTER> --lib
cargo test -p canic-core
cargo clippy -p canic-core --all-targets -- -D warnings
cargo check --workspace
git diff --check
```

## Strict Constraints

- Do not edit code.
- Do not preserve internal APIs for their own sake.
- Do not recommend keeping code unless it pays rent against the behavior
  contract.
- Do not hide behind "safety" without naming the concrete invariant.
- Do not propose compatibility preservation unless compatibility is part of the
  behavior contract.
- Do not propose new abstractions unless they remove more code than they add.
- Do not treat LoC reduction as the goal. Treat LoC as supporting evidence for
  redundancy, boundary drift, and code hygiene.

## Reusable Implementation Prompt Template

Use this after the audit, once Phase 1 has been selected:

```text
You are working in:

    <MODULE_PATH>

Implement Phase 1 from the module code-hygiene audit.

This is a narrow deletion/compression pass, not a broad refactor.

Scope:

- <FILES_TO_TOUCH>

Goal:

- <SPECIFIC_DELETION_OR_COLLAPSE_GOAL>

Preserve this behavior contract:

- <BEHAVIOR_1>
- <BEHAVIOR_2>
- <BEHAVIOR_3>

Do not touch in this pass:

- <HIGH_RISK_AREA_1>
- <HIGH_RISK_AREA_2>
- <UNRELATED_MODULES>

Expected cleanup:

- delete <ITEMS>
- inline <ITEMS>
- collapse <ITEMS>
- rewrite/delete tests that only preserve <SCAFFOLDING>

Rules:

- Do not preserve wrappers just because tests reference them.
- If a test asserts implementation layering, rewrite it to assert behavior.
- Do not add new abstractions unless they remove more code than they add.
- Keep public behavior unchanged.
- Keep error/metric/protocol behavior unchanged unless the audit explicitly
  authorized a change.

Before editing, inspect and summarize:

1. Current call sites.
2. Tests that mention the implementation detail being removed.
3. Behavior those tests should assert instead.

Validation:

- cargo fmt --all --check
- cargo test -p canic-core <MODULE_OR_RELEVANT_FILTER> --lib
- cargo test -p canic-core
- cargo clippy -p canic-core --all-targets -- -D warnings
- cargo check --workspace
- git diff --check

Output:

1. Items deleted.
2. Items inlined/collapsed.
3. Tests rewritten/deleted.
4. Before/after LoC.
5. Behavior preserved.
6. Validation results.
7. Deferred cleanup.
```
