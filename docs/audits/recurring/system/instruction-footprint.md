# Audit: Instruction Footprint

## Purpose

Track runtime instruction drift over time using Canic's endpoint perf counters
and `perf!` checkpoints.

This is a runtime execution audit.

It is NOT:

- a cycle-cost audit
- a wasm-size audit
- a correctness audit

The job of this audit is to measure how many local instructions Canic endpoints
and multi-step flows actually execute, explain where the hot paths live, and
catch regressions before they become permanent shared runtime cost.

This audit is not permission to remove intended behavior to make the numbers
look better.

## Why This Audit Is Canic-Specific

Canic already has two instruction-observability mechanisms:

1. endpoint/timer aggregation in `canic-core::perf`, surfaced through
   `canic_metrics(MetricsKind::Perf, ...)`
2. manual `perf!` checkpoints that now both record structured checkpoint rows
   and log `Topic::Perf` entries inside a single call context

That means this audit must do more than "time a request":

- endpoint totals need to be captured from the metrics surface
- long-running flows need checkpoint coverage, not only end-of-call totals
- replay/auth/bootstrap paths must be sampled with multiple argument classes
- fresh setup boundaries matter because perf counters are cumulative inside a
  canister instance

An audit copied from a generic HTTP service or a single-canister project will
miss these properties and will not be comparable.

## Risk Model / Invariant

This is a drift audit, not a functional correctness invariant audit.

Risk model:

- silent instruction growth taxes every shared endpoint and background path
- argument-sensitive regressions hide behind "happy path" spot checks
- multi-step flows without checkpoints are hard to optimize safely
- auth/replay/admin rejection paths can become more expensive than the
  authorized path without anyone noticing

Optimization constraint:

- reduce instruction use without removing intended behavior or operator-facing
  signal
- do not treat feature removal as a normal perf win
- do not confuse instruction count with cycle charges from management calls

Invariant:

- important endpoints and flows should remain measurable, comparable, and
  explainable across runs
- critical multi-step flows should either have named `perf!` checkpoints or be
  explicitly listed as coverage gaps

## Run This Audit After

- endpoint bundle changes
- auth, replay, or capability pipeline refactors
- sharding, scaling, or pool orchestration changes
- lifecycle/bootstrap changes
- changes to `perf!`, `canic_metrics`, or `canic-core::perf`
- any PR claiming "performance improvement" or "no perf impact"

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Definition path
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status
- Auditor
- Run timestamp (UTC)
- Branch
- Worktree
- Execution environment (`PocketIC`, local `dfx`, mixed)
- Target canisters in scope
- Target endpoints/flows in scope

## Measurement Model (Mandatory)

Use these terms consistently.

### Canonical Row Model

The audit authority is a normalized row model, not the exact public metrics DTO
shape of the current release.

Every captured perf sample must normalize into rows with these semantic fields:

- `subject_kind`
- `subject_label`
- `count`
- `total_local_instructions`
- `avg_local_instructions`
- `scenario_key`
- `scenario_labels`
- `principal_scope` when relevant
- `sample_origin` (`metrics`, `checkpoint`, or derived)

Minimum expectations:

- endpoint samples normalize to `subject_kind = endpoint`
- timer samples normalize to `subject_kind = timer`
- checkpoint samples normalize to `subject_kind = checkpoint`

The transport may change.

Current likely source transport is:

- `canic_metrics(MetricsKind::Perf, PageRequest { ... })`

But reports must compare canonical row fields, not concrete response enum names,
DTO variants, or label vector layout.

If the transport changes but the normalized semantics stay the same, the method
tag may remain stable.

If the normalized semantics change, the method tag must change.

### Endpoint Perf Counters

The authoritative machine-readable signal is the canister perf table exposed
through the public metrics surface and normalized into the canonical row model.

Current interpretation:

- `count` = number of recorded executions
- `total_local_instructions` = accumulated local instructions for that subject
- `avg_local_instructions = total_local_instructions / count`

### `perf!` Checkpoints

`perf!` is a checkpoint mechanism for within-flow attribution.

It:

- reads `performance_counter(1)`
- computes delta since the last checkpoint in the current thread/call
- records a structured checkpoint row in the shared perf table
- emits a `Topic::Perf` log line

Checkpoint consequences:

- checkpoint order matters
- comparisons require the same checkpoint names and placement
- reports should prefer structured checkpoint rows when they exist
- raw `Topic::Perf` lines remain useful supporting evidence when log context
  matters
- flows without checkpoint callsites must still be reported as coverage gaps

### Accounting Rule: Endpoint Totals vs Checkpoint Deltas

Endpoint perf totals and `perf!` checkpoint deltas are both useful, but they
are not interchangeable.

Current accounting model:

- endpoint perf rows are recorded from the perf stack as exclusive endpoint
  totals
- `perf!` checkpoint rows/logs are inclusive call-context deltas from
  `performance_counter(1)` between two named checkpoints

Audit rule:

- do not subtract checkpoint deltas from endpoint totals as if they were the
  same accounting layer
- do not compare a checkpoint delta directly to an endpoint total unless the
  report explicitly states why that comparison is valid
- use endpoint rows for stable regression tracking
- use checkpoints for within-flow attribution and hotspot localization

### Counter Semantics

These counts come from `performance_counter(1)`.

That means:

- they count local canister instructions in the current call context
- they accumulate across `await` points inside that call context
- they do not count instructions executed in other canisters
- they do not represent total cycle cost

Do not compare instruction counts directly to management-call cycle charges.

### Freshness Rule

Perf counters are cumulative within the running canister instance.

So comparable samples require one of these:

- a fresh PocketIC topology or fresh canister install per scenario group
- a documented single-scenario-per-instance run
- an explicit report note that multiple scenarios intentionally share the same
  counter table

If this freshness rule is violated, deltas are non-comparable.

## Scope

Measure and report:

- shared query/update endpoint instruction totals
- root-only admin and capability paths
- representative non-root endpoints
- timer rows when relevant
- available `perf!` checkpoints inside multi-step flows
- explicit checkpoint-coverage gaps where `perf!` is absent

### Default Canister Scope

Default scope is the full reference topology in `dfx.json`:

- `app`
- `minimal`
- `user_hub`
- `user_shard`
- `scale_hub`
- `scale`
- `test`
- `wasm_store`
- `root`

### Default Endpoint Classes

For each canister in scope, sample what exists:

- observability endpoints (`canic_metrics`, `canic_log_page`, directory/state)
- role-specific business endpoints
- auth/capability endpoints
- scaling/sharding endpoints
- store/template publication endpoints
- lifecycle-adjacent admin endpoints where callable in tests

### Timer Isolation Rule

Timer rows must never be mixed into endpoint scenario groups.

Timer samples must document:

- trigger mode (`once` or `interval`)
- expected firing count
- whether the canister instance was otherwise idle
- whether the timer sample shared a counter table with endpoint scenarios

If timer activity cannot be isolated cleanly, mark timer comparability
`PARTIAL`.

### Default Flow Classes

For recurring runs, include at least one representative flow from each active
subsystem:

- root capability dispatch (`create`, `upgrade`, `cycles`, attestation/delegation)
- delegated auth issuance/verification
- replay/cached-response path
- sharding assignment/query flow
- scaling/provisioning flow
- bootstrap/install/publication flow

## Argument Matrix (Mandatory)

For each endpoint sampled, cover as many of these as the endpoint supports:

1. minimal valid input
2. representative valid input
3. boundary or high-cardinality valid input
4. rejection/failure path
5. repeated-call path where caching/replay/paging matters

Examples:

- page endpoints: small page, larger page, empty page
- capability endpoints: authorized, unauthorized, proof-rejected, replayed
- create/upgrade flows: cheap/no-op path and real execution path
- sharding/scaling queries: empty registry and non-empty registry

If a class is not applicable, state that explicitly.

### Scenario Identity Tuple

Every measured scenario must have a stable identity tuple.

Minimum tuple:

- `canister`
- `endpoint_or_flow`
- `arg_class`
- `caller_class`
- `auth_state`
- `replay_state`
- `cache_state`
- `topology_state`
- `freshness_model`
- `method_tag`

Why this matters:

- "same endpoint" is not enough for auth, replay, and capability paths
- caller identity and prior replay/cache state can materially change instruction
  cost
- comparability must be anchored to scenario identity, not human memory

## Coverage Scan (Mandatory)

Before capturing perf data:

1. enumerate endpoints in scope
2. scan current checkpoint coverage with
   `rg -n '^[[:space:]]*perf!\\(' crates`
3. list critical flows that have zero checkpoints today
4. mark missing checkpoint coverage as `PARTIAL`, not as silently omitted

Important:

- if there are no current `perf!` call sites, that is a real audit result
- endpoint perf coverage can still pass while flow-checkpoint coverage remains
  partial

### Checkpoint Naming Contract

Checkpoint labels should be short, stage-like, and stable.

Preferred examples:

- `load_cfg`
- `verify_token`
- `read_registry`
- `select_target`
- `assemble_response`

Avoid:

- prose sentences
- unstable wording
- labels that encode transient values

If checkpoint names or order change for a flow, that flow becomes
`N/A (method change)` for cross-run checkpoint deltas.

## Decision Rule

- primary regression authority: isolated instruction totals from comparable
  endpoint or flow runs
- secondary diagnostic: average instructions per execution
- compare only same scenario identity tuple
- if checkpoint names or placement changed, mark the affected delta as
  `N/A (method change)`

Do not claim improvement from:

- comparing authorized vs rejected paths as if they were the same scenario
- comparing fresh-instance results against accumulated counter tables
- comparing instruction counts against wasm-size changes without separate evidence

## Required Report Sections

Every report generated from this definition must include:

- `## Endpoint Matrix`
- `## Flow Checkpoints`
- `## Checkpoint Coverage Gaps`
- `## Structural Hotspots`
- `## Hub Module Pressure`
- `## Dependency Fan-In Pressure`
- `## Early Warning Signals`
- `## Risk Score`
- `## Verification Readout`

### Endpoint Matrix

Must include:

- canister
- endpoint or timer label
- scenario label
- count
- total instructions
- average instructions per execution
- baseline delta or `N/A`

### Flow Checkpoints

Must include:

- flow name
- checkpoint names in order
- per-checkpoint instruction deltas from `Topic::Perf` logs
- missing-checkpoint gaps, if any

### Checkpoint Coverage Gaps

Must include:

- critical flows with checkpoints
- critical flows without checkpoints
- proposed first checkpoint insertion sites for uncovered critical flows

### Structural Hotspots

For the highest-cost endpoints/flows, map the cost back to concrete modules and
files with command evidence.

Examples:

- `rg -n '<endpoint-name>|<flow function>' crates`
- `rg -n '^use ' <hot module directory>`
- direct references to likely hot modules such as:
  - `crates/canic-core/src/workflow/rpc/`
  - `crates/canic-core/src/ops/storage/`
  - `crates/canic-core/src/workflow/ic/`

### Hub Module Pressure

For the hottest instruction paths, normalize pressure on the modules they pass
through:

- number of subsystems imported
- number of sibling module dependencies
- whether the hotspot crosses multiple layers for one request

### Dependency Fan-In Pressure

For each hotspot module, report whether the module is a fan-in hub that raises
regression risk even when the current numbers look acceptable.

### Early Warning Signals

Must call out signals such as:

- new endpoints entering the perf table
- high-growth endpoints with unchanged behavior claims
- rejection paths approaching or exceeding happy-path cost
- critical flows still missing `perf!` checkpoints
- perf growth concentrated in shared hubs rather than role-specific leaves

### Risk Score

Use a normalized `0-10` score.

Rubric:

- `0-2`: shared-runtime regression severity
- `0-2`: hotspot concentration in hub modules
- `0-2`: checkpoint coverage gaps on critical flows
- `0-2`: comparability loss or method drift
- `0-2`: rejection-path inflation / replay-cache-state sensitivity

Report both:

- total score
- one short line per rubric component

## Required Checklist

For each run, explicitly mark `PASS` / `PARTIAL` / `FAIL` with concrete evidence.

1. Endpoints in scope were enumerated before measurement.
2. Checkpoint coverage was scanned with
   `rg -n '^[[:space:]]*perf!\\(' crates`.
3. Comparable scenario identity tuples were defined for each sampled endpoint or
   flow.
4. The current metrics transport was normalized into the canonical row model.
5. Counter freshness/isolation strategy was documented.
6. `Topic::Perf` logs were captured for flows with checkpoints.
7. Flows lacking checkpoints were listed explicitly with proposed insertion
   sites where possible.
8. Timer samples, if present, were isolated from endpoint scenario groups.
9. Baseline path was selected according to daily baseline policy.
10. Deltas versus baseline were recorded when comparable.
11. Verification readout includes command outcomes with `PASS` / `FAIL` /
    `BLOCKED`.

## Execution Contract

Preferred execution environment:

- PocketIC integration tests for repeatable endpoint and flow measurement

Use local `dfx` only when a scenario cannot be represented in PocketIC and the
report explains why.

No canonical runner script exists yet.

Until one exists, each report must:

- list the exact commands used
- emit the required normalized artifacts
- state any manual steps explicitly

Recommended command bundle:

- `rg -n '^[[:space:]]*perf!\\(' crates`
- `cargo test -p canic-tests --test delegation_flow -- --nocapture --test-threads=1`
- `cargo test -p canic-tests --test root_replay -- --nocapture --test-threads=1`
- `cargo test -p canic-tests --test root_hierarchy -- --nocapture --test-threads=1`
- `cargo test -p canic-core --test pic_role_attestation -- --nocapture --test-threads=1`
- any additional targeted flow tests needed by the scenario matrix

Required capture artifacts:

- `scenario-manifest.json`
- `perf-rows.tsv` or `perf-rows.json`
- `flow-checkpoints.log`
- `verification-readout.md`
- `method.json`
- `environment.json`

The report may additionally attach raw current-transport metrics responses, but
those are supporting artifacts, not the audit authority.

## Comparability Rules

Two runs are comparable only if all of these hold:

- same method tag
- same scenario identity tuple
- same checkpoint names and placement for `perf!` flows

If any item changes, mark the delta `N/A (method change)`.

### Method Change Triggers

The method tag must change when any of these change:

- metrics transport shape changed in a way that affects normalization
- canonical subject labels changed
- checkpoint names or placement changed
- scenario harness changed
- freshness/isolation model changed
- topology or default canister scope changed
- replay/cache preconditions changed
- risk-score rubric changed

When the method tag changes:

- add a `Method Changes` section to the report
- mark affected deltas as `N/A (method change)`
- keep at least one unchanged anchor metric where possible

## Failure Classification

Use these classifications when results move:

- `PASS`: stable or improved, with coverage intact
- `PARTIAL`: data captured but checkpoint coverage or comparability is incomplete
- `FAIL`: material regression, missing required evidence, or hotspot growth
  without explanation

## Follow-Up Expectations

If the report identifies a hotspot or regression:

- name the endpoint/flow
- name the owning module(s)
- state whether the issue is shared-runtime or role-specific
- propose the next investigation target

Examples of acceptable follow-up actions:

- add `perf!` checkpoints to an uncheckpointed critical flow
- split a hotspot module so perf attribution is less ambiguous
- reduce repeated storage scans or repeated DTO assembly in one request path
- narrow a scenario matrix where a boundary case is dominating regressions
