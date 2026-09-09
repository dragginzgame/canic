# Current Status

Last updated: 2026-09-09

## Purpose

This is the compact handoff for active Canic source and roadmap work. Read it
first, then open only the linked design, changelog or audit owner needed for the
task.

Historical handoffs:

- [through 2026-06-30](archive/2026-06-30-precompact.md);
- [through 0.90.2](archive/2026-07-13-precompact.md);
- [through 0.101.52 Q4](archive/2026-08-12-precompact.md);
- [through published 0.109.12](archive/2026-08-26-pre-root-repair-hard-cut.md);
  and
- [pre-reorientation 0.109.24](archive/2026-08-30-pre-roadmap-reorientation.md).

## Release Evidence Contract

Release truth comes from workspace package versions, the root and detailed
changelogs, the annotated Git tag and release commit, the complete published
package set, and the governed validation marker at the end of this file. The
version transaction owns that marker; explanatory prose is not a second release
guard.

Current development begins from tagged `v0.110.12`. Its immutable
release details are in [the 0.110 changelog](../changelog/0.110.md). Package
versions remain 0.110.12; the current source batch uses the open 0.110.13 draft.

## Integration Guard Corrections

The maintainer's ordinary integration run failed three targets: workspace
manifest inheritance, managed stable-memory ABI ownership and timer dependency
inventory. The internal fixture now inherits `canic-cli` from an exact workspace
dependency declaration. Public stable-memory extent observation goes through
the existing memory boundary; its size conversion and allocation behavior are
unchanged. The timer guard verifies the locked IcyDB version against the exact
workspace pin instead of retaining a second obsolete version literal.

All 23 cases across `workspace_manifest`, `stable_memory_abi_guard` and
`timer_inventory_guard` pass (`/tmp/canic-integration-guards-tests.log`). Native
all-target/all-feature core Clippy and Wasm32 all-feature library Clippy pass
(`...-clippy.log` and `...-wasm-clippy.log` with the same prefix). The Wasm check
also exposed two enum-size expectations that now apply only on 64-bit targets;
DTO and stable-record layouts remain unchanged. Formatting, whitespace,
document checks and 0.110.13 changelog preflight pass. The accepted scoped batch
and changelog are ready for release review; complete release validation remains
the maintainer's rerun. No broad suite, version bump or Git publication ran here.

## Release Preflight Rust Alignment

The latest maintainer Clippy failure was one stale
`significant_drop_tightening` expectation on the autonomous Root deletion test.
Removed only that expectation and updated the remaining test-length exemption
to describe its exclusively owned fixture. No test behavior changed.
Warning-denied all-target/all-feature Clippy passes for
`canic-testing-internal`, including governed PocketIC test compilation (59.24s).
Log: `/tmp/canic-stale-expectation-clippy.log`. Targeted formatting, whitespace
and 0.110.13 changelog preflight pass. The scoped correction and changelog are
ready for release review; no PocketIC execution or full workspace validation
was repeated for this annotation-only change.

The subsequent full Clippy run exposed nine `redundant_pub_crate` warnings in
the User Hub/User Shard reinstall fixtures and the runtime process fixture.
Removed their redundant `pub(super)` restrictions while retaining private
containing modules and the same effective external visibility/Candid surface.
All-target/all-feature warning-denied Clippy passes for `canister_user_hub`,
`canister_user_shard` and `runtime_probe` (40.20s); log:
`/tmp/canic-fixture-visibility-clippy.log`. Targeted formatting passes. This
visibility-only correction adds no runtime behavior and does not repeat the
PocketIC journeys. The open 0.110.13 notes include the correction. The scoped
batch and changelog remain ready for release review; a successful complete
workspace Clippy/release run is still unclaimed.

The next maintainer validation passed workspace checking and stopped at the
same Rust 1.98.1 `chunks_exact_to_as_chunks` lint in control-plane Root Store
digest parsing. A repository-wide Rust source scan found two more sites in host
network fingerprint parsing and its CBOR fixture helper. All three now use
fixed-size byte pairs without changing input validation or serialized bytes.
Warning-denied all-target/all-feature Clippy passes for `canic-control-plane`
and `canic-host`; four Root Store and seventeen host network/CBOR regressions
pass. Logs: `/tmp/canic-remaining-hex-clippy.log` and
`/tmp/canic-remaining-hex-tests.log`. Targeted formatting, diff whitespace and
the existing 0.110.13 changelog preflight pass. This resolves the reported lint
failure and leaves the scoped batch/changelog ready for push/release review;
the complete release gate has not been rerun by the agent.

The maintainer's patch validation stopped at the release-integrity guard:
`rust-toolchain.toml` used 1.98.1 while primary CI, scheduled security CI and
developer setup still pinned 1.97.1. All three pins and the active README now
match 1.98.1. The accompanying `validated source changed` output came from the
guard's intentional source-drift fixture, not a detected change to this checkout.
That fixture now captures its expected diagnostic and prints it only on an
unexpected exit status; its no-version-bump assertion remains intact.

The exact release-integrity guard passes, as do the release-validation-lane
regression, actionlint for both workflows, ShellCheck for both edited scripts,
diff whitespace and the 0.110.13 release-notes preflight. Guard log:
`/tmp/canic-rust-pin-integrity.log`. The scoped correction and root/detailed
changelog are ready for push/release review. The maintainer's complete gate
still needs its release-flow rerun; this correction did not run broad validation,
change package versions or publish Git state.

## Active IcyDB Update

The maintainer requested the latest IcyDB on 2026-09-09. The official crates.io
registry reports 0.257.2 as the latest published non-yanked release; the sibling
checkout's 0.257.3 is not yet published. The workspace exact pin and all six
IcyDB lockfile entries now use 0.257.2, with no unrelated dependency changes or
source adaptation. The existing `icydb_lifecycle_composition` PocketIC target
passes (one case, 40.40s; runner 56s), covering startup, timer custody,
same-release upgrade and failed-upgrade retry. Log:
`/tmp/canic-icydb-pocketic.log`. The runner cleaned its server and scratch.

The maintainer then requested lint cleanup and changelog readiness. Fixed the
five `chunks_exact_to_as_chunks` warnings in Component, Fleet, Network,
ReleaseBuild and Operation ID parsers, plus two matching stable-byte test
helpers exposed by all-target Clippy. Existing width/character validation and
stable bytes remain unchanged. Warning-denied all-target/all-feature Clippy
passes for `canic-core`, `canic_icydb_lifecycle_probe` and
`canic-icydb-lifecycle-schema`; 48 selected ID, replay and serialization tests
pass. Logs: `/tmp/canic-lint-cleanup-clippy.log` and
`/tmp/canic-lint-cleanup-tests.log`. Targeted Rust formatting, diff whitespace
and the 0.110.13 release-notes preflight pass.

The previously recorded lint blocker is resolved. The complete accepted scoped
batch is ready for push/release review, and both open 0.110.13 changelog surfaces
are prepared for the governed publication flow. Earlier build/performance
evidence retains its original dependency and toolchain scope; the prior IcyDB
PocketIC result predates this behavior-preserving parser cleanup. Package
versions remain 0.110.12. No broad gate, Git publication or sibling mutation
was performed; the recorded deferred upstream criteria remain open.

## Active Build Feedback 087 and 139

The maintainer accepted CANIC-087 and CANIC-139 on 2026-09-09. The
[build feedback owner](../audits/reports/2026-09/2026-09-09/build-feedback.md)
tracks declaration/runtime build separation, package-bound runtime batching,
exact artifact reuse and measured qualification. This extends the open 0.110.13
batch; package versions remain 0.110.12. The scoped implementation and
qualification are complete and the combined source batch/open changelog are
ready for push/release review. Unchanged complete builds return in 2.83s; a
warm one-role edit improves from 477.10s to 291.03s. Cold time is roughly unchanged.
Feature-preserving batches retain fat LTO after measuring Thin's size tradeoff.
The focused managed/standalone initialization and same-release upgrade case
passes. The mixed-topology case completed fresh readiness, conservation and
terminal replay assertions, then was interrupted during its extra reset exercises;
it is not recorded as a full case pass. Full upstream closure remains open for
precise per-role/cross-identity reuse, cold improvement, clean determinism and
Toko qualification. Earlier recovery/operator evidence retains its scope.
Toko Miner remains read-only; no broad gate, version bump or publication.

## Active Activation Feedback 157–160

The maintainer accepted working through CANIC-157–160 with an explicit sanity
and drift review. The [scope and evidence owner](../audits/reports/2026-09/2026-09-08/activation-feedback.md)
confirms the activation-authority, retry and originating-failure gaps, and
keeps planning latency measurement separate from unproven optimization.
Existing Ensure, provisioning, protected-status and observation owners remain
authoritative. Prevention, bounded retries, originating diagnostics and measured
observation concurrency are complete. The installed-release recovery now passes
PocketIC against immutable `v0.110.12` runtime: an Issued operation reaches
Published/inactive Root, source-bound preparation rejects controller drift,
stop/restart settles the source, a separately reviewed Root reinstall reconciles
a lost response, and Full Ensure reaches readiness with the exact retained pool,
bounded native debit, unchanged Root/operator Ledger accounts and effect-free
replay. Exact source plan/journal/state bytes are archived before the recoverable
local replacement; the source journal is never marked Converged.

The proof exposed and corrected a changed-Wasm history assumption: reinstalls
now bind the reviewed prior module as well as the requested module. Three history
regressions, 167 direct host Fleet Ensure cases, 52 CLI cases and focused
host/CLI/fixture Clippy pass. Earlier 63 core/control-plane provisioning and
24 pool cases retain their scoped evidence. The final recovery log is
`/tmp/canic157-installed-recovery4.log` (731.72s test, including 301s artifact
build; 810s runner including native compilation). Source integrity checks match
all 3,738 snapshot files to commit `95c6caa03867cd68b3bc57ba88c02b961251460b`.

The recovery is bounded to one Root, complete inventory and successful settlement;
changed membership, pending paid work or missing authority prevents reset.
The smaller recovery fixture is separate from the earlier 24-asset RF2 proof
and from live Toko adoption. The complete accepted OD1/RI1/RF2/157–160 batch and
open 0.110.13 changelog are ready for push and release review. Package versions
remain 0.110.12. No broad gate, Git publication, deployment or sibling mutation.

## Active Upstream Follow-ups 151–156

The maintainer accepted all six follow-ups on 2026-09-08. Sequence: launcher
alignment (152), complete retained-estate review before Root reset (154),
affordable continuation phases and dependent funding review (155/156), exact
qualification configuration (151), then bounded public process coverage (153).
These extend the existing 0.110.13 draft. Implementation, targeted recovery
qualification, propagation and cleanup are complete. The isolated CLI/launcher
probe passes. The 27-canister changed-release journey passes omission rejection,
reviewed reuse of all 24 pool assets, affordable continuation admission, six
explicit top-ups, lost install/funding response recovery, terminal conservation
and effect-free replay. All eight timer PocketIC cases, 18 all-feature metrics
cases, 167 host and 51 CLI cases pass. The sampling bound remains 20 million
instructions. Final affected-package hygiene is recorded in the
[feedback handoff](../audits/reports/2026-09/2026-09-08/operator-feedback.md).
The accepted OD1/RI1/follow-up source batch and open 0.110.13 changelog are ready
for push/release review; package versions remain 0.110.12. No broad gate,
publication, deployment or sibling mutation was performed.

A later read-only tracker refresh found CANIC-157–160: activation identity
coherence, bounded autonomous retries, originating failure diagnostics and
planning latency measurement. These are new follow-up scope, outside the six
accepted here; none is claimed fixed by this qualification. The 154 proof
refreshes authority before Root reset; it does not reproduce 157's changed
inputs between Root and Store effects. CANIC-141 remains deferred.

## Active Same-Release Fleet Reinstall

The maintainer accepted CANIC-149 on 2026-09-08. RI1 extends the open
0.110.13 batch with explicit `fleet ensure --reinstall` preparation, a sealed
physical-inventory reset review and same-operation recovery. The operation
retains physical canisters and cycle accounts while wiping application data.
Implementation, recovery evidence, Candid/fixture propagation and cleanup are
complete. Targeted native/interface checks, affected-package Clippy and the
production-adapter PocketIC journey pass. The journey proves two actual database
wipes, before/after-response recovery, ordinary Root-version advancement,
retained estate/controllers/cycle accounts, conservation and effect-free replay.
The OD1/RI1 scope is complete; the expanded 0.110.13 batch now depends on the
accepted follow-ups above. CANIC-141 remains deferred.

## Active Downstream Diagnostic Feedback

The maintainer requested the upstream feedback from Toko Miner. The bounded
diagnostic batch addresses reopened CANIC-042 and CANIC-150: retain actionable
nested Cargo causes and correct Medic advice, then explain Fleet plan actions,
creation counts and cycle allowances. Host role evidence and CLI presentation
own these corrections; the JSON and progress contracts remain unchanged.
The diagnostic slice is complete. Its 0.110.13 draft now also contains the
completed RI1 batch. Five host evidence tests,
thirteen Fleet CLI tests, the Medic advice regression and all-target/all-feature
Clippy for both affected packages pass. Formatting, changelog governance and
document checks pass.
The [feedback handoff](../audits/reports/2026-09/2026-09-08/operator-feedback.md)
records scope and implementation/qualification evidence for the combined batch.
The downstream live-output demonstration remains adoption evidence.
CANIC-149 is complete as RI1 above; CANIC-141 remains deferred. No downstream files,
package versions, Git publication or deployment were changed.

The proposed 0.110.13 exceeds the soft twelve-release minor guideline. It
includes corrections to the published build/operator diagnostics, the
maintainer-selected CANIC-149 operator workflow and accepted CANIC-151–156
recovery/tooling/opt-in diagnostic follow-ups. These form one accepted batch
on the affected 0.110 line; they do not promote another minor or close this one.

## Active Release Turnaround Correction

The completed release retained a 4,497-second workspace test run, including
3,524 seconds in the internal PocketIC tier. A separate compilation for six
pure internal cases took 185 seconds. The current batch folds those cases into
the ordinary workspace library graph, removes their redundant aggregate harness,
and reuses artifact-builder preflight within each Fleet release. The shorter
polling experiment showed no measured improvement and was removed. Ten native
tests, governed catalogue verification, default/all-feature Clippy, runner and
document guards pass. The focused Fleet experiment also passed recovery,
conservation and zero-effect replay; its exact scope and timing limitations are
in the [compile-consolidation handoff](../audits/reports/2026-09/2026-09-08/test-compile-consolidation.md).
The bounded turnaround correction is complete. The maintainer has added
CANIC-147 public history to the open 0.110.12 batch. Nineteen final native cases,
eight timer-focused PocketIC cases, canonical Candid equality and focused Clippy
pass. The complete Canic source batch and changelog draft are ready for release
approval. Downstream CANIC-148 producer qualification remains separate and open.
The subsequent maintainer validation found one stale timer-owner inventory entry
for CANIC-147. The exact owner is now registered in the guard; all 15 focused
timer-inventory tests pass. That validation skipped serial PocketIC at the
ordinary-test barrier; the earlier focused timer qualification remains separate.
The next maintainer run exposed a shared Store artifact blocker: hand-edited
history Candid was structurally equivalent but differed from the compiled profile
bytes. Canonical regeneration corrects it; ordinary Store and Coordinator builds
now pass without refresh. The full interrupted PocketIC run was not repeated.
No new full-release duration is established; the serial
PocketIC tier remains the largest cost. No version, Git publication, deployment
or broad validation command was run.

The proposed 0.110.12 would exceed the soft twelve-release minor guideline.
The maintainer explicitly selected CANIC-147 to complete the published public
metrics surface alongside the release-turnaround correction. This accepted
consolidation remains on 0.110;
no next-minor implementation or closeout approval is inferred.

The preceding [turnaround handoff](../audits/reports/2026-09/2026-09-08/release-turnaround.md)
records the shipped metadata publication guard, registry-observation reuse and
persistent timing logs. `sccache` is already enabled through Make when installed.

## Public Sampling Follow-up

CANIC-148's bounded sampling and family isolation shipped in 0.110.11. The
[sampling handoff](../audits/reports/2026-09/2026-09-08/public-sampling-bounds.md)
retains its focused evidence. Downstream adoption/live qualification remains
with Toko Miner. The maintainer accepted CANIC-147 on 2026-09-08: one optional
five-minute native sampling claim, bounded 24-hour heap history and an explicit
measurement participant. CANIC-147 implementation and qualification are complete.
All eight timer-focused PocketIC cases pass, including
actual periodic sampling below the existing 20-million-instruction fixture budget.
The [history and qualification handoff](../audits/reports/2026-09/2026-09-08/public-chart-history.md)
records the implementation, measured costs and precise downstream evidence gap.
The [public observability contract](../features/runtime/public-observability.md)
contains current caps, counter/reset semantics and application composition.
Public health now uses the precise `responding` label in Rust and canonical
Candid. It proves query responsiveness only, not Fleet readiness; downstream
bindings and labels must follow that hard cut.
CANIC-141 stays deferred. Sibling repositories remain read-only; actual Toko Miner
producer/cost qualification cannot be claimed from Canic's neutral fixtures.
The following sections retain historical implementation evidence.

## Active Generated Fleet Build Batch

The maintainer selected one host-generated package path for Root, Coordinator
and Store, removing their separately published entrypoint packages. This extends
the open 0.110.10 draft. Shared runtime owners remain in Canic; canonical
Coordinator/Store Candid ships in `canic/candid`. Package generation, publication
inventory, passive reports, fixture/package qualification and active docs are
complete. All three artifacts pass the isolated packaged-source proof. The
Prepared Root initial-Shard case passes terminal membership and replay in
425.19s including cold builds (489s runner). The 92 focused host tests, 42
protocol tests, seven workspace checks, configuration/storage guards, affected
Clippy configurations, crypto closure, shell and formatting checks pass. The
[generated-build handoff](../audits/reports/2026-09/2026-09-08/generated-fleet-artifacts.md)
records exact scope and evidence.
The release-preflight audit catalog failure is corrected: five method scopes
and versioned fingerprints now follow generated Fleet ownership. Targeted
catalog and document checks pass; runtime/build sources are unchanged.
The subsequent unit-test failures are corrected in fixture inputs and exact
expected outputs, including standalone Cargo workspace isolation inside deployment
scratch. All 32 selected regressions pass; production behavior is unchanged.
The subsequent protocol-test import failure is also corrected: role-specific
read-contract tests match facade feature gates. Isolated default, Coordinator,
Store and Root selections and changed-target all-feature Clippy pass. Targeted
qualification now explicitly covers those feature and scratch boundaries.

The expanded canonical Root, retained Fleet, read-surface and generated-build
batch is ready for release approval. The open 0.110.10 changelog is ready;
package versions remain 0.110.9. The maintainer-selected release gate/publication
and downstream adoption/live qualification remain. CANIC-141 stays deferred.
No version change, publication, deployment or sibling edit was made. The
preceding reports retain evidence for their recorded source boundaries.

## Active Read-Surface Separation Batch

The maintainer accepted separating public status from protected observability,
with optional aggregate metrics disabled by default. The open 0.110.10 batch
now includes this implementation. Public snapshots expose explicit sampling
and staleness, bounded aggregate shard occupancy, cycles, performance and
application-supplied values; controller/Root authority stays independent of
publication and player admission. Authentication and durable-operation reads
retain their owning contracts on separate methods. Implementation, interface
propagation, cleanup and targeted qualification are complete. Public snapshot
and observer PocketIC tests (6), native authentication/session tests (7), and
the Coordinator Registry/replay case pass, including fresh infrastructure
activation. Canonical Wasm builds, Candid equality, focused host/CLI/policy
checks, affected-package Clippy and formatting pass. The
[read-surface handoff](../audits/reports/2026-09/2026-09-07/public-observability.md)
records evidence, metric semantics and downstream adoption requirements.
The combined canonical Root, retained Fleet and read-surface batch is ready
for release approval, with the open 0.110.10 changelog ready; package versions
remain 0.110.9. The maintainer-selected release gate and publication remain.
Toko Miner binding/UI adoption and live qualification remain downstream work.
CANIC-141 stays deferred. No sibling edits, compatibility aliases, publication
or version transaction were made.

## Active Retained Fleet Feedback Batch

The maintainer selected CANIC-143, CANIC-144 and the CANIC-125 follow-up after
CR1. Work extends the open 0.110.10 draft: bind native pool funding to its
reviewed reset lifecycle, resolve symbolic Root prerequisites from existing
retained identity, and size bounded provisioning waits from initial child
demand. Existing production-adapter recovery journeys own qualification.
The corrections are complete and the combined CR1/RF1 draft is ready for
release approval. All 150 focused Fleet host tests and affected-package
Clippy pass. Installed-import funding/reset recovery, conservation and replay
pass in 310.66s; generated symbolic-seed restart through reinstall and replay
passes in 393.03s. Formatting and document checks pass. The
[feedback handoff](../audits/reports/2026-09/2026-09-07/retained-fleet-feedback.md)
records final evidence and downstream qualification. CANIC-141 stays deferred.
The maintainer-selected release gate/publication and downstream live timing
qualification remain; no new recovery mode, compatibility lane or sibling edit.

## Active Canonical Root Batch

The maintainer reports 0.110.9 pushed and selected canonical Fleet Subnet Root
as the next hard-cut batch. CR1 in the [0.110 tracker](../design/0.110-fleet-runtime-contraction/status.md)
owns canonical Root packaging/builds, removal of application-owned Root hooks
and package selection, the Fleet Store crate rename and complete focused
qualification/propagation. CR1 is complete and ready for release approval;
the open 0.110.10 changelog covers the complete batch while package versions
remain 0.110.9. Fresh generated Fleet recovery and both effect-free replays
pass in 12m00s, including a 4m03s cold artifact build. Generated reinstall,
lost-response recovery, conservation and replay pass in 8m16s. Packaged Root
building, focused config/host/CLI/interface checks, affected-package Clippy
and formatting pass. The [CR1 handoff](../audits/reports/2026-09/2026-09-07/canonical-fleet-root.md)
records the final evidence and adoption contract. No implementation blocker
remains within CR1; the maintainer-selected release gate and publication remain.
CANIC-141 and downstream live qualification remain deferred. The sections below
retain the preceding release's implementation evidence.

## Active Test Throughput Batch

The maintainer authorized auditing and refining tests after publishing 0.110.8,
then requested application-neutral fixtures. The open 0.110.9 draft covers
smaller synthetic Fleet proofs, removal of repeated recovery setup, runner
consolidation and exact-selector validation. Package versions remain 0.110.8.
The [test throughput audit](../audits/reports/2026-09/2026-09-07/test-throughput-audit.md)
owns coverage and focused evidence. This test-throughput batch is complete
and ready to push within its scope: generated mixed-topology recovery/replay
passes in 9m00s, prepared Failed-import repair in 4m24s, and generated reinstall
in 7m24s. The smaller host proof, generator and catalogue regressions,
protocol-inventory/changelog tests, affected-target Clippy, formatting and
invalid-selector check pass. Full release duration remains unmeasured. No broad
gate, version change, Git publication or deployment is inferred. The earlier
0.110.8 implementation handoff below is retained as historical context.

The maintainer also requested a CLI command audit. Its
[ownership and cleanup report](../audits/reports/2026-09/2026-09-07/cli-command-audit.md)
records the `restore apply` hard cut, shared backup selection and help fixes.
These changes extend the open patch draft; they do not complete the independent
test-throughput qualification above.

## CANIC-140 Retained Creation Fee

The maintainer requested the confirmed Toko Miner retained-growth fee correction.
Every current estate seed now requires explicit `management_creation_fee_cycles`
in compact units. Generation carries the fee into future creation funding;
the retained-only rejection and implicit zero are removed. Already-paid assets
remain retained without another creation charge. The open 0.110.9 draft,
operator guide and CLI help include the contract.

All 20 generator regressions pass, including a generated retained-growth plan
with exact nonzero creation/Ledger fees before any mutation. The existing small
paid-growth PocketIC proof passes in 181.20s (218s runner), covering lost funding
and creation replies, conservation and effect-free replay. All-target,
all-feature warning-denied Clippy passes for the changed host, CLI and internal
fixture packages, and both CLI help regressions pass. The [correction report](../audits/reports/2026-09/2026-09-07/canic-140-retained-creation-fee.md)
distinguishes host generation evidence from runtime and downstream qualification.

The subsequent CANIC-140 qualification follow-up now feeds the production
generator's output directly into the existing small paid-growth PocketIC case.
The combined journey passes in 118.47s (127s runner), including nonzero fee
authority, lost transfer/creation replies, conservation and terminal replay.
It retains IC-built runtime authority while all network effects stay inside
the local fixture. No application-specific or duplicate long test is added.

The Canic source correction is complete within the accepted batch. It does not
close the downstream mainnet qualification: Toko Miner must adopt the published
correction, supply the exact fee in its retained seed, regenerate and review its
plan, then validate complete growth and terminal application behavior. No sibling
edit, broad validation, version change, publication or live deployment was made.

The maintainer selected resumable reviewed funding for CANIC-142 and explicitly
deferred CANIC-141. The host now retains an exact additional funding review
inside the existing operation, without replacing its original plan or issued
protocol effects. Planning exposes `funding_review.review_sha256`; applying that
digest approves only the recorded shortfall and Ledger fee. A retained intent
survives response loss with the same transfer identity. Terminal conservation
includes the additional funding and fee while retaining exact creation receipts.
The current journal schema is hard-cut in place. The focused real-adapter
funding-pause journey passes in 190.30s (221s runner), including fee rejection,
lost transfer/creation replies, preserved pending identity, complete readiness,
cycle conservation and effect-free replay. Root maintenance and the outer ICP
adapter now preserve the typed pause instead of dropping it. The 147 selected
Fleet host tests, 10 Fleet CLI tests and registration check pass. Four focused
funding regressions cover the final authority cleanup and reject a completed
operation's funding review against a later plan. All-target/all-feature
warning-denied Clippy passes for host, CLI and internal fixtures; formatting and
diff checks pass. The accepted open batch, including CANIC-140 and CANIC-142, is
ready for release approval, with the 0.110.9 changelog draft ready. CANIC-141
remains deferred; no version, publication or downstream deployment was made. The
[funding-resume report](../audits/reports/2026-09/2026-09-07/canic-142-reviewed-funding.md)
records the injected recovery fixture and downstream qualification boundary.

The subsequent maintainer validation found a stale zero-balance funding pause
in the four-Workload refill case after its transfer had already completed.
The adapter now refreshes the exact Root Ledger balance before reporting a
pause, retaining ordinary bounded reconciliation when the account is funded.
The focused host regression and scoped all-target/all-feature Clippy pass.
The exact affected PocketIC case passes on the final source in 225.83s (264s
runner), including lost responses, four Ready assets, exact conservation and
effect-free replay. Formatting and diff checks pass. The accepted batch is
ready for release approval again and the existing changelog draft is updated.
The interrupted full release suite was not restarted. The funding-resume report
above records the cause, correction and focused evidence.

## Maintained 0.109 Contract

Fleet admission retains one Coordinator-owned canonical policy, one Root-owned
distribution operation per Root, and one exact local projection on each
enrolled non-Root target. `caller::is_fleet_admitted()` and
`canic::fleet_admission::require_caller()` read that same projection and the
observed transport caller. Admission never replaces application membership,
resource ownership, service authority or infrastructure authority.

Fleet Ensure remains the sole desired-state reconciliation owner. Paid or
identity-changing effects require exact reviewed authority, durable intent,
bounded debit and lost-response reconciliation. Terminal convergence proves
cycle conservation and immediate replay is effect-free. Historical install,
repair, migration and recovery compatibility is not restored.

Published `v0.109.34` owns the complete `CANIC-102` and `CANIC-112` through
`CANIC-117` corrections: terminal Create balances, exact available/required/
shortfall guidance, bounded first-observation burn, active Registry-operation
binding, retained terminal Component inventory, issued withdrawal evidence,
bootstrap import capacity, role-specific command entrypoints and the managed
cross-release runtime fence. Exact tests and negative cases are retained in the
detailed changelog.

## Accepted 0.109 Closeout

Published `v0.109.35` closes `CANIC-118`. ICP CLI 1.3.0 returns
only public identity/controller/module fields when the operator is not a newly
created Root-owned pool's controller; those fields cannot prove its live cycle
balance.

The accepted correction keeps one executable authority sequence:

- a new fresh-estate plan creates each Root-only pool with the exact operator
  as a temporary direct controller, observes its real balance, installs the
  Root, then removes the operator before protocol convergence;
- an immutable 0.109.34 plan is not rewritten: its issued Create retains the
  exact Ledger receipt and Principal, defers only the unavailable balance
  observation, advances its reviewed infrastructure prerequisites, then uses
  the installed Root's protected inspection;
- the inspecting Root must match the retained Principal, exact desired
  controllers, successor module and running state;
- the target must retain the exact Root-only controller set, module-free pool
  shape and live native balance; and
- public status never supplies inferred cycles, controllers or runtime state.

The deferral is restricted to an issued fresh Root-only pool Create with exact
retained identity and a later Root install in an infrastructure-only plan. It
cannot authorize protocol work, duplicate creation, funding or a different
effect. The global stalled-observation budget resets only on genuine progress.

Primary owners:

- `crates/canic-host/src/icp/model.rs` — typed full/public ICP status shapes;
- `crates/canic-host/src/fleet_ensure/policy/mod.rs` — temporary-controller plan;
- `crates/canic-host/src/fleet_ensure/ops/platform.rs` — exact observations;
- `crates/canic-host/src/fleet_ensure/workflow/mod.rs` — bounded deferral; and
- `crates/canic-host/src/fleet_ensure/generate/tests.rs` — production-shaped
  fresh and immutable-plan replay evidence.

## CANIC-119-CANIC-123 First 0.110 Release Corrections

Published `v0.110.0` through `v0.110.3` close the fresh-estate corrections:
applied Creates retain exact nonterminal identities, cycles and topology;
pool readiness includes separately reviewed observation/update burn; and only
controller-authenticated `InspectCanister` is admitted while a Root is
Prepared. The concrete `IcpEnsurePlatform` proof crosses lost Create and
controller-update responses, reconstructs the adapter, finishes one Workload
plus one Ready pool asset and replays without repeating an effect.

Published `v0.110.3` also hard-deletes the obsolete temporary pool-Ledger
helper, contracts offline Medic/state-audit CI output and aligns the PocketIC
test stack. Exact sealed Root-module authority remains mandatory. Full
correction and test details are retained in the 0.110 changelog.

## CANIC-125 Bounded Component-Provisioning Observation

GitHub issue 23 reports a retained Fleet Ensure operation with 65 of 66 effects
Applied and the final `ProvisionComponents` action Issued. The Coordinator was
still progressing, but eight immediate identical status queries exhausted the
generic unchanged-observation limit before the distributed operation could
reach its next durable phase.

Published `v0.110.4` keeps the command/status boundary intact:

- the command is still issued once, and only exact typed retryable-failure
  evidence may replay its retained operation identity;
- passive `ProvisionComponents` status observations use bounded exponential
  pacing from 250 milliseconds to five seconds;
- the unchanged-progress limit is raised only for that exact protocol action,
  using a retained topology-derived floor capped at 64 while honoring an
  explicitly reviewed larger configured limit;
- any durable phase, Root-count, Component-count or failure change resets the
  consecutive-stall budget; and
- a true stall remains typed, resumable and reports the action plus compact
  durable progress evidence and its full status digest.

Retained current-schema plans and journals require no rewrite. This is a host
reconciliation correction and does not add runtime capability or alter the
0.110 contraction design.

## CANIC-124 Managed Component-Tree Qualification

Published `v0.110.5` adds one public host-only fixture for downstream tests
that must qualify a managed Hub together with children created through Canic's
placement workflows. `install_managed_component_group` consumes one validated
Component Group deployment and exact Wasms for all selected roles; Canic alone
derives each top-level `Component` and descendant `ComponentChild` authority.

The governed proof covers configured sharding and scaling children, on-demand
index and scale-out allocation, exact parent and Component Group bindings,
Fleet-admitted and denied direct child ingress, same-release child upgrade,
timer restoration and successor projection fencing. The same Root allocation
journal and fixture settlement path serves sharding, scaling and index; the
downstream never constructs protected child payloads or directly pins private
`canic-core`/`ic-testkit` lifecycle machinery.

## CANIC-126-CANIC-127 Convergence Corrections

Published `v0.110.5` also closes the two production ordering defects exposed
after the final Fleet protocol action:

- a Root-owned pool asset whose exact balance is not yet available remains a
  bounded passive observation, not a failed effect. Fleet Ensure re-observes the
  same operation and topology without issuing a protocol, install, controller,
  creation or funding command. Exact progress clears the stall count; exhaustion
  reports the target and last authoritative lifecycle while leaving the operation
  resumable; and
- a managed Hub keeps readiness closed while its configured initial children are
  unavailable. The exact registered Hub may request only its compiled initial-
  child allocation while both it and the Root are Prepared. One durable Root
  allocation owns creation, installation, Directory convergence and membership;
  a detached idempotent driver prevents a Root-to-Hub callback cycle, and the Hub
  retries only typed transient bootstrap failures within a finite bound. If that
  bound is exhausted, an exact Root-authenticated runtime-configuration replay
  may reclaim the transient init failure without rerunning application init;
  non-retryable failures and active retry owners remain unchanged.

Root membership activation now requires the target's exact readiness response.
Runtime activation remains bound to the Directory authority under which it
occurred even when an initial child legitimately advances the current Directory
before Root records the activation response. The Root therefore cannot publish
an Active zero-descendant Hub whose required initial-child bootstrap failed, and
lost activation responses adopt only the exact already-active runtime receipt.

Component retirement keeps the committed runtime operation resolvable during
the exact validated `Draining` interval before a quiescence intent exists. This
allows the Root to converge the final member Directory while the Component is
still runnable; quiescence intent, a stopped receipt and removal all close that
runtime-operation path again.

The governed Prepared-Root journey reaches three top-level Components plus
configured sharding and scaling children, terminal Component membership and an
effect-free replay. A second governed literal-zero-estate journey now drives the
real Fleet Ensure plan and journal through the concrete `IcpEnsurePlatform`, an
actual lost controller response, fresh-process adapter reconstruction and the
real Coordinator/Root/Store protocol. It reaches one Workload plus one Ready
pool asset, proves cycle conservation and immediately replans and applies with
zero effects. The public fixture independently covers configured and on-demand
sharding, scaling and index children, direct admission, same-release upgrade,
timer restoration and fencing. A downstream live reset remains downstream-owned
adoption evidence rather than a Canic release effect.

## 0.110.5 Fleet Ensure Operator Corrections

Published `v0.110.5` distinguishes a retained Component-provisioning source Registry
from its published active successor during terminal inventory validation. Root
top-level status is bound to the Coordinator's retained source Registry, plan
hash and configuration digest; Root and Coordinator publication remain bound
to the active Registry. Fleet Ensure JSON also keeps Store chunk bytes in the
existing content-addressed object store and reports only their local path,
SHA-256 and byte size. Text-mode cycle quantities use consistent three-decimal
`B`, `T` and `Q` units.

## Published 0.110.7 Endpoint Contract Correction

The maintained Canic surface no longer overloads one top-level method name
with incompatible request and response types. Ordinary managed roles retain
`canic_command` and `canic_status`; Fleet Coordinator, Fleet Subnet Root and
Wasm Store own `canic_coordinator_*`, `canic_root_*` and
`canic_wasm_store_*` command/status pairs respectively. Generic host tooling
selects the exact pair from the already verified role binding. This is a
pre-1.0 hard cut with no alias or fallback endpoint.

## Published 0.110.7 Validation Throughput

The ordinary runner uses libtest's default captured output; governed PocketIC
journeys alone retain live `--nocapture` progress. Its fast internal tier now
compiles without the large PocketIC fixture catalogue, while the serial lane
enables that catalogue explicitly. On the retained development cache, the same
six fast checks complete in 2.34 seconds and the full governed catalogue still
compiles independently in 10.37 seconds.

Release guards retain executable authority, security and current-schema
checks. Incidental source spelling, explanatory document layout and transitive
informational-advisory inventory drift no longer block a candidate; missing
authority documents, vulnerabilities, yanked packages and directly selected
unmaintained dependencies remain fail-closed.

## B1-B10 State

| Batch | State | Current evidence owner |
| --- | --- | --- |
| B1 | Accepted | 0.109 design baseline |
| B2-B7 | Complete | design/status tracker and governed admission suites |
| B8 | Complete | published CANIC-118 correction and downstream evidence |
| B9 | Complete | accepted immutable superseding audit |
| B10 | Complete | published host-only facade and downstream adoption report |

The immutable
[B9 superseding audit](../audits/reports/2026-08/2026-08-31/0.109-b9-superseding-complexity-audit.md)
reports `closeout_verdict: pass` on `v0.109.32`; the human maintainer accepted
it on 2026-08-31. The three control-plane parents remain 6,303, 5,838 and 2,688
lines. Canonical complexity and change-friction remain 8/10 and 7/10 pressure,
routed to blocked 0.110 rather than a second 0.109 authority. The accepted
audit's handoff snapshot was below its 250-physical-line ceiling; that
historical measurement is not a size claim about this live handoff.

Published `v0.109.33` completed the host-only `canic::testing` facade, isolated
packaged consumer and managed plus standalone-local lifecycle proof. Read-only
downstream evidence confirms adoption of that facade, removal of the private
payload adapter and direct `canic-core`/`ic-testkit` test dependencies, and a
passing exact managed-Wasm lifecycle. The
[B10 reconciliation](../audits/reports/2026-08/2026-08-31/0.109-b10-managed-app-qualification-reconciliation.md)
records the boundary.

The human-requested closeout audit against `v0.109.34` is retained at
[the canonical report](../audits/release-lines/0.109-closeout-audit.md). Its
CANIC-118, active-handoff and documentation blockers are correction inputs, not
an accepted verdict for that older candidate. Published `v0.109.35` corrected
those blockers, and the human maintainer accepted the 0.109 closeout on
2026-09-01 before explicitly promoting 0.110 B1.

## Roadmap Boundary

Toko Miner remains a read-only steering source. Canic gains no downstream
runtime or repository dependency.

| Line | Active owner | State |
| --- | --- | --- |
| [0.109](../design/0.109-fleet-wide-ingress-admission/status.md) | admission, Ensure and managed-App support | accepted and closed at `v0.109.35` |
| [0.110](../design/0.110-fleet-runtime-contraction/status.md) | zero-capability runtime contraction | `v0.110.7` published; bounded authorization-persistence split active without accepting B1 or the remaining B2/B3 families |
| [0.111](../design/0.111-bounded-multi-fleet-estates/status.md) | bounded cycle-safe multi-Fleet estates | blocked on 0.110 and Q0 capsule proof |

The cancelled stateful-adoption proposal remains archived. Pre-1.0 release
transitions are reinstall-only; cycle conservation is the sole cross-release
compatibility invariant. Same-release interruption recovery, idempotency,
backup, restore, authority and cycle-safe retirement remain mandatory.

## Active 0.110 B1

The accepted first batch freezes the post-0.109 artifact, tool and capability
baseline before any runtime contraction. Initial work:

- freezes `v0.109.35` (`3185dc45b`) as the Canic predecessor;
- confirms dated IC limits of 10 MiB code section, 100 MiB total module and
  50,000 replica-limited defined functions from the authoritative IC
  documentation and source;
- promotes `CANIC-WASM-001/v6` so path-confined staged release artifacts are
  measured from one role-local build log;
- retains the corrected deterministic eleven-role size baseline from immutable
  `v0.110.5`, whose largest role has 3,826,016 code-section bytes and 40,404
  replica-limited defined functions of absolute headroom;
- retains source inventories that separate the immutable generated role
  surface from the working overlay and classify all 39 Canic state allocations
  as reconstructable, reset-only or consumer-owned discard/reseed domains;
- proves that the temporary pool-Ledger recovery family remains absent from
  current product source while keeping its compatible artifact delta open;
- retains a machine-checked eighteen-row ablation catalog, fail-closed
  two-build harness and repository-owned function counter frozen to the IC
  replica's local-function quantity for the canonical roles and four owned
  fixtures, plus immutable all-role global-registration attribution removing
  273,554 artifact-summed optimized code bytes and 662 defined functions while
  leaving bootstrap/lifecycle parity open,
  immutable inclusive activation-persistence attribution removing 3,001,136
  artifact-summed optimized code bytes and 2,025 defined functions while
  preserving every role's Candid hash and leaving activation parity open,
  immutable authorization-persistence integration attribution removing
  1,628,872 artifact-summed optimized code bytes and 897 defined functions
  across all canonical roles, with the runtime fixture independently removing
  148,935 code bytes and 88 functions while persistence and authorization
  parity remain open,
  immutable shared-CBOR-helper attribution removing 9,332,399 artifact-summed
  canonical code bytes and 5,344 defined functions while preserving every
  Candid hash and leaving codec, persistence and runtime parity open,
  a canonical-plus-runtime-qualified watchdog-recovery dispatch switch
  and an endpoint-
  declaration-construction switch plus bounded endpoint-reply serialization,
  a specified metrics-provider switch and immutable payload-limited raw-
  adapter attribution that retains the safety path after measuring only 967
  optimized code-section bytes and zero defined functions;
  and
- keeps downstream pressure observations non-binding and separate from Canic
  source and release authority.

No broad workspace or full PocketIC gate is run during coding. The maintainer's
release flow owns that boundary. Published `v0.110.5` closes the independent
CANIC-124/CANIC-126/CANIC-127 qualification and convergence corrections, so B1
measurement may proceed; B2 remains blocked on accepted complete B1 evidence.

Published `v0.110.6` also closes an observability exposure discovered during
downstream review. Exact cycle balance/history/top-up values and raw
runtime metrics are controller-only on managed, standalone-local, Root and
Store status surfaces. Fleet `info list`, `info cycles`, `info metrics` and
terminal conservation retain operator access through existing Root authority:
native balances use Root management inspection, while managed runtime history
and metrics use a controller-authenticated Root relay. No human principal is
added as a managed Component controller, and Toko Miner remains downstream-
owned.

Published `v0.110.6` also corrects the `v0.110.5` Component Child response
regression. Child allocations requested by an Active parent now complete
through the existing durable Root driver before returning their canister ID.
Only initial bootstrap uses detached completion, because the Prepared parent
cannot yet serve the Directory convergence callback. This is a generic Canic
lifecycle correction; no downstream application behavior is embedded in
Canic.

Terminal Fleet inventory in published `v0.110.6` reconciles every Root
pool Workload against the complete protected Component tree, including nested
sharding, scaling and index descendants. Each physical workload must match its
exact Component ID, allocation operation, Root, parent, role and current
release module before terminal publication. The authority-derived pool bound
includes every permitted descendant. A retained Pool row may adopt a different
logical parent only when the terminal row is an observed, protocol-bound
Component; all other parent drift remains rejected. `canic info subnets` also
retains the caller's selected ICP executable and environment instead of
falling back to `local`.

## 0.110.7 Audit and Validation Hardening

Artifact builds now resolve the canonical `ic-wasm 0.11.1` and, for release,
Binaryen 132 before Cargo compilation or release-build planning. One admitted
absolute tool set serves the complete build, the canonical `~/.local/bin`
installation takes precedence over PATH wrappers, and the public toolchain
installer owns both checksum-verified downloads. Missing, mismatched and
root-level `HOME=/` cases retain actionable exact-path diagnostics.

The current Unreleased overlay fails terminal Component inventory closed unless
Root management inspection proves the canister is running, Root-only controlled
and on the exact current module. Authority, module, Directory, Registry-
principal, release-network, Fleet-admission, protocol-sidecar and generator
Candid-sidecar failures carry typed values, digests or paths through their test
boundary.

Cycle reporting now derives its observation plan from exact role and capability
records. Coordinator history is unavailable, unbound pool assets report a typed
balance-only result, Root and Store omit unsupported top-up calls, and eligible
automatic-top-up Components retain the complete history/top-up path.

Fleet Ensure also accounts for each Root's Cycles Ledger account independently
from operator funding. Its current plan forecasts the canisters required by the
selected Component action, recursive initial-child topology and configured
Ready-pool floor, while crediting reusable Ready assets and completed
workloads. It reports raw account balance, creation amount and count, Ledger and
management fees, maximum plan-owned funding and shortfall. The reviewed plan
places an exact `FundEstate` Ledger transfer before protocol work, persists its
intent before debit, adopts an exact duplicate receipt after response loss and
proves both operator debit and Root-account credit. Immediately before the
first Fleet protocol effect, apply re-reads the exact Ledger/Root account;
unexpected underfunding becomes a durable typed no-effect pause and an
unchanged retry does not rewrite the journal or call the control plane.
Terminal conservation reconciles actual plan-owned Root-account funding and
actual protected creation receipts separately from operator funding of managed
canisters.

Coordinator observation no longer assumes it sees each Root provisioning step.
It accepts a strictly advanced canonical cursor, including one that also
reports the exact typed estate-funding pause, and records that progress before
returning the pause. Unchanged, regressed and noncanonical cursors still fail
closed. The governed five-Component journey supplies an exact Root Ledger
balance and genuinely funded creation results; it reaches terminal provisioning
without depending on a transient conflict or zero-cycle fixture shortcut.

Autonomous creation funding includes the generated 1T execution margin above
the Ready floor and management creation fee. Root retains each exact creation
operation, Ledger block, amount, fees, policy and first live native balance;
the first observation bounds pre-ready burn and cannot be rewritten by later
inspection. Planning pages the complete protected pool inventory, so every
dynamic asset and pending creation consumes capacity before any funding action.
A capacity blocker discovered after a reviewed plan's effects have all applied
retains its typed detail and durably closes that operation as replan-required,
so a later plan cannot replay the completed infrastructure or funding effects.

Behavioral PocketIC evidence replaces the removed source-text observability
guard. The focused proof checks exact diagnostic codes and response variants for
Root, Store, standalone, managed and relayed calls, including a capability-
accurate automatic-top-up fixture and the restored active-Registry baseline that
failed the prior full run. Governed progress emits `CANIC-TEST:E001`, allowing
the validation runner to surface failure events without parsing presentation
wording. The B1 ablation helper likewise emits a typed transform-metrics v1
record instead of requiring optimizer log prose or changing the historical
product helper it measures. Its separately resolved offline lock and exact
experiment patch hashes are machine-bound. A development-only row-3 canonical
App smoke passes for both patched and baseline artifacts against immutable
`v0.110.5`, with the historical source tree and product lock restored exactly;
it is intentionally not retained B1 evidence. The subsequent retained all-role
run passes 44 clean artifact builds, structured transform checks and complete
determinism, and records the material inclusive activation-persistence result.
The retained row-4 run likewise passes 48 clean builds and complete
determinism across the canonical roles plus `runtime_probe`, preserves every
Candid hash and records material repeated authorization-persistence integration
footprint without claiming persistence or authorization parity.
The retained row-5 run passes 52 clean builds and complete determinism across
the canonical roles plus `runtime_probe` and `blob_storage_probe`, preserves
every Candid hash and attributes 9,332,399 artifact-summed optimized code bytes
and 5,344 defined functions to the shared bounded CBOR helper's canonical-role
reachability. Its intentionally overlapping build-only stub proves no codec,
persistence, restore or runtime parity; it prioritizes role-selected storage
and residual-codec remeasurement rather than direct production deletion.
Row 8's first development qualification passed ten canonical artifacts before
the Wasm Store's production sidecar comparison correctly rejected its
deliberately changed counterfactual Candid. The audit switch now isolates that
comparison without weakening the production builder, and an exact Store
qualification passes. The full selector has not passed against the current
patch identity, so row 8 remains `specified`; narrowed qualification is
development-only and cannot retain a measurement.
Row 10's exact canonical-App qualification also passes artifact, gzip, Candid
and structured-metric validation. Its complete selector and immutable paired
measurement remain open, so row 10 remains `specified`.
Row 12's exact canonical-App qualification passes the same artifact boundary;
its complete selector, metrics behavior proof and immutable paired measurement
remain open, so row 12 remains `specified`.
Fleet plan persistence regressions likewise decode structured JSON and assert
named values or absence rather than serializer text.

Role-contract validation now derives Root chain-key signing from actual
delegated-token issuer configuration and rejects surplus cryptographic features
with a typed finding. The workspace selects the existing IC-stack `k256 0.13.4`
family only, eliminating the parallel 0.14 cryptographic dependency family. A
target-Wasm dependency gate checks every canonical deployed role for duplicate
cryptographic package versions and proves all eight zero-auth roles contain no
signature stack. Only Root and the two canonical roles that exercise token
verification or issuance retain authentication cryptography. Optimized
capability reports retain cryptography as a separate measured category so later
symbol-level regressions remain visible.

## Open 0.110.8 Capability-Owned Authorization Persistence

The maintainer explicitly authorized this bounded vertical slice without
accepting complete B1 or opening the remaining B2/B3 state families. The former
aggregate authorization record, stable cell and storage facade are hard-cut
into three exact owners:

- local application authorization owns sessions, replay fences and the local
  authority generation;
- a delegated-token issuer owns only its installed active proof; and
- Root owns issuer policy, proof and Registry epochs, renewal state and
  chain-key batches.

Verification alone owns no persistence. Coordinator and Store select no auth
allocation, and the facade no longer activates all control-plane role defaults
through its optional dependency. Root still authenticates operator calls with
controller authority and protocol calls with the exact Coordinator Principal
in its protected Fleet activation binding; neither path depends on local
application authorization.

Typed role-contract tests prove the exact allocation and missing-feature
values. Stable-state tests cover each record owner and bounded local-session
capacity without exact serialized-byte assertions. Targeted optimized builds
show no auth declaration in Coordinator or the verifier-only runtime probe,
only Root delegation state in Root, and exactly issuer plus local-application
state in their combined fixture. The governed seven-test native authorization
and delegation PocketIC target passes, including same-release restoration
rejection after corrupting the selected issuer cell. These results do not claim
that unrelated runtime storage has been contracted or that the full B1/B2
artifact matrix has been remeasured.

The same open patch advances only the isolated lifecycle-composition fixture to
the exact published IcyDB 0.253.0 family with default features disabled.
Production Canic crates remain IcyDB-free, and the governed dependency check
now compares parsed Cargo package and dependency fields instead of source
formatting. Targeted host, lint and Wasm builds plus the exact PocketIC
composed-ingress journey pass against the new family.

### Downstream feedback disposition

The current patch candidate addresses the Canic-owned launch blockers recorded
as `CANIC-007`, `CANIC-132` and `CANIC-133`: funding is plan-owned and
replay-safe, autonomous creation has non-zero execution margin and exact
first-observation evidence, and planning uses the complete protected pool
inventory before authorizing a debit. Reviewed native funding and Root
reconciliation for exact retained Failed/PendingReset identities now have
production-host repair, conservation and replay evidence linked below.
Immutable release and downstream replay remain closure boundaries. `CANIC-129` is covered by an isolated
packaged-consumer lock-resolution proof; the public testing facade remains the
only supported downstream testing boundary. `CANIC-130` is already corrected
in Canic; the remaining wrapper replay is downstream evidence rather than a
second Canic owner.

The non-blocking product requests stay explicit without expanding this release:
the Fleet observatory (`CANIC-002`) and frontend delivery handoff (`CANIC-008`)
retain their existing deferred designs; operator top-level Component lifecycle
(`CANIC-010`) and a long-running multi-Subnet local Fleet (`CANIC-017`) now have
separate unnumbered design homes. Release-evidence truth (`CANIC-014`) remains
governed by structured release metadata, while release-build cost
(`CANIC-087`) remains active B1 work. None of these deferred surfaces is a
prerequisite for the current cycle-safe Fleet Ensure correction.

## Architecture Consolidation Audit Update

The commit-bound
[architecture consolidation audit update](../audits/reports/2026-09/2026-09-03/architecture-consolidation-audit-update.md)
confirms at `6cad3dcc568e9309f6294d324cc97d0b75c31008` that Canic retains one mutable
Fleet authority. Its highest-priority remaining duplication is supporting
machinery: release-validation shadow specification, parallel allocation
mechanics, fragmented Fleet Ensure test platforms, repeated CLI fan-out,
controller-set normalization and host path resolution. Narrow role-specific
Candid fragments remain intentional and should gain conformance evidence rather
than one complete shared enum.

This review does not expand or interrupt B1. After B1 acceptance, its preferred
order is validation manifest, Ensure test platform, Component transition kernel,
CLI/path utilities and controller-set normalization. Store-local GC ownership
and an await-safe Root validation-context pilot remain deferred inputs.

## Historical 0.110.8 Validation Follow-up

The accepted `CANIC-007` and `CANIC-132`–`CANIC-138` correction batch is
implemented, with release validation still pending after the fixture
corrections below. The latest maintainer run passed the corrected
fresh-provisioning case and the generated nineteen-Workload/five-Ready journey,
then was stopped after more than an hour. The complete release gate has not passed.
The next boundary is the maintainer-approved governed release flow; no version,
Git publication, broad validation, deployment or 0.111 action is inferred.
Package versions remain 0.110.7 and the complete open changelog draft is 0.110.8.
Initial closeout changed only documentation and evidence. Subsequent test-only
Clippy corrections use explicit `HashSet::default()`, extract JSON funding
fixture setup and release a PocketIC fixture after its last use. Combined
all-target/all-feature Clippy passes for `canic`, `canic-host` and `canic-tests`;
the changed JSON regression passes. Runtime behavior and contracts are unchanged.
The release attempt exposed missing test-target lint coverage in the earlier
closeout: dependency-library checks did not lint the owning packages' tests.
CI/deployment governance now requires explicit affected-target coverage and
collection of independent failures with `--keep-going`. The governed release
gate still needs to complete; this scoped pass is not a workspace validation.
The subsequent `canic-core --lib` failures are also corrected: the reservation
hash vector includes the current asset recipient, config validation discovers
its inventory without a fixed count, and the Coordinator manifest declares
`Retire` with its existing replay and value-transfer guards. All 32 affected
core regressions and all-feature core library/test Clippy pass. Retirement
execution and Candid are unchanged; the retained closeout check records include
these final-source results.
A later placement-index test failure came from comparing separate clock reads
across a second boundary; eleven subsequent failures were poisoned-lock fallout.
The test now bounds the returned release time by the recovery start/end times
while retaining exact authority and removal assertions. All 64 affected
placement/auth/intent tests and all-feature core library/test Clippy pass.
Production source and the seam-lock policy are unchanged.
The subsequent ordinary-integration failures are corrected: the facade README
documents `auth-local-application-authorization`, and the Ledger cost guard
checks each named paid adapter's mandatory permit type through Rust syntax
instead of counting permit spellings across the file. Both affected targets
pass (14 tests); all-feature Clippy passes for the changed cost-guard target.
Runtime and package manifests are unchanged. The broader Clippy attempt lost
dependency artifacts when `target/` disappeared; the focused retry passed.

The latest release attempt exposed a fixture controller mismatch in fresh
provisioning: pool maintenance and import commands, plus their status queries,
used the Store installation controller against a Root controlled by the default
PocketIC identity. Those calls now use the existing Root helpers; Store calls
retain their distinct controller. Production authorization is unchanged.
All-feature library/test Clippy passes for `canic-testing-internal`. The fresh
provisioning journey and the earlier `ICP_ENVIRONMENT` wrapper correction passed
in the maintainer's subsequent run; generated reinstall took 8m23s and generated
nineteen-Workload convergence plus replay took 17m06s. That interrupted run does
not qualify the remaining suites.
Earlier journey logs and selected source digests describe their retained
checkpoints, not qualification of these subsequent fixture edits.

The test-flow correction moves expensive Fleet journeys after the shorter
internal regressions in the same process, removes the
duplicate reporter-regression invocation, and makes `test-wasm` select only its
advertised fast integrations. `make test-pocketic-case CASE=<exact-test-path>`
runs a focused regression through the existing runner. This improves development
selection and time to useful failures; it does not establish a shorter complete
release time. Repeated full-estate setup and production-adapter observation
costs remain performance work. No long PocketIC rerun was started for this change.
Both catalogue regressions, all-feature internal library/test Clippy, ShellCheck,
the release-integrity guard and exact-case/fast-lane command-plan checks pass.
The standalone one- and five-Workload fresh journeys are removed as repetitions
of the generated nineteen-Workload/five-Ready journey's exact recovery and replay
path. The one-Workload fresh setup also remains inside generated reinstall;
small-topology activation, funding and Failed-asset recovery remain distinct
registered proofs. Retained runs spent 4m32s and 9m08s respectively on the removed
journeys; removing that work is not a measurement of the complete revised gate.

Further performance corrections remove the duplicate infrastructure-status scan
inside protocol planning: each ready Coordinator, Root and Store is now read once,
with running state and module identity checked against that same response. The
next call still observes fresh state, and transport failures retain precedence
over missing-owner/module decisions. Two focused transport regressions pass.
The reinstall fixture now caches both complete sealed artifact sets under two
fixed, distinct test release identities. A private-cache regression passes for
separate keys and exact first-release restoration after acquiring the second;
production release identity allocation is unchanged. Combined all-feature
library/test Clippy passes for `canic-host` and `canic-testing-internal`.
These remove known repeated work but do not establish a new full-suite duration.
The generated reinstall journey has not been rerun after this cache correction;
its existing authority, interruption and effect-free replay assertions remain.

Terminal inventory now overlaps independent Component and descendant status
reads in batches of four. It retains fresh observations, exact authority checks,
cycle accounting and deterministic inventory order. Each issued batch is joined
before returning a typed failure; failures prevent scheduling later batches.
All 18 focused inventory regressions pass, including bounded concurrency,
complete batch drainage, first-input error selection and fresh-read tests.
Scoped all-feature host library/test Clippy passes. The retained inventory
qualification is `/tmp/canic-bounded-inventory-pocketic.log`; both catalogue
regressions also pass after the duplicate fresh journeys were removed.

Funding tests now prepare real infrastructure and Ready imports through the
production Ledger/initialization adapters and public Root commands. They begin
host review at funding or repair, rather than repeating fresh infrastructure
recovery. No completed host journal is manufactured. Full generated fresh and
reinstall journeys retain their existing startup/recovery paths.

The focused refill rerun reproduced the earlier 24-call maintenance failure:
four assets existed, but the last was still PendingReset. The host had reset
backoff after every successful maintenance call even when pool state was
unchanged. It now retains unchanged-observation backoff until visible progress,
without raising the update budget. Funding fixtures use production pacing.
The focused restart regression proves increasing backoff, durable lost-call
accounting, budget exhaustion and terminal observation at the bound. All five
focused continuation authority/review regressions also pass. Combined all-feature
library/test Clippy passes for both changed packages.

Observed case timings, compared with the preceding maintainer run:

| Case | Previous | Focused result |
| --- | --- | --- |
| Estate funding and autonomous creation | 3m41s | PASS, 171.19s |
| Four Workloads plus four Failed assets | 5m29s | PASS, 186.95s |
| Four Workloads refill four Ready assets | 5m18s | PASS, 208.90s |

The final funding/refill runs include 27.73s/29.12s of sealed-artifact rebuilds;
the repair run used cached artifacts and preceded the maintenance-only host
correction. Its reviewed repair path contains no maintenance action. These are
local observations, not controlled benchmark pairs. Exact funding, fee rejection,
lost-response recovery, conservation and terminal replay assertions remain in
their relevant cases. Logs are `/tmp/canic-prepared-funding-one.log`,
`/tmp/canic-prepared-funding-repair.log`, `/tmp/canic-prepared-funding-refill.log`,
`/tmp/canic-maintenance-backoff-tests.log` and
`/tmp/canic-maintenance-backoff-clippy.log` and
`/tmp/canic-prepared-funding-continuation.log`.

The fixture/backoff correction is ready for review and the existing 0.110.8
changelog draft includes it. The complete performance batch must not yet be
reported as meeting a sub-hour release target: complete-gate runtime remains
unmeasured. No broad suite, version transaction or publication was performed.

The maintainer's final contract is explicit pre-1.0 reinstall. Application state
and former host records may be discarded while cycles and asset control remain
accounted for. Whole-Fleet evacuation applies only when deleting the Fleet.
The earlier classification of complete deletion as a release blocker was too
broad and is withdrawn. Custody, pinned predecessor readers, superseded
Store-adoption handling and compatibility ordering are removed. No replacement
recovery mode or redesign is part of this batch.

The [generated reinstall journey](../audits/reports/2026-09/2026-09-05/fleet-reinstall-journey.md)
passes in 522.04s. Generation avoids protected queries on a different Root
module. A reviewed three-effect `root_reinstall_prerequisite` uses the existing
intent/version journal; it survives an actual lost install response and fresh
adapter reconstruction. A separately reviewed `full` plan completes 18 effects
through bounded successor phases and reaches a working Fleet. Reset replay,
original full-plan replay and newly planned replay are effect-free. Exact
controllers and the Root's 1B Ledger balance are retained; aggregate native
debit remains below the fixture's 10T bound. The host now accounts for installed
PendingReset assets and defers creation funding while infrastructure and import
reconciliation restore owned capacity.

Final focused evidence:

- 138 Fleet host tests, 47 selected Fleet CLI tests and scoped warning-denied
  Clippy pass after the final host/policy/fixture changes.
- Five canonical Coordinator Candid tests pass, including structural Rust/DID
  equality for the required Ledger receipt. DTO/Candid source is unchanged
  since that run. JSON includes current scope and exact reinstall bindings;
  CLI reports distinguish prerequisite completion from full Fleet readiness.
- The final [Coordinator preflight checkpoint](../audits/reports/2026-09/2026-09-05/artifacts/fleet-retirement/coordinator-preflight-pocketic.log)
  passes in 142.74s: fee rejection permits a valid request, Root and Coordinator
  Ledger transfers reconcile both lost replies, replay is effect-free, and
  returned assets remain controlled after Root deletion.
- Earlier qualified outcomes remain retained: complete nineteen-Workload plus
  five-Ready supply, four-Workload refill, full-capacity four-Failed repair,
  selected authorization storage and seven native auth/delegation cases,
  shared funding-policy admission and bounded continuation. Their exact
  checkpoint boundaries are in the readiness report.

[Check records and selected source digests](../audits/reports/2026-09/2026-09-05/artifacts/fleet-reinstall/checks.json)
retain the final logs and reviewed source. This is targeted working-tree
qualification, not an immutable published validation receipt or B1 acceptance.
No broad suite was pre-run during coding.

Toko Miner must pin the published release, rebuild its sealed artifacts,
reconcile staging capacity with its complete generated topology, verify actual
controller/seed/cycle authority, and rehearse the reviewed reset followed by
full Ensure convergence. It must validate its application initialization,
authorization, required descendants and Ready reserve, interruption recovery,
cycle accounting and both effect-free replays. The Canic nineteen-plus-five
proof is not a live Toko adoption result. Toko remained read-only.

Optional follow-up, not a blocker for this release: complete Coordinator native
evacuation/deletion and whole-Fleet terminal conservation if Fleet deletion is
requested; CANIC-139 whole-release reuse; further build/deployment performance
work. The existing Coordinator Ledger receipt alone does not authorize deleting
its still-funded canister. Live deletion applicability remains downstream work.

The [readiness handoff](../audits/reports/2026-09/2026-09-05/fleet-feedback-readiness.md)
owns the full requirement/evidence map and release/downstream/follow-up
classification. The [hard-cut review](../audits/reports/2026-09/2026-09-05/fleet-hard-cut-review.md)
owns the design disposition. Historical custody artifacts qualify only removed
code. The [complete five-plus-five comparison](../audits/reports/2026-09/2026-09-05/fleet-feedback-performance.md)
retains the 27.86% apply-to-readiness and 33.47% apply-through-replay reductions;
these are one controlled local pair, not mainnet timing or unchanged-build reuse.

After the correction batch, continue B1 from immutable `v0.110.5`:
qualify row 8 endpoint-declaration construction and rows 10 and 12
endpoint-reply serialization and metrics-provider attribution, then complete
the controlled ablations, optimized generated-surface proof, generic-
instantiation cohort, accepted allowances and required compatible predecessor
comparisons. Row 3 is an inclusive build-only attribution with no activation-
parity or isolated-codec claim; row 4 preserves the auth call graph and same-
execution heap behavior but makes no persistence or authorization-parity
claim; row 5 covers every reachable shared-helper caller but leaves direct CBOR
uses unchanged; row 6 leaves timer custody and ordinary maintenance intact but
makes no watchdog-recovery parity claim; row 8 retains runtime endpoints and
wire serialization but makes no Candid or profile-metadata parity claim. Row 7
remains fail-closed until an exact expanded-source/projection counterfactual is
frozen without bundling the provider ablations. Row 9 remains fail-closed
because Canic has no derivation-level type-documentation suppression. Row 10
retains typed request decoding, endpoint execution, exact Candid and exports,
but makes no reply or wire-parity claim and leaves direct inter-canister
encoders intact. Row 11 retains inspect-message registration and endpoint
dispatch but removes the raw predecode/copy/reply adapter, so it makes no
complete payload-safety or canister-origin-call parity claim; its immutable
measurement retained the production path. Row 12 retains metric recording and
the typed status protocol while
disconnecting read-side snapshot/projection providers, so it makes no metrics-
behavior parity claim. The source inventories distinguish
reconstructable state from reset-only and consumer-owned reseed domains, and
the working-tree audit fixture freezes the generated `Page<T>` cohort at
`N = 5`. Its immutable optimized deltas and named post-`-Oz` mapping are still
required. The exact Aug-31 downstream artifact remains hash-bound pressure
evidence only; its source and application release policy do not gate Canic. Do
not begin B2 until the maintainer accepts the complete B1 baseline.










<!-- canic-release-validation: version=0.110.12 source=ae3f972ee9196c1a43b5e338fd9084b8564a29f8 date=2026-09-08 gate=complete -->
