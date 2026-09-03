# Architecture Consolidation Audit Update - 2026-09-03

## Verdict

The architecture still makes sense at source commit
`6cad3dcc568e9309f6294d324cc97d0b75c31008`.

No duplicate mutable Fleet authority was found. The 0.110.6 candidate removes
the obsolete host Fleet catalog and derives operator status from validated
terminal Fleet Ensure snapshots. The remaining duplication is supporting
machinery around the maintained authorities, not evidence that Canic needs a
redesign.

The highest-value consolidation target is the release-validation shadow
specification. Component allocation mechanics and Fleet Ensure test platforms
are the next material drift risks. Smaller repeated CLI, controller-set and
path mechanics are valid bounded cleanup candidates. Narrow role-specific
Candid fragments should remain narrow unless a generated or conformance-checked
replacement proves equal wire behavior and acceptable Wasm reachability.

## Evidence Boundary

- Source anchor: `6cad3dcc568e9309f6294d324cc97d0b75c31008`.
- Repository state observed by the review: clean and five commits ahead of
  `origin/main`.
- Review mode: read-only source and structure inspection.
- Validation: no tests, builds, release gate or PocketIC suite was run for this
  review. This is an architecture update, not a release or closeout verdict.
- This document records the review; it does not authorize implementation,
  versioning, publication or a change to the active 0.110 B1 boundary.

Line numbers below identify the anchored source. Function and module ownership,
not mutable line positions, is the durable reference.

## Reconciliation With The Accepted Audit Direction

| Earlier concern | State at the source anchor | Audit disposition |
| --- | --- | --- |
| Fleet status authority | The orphaned host catalog is hard-cut; status enumerates validated terminal Fleet Ensure snapshots. | Closed in the 0.110.6 candidate. Do not reconnect a second writer or catalog. |
| Backup/state/journal ordering | Backup explicitly selects last-converged inventory. Apply publishes a nonterminal journal before effect-owned state and terminal validated state before the matching `Converged` journal. | Closed in the 0.110.6 candidate with crash-boundary evidence. Preserve write ordering. |
| Role-surface contraction | The active B1 inventories, immutable baseline and controlled-ablation work remain in progress. | Continue B1; do not mix the consolidation candidates below into its measurements. |
| Store GC ownership | Root remains the cross-canister publication coordinator; Store-local GC semantics belong to the Store state machine. | Retain as deferred design input. A future endpoint may schedule execution but must delegate semantic advancement to one Store-local operation. |
| Repeated Root validation context | A synchronous validation/projection spine may still reduce repetition, but a context retained across an inter-canister `await` can become stale. | Retain as a deferred, separately measured pilot. Revalidate after every effect boundary. |
| Visibility and documentation cleanup | Still useful after ownership changes settle. | Perform opportunistically after code consolidation, not as an independent architecture rewrite. |

## Findings

### 1. High - Release Validation Has A Shadow Specification

`scripts/ci/check-release-integrity-contract.sh` is 1,145 lines, compared with
473 lines in the root `Makefile`. Beginning around line 231, the guard parses
Make recipes and runner source, then asserts exact target names, barrier counts,
commands and shell fragments. The executable validation authority already lives
in the `Makefile` targets and their runners.

This is the clearest brittle duplicate flow. A behavior-preserving refactor of
the Makefile or a validation runner can fail the meta-check because its textual
shape changed, even when the validation graph did not.

Direction:

- define one machine-readable validation graph containing stable target IDs,
  barrier ordering, execution owner and release eligibility;
- have Make execution and structural verification consume that graph;
- keep executable semantic guards at their owning scripts rather than encoding
  their implementation text in the manifest; and
- validate exact behavior, identifiers and graph edges, not prose, shell
  fragments or incidental recipe layout.

The manifest must not become a second list maintained beside the Makefile. One
source must generate or drive both execution and conformance checks.

### 2. High - Top-Level And Child Allocation Repeat Transition Mechanics

The following owners repeat closely related creation, installation, intent
renewal, completion and verification phases:

- `ops/component_registry/top_level_allocation/mod.rs`: 397 lines, with the
  creation path beginning around line 151;
- `ops/component_registry/child_allocation/mod.rs`: 529 lines, with the child
  creation path beginning around line 233; and
- `workflow/component_registry/component_installation/mod.rs`: 988 lines,
  containing parallel top-level and child effect/reconciliation paths.

The duplication is mechanical, not authoritative. Child partition capacity,
parent binding, descendant counts and child-specific records must remain
distinct from top-level reservation and Component Group authority.

Direction: pilot one typed internal transition kernel for the mechanics shared
by both paths. It may classify the current phase, own intent renewal and express
the permitted next transition, while caller-supplied typed operations retain
record loading, authority validation, capacity accounting and commits.

Acceptance must prove both paths independently, including lost create/install
responses, exact retry, conflicting identity rejection, terminal replay and the
Prepared-versus-Active child scheduling distinction. Measure source shape,
optimized Wasm bytes and defined-function count; source-line reduction alone is
not evidence of a successful consolidation.

### 3. Medium - Fleet Ensure Test Platforms Are Fragmented

Four separate test implementations sit beside the production
`IcpEnsurePlatform`:

- `MockPlatform` in `fleet_ensure/tests.rs`, around line 523;
- `PocketPlatform` in the same file, around line 3979;
- `VersionlessPlanningPlatform` in `fleet_ensure/generate/tests.rs`, around
  line 3488; and
- `RetainedEnsurePlatform` in that file, around line 3736.

The two main test files total 10,235 lines. Each fixture has legitimate special
cases, but separately evolving observation, effect, replay and failure behavior
creates a high probability that one proof models a different platform contract
from another.

Direction: introduce one scriptable test platform with typed steps for live
observations, issued effects, lost responses, versionless responses, durable
progress and injected failures. It should retain an exact effect ledger and
reject unconsumed or unexpected steps.

Do not replace the production-adapter boundary or PocketIC journeys with the
scripted platform. Keep small dedicated wrappers only where the transport or
replica behavior itself is the subject of the proof.

### 4. Medium - CLI Worker Fan-Out Is Repeated

Cycles, metrics and live list each assemble their own worker collection:

- cycles spawns at line 98 and joins around line 125;
- metrics spawns at line 96 and joins around line 110; and
- live list spawns at line 207 and joins around line 212.

All three clone immutable context, spawn one worker per selected entry, preserve
row identity, join in deterministic collection order and convert a panic into a
command-local fallback row or value.

Direction: one private CLI fan-out primitive may own spawning, ordered joining
and panic capture. Each command must continue to own selection, query logic,
typed errors and output rendering. The helper should not introduce a common
domain result enum.

### 5. Medium - Controller-Set Normalization Is Repeated

Controller names and Principals are resolved, sorted and deduplicated in three
main stages:

- pure planning policy at `fleet_ensure/policy/mod.rs:1198`;
- live platform resolution at `fleet_ensure/ops/platform.rs:618`; and
- Canic init construction at `fleet_ensure/ops/canic_init/mod.rs:498`.

The stages consume different evidence and produce different failure meanings.
Those distinctions are safety boundaries and must not be hidden behind one
fallible resolver.

Direction: introduce a named canonical controller-set value that owns only
ordering, deduplication and exact equality. Keep stage-specific constructors or
conversion functions responsible for resolving configured names, current
Principals and missing-evidence errors.

### 6. Low - Root-Relative Path Resolution Is Repeated

The same absolute-or-root-relative projection occurs in:

- `fleet_ensure/ops/current_inventory/mod.rs:1799`;
- `fleet_ensure/ops/current_protocol/mod.rs:2230`;
- `fleet_ensure/ops/platform.rs:2971`; and
- inline in `fleet_ensure/ops/mod.rs:521`.

Direction: move this exact projection to one private Fleet Ensure host-path
helper. File-kind, existence, no-follow, hashing and domain-specific error
checks remain at their call sites.

### 7. Low - Keep Narrow Candid Fragments, Add Conformance

Root and Store request/response fragments recur in host observability, Fleet
Ensure protocol transport, cycles support and internal test adapters. They are
deliberately narrower than the complete protocol enums, which protects
role-specific reachability and Wasm size.

A shared whole-world enum would be a regression. The lower-risk first step is a
wire-conformance test proving each retained fragment's variant labels and
payload types against the canonical protocol contract. Generation is acceptable
only if it still emits narrow role/consumer-specific fragments and its Wasm and
function-count impact is measured.

## Boundaries That Must Remain Separate

The review found no basis to merge these repetitions:

- planning, pre-effect validation and post-effect verification;
- pure policy decisions and live ops verification;
- source-side and destination-side cycle-transfer evidence;
- stable records and DTO/view projections;
- terminal Fleet inventory and protocol-result validation;
- Coordinator policy, Root distribution and Component-local admission
  projections; or
- independent host validation of canister-provided evidence.

These are fault-containment and exact-retry boundaries. Reducing their source
similarity would weaken the architecture.

## Sequencing

The active 0.110 B1 audit remains first and unchanged. After B1 is complete and
accepted, the recommended consolidation order is:

1. replace the release-validation shadow specification with one executable
   validation manifest;
2. consolidate Fleet Ensure test platforms;
3. pilot the shared Component allocation transition kernel;
4. consolidate CLI fan-out and Fleet Ensure path utilities; and
5. introduce canonical controller-set normalization.

Add Candid fragment conformance alongside the nearest protocol-surface work.
Store-local GC ownership and the await-safe Root validation-context pilot remain
deferred inputs; this update does not supply enough evidence to insert either
into the five-step sequence.

Each implementation batch needs its own focused positive, invalid-path and
recovery evidence. Wasm-producing changes additionally require role-appropriate
optimized byte and defined-function comparison before acceptance.
