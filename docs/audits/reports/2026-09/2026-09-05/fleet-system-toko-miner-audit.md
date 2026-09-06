# Fleet system and Toko Miner deployment audit — 2026-09-05

Canic's authority structure is worth retaining. The principal opportunities are
to complete convergence as one bounded operator workflow, avoid rebuilding an
unchanged release, and reduce serialized observation and provisioning work.
A wholesale control-plane rewrite is not supported by this review.

The current downstream launch is incomplete. Its retained local journal contains
29 Applied effects and one Issued effect in runtime activation. The open Canic
corrections have useful focused evidence, but the handoff still requires the
final five-Workload/five-Ready journey and automatic successor-phase completion.
Neither repository's dirty work should be treated as a published correction.

This is a broad source audit of the Fleet path, with retained-byte verification.
It is not a fresh deployment benchmark, exhaustive security assessment, release
validation, or minor closeout. No builds, tests, canister calls, Git publication,
dependency changes, or downstream edits were performed.

## Scope, identity and method

- Canic anchor: `v0.110.7`,
  `2cd3588b639f467e4f536461fcc7cec60af5719e`, plus the observed dirty overlay.
- Toko Miner anchor: `30bf6522fce1c0f37a56b66fe6c0e5185efb546f`, plus its
  observed dirty overlay. The staging estate was not contacted.
- Inventory capture: 2026-09-05 15:14 UTC; 97 dirty Canic paths and 172 dirty
  Toko Miner paths before this report. These are snapshot facts, not ownership
  claims about concurrent changes.
- Primary method: `CANIC-DUPLICATION-001/v1`, manual `code_trace`, with
  additional build, recovery, deployment and downstream boundary inspection.
  The versioned checklist is build → generate → plan → effects → pool
  reconciliation → Store publication → provisioning → activation → terminal
  inventory/conservation → operator/application readiness.
- Reviewer: Codex, single reviewer. No second review or waiver was obtained;
  this report does not ratify a P0/P1 finding or issue a passing closeout.
- Formal method `run_result: partial`; `result_validity: valid` for the
  stated source findings. Unexamined full-method and execution boundaries
  remain open. Completion of this advisory review does not imply execution of
  every method in the audit catalog.
- Earlier comparison:
  [September 3 consolidation audit](../2026-09-03/architecture-consolidation-audit-update.md).
  Comparison is qualitative: the source and overlay differ, so there is no
  numerical complexity-trend claim.
- Exact commits, committed product-tree hash, lockfile hashes, tools, commands
  and 38 decision-bearing input hashes are in the
  [evidence manifest](artifacts/fleet-system-toko-miner-review-v1.json).
  The manifest fingerprints the read working files; it is not an immutable
  snapshot of the entire dirty repositories.

The review covers host/CLI generation and reconciliation, build provenance,
Coordinator/Root/Store boundaries, pool supply and funding, top-level and child
provisioning, runtime release fencing, terminal inventory, the backup inventory
selection boundary, Toko Miner's wrappers, configuration and application
readiness. It does not re-audit cryptographic algorithms, every endpoint,
backup cryptography, every restore/retirement branch, dependency advisories,
all CI jobs, frontend security, or current mainnet state.

## What the actual Toko Miner workload requires

The current App configuration has User Hub, Game Hub and Translation, eight
initial User Shards, and eight initial Game Shards. That means:

| Quantity | Current local shape |
| --- | ---: |
| Distinct application Wasms | 5 |
| Infrastructure Wasms | 3 |
| Initial application Workloads | 19 |
| Independent Ready reserve | 5 |
| Pool assets required at terminal bootstrap | 24 |
| Infrastructure canisters | 3 |
| Total fresh controlled canisters | 27 |

Eight Wasm artifacts therefore do not mean eight creation/install operations.
A fresh estate needs 27 controlled identities; 22 of them host infrastructure
or application code, and five remain Ready pool assets.

Evidence:
[Toko Miner App configuration](../../../../../../toko-miner/apps/toko_miner/canic.toml)
and [local policy generator](../../../../../../toko-miner/scripts/dev/local-dev.sh).

The staging policy is different and currently inconsistent with that App:
`maximum_component_instances = 5`, pool maximum 8, and Ready minimum 4.
The same nineteen-Workload App would need at least 23 pool slots with that
reserve. This is a confirmed configuration mismatch, not evidence of a live
mainnet incident. Align or explicitly select the intended staging topology
before attempting deployment. Do not silently raise spend limits.

Evidence:
[staging policy](../../../../../../toko-miner/deployments/toko-miner-staging-001.toml).

Reducing initial shards is a possible product decision, not a Canic fix. One
initial shard per Hub would yield five Workloads; keeping five Ready assets
would require ten pool assets plus infrastructure. It removes fourteen initial
Workloads, but must not be used to conceal the nineteen-Workload correctness
requirement. Local autonomous refill is deliberately disabled, so a smaller
local estate also has less growth capacity unless a host-owned supply plan
provides more assets. Keep the accepted large configuration as a qualification
case even if an explicitly selected small development configuration is added.

## Evidence about time and artifacts

| Evidence | Observation | What it establishes |
| --- | --- | --- |
| Downstream fast build `6b2bbd1a…` | 63.27 s total; 46.50 s configured application batch | Retained operator timing, not remeasured here |
| Downstream fast build `2727bc54…` | 62.30 s reported; eight artifact pairs verified here | Actual retained bytes and incomplete deployment identity |
| Earlier two-Workload/three-Ready host journey | 9m57s total in handoff | Correctness evidence for a smaller fixture |
| Comparable five-plus-five infrastructure phases | 3m47s → 3m23s | Reported phase improvement |
| Comparable five-plus-five import phases | 4m40s → 2m35s | Reported phase improvement |
| Combined two comparable phases | 8m27s → 5m58s, about 29.4% lower | Not an end-to-end deployment improvement |

The timing source is the
[current handoff](../../../../status/current.md) and the
[downstream feedback ledger](../../../../../../toko-miner/docs/upstream/canic.md).
The comparable five-plus-five timing runs did not finish provisioning,
terminal conservation and replay. Their durations cannot be combined with a
different topology or downstream build to invent one measured startup total.

During final review the other session updated the handoff: a subsequent run
completed infrastructure in 3m37s and imports in 2m32s but still failed Root
activation. A focused four-Shard reconstruction/activation/replay correction
then passed in 178.94s including artifact builds. The complete five-plus-five
production-host rerun remains required. These newer results are reported
evidence, not executions performed by this audit.

For retained build
`2727bc5436519db3c58114680a6d443bb33e6f5e80e1a59e974d52d95eb4cd87`,
this review read all eight raw and gzip files, checked both SHA-256 values
against their manifests, and checked that decompression reproduces the raw
Wasm. Total gzip bytes are 13,326,823; the five application artifacts sum to
8,959,986 bytes. These are fast-profile artifact sizes, not optimized
code-section measurements or a Wasm-limit verdict.

| Role | Raw bytes | Gzip bytes |
| --- | ---: | ---: |
| Coordinator | 4,620,341 | 1,170,425 |
| Root | 9,125,552 | 2,334,681 |
| Store | 3,261,401 | 861,731 |
| Game Hub | 3,586,689 | 928,326 |
| Game Shard | 7,458,750 | 2,160,879 |
| Translation | 7,098,824 | 2,007,778 |
| User Hub | 6,850,165 | 1,986,696 |
| User Shard | 6,413,787 | 1,876,307 |

The Store compiler already deduplicates by role. It uploads each selected
artifact once per Root's Store, not once per shard. At the current 1 MiB chunk
size these five application payloads require ten chunks, plus the separate
release-set manifest. More shards primarily add creation, installation,
Directory and readiness work; they do not multiply the host artifact upload
by the shard count.

## Findings and recommendations

### 1. High priority: complete one bounded convergence workflow

**Existing owner:** CANIC-138; host Fleet Ensure and CLI.
**Confidence:** confirmed source shape and retained downstream failure.
**Disposition:** duplicate of the active correction, with the acceptance
requirements below; do not open a competing implementation.

[CLI dispatch](../../../../../crates/canic-cli/src/fleet/mod.rs) invokes either
one plan or one apply. The
[platform compiler](../../../../../crates/canic-host/src/fleet_ensure/ops/platform.rs)
first waits for current infrastructure, then returns pool reconciliation
actions, then compiles protocol work. Toko Miner's wrapper loops through up
to twenty plan/apply passes and still contains a fresh-import maintenance
bridge. Normal prerequisite completion therefore leaks into application
orchestration and can look like a failed deployment.

Finish a Canic-owned bounded driver over the existing immutable plans and
journals. Every successor must remain bound to the selected desired identity,
network, operator, permitted effect classes, and a cumulative debit ceiling.
An approval for one plan hash must not silently authorize arbitrary newly
observed spending or a changed topology. If the reviewed authority cannot
cover a successor, emit a typed review-required result.

Expose prerequisite completion, awaiting progress, funding pause,
review-required and terminal completion as distinct structured outcomes.
Preserve exact interruption recovery at every phase transition. Remove the
downstream bridge only after released behavior and downstream replay prove it
unnecessary.

Acceptance: one supported invocation completes the selected nineteen-plus-five
fresh shape, resumes after lost responses/process death, preserves all
controlled cycles, and immediately replays with zero repeated deployment
effects. Five-plus-five remains useful narrower regression evidence.

### 2. High value: whole-release cache lookup must precede new identity allocation

**Existing owner:** CANIC-139.
**Confidence:** confirmed implementation gap.
**Disposition:** separate follow-up from the active correctness batch.

[Build dispatch](../../../../../crates/canic-cli/src/build.rs) unconditionally
calls `plan_release_build_for_profile_and_network` for a complete App build.
[Release planning](../../../../../crates/canic-host/src/release_build/mod.rs)
allocates a random nonce. Returning an exact previously finalized release
before that allocation is the safest first cache increment: it retains the
same release ID, manifests, Wasms and runtime authority.

Do not use existing `dirty_summary_digest` as a source-content key:
[source provenance](../../../../../crates/canic-host/src/build_provenance/source.rs)
hashes Git porcelain status bytes. Two edits to the same already-modified file
can produce identical status bytes while changing its content. That digest
describes worktree status, not complete artifact inputs.

An exact cache must account for source bytes, lock/dependency closure, build
scripts and included data, features, configuration, target, network, compiler,
Cargo configuration/environment, tool identities, generated declarations and
protocol inputs. Unknown or untracked build inputs should conservatively miss.
Verify retained manifest and artifact bytes on a hit.

Acceptance: unchanged complete build performs no Cargo compilation, link or
artifact finalization; a content change in an already-dirty file invalidates
the cache; corrupted artifacts fail validation; network/feature/config/tool
changes cannot hit the wrong release. Report the cache decision and hashing
time. The observed approximately one-minute build is the relevant opportunity,
not a promised one-minute saving in every environment.

### 3. Important CANIC-139 design constraint: per-role reuse conflicts with release identity

**Owner:** release-build/runtime authority, contributing to CANIC-139.
**Confidence:** confirmed exact checks.
**Disposition:** settle before promising incremental composition.

This is deeper than missing caching.
[`start!`](../../../../../crates/canic/src/macros/start.rs) embeds
`CANIC_RELEASE_BUILD_ID`.
[`prepare_nonroot_install`](../../../../../crates/canic-core/src/model/fleet_activation/mod.rs)
rejects a supplied ID different from the embedded ID.
[Same-release restoration](../../../../../crates/canic-core/src/ops/storage/fleet_activation/mod.rs)
also checks retained versus embedded identity, and release-set admission checks
all entries against the complete release identity.

Consequently, copying an unchanged old role into a newly identified complete
release cannot satisfy the current contract merely by rewriting its manifest.
Changing the embedded ID changes the Wasm bytes, too.

Split the objective: implement exact whole-release reuse first. Treat reuse of
role artifacts across newly composed releases as a separately reviewed current
contract change, potentially separating artifact content identity from release
composition authority. Preserve exact manifest/module binding, initialization
checks and pre-1.0 reinstall-only semantics. This is not a recommendation for
mixed-version operation, live state preservation, or bypassing the fence.
A smaller intermediate goal is reusing compilation intermediates while still
producing correctly bound final Wasms.

### 4. High value: remove declaration linking from the release critical path

**Existing owner:** CANIC-087.
**Confidence:** confirmed source structure; speedup unmeasured.
**Disposition:** retain existing owner and B1 sequencing.

[Configured artifact building](../../../../../crates/canic-host/src/canister_build/artifact.rs)
uses the selected build profile for declaration production, derives each
protocol digest, then invokes runtime Cargo separately for every role.
For local builds, the first pass can be a workspace batch. For nonlocal
builds, declaration builds themselves currently iterate per role. The final
artifact transforms already run in scoped threads; recommending parallel
finalization alone would repeat existing work.

The release profile still selects fat LTO and one codegen unit. First use a
cheaper declaration pass with exact feature/network/Candid parity, and consume
canonical built-in Candid where its authority is sufficient. Then replace the
single process-wide role digest input with exact per-package build context so
compatible runtime roles can share one Cargo invocation. Do not start
competing Cargo processes against the same artifact target.

Only then benchmark ThinLTO/codegen-unit changes against optimized Wasm,
instructions, determinism and protocol parity. Cargo documents the compile
time/runtime tradeoff; it is not evidence of a measured Canic improvement.
[Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html#lto).

Earlier CANIC-087 counts describe an older role/helper topology. The current
review must use eight artifacts and the current Binaryen 132 authority, not
repeat the removed helper or historical Binaryen 108 configuration.

### 5. High value: measure and reduce serialized transport and observation

**Owner:** host Fleet Ensure/ICP transport; snapshot reuse overlaps CANIC-138.
**Confidence:** confirmed serial paths; exact cost attribution unmeasured.
**Disposition:** follow the active snapshot correction with a bounded pilot.

[`observe_configured_canisters`](../../../../../crates/canic-host/src/fleet_ensure/ops/platform.rs)
collects synchronous observations in sequence.
[Typed Candid transport](../../../../../crates/canic-host/src/icp/candid.rs)
writes and syncs a temporary argument file and launches ICP CLI for each call.
Root-owned balance inspection is a protected Root command, so an apparently
read-only observation can itself require replicated work.

The dirty overlay already reuses status and pool pages within one observation
scope and explicitly discards the scope before an effect. Preserve that
boundary. Next, count calls by target/method/phase and measure subprocess,
network wait, polling sleep, codec and durable-write time separately.

A bounded worker pool for independent observations is a lower-risk first
concurrency experiment. A bounded Root inspection batch or persistent host
transport may follow if measured call startup/round trips dominate. The Root
must still inspect every exact target; batching does not eliminate management
work. Retain network trust, identity selection, response verification and exact
error/retry semantics. A snapshot must not be reused across mutations.

Do not replace authoritative inspection with public query data. ICP documents
different execution and verification guarantees for queries and updates.
[Client call semantics](https://docs.internetcomputer.org/guides/canister-calls/calling-from-clients/#query-vs-update-calls).

### 6. Medium-term: pipeline only dependency-independent effects

**Owner:** host action scheduler, Store publication and Root provisioning.
**Confidence:** serial structure confirmed; benefit and safe windows require proof.
**Disposition:** design candidate after correctness and instrumentation.

The [host action loop](../../../../../crates/canic-host/src/fleet_ensure/workflow/mod.rs)
reconciles each action before advancing. Store chunk publication similarly
performs status/effect/status work through that loop.
[Root provisioning](../../../../../crates/canic-control-plane/src/workflow/component_provisioning.rs)
advances canonical reservation/claim/install/commit/publication/activation
cursors. [Hub bootstrap](../../../../../crates/canic-core/src/workflow/placement/sharding/bootstrap.rs)
awaits each initial shard allocation.

Start with immutable chunk publication after the exact chunk set has been
admitted, or independent targets/Roots. Persist every issued intent and bound
aggregate in-flight debit. Retain a receipt per chunk/target, allow exact
out-of-order completion, and reconcile all uncertain effects after process
death. Current prefix-shaped journals and single in-flight Root cursors cannot
simply be put behind `join_all`.

Within a Root, reserve distinct assets/slots before overlap, serialize shared
Registry commits where needed, and retain readiness/Directory prerequisites.
The Prepared-parent detached bootstrap distinction must survive any change.
IC message execution can interleave across awaits, so revalidate authority at
commit boundaries.
[Inter-canister execution](https://docs.internetcomputer.org/guides/canister-calls/inter-canister-calls/).

### 7. High value: qualify the generator-to-ready path, not only a hand-built desired Fleet

**Owner:** Fleet generation and production-adapter qualification; CANIC-138 evidence.
**Confidence:** confirmed test construction.
**Disposition:** extend the existing journey, not a second fixture framework.

The large
[`literal_zero_fleet_with_initial_children_reaches_effect_free_terminal_replay` test](../../../../../crates/canic-testing-internal/src/pic/fleet_registry/baseline.rs)
uses real Wasms and `IcpEnsurePlatform`, but constructs
`DesiredFleetBootstrap`, imports and `DesiredFleet` in the test. It currently
sets five initial Workloads and five Ready assets directly. That is meaningful
apply/recovery evidence, but it bypasses the generator that previously selected
insufficient local supply.

Keep narrow generation regressions and add a joined path:
current App configuration → source policy → production generator → exact
retained plan → apply/recovery → full Component tree → application-ready
verification → effect-free replay. Include nineteen-plus-five and impossible
capacity rejected before paid effects. Derive expected topology from registered
configuration and assert semantic membership, parent bindings and reserve
requirements, rather than maintaining an unrelated aggregate test count.

The test transport also uses a direct local replica target and a configurable
observation delay. Its timing is not automatically the timing of Toko Miner's
normal ICP CLI process path.

### 8. Medium: distinguish infrastructure readiness from application usability

**Owner:** Toko Miner application startup and local wrapper; Canic generic status.
**Confidence:** confirmed separate gates.
**Disposition:** downstream follow-up only.

[Game Shard startup](../../../../../../toko-miner/apps/toko_miner/game_shard/src/startup.rs)
waits for IcyDB, then reconciles seven fixture families in a watchdog callback.
The wrapper performs catalog and public-metrics verification after Fleet
convergence. Fleet Active therefore does not by itself establish that all game
fixtures and application checks are complete.

Measure time to infrastructure ready, Fleet terminal, and first usable game
request separately. Expose typed application readiness and inspect all required
initial shards in qualification. Keep database fixture ownership downstream;
Canic should carry a generic readiness projection, not learn Toko Miner's
catalogs. If fixture reconciliation becomes expensive, measure and bound its
work before increasing initial shard count or retry frequency.

Toko Miner's current accepted access design deliberately separates public
enrolment/application membership from Fleet operator policy. Do not infer an
admission bypass solely from its raw `ic_cdk` endpoints or reapply an old
allowlist model. Test the selected application contract and Canic's protected
control-plane boundaries independently.

### 9. Medium: simplify transition ownership and qualification machinery

**Owner:** existing architecture consolidation direction.
**Confidence:** confirmed source concentration; not a new mutable Fleet authority.
**Disposition:** deferred behind accepted B1 and current corrections.

Current file sizes include host workflow 4,560 lines, host platform 4,767,
Component Registry ops 6,468, Component Registry workflow 6,298 and the Fleet
PocketIC baseline 10,245. Source size is a maintenance signal, not proof of
runtime cost.

The host workflow also directly constructs and mutates `FleetEnsureJournalRecord`,
`EffectRecord`, topology and funding-pause records. Root AGENTS assigns record
construction/mutation to ops/model. Move those transitions into named atomic
ops that enforce intent/issued/applied and publication ordering; leave the
workflow responsible for sequencing. Do not achieve this merely by moving
blocks into arbitrarily named files.

Retain the previous recommendations for a shared typed allocation transition
kernel and scriptable host test platform, while preserving top-level versus
child authority and real production-adapter journeys. The release integrity
guard still parses substantial Make/shell source shape; one executable
validation manifest remains preferable to a second handwritten specification.

The current layering method is narrower than root AGENTS and still presents a
linear workflow→policy→ops diagram. Root AGENTS governs: workflow may call
policy and ops independently, and policy never calls ops. This method drift
should be corrected separately; it cannot exempt a forbidden dependency or
supply a passing whole-repository layering verdict.

## Safety and ownership worth preserving

The sampled Coordinator macro authenticates protected command variants before
dispatch; operation and Registry reads retain caller-specific checks. Root
pool creation owns durable exact creation authority and an explicit network
boundary. Host apply retains intent before effects, receipt reconciliation,
bounded paid work and terminal conservation. The dirty terminal inventory
requires actual running state, exact Root controllers and current module.

Backup creation explicitly selects last-converged Fleet inventory. Keep that
separate from an in-progress observation. Similarly, the following repetitions
are useful fault boundaries: pre-effect validation versus post-effect proof,
Root evidence versus independent host checks, source versus destination cycle
accounting, and canonical admission policy versus each runtime's local
projection. Do not collapse them for source-line savings.

The current pre-1.0 contract permits reinstall-only release transitions. Fast
deployment must not introduce state-preserving cross-release upgrades,
predecessor adapters, mixed-version operation or removal of cycle-conservation
checks. Same-operation retry and same-release restore remain required.

## Proposed order and acceptance measurements

This is advice, not authorization to expand the active batch.

1. Finish the other session's accepted correctness batch and the missing joined
   convergence evidence. Keep CANIC-007/132–138 with their existing owners.
2. Reconcile Toko Miner's intended staging topology and limits downstream.
3. Add phase/call timing alongside the bounded successor driver; record complete
   successful local results for the actual nineteen-plus-five topology.
4. Implement exact whole-release reuse under CANIC-139, including source-content
   invalidation and corrupted-cache rejection.
5. Remove release-LTO declaration work and batch compatible runtime compilation
   under CANIC-087, respecting the accepted B1 boundary.
6. Pilot bounded observation concurrency, then immutable upload concurrency.
7. Reconsider artifact/release identity separation and Root provisioning
   pipelining only with measured benefit and a separately accepted contract.
8. Continue capability-owned runtime contraction and supporting consolidation
   under their existing design gates.

For each optimization, retain cold build, unchanged build, one-role source
change, fresh local deployment, interrupted/resumed deployment, and terminal
replay measurements separately. Include source/tool/network identities,
topology, phase durations, query/update/subprocess counts, artifact bytes,
maximum in-flight work, debit and result. Use repeated comparable runs; report
median and range, and collect enough samples before making percentile claims.

Do not optimize poll intervals first. The existing 250 ms to 5 s backoff
protects a distributed operation from being mistaken for a stall. A compact
durable progress cursor plus elapsed-time/call attribution will show whether
the wait is idle, useful replicated work, contention, or a true failure.

No speedup factor is justified yet. Exact release reuse can remove repeated
build work; scoped observation reuse already has partial phase evidence.
Whether the complete nineteen-Workload startup can be cut by half requires a
successful comparable baseline and the measurements above.

## Verification and handoff

Performed: source/contract trace; parsed retained journal and plan; verified
all eight retained raw/gzip artifact pairs; recomputed committed product-tree
identity and recorded working-input hashes; inspected targeted test source and
existing reported results. No tests were executed in this session.

The completion recheck found all 37 other fingerprinted inputs unchanged;
only the concurrently maintained Canic handoff changed. Its relevant update
was reread and incorporated above. Both handoff hashes are retained in the
evidence manifest. This report and its index/evidence files are the only
files written by this audit session.

The original inventory command referenced a removed `fleets/` directory and
failed; the corrected read-only inventory used existing `crates/`,
`canisters/` and `scripts/` roots: 1,562 selected source files, 463,385 physical
lines, and 149,584 Rust lines across host/CLI/backup. These counters do not
measure capability, security, performance or release readiness. Full-method
backup/restore duplication scoring was not performed; no aggregate DRY score
is issued from partial coverage.

The complete open correction batch remains not push-ready or publish-ready
according to the observed handoff. The unrun broad suite is not the reason:
required final convergence/recovery evidence and successor handling remain
unfinished. No version or changelog surface was changed by this audit.

Existing CANIC-xx findings remain in their ledger; this report contributes
analysis and acceptance criteria rather than closing or renumbering them.
New staging/configuration and identity-design observations require owner
triage. No downstream file or running estate was modified.
