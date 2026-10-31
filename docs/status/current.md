# Current handoff — 2026-10-31

## Current work and authority

Canic 0.113.0 remains the published local HEAD at
247ba9d3599658c478373cb0dfc74439a859dd8a; all nine family crates are published.
Release attempt 4 completed. Its former target/ evidence is absent from the
main checkout at this task's start; current adoption evidence is retained outside
target/ as described below.

The maintainer explicitly selected pushing 0.114 and correcting in-scope failures
until its governed source release succeeds. This names the exact next minor and
authorizes its source commits, complete release validation, version transaction,
tag and branch push. It does not record acceptance of the separate 0.113 human
closeout audit or select package publication or application deployment.

The complete published sibling dependency group is aligned and qualified locally,
including Timers 0.35.0. The previous Platform/Wire/Metrics blocker is resolved.
Own package versions remain 0.113.0 before release execution. The pending 0.114
notes describe the complete implemented batch. The reviewed source roster,
pre-release bytes and command evidence are retained outside target/ under
.tmp/release-0114-20261031/. Release progress and exact retry state belong to
the governed Git release journal, not this descriptive handoff.

The subsequent request authorized scoped repairs, commits and normal branch
pushes for PRs #717, #716 and #609, plus a local Platform update attempt. Those
PR changes use isolated checkouts; they are not integrated into this dirty main
checkout. No PR merge, main push, own version bump or release was performed.

## Current upstream adoption

Timers 0.35.0 is published and the previously held dependency group is adopted
in the isolated source at .tmp/runtime-adoption-20261031/consumer/. Retained
commands, observations and checks live in its sibling evidence/ directory.
The prior qualified source/evidence directories remain unchanged. The official
index capture is evidence/registry-latest.json; all direct sibling selections
match its latest non-yanked stable versions.

| Family | Selected |
| --- | --- |
| Auth / protocol | 0.28.0 |
| Backup / Agent | 0.53.1 |
| Blob / contracts | 0.61.0 |
| Build / Host Artifacts / Fs / Process / Text / metadata editor | 0.73.2 |
| Host Tools / Testkit | 0.75.1 |
| Logging | 0.23.3 |
| Memory / Storage | 0.60.3 |
| Metrics Stable | 0.61.1 |
| Metrics | 0.24.1 |
| Platform / macros / runtime guard | 0.15.3 |
| Platform types | 0.15.1 |
| Query | 0.79.0 |
| Timers | 0.35.0 |
| Wire / derive | 0.13.2 |

The root and both independent Blob consumer catalogs/locks move together.
Core, Blob, dedicated consumer and embedded consumer selected Wasm graphs each
contain exactly one Memory, Storage, Metrics, Wire, Platform, runtime guard and
Timers identity. There are no dependency overrides, compatibility aliases,
identity conversions or weakened runtime guards. The former Blob/Timers
publication blockers are resolved.

Query 0.79 replaces the removed NNS-only source request with its upstream
report::CollectionSourceRequest. The Host acquisition adapter and three fixture
modules use that contract directly, preserving their fields, deadlines,
agreement and cache semantics. No Canic-owned request shim is introduced.

Shared Tooling remains the already adopted committed revision
619d194875369e0f72b45477b66adb430acbf6b7, VERSION 0.27.23, with 148 payloads.
Its pending VERSION automation does not replace Canic's Cargo version owner.
Own package versions remain 0.113.0 and the existing open 0.114 notes cover the
complete batch's earlier breaking contracts and new dependency identities.

Successful focused checks cover 406 Core cases for auth, RPC, stable state,
logging, metrics and memory; 80 Host/catalog/Backup cases; and 28 control-plane
cases. All 514 pass. Strict Core/Host library/test Clippy and scoped Host/internal
fixture formatting pass. The initial Query import failure was corrected through
the new upstream contract; failed/interrupted setup attempts remain separate
from successful qualification.

The canonical Testkit installer admits the locked 0.75.1 CLI and PocketIC 16.1.0.
The embedded allocation peer and structured provenance are regenerated and
independently verified for the new dependency graph. Both selected managed Blob
composition journeys pass with complete locked App builds, generated boundary
contracts, installation, metrics, permissions and same-release recovery. The
embedded journey also checks retained application state and its five funding
recovery outcomes. No broad workspace or PocketIC suite ran.

The requested dependency adoption is complete, and the maintainer has selected
the 0.114 source release above. The open changelog describes the selected current
contracts; the governed release command owns the complete validation and exact
version transaction. Prior PR branch work remains outside this checkout.
Old target/ evidence
referenced by historical handoffs was absent at the earlier task's start;
current evidence is retained outside target/.

## Preceding control-plane issue repairs

The maintainer requested further independent repairs. The selected scope is the
behavior-preserving receipt/member validation cleanup in #690 and #691; the
broader single-Clap-tree work in #300 remains a separate outcome. Evidence is
retained in target/control-plane-issue-repairs-20261031/; qualified source is
.tmp/control-plane-issue-repairs-20261031/. Scoped files are integrated into this
working tree, with replaced bytes retained in pre-integration/.

- #690: one receipt helper owns authority validation, in-flight omission and
  verified Store adoption projection. Operation-scoped reads still return no
  receipt before authority validation for another operation; strict reads still
  report the typed conflict. Missing state, exact fields and read-only replay
  remain covered through the actual ops/storage boundary.
- #691: one validator owns member authority and expected Reserved/Verified
  progress. The two cursor advances select their exact required state; committed
  members retain separate Registry partition validation. Refusal leaves cursor
  and retained records unchanged; existing restart and terminal replay cases pass.

All 28 selected control-plane library cases pass, including four new focused
receipt/member refusal cases. Strict control-plane library/test Clippy and
scoped formatting pass. The embedded allocation peer and provenance are refreshed
for these sources and independently verified. No broad suite, PocketIC/installed
journey, Git publication, own version change or release ran.

#690, #691 and #551 have updated local evidence and remain open for delivery and
applicable required CI. Main is still 247ba9d3599658c478373cb0dfc74439a859dd8a;
its latest required CI run 41063887855 is failed. No new closure is justified.

All catalogs/locks retain their frozen bytes. input-graphs.json records the same
root lock 8a02e082bb62e6aae393302ea9caf43bcf8195ca592336a57197ce24c55f4ee0
and two Platform identities. Fixture verification does not qualify that dependency
graph. The complete open batch still needs dependency reconciliation before push
readiness; its changelog includes these cleanups, own versions remain 0.113.0 and
publication retains its separate gates.

## Preceding CLI issue repairs

The follow-up request selected two further bounded CLI repairs, independent of
the blocked upstream Platform cohort. Evidence is retained in
target/cli-issue-repairs-20261031/; qualified source is
.tmp/cli-issue-repairs-20261031/. The scoped changes are integrated into this
working tree with original replaced bytes retained in pre-integration/.

- #277: use fallible scoped thread startup and typed startup/panic failures in
  the existing ordered collector. Cycles, Metrics and live list project failures
  onto the selected target while retaining successful rows and the eight-worker
  ceiling. Deterministic refusal coverage uses an injected std::io::Error result;
  it does not claim actual host exhaustion or a new Root relay measurement.
- #367: cycle/token transfer help explicitly describes ledger credits, with
  selected-identity transfer examples. Native Coordinator/Root funding stays on
  cycles topup. Existing arbitrary-recipient parsing and dispatch remain covered.

All 31 selected CLI library regressions and both recursive help cases pass;
strict CLI library/binary/test Clippy, scoped formatting and review of all five
affected help pages pass. The initial binary-only test selection ran zero cases;
cli-regressions.log records the corrected library selection and actual 31 passes.
No broad suite, PocketIC, Git publication, own version change or release ran.

Both issues remain open for delivery and applicable required CI. Delivered main
is still 247ba9d3599658c478373cb0dfc74439a859dd8a, whose latest required CI
run 41063887855 is failed. Native macOS/installed Root relay qualification is not
claimed by these Linux host tests. No additional issue closure is justified.

All manifests and locks retain their frozen bytes. input-graphs.json records
the root lock 8a02e082bb62e6aae393302ea9caf43bcf8195ca592336a57197ce24c55f4ee0
and its two Platform identities. The entire open batch still needs dependency
reconciliation before push readiness. Changelog notes include these repairs;
own versions remain 0.113.0 and publication retains its separate gates.

## Preceding issue-repair pass

The maintainer requested issue review, bounded repairs and eligible closures.
The inventory contains 326 open issues and 25 open PRs. Default main remains
0.113.0 at 247ba9d3599658c478373cb0dfc74439a859dd8a; its required CI run
41063887855 is failed. Routine qualification issues remain open for delivery
and applicable required checks; a local pass does not close those gates.

Evidence: target/issue-repairs-20261031/. Source: .tmp/issue-repairs-20261031/.
The scoped repair changes are copied into the current working tree after frozen
source/index checks, with original replaced bytes retained in pre-integration/.

- #450: the changelog guard accepts the latest pending or dated release while
  retaining detailed-note pairing and unambiguous draft selection. Tool fixtures
  compare canonical executable identities, with an aliased-parent regression.
- #712: remove the duplicate RuntimeLog/limits wrapper and use the upstream Log
  directly. Count/message limit changes still reset the diagnostic window;
  zero-capacity disabling, filtering, paging and typed refusals remain. #715 owns
  the complete Logging adoption and its delivery/qualification acceptance;
  #100's read-work budgets and #101's timer work remain separate.
- #714: use ic_wire::ByteBuf at both Host import sites, retaining identical types
  and external CBOR/Candid behavior. Delivery acceptance remains open.

#712 is closed as superseded into #715, not as delivered qualification.
#450, #551, #714 and #715 have updated evidence and remain open for their
delivery/qualification obligations.

All 26 focused package executions pass: seven logger, 16 Host tool/handoff/archive
and three changelog cases. All Host cases run under an aliased TMPDIR. The exact
published 0.113.0 changelog and detailed notes also pass the corrected guard's
three std-only fixture executions. Strict Core/Host library/test and Canic
changelog-target Clippy, scoped formatting, peer refresh and verification pass.
Native macOS execution and new hosted qualification remain outstanding. No broad
suite, PocketIC case, Git publication, version change or release ran in this pass.

The root lock had drifted before this pass: its frozen SHA-256 is
8a02e082bb62e6aae393302ea9caf43bcf8195ca592336a57197ce24c55f4ee0,
selecting Backup/Agent 0.52.2 and Persistence 0.58.1 and introducing Platform 0.12
alongside 0.11.1. The preceding qualified root lock was
0ca2b06fc215737b218554cbce293bac8541e9f0f6c142a1b5f667ade06b5184.
These repairs preserve all manifests and locks exactly; input-graphs.json records
their actual identities. The prior single-Platform adoption proof below applies
to its archived selection, not this drifted root graph. The entire working batch
therefore still needs dependency reconciliation before push readiness; notes are
current, own versions remain 0.113.0 and publication retains its separate gates.

## PR repairs and remaining blockers

Evidence: target/pr-platform-work-20261031/. The PR source cohort remains the
published main cohort, separate from the newer qualified local adoption below.

| PR | Pushed head | Result |
| --- | --- | --- |
| #717 Candid declaration printer | c022353c48d6d4b7977698c9b16bcb054e110165 | Classify the exporter as non-role tooling; select it in internal and audit declaration builders. 178 native executions, strict scoped native/Wasm Clippy, refreshed peer verification, installed dedicated Blob composition and all 11 installed ingress/frame/quota/reply-rollback cases pass. Hosted CI remains asynchronous. |
| #716 deferred-work admission | 3efc42221f59713d19c5ae4485b332cf9cf37d97 | Merge current main into the PR with history retained, reconcile pending 0.113 notes, share startup decisions and refresh peer/provenance. 42 readiness/activation/startup/timer cases, changelog governance, strict scoped native/Wasm Clippy and peer verification pass. Hosted CI remains asynchronous. |
| #609 runtime SHA-256 0.11.1 | 08f8e65e2d8f2ea981edc0107b1c6d3f91758742 | Refresh both Blob consumer locks and peer/provenance. 237 native cases, strict affected-owner/consumer Clippy and Contracts/Wasm on Rust 1.91 pass. Fresh CI and the exact local crypto-closure gate reject duplicate crypto identities in all 12 canonical Wasm profiles. Converted to draft; not ready to merge. |

#609 needs a coordinated upstream crypto cohort: maintained protocol/signature
dependencies still select SHA-256/digest 0.10. Platform 0.13 also retains its
SHA-256 0.10 declaration, so that update alone cannot qualify this runtime bump.
The guard is unchanged. The installed #717 run exposed and then qualified the
internal-builder exporter fix; its initial source failure, strict-lint correction
and sandbox loopback refusal remain recorded alongside the successful retries.
PR bodies contain the final scope and evidence. Isolated sources are
.tmp/pr-work-20261031/, .tmp/pr716-work-20261031/ and
.tmp/pr609-work-20261031/; all three worktrees are clean after their pushes.

## Preceding qualified upstream adoption

Evidence: target/observability-platform-adoption-20261031/. Qualified source:
.tmp/observability-platform-adoption-20261031/. The final registry observation
at 09:59 UTC on 2026-10-31 found no matching supplier releases beyond this selection.

| Family | Qualified selection | Latest requested line |
| --- | --- | --- |
| Logging | 0.22.0 | Adopted |
| Metrics | 0.21.1 | 0.22.0 blocked by integration types and native Wire metadata |
| Platform | 0.11.1 | 0.13.0 blocked by the runtime guard and required supplier identities |
| Wire/derive | 0.10.0 | 0.11.0 targets Platform 0.12; the requested Platform is 0.13 |
| Shared Tooling | 0.27.12 at 69a8b92a1a892b17d355ec853ca63faa9599a928 | Adopted through the canonical exporter |

The combined candidate fails Cargo resolution: canic-tests selects Testkit
0.72.1 and its Platform 0.11.1 runtime guard, conflicting with the requested
Platform 0.13.0 guard. Required Auth, Blob, Query and Timers edges also select
Platform 0.11; Persistence 0.58.1 and Wire 0.11 target Platform 0.12. Metrics
Stable, Blob contracts, Query and Timers still require Metrics 0.21 types.
Metrics 0.22's explicit native-wire feature targets Wire 0.11; Canic's maintained
native contracts use Wire 0.10. These include actual runtime and metadata type
boundaries, so isolating Testkit alone cannot qualify the requested combination.
No aliases, overrides, compatibility bridge or weakened guards are introduced.

The frozen root lock already contained unqualified Backup/Agent 0.52.2 and
Persistence 0.58.1 selections. Existing admission rejected their additional
Platform 0.12 identity. Exact Cargo lock selection restores the qualified
Backup/Agent 0.52.1 and Persistence 0.58.0 cohort. Logging 0.22 and Metrics 0.21.1
are selected consistently in the root and both independent Blob consumer locks.
Original lock bytes, rejected graphs and the precise introducing paths remain
in the evidence directory.

All 73 selected native executions pass: bounded log recording/query/refusals,
metric providers/storage, facade projections and changelog governance. Strict
Core/facade and both Blob consumer Clippy pass. The embedded allocation peer is
refreshed and verified. All three locked graphs pass dependency admission, pin
and boundary checks; official Logging/Metrics checksums and single identities
are recorded in qualified-graphs.json. The installed dedicated Blob composition
passes its four-artifact build, controller log/metric queries and typed refusals
for other callers. Shared integrity verifies 146 canonical files and 330 local
references across 51 governance documents. Canic retains its publication owner;
new shared publication options remain opt-in.

Those qualified scoped files were copied under the main build locks after
verifying the frozen source roster. Original replaced bytes are retained in
pre-integration/; the real index and unrelated dirty work were preserved. That
compatible selection was ready for maintainer-directed review/commit/push before
the later root-lock drift recorded above. The full requested newest cohort
remains blocked, and publication still awaits the accepted 0.113 closeout and
governed version bump. No full workspace gate is claimed.

## Adopted upstream dependencies

| Family | Selected version |
| --- | --- |
| Auth/protocol | 0.26.0 |
| Backup/Agent | 0.52.1 |
| Blob/contracts | 0.60.0 |
| Build/Host/Testkit and exact metadata editor | 0.72.1 |
| Logging | 0.22.0 |
| Metrics | 0.21.1 |
| Memory/Storage/Metrics Stable | 0.58.0 |
| Platform | 0.11.1 |
| Query | 0.78.0 |
| Timers | 0.33.1 |
| Wire/derive | 0.10.0 |

In the archived qualified selection, root and both independent Blob consumer
catalogs and locks are aligned. Those three graphs contain one Platform, runtime
guard, Wire, Metrics, Memory, Storage and Timers identity. The later root-lock
drift and newest requested cohort remain unqualified as described above.

Prepared calls now use ic_platform::call. Remove the standalone ic-call catalog
and member dependencies; ownership, DTO and policy guards follow the new path.
Wire's platform-types feature replaces Platform's removed wire-schema feature
for native Principal/Account schemas. Required external IC protocols retain
their upstream Candid boundary. Current Blob assessment identities and Auth's
typed scope/error contracts remain adopted; callers regenerate from current
schemas. There are no dependency overrides, compatibility wrappers or extra
runtime owners.

The Rust floor remains 1.91; qualification uses Rust 1.99.1. The reviewed Shared
Tooling snapshot is 0.27.12 at
69a8b92a1a892b17d355ec853ca63faa9599a928. Earlier baseline adoption and
adapter/order evidence remain under .tmp/runtime-adoption-20261030/.

## Qualification and readiness

Evidence: target/coordinated-upstream-adoption-20261030/. Isolated source and
independent targets: .tmp/coordinated-upstream-adoption-20261030/. Attempt 19
retains the primary registry observations, verified source checksums and all
three resolved graphs. Earlier attempts retain their actual failed selections.

The following retained evidence qualifies that preceding dependency selection;
the latest Logging/Metrics/Shared Tooling follow-up is qualified separately above.
For that exact preceding cohort, strict all-target/all-feature Clippy passes for
the affected owner packages and
both Blob consumers. All 825 selected native executions pass, covering auth,
storage, calls, metrics/logging, contracts, checkpoint custody/recovery, artifact
builds, endpoint macros, external ICRC contracts and schema declarations.
The initial native invocation hit sandbox DNS in one generated Cargo fixture;
its exact Host retry passed. The facade runner's missing wire-schema feature
was corrected before its successful execution. native-summary-19.json records
the combined results; failed invocation logs are retained with their scope.

All 30 selected framework PocketIC cases pass: memory restoration through two
identical-Wasm upgrades; dedicated and embedded Blob composition, including the
five embedded funding variants; 11 timer/logging/metrics cases; 11 ingress/frame,
quota and reply-rollback cases; original Host checkpoint recovery; and four
funding-recovery cases. Four additional installed ICRC profiles pass: default,
feature-enabled but unmounted, discovery-only and consent. Exact tools, final
selected Clippy and formatting checks pass. The embedded allocation peer and
its source/tool/lock evidence are refreshed and verified.

Scoped files are integrated under the main build locks after checking the frozen
source roster. Original replaced bytes are retained under pre-integration-19/;
the Git index and unrelated dirty work are preserved. That compatible dependency
selection was qualified with a matching changelog draft; later root-lock drift
must be reconciled before the current complete batch is ready for commit/push.
Publication surfaces await the accepted 0.113 closeout
and governed version bump; own versions remain unchanged. No full workspace gate
is claimed.
Older evidence under target/upstream-platform-refresh-20261030/ and
target/upstream-refresh-0113-20261030/ retains its original dependency selection.

## Maintained boundaries and later work

Pre-1.0 release transitions remain clean reinstall. Same-release interruption
recovery, cycle conservation, uncertain-effect custody and effect-free replay
remain required. Wire owns owned RPC/storage codecs; dedicated Fleet activation
and release-build CBOR commitments retain their exact encodings.

The optional Wire endpoint adoption draft remains separate under
.tmp/wire-endpoint-adoption-20261030/. Preserve startup/access checks, decoded
proof guards, dispatch metrics, native schema registration, exact method IDs
and budgets, and compiled declarations when resuming that work.

Blob adapters mount explicitly with one lifecycle and Memory owner. ICRC mounts
are explicitly selected by consuming source. Auth's unpublished mounting
scaffold and production Backup transport remain unfinished. Backup's CLI stays
retired; Canic retains thin custody-bound Host integration over the upstream
engine. Live backup/restore remain unavailable until production qualification.
Logs use upstream bounded heap recording and reset on upgrade; stable metric
overrides use Metrics Stable. Controller guards and explicit public metric
selection remain. Jobs/Relay extraction remains unimplemented.
