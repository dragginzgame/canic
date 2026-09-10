# Canic 0.110 Implementation Status

## Validation throughput before push

The maintainer's pre-push VS1 outcome is qualified at the recorded ic-memory
0.13.1 checkpoint. BF2-BF4 and the other feedback corrections retain their scope.
Published IcyDB 0.257.4 now shares ic-memory 0.13.2, so the maintainer requested
adoption and CANIC-162 is restored. Fresh aligned lifecycle/allocation PocketIC
proofs, eight memory cases, three ABI guards, 15 timer guards, the catalogue
regression and affected-target warning-denied Clippy pass. The complete scoped
.14 batch and changelog are ready for release review; package versions remain
0.110.13.

| Batch | Outcome | Owner | Included evidence | Validation | Status |
| --- | --- | --- | --- | --- | --- |
| VS1 | Reduce dominant release-test work while preserving distributed invariants | Internal fixture/runner and host observation owners | Phase attribution, exact artifact reuse, native CLI selection, authority/retry/conservation and final-source timing | Both dominant exact PocketIC journeys, 23 host cases, timing/catalogue regressions, scoped lint and runner guards | Qualified; same open 0.110.14; complete release gate remains maintainer-directed |

The [final report](../../audits/working/0.110-validation-throughput/report.md)
records 1,153.72s for mixed topology plus two resets and 575.12s for retained-estate
recovery. Both run with an unchanged 1,686-file source inventory. All 144 duplicate
balance calls disappear; fresh authority, recovery and replay checks remain.
Native regressions and warning-denied all-target/all-feature host/internal-fixture
Clippy pass. Both test invocations clean their owned servers and scratch.

The [VS1 amendment](0.110-design.md#validation-throughput-amendment-vs1)
retains serial Cargo and PocketIC ownership. Existing verified artifact reuse
already works across invocation roots; sub-second replica setup does not justify
new mutable fixture pooling or catalogue splitting. This measured disposition
completes the artifact/setup slice without a speculative runtime-identity change.

The [historical baseline](../../audits/working/0.110-validation-throughput/baseline.json)
records a 6,322s older runner. Earlier attribution and final runs differ in
memory dependencies and cache warmth, so no isolated whole-journey percentage,
new complete-gate duration or sub-hour result is claimed. The open changelog
records the completed scope; no version or publication ran.

## Build feedback 087 and 139

The maintainer accepted these build-throughput follow-ups on 2026-09-09.
BF1 shipped in 0.110.13. The host build owner now separates
declarations from runtime linking, batches package-bound protocol contexts and
verifies complete unchanged-build reuse before allocating a release identity.
Changed inputs still rebuild the runtime set because every artifact embeds the
complete release identity; cross-identity role reuse is outside this scope.

| Batch | Outcome | Owner | Included evidence | Validation | Status |
| --- | --- | --- | --- | --- | --- |
| BF1 | Faster governed builds and verified no-op reuse | Host artifact/release owners and build macro | Input/output invalidation, exact role binding, Candid/features, managed lifecycle and measured cost | Focused native checks, controlled release comparison and scoped PocketIC evidence | Complete within recorded scope |
| BF2 | Overlap infrastructure finalization with subsequent compilation | Host artifact owner and CLI build orchestration | Exact captured inputs, output/provenance parity, failed-build cleanup and valid retry | 53 host cases, 25 CLI cases, affected-package Clippy and controlled two-artifact comparison | Complete; open 0.110.14 |
| BF3 | Admit unchanged first builds without confusing Cargo inventory changes with source drift | Host build reuse and generated-package owners | Infrastructure source coverage before compilation, stale-record replacement, real source changes and unknown-input refusal | 58 combined focused host cases and affected-package Clippy | Complete; same open 0.110.14 |
| BF4 | Reuse unchanged compiled declaration extraction across backend edits | Host artifact and Candid extraction owners | Role/shared-input changes, corrupted records, exact native tool identity, failed extraction and real-tool byte parity | Focused build regressions, six-role extraction comparison and affected-package Clippy | Complete within recorded scope; same open 0.110.14 |

The maintainer selected changed-input reuse on 2026-09-10. Every runtime's
embedded complete release identity is itself a changed dependency, so BF4
preserves that contract and reuses the independent declaration extraction
stage. Cargo remains the dependency owner; there is no second source inventory
or per-role cache claiming runtime reuse across identities. A six-role real-tool
comparison reports 1.922s fresh extraction versus 0.087s verified reuse with exact
Candid parity (2.234s first cache population). Complete-build speedup and
cross-identity runtime reuse remain unclaimed.

The [build feedback report](../../audits/reports/2026-09/2026-09-09/build-feedback.md)
owns BF1 qualification and its limits. Its unchanged replay is 2.83s; a warm
one-role edit improves 477.10s to 291.03s; cold time is roughly unchanged.
The managed/standalone lifecycle case passes. Mixed-topology startup, conservation
and replay assertions completed before the extra reset exercises were stopped;
the complete case is not marked passed. Precise per-role/cross-identity reuse,
material cold improvement, full clean determinism and downstream qualification
remain outside this completion claim. Earlier evidence below retains its scope.

After publishing 0.110.13, the maintainer prioritized another bounded build-speed
batch. BF2 retains serial Cargo and existing release settings while captured
Coordinator and Store outputs finalize in scoped workers. The controlled
comparison improves two-artifact wall time from 112.89s to 85.03s (24.68%);
Wasm, gzip, Candid, protocol hashes and transform provenance match exactly.
Configured-admission failure drains workers without staging leftovers, and a
following valid request with the same release identity reproduces the artifacts.
The [scheduling report](../../audits/reports/2026-09/2026-09-09/build-pipeline.md)
owns the evidence and its limits. BF2 and its open 0.110.14 notes are complete;
VS1 is qualified; restored CANIC-162 shares the published IcyDB memory
runtime and passes fresh qualification. The .14 batch is ready for release review.
Package versions remain 0.110.13. No full downstream, broad
workspace or aggregate-memory qualification is claimed. CANIC-087/139 stay
partial and CANIC-141 stays deferred. Continue this maintainer-selected feedback
on the published 0.110 line despite the soft release-count guideline, rather
than allocate a new minor for the bounded follow-up.

The maintainer then selected CANIC-139's first-build refusal for BF3. Current
downstream dependency records name 143 control-plane files outside the initial
App package scan. Reuse now includes the exact generated-infrastructure family
roots before compiling, retains file-level evidence through Cargo record
replacement, and records the verified final inventory key. Real mutations and
unobserved external inputs still fail, with distinct path-specific diagnostics.
The [first-build report](../../audits/reports/2026-09/2026-09-09/build-reuse-first-build.md)
records the tiny real Cargo/Wasm regression and its limits. BF2/BF3 remain one
completed 0.110.14 batch; full downstream qualification and changed-input
per-role reuse remain outside its completion claim.

## Sampling cost reference clarification

On 2026-09-10 the maintainer directed that the unexplained 20M instruction
threshold must not become a future blocker. Canic now reports it as an advisory
reference and keeps structural, relative source-growth and recovery checks.
The [cost policy record](../../audits/reports/2026-09/2026-09-10/metrics-cost-policy.md)
links the maintained evidence requirements for any future absolute threshold.
This is an open 0.110.14 test/documentation correction, with no runtime budget
or protocol change. A matching Toko test patch is prepared but unapplied.

## Additional downstream qualification

CANIC-159's corrected runtime now passes the existing five-component PocketIC
journey with a Store outage during Root Accepted. Exact origin fields reach
Coordinator; same-operation recovery clears failures and immediate replay
requires no updates. The [runtime report](../../audits/reports/2026-09/2026-09-10/canic159-runtime.md)
retains the candidate evidence separately from downstream adoption.

The maintainer's four-item Toko Miner request extends open 0.110.14 with
corrections for Root Accepted-phase provisioning origins, optional public
sampling after rejected producer data, and metric history lookup cost. The
[four-item report](../../audits/reports/2026-09/2026-09-09/toko-feedback.md)
records disposable retained reset/data/replay, a controlled downstream first
build, and an unapplied staging preparation review. The corrected history index
passes the real metrics ceiling through 300 periods and rejection/recovery, with
19,963,567 peak instructions against 20M (0.18% headroom). Downstream runtime
adoption and staging recovery remain explicit acceptance boundaries. The
approved wrapper cleanup is applied. No protocol generation,
compatibility layer, version bump or new minor is introduced.

## Activation feedback 157–160

The maintainer accepted these follow-ups with an explicit sanity and drift
review. Existing Ensure, provisioning, protected-status and observation owners
contain the changes. Root prerequisite identity and dependent Store admission
now reject incompatible reuse; bounded retry state and originating failures
remain in the existing operation. Observation timing and bounded independent
reads retain the existing snapshot lifetime.

Installed-source queries on 2026-09-09 confirm a sealed initial inventory and
all 24 pool assets. Source evidence inspection does not make the old plan
executable or supersede its journal. Refill-intent admission now rechecks full
physical capacity after fee discovery, with 24 targeted pool cases passing.
Lease takeover prevents using one successful maintenance pass as settlement
proof. The single-Root preparation, Root reset and Full Ensure reviews now use
the existing effect driver. Exact source archival and crash recovery precede
the first remote effect; admission binds source modules/controllers, complete
assets, unchanged Ledger accounts and bounded source debit. The immutable
`v0.110.12` PocketIC journey passes controller-drift rejection without source
replacement, stop/restart preparation, lost Root install response recovery,
Full Ensure readiness, exact retained assets, conservation and effect-free
replay. The proof also corrected changed-Wasm history reconciliation by binding
both the prior and requested module. Three history regressions, 167 direct
Fleet Ensure cases and focused host/CLI/fixture Clippy pass.

The [activation feedback report](../../audits/reports/2026-09/2026-09-08/activation-feedback.md)
owns the exact evidence and single-Root applicability boundary. The complete
accepted 0.110.13 source batch and changelog are ready for push/release review;
package versions remain 0.110.12. Live downstream adoption remains separate.
Earlier OD1/RI1/RF2 results retain their completed scope.

## RF2: Accepted upstream follow-ups 151–156

The maintainer accepted all six follow-ups on 2026-09-08, extending the same
0.110.13 batch after OD1/RI1. The launcher is aligned and qualified. Retained
estate omission rejection, affordable continuation reserves/prefixes and typed
dependent-funding review are implemented. The compiled-configuration fixture
boundary and matching-build recipe are explicit. Bounded public process, timer
and memory projections use the existing five-family sampler and history.

The complete accepted OD1/RI1/RF2 batch is ready for push/release review.
Qualification passes: 167 host and 51 CLI cases, 18 all-feature metrics cases,
all eight timer PocketIC cases and the expanded 27-canister changed-release
journey. The latter retains all 24 pool assets, rejects sixteen omitted imports
before Root reset, reviews six low-balance top-ups, recovers lost install and
funding responses, reaches 19 Workloads plus five Ready reserves, checks exact
debit/controllers/conservation, and replays without effects. First/repeat/full
scheduled public sampling remains below 20 million instructions. The open
0.110.13 changelog is ready; package versions remain 0.110.12.

CANIC-157–160 appeared in a later tracker refresh and are accepted separately
above; RF2 does not claim their completion.

## RI1: Explicit Same-Release Fleet Reinstall

Accepted by the maintainer on 2026-09-08 after OD1. CANIC-149 adds one explicit
operation-scoped `fleet ensure --reinstall` intent. Preparation durably seals
Coordinator/Root allocation through the existing authority-snapshot owner;
the following reviewed full plan resets the same-Wasm infrastructure and the
complete sealed Root-owned pool, then reconstructs application fixtures and
full Fleet readiness. Both phases retain the same operation identity. The
network, controlled Principal set and cycle accounts remain controlled;
logical pool role assignments may change. No permanent desired-state flag,
new scheduler, cross-release compatibility or downstream executor is added.

Host policy/ops/workflow, CLI, the existing authority inspection boundary and
governed PocketIC fixtures own one complete batch. Required evidence covers
controller/closure rejection, real Hub/Shard user-data erasure, fixture restore,
interruption before/after effects, a second deliberate wipe, conservation and
effect-free replay. Includes Candid/fixture/docs propagation and cleanup.
Implementation, recovery evidence, propagation and cleanup are complete.
The production-adapter journey passed two real database wipes, Coordinator
before/after-response interruptions, a Root interruption after ordinary version
advancement, retained estate/controller/cycle-account checks, conservation and
both terminal/ordinary-ensure replays. Targeted native/interface checks and
warning-denied affected-package Clippy pass. The combined OD1/RI1 batch is ready
to push and the open 0.110.13 changelog is ready for release approval; package
versions remain 0.110.12. Publication and deployment are separate.

## OD1: Downstream Operator Diagnostics

The 2026-09-08 request to work on Toko Miner upstream feedback selects reopened
CANIC-042 and CANIC-150 as one bounded diagnostic correction. Host role evidence
owns actionable Cargo causes; CLI Medic and Fleet rendering own advice, named
budget terms, compact cycle values and action descriptions. Positive and invalid
evidence includes nested offline failures, source/credential sanitization,
bounded Unicode, pending/unknown funding observations, initial creation versus
Root-funded capacity, reinstall and no-op presentation, and unchanged JSON and
progress behavior. Targeted host/CLI tests and affected-target Clippy own
qualification. OD1 is complete; the maintainer subsequently extended the
0.110.13 batch with RI1 above. Its completed evidence is: five host evidence cases,
thirteen Fleet CLI cases in the combined batch, the Medic advice case and all-target/all-feature Clippy
for both affected packages pass. Downstream live-output adoption remains separate.
Propagation includes the Fleet operations contract, changelogs and
[feedback handoff](../../audits/reports/2026-09/2026-09-08/operator-feedback.md).

Extend the affected 0.110 line with the 0.110.13 draft despite the soft release
count guideline: these correct published diagnostics and complete the accepted
reinstall operator workflow. CANIC-149 is complete as RI1 above. CANIC-141 stays
deferred.

## PM1: Bounded Public Sampling

Complete and ready for release approval after maintainer-pushed 0.110.10.
CANIC-148 corrects optional public
collection through bounded owner projections, independently attempted families
and unchanged cycle-tracking ownership. Includes native bound/identity/failure
regressions, a small IC sampling-cost and cycle-tracking proof, documentation
and the open 0.110.11 changelog. No history task or new scheduler. CANIC-147
is a separate capability request awaiting contract acceptance; CANIC-141 remains
deferred. Native regressions (112 all-feature, 10 default), 17 storage/timer guards,
affected-target Clippy and seven PocketIC cases pass. The focused PocketIC runner
took 96 seconds including artifact rebuilds. Sampling 256 versus 4,096 recorded
checkpoints took 4,449,546 versus 4,647,992 instructions. The
[sampling handoff](../../audits/reports/2026-09/2026-09-08/public-sampling-bounds.md)
records bounds, evidence and downstream qualification. Package versions remain
0.110.10; the open 0.110.11 changelog is ready. Release execution remains
maintainer-selected.

## RF1: Retained Fleet Feedback

Ready for release approval, selected by the maintainer after CR1 on 2026-09-07. Extends the open
0.110.10 draft with CANIC-143, CANIC-144 and the CANIC-125 observation follow-up.
Host policy, transport and the existing journal remain the owners. The outcome
is ordinary convergence of installed underfunded imports, symbolic retained
Root restart and bounded initial-child activation without application retries.
Required evidence: exact lifecycle/controller rejection before funding, retained
withdrawal receipts across lost replies, generated symbolic restart through
reviewed reinstall, conservation/replay, initial-child demand bounds and no
duplicate issuance. Reuse the existing focused production-adapter journeys.
The implementation, propagation and focused qualification are complete:
150 Fleet host tests, affected-package Clippy, installed-import recovery
(310.66s) and generated symbolic-seed reinstall/recovery (393.03s) pass.
Both runtime cases include conservation and effect-free replay. The
[feedback handoff](../../audits/reports/2026-09/2026-09-07/retained-fleet-feedback.md)
records the final boundaries. CANIC-141 remains deferred. The combined CR1/RF1
draft is ready; release execution and downstream live timing remain separate.

## CR1: Canonical Fleet Subnet Root

Ready for release approval, explicitly selected by the maintainer on 2026-09-07
after 0.110.9.
This is the next accepted batch in the current minor, ahead of further downstream
feedback; CANIC-141 remains deferred. The canonical Root build, package rename,
configuration and lifecycle hard cut, fixture/documentation propagation and
focused fresh/reinstall/recovery qualification are one release batch.
Owners: facade/control plane, host build/role contract and test infrastructure.
See [the CR1 contract](0.110-design.md#canonical-fleet-subnet-root-batch-cr1).
The complete hard cut and propagation are implemented in the open 0.110.10
draft. Packaged Root construction, fresh generated Fleet recovery/replay
(720.46s, including cold builds), generated reinstall/recovery/conservation/
replay (496.36s), focused source/interface tests and affected-target Clippy pass.
See [the CR1 handoff](../../audits/reports/2026-09/2026-09-07/canonical-fleet-root.md).
Package versions remain 0.110.9; the maintainer-selected release gate and
publication remain. CR1 does not close the independent runtime-contraction line.


Date: 2026-09-06

## Status

- Authorized correction batch (2026-09-05): resolve `CANIC-007` and
  `CANIC-132` through `CANIC-138` within Canic. Runtime/auth owns synchronous
  selected-store validation and auth-free Root reachability; host/Fleet Ensure
  owns shared policy admission, complete capacity, reviewed native funding and
  pool reconciliation, and management-bound same-module Start. Required
  evidence includes typed rejection before effects, corrupted issuer restore,
  exact enabled/disabled role surfaces, lost responses, process reconstruction,
  terminal cycle conservation and immediate effect-free replay. Status: ready
  for release approval;
  publication and downstream effects remain separate boundaries.
  The focused four-Shard and direct nineteen-Workload activation/replay cases
  pass. Automatic five-Workload/five-Ready convergence passes in 548.96s;
  four-Workload reserve refill passes in 377.41s; the full four-Workload/four-
  Failed repair passes in 349.17s without Root-account funding or creation.
  All 138 focused Fleet host tests and scoped Clippy pass. Seven native auth
  cases, auth-free Root activation, and the controlled Toko Root comparison
  pass; the latter removes 333,288 code-section bytes (5.15%). Named optimized
  symbol evidence remains separate from canonical artifact measurements.
  Generated nineteen-plus-five passes in 990.33s, including all 104 effects,
  terminal conservation and both completed-plan and newly planned effect-free
  replay. The corrected fixture preserves source-relative package authority
  and checks it before host effects.
  CANIC-134 follows the maintainer's 2026-09-06 correction: explicit reinstall
  discards state while preserving cycle and asset control. Full evacuation is
  conditional on deleting the Fleet. The separate custody implementation and
  pinned predecessor owner have been removed. The Root Ledger deletion slice
  passes its lost-reply checkpoint; complete whole-Fleet deletion is optional
  follow-up. Pool-policy/status errors no longer select implicit resets.
  Start retains only exact installed-module authority. The Root reinstall
  prerequisite now binds stop/reinstall/start to exact management authority;
  generation avoids protected calls on a different Root module. All 138 focused
  host cases pass after this change. The generated changed-release PocketIC
  journey passes in 522.04s, including a working Fleet after lost-response
  recovery, conservation and both effect-free replays. Live Toko applicability
  remains downstream qualification. All 47 selected Fleet CLI tests and five
  canonical Coordinator Candid tests pass. The final Coordinator preflight and
  Ledger lost-reply checkpoint passes in 142.74s. The correction batch is ready
  for release approval; the governed release gate and publication remain.
  See the [feedback readiness report](../../audits/reports/2026-09/2026-09-05/fleet-feedback-readiness.md).

- State: B1 promoted and active after accepted 0.109 closeout.
- Review verdict: the frozen replica-validator-equivalent function counter,
  whole generated-surface and repository-owned capability-fixture gates are
  binding. Consumer artifacts remain non-binding routing evidence. Predecessor
  closeout and explicit promotion remain mandatory.
- Outcome: create durable absolute Wasm code-section and replica-limited
  defined-function headroom through storage, codec and whole generated-surface
  endpoint/provider/recovery reachability cuts.
- Capability boundary: the contraction work adds no runtime capability.
  The separately authorized CANIC-134 correction permits explicit hard-cut
  reinstall and cycle-safe Fleet deletion. General estates, host-only version inventory and
  status redesign remain outside this line.
- Provenance: immutable predecessor `3185dc45b` (`v0.109.35`), superseding the older
  tracked 0.110 revision last changed at `ef3acc17c`.
- Compatibility: reinstall-only. Selected codec cuts retain no predecessor
  decoder and require complete destroyed-state/reconstruction evidence.
- Predecessor: 0.109 B8-B10, its immutable complexity evidence and the human
  minor-closeout acceptance are complete.
- Promotion: the human maintainer explicitly promoted B1 on 2026-09-01.
- Successor: 0.111 owns bounded cycle-safe multi-Fleet estates after 0.110
  closeout.
- Downstream: 0.110 cannot delay the published 0.109 Toko Miner unblock.
- First release: immutable `v0.110.0` at `d29a7cc72` carries the initial
  `CANIC-119` through `CANIC-121` fresh-estate correction and corrected
  canonical-role baseline; it does not claim B1 completion or authorize B2.
- Latest release: immutable `v0.110.7` at `2cd3588b6` completes the endpoint,
  validation, terminal-authority, estate-funding, cryptography and retained
  B1 attribution work recorded in the detailed changelog. It does not accept
  B1 or authorize the remaining B2/B3 families.
- Published `v0.110.6` repairs active child
  response completion, terminal Fleet discovery, backup and descendant
  authority, and controller-protects exact observability while preserving
  operator access through Root relays. The earlier public host-only CANIC-124
  managed Component-tree qualification fixture, bounded CANIC-126 Root-owned
  balance re-observation and CANIC-127 Prepared-Root initial-child convergence
  remain current. The
  fixture derives exact `ComponentChild` authority for configured and on-demand
  sharding, scaling and index children. An exact active runtime-configuration
  replay can also reclaim an exhausted transient non-Root init bootstrap without
  rerunning application init or reopening a non-retryable failure. These
  corrections do not promote B2. One governed literal-zero-estate proof drives
  the current plan, durable journal, concrete production adapter, lost-response
  recovery and real Coordinator/Root/Store protocol through terminal cycle
  conservation and immediate zero-effect replay.
- Published `v0.110.6` correction: an Active Component parent receives its newly
  provisioned child ID from the original capability call, while Prepared
  initial bootstrap alone retains detached completion. The managed Component
  Group PocketIC journey proves both branches. This correction does not change
  the immutable `v0.110.5` B1 baseline; candidate measurements include the
  corrected working overlay.
- Published `v0.110.6` terminal-inventory correction: Root pool Workloads are matched to
  the complete protected top-level and descendant Component tree by exact
  Component and allocation-operation authority. Only an observed,
  protocol-bound Pool-to-Component promotion may change logical parent, and
  `info subnets` retains the selected ICP executable and environment. This
  operator correction does not promote B2 or expand runtime capability.
- Published `v0.110.7` hardening: terminal Components additionally require exact
  running state, Root-only controllers and current management/Directory module
  identity; cycle reporting is capability-aware; Fleet Ensure treats every
  Root Cycles Ledger account as a separately observed and reviewed funding
  domain, pausing without a protocol effect when its exact creation forecast is
  underfunded, executing only a reviewed plan-owned Ledger transfer and
  reconciling exact creation receipts and fees at terminal;
  observability authority is
  proven through exact diagnostic codes and response variants across fresh and
  restored PocketIC fixtures; and B1 consumes typed transform metrics from a
  method-owned harness that compiles against the immutable product worktree
  instead of optimizer log prose. Its separate offline-resolved lock and every
  runnable patch hash are machine-bound. A retained all-role row-3 measurement
  against immutable `v0.110.5` attributes 3,001,136 optimized code bytes and
  2,025 defined functions to the inclusive activation-persistence family while
  preserving every role's Candid hash. It proves no activation parity and does
  not promote B2. Role-contract validation also rejects auth-crypto features
  without matching role authority, target-Wasm dependency closure rejects
  duplicate crypto package generations, and capability reports expose retained
  cryptography symbols separately. Aligning the existing chain-key
  implementation with the IC client stack's `k256 0.13.4` dependency is a
  non-protocol contraction; chain-key replacement remains unauthorized.
- Current open 0.110.8 slice: the maintainer explicitly authorized the bounded
  authorization-persistence split before complete B1 acceptance. Local
  application sessions, delegated-token issuer proof and Root delegation
  policy now use separate records, stable cells, operations and allocation
  keys. Verification alone, Coordinator and Store select no auth persistence;
  Root selects only Root delegation state. Typed contract tests and targeted
  optimized Coordinator, Root, verifier-only and combined issuer/local builds
  prove the selected declaration set. This slice neither accepts B1 nor opens
  unrelated storage families.
- Current open 0.110.8 dependency alignment advances only the isolated
  lifecycle-composition fixture to the exact IcyDB 0.253.0 family with default
  features disabled. It adds no IcyDB edge to a production Canic crate and
  changes no Fleet runtime capability or B1/B2 verdict.
- Row 8 remains `specified`. Its first full development qualification passed
  ten canonical artifacts before the Wasm Store correctly enforced canonical
  Candid parity. The audit-only switch now isolates that counterfactual
  comparison and the exact Store artifact qualifies, but the complete selector
  has not passed against the current patch identity and no retained measurement
  is claimed.
- Rows 10 and 12 remain `specified`; their exact canonical-App development
  qualifications pass, while their complete selectors and immutable paired
  measurements remain open.

Design: [Fleet runtime contraction](0.110-design.md)

Planning evidence: [B1 Wasm input evidence](../../audits/working/0.110-fleet-runtime-contraction/b1-input-evidence.md)

Generated-surface source trace:
[B1 generated surface inventory](../../audits/working/0.110-fleet-runtime-contraction/b1-generated-surface-inventory.md)

Destroyed-state source trace:
[B1 destroyed-state and reconstruction inventory](../../audits/working/0.110-fleet-runtime-contraction/b1-destroyed-state-inventory.md)

Pool Ledger recovery hard-cut trace:
[B1 source-family absence ledger](../../audits/working/0.110-fleet-runtime-contraction/b1-pool-ledger-recovery-hard-cut.md)

Controlled-ablation trace:
[B1 experiment manifest](../../audits/working/0.110-fleet-runtime-contraction/b1-controlled-ablation-manifest.md)

Current canonical-role evidence:
[CANIC-WASM-001/v6 baseline](../../audits/reports/2026-09/2026-09-03/wasm-footprint-v6.md)

## Binding Corrections

- B1 freezes dated code-section, total-module and defined-function limits.
  Completion requires at least 5% of both the code-section and replica-limited
  defined-function limits free under frozen tools: 512 KiB code headroom under
  the retained 10 MiB limit, with 1 MiB preferred. The frozen IC source proves
  that imports do not consume the 50,000-function limit; the optimizer-defined
  metric is an independent cross-check, while `ic-wasm`'s total is attribution
  only.
- B1/B5 bind the canonical eleven-role roster and Canic-owned `runtime_probe`,
  `payload_limit_probe`, `blob_storage_probe` and `leaf_probe` fixtures. Their
  matrix covers both authentication paths, blob storage/billing, lifecycle,
  payload adapters, status, metrics, timers, applicable recovery and the
  generic cohort without importing a consumer protocol.
- The supplied 10,275,629-byte/268-method downstream observation remains useful
  pressure evidence only. A Toko/TokoMiner commit, lockfile, application state
  policy or release procedure is not a Canic gate.
- Baseline attribution uses controlled storage, activation, authorization,
  CBOR, recovery, macro projection, Candid construction/documentation/
  serialization, async-adapter, provider, timer and status ablations. Every
  experiment records code, total, compressed, function, table and instruction
  evidence.
- B1 adds a controlled `1..=N` Canic generic-instantiation cohort and a named
  post-`-Oz` report mapping remaining Canic-owned generic families to concrete
  type arguments and specialized bodies. Per-instantiation deltas are measured,
  not inferred from downstream generator slopes.
- B2 emits direct role-specific initialization, restore and recovery calls.
  Function-pointer, trait-object and erased-callback registries are excluded.
- B1 inventories every runtime-contributing macro expansion. B2 contracts its
  storage/lifecycle portion; B4 closes endpoint, authentication/admission,
  status, Candid, metrics, command, configuration, provisioning, timer,
  recovery and role-dispatch reachability.
- Macro imports, macro splitting and source shape receive no size credit.
  Canonical optimized artifacts must prove unselected machinery absent.
- Every B2-B4 batch ends with canonical roles, each affected Canic-owned
  capability fixture, fresh marginal attribution and an explicit
  stop/continue/retarget decision.
- B3 records/codecs proceed only where the latest residual evidence justifies
  them. B4 remains mandatory while any known role-inapplicable generated
  endpoint/provider/serializer/timer/recovery/control-plane surface remains;
  reaching the size reserve cannot waive final artifact absence.
- Endpoint extraction is accepted from optimized artifacts, not source shape.
- The temporary pool Ledger recovery helper is hard-deleted in published
  `0.110.3`. B1 records the current/predecessor artifact delta;
  B4 verifies that no helper/Store/Root/DTO/host/test/CI reachability returned.
  Current Fleet funding targets native canister balances and retains no helper
  compatibility path or fallback.
- Chain-key ECDSA replacement is not authorized. Missing the safe contraction
  gates stops the line for a separate protocol design and threat-model audit.
- Release-build acceleration is parallel supporting work. It is not a batch
  and cannot delay the first evidenced runtime cut.
- Governed release metadata, planning and qualification enforce the
  reinstall-only hard cut structurally. An existing stateful Fleet's ordinary
  upgrade/adoption attempt must reject with no install, stop, controller,
  funding or workflow effect. Only an explicit authorized reinstall may then
  proceed.
- If safe Canic-owned cuts do not place the canonical roster and fixture matrix
  above both 5% reserves, B5 stops with an exact Canic-owned residual handoff.
  Optional consumer observations do not authorize downstream mutation or
  weaken authentication.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence | Status |
| --- | --- | --- | --- |
| B1 | Immutable baseline, differential attribution and absolute budgets | Dated limits, repository-owned capability fixture matrix, replica-validator-equivalent local-function count, generated-surface inventory, complete artifact vector, current/predecessor delta for the deleted temporary pool Ledger recovery family, `1..=N` generic-instantiation cohort, named post-`-Oz` report, destroyed-state/reconstruction inventory and accepted allowances | Active from immutable `v0.110.5`; valid `CANIC-WASM-001/v6` size/determinism evidence, generated-surface/destruction traces, pool-Ledger source absence, the machine-checked eighteen-row ablation harness and repository-owned frozen function counter, immutable all-role row 2 attribution supporting role-selected storage wiring without lifecycle parity, immutable all-role row 3 inclusive activation-persistence attribution supporting role-selected separation without activation parity, immutable canonical-plus-runtime-fixture row 4 authorization-persistence attribution without persistence or authorization parity, immutable canonical-plus-runtime/blob-fixture row 5 shared-CBOR-helper attribution without codec or persistence parity, selected-artifact qualification for row 6, specified audit-only rows 8, 10 and 12, immutable row 11 payload-adapter attribution retaining the safety path, the `Page<T>`/`N = 5` generic fixture and hash-bound downstream routing observation are retained, while counter-backed immutable role/fixture measurements, complete selected-artifact build qualification for the remaining specified rows, remaining source-ablation patches and measurements, optimized-artifact absence, generic measurements/post-`-Oz` mapping, accepted allowances and compatible predecessor artifact evidence remain open |
| B2 | Role-selected storage reachability | Lazy TLS, direct generated wiring, storage/lifecycle inventory contraction, data-only reservations, symbol absence and full remeasurement | Blocked overall on B1; bounded auth stable-declaration sub-slice explicitly active and targeted role evidence passes |
| B3 | Capability-owned activation/auth records and only still-justified codecs | Concrete records, phase cache, bounded codec evidence and full remeasurement | Blocked overall on B2 decision; bounded auth-record split explicitly active without selecting another codec cut |
| B4 | Endpoint, recovery and role-capability pruning | Complete generated-surface inventory, exact Candid/provider reachability, optimized body/function evidence, direct dispatch, continued absence of the hard-deleted temporary pool Ledger recovery family, role pruning and full remeasurement | Mandatory after the B3 decision while known role-inapplicable reachability remains |
| B5 | Canic-owned qualification and closeout | Canonical and fixture 5% byte/function reserves, capability matrix, per-role generated-surface absence, total-module limit, instructions, determinism, structured reinstall-only guard, optional consumer observations and immutable audit | Blocked on final B2-B4 decision |

## Deferred From 0.110

- Host-only semantic-version inventory, release-narrative cleanup and status
  redesign return to later operator planning.
- Indexed estates, a bounded reserve Fleet and cycle-safe source disposition
  remain 0.111 work.
- Adaptive lanes, broad automatic funding, batches and 1,000-canister
  qualification are unscheduled.
- The generic runtime Observatory is an unnumbered idea.

## Next Authorized Action

VS1 throughput, the restored CANIC-162 allocation report and published IcyDB
0.257.4 / ic-memory 0.13.2 composition are qualified. The same open .14 batch
and changelog are ready for the maintainer-selected release gate. Do not infer
broad validation or publication authority.
The contraction work below remains sequenced separately and gains no completion
credit from faster tests.

Finish focused review of the explicitly authorized authorization-persistence
slice and keep its targeted optimized-artifact evidence separate from complete
B2 remeasurement. This does not accept B1 or authorize another state family.

Then continue B1 from immutable `v0.110.5`: qualify row 8 endpoint-declaration
construction and rows 10 and 12 endpoint-reply
serialization and metrics-provider attribution. Then
complete the remaining controlled ablations,
optimized generated-
surface absence, generic cohort and accepted allowances and obtain compatible
predecessor evidence where required. The source generated-surface and complete
allocation/destruction inventories are retained. Do not begin the remaining B2
or B3 scope until the maintainer accepts the complete B1 evidence.
