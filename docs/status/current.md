# Current handoff — 2026-10-02

Review progress, closure-count limits and remaining owners are summarized in
[the code-review status](../code-review/status.md).

## Dependency lock and embedded-fixture qualification — 2026-10-02

The reported crypto-closure and dependency-risk failures shared one cause:
the manifest requested ic-query 0.44.2 while the lockfile retained 0.44.1.
Reconcile only that package with `cargo update -p ic-query --precise 0.44.2`;
preserve the maintainer's testkit 0.10.4 and optional IcyDB selections. Both
reported gates now pass: 12 canonical Wasm roles have valid crypto closure and
the dependency audit reports zero vulnerabilities with two reviewed warnings.

Compiling the new query version exposed its added `HistoryCache` progress
variant. Host now preserves its path, disposition, watermark and optional reason
as the `history_cache` JSON phase; CLI progress renders those diagnostics.
Canic's caller-owned source still does not opt into disk history reuse. The
root and detailed 0.110.50 notes reflect query 0.44.2 and testkit 0.10.4.

Qualification passes 10 Host catalog tests, three CLI catalog tests and
warning-denied Host/CLI library/binary Clippy. The embedded peer refreshed in
the prior turn verifies successfully against the final graph. The exact public
managed-component lifecycle PocketIC case passes in 166.24 seconds; the governed
runner exits successfully after 309 seconds including build and cleanup. This
qualifies that journey with memory 0.15.3, query 0.44.2, testkit 0.10.4 and timers
0.8.1. Formatting, document semantics and whitespace checks pass.

Evidence: `target/review-validation/lock-reconcile-*` and
`target/test-runs/20261002T101700Z-4862.niWBgk/1.log`. The earlier offline-cache
and missing-variant failures are superseded by these final passes. The CLI binary
selector contained no tests; the subsequent library selector ran all three
catalog tests. The corrective dependency/fixture work is complete for review;
no broad validation, version transaction, commit, push or deployment ran here.

## Gitleaks flow removed — 2026-10-02

At the maintainer's explicit request, remove the Gitleaks target from local and
CI validation, developer installation/update paths, CI tool setup, version and
checksum pins, dedicated release-tool fixtures, installer, scanner script and
fingerprint exclusion file. Active setup, platform, validation and CS1 installer
documentation now matches the maintained tools. Release-integrity audit revision
2 removes mandatory scanner evidence while retaining credential-handling review;
the catalog preserves the historical revision and existing scan reports.

Targeted shell syntax, ShellCheck, actionlint, release-integrity authority,
audit-method catalog, validation-matrix, document semantics and whitespace checks
pass. Make dispatch with a print-only runner confirms local/CI validation retains
the remaining gates. Release-tool fixtures pass in an isolated copy, excluding
the unrelated tag-deletion fixture because it creates Git commits. Evidence:
`target/review-validation/gitleaks-removal-{validation-dispatch,release-tools}.log`.
The removal is complete and included in the open 0.110.50 notes. No Rust build,
broad validation, version transaction, commit, push or deployment ran.

## Testkit follow-up — 2026-10-02

Canic's manifest permits ic-testkit 0.10.2 and its lockfile now selects registry
0.10.3. The upstream release changes only the teardown proposal/probe and related
documentation; production crate source is unchanged from the classifier fix in
0.10.2. No dependency edits were made by this check.

The exact locked 0.10.3 library builds successfully in 6.57 seconds. Three fresh
public-API probe tests pass: application/quoted text and bare I/O refusal do not
trigger dead-transport recovery; maintained refused/incomplete/channel-closed
instance request shapes still qualify; consumer error wrappers preserve both
positive and negative classification. Logs and source are retained under
`target/review-validation/testkit-0103-*`. Direct Cargo dependency unit testing
was unavailable because the package is not a workspace member, so the successful
regression probe links the freshly built registry library instead.

0.10.3 does not fix production PocketIC teardown. Its candidate upstream patch
adds bounded fallible shutdown and has seven synthetic parent tests reported
upstream; Canic still uses unmodified registry PocketIC 16. Persistent-state
handoff after unconfirmed deletion requires further review, and the original
Busy/tick cause remains unproven. No live PocketIC or full Canic suite ran here.
The classifier feedback is resolved; teardown must remain open in the tracker.

## Upstream feedback recheck — 2026-10-02

Read-only sibling review confirms Canic now selects registry releases ic-memory
0.15.3, ic-query 0.44.1, ic-testkit 0.10.2 and ic-timers 0.8.1. This supersedes
the feedback dispositions in the earlier 0.15.2 checkpoint below, not its exact
qualification record. No dependency or upstream repository edits ran here.

- Memory 0.15.3 fixes diagnostic-triggered default runtime construction. A fresh
  public-API probe against Canic's existing exact-version rlib confirms export,
  commit-recovery and both doctor helpers return typed `NotBootstrapped` before
  bootstrap, then allow configured 16-page bootstrap. All four cases and the
  fresh-thread control pass. Source and log:
  `target/review-validation/upstream-feedback-recheck-memory.{rs,log}`.
- Query 0.44.1 still writes an export before its dry-run cache-write branch and
  opens that output with `File::create`; an alias can overwrite the cache. The
  upstream working tree has the alias fix and records successful focused/full
  tests, but that fix is uncommitted and absent from the selected release.
  Canic's loader does not supply an output path, so the triggering combination
  is not exposed there. Persistent acquisition checkpoints across CLI processes
  remain a performance opportunity; no new live timing claim is made.
- Testkit 0.10.2 fixes the generic-text dead-transport false positives. Source
  and upstream tests cover negative application/quoted messages and positive
  transport evidence. PocketIC 16 synchronous teardown waiting is now reproduced
  in isolation, not fixed; the original Busy/tick cause is still unproven.
- Timers 0.8.1 changes tooling, lints and docs without scheduling logic changes.
  No new runtime issue was found. Its handoff's claim that 0.8.1 is unpublished
  is stale relative to the release commit and registry selection.

The [review status](../code-review/status.md#upstream-feedback-recheck--2026-10-02)
tracks these dispositions. This was source review plus the small memory probe,
not combined Canic lifecycle qualification of the new dependency graph. Prior
memory 0.15.2 / timers 0.8.1 lifecycle evidence remains correctly scoped below.

## ic-memory 0.15.2 and upstream feedback — 2026-10-02

The maintainer selected published ic-memory 0.15.2 and feedback notes here,
without upstream issue publication. The workspace dependency, lockfile, runtime
guides and existing 0.110.50 changelog draft now select 0.15.2. Package versions
remain 0.110.49. This supersedes the earlier 0.15.0 adoption checkpoint below;
the upstream maintenance patches require no Canic API or allocation-policy change.

Qualification passes 35 focused memory tests, 70 receipt tests, four stable-memory
ABI/identity guards and Core library/test all-feature Clippy with warnings denied.
These native checks used timers 0.8.0. The refreshed embedded peer and its structured
provenance qualify the final graph with the concurrent timers 0.8.1 selection;
the exact public managed-component lifecycle PocketIC case passes in 201.44 seconds,
including six freshly built Wasm fixtures; the governed runner completes in
285 seconds with cleanup. The selected production/peer Wasm graph
contains one ic-memory identity, 0.15.2. Documentation semantics, scoped diff checks
and the 0.110.50 draft preflight pass.

Earlier lifecycle attempts stopped at sandbox loopback restrictions or concurrent
manifest/lockfile changes. The final permitted run uses consistent current inputs.
Locked packages were fetched for offline metadata after another session changed
optional IcyDB entries; this work did not change those versions, build or repair
IcyDB composition, or mutate sibling repositories. Optional composition remains
separately unqualified and does not block Canic-owned adoption.

Updated upstream feedback remains reproducible through public APIs:

- Memory 0.15.2: default export/recovery/doctor diagnostics before bootstrap
  construct 128-page buckets, then configured 16-page bootstrap rejects with
  `BucketSizeMismatch`. Export first returns `NotBootstrapped` despite this effect.
  A fresh-thread control bootstraps 16 pages successfully. Request nonconstructing
  inspection and explicit configuration ownership for prebootstrap diagnostics.
  This is an upstream API hazard, not an observed failure in Canic's normal startup.
- Testkit 0.10.1: dead-transport matching accepts unrelated application channel
  closure and quoted `ConnectionRefused` text. Narrow recognition to transport
  evidence; the existing upstream classifier issue already owns this feedback.
- Query 0.44.0: a dry run whose output path is the managed catalog overwrites that
  valid catalog while reporting `wrote_catalog=false`. A distinct-output control
  preserves it. Reject managed-path aliases before writing.
- Timers: four focused registration/deadline/ownership tests pass. The 0.8.1
  production-source changes are lint annotations; no additional runtime defect
  was found in this review.

Evidence: `target/review-validation/ic-memory-0152-*`, including the feedback log,
retained probe source, timer review and final PocketIC log. Complete lifecycle output
is in `target/test-runs/20261002T083700Z-61887.pWYK00/1.log`.
The dependency-adoption batch is ready for maintainer review/push and its changelog
surfaces are complete. Publication still requires the selected version/release
transaction. Preserve concurrent corrective batches. No broad gate, Canic version
transaction, commit, push, upstream posting or live deployment ran.

## Deployment and release guard cleanup — 2026-10-02

The maintainer authorized correcting trivial deployment refusals after the
guard audit. This cleanup batch is complete in the open 0.110.50 draft; package
versions remain .49 and all changes are uncommitted.

Retained-operation selection now recognizes owned records and evidence instead
of treating arbitrary directory entries as an installation. Metadata, notes,
backups, temporary writes and empty evidence directories no longer require a
reset. Incomplete owned records and paid-effect evidence retain their recovery
and explicit-inventory requirements. Root Ledger-account surplus is accepted
at admission and terminal conservation, including no-creation operations;
observed net credit never expands reviewed authority or replaces exact receipts.
Unexplained deficits, wrong custody, excess creations and uncertain payments
still refuse or require their existing recovery owner.

Release integrity now checks authority records and executable behavior instead
of freezing shell/Make source spelling. CI authority is parsed as YAML; product
generations are checked through Rust syntax, excluding comments, rustdoc and
test-only input. Audit-method fingerprints remain an explicit audit lane rather
than a deployment/release blocker. Passing release-tool fixtures capture their
expected rejection diagnostics.

All 17 selected Rust regressions passed, including exact creation receipts,
transfer-loss recovery, surplus accounting and metadata selection. Focused Host
and Canic Clippy passed with warnings denied. The complete targeted
release-integrity-contract gate, ShellCheck, formatting, document semantics and
test-inventory checks passed. Evidence: `target/review-validation/deployment-guards-*`.
No broad validation, version transaction, Git publication or live deployment ran.

This guard-cleanup batch is ready to push. The combined 0.110.50 draft also
contains concurrent batches whose qualification is tracked below; preserve them.
Root and detailed changelog notes are updated; package publication still requires
the maintainer-selected version/release transaction.

## R2 import efficiency and CANIC-190 — 2026-10-02

The maintainer explicitly selected R2 import-call reduction and adjacent CANIC-190.
Both implementations and their direct qualification are complete in the open
0.110.50 corrective draft. Package versions remain .49 and changes are uncommitted.

Root continues stop/controller confirmation into the next mutation from the
same final status sample, with no intervening await before durable intent. Fresh
mainnet placement is still required on every resumed advance and controller/
uninstall history remains exact. Admission derives 13 calls per running source
plus one terminal Root read; recommended retries derive 26 per source plus 16.
Eight sources require 105/224 minimum/recommended, down from 137/272. A 72-call
envelope still refuses before handoff. No issued Root authority is expanded.

`fleet recover-attempts` reviews exact exhausted Host owners without IC calls and
grants two attempts per resource only after exact-digest approval. Surveys,
submissions, inspections and retired handoff envelopes retain consumption,
original approvals, ingress and balance baselines. Changed owners refuse before
publication; partial publication and approval replay retain each grant once.
Current v1 owner records require `attempt_recoveries` arrays, through the existing
reinstall-only hard cut. Root cap exhaustion still requires cycle-safe reset.

Qualification passes three Core budget tests, 32 Control Plane import tests,
88 Host import tests, three Host recovery tests, bootstrap qualification,
121 focused CLI tests and recursive help checks. Warning-denied all-target/
all-feature Clippy passes for the five affected packages. Native continuation
proofs retain spent counters, Root caps and retired-envelope history, reject
changed owner files, and resume partially published grants exactly once.

All three focused PocketIC cases pass: mainnet-shaped eight-source call/debit
measurement with placement drift, retained running/stopped IDs and reset receipts,
and signed HTTP handoff recovery after Host submission exhaustion. The clean
eight-source run uses 105 paid calls in 2,087 ms with 44,571,742,321 observed Root
debit cycles. The placement-drift run refuses the mutation and completes with
106 calls in 2,026 ms and 44,616,376,394 debit cycles. These are individual
PocketIC measurements, not a comparative latency benchmark. The previous
137-call baseline derives from the original workflow. Applied journal effect
counts are unchanged; the reduction removes observations.

Evidence: `target/review-validation/r2-*`. The active R2 tracker records this
bounded outcome without closing unrelated R2 findings or conditional placement
caching. R2 plus CANIC-190 is ready for maintainer review/push, with its changelog
surfaces complete; package publication still needs the governed version/release
transaction. Combined 0.110.50 readiness also depends on the concurrent batches
tracked here, including CANIC-192 qualification. Preserve those edits and serialize
target validation. No broad gate, version transaction, Git publication, sibling
edit or live effect ran.

## Toko input continuity: CANIC-192 — 2026-10-02

CANIC-192 is implemented and qualified. Ensure's argument arrays and human
next-command retain desired, policy, seed and identity through apply/resume/import/
successor review. Policy/seed flags are accepted on apply without requesting a new
reset. Omitted retry paths inherit the retained selection. Explicit destination
changes reject before paid work with both paths in the typed error; equivalent
existing path spellings remain valid. The Host checks path continuity when
reviewing the next phase. Typed successor errors use the resolved reset selection.

Final checks pass: 82 Fleet CLI tests, 23 Host operation-selection tests (one
existing manual test ignored), and warning-denied all-target/all-feature Clippy
for CLI, Host and Testing. Scoped formatting, document semantics and the 0.110.50
release-draft preflight pass. The extended public CLI PocketIC journey passes in
481.48 seconds (484-second governed invocation). It covers non-default paths,
changed-input refusal without mutations, interrupted infrastructure and import,
Fleet completion, retained accounting and effect-free offline replay. Fixture
artifact preparation accounts for roughly 214 seconds of the journey.

Evidence: `target/review-validation/canic192-{cli-final,host,clippy,pocketic,docs}.log`
and `target/test-runs/20261002T080829Z-48987.qOe5vd/1.log`. The earlier Host attempt
failed during concurrent, incomplete CANIC-190 edits; the final Host run above
replaces that incomplete-build result. The existing 0.110.50 draft and Fleet guide
include this correction; package versions remain .49. No broad gate, deployment,
version transaction, commit, sibling edit or Cargo cleanup ran.

This closes CANIC-192's implementation and qualification for the corrective
release batch. CANIC-190/R2 and guard-cleanup qualification are recorded above.
A separate session changed ic-memory from 0.15.0 to 0.15.2 while the PocketIC
journey was running; this journey's already-built binaries and artifacts qualify
0.15.0. The dependency update owns its separate final qualification and combined
push-readiness handoff; preserve its edits and evidence. Its 35 memory and 70
receipt native checks already pass in `ic-memory-0152-{native,receipts}.log`.
Do not represent this journey as exact-source validation of that later update.
Toko's reported .49 operation remains untouched; local Canic qualification does
not establish downstream adoption or live acceptance.

## Ordinary test guard follow-up — 2026-10-02

The maintainer's release run reported seven real failures across the workspace
manifest guard, receipt inventory guard and Host library. The runner deliberately
collects the remaining native results before its ordinary-test barrier rejects
PocketIC execution; these failures are not swallowed or expected negative cases.

Corrected the test fixtures and inventories without changing runtime behavior:
role declarations are discovered only under maintained package trees and skip
`.canic` state; the receipt inventory names the memory-reservation regression;
the IcyDB guard verifies the optional dependency, feature and absent default
harness edge; activation handoff fixtures retain reviewed desired input before
sealing their plan hash. Production plan-integrity checks remain intact.

All 15 targeted tests pass (six manifest, two receipt and seven Host tests), as
does all-target/all-feature warning-denied Clippy for the three affected packages.
Evidence: `target/review-validation/ordinary-fallout-{guards,host,clippy}.log`.
The 0.110.50 draft includes these corrections. The corrective batch is ready for
maintainer review and another selected release attempt; no full gate, version
transaction, commit or deployment was run for this follow-up.

## Rust 1.99 qualified — 2026-10-02

The maintainer-requested Rust 1.99 update and full Canic-owned Clippy check are
complete. Repository, CI/security, developer installer and README pins select
1.99.0; the published MSRV remains 1.91.0. Package versions remain 0.110.49 and
the existing 0.110.50 draft includes this work. No commit, version transaction,
publication, deployment, cargo clean or full test suite ran.

Removed redundant `must_use` annotations on the two Host diagnostic iterator
accessors and unnecessary closure borrows in Backup and Host import-journal
lookup. Workspace policy allows the new `assert_is_empty` style lint: existing
emptiness predicates remain valid assertions without typed empty-array casts.

Default build/check/test graphs now exclude optional IcyDB fixture packages.
The cross-crate integration package gates its IcyDB dependency and target behind
`external-composition`; the existing explicit PocketIC selector enables it.
Full Clippy checks all targets/features in other maintained packages and all
default targets in that integration package. A locked dependency-tree check
confirmed IcyDB is absent from the default integration graph. AGENTS explicitly
requires expected upstream/API/type drift to remain a separately reported,
nonblocking local-consumer limitation. This work does not qualify IcyDB or
synchronize its upstream versions.

Qualification passed:

- Full `make clippy` with Rust 1.99 and warnings denied; final passes took
  7.83 seconds and 2.46 seconds with the warmed cache.
- Focused `external_composition_qualification_is_explicit` regression; its new
  assertion was corrected to accept an omitted Cargo `default` feature.
- Embedded allocation peer refresh, including Wasm bytes and structured Rust
  1.99 provenance, followed by the single public managed-component lifecycle
  PocketIC proof: 1 passed in 214.38 seconds, 383-second runner invocation.
- Scope-helper failure-propagation tests, governed runner regressions including
  explicit external selection, ShellCheck, release-integrity contract,
  scoped Rust formatting, document semantics and 0.110.50 draft preflight.
- Follow-up: the maintainer's release gate exposed a missing `workspace-scope.sh`
  copy in the isolated PocketIC worker fixture. Added that dependency, checked
  the other fixture-copy paths, and passed the complete targeted
  `make validation-runner-gate`, including worker interruption/cleanup and native
  ICP selection. ShellCheck and Bash syntax checks also pass. Evidence:
  `target/review-validation/rust199-validation-runner-gate.log`.

Evidence is retained under `target/review-validation/rust199-*`; complete
lifecycle output is in `target/test-runs/20261002T061230Z-63634.D7uBxi/1.log`.
No owned validation process remains running. The corrective batch is ready for
maintainer review and the selected release flow, with the earlier corrective
qualification below and these Rust 1.99 checks. No additional broad gate was
pre-run. Preserve the separated FR1 work described below.

## Corrective release separation — 2026-10-01

The maintainer selected a bounded corrective release instead of waiting for FR1.
The active tree retains CANIC-191, native-credit/controller-order corrections,
ic-memory 0.15.0, current Host hard cuts and the frozen-repair tooling removal.
The root and detailed changelogs now open the `0.110.50` draft; package versions
remain `0.110.49`. No version transaction, Git publication or live reset ran.

Newer FR1 reservations, registered-inventory collection and Core release fences
are set aside in `.canic/local-work/fr1-separated-20261001T200203Z/`.
Its `README.txt` explains restoration. `resume-fr1.patch` contains only the
separated source changes; disposable-copy application with zero fuzz reproduced
all 42 original file hashes. `before/` also preserves all 122 original changed,
new or deleted paths, including documentation. This ignored local bundle is
outside build/test cleanup and is not included in Git publication; retain it
until FR1 is restored or backed up. Earlier committed admission, snapshot-deletion
and physical-observation helpers remain, with no whole-Fleet release command.

The FR1 checkpoints below describe preserved development evidence, not features
included in this corrective release. FR1/CS1 and conditional CANIC-190 work remain
follow-ups and do not block this corrective release.

The separated corrective tree is ready for maintainer review/commit and the
chosen release flow. All 208 selected native tests pass (one existing manual
inspection remains ignored), along with Core/Host/CLI/Testing library/test
all-feature Clippy with warnings denied, scoped formatting, document semantics
and the `0.110.50` release-draft preflight. The public-CLI CANIC-191 PocketIC
journey passes in 509.76 seconds (590-second invocation); roughly four minutes
rebuilt its changed canister artifacts. The refreshed embedded Component Group
peer passes its lifecycle journey in 121.83 seconds (184-second invocation).

The separation check found an embedded peer built from the removed fence code;
its Wasm and structured provenance were refreshed. Two test-only lint fixes in
the preserved FR1 edits were retained in the corrective tree. The restoration
patch was regenerated and again reproduced all original source hashes with zero
fuzz. No unresolved separation failure remains. Existing builds are retained;
only invocation-owned scratch was cleared by the test runner.

Evidence: `target/review-validation/release-split-{native,format,clippy,embedded-refresh,pocketic,lifecycle,docs}.log`.
Complete PocketIC logs:
`target/test-runs/20261001T201943Z-55381.eT7IL5/1.log` and
`target/test-runs/20261001T202814Z-17421.LKP0m7/1.log`.
The earlier embedded verification refusal and first Clippy findings remain in
`release-split-embedded.log` and `release-split-clippy-first.log`. No broad gate,
package version change, staging, commit, push, deployment or sibling edit ran.
Full release validation remains owned by the maintainer-selected release flow;
its not having run during coding is not an implementation blocker.

## Toko feedback and corrective priority — 2026-10-01

CANIC-191 / [RD1](../design/0.110-fleet-runtime-contraction/0.110-design.md#rd1-remove-repair-before-reset-admission--prioritized-2026-10-01)
has a locally qualified admission correction through the existing clean-reinstall owner. Generation and
readiness now select explicit physical inventory without requiring predecessor
completion. Explicit reset qualifies current artifacts and certified physical
custody before archiving opaque predecessor bytes and binding the new selection
under the same Fleet lock. An unfinished Root import alone no longer blocks reset;
old executable plans and application state are not reset authority. Current retries
retain their spent allowances, including unpaid successors of completed phases.
Unresolved Host paid-effect envelopes still refuse with a typed, located error;
this does not require completing an old application or restoring its binary.

Focused checks pass: 21 operation-selection/archive/retirement tests, 28 generation
checks, seven readiness tests, three cancellation tests and 76 Fleet CLI tests.
Host/CLI/Testing library/test all-feature warning-denied Clippy, final fixture lint,
scoped formatting and document semantics pass. The final public CLI PocketIC
journey passes in 249.78 seconds (268-second governed invocation). Its three-source
estate covers interrupted infrastructure, Coordinator activation before Root mirror
activation, mixed cleared/stopped/untouched import sources, malformed predecessor
documents, changed-controller refusal, lost-response reset recovery, bounded debit,
cycle conservation and effect-free offline replay. Exact same-selection retry retains
allowances; a damaged matching current record does not force application repair.

Evidence: `target/review-validation/canic191-{native,generation,readiness-cancellation,cli,clippy,fixture-clippy,pocketic,docs}.log`
and `target/test-runs/20261001T185224Z-34716.H7C65V/1.log`. The corrected fixture uses
the dynamically expanded bootstrap activation phase, not a presumed action in the
later workload review. The stopped disposable run's own scratch was removed; shared
build artifacts and real operation evidence remain intact.

The reported CANIC-191 admission blocker is corrected and qualified in Canic; this
is not live Toko acceptance. The public explicit-reset CLI no longer selects the
source-bound activation preparation/adoption route. Retained in-flight provisioning
reconciliation is not replaced by this slice: genuinely uncertain Host calls and
unreadable paid-effect envelopes still require cycle-safe reconciliation, and the
older preparation machinery must remain until those obligations are covered. Its
remaining contraction is not a requirement to finish an exhausted Root import.
The correction is included in the `0.110.50` draft. Newer incomplete FR1 work
has been preserved outside the corrective source tree as described above.

Toko's current .18 handoff reports its 24-source .48 installation blocked. The
separately reported eight-source recovery is a different operation and does not
establish recovery of Toko. No live reset, sibling mutation, broad validation,
version or Git publication ran. RD1 takes priority over FR1/CS1; their unrelated
remaining work is not a prerequisite for delivering this deployment correction.

CANIC-190's conditional attempt-exhaustion gap is also confirmed in the named
Host survey/submission owners: two attempts can remain spent without a usable
resolution. It is not an observed additional Toko failure. Bounded reviewed
continuation or a concrete cycle-safe reset follows RD1 unless needed by that path;
do not silently refresh counters. Maintained native-credit corrections below
already cover the reported positive-balance refusals locally, not in published .49.
Toko reports its JSON next-action adapter locally implemented with passing offline
contracts; its older audit's missing-adapter finding is superseded. Those downstream
tests were not rerun here; live acceptance remains downstream-owned.

The controller-order false refusals in import admission/destination now compare
sorted temporary copies. Retained Registry bytes, review and authority hashes stay
unchanged; duplicate and changed membership remain invalid. All eleven owning
native tests and Host library/test all-feature Clippy with warnings denied pass
against the concurrently updated `ic-memory` dependency. Scoped formatting, diff
hygiene and document semantics pass. Evidence:
`target/review-validation/toko-controller-order-{native,clippy}.log` and
`toko-feedback-docs.log`. The transient manifest/lock mismatch and missing cached
crate during the concurrent update were resolved by its owner and normal locked
dependency download; no dependency source or lock edits were made for this fix.
AGENTS.md now explicitly rejects historical completion and incidental
representation drift as independent safety requirements. No sibling mutation,
live reset, broad validation, version or Git publication is authorized/performed.
The controller comparison and CANIC-191 admission corrections are qualified; FR1
remains unfinished and is set aside. The correction is included in the `0.110.50`
draft and does not depend on FR1 completion.

## ic-memory 0.15.0 adoption

Canic resolves published ic-memory 0.15.0 and hard-cuts direct runtime growth to
typed failures. Receipt-capacity reservation retains `RuntimeGrowError` in its
ops error and rejects before inserting a receipt. Anonymous allocation metrics
use numeric summaries, preserving gauge names, independent conservation checks,
cache failure timestamps and the 34,848-byte metadata-read bound. Committed ID
and authority helpers replace manual resolution in the IcyDB admission fixture;
early default-runtime access is qualified without choosing bucket configuration.
No compatibility shim, product schema generation or Canic version bump is added.

Thirty-five focused memory tests, seventy receipt tests and Core library/test
all-feature warning-denied Clippy pass. The embedded allocation peer and its
provenance are refreshed, and its exact Component Group lifecycle PocketIC proof
passes in 214.44 seconds, including fixture Wasm acquisition. The governed runner
completes successfully in 372 seconds including native compilation and cleanup.
Evidence:
`target/review-validation/ic-memory-015-{native,receipts,clippy,embedded-refresh,embedded-pocketic,wasm-dependency}.log`.
The selected peer/managed-role Wasm graph contains only ic-memory 0.15.0.

The maintainer clarified that IcyDB is exclusively an optional local test
consumer with an independent upstream dependency schedule. AGENTS.md and CI
governance now prohibit synchronizing its dependencies with Canic, chasing or
waiting for matching releases, or treating upstream skew as a Canic upgrade,
push or publication blocker. IcyDB 0.262.2's composition against the new runtime
is unqualified; no matching-release follow-up is required for this Canic batch.
The earlier alignment-blocker verdict is withdrawn. Canic's ic-memory adoption
is qualified by the focused evidence above and included in the `0.110.50`
corrective draft. FR1 remains a separate unfinished batch. No broad gate, Git
publication, Canic version transaction or deployment ran.

## Hard cut and reinstall policy — 2026-10-01

The maintainer withdrew the CANIC-188 frozen `.48` repair exception and requires
hard cut plus reinstall for every pre-1.0 release transition, including malformed
or incomplete installations. Discard predecessor application/framework state.
Select qualified current-build artifacts, explicit physical inventory and current
controllers for reset authority. Reconcile unfinished paid effects only to
prevent duplicate spending and establish cycle-safe disposition; do not repair
old state, restore an old Root or maintain an old CLI to continue it. Current
same-release interruption recovery, retry, idempotency and conservation remain.

AGENTS.md, active 0.110 design/status, operator guidance and the corrective draft now
carry this decision. Dedicated incident preparation/build/qualification helpers,
frozen patches/fixtures and the Host qualifier example are removed. The
[withdrawn decision and tooling record](../audits/release-lines/supporting/0.110-fleet-runtime-contraction/canic188-issued-import-recovery.md)
and existing incident bundles remain historical evidence only. Their `.48`
terminal-surplus limitation is no longer active work or a readiness condition.
The remaining Host examples compile with all features after removal of the
qualifier; Cargo metadata lists only maintained example targets. Current document
semantics pass without advisories, relative file links resolve in all ten changed
policy/history documents, and scoped diff hygiene passes. Both retained incident
bundle checksum inventories remain unchanged and pass. Evidence:
`target/review-validation/reinstall-only-policy-*`. This policy/tooling cut is
complete and included in the `0.110.50` draft; FR1 remains unfinished. No live reset, deployment, sibling edit,
version or Git publication ran.

## Maintained native-credit corrections

Current import admission, handoff/lost-response recovery, Root callbacks, reset
and net-debit receipts accept native credits while preserving spent debit,
consumed calls and uncertain reserves. Custody, floors and original caps remain
binding. The maintained PocketIC import journey passes real Root/source credits,
settlement and effect-free replay. Native evidence passes 27 Control Plane and
83 Host import tests; affected library/test and qualifier lint passed before the
now-withdrawn frozen repair tooling was retired. Evidence remains in
`target/review-validation/canic188-credit-*`.

The three Host follow-ups also pass: infrastructure bootstrap, unfinished
activation preparation and Fleet-release custody/held-capacity checks admit
increases and bound only net debit. Reviewed source records, debit/funding
ceilings, native floors, custody/content bindings and exact Ledger accounts stay
binding. A new activation preparation records its current balance while retaining
its original source. All thirteen focused native tests pass, including the
bootstrap/activation generator journey and twelve release policy tests. Host
library/test all-feature Clippy with warnings denied, scoped formatting and diff
hygiene pass. Evidence is `target/review-validation/balance-credit-followup-*`.
These current-code fixes remain maintained after withdrawal of the incident
repair and are included in the `0.110.50` corrective draft. FR1 is set aside
and does not block these completed corrections.

## Preserved FR1 development checkpoints (excluded from corrective release)

Core now distinguishes irreversible Fleet-release sealing from resumable snapshot
sealing. The current `v1` fence retains exact operation, review digest and recipient;
exact replay preserves its timestamp, changed bindings refuse, and snapshot
prepare/resume cannot replace or reopen it. The internal synchronous workflow hook
checks role-owned settlement, suspends producers and commits in one message.
It exposes no endpoint and admits no existing command through a release seal.
Actual role paid-obligation checks, bounded handoffs and operator wiring remain
unimplemented; this is not yet proof of whole-Fleet quiescence.

Seventeen focused Core/facade tests pass, including existing snapshot behavior,
typed release refusals, native retained-state reopening, maximum-width record
encoding and canonical Candid equality. Core/facade library/test all-feature
Clippy with warnings denied, scoped formatting and diff hygiene also pass.
Evidence: `target/review-validation/fleet-release-fence-{native,clippy}.log`.
Native store reopening
does not prove IC snapshot restoration. The first compile found an unused future
handoff helper; it was removed rather than suppressing dead-code checks. Root
Unreleased records the potentially breaking current record/status change and
reinstall-only boundary. FR1 remains unfinished and not push-ready; no release
command, broad gate, version or Git action ran.

The maintainer reports `.49` pushed and selected the Root/Coordinator/Wasm Store
FR1 slice next. The first Host library slice now seals an effect-free `v1` review
and validates complete ownership, quiescence/pending-effect evidence, explicit
destructive disposition, snapshot IDs, exact controllers, budgets and native/
reserved-cycle conservation. Known Root/Coordinator Ledger accounts must remain
explicitly recoverable. Independent destination pools require matching network,
operator and subnet, distinct surviving infrastructure and aggregate capacity.
Before-reset checks require every source stopped under operator custody without
changing reviewed code/snapshots; held-capacity checks require empty code and
snapshots with retained balances. Review integrity is checked at both boundaries.

Ten focused Host tests pass (`target/review-validation/fleet-release-admission-native.log`).
Warning-denied Host library/test Clippy, scoped formatting and diff hygiene also
pass (`target/review-validation/fleet-release-admission-clippy.log`). The two
redundant test clones found by the first lint pass are corrected. Existing dependency and test-cleanup
edits from another session were preserved. Shared-target validation owners were
allowed to finish before editing or continuing checks. No deployment, versioning,
Git action, broad gate or Cargo cleanup was performed for this work.

FR1 remains an unfinished batch: no release CLI or whole-Fleet executor is exposed,
no retirement code is removed, and complete role/account observations still need
authenticated collection. Next wire complete inventory/quiescence, qualified
account recovery/settlement and existing-journal handoff/reset execution; then
prove interruptions, fresh bootstrap and replay in PocketIC before contraction.
The root Unreleased notes hold this incomplete batch; no patch number is assigned.
Toko now reports its separate eight-source import recovered, backend Ensure
converged and the original Root restored. Its Root `5lnwm-ziaaa-aaaae-agtqa-cai`
is distinct from the historical CANIC-188 incident; the downstream bundle is not available
here and no independent live verification was performed. The maintainer retained
all report feedback and call-reduction work in the
[0.110 future-work tracker](../design/0.110-fleet-runtime-contraction/status.md#toko-staging-follow-up--accepted-future-work-2026-10-01):
bounded workflow-derived budgets, fewer repeated subnet/status calls with validity
proof, controller-order fixes, observation diagnostics, reproducible incident/raw/
gzip evidence, quiescence and effect-free replay, plus downstream application
acceptance and dependency provenance. RD1 now precedes FR1; no patch or external
effect is authorized by this planning update.

The next execution primitive is now implemented in the existing Host executor:
`DeleteSnapshot` binds exact target, module and snapshot inventory, requires
stopped sole-operator custody, and reconciles only the exact before/after sets.
It carries normal journal identity, debit observation and CLI progress reporting;
it introduces no second journal or whole-Fleet command. The production-adapter
PocketIC case passes unsafe-custody/inventory refusal, exact deletion, a discarded
response with disk intent still at `Intent`, and replay without a second deletion.
The source ID, subnet, code and controllers remain intact, with bounded observed
native debit. Test execution took 3.01 seconds; compilation dominated the 84-second
invocation. The initial fixture omitted the NNS trust anchor and failed before
deletion; that setup is corrected. Evidence:
`target/review-validation/fleet-release-snapshots-pocketic.log`.

Final focused validation also passes fourteen native tests (ten admission,
three snapshot policies and the shared signer-helper regression), Host/CLI
library/test all-feature Clippy with warnings denied, scoped formatting, runner
regressions, shell syntax/lint and diff hygiene. Logs are
`target/review-validation/fleet-release-snapshots-{native,clippy,runner}.log`.
No full gate or release action ran. The first lint pass's duplicate match arm,
redundant test clone and long journey annotation are resolved.

`ops/platform.rs` moved to `ops/platform/mod.rs` to host the focused PocketIC
module using normal Rust directory discovery. Historical audit paths still name
their pinned source revision. The existing authority snapshot fence is not a
drop-in release fence: it blocks normal handoffs and permits resumable retained
work. Explicit release quiescence and reconciliation remain necessary. The whole
FR1 batch is not push-ready; its root Unreleased entry stays open.

Authenticated physical observation after operator handoff is now implemented.
It reuses certified import status reads and shares bounded snapshot decoding with
the deletion adapter. Free preparation binds the signer, network and certified
custody; consuming it performs at most four management reads without automatic
retry. Changed code/version, controllers, subnet, running state or snapshot
inventory refuses the sample. Current balances are returned for conservation
checks. This is neither role quiescence nor account recovery. The following
reservation slice connects its read allowance to the existing operation journal.

The new PocketIC observer case passes actual stopped-canister/snapshot sampling,
wrong signer/network/subnet refusal and changed-custody refusal after preparation
(1.47 seconds). The shared deletion/lost-reply regression also passes (2.97
seconds); recompilation dominated its 169-second invocation. Logs are
`target/review-validation/fleet-release-observation-pocketic.log` and
`fleet-release-observation-snapshot-regression.log`. All seventeen focused native
tests and Host library/test all-feature Clippy with warnings denied pass
(`fleet-release-observation-{native,clippy}.log`), alongside runner regressions,
scoped formatting, shell syntax/lint and diff hygiene. The lint check found a
single-case loop in concurrent import-test edits; it is now the same direct
assertion, with import behavior unchanged by this correction. Only targeted checks ran; shared-target
ownership was checked before each compile. Another session advanced HEAD and
continued import-budget edits; those changes remain preserved.
Use `ICP_ENVIRONMENT=local CARGO_INCREMENTAL=0` with direct Cargo checks to
match local Make runs. Core watches the environment selector, so switching
between unset and explicit `local` also invalidates otherwise reusable artifacts.

The release observer now requires an opaque, non-cloneable reservation borrowing
the ordinary Fleet journal lock. The review freezes each source's per-call quote;
the existing journal retains its review, separate executable-plan identity and
monotonic reserved-call counters. Four calls and their worst-case debit must fit
before persistence can issue the token. Lost results remain charged, uncertain
persistence forces reopening, and ordinary Ensure/import rejects takeover of an
unfinished release. This attaches to an existing matching operation envelope;
operation creation, full quiescence/account collection and execution are still
pending. Current journal/review fields change through a hard cut with no fallback.
Source edits waited for the other session's governed import journey to finish
successfully. Later checks were also serialized with the other validation owners.

Final reservation qualification passes 31 native tests (29 Host, two CLI), the
exact reserved-observation PocketIC case and Host/CLI/Testing library/test
all-feature Clippy with warnings denied. Logs:
`target/review-validation/fleet-release-reservation-{native,pocketic,clippy}.log`.
The PocketIC test takes 1.59 seconds; its isolated Host test binary rebuild dominates
the 92-second invocation. Combining Host and CLI test packages selects a different
Host feature graph; prefer separate native selectors when reusing the isolated
PocketIC binary. The initial test-only redundant clone is corrected. Scoped
formatting and diff hygiene pass; the unrelated incident repair patch retains its
own owner's changes, including blank patch-context whitespace diagnostics.
No broad gate, Git action, release mutation, deployment or Cargo cleanup ran.
FR1 remains unfinished and not push-ready: complete live role inventory/quiescence,
account recovery, operation creation, whole-Fleet execution/reuse proof and
retirement contraction still need implementation and qualification. CS1 follows FR1.

The registered-ownership collector now reuses bounded Registry/pool pagination,
checks the exact reviewed ownership closure, and verifies signer/network plus
certified owner custody around the queries. Every selected child receives a final
custody check; redundant preliminary child reads are avoided. Shared reply decoding
bounds bytes, work, type count and header complexity. Pending registered assets
remain visible, and the returned inventory cannot claim quiescence. Unfinished
creation/import may own IDs outside the registered pool; reconcile those effects
before treating the census as complete destructive authority.

Twenty-three focused native tests pass against the concurrent release-credit
correction. The PocketIC wire fixture passes actual signed queries/certificates,
pending-reset membership, omitted-member/Registry/signer/custody refusal and
malformed replies (1.69 seconds). This qualifies the collector, not runtime role
quiescence or whole-Fleet release. Runner regressions and scoped shell checks also
pass, alongside final Host library/test all-feature Clippy with warnings denied
and scoped formatting/diff checks. Evidence:
`target/review-validation/fleet-release-inventory-*`. Another
session's separate activation assertion correction now passes its thirteen-test
rerun; those edits are preserved. FR1's account recovery, quiescence/handoff,
operation creation, execution/reuse proof and retirement contraction remain.

## Accepted simplification follow-up

The maintainer accepts all seven candidates from the later read-only audit into
the [0.110 CS1 tracker](../design/0.110-fleet-runtime-contraction/status.md#pre-blob-simplification--accepted-2026-10-01).
RD1 now takes priority; CS1 follows FR1 and coordinates with remaining owner
corrections. Shared Fleet transitions, chain-key decoding, nonroot retries,
bootstrap survey loops, role overviews, Backup staging and CI installers must
all complete with focused qualification, propagation and cleanup before final
B5/human 0.110 closeout and 0.111 blob-storage removal/extraction. All are pending;
this planning update implements no source cut and assigns no patch version.

## Separate complexity audit

The maintainer requested repository-wide complexity/obsolete-surface screening,
kept it separate from FR1, then stopped the other implementation and requested
completion. The [published-baseline report](../audits/reports/2026-10/2026-10-01/complexity-hard-cuts-and-feature-gaps.md)
retains the immutable `.49`/`.48` census and first cleanup's 102 focused tests.
The [follow-up report and manifest](../audits/reports/2026-10/2026-10-01/complexity-hard-cuts-and-feature-gaps-2.md)
close all three deferred Host families: inline durable-plan loading/compaction,
omitted empty import credits and omitted bootstrap recovery fields. Executable
plans also require retained reviewed input; their working-input fallback and
its error are removed. Active guides describe explicit current fields. The
generation guard now covers colon wire domains and journal/plan/state families.

All 241 follow-up native tests pass (165 Host, 76 Fleet CLI), plus warning-denied
Host/CLI library/test Clippy, scoped formatting, guard fixtures, audit catalog,
document semantics and diff hygiene. Two existing document-layout advisories
remain for the exact incident-design exception; two native-selected funding
cases remain ignored. Logs and isolated source diff:
`target/review-validation/complexity-finish-*`. Across both slices, 343 native
tests pass and 101 net Rust lines are removed; the follow-up removes 55 production
module lines while adding current authority/shape evidence. One predecessor-only
test struct and three standalone tests were removed by the first slice.

The named cleanup is complete and ready for review. No maintained Canic-owned
generation above `v1` or additional obsolete decoder was found in the repeated
screen and named traces. Exhaustive per-function reachability, semantic test
deduplication and full entropy scoring remain outside the audit's evidence.
The twelve feature/qualification limits remain: fresh backup preflight is
unimplemented, and FR1 is unfinished despite its admission/snapshot primitive.
The stopped task's source, dependency and runner edits are preserved; its later
design-feedback additions were also left intact. No retained paid-operation
files, reservations or frozen CANIC-188 bytes were rewritten. Current hashes
change through a hard cut; no migration or old-format reader is added. Release
scope is now the `0.110.50` corrective draft; newer FR1 work is set aside.
No version, Git, live effect or broad gate ran.

## Prior release qualification

An earlier maintainer release attempt failed only the embedded allocation-peer
lifecycle case: endpoint framework changes had left its checked-in Wasm stale.
The test stage took 3,382 seconds; the internal suite took 2,410 seconds because
independent cases continued after the early fixture failure. Native, documentation,
Host and runtime/blob/payload suites passed. There is no complete release
success receipt for this failed run; retained build caches are reusable, but the
current release owner does not reuse partial test results.

The fixture and structured provenance are now refreshed in unstaged changes.
Complete and PocketIC-only test runs now verify fixture freshness before starting
their suites or server. Narrow lanes retain their selection. The runner regression
passes both stale-fixture refusal and normal suite ordering. The exact previously
failed lifecycle case passes in 55.98 seconds (144-second invocation including
compilation and server exit). The read-only verifier and its warning-denied Clippy
check pass; shell lint, syntax and scoped formatting pass. Evidence:
`target/review-validation/embedded-root-release-*.log`
and `embedded-root-preflight-runner.log`. No full gate, version mutation, commit,
publication or Cargo cleanup ran for this correction. The open `.49` notes include
it. The complete urgent release batch is ready for the maintainer's release retry;
these focused results do not constitute a complete validation receipt.

The reported release Clippy failure and false `[CANIC-TEST:E001] ... FAIL failing`
line were repaired before that release attempt. The reinstall fixture now uses
the equivalent inclusive range. Progress uses libtest-aware stderr reporting:
passing self-tests retain expected failures in capture, while failed native tests
and uncaptured PocketIC runs still expose diagnostics. The two runner self-tests
and two rendering tests pass; the captured run contains no failure event, and an
explicit uncaptured run still emits it. The validation-runner shell proof and
`canic-testing-internal --all-targets --all-features` warning-denied Clippy pass.
Evidence: `target/review-validation/progress-capture-*.log`. This closes those
two reported issues. The later maintainer gate also passed the reinstall journey
described below. No version or Git action was performed by the agent.

The requested generic endpoint framework work for `ic-blob-storage` is complete
in this repository. Public `on_access_denied = "reject"` keeps
plain Candid replies with normal Fleet/custom guards, denial-only metrics and
synchronous handler dispatch. `decode = LIMITS` selects public `ArgumentLimits`
for bytes, decoding, skipping, type count and header complexity. Artifact-owned
`argument_limits = LIMITS` bounds initial lifecycle envelopes before restoration
and participants. Bounded proof predicates reuse decoded arguments. Adoption
examples and semantics are in [endpoint controls](../features/runtime/update-payload-limits.md).

Focused evidence passes 50 macro tests, 28 Core access tests, 2 decoder tests,
13 facade/invariant tests, 7 public API doctests (6 compile-fail), and all 6
`pic_ingress_payload_limits` PocketIC cases. Wasm proofs cover exact reply bytes
and declarations, denial metrics/short-circuiting, malformed and over-budget
query/inter-canister arguments, and init/post-upgrade refusal before participants.
A deliberately trapped participant proves the lifecycle log witness survives
failed installation; reinstall clears prior logs. Only typed install-rate-limit
responses are retried. Affected-package/target warning-denied Clippy and scoped
formatting pass. Evidence: `target/review-validation/endpoint-framework-*.log`.

This framework batch and the existing open `.49` changelog are ready for review;
the later maintainer gate qualified the separate Fleet regression below. The earlier
in-progress macro compile errors are resolved. The sibling repository was read
only; downstream adoption, composed-service/Caffeine qualification and paid
uploads are not qualified or authorized by this work. No version bump, broad
gate, commit, publication, deployment or Cargo cleanup was performed.

An earlier maintainer-selected gate exposed a fixture ordering defect in
`generated_reinstall_recovers_lost_install_and_reaches_working_fleet`: it assumed
a withdrawal must precede every installation, although targets advance in order.
The correction accepts either injected reply-loss order, requires
both applicable interruptions and checks exact withdrawal counts after recovery.
Its focused test compiled, but replacement-artifact compilation stopped on unused
`decode`/`reject_access` fields during concurrent macro/runtime edits, before the
corrected assertions ran. Formatting and the scoped diff check pass. Evidence:
`target/review-validation/generated-reinstall-recovery-order.log`.
The later release run qualified this journey in 5m 04s; see
`target/test-runs/20261001T113447Z-23922.mBMjjH/10.log`. Its earlier qualification
blocker is closed. The subsequent stale embedded-peer failure is repaired above.

The deployment-reliability and completed-Fleet cleanup implementation, prior
direct negative/recovery evidence, propagation and cleanup are complete. Package
versions remain `0.110.48`; both changelog views describe the existing open
`0.110.49` batch. The failed release tested the maintainer-created `c36a0edf8`
checkpoint; the embedded-fixture repair above is additional uncommitted work.
This diagnosis ran only focused checks; no broad validation, version transaction,
Git publication or live deployment was performed by the agent.

[Deployment reliability audit](../audits/reports/2026-09/2026-09-30/deployment-reliability.md)
findings own the current delivery work:

- Findings 1–8 are implemented and qualified: complete test failure feedback,
  native/gated/doctest selection, bootstrap budget admission, held-source funding
  credit, generated lock ownership and exact Cargo artifact capture.
- Findings 11–16 and 18 have implemented release/publication/runner corrections
  and focused shell/package evidence. Native macOS execution remains for CI.
- Finding 17 has native and installed-package evidence for build JSON and Fleet
  automation phases, exact argument-array next actions, approval versus resume,
  and successor review. Downstream adoption in Toko Miner remains separate work
  in its read-only repository.
- Finding 10 is implemented and qualified: structured embedded-Wasm provenance,
  an explicit refresh helper, unchanged-input reuse and changed-input byte
  qualification before the early public lifecycle test. The refreshed artifact
  is byte-identical to the checked-in peer. Two native/inventory tests and the
  exact public Component Group lifecycle pass (`embedded-root-*` logs).
- Finding 9's installed-package journey passes. The extracted and installed CLI
  builds a separate consumer, reuses the exact release, deploys through public
  JSON actions, recovers three injected interruptions, verifies installed
  infrastructure/application hashes and the application endpoint, and replays
  completion with ICP unavailable and no repeated effects. The case took 506.20
  seconds, including a 399.97-second cold build and 32.14-second reuse check;
  offline replay took 1.34 seconds. Evidence:
  `target/review-validation/packaged-consumer-eighth.log` and
  `target/test-runs/20261001T093639Z-27203.vu4IVl/1.log`. This expensive journey is
  explicit opt-in, outside ordinary tests.

Package qualification exposed four product defects that are now corrected:
human progress on JSON stdout; implicit `sccache` retaining deleted scratch;
fresh Root pool queries before Wasm installation; and ordinary terminal replay
repeating live IC observations. Host leaves compiler-wrapper selection to Cargo
and retains unavailable inventory for fresh uninitialized Roots. Ordinary Ensure
and clean reinstall now share durable terminal accounting, retaining the clean
reinstall selection binding. Exact replay returns historical completion evidence;
a new review observes current state. Missing or altered evidence rejects locally.
The old compiler-cache discovery/probe and live-replanning replay paths are removed.

Seven focused compiler-wrapper tests, the positive/negative fresh-Root observation
and retained creation-balance recovery regressions, all 497 native Fleet tests,
and Host/CLI/Testing all-target/all-feature warning-denied lint pass. Final journal
publication interruption and missing/altered receipt evidence are covered. The
small completed-Fleet reset/recovery/offline-replay PocketIC journey also passes
against the shared receipt implementation (`shared-terminal-*` logs). Source
hashes were unchanged throughout the final installed-package qualification.

Existing qualification logs live under `target/review-validation/`: `deployment-*`,
`bootstrap-admission-*`, `import-funding-*`, `generated-lock-*`, `artifact-drift-*`,
`cli-*`, `release-content-native.log` and `release-core-package.log`. These qualify
their recorded source states, not subsequent changes or the complete workspace.
Do not run the entire historical release-flow/remote-state test files: some create
real fixture commits. New release fixtures use fake Git.

## Completed-Fleet cleanup

The accepted cut is implemented: approximately 16,000 Rust lines of superseded
completed-source preparation, receipt/interface reconstruction, seal/publication,
reset paths and their tests are removed. Current clean reinstall is the sole
completed-Fleet reset route. Shared certified-controller observation belongs to
bootstrap/import; unfinished activation and paid-import reconciliation remain.
The retained CANIC-188 incident bundle is untouched.

Focused evidence passes 494 Host Fleet tests, 76 Fleet CLI tests and three exact
PocketIC journeys: completed-Fleet clean reinstall/recovery/replay, unfinished
activation recovery/replay, and running-application import reset/idempotency.
Formatting, document links/semantics, layering, hard-cut and scoped shell checks
pass, including warning-denied Host/CLI/Testing all-target/all-feature lint.
Evidence is retained as `target/review-validation/fleet-cleanup-*`; no broad
suite ran. The cleanup and complete deployment batch are ready for review; the
later shared-receipt evidence above covers the subsequent terminal replay change.

The [follow-up cleanup and usefulness audit](../audits/reports/2026-10/2026-10-01/surface-cleanup-and-subsystem-usefulness.md)
removes 100 net Rust lines and 381 lines from active design/operating docs;
historical plans remain archived. All 79 focused native tests and scoped
warning-denied lint pass. Standalone Root retirement and fixture-data delivery
are larger scope candidates. The [Root retirement follow-up](../audits/working/0.110-surface-contraction/root-retirement-usefulness.md)
informs accepted FR1 below. No Fleet-to-capacity route or feature cut is qualified;
preserve unfinished paid recovery.

## Retained incident and operating constraints

The maintainer prioritised CANIC-188 on October 1. Its existing source correction
passes the exact 24-source PocketIC regression against the completed-Fleet cleanup:
all imports, lost-response recovery, nine Workloads/fifteen Ready canisters,
conservation and effect-free replay. Import used 220 of 784 reviewed calls and
51,091,964,838 of 33,008,458,000,000 allowed Root debit cycles. The case passed in
615.09 seconds (706-second invocation including compilation and fresh artifacts).
Log: `target/review-validation/canic188-post-cleanup-pocketic.log`. All 17 retained
incident-bundle checksum checks pass. No additional runtime correction, live call,
sibling mutation or broad gate was needed or performed for this verification.

The CANIC-188 frozen-state repair was withdrawn on 2026-10-01. Its
[historical decision](../audits/release-lines/supporting/0.110-fleet-runtime-contraction/canic188-issued-import-recovery.md)
and retained `.canic/incident-repairs/canic188/` bundles are evidence only. Preserve
existing journal/status/artifact evidence without maintaining the old executable
owner or its continuation route. Current-build hard cut plus reinstall governs
replacement; observed controlled cycles and unfinished paid effects still need
cycle-safe disposition. Live repair/restoration is no longer planned work.

No live deployment, incident execution, sibling mutation, version transaction,
commit or Git publication is authorized by this cleanup. Toko Miner and other
repositories remain read-only. Keep release build artifacts and retained paid
operation evidence. Agents must never create commits. Check shared `target/`
ownership before each targeted build/test; use focused checks during implementation.

Fresh live backup execution remains unavailable because Coordinator-backed
Component Registry topology preflight is unimplemented. Preserve existing backup
and same-release restore/recovery machinery; see the
[backup availability guide](../features/backup-and-restore/README.md#current-availability).
Blob extraction remains separate accepted future work, not this cleanup's scope.

## Accepted follow-up and history

[FR1 Fleet release to reusable capacity](../design/0.110-fleet-runtime-contraction/0.110-design.md#fr1-fleet-release-to-reusable-capacity--accepted-2026-10-01)
is active at the maintainer's post-push request, before final 0.110 closeout/blob
extraction. It covers retained
Coordinator/Root/Store and child IDs, conservation, one existing Host journal,
controller/reset recovery and retirement contraction. Host admission is qualified
as described above; execution and contraction remain. It authorizes no live or
downstream effects. Toko reports its distinct exhausted import recovered; its
evidence review and preventive follow-ups are tracked above.

The [0.110 tracker](../design/0.110-fleet-runtime-contraction/status.md#accepted-code-review-corrections--2026-09-30)
owns remaining R2–R8 work. Open outcomes include exhausted/older-unknown imports,
remaining funding accounting, allocation-scoped caller/issuer/funding authority,
backup upload/capture/restore authority, background-driver trap recovery and
operation-specific convergence. Its conservative count is 27 of 401 original
findings; do not treat partial corrections as closed or the full queue as a gate
for every bounded corrective release.

The accepted deployment/cleanup outcomes and their direct evidence are complete;
the open changelog covers the whole batch. Broad validation, versioning and
publication retain the maintainer-selected release boundary.
Final qualification includes FR1. The 0.110 closeout audit must be explicitly
requested and accepted before 0.111 implementation; continuation does not cross
that boundary.

Earlier checkpoints, superseded next steps and detailed timings are retained in
[historical handoffs](archive/2026-10-01-prior-fleet-handoffs.md). They are evidence,
not current instructions. The
[0.110 design](../design/0.110-fleet-runtime-contraction/0.110-design.md), tracker,
audits and governance own their respective contracts; status grants no release authority.
