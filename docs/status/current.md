# Current handoff — 2026-10-14

**Canic 0.111.0 is live; the integration batch remains uncommitted.** The maintainer
explicitly waived the 0.111 closeout audit for completing Auth, Backup and Blob
integration, then a cleanup slice to reduce Canic's surface. That decision is
recorded on [#517](https://github.com/dragginzgame/canic/issues/517#issuecomment-6119790876).
It authorizes this in-repository batch, not release/version/Git effects or a
blanket exception for future minors. The pending incompatible public family is
classified under the existing **0.112.0** changelog entry. Own package versions
remain **0.111.0**. The objective is to delete duplicate mechanisms in favor of
upstream owners; moving declarations alone does not demonstrate faster builds.


## IcyDB raw entrypoint prerequisite — 2026-10-17

The dedicated contribution for [#590](https://github.com/dragginzgame/canic/issues/590)
adds synchronous bounded raw query/update macros to the existing access/dispatch
owner. Host export validation combines raw declarations and Candid methods and
rejects overlap or duplicate names. The public checker is now
`validate_wasm_endpoints`; callers must use that name.

Focused Linux checks pass: 44 endpoint macro cases, bounded declaration parsing,
strict changed-owner and selected installed-test lint, and the installed raw
probe's admission/denial, byte, application failure and trap rollback controls.
Testkit 0.49's prepared launcher owned the temporary server lifecycle. The first
installed attempt stopped before calls because no governed URL was supplied;
the launcher run passes. Evidence: `/tmp/canic-raw-*.log`.

The actual `canic-icydb` integration and database endpoint cut remain outstanding.
This contribution's committed graph uses Memory 0.44; IcyDB's pending native API
uses Memory 0.46. Use one compatible runtime and a committed native binding; do
not consume another checkout's dirty adoption. Installed startup refusal and
codec-specific checks still belong to complete adapter acceptance. Broad gates,
macOS and raw Wasm/cycle/instruction deltas were not selected or measured. Package
versions remain unchanged; this is a draft contribution, not a release.

## Auth status capability emission — 2026-10-16

Managed Auth status types and their endpoint now require delegation-proof issuer
capability. Local authorization alone previously emitted two empty Candid enums,
whose derives failed strict unreachable-code lint. The redundant local-auth macro
attribute parameter is removed; no placeholder variant or lint suppression remains.
Independent locked/offline strict library lint passes for `local_authorization_probe`
and `delegation_root_stub`, avoiding cross-fixture feature unification. Evidence:
`/tmp/canic-auth-empty-local-clippy.log` and `/tmp/canic-auth-empty-root-clippy.log`.
Incoming source: `/tmp/canic-auth-empty-before/`. The 0.112 changelogs are updated;
portable Auth admission and installed qualification still block the complete batch.


## Auth storage visibility lint — 2026-10-16

The two conditional Auth record reexports now use `pub` inside the existing
crate-private storage module. The layering guard explicitly requires that storage
boundary and exempts only its internal reexports; `ops/storage` remains checked.
Core library/test strict lint with `auth-local-application-authorization`, the
layering guard and repository-configured shell lint pass. Evidence:
`/tmp/canic-auth-visibility-clippy.log`; incoming content:
`/tmp/canic-auth-visibility-before/`. This clears the reported visibility failure;
the complete 0.112 Auth/installed-qualification blockers below remain.


## Host 0.23 caller alignment — 2026-10-16

Host/Backup/CLI compile with Host 0.23.1. Missing-path resolution explicitly
preserves its existing depth policy; regular-file opening and exclusive acquisition
are separate, with locks acquired before protected work. Candid normalization now
uses a direct Host Text dependency. The only task-owned lockfile change adds that
Host dependency edge; selected package versions were retained.

All 36 focused native regressions pass across JSON persistence, Candid extraction,
output roots, build/CLI locks, component operations, bootstrap inspection and
reinstall handoff. Strict all-target lint for Host, Backup and CLI passes. Two
existing overlong functions were split along inspection spending and interrupted
handoff boundaries. Evidence: `/tmp/canic-host023-*.log`; incoming content:
`/tmp/canic-host023-before/`. A concurrent Cargo update interrupted the check sequence;
checks resumed after it ended with the selected dependency graph unchanged.

The open 0.112 changelogs include this adoption. The complete batch is not ready
to push: portable Root-managed Auth admission and installed recovery/fixture
qualification remain unfinished. Own versions remain 0.111.0. No broad validation,
Git publication or release ran.


## Invariant gate repairs — 2026-10-16

The layering guard now passes: checkpoint DTO round-trip qualification lives in
ops tests, and the two Auth record reexports are crate-visible. The release
validation-lane fixture now copies the shared version-input validator required by
`next-release-version.sh`; its own rejection/reuse/failure cases pass. The relocated
checkpoint round-trip regression passes. Evidence: `/tmp/canic-release-lane-fix.log`
and `/tmp/canic-checkpoint-wire-fix.log`.

Auth 0.7.5 now clears the crypto closure check: all 12 canonical roles pass,
with no duplicate crypto package versions and no signature stack in the nine
zero-auth roles. This resolves the local dependency evidence for
[IC Auth #65](https://github.com/dragginzgame/ic-auth/issues/65).
Evidence: `/tmp/canic-auth075-crypto.log`. The gate was not weakened.
Complete 0.112 readiness still requires portable Auth admission and installed
qualification. No broad gate, commit, version change or release ran.

## Memory 0.44 integration — 2026-10-16

Canic now owns one explicit thread-local Memory runtime. Lifecycle bootstrap
constructs it; framework/application stores and the Blob adapter receive handles
from that owner. Readiness and diagnostics do not initialize memory. Construction,
bootstrap and reentrant access retain typed errors. Allocation projections derive
bytes from upstream page/bucket measurements and constants, while checking primary
conservation inputs before subtraction. Current Canic diagnostic DTO values remain
unchanged. Removed upstream singleton and fixture transition APIs have no callers.

Focused qualification passes: 33 Core Memory tests, two public composition tests,
strict Core library/test lint, Host/CLI/Backup all-target compilation, Core Wasm
compilation and the managed lifecycle probe's native compilation. The Wasm probe
check exposed and corrected two stale Auth feature gates, then reached the required
`canic build` artifact guard; no governed installed artifact or PocketIC qualification
is claimed. Evidence: `/tmp/canic-memory44-*.log`; incoming source:
`/tmp/canic-memory44-before/`. Tests began with Timers 0.25.1; subsequent lint and
compilation used the maintainer-selected 0.25.2. No dependency files were edited.

Build 0.10.0 aligns with Host 0.22.1. Blob/contracts 0.30.0 now select the
same Memory 0.44.0 as Canic, clearing the known dependency mismatches. The Blob
adapter's locked/offline all-target compile and strict library lint pass; evidence:
`/tmp/canic-blob030-check.log` and `/tmp/canic-blob030-clippy.log`. No source or
dependency changes were needed for this recheck. The complete 0.112 batch remains
not push-ready: portable Root-managed Auth admission and installed recovery/fixture
qualification are unfinished. The open changelog documents the implemented slices;
no commit, version change, release or broad gate ran.

## Shared Tooling and Host adoption — 2026-10-16

Shared Tooling 0.15.7 is adopted at `05548771d95c4f27f23f29e28a6db8f4c0cac44d`;
the 119-file snapshot integrity check passes. The maintainer-selected Host 0.22.1
callers now specify durable publication mode and permissions explicitly, preserving
replace/create-only behavior and owner-only environment-key creation. No compatibility
wrappers or package-version changes were added.

The focused Host/CLI/Backup all-target compile exposes five Host errors: published
Build 0.9.0 takes Host Process 0.21 `OutputLimits`, while Canic selects 0.22.1.
Build's local checkout already selects Host 0.22; a published aligned Build is
required before the consumer compile and focused publication tests can complete.
Evidence: `/tmp/canic-host022-check.log`; incoming source is retained under
`/tmp/canic-host022-before/`. No broad gate or Git publication ran.

Auth 0.7.2 includes the required session selectors; Blob 0.29.0 aligns with Memory
0.43. Their prior focused compile passed, as did eight portable session-storage
tests. Those upstream blockers are cleared. The complete 0.112 batch remains
unready: portable Root-managed enrollment/admission and installed recovery/fixture
qualification are unfinished. Application admission still refuses. Live Backup
remains unavailable under the accepted release scope.

## Application Auth hard cut — 2026-10-16

The maintainer explicitly selected removal of both delegated-token issuance/
per-call guards and delegated-token session establishment. The DTOs, command and
status variants, access guard, issuer retention and replay helpers, session
workflow and obsolete installed journeys are removed, including sample-app
endpoints. Root proof provisioning/renewal, role attestations and portable
Root enrollment/storage infrastructure remain. Public application admission
refuses while the portable workflow is unfinished; this is not a working login
replacement.

Focused evidence under `/tmp/canic-auth-hardcut/`: 321 Auth-enabled Core tests,
32 replay-policy tests, 39 runtime metric tests, 42 endpoint macro tests and 38
public endpoint/protocol tests and 283 current DTO tests pass. The instruction-audit native tests pass
38 cases with report generation ignored. Default Core/Contracts and Auth-enabled
Core strict lint pass; final Core lint includes the serialization benchmark.
The affected native-agent/instruction targets and four changed canister packages
pass strict lint. No installed PocketIC proof or broad gate ran for this cut.

During qualification the maintainer selected Build 0.9. Its consumers now derive
output directories from existing Cargo profile arguments. Testkit 0.48 baseline
callers return the successful restore receipt directly. The agent did not change
manifest or lock selections. Both changes remain within the existing integration
batch; no publication, staging, commit or own-version change ran.

Active guides and 0.112 notes describe the removal. The reduced instruction audit
uses method v4, with a separate comparison identity. Four changed audit method
fingerprints match; the catalog-wide check still fails on the unchanged incoming
`capability-surface.md` fingerprint mismatch. Historical reports retain their
original scope. The selected Blob 0.28/Memory 0.43 alignment and portable Auth
workflow/enrollment/clock-floor/installed qualification remain unfinished. The
complete 0.112 batch and changelog are not ready to publish; this cut is local
source preparation. Retained fixture artifacts still need final release-graph
qualification. Live Backup remains unavailable under the accepted release scope.

## Auth 0.7 package recheck — 2026-10-16

The maintainer selected published Auth/protocol-types 0.7.0 and Metrics 0.11.4.
The session-storage fixture now consumes Auth's decoded claims in the two-argument
verifier callback, avoiding a second decode. All 18 owning storage tests and strict
Core library/test lint pass. Auth-enabled compilation still fails at the two
unfinished establish/authorize workflow calls. Auth PR #64 remains open and the
published Session still lacks mechanism/key-binding selectors.

Registry inspection confirms Blob/contracts remain 0.28.0 with Memory 0.41;
Canic still selects Memory 0.43. Build 0.8.0, Host Process 0.21.0, Testkit 0.48.0
and Backup 0.25.1 remain current. Blob publication/alignment and the unfinished
Root-managed Auth integration still block the complete 0.112 batch and publication
readiness. No dependency selection, own version, Git index or release changed.
Evidence and incoming copies: `/tmp/canic-auth07-check/`. No broad gate ran.

## Build 0.8 and Memory 0.43 consumer qualification — 2026-10-16

Published Build 0.8 clears the Host Process type mismatch. Memory 0.43 adoption
now uses the consolidated RuntimeBootstrapPolicy, typed resolution errors and
canonical ledger identity constants. Diagnostic fixture constructors use Memory's
test-support feature through a development dependency; production admission and
report fields retain their existing behavior.

All 33 memory-filtered tests, two public memory-provider tests, six CLI diagnostic
tests and 17 Host artifact tests pass. The diagnostic and artifact fixes previously
blocked below are now exercised. During qualification the maintainer selected
Testkit 0.48; a scoped lock update selects 0.48.0 and removes Host Process 0.20.1.
Strict Core/Host/CLI library-and-test lint and public-provider lint pass on that
final graph. Behavioral runs preceded the Testkit update and retain their original
0.47 input scope. Evidence and incoming copies: `/tmp/canic-build08-check/`.

The Blob adapter check fails at its request return boundary: published Blob 0.28
exposes Memory 0.41 requests while Canic selects 0.43. The sibling Blob checkout
already prepares this alignment for 0.29; inspection was read-only. Blob #77 and
Canic #517 retain the publication/adoption dependency. No bridge or downgrade was
added. Root/private-consumer adoption and installed qualification follow publication.

Auth PR #64 remains open at `19ea243`; selected Auth 0.6.4 still lacks its session
mechanism/key selectors. Root enrollment, portable endpoint wiring, persisted
clocks/floors, cleanup and installed recovery qualification remain unfinished.
The complete 0.112 batch and changelog are not ready to publish. Live Backup remains
unavailable under the accepted release scope. No broad gate, own version change,
staging, commit, sibling edit or release ran. Older entries retain historical scope.

## Host transform write-policy adoption — 2026-10-16

Both Wasm shrink and Binaryen optimization now pass explicit Host WriteOptions:
Replace publication and 0666 staging permissions reduced by umask, matching the
previous Host 0.19 named-writer defaults. Qualification-before-publication and
producer failure handling are unchanged. The focused locked/offline Host library
check confirms both missing-argument errors are gone; five Build/Host OutputLimits
identity errors still prevent successful compilation and test execution.

Published Build remains 0.7.0 with Host Process 0.20. The sibling Build checkout,
inspected read-only, already prepares Host 0.21 alignment for pending 0.8.0 and
records targeted producer qualification. Do not duplicate that in-progress work
or introduce a consumer bridge. Adoption remains tracked by Canic #517 and
Build #9. Evidence and incoming copies: `/tmp/canic-host021/`. The #547 tests
remain unexecuted. The complete 0.112 batch and notes remain unready to publish;
no broad gate, dependency/version change, sibling edit, commit or release ran.

## Diagnostic range reporting prepared — 2026-10-16

Issue #547 is corrected locally: decimal codes outside u16 receive a distinct
CodeOutOfRange error; malformed syntax retains InvalidCode. Focused regressions
cover overflow, in-range unknown values, leading zeros and current catalogue
identities. Tests have not executed: the selected graph stops in Host compilation
with two missing WriteOptions arguments after Host FS 0.21 adoption and five
OutputLimits identity mismatches between Build 0.7's Host Process 0.20.1 and
Canic's 0.21. Source and failure evidence are under `/tmp/canic-547/`.

This is a dirty local fix, not delivered or qualified behavior. The existing
integration owner #517 retains the new dependency blockers. Auth integration and
the complete 0.112 release batch remain unfinished; notes are not ready for
publication. No dependency downgrade, broad gate, commit or release ran.

## Auth restoration association check — 2026-10-15

Restoration now checks each retained login with a corresponding session against
IC Auth's canonical session/login association API. A different account binding
or a revoked login with a retained session rejects before indexes are published.
Logout state without a session remains valid and preserves replay evidence.
The cross-record lookup is allocated only after existing state size/count limits.
This does not complete missing-row, clock/floor or live enrollment restoration.

The new regression failed before the correction. All 18 targeted session-storage
tests and strict Core library/test lint now pass on Auth 0.6.2, Metrics 0.11.3
and Timers 0.25. Evidence and incoming copies: `/tmp/canic-auth06-restore-binding/`.
No dependency selection changed. Upstream Auth PR #64 remains open; published
Auth is still 0.6.2. Root enrollment, the portable workflow, clocks/floors,
cleanup and installed qualification remain unfinished. The complete 0.112 batch
and its changelog are not ready to publish. No broad gate, staging, commit,
version transaction or release ran.

## Auth logout binding preservation — 2026-10-15

A focused regression reproduced a stable-adapter defect: after session removal,
a logout proposed from another account's login for the same transport caller
could replace the retained login binding. The adapter now requires binding
equality for logout as well as admission. The failed commit changes neither
stable state nor the proposing heap ledger; matching logout retries remain
idempotent and preserve replay custody through restoration.

All 17 targeted session-storage tests and strict Core library/test lint pass.
The current lock selects Auth/protocol-types 0.6.2 and Timers 0.25; no dependency
selection was changed by this slice. Inspection of published Auth 0.6.2 shows
that the session mechanism/key selectors are still absent. Evidence, the failing
regression before the fix, and incoming source copies are retained under
`/tmp/canic-auth06-logout-binding/`.

Root enrollment, clocks/floors, cleanup and the portable endpoint workflow remain
unfinished. These native checks do not qualify installed Auth or the complete
0.112 batch. The batch and changelog are not ready for publication. No broad
gate, staging, commit, version transaction or release ran.

## Auth policy and verifier diagnostics — 2026-10-15

The local session policy now uses Auth 0.6.1's maintained fields. The adapter
handles its current typed failures and preserves budget exhaustion and clock
rollback through both portable credential verifier envelopes. The production
session capability selects upstream portable signature verification. Revoked
credentials remain security refusals; storage failures remain unavailable.

Both focused policy/diagnostic tests and strict Core library/test lint pass.
Feature-enabled compilation now has two remaining errors, both in the old
establish/authorize workflow calls; the three policy/error-projection errors
recorded below are resolved. This run selects Metrics 0.11.2, so it does not
relabel earlier graph evidence. Evidence and incoming copies are retained at
`/tmp/canic-auth06-policy/`. No upstream or dependency-version edit was made.

Continue with the portable credential workflow and protected Root enrollment,
persisted clocks/floors, bounded cleanup and installed recovery qualification.
The complete 0.112 batch and its changelog remain unready for publication. No
broad gate, staging, commit, version transaction or release ran.

## Auth durable admission/logout continuation — 2026-10-15

The stable adapter now commits canonical login rows and protected pool totals
with session/replay admission, and commits logout denial with session-only removal.
Replay custody survives logout. Restoration validates row identities and exact
pool occupancy; failed staging, mismatched bindings and exhausted capacity leave
storage unchanged. Native fixtures now use signed portable credentials instead
of the old retained codec files. Runtime login clocks/floors, cleanup, Root
refresh and endpoint wiring remain unfinished; this is storage qualification,
not a working Auth-enabled deployment.

The selected Core graph now uses Memory 0.42. Its snapshot diagnostic is mapped
directly; the existing optional payload measurement remains unavailable because
allocation metadata does not measure payload occupancy. No dependency downgrade
or local override was introduced. Auth remains published 0.6.1; upstream
[PR #64](https://github.com/dragginzgame/ic-auth/pull/64) is still open.

On this graph, 369 Auth-filtered native tests, 30 role-contract tests, 11
state-contract tests and 15 memory-ops tests pass; strict Core library/test lint
passes. Feature-enabled compilation still reports five errors in the old
session policy/error projection and delegated-token workflow. The durable
transaction/test compile errors recorded below are cleared. Evidence and the
incoming source snapshot are under `/tmp/canic-auth06-transactions/`.

The complete 0.112 batch and its changelog are not ready to publish. Continue
with portable credential admission, persisted login clocks/floors and bounded
cleanup, Root issuance/refresh and installed recovery proofs. Earlier installed
fixture evidence does not qualify these changed sources or dependency graph.
No broad gate, staging, commit, version change or release ran.

## Auth 0.6.1 and caller-bound replay continuation — 2026-10-15

The maintainer's root lock now selects Auth/protocol-types 0.6.1. Both private
Blob consumer locks are aligned to that patch, and the root catalog requires
0.6.1 for its public session/login association check. No unrelated dependency
selection was changed by this continuation. Earlier installed qualification
below remains evidence for its original 0.6.0 graph.

Canic's session projection now permits distinct transport and account principals.
Replay records, pair checks and capacity accounting use the transport caller;
the obsolete account field is removed from replay storage. This is part of the
pending 0.112 reinstall-only cut. The local model regression now retains distinct
caller/account/issuer identities, and a new capacity case checks separate caller
budgets. These cases have not executed: Core native test compilation still fails
with eight errors in the old transaction adapter and its old codec fixtures.
Ordinary Core compilation and strict library lint pass on 0.6.1; this is not
Auth-enabled qualification.

The explicitly authorized isolated upstream proposal is
[Auth PR #64](https://github.com/dragginzgame/ic-auth/pull/64), head `19ea243`.
It exposes retained session mechanism/key selectors so Root enrollment lookup
does not parse private records or guess among an issuer's keys. All 20 upstream
session tests, strict affected library/test lint and the Rust 1.88 session-restore
library check pass. Merge/publication remain separate; Canic has no path override.

Evidence and incoming source copies: `/tmp/canic-auth06-durable/`. The durable
login/session/replay transaction, portable workflow, Root issuance/refresh and
installed restoration proofs remain unfinished. The complete 0.112 batch is not
push-ready; the notes are maintained but not ready to publish. No Canic staging,
commit, own version change, broad gate or release ran.

## Blob alignment qualified; Auth 0.6 boundary remains — 2026-10-15

Published Blob/contracts 0.27.1 and both private composition locks now share
Memory 0.41. The selected graph also includes Metrics 0.11, Timers 0.24, Build
0.6, Host 0.19, Testkit 0.46 and Auth/protocol-types 0.6. No compatibility bridge
or override was added. The embedded allocation peer and its structured evidence
are refreshed; provenance verification, adapter/internal compilation and strict
internal/integration fixture lint pass.

All four selected installed cases pass on the final fixed graph: checkpoint
freeze/restore (35.58s), dedicated Blob permissions/recovery (3m10s), embedded
application-state/recovery (3m09s) and production initialization target binding/
lost replies (16.44s). The governed PocketIC run takes 491.32s. The live-create
refusal regression passes and retains existing output unchanged. Earlier metrics
(86) and timer (12) cases passed before the Auth update; their input identity
remains separate. Evidence: `/tmp/canic-0271-ready/`, with final graph hashes in
`graph-auth06.sha256`. The first installed attempt overlapped the maintainer's
Auth update and failed on mixed protocol identities; it is retained separately.

Core's targeted native Auth check fails against 0.6: the session store still
implements the old transaction and subject-based replay contract. The new owner
requires atomic login revocation persistence, caller-based replay custody,
session-only removal and explicit restoration/verification features. Do not
ignore the proposed login change merely to compile. The existing
[Auth issue](https://github.com/dragginzgame/canic/issues/491#issuecomment-6192040229)
records the failure and required safety evidence. The maintainer has now resumed
Auth integration and selected Fleet Root management of issuer enrollment. The Blob consumer
blocker is resolved; its separate publication-classification concern remains on
[Blob #77](https://github.com/dragginzgame/ic-blob-storage/issues/77#issuecomment-6192039897).

The complete 0.112 batch is not push-ready on Auth 0.6. Open changelog notes are
updated; own versions remain 0.111.0 and publication readiness is blocked by the
Auth integration. Retain the accepted release scope with live Backup unavailable;
its resumed production Root workflow remains incomplete separate work. No broad
gate, staging, commit, version transaction or release ran. Older sections below
retain their historical dependency selections and limitations.

### Root-managed Auth continuation — 2026-10-15

The approved design is recorded at the top of the 0.112 extraction plan. Passive
enrollment contracts now bind the Fleet, exact Root/receiver installations, role,
fresh challenge, revision/generation, validity windows, approved issuer keys and
scopes, and terminal key revocations. They are not yet a callable endpoint or an
installed enrollment protocol. The local session capability selects IC Auth's
checked restoration feature; native verification tests select the verifier feature.

The unconnected enrollment validator proposal is retained outside production at
`/tmp/canic-auth06-integration/enrollment-proposal/`; it must be integrated with
the actual stable transaction owner rather than added as unused runtime code.
Remaining work is Root issuance and receiver refresh/startup closure, portable
credential admission, atomic login/session/replay persistence through shared commit
hooks, bounded cleanup/logout, complete restoration and installed recovery proofs.
Do not treat the passive contract checks as qualification of those workflows.
The existing Auth-enabled compile failures remain in scope; no additional
maintainer choice is needed to proceed.

Both enrollment Candid round-trip tests, strict Contracts library/test lint and
ordinary Core library compilation pass. Evidence is under
`/tmp/canic-auth06-integration/`. Auth-enabled compilation and installed admission
remain unqualified; the passing ordinary build does not cover that feature.

## Memory 0.41 adoption and Blob alignment — 2026-10-15

The maintainer updated the root catalog/lock to Memory 0.41.0. Registry checks
still report Build 0.6.0, Host 0.19.0, Backup 0.24.7, Blob 0.27.0, Query 0.56.0
and Testkit 0.45.0. Core compiles; the Blob adapter fails at its request return
boundary because published Blob exposes Memory 0.40 types. Keep the maintainer's
0.41 selection; no type bridge, downgrade or source override was introduced.

Three Memory test constructors now omit the removed declaration label. All 27
Memory-filtered cases and 21 checkpoint-model cases pass on 0.41; strict Core
library/test lint also passes. The numeric-slot
encoding and constructor hard cuts are documented in the open 0.112 notes;
pre-1.0 retained installations require the existing clean reinstall procedure.
Earlier installed Blob/checkpoint proofs remain evidence for their Memory 0.40
graph and do not qualify 0.41.

[Blob #77](https://github.com/dragginzgame/ic-blob-storage/issues/77) records the
exact consumer failure and public version boundary. The maintainer's local Blob
commit `b92ccb8` already selects 0.41; published main was `cc1a720` when checked.
No duplicate implementation PR or sibling edit was made. The pending 0.27.1
notes claiming unchanged contracts need to reflect the exposed identity change
before publication. Adoption in both private consumer locks and installed
composition qualification follow a published coherent Blob family.

Evidence and incoming copies: `/tmp/canic-memory041-adoption/`; initial compile
failure: `/tmp/canic-memory041-check.log`. Live Backup's production Root workflow
remains unfinished, independently of this dependency mismatch. The complete 0.112
batch is not ready to push or publish. Own versions, Canic Git index and release
records remain untouched.

## Published Build 0.6 qualification — 2026-10-15

The maintainer's graph now selects published Build 0.6.0. Its Host 0.19 boundary
removes the previous producer mismatch. Fixture workspace discovery uses the new
Cargo observation API through one Canic-owned policy: the internal fixture
manifest, 64 KiB stdout, 256 KiB stderr and no elapsed-time deadline. Cargo still
owns workspace membership; no path-layout guessing or compatibility import remains.
Default and governed fixture caller compilation and strict internal/integration
fixture lint pass on the actual registry graph.

The exact Testkit 0.45 runner is prepared through the governed setup target.
The single installed checkpoint case passes using that owner, PocketIC 16.1.0,
Build 0.6, Host 0.19, Blob 0.27 and Memory 0.40. It covers durable business freeze,
upgrade retention, snapshot capture/restore, fresh restore authority, delayed
requests and release replay. Its selected case takes 29.14s; the governed run,
including managed artifact preparation, takes 108.14s. This replaces the previous
candidate/old-driver qualification limitation for the participant proof.

Both managed Blob composition cases also pass on the published graph: dedicated
permissions/recovery and embedded application-state retention/recovery. All four
artifacts were rebuilt for each consumer. The selected run takes 336.91s; the
root catalog and all three lockfile digests remain unchanged. This supersedes
the pending installed Blob 0.27 qualification noted below.

Evidence and incoming source copies: `/tmp/canic-build06-adoption/`. Live Backup
still requires the production Root checkpoint store, authenticated complete-unit
orchestration, paid snapshot reconciliation and host restore integration. The
complete 0.112 batch remains unready; no own package versions, Git index or release
records changed. Build and fixture notes now identify the maintained 0.6 contract.

## Upstream recheck and Host proposal repairs — 2026-10-15

No newer registry versions are available than the preceding refresh: Build
0.5.0, Backup 0.24.7, Blob 0.27.0, Host 0.19.0, Query 0.56.0 and Testkit 0.45.0.
Build #44 is open and mergeable with successful Linux, macOS Apple Silicon,
macOS Intel and lint/security CI. Its publication still blocks the normal Canic
Host/fixture graph; candidate rehearsal evidence remains separate.

Host PRs #120 and #127 now incorporate released 0.19.0 (`804a78b`) through
normal isolated topic merges, at `415f86a` and `c89b1b3` respectively. Their
implementations are unchanged; compatible pending notes now select 0.19.1.
Two duration cases and 69 durable filesystem cases pass on Rust 1.88, alongside
strict affected-package lint, Rustdoc, formatting and diff checks. Hosted checks
must qualify these new heads. No integration branch merge or publication ran.
Evidence is retained in `/tmp/canic-refresh-followup-20261015/`.

The Root checkpoint storage/topology trace confirms these must be coupled to
complete authenticated issuance/release and participant discovery. No unused
store, standalone freeze gate or live-create bypass was added. Coordinated Backup
implementation remains unfinished; the full 0.112 batch is not push-ready.
Canic source, dependencies, package versions and Git index remain unchanged by
this follow-up; the handoff records the upstream contribution progress.

## Published dependency refresh — 2026-10-15

Registry inspection confirms Build 0.5.0, Host 0.19.0, Testkit 0.45.0,
Query 0.56.0 and Backup 0.24.7. Blob/contracts 0.27.0 became available during
this work; the root and both private consumer locks now select it with Memory
0.40.0, without changing unrelated pins. The registry-backed Blob adapter check
passes. Installed composition qualification on this exact graph remains pending.

Testkit 0.45 callers now use fallible installation/Candid APIs, nonzero retries
and a fallible baseline builder. Canic owns the fixture target paths removed
by Build 0.5. The source changes from draft Canic PR #572 were reconciled with
current local fixtures rather than replacing their incoming changes.

Build 0.5 still exposes Host 0.18 output-limit types while Canic selects 0.19.
The isolated [Build #44](https://github.com/dragginzgame/ic-build/pull/44)
aligns its three Host dependencies and classifies the exposed identity change
for pending 0.6. Five observation and six real Cargo example cases pass on Rust
1.91; strict library lint passes on the default toolchain. Publication is needed
before normal Canic Host/fixture qualification can pass. No shipping override,
compatibility shim, dependency downgrade or maintainer sibling edit was added.

A frozen consumer rehearsal with only that Build candidate substituted passes
internal/test-package caller compilation and strict internal fixture lint,
including the governed feature; strict integration-test lint also passes.
Checkpoint fixture lint now separates restore
assertions and removes redundant Copy-error clones. This is candidate evidence, not
published adoption. Evidence and incoming copies are retained under
`/tmp/canic-upstream-refresh-20261015/`. The accepted coordinated live Backup
work remains incomplete; its Root stable store, topology gate, paid snapshot
reconciliation and production capture/restore path still need implementation.
The full 0.112 batch is not push-ready. Changelog notes are updated; Canic's own
versions, Git index and release records remain untouched.

## Live Backup implementation resumed — 2026-10-15

The maintainer now requests getting Backup working. This supersedes the temporary
implementation deferral; the accepted consistency contract remains application
coordinated, with framework-owned fixtures. The design/status tracker records
Root state, topology exclusion, paid capture/reconciliation, host custody and
same-release restore as one complete outcome. Live create must keep refusing
until the actual production path and its interruption evidence pass.

The Root checkpoint model now retains a bounded complete unit and Root-issued
sequence, replays an exact original begin without issuing a second sequence,
rejects stale issuance and changed installation/membership, and retains release
intent even for unanswered prepare calls. Every participant must acknowledge
before the aggregate reaches Prepared or Released. Membership admission remains
closed until Released. CBOR decode validates the retained identity and phase.
All 18 participant/Root native cases and strict Core library/test lint pass.
Evidence/before-images: `/tmp/canic-live-backup-20261015/`.

This slice exposes the canonical model through Core's shared control-plane
support; it does not install a Root store or dispatch a snapshot. Next implement
that stable owner and topology gates, then authenticated Root command/RPC steps
with existing paid-effect custody, followed by host capture/restore integration.
The installed participant proof is already retained from the previous batch;
current installed qualification is blocked by Blob 0.26 / Memory 0.40. Do not
replace the graph with overrides or infer that native journal tests enable live
backup. The complete batch remains unready, with no own version/Git/release effects.

## Upstream integration PR repairs — 2026-10-15

Four existing PRs were reconciled with their latest released upstream bases in
isolated checkouts and pushed without integration merges or publication:
[Build #38](https://github.com/dragginzgame/ic-build/pull/38) at `aaff825`
and [Build #37](https://github.com/dragginzgame/ic-build/pull/37) at `9b3b67c`
now build on released 0.5.0; [Host #120](https://github.com/dragginzgame/ic-host-tooling/pull/120)
at `b3d267e` and [Host #127](https://github.com/dragginzgame/ic-host-tooling/pull/127)
at `7d06ae5` now build on released 0.18.6. GitHub reports all four conflict-free;
hosted checks must qualify these new heads before upstream integration.
Published notes and metadata are preserved, with compatible pending notes updated.
No maintainer sibling checkout or Canic dependency graph was changed.

Focused evidence passes: Build configuration has three public cases, four input
mutation/include/alias cases and one compiler-identity case on Rust 1.91; flags
has its real-Cargo harness with five scenarios on Rust 1.91. Host duration has
two cases and filesystem primitives have 69 durable cases on Rust 1.88. Strict
affected package lint, warning-denied Rustdoc, formatting and diff checks pass.
Retained checkouts/logs are under `/tmp/canic-upstream-prs-20261015/`.

Backup custody/process integration is delivered. Further generic engine adoption
and live Root coordination are distinct: publishing Host filesystem helpers does
not implement Root-issued checkpoint sequences, complete-unit exclusion, paid
snapshot reconciliation or restore/release authority. The accepted live-backup
deferral and installed participant proof are recorded on
[#550](https://github.com/dragginzgame/canic/issues/550#issuecomment-6189432882).
The registry now has Build 0.5.0; normal Canic adoption still needs removal of its
remaining fixture-only test-target helper callers. Blob remains published at
0.26, so the Memory 0.40 mismatch still blocks the narrowed release batch.

## Shared Tooling 0.15 adoption — 2026-10-15

Refreshed the existing snapshot selection from committed upstream HEAD
`3bef3f2daec7f64f875397ed4d53d5b9e2e68a08` (version annotation 0.15.0),
verified against the remote HEAD. The exporter now records 115 payloads, including
the required pure version/date validator. This is exact-commit adoption, not a
claim that every pending feature listed in upstream changelogs has shipped.
Canic's isolated release fixture now includes that required helper. Current
reporting rules count genuinely created/deleted symbols only; changelogs describe
outcomes and consumer actions without structural symbol inventories.

Snapshot integrity, governance closure (227 references across 47 documents) and
Canic's real-Cargo/fake-Git transaction/rollback fixture and shared release
command simulation pass. Focused ShellCheck with sourced companions and Perl
syntax admission also pass. Repository description
matches the maintained framework scope. Evidence and incoming dirty-file copies
are retained in `/tmp/canic-shared015-adoption/`. The 0.112 notes and adoption
guide identify the new revision. No application dependencies, own package versions,
Git index/refs or release records changed. The previously recorded Memory/Blob
integration blocker is independent of this tooling adoption.

## Accepted release scope and current blocker — 2026-10-15

The maintainer selected **ship integrations; keep live backup unavailable**.
The 0.112 draft now explicitly defers Root-paid capture, membership exclusion and
coordinated restore/release under #550. Pending unpublished Host/Build extractions
and the Auth redesign do not gate this narrowed release. Existing Backup custody
and checkpoint participant SDK remain included. Live create still returns its
typed unavailable error before resolving or creating any output; dry-run is only
planning. This decision supersedes earlier handoff statements requiring complete
live Backup or every pending upstream extraction for 0.112.

The dependency graph changed during checkpoint qualification: the root requests
Memory 0.40 and selects Metrics 0.10.1, while Blob 0.26 uses Memory 0.39.
Offline metadata reconciled the root lock without updating other selections.
The participant fixture checks successfully on Memory 0.40, but affected internal
lint cannot compile the Blob adapter because its public memory declaration types
come from different crate versions. Registry inspection confirms Blob 0.26.0 is
the newest published release; sibling 0.27 is a draft targeting Memory 0.40.
Do not bridge these types or silently revert the maintainer's Memory selection.
A published matching Blob family and aligned private consumer locks, followed by
owning adapter/composition checks, remain required before declaring push readiness.

The new installed participant fixture covers business freeze, lifecycle recovery,
delayed requests and snapshot restoration; it is not Root orchestration. The SDK
now exposes its canonical checkpoint binding. Qualification evidence is retained
under `/tmp/canic-112-checkpoint-20261015/`. The installed participant case now
passes with a Memory 0.40 Wasm; its native driver was built before that graph
change. This is a scoped participant proof, not complete current-graph validation.
The participant fixture also passes warning-denied lint, and the registered-case
partition regression passes using the retained native driver. Internal lint is
blocked before reaching the new test module by the Blob memory mismatch.
The CLI live-create refusal regression also passes on the current root graph:
new output is not created and retained output is unchanged.
The initial fixture omitted activation and was corrected; the next attempt was
stopped by the changed locked graph before execution. No broad gate, package
version change, commit, push or release has run.

## Remaining fixture Cargo observation — 2026-10-15

The registry index still selects IC Build 0.4.0 as the newest published release;
0.4.1 remains an upstream documentation draft. Canic already meets its Rust 1.91
floor and has no callers of the removed Wasm-path convenience functions.
The embedded-peer source builder now uses IC Build's canonical metadata
observation with 16 MiB stdout and 1 MiB stderr limits. Its locked/offline recipe,
local-version normalization and external-lock preservation remain Canic-owned.
The internal testing package explicitly selects the existing Host process
workspace dependency for capture limits; no dependency version changed here.

Four fixture regressions, strict internal-library lint and current-source
embedded-peer verification pass on Build 0.4.0, Host 0.18.5 and the maintainer's
Timers 0.23.1 selection. The explicit relocation/version real-Wasm test remains
ignored; it was not run. Evidence and task before-images are retained under
`/tmp/canic-build04-continuation-20261015/`. Duration PR #120 is still open;
upstream scratch and Cargo configuration/flag owners remain pending. This
completion does not make the full 0.112 batch push-ready: coordinated Backup and
remaining upstream delivery are still open. Changelog notes are updated; package
versions, Git index and publication remain untouched.

## Blob 0.26 adoption — 2026-10-15

The adapter now delegates continuity preparation, version observation, resume
orchestration and certificate issuance to published Blob 0.26.0. Both private
consumer locks select Memory 0.39.0, Metrics 0.10.0 and Timers 0.23.0 alongside
Blob 0.26.0, matching the framework without updating unrelated transitive pins.
Memory diagnostics now expose the current snapshot; Metrics projections use the
current aggregate API. Testkit 0.44 callers use current capture/restore, error
fields and native PocketIC time access. Remaining nested nullable-decoder callers
now use Contracts directly.

The adapter check, 15 Memory cases, 17 Metrics cases and the Memory Candid
roundtrip pass. Strict Blob/Core/Host/CLI and changed integration-test lint pass;
both Host nullable-owner and capacity-import decoding regressions pass.
The embedded allocation peer was refreshed and its verifier
passes. Dedicated and embedded managed composition cases pass on IC Build 0.4.0 and
Host 0.18.5. The root catalog changed during the first embedded build, invalidating
its follow-up cache admission; both cases were rerun on the final stable graph.
Adoption evidence is recorded on [Blob #72](https://github.com/dragginzgame/ic-blob-storage/issues/72#issuecomment-6187960855).
Retained evidence and source before-images are under
`/tmp/canic-blob026-20261015/`. This is framework qualification; application/provider
acceptance and coordinated live Backup remain separate unfinished obligations.
The complete 0.112 batch is not yet ready to push or publish; package versions
and the Git index remain untouched.

## JSON and timestamp consolidation — 2026-10-15

Fleet and build-lock records now call Contracts' required-nullable decoder;
Backup validation, runner and CLI report consumers share a pure timestamp parser.
Missing/null semantics and caller-specific malformed-timestamp policies remain.
Five timestamp-filtered, three Contracts serialization and 18 Backup preflight
cases pass. Host's owning nullable-field regression cannot compile because the
current Core dependency adoption has unrelated Memory diagnostic and Metrics
summary API errors; CLI qualification shares that dependency boundary. No broad
gate or unrelated dependency repair ran. Source before-images and logs are in
`/tmp/canic-json-time-20261015/`.

[Host #122](https://github.com/dragginzgame/ic-host-tooling/issues/122) owns the
prepared invocation scratch API at `/tmp/canic-host-scratch-20261015/`. Four
focused Linux cases, strict lint, Rustdoc and Rust 1.88 checks pass. The Canic
adoption patch passes applicability checking but waits for registry publication;
current Candid argument-file handling remains intact. This slice does not make
the complete 0.112 batch ready to push or publish. Package versions and the
current checkout's Git index remain untouched.

## Staging and text extraction — 2026-10-15

Download and restore staging now delegate private directory creation to published
Host FS. Native I/O kinds and before/after-creation evidence remain available;
Canic owns journal sequencing, namespace custody and stale-stage cleanup.
Twelve staging cases, 36 restore-runner cases and strict Backup library/test
Clippy pass in the isolated package workspace
`/tmp/canic-backup-staging-20261015/`. The main locked invocation refuses the
concurrent Testkit catalog 0.44 / lock 0.43 mismatch before compiling. Its lock
was not changed; isolated evidence is not full current-workspace qualification.
Before-images, checks and the pending duration adoption patch are retained under
`/tmp/canic-extractions-20261015/`.

[Host PR #120](https://github.com/dragginzgame/ic-host-tooling/pull/120) supplies
compact duration formatting, with focused Linux tests, strict lint and Rust 1.88
checks. Canic's direct CLI adoption waits for publication; no live path override
or compatibility reexport is added. Native macOS remains upstream qualification.
The proposed complete Wasm-tool relocation was narrowed after reading Host's
ownership contract: pins, installation policy, archive recipes and differing
admission requirements stay local. The remaining generic bundle-publication gap
is tracked by [Host #121](https://github.com/dragginzgame/ic-host-tooling/issues/121).
Existing Host directory PRs own deeper staging primitives; no competing owner
was added. These scoped changes do not complete the accepted 0.112 batch or make
its changelog/version surfaces ready for publication. Canic remains uncommitted.

## Published Cargo observation owner — 2026-10-15

Discovery, role-package admission and complete-build export inventories now use
published IC Build 0.3.3's bounded observations and canonical Cargo metadata types.
The private schema, feature argument builder and unbounded capture are removed.
Canic retains commands, 16 MiB stdout/1 MiB stderr limits, sanitized diagnostics
and package/recipe authority; the existing #553 export repair is preserved.
The catalog minimum is 0.3.3; no source override or own version change is added.

The 41 role, 44 reuse/export, seven Fleet, 18 bootstrap, ten artifact and four
canonical-selection checks pass. Strict Host library/test Clippy passes; actual
Rust 1.91 library checking passes in a frozen copy on the unchanged incoming lock
`1fb060c8029f246f1841f67d2d6c61c8e8bda51cc3c07ef03fa10d35db64372f`:
Build 0.3.3, Host 0.18.3, Testkit 0.43, Memory 0.38, Metrics 0.10, Timers 0.22.6.
The opt-in real-Cargo cold/warm/relocation/source-change matrix also passes
in that copy; its synthetic release sealing is not installed Fleet acceptance.

Another task raised the live Memory requirement to 0.39 during qualification;
the live locked-check refusal is retained separately. The older graph's passing
checks do not qualify the independent adoption or current installed artifacts.
Build #33 and Canic #530 retain delivery, native and current-graph/application
acceptance. Source and pending 0.112 notes stay uncommitted; no version, release,
publication or live effect ran. Before-images, failed attempts and frozen-source
evidence: `/tmp/ic-build-canic-adopt033-20261015/`.

## Selected build-script inputs — 2026-10-14

The local [#553](https://github.com/dragginzgame/canic/issues/553) correction
retains Canic include exports from current Cargo JSON messages, associated with
exact compiler-artifact package IDs and canonical manifests. Config, profile,
network, target and build environment bind each observation recipe. The selected
App and generated Fleet catalogs admit producer records through Host's bounded
descriptor reader; IC Build still owns input evidence and before/after refusal.
Same-name foreign workspaces, malformed/nonregular foreign records and replaced
selected records have owning regressions. Selected external includes remain
observed before depfiles exist; late discovery and byte changes still refuse.

All 44 reuse, 10 artifact and 18 bootstrap cases, the opt-in real-Cargo cold/warm/
relocation matrix and strict Host library/test lint pass on the final root graph.
The matrix seals synthetic release records; it is not an installed Fleet proof.
The root lock changed externally during qualification to Timers 0.22.6 at SHA-256
`1b7633d5a6ad5aa7fceffc864d4aa03a82b5e5f2dcdea6327a95436d12444283`.
Catalog and Shared snapshot remain unchanged. Earlier installed composition and
embedded-peer receipts, and both private Blob locks, retain Timers 0.22.5; they
are not current 0.22.6 installed acceptance. Evidence/before-images are retained
under `/tmp/canic-553-20261014/`. One intermediate real-Cargo fixture failure
exposed the already-diagnosed shared-target build-script clobbering hazard;
foreign input acquisition is qualified independently from subsequent recompilation.
The full 0.112 batch remains open for coordinated Backup, upstream ownership,
delivery/native-host acceptance and new-graph alignment/installed qualification.
No version, Git publication, release or live effect ran.

## Previous adoption and first sediment removals — 2026-10-14

The maintainer requested latest Shared Tooling/Host adoption and implementation
of the audit findings. That adoption selected Host 0.18.2 (including CLI-only Text),
Build 0.3.2, Backup 0.24.5, Blob/contracts 0.25, Query 0.55.8, Testkit 0.42,
Memory 0.38, Metrics 0.9.6 and Timers 0.22.5. Both private Blob locks now select
Blob 0.25/Memory 0.38 and the same Metrics/Timers family. Root lock SHA-256:
`1a9464b6c5ce5fc86c877947c341d24672b8fbcfd622c33991d7962ff444951c`.
Shared's clean main snapshot is 0.13.3 at
`d963db4b85b4fa65560fd87e8398f228955a2b71`, with 114 verified payloads.
Its selected payloads match 0.13.2; the future version/date/prose helpers in its
notes remain pending PRs and are not adopted as implemented APIs.

The obsolete Memory reservation callback/direct-call fixture is removed (#559).
Bootstrap admission, policy identity and explicit namespace grants remain.
CI/dev callers use the maintained shared CI installer and common lint pins;
additional platform digests and Canic tool policies remain local (#563).
Native query fixtures use bounded Testkit HTTP acquisition; ordinary executable
fixtures use Host's fork-safe writer. Fleet progress delegates text escaping to
Host, while interactive clipping, ANSI layout and screen custody remain local.
Backup layout/restore/CLI record acquisition shares Host's bounded descriptor
reader, with a matching 64 MiB publication limit (#562). Snapshot artifacts have
independent limits; schema/terminal receipt validation and path custody remain.

The 27 memory cases, 47 Host fixture cases, 11 JSON/durability cases, 36 restore
runner cases, 42 CLI restore cases (one ignored), 33 progress cases and strict
affected Core/Host/Backup/CLI library/test lint pass on their recorded inputs.
Host 0.18.2 reruns pass; source/lock drift checks retain their exact scope.
Snapshot/governance/link checks and the four-platform installer/Make fixtures,
including Bash 3.2 admission, pass. Testkit 0.42/PocketIC 16.1 are prepared and
admitted. The allocation peer is refreshed and read-only verification passes
against the selected lock. The subsequent Timers 0.22.5 update is fetched and
both private locks aligned; its 27 memory and six timer cases and refreshed-peer
verification pass. All three installed cases pass on that unchanged final graph:
dedicated composition, embedded composition and production Root initialization/
recovery. A declaration build first ran out of space; `target/` was then removed
externally. The retry rebuilds and passes without agent cleanup. Original failed
and earlier-graph receipts remain distinct. Final four-package strict lint also
passes on the unchanged Timers 0.22.5 graph.
Evidence and incoming dirty before-images: `/tmp/canic-removals-20261014/`.

The complete 0.112 batch remains **not ready to push or publish**. Coordinated
live capture/restore (#550/#394), generic Backup engine convergence (#513),
selected generated-export evidence (#553), Build metadata ownership (Build #33)
and shared Blob continuity/certificate orchestration (Blob #72) remain open.
Auth redesign is deferred. No broad gate, Git publication, package version or
live effect ran. Earlier receipts below retain their original graphs.

## Shared Tooling and recent changelog layout — 2026-10-14

The maintainer requested the latest Shared Tooling adoption, local PR inspection
and revision of recent release notes. The canonical exporter selects clean source
0.12.8 at `eafa07cac1fdba7d6303d061ed8af458aacf32b7`, verified against remote main,
with 114 payloads. New derived guides describe Rust preparation and CI change
selection; this refresh changes no tool pins or Canic workflow selections.

The new shared readable-layout policy replaces the local one-bullet restriction.
Root and detailed notes for pending 0.112, published 0.111, 0.110.54 and 0.110.53
now separate outcomes and upgrade actions, keep descriptive issue links at their
owners and distinguish live-backup/graph limitations from implemented mechanisms.
Intermediate 0.112 adoption narratives are consolidated; all existing issue
destinations remain represented. Older historical sections, published version
identities/dates and the recorded 0.110.53 minor-boundary conflict are retained.
The pending version remains 0.112.0 because public Rust, access, memory and journal
contracts change; contributor-only Shared Tooling adoption is not its breaking cause.

Both open local source PRs are inspected without modifying their branches:
#546 at `f76ea5bf3835462920f8f4ff383cb6b979aecd95` and #519 at
`a25f4fd22eab23e1a2a5fa9947a9bbfa064e25a0`. Their exact hosted Linux checks,
ordinary tests and MSRV pass; both PocketIC jobs fail the embedded allocation-peer
preflight before server/test startup. Both macOS lanes are queued at inspection.
Existing #551 owns the repair, which already has active isolated-checkout work;
no duplicate implementation or integration is performed.

Snapshot integrity, source-owner refresh fixtures, documentation links and the
existing compiled changelog identity test pass. Evidence/before-images are under
`/tmp/canic-shared-changelogs-20261014/`; the identity test reads the current notes,
and is not a current-graph Cargo qualification. This scoped documentation/tooling
task is complete; the full integration batch remains not ready to push/publish
for the previously recorded runtime graph and live-backup work. No broad gate,
Cargo/version/index/ref mutation, PR merge or live effect ran.

## Maintainer Cargo update recheck — 2026-10-14

The maintainer updated the root catalog and lock. It now selects Build 0.3.0,
Backup 0.24.3, Blob/contracts 0.23.4, Host 0.17.13, Query 0.55.7, Testkit 0.40.0,
Memory 0.37.0, Metrics 0.9.4 and Timers 0.22.3. Auth redesign remains deferred.
Root lock SHA-256 is
`a03c6dd8306baabd8d738ea4305b0875256f468cfbb0cc8eb0530ede16a90a45`;
both private Blob locks and Shared Tooling 0.12.5 remain unchanged.

The focused initial compile exposed Memory 0.37's allocation-summary hard cut.
The Canic projection and direct report assertions are corrected; all 15 owning
allocation/bootstrap/conservation cases and eight memory-metrics cases pass.
The prepared Testkit 0.40 caller migration is applied: borrowed-server construction
uses `build_instance`, and standalone/root/registry baseline captures transfer an
explicit instance owner. Governed internal library compilation and strict lint
pass. Strict all-feature Core/Host library/test lint passes after correcting the
proposal's map/unwrap lint. Original failed attempts are retained.

The remaining compile blocker is published Blob 0.23.4's Memory 0.36.4 dependency.
The root graph contains both Memory versions; the adapter cannot return Blob's
0.36 `MemoryRequest` as Canic's 0.37 request. Crates.io still reports 0.23.4 as
the latest Blob release at this inspection. Its owner already records the pending
0.24.0 adoption under [Blob #68](https://github.com/dragginzgame/ic-blob-storage/issues/68).
No duplicate PR, local override or conversion between runtimes is introduced.
Both private consumers still select Memory 0.36.4 and Timers 0.22.2; align all
three graphs with the published compatible Blob family before installed checks.
[Canic #548](https://github.com/dragginzgame/canic/issues/548) and
[#549](https://github.com/dragginzgame/canic/issues/549) retain their delivery and
installed qualification requirements; the owning issues have current evidence.

Earlier Blob/PocketIC and embedded-fixture receipts bind the preceding graph.
The embedded fixture's recorded lock differs from this selection; current-source
verification/refresh remains required after the Blob compile blocker clears.
No updated-graph installed journey or broad gate is claimed. Evidence and source
before-images are in `/tmp/canic-update-check-20261014/`. The complete 0.112 batch
remains **not ready to push or publish**: graph convergence and installed checks
remain, as does coordinated live backup under #550/#394. Changelog surfaces retain
the open 0.112 draft; own package versions, Git refs and live effects are unchanged.

## Previous registry graph and checkpoint participant — 2026-10-14

The maintainer resumed implementation and requested adoption plus owning issues/PRs.
The root now requires published Build 0.3, Backup 0.24.3, Blob 0.23.4, Host 0.17.12,
Query 0.55.6 and Testkit 0.39. Auth redesign is explicitly deferred. The root lock
selects those releases, Memory 0.36.4, Metrics 0.9.4 and Timers 0.22.2. Both private
Blob consumers select Blob/contracts 0.23.4 and those same Memory/Metrics/Timers
versions; their unchanged Auth selection remains separate.

Shared Tooling 0.12.5 is adopted from clean committed source
`4256171b5b7ec4c4cc4078548c1c1617ee150699`, with 112 verified payloads and unchanged
common tool versions/checksums. Snapshot, governance links, IC installer fixtures,
Canic formatter/tool-recipe fixtures and actual prepared tool checks pass.
The selected Testkit 0.39 runner and PocketIC 16.1.0 are prepared explicitly.

On this graph, 41 Build reuse cases (one optional performance case ignored),
30 Query receipt cases, 309 Backup native cases and 42 CLI restore cases (one
ignored) pass. Strict affected Host/Backup/CLI library/test lint passes. Generated
Cargo metadata requires the explicitly inherited offline setting for offline
qualification; the initial network-resolution failure remains recorded.

The interrupted checkpoint draft now has an application-facing SDK record API,
exact original restore-source retention and durable release high-water behavior.
Wire and stored records share one canonical phase enum; the duplicate is removed.
The SDK endpoint predicate authenticates the issuing Root against Canic's
independently installed receiver authority. Root recovery authentication remains
independent from ordinary application admission; it confers no freeze/effect claim.
All 15 participant ordering/replay/serialization/Candid and authority cases and strict Core/SDK
library/test lint pass. This qualifies local transitions, not application freeze
or IC effects. [#550 — Coordinated Fleet backup checkpoints](https://github.com/dragginzgame/canic/issues/550)
owns remaining installed-Root authority, membership exclusion, application hooks,
Root-paid capture and outside-snapshot same-release restore/release qualification.
Live creation remains unavailable under #394; generic engine adoption stays #513.

Exact producer hosted results are separate: at inspection Build 0.3 CI is queued; Host 0.17.12's
run is cancelled. Testkit 0.39 passes the other eleven hosted jobs but its Intel
checks fail in the synthetic async peer fixture. Its owner has a pending repair
under Testkit #68 and a separate unpublished 0.40 ownership cut; Canic #548 owns
that caller migration after publication. No local override or premature 0.40
selection is introduced. Earlier producer failures are historical.

Current evidence and source before-images: `/tmp/canic-adoption-20261014/`.
All three installed Blob 0.23.4 journeys and all six Testkit payload-limit cases
pass on the selected graph. The embedded fixture is refreshed and read-only
verification passes. The final participant/guard source is explicitly refreshed
and its read-only source/toolchain/artifact verification also passes. Blob/payload
receipts retain their original artifact/source scope before the unused SDK-only
refinements; their consuming behavior is unchanged and is not silently rebound.
The complete
0.112 batch remains open and not ready to push or publish. No broad Canic gate,
version, Git publication, live deployment or paid effect ran.

## Framework live-readiness completion — 2026-10-14

The maintainer requested completing Build, Blob, Auth, Backup, IC tools and
Shared Tooling, and selected **Canic framework only** for this qualification.
The existing 0.112 batch stays open; package versions stay 0.111.0.

Build 0.2.6 now owns bounded selected Cargo-depfile reads, parsing and input
evidence. Unrelated App records no longer contaminate selected recipes; malformed,
changed and newly observed selected inputs refuse. All 40 reuse cases and the
actual generated-input preflight pass. Published Query 0.55.5 owns certificate,
tip and bounded block-chain validation; 30 receipt cases preserve Canic mint
binding. The Store's installed held-reply/Root-restart recovery case and all eight
native-agent cases pass on their recorded graphs. All three Blob framework
journeys and all eight native-agent cases pass on the frozen selections.

The separately authorized Host child transfer API is published in 0.17.8 and
selected by Canic. Backup uses published 0.24.2 journal, command and layout guards.
Original command identities are persisted before dispatch; missing/replaced
custody refuses. Fresh complete terminal bytes and exact quiescence govern
reference release. All 309 Backup cases, 42 CLI restore cases and the one-spawn,
bounded-output Host regression pass. Strict Backup/Host/CLI library/test lint
passes with all features. The registry-only custody composition proves descendant
exclusion, cancellation and output-overflow cleanup. Local generic lock/reference
engines and raw descriptor handles are removed. Live create remains unavailable:
actual Fleet capture consistency/authority and a same-release capture/restore
journey remain required under #394/#513.

Shared Tooling 0.12.0 is adopted at f7fe31bf80a2b445bf37a470527f001210241aea,
with 111 verified payloads. Governance links, snapshot/release adoption, IC installer,
formatter and local tool adapters pass. Exact prepared host/IC/Rust/Testkit tools
pass offline checks. The selected manifest, lockfiles and snapshot are frozen;
qualification checks their hashes, with receipts under `/tmp/canic-112-completion/`.
The stale embedded allocation peer refusal was reproduced, explicitly refreshed
from these inputs and then passed read-only verification. Its old bytes and failed
receipt remain retained. Exact Host 0.17.8 CI passes Linux/MSRV but both macOS lanes fail; the Intel
log proves a nonportable FIFO fixture. Producer correction/adoption and exact
native acceptance remain required. The frozen selections remain unchanged.

Auth's accepted pending cut is now 0.6, including `session-restore`, explicit
storage/reset policy and receiver-wide `SessionAdmissionBudget` (#58/#59).
Published 0.5.10 does not provide those APIs; its current-source consumer proofs
cannot qualify the pending contract. Shape validation and coordinated Canic
framework adoption remain open. Downstream application/provider acceptance is
outside this maintainer-selected framework scope.

Evidence and before-images: `/tmp/canic-112-completion/`. Linux proofs do not
establish native macOS acceptance. The complete batch is **not ready to push or
publish**. No broad Canic gate, version/Git/publication or live effect ran.

## Testkit 0.38 payload-limit caller migration — 2026-10-14

The previously approved caller migration under
[#540](https://github.com/dragginzgame/canic/issues/540) is applied to the existing
0.112 batch. The suite uses Testkit's standalone recipe and baseline pool with
one slot, lazy construction, retained canister metadata and the original explicit
cycle top-up. Payload, dispatch, lifecycle and Candid-boundary assertions remain.
The already-selected Testkit 0.38 requirement and own package version are preserved.

Focused compilation and all six tests through `make test-pocketic-case
CASE=pic_ingress_payload_limits` pass; the live artifact observes one cold build
and five warm restores. Exact CLI preparation was explicit after offline admission
refused the missing selected 0.38 runner. An initial source-cache miss was prepared
separately with locked fetch, without dependency resolution.

Strict lint is incomplete: concurrent Host reuse changes trigger too_many_lines
in input_snapshot; the caller-only attempt then stops on missing IC Query
certificate-host/ledger-evidence-host features in concurrently changed receipts.
Those inputs remain intact. The tests qualify their compiled artifact, not the
whole concurrently edited batch. Logs: `/tmp/ic-testkit-issues-20261014/`, with
full live output in `target/test-runs/20261014T065121Z-3666646.8CJlEX/1.log`.
Delivery, complete current-source validation and native macOS acceptance remain
outstanding; Host 0.17.8 is still unpublished. No version, index, ref or release
mutation ran for this migration.

## 0.112 duplication-removal plan and Query extraction — 2026-10-13

The maintainer requested the existing 0.112 design's duplication-removal plan
and implementation, then separately authorized scoped IC Build/Query work.
The [design](../design/0.112-standalone-blob-service-extraction/0.112-design.md)
and [batch status](../design/0.112-standalone-blob-service-extraction/status.md)
record Backup custody, Build input evidence, read-only evidence and Auth owners.
The concurrent published Build adoption below is preserved rather than repeated.

Canic local replica queries now use IC Agent's envelope/response owner. Delete
the private CBOR codec and golden codec tests; retain environment/readiness
projection, anonymous selected-origin HTTP, no proxies/redirects, one request
on HTTP refusal, typed rejection and the 30-second overall timeout. All 13
owning HTTP/projection tests and strict all-feature Host library/test Clippy pass.
Logs: `/tmp/canic-112-replica-tests-final-pass.log` and
`/tmp/canic-112-host-clippy.log`. Earlier source-race, sandbox and fixture failures
remain retained and are superseded only for their corrected source/command.

IC Query's uncommitted 0.55.5 draft now owns bounded certificate/witness/tip
authentication and immutable bounded generic block-chain validation. Nine new
cases, 18 directly affected report/certificate regressions and scoped strict
library/test lint pass. The default library remains free of the optional host
engines. A frozen Canic candidate replacing its certificate/value-budget/chain
engines passes all 30 receipt cases and strict Host library/test lint, preserving
network/account/memo/fee/amount binding, Candid structure and original mint
authority. Its source override exists only under
`/tmp/canic-112-certificate-consumer-02/`; the real Canic graph remains registry-only.

Query publication and actual registry adoption are still required before deleting
those remaining Canic receipt engines. Backup's original-command/terminal custody
integration and Auth's published validation adoption remain open. The complete
0.112 batch is not ready to push; source counts establish no build-time or Wasm
improvement. Own package versions, Git refs, releases and live/paid effects were
not changed by this task; independent maintainer work retains its own authority.

## Published IC Build input-engine adoption — 2026-10-13

The maintainer authorized Canic integration after publishing IC Build 0.2.2.
The catalog now requires that registry API; the incoming lock already selected
0.2.2 and remains unchanged. Its archive checksum is
`d408281094f1ae6928f02102131a904b8edcf5693b6c8f01b105e6488d0c4d82`, with
VCS receipt `e9be97fa356c14cb5431b879e304a0f8cc5077c3`.

[#530](https://github.com/dragginzgame/canic/issues/530) now delegates per-path
input capture/digests and before/after mutation guards directly to IC Build's
`InputEvidence`; selected retained Cargo records use its strict discovery API.
The private scanner, snapshot module and parser are removed. Two remaining
generic regressions move to IC Build, whose complete Linux gates pass; Canic
keeps product recipes, required toolchain admission, generated export/fixture
selection, source custody, complete-release format/namespace, finalized release
bindings, output projection and diagnostic serialization. There is one generic
traversal owner and no compatibility reader or incidental cleanup.

The targeted Host reuse suite passes 40 cases, including real Cargo generation,
external/disappearing input evidence, cold/repeat release reuse, environment/tool
changes, tampered outputs, rejection diagnostics and cancellation. Its opt-in
synthetic-sealing performance case remains ignored. Evidence and task-owned
before-images are under `/tmp/ic-build-canic-022/`; `canic-final-reuse-tests.log`
owns this pass and `canic-final-clippy.log` owns the strict Host library/test
Clippy pass. The incoming graph selects Host 0.17.3/Query 0.55.4; locked cache
preparation is explicit, with lock SHA-256
`0b30060e6d39ff08ac2c54b2d07de74b21f10863e19426ebba89a5630d78bc10`
unchanged through final checks. The query owner resolves the earlier duration
lint; initial receipts remain retained. Required tool absence still yields the
original NotFound failure rather than optional source evidence. This is focused
Linux evidence, not broad workspace or native consumer acceptance.

IC Build 0.2.1's exact release CI passes Linux and both native macOS hosts;
the newer 0.2.2 Intel CI fails an inadmissible invalid-byte filename in its new
test fixture. IC Build #15 owns the local repair and exact native acceptance;
Apple Silicon remains queued. Complete Canic native/application acceptance remains
outstanding. The wider Auth/Backup/Blob integration batch stays open under 0.112.0.
No package version, staging, commit, push, release, publication or broad gate ran.

## Backup lock adoption and pending Auth/Build APIs — 2026-10-13

Canic Backup now uses the published IC Backup 0.24 journal lock/error directly.
The redundant wrapper and generic lock tests are removed; upstream retains
contention, unsafe-entry and process-death coverage. Canic's restore-reference
retention and completed-restore lost-response replay cases pass, with strict
package library/test Clippy. Evidence: `/tmp/canic-backup-shared-lock-*.log`.
This prepares guarded reference-release adoption but does not complete #513 or
enable live backup creation (#394). Product journal and command custody remain.

The selected Auth 0.5.10 and Build 0.2.1 lack the next shared APIs: scope/grant
validation (Auth #50) and standalone input evidence/depfile discovery (Build #12).
Their sibling checkouts contain uncommitted implementations for 0.5.11 and 0.2.2;
no sibling edits or local dependency overrides were made. Adopt published APIs
before retiring the corresponding Canic implementations. The release batch stays
open; package versions and Git refs are unchanged.

## Native-agent Memory diagnostics repair — 2026-10-13

The native-agent integration fixture no longer accesses Memory 0.36.2's private
commit store. It reopens the existing memory geometry and reads recovered ledger
bindings from the public doctor report, without bootstrapping or treating its
empty diagnostic inputs as admission authority. All four fixture lookups use the
raw stable-memory snapshot. The focused byte-preservation regression passes;
strict Clippy for the native-agent target passes. The initial regression used an
invalid key without `.v1`; that test input is corrected and its failure retained.
Evidence: `/tmp/canic-native-agent-memory-clippy.log` and
`/tmp/canic-native-agent-memory-test-final.log`. No broad or PocketIC suite ran.
This compile blocker is cleared; the complete integration batch remains open.

## Published Build 0.2 and Shared Tooling 0.10 adoption — 2026-10-13

The real Canic graph now selects registry IC Build 0.2.0, clearing the Host 0.17
`OutputLimits` mismatch. Strict Host library/test Clippy with all features passes
against the published package; no source override is present. The root requirement
was already 0.2 when this adoption began; the scoped lock update selected 0.2.0
and removed the obsolete Host process 0.16 entry. Other incoming updates remain.

Shared Tooling 0.10.0 is adopted at
`ed9c26f015691859f67b50b512a94709dcdc0c6c`, with 111 verified payloads.
Common host/IC/Rust versions and digests match the previous catalogs. Remove
redundant Canic Make defaults and use snapshot-owned catalogs; retain product
compiler and Testkit selections. Explicit tool setup and all offline tool checks
pass, including the selected Testkit runner. Snapshot governance/link checks,
dependency pins, formatting and both local tool/formatter adapter fixtures pass.
The tool fixture now expects the snapshot's absolute catalog paths; its original
failure is retained. The newly referenced reporting rule is an explicit entrypoint.

Evidence: `/tmp/canic-build020-host-clippy.log` and `/tmp/canic-shared010-*`.
These are focused Linux checks, not the full workspace or native macOS gate.
The reported compile blocker is resolved; the wider integration batch remains
open. The existing 0.112 draft reflects these selections; package versions and
Git refs were not changed by this task. Earlier blocked evidence below is historical.

## Validation fallout after Host 0.17 selection — 2026-10-13

The reported validation exposed missed bench/app paths and the newly selected
Host 0.17 feature split. Host now enables `candid-service`; the lock adds its
`candid_parser` edge without changing versions. Both disposable Hub/Shard apps
provide explicit Memory composition for their rows and fixture-import cells,
using the same named keys for requests and opens. The authorization probe now
declares its existing `probe:read` scope in its component token allowlist.
Remove the upstream crypto microbenchmark, `CanisterSignatureCbor`,
`root_canister_sig_verification_message`, `self_describing_cbor` and its private
domain constant; retain Canic token serialization benchmarks.

Warning-denied Clippy passes for the serialization bench with all Core features
and for the three repaired fixture libraries. The initial fixture visibility
lint was corrected; evidence is retained in `.tmp/memory-036/` under
`serialize-clippy.log` and `fixtures-fallout-clippy-final.log`. No managed runtime
proof was rerun for these fixture repairs, and no broad suite ran.

Before the published adoption above, the focused Host check reached
`canister_build/batch/mod.rs` and fails because IC Build 0.1.3 accepts Host 0.16
`OutputLimits` while Canic selects Host 0.17. Published Build 0.1.4 was also
inspected after the repeated validation failure: its registry manifest and public
grouping API still use Host 0.16, so updating to that patch cannot clear this
error. The maintainer subsequently authorized the sibling fix: IC Build now
selects Host 0.17, with a breaking 0.2.0 draft and unchanged 0.1.4 package
metadata. Its Linux delivery checks pass, including the corrected release
fixture. Canic Host compilation and strict all-feature library/test Clippy pass
against a frozen producer in `/tmp/canic-host017-proof.oEVphD`; the source
override exists only in that disposable copy. See `/tmp/ic-build-host017-*`
and `/tmp/canic-host017-consumer-*.log` for evidence. The published adoption above
supersedes this dependency blocker without relabeling the earlier proof.
Earlier Host 0.16 and Blob managed proofs below retain their exact graph scope.

## Artifact staging owner convergence — 2026-10-13

The authorized [#529](https://github.com/dragginzgame/canic/issues/529) replacement
is applied. Configured-role and infrastructure producers capture completed Cargo
outputs with registry IC Build 0.1.3; temporary allocation and cleanup use its
shared staging owner. Canic keeps product paths, transform/tool selections,
source/output admission policy, release binding, qualification and durable
publication under its build lock. Publication still replaces gzip, Candid and
Wasm in that order after qualifying the complete candidate set. Source capture
retains the existing unbounded read policy; staging is not release/cache admission.
Role-facing size diagnostics recognize the current temporary owner.

The published archive checksum is
`1527d2dfe308fc7c1236b6db42c8c6883635f0d60d1dbbabd28462648058caec`.
The qualified graph selects Host 0.16.0, Memory 0.36.1, Testkit 0.36.0,
Blob/contracts 0.23.0 and Timers 0.21.4. Its lockfile SHA-256 is
`62547e79b68f73a3e6eb3e83b7b92fdea4e58f8ad8ed4ee8fba2a5f4fed9bf94`.
Concurrent catalog updates were preserved; this migration did not rewrite them.

A frozen current-graph checkout at `/tmp/canic-529-registry-qualification/`
uses the registry package without an override and a separate target. All 45
focused checks pass: 23 artifact tests, two actual Cargo-output tests, three
release-binding tests and 17 declaration/extraction tests, including the real
extractor 0.1.6 case. The governed PocketIC binding case remains ignored.
Warning-denied Host all-target/all-feature Clippy and changed-file formatting
pass. Final source custody matches 1,592 owning-package Rust/manifest/config
inputs to the live checkout. Receipts are `/tmp/canic-529-registry-*.log`;
the snapshot and final custody log retain their exact inputs. The earlier
uncached-Timers refusal and its scratch remain retained.

Three production adapters shrink from 1,506 to 1,481 code lines (Cloc 2.10,
comments/blanks excluded). There is one staging/capture engine and no parallel
compatibility flow. The open 0.112 draft covers this change; own versions remain
0.111.0 and Git delivery is user-owned. Native macOS and application acceptance
remain on #529/#528; these scoped proofs do not close the wider integration batch.

## Funding decoder and explicit Memory adoption — 2026-10-13

The local #436 repair decodes the canonical Registry endpoint result once through
the shared query client. All three funding tests pass on published IC Build 0.1.3,
Testkit 0.36.0, Host 0.16 and Memory 0.36.1, including the actual query path,
canonical request decoding and typed remote rejection. This is native codec
evidence, not live funding acceptance. Build's required minimum is now 0.1.3;
the earlier Host public-type mismatch is cleared. The internal testing library
also checks successfully against Testkit 0.36.0. Locked source preparation ran
separately; no dependency downgrade or sibling override was introduced.

The local #527 implementation replaces global memory declarations with explicit
Core/Control Plane requests composed by the lifecycle owner. Store opens and
request factories share unchanged named keys and role/feature selection. Remove
`ic_memory_key!`, `memory_bootstrap_admission!`, the no-argument `MemoryRuntimeApi`
and declaration/pool/admission registration. The pool macro requires a named
pure provider; applications pass their composition to `start!`. Independent
ingress constructors now have a direct Canic-owned `ctor` dependency.

Earlier focused proofs on Memory 0.36.0 pass: 28 Core memory cases, the Control
Plane lazy-store allocation case, two composition cases, strict Core/Control
Plane library/test Clippy and Root/Coordinator/Wasm Store feature checks.
On the latest Memory 0.36.1 graph, both composition tests and all three metrics
facade tests also pass.

Published Blob/contracts 0.23.0 now clear the former Memory-type blocker. Root
and both isolated consumer locks select one Memory 0.36.1 each, without a source
override. Warning-denied adapter Clippy passes. The selected published Testkit
0.36 runner is prepared, and both focused managed Blob cases pass: dedicated
permissions/recovery and embedded application-state retention/recovery. Their
checks include exact installation decoding, endpoint parity and restore fencing.
Root selects Auth 0.5.8 and Timers 0.21.4 for these managed checks; consumer locks
retain their own compatible published selections. The root manifest and all
three locks stayed fixed during both runs.

Dedicated artifacts are retained at build
`39021f01483ac31d8be5ca364ca32605ce38d382c13ad0ee0ae7315d6a5069e8`;
embedded artifacts at
`b6bbcd1b93783b92f3f4325e00f416976495ce228c29f4e783e4551ea52bf6d0`.
Fast-profile code sections are 4.56 MiB (dedicated Blob), 4.57 MiB (embedded),
8.97 MiB (Root), 4.47 MiB (Coordinator) and 3.02 MiB (Wasm Store), all below
12 MiB. These are current fixture measurements, not a before/after reduction or
Toko release acceptance. The first runner took 424s including harness compilation;
the second reused those outputs and took 51s. Logs, input hashes and the former
failed check remain under `.tmp/memory-036/`.

The Blob/Memory dependency blocker is cleared and this scoped adoption is
qualified. The wider Auth/Backup release batch remains separately open; these
results alone do not qualify that whole batch as ready to push. The open 0.112
draft includes the changes; package versions remain 0.111.0. No broad gate,
commit, push, publication or sibling edit ran.

## Async instrumentation size reduction — 2026-10-13

The scoped local [#526](https://github.com/dragginzgame/canic/issues/526) change
shares async accounting across handler types, retaining safe pinned references,
per-poll context restoration and completed-handler destruction inside its scope.
Eleven Core accounting tests and warning-denied all-features library/test Clippy
pass. The exact held-HTTP overlap PocketIC case passes on both baseline and final
source, including both completion orders. Default-feature native tests remain
blocked by untouched Auth dead-code checks; this does not qualify that graph.

The runtime probe's `fast` code body falls from 3,349,506 to 3,338,038 bytes.
After identical `ic-wasm shrink` and Binaryen 133 `-Oz` processing it falls from
2,652,013 to 2,643,053 bytes (8,960 saved), with equal export names/kinds. These
thin-LTO probe measurements are not Toko release headroom. Source/artifact hashes,
commands and logs remain in `.tmp/perf-wasm-headroom/measurement.json` and adjacent
files. No dependencies, package versions, Git refs or sibling files changed.
The open 0.112 draft covers this slice; publication and exact Toko release
remeasurement remain outstanding. The wider integration batch is still open.

The follow-up [#531](https://github.com/dragginzgame/canic/issues/531) experiment
confirmed that baseline explicit access expressions forced async wrappers even for sync
handlers with local-only authentication. Six compiled query/update expansion
scenarios confirm the distinction. Separately making seven public or seven
guarded non-awaiting probe handlers sync saves 1,875 or 3,757 optimized code-body
bytes against the same baseline; the guarded wrappers remain async. Each variant
passes all ten owning timer/observability PocketIC tests. These are independent
`fast`-profile experiments, not a combined or Toko release measurement. Temporary
fixture and macro-test edits were restored byte-for-byte. Evidence and hashes
remain in `.tmp/endpoint-async-audit/measurement.json`; the #526 fix is unchanged.

## Declaration cache owner convergence — 2026-10-13

The maintainer authorized [#528](https://github.com/dragginzgame/canic/issues/528).
Configured-role and infrastructure extraction now use the existing IC Build
0.1.1 artifact transaction API through the same Canic recipe adapter. Generic
key/environment framing, manifests, verification, staging and retained output
ownership have one engine. The private JSON cache reader/writer is retired;
existing files remain untouched. The new store is beneath
`.canic/build-reuse/declarations/ic-build`, with independently materialized outputs
beside it. No dual reader, migration, implicit pruning or package-version change.

Canic retains native extractor admission and invocation custody, normalization,
its output budget, four-worker scheduling/order/draining, and optional-cache
fallback. Cached Candid reads/writes are capped at 4 MiB; larger valid extraction
results are returned without cache publication. Changed declarations or tools
remain fatal even when cache preparation/publication fails.

All 17 scoped extraction/cache tests pass on Linux, including the explicitly
selected real `candid-extractor` 0.1.6 cold/reuse/fresh-byte parity case. Strict
Host all-target/all-feature Clippy and changed-file formatting pass. Test receipts
are `/tmp/canic-528-candid-final.log` and `/tmp/canic-528-clippy-final.log`; earlier
focused receipts and the corrected Clippy finding are retained. These tiny-tool
fixtures do not establish a speed-up or application release acceptance. Root
manifest and lock bytes are unchanged. Source remains uncommitted; native macOS
Intel/Apple Silicon qualification remains on the issue. Other concurrent dirty
work is preserved, and the wider integration batch remains unfinished.

## Synchronous endpoint admission — 2026-10-13

The maintainer authorized implementing [#531](https://github.com/dragginzgame/canic/issues/531).
Builtin caller guards, role-attestation verification, composed access evaluation
and custom predicates are synchronous. The public custom contract is now
`AccessCheck`; remove `AsyncAccessPredicate`, its async-trait imports/attributes
and awaits on local helpers. There is no compatibility alias. Generated wrappers
follow handler synchrony; 25 non-awaiting framework handler definitions are now
sync. Proof issuance, provisioning and renewal keep actual external awaits.
Consuming applications still need to remove auth-only async from their handlers.

Validation passes: 51 endpoint macro tests, 28 Core access tests, two facade
tests, warning-denied Clippy for the four changed libraries, ten timer/authority
PocketIC cases, six payload/denial cases, the exact Registry role-attestation
case and the fresh-issuer token case including wrong-presenter rejection.
The token test had a stale `session` grant outside its configured scope ceiling;
it now requests `verify`, without widening production authorization.
Including Control Plane tests in Clippy still finds unrelated redundant clones
at `ops/component_provisioning/tests.rs:1923` and `:2024`.

Against the unchanged #526 probe baseline, raw code body falls from 3,338,038
to 3,318,649 bytes; identical shrink/Oz processing gives 2,643,053 to 2,630,180
bytes (12,873 saved). Export names/kinds, Cargo inputs and the #526 source hash
match. This is a fast-profile probe, not Toko release headroom. Evidence,
source/artifact hashes, original files and logs are retained under
`.tmp/auth-sync-implementation/`. The open 0.112 draft covers the hard cut;
this scoped change is validated, but the wider integration release batch remains
open. No package version, Git publication or sibling edit was performed.

## Published Testkit selection — current graph

The maintainer removed the Testkit sibling link. Root now selects published
Testkit 0.35.0, archive SHA-256
`3313fa4e49094266268ccc732512933d8f159946a7176cf7213a90f2cfa49228`.
The temporary path exception is removed, and the standard locked server selector
passes using the prepared published 0.35 runner. Root also selects Query 0.55.0,
Backup 0.21.1, Timers 0.21.1, Host 0.15.1, Metrics 0.8.1, Blob/contracts 0.22.14,
Memory 0.35.19, Auth/protocol-types 0.5.7 and IC Build 0.1.1.

Query's current catalog loader is asynchronous. Host calls its maintained name;
fixture callers now await it or drive it on their owning runtime. Ten catalog
regressions and four cache/timing tests pass, including cancellation, cache
preservation, assurance and use inside an existing runtime. Host local-Fleet
and internal-testing library checks, strict Host library/test Clippy, governed
test compilation, dependency inheritance/pinning and locked metadata pass.
Both standard published-runner managed cases pass: maximum-state authorization
uses 989,364 instructions (unchanged 1,000,000 ceiling), with bounded restoration,
cleanup and memory checks; two-target recovery/composed certification also passes.
The embedded preflight detected genuinely stale bytes. Canonical refresh and the
read-only verifier now pass; original artifact/evidence and failure remain retained.
Evidence and held inputs: `.tmp/testkit-registry-adoption-20261012/`.
Earlier source-path proofs below retain their historical graph identity; the
whole Auth/Backup release batch remains unfinished.

## Host 0.15 artifact-interface adoption

The maintainer authorized applying the prepared Host migration to Canic. Incoming
root Host 0.15 and Metrics 0.8 selections were preserved. Canic Host enables
`ic-interface`, uses the moved text API and inspects Candid methods without
building rich argument presentation trees. Borrowed canonical Wasm facts retain
name/kind and metadata checks around transforms; function renumbering remains
allowed. Artifact comparison now includes modes and maps oneway to update,
retaining Canic's internal-export policy and release/type authorities.

Linux checks pass: 24 artifact tests, 12 Candid/extraction/cache tests and strict
Host library/test Clippy. The existing real-extractor case remains intentionally
ignored. Locked metadata passes, all prior package identities/checksums and
Windows dependency selections remain, and the lock adds only the parser edge to
Host tools. Evidence is retained in `/tmp/host015-consumer-apply/canic/`.
[Canic #458](https://github.com/dragginzgame/canic/issues/458) and
[Host #71](https://github.com/dragginzgame/ic-host-tooling/issues/71) own acceptance.
Changes remain uncommitted; full gates and native macOS were not run. This scoped
result does not qualify the other ongoing integration work below.

## Blob exact installation decoding

The targeted [#524](https://github.com/dragginzgame/canic/issues/524) repair retains
zero CONFIGURATION_LIMITS skipping quota and unchanged ordinary endpoint limits.
Its two native dedicated/embedded regressions now pass after the old Auth blocker
cleared. The managed fixture retains exact original initialization bytes, enabling
installed cases to vary only nested application arguments under the real protected
envelope. This helper is host-side test support, not reset/recovery authority.

Both actual managed compositions pass in frozen temporary checkouts at
`/tmp/canic-524-isolated.v5md68w9/`, with a separate owning target and explicit
published Testkit 0.35 CLI / PocketIC 16.1 server. Source snapshots cover Canic and
its selected Testkit path dependency. The dedicated/embedded cases reject unknown
fields and extra arguments, preserve exact stable bytes/configuration and app
state, retain the required Blob fence, and recover through the existing operator
same-instance workflow. Existing permission/capacity and restoration journeys also
pass. Scoped denied-warning Canic/testing Clippy passes. Old attempts remain.

Dedicated build: `f665751a199eb49b43d11b0a38b7c59263ac44dc6d5900e27140a9f858d3f85b`.
Embedded build: `e0225bbcdcb30a50a24c3b28e246abde37fb7fbf3f25d25f2625acc763ccd752`.
The first installed assertion incorrectly expected an unfenced view after failed
reinstall; its original log/artifacts remain. The correction requires the fence,
then performs real recovery rather than directly clearing it.

Original root attempts remain in `.tmp/issue-524/recheck.*`: native successes,
stale fixture-lock refusal, interrupted check during owner build contention,
standard published-CLI admission refusal for source-path Testkit, and a later
16-error Timers 0.21 caller mismatch during that independent adoption. The timer
owner's repaired calls are frozen for the passing cases; fixture locks are aligned
only with the directly required published Blob/Auth/Timers APIs. The corrected
regression and locks are copied back only after original files match the frozen
baseline and its build directory is idle. The whole live Auth/Backup batch is not
qualified by this local proof. #524 remains open pending family delivery; no full
gate, source fallback, package version or Git release effect is introduced.

## Scoped metrics qualification repair

The Metrics maintainer explicitly authorized two tool-resolution assertion repairs,
embedded allocation-peer regeneration and the named held-HTTP PocketIC case.
Warning-denied Host Clippy and all four resolver tests pass on Linux. The canonical
refresh and verifier pass against root Metrics 0.7.3, Host 0.14.9, Memory 0.35.17
and Testkit 0.34.5. Exact Testkit/PocketIC setup and offline admission pass.
The case passes inventory/fixture preflight and starts PocketIC, then refuses the
required `canister_test` Wasm build because unfinished Auth integration leaves
`ISSUER_CANISTER_SIG_DOMAIN` and `IssuerProofPrepareFailed` unused under denied
dead-code warnings. No overlap/completion-order pass is claimed.
[#491](https://github.com/dragginzgame/canic/issues/491) owns that integration;
[#450](https://github.com/dragginzgame/canic/issues/450) retains native qualification,
and [#99](https://github.com/dragginzgame/canic/issues/99)/
[#447](https://github.com/dragginzgame/canic/issues/447) remain open.

Complete repair logs, exact selected inputs, original and regenerated fixture
identities, the missing-runner attempt and failed private scratch are retained in
`.tmp/metrics-qualification-20261012/` outside Cargo clean. Existing integration
work, own package versions and Git index/refs are preserved; no broad gate ran.

## Earlier qualified Blob graphs

Earlier Blob qualification held
root at Auth/protocol-types 0.5.3, Backup 0.20.1, Blob/contracts 0.22.11,
Memory 0.35.15, Timers 0.19.1, Metrics 0.7.2, Host 0.14.7, Query 0.54.13 and
Testkit 0.34.4 / PocketIC 16.1.0. Those selections are preserved. The root Blob
requirements now start at 0.22.11 because the adapter uses its delivered API.

- Cargo.toml SHA-256: `095be1d4f9b1f56d6cfbec6f0e60ec53971b8b8279ff733c524188819dbb9ac9`.
- Cargo.lock SHA-256: `c21bc626993d7845628c1e711d6fd05838ea0b4d22f464399217fa5cb3d39e41`.

Both private Blob locks independently select Auth 0.5.1, Blob/contracts 0.22.11,
Memory 0.35.14, Timers 0.19.0 and Metrics 0.7.1. They retain private package
versions 0.1.0; the adapter uses the main workspace version/catalog. Actual Blob
proofs below bind these fixture graphs and their retained artifacts, rather than
claiming the later root patches were installed in those canisters. Scoped final
Clippy compiles the newest root graph with warnings denied.

## Current integration

Auth's SDK session/backend adoption is implemented under
[#491](https://github.com/dragginzgame/canic/issues/491). The durable backend
stages a session/replay pair, checks all resource limits before mutation and
commits synchronously. Bounded stable vectors and derived indexes replace the
whole-store admission rewrite. The SDK owns replay/admission and live authority
checks; status/audit also recheck current Root policy. Core's generic canister
signature adapter delegates canonical DER, certificate freshness and cryptography
upstream, removing its direct signature-crate dependencies.

Current focused managed resource cases pass on Auth/protocol-types 0.5.7,
Timers 0.21, Host 0.15 and the explicit Testkit 0.35 source library with the
prepared published 0.35 server. Maximum state (2,048 sessions / 4,096 replays)
uses **989,121 authorization instructions**, below the unchanged 1,000,000
limit; one/median state uses 822,370 / 988,222. The maximum case also passes
bounded restoration/cleanup and no cleanup memory-growth checks. Previous
3.12M/2.66M/2.46M/1.17M failures remain historical measurements, not current
failures. Logs and held input hashes live in `.tmp/auth-session-20261012/`.
The current-graph two-target recovery/composed-certification case and real
Registry-bound Root-attestation verifier case pass (16.69s / 18.71s).
Eighty-eight selected native cases, warning-denied Core library/test Clippy and
default/session-enabled Wasm compilation also pass. The changed Timers probe's native API check passes. Direct fixture Wasm Cargo
compilation correctly refuses outside Canic's governed artifact builder; that
separate installed probe was not run. Actual SDK-facade single/combined denial acceptance
remains [#525](https://github.com/dragginzgame/canic/issues/525#issuecomment-6130059637), and publication /
downstream acceptance remain separate. Keep the whole integration batch open.

Auth now owns canonical request hashing, certificate issuance rules, enabled
SEC1/prehash checks and low-S signer normalization. Default request construction
requires no crypto feature. Canic retains configured scope/TTL diagnostics,
issuer/key policy, exact caller/operation custody and committed replay/session
transactions. Invalid raw Canic role projection precedes shared certificate
lifetime validation as a deliberate hard cut; the combined-invalid regression
observes zero proof callbacks. The certificate adapter contracts from 124 to 51
production lines. Token preparation and complete current-policy proof checks
already delegate upstream; the positive-proof cache is retired.
[Auth adoption](https://github.com/dragginzgame/ic-auth/issues/32#issuecomment-6120465762)
and [#491](https://github.com/dragginzgame/canic/issues/491) own remaining acceptance.

Timers 0.21 registration uses the typed initial start policy. Lifecycle one-shots
use atomic scheduled creation instead of separate scheduling/rollback. Retained
registrations remain inactive until their owner schedules. Metrics 0.8 owns the performance aggregate; totals
are `u128`; the public Rust PerfEntry field changes accordingly. Existing u64
views omit unrepresentable totals while retaining the exact boundary value.
No second memory/timer runtime or compatibility wrapper was added.

Backup owns checksums/staging, checked digest validation, journal locks and
private sidecar admission. Canic retains command/descendant custody, original
restore authority and terminal reference release. Root selects published 0.21.1;
the authorized sibling implementation now adds guarded reference release to the
existing 0.21.3 draft, with product terminal admission and exact existing command
custody. Six new native process/filesystem cases and strict Clippy pass on its
held latest graph. Qualification now covers every configured Backup gate target,
including native/PocketIC tests, docs, Rust 1.88 consumers and both archives.
Unchanged tooling/release/hook evidence was reused; all affected checks were rerun
after correcting directory-shaped generic lock admission. Exact Testkit 0.35.1
setup and the original failures remain retained. Publication and Canic
adoption remain separate under [Backup #29](https://github.com/dragginzgame/ic-backup/issues/29)
and [#513](https://github.com/dragginzgame/canic/issues/513);
[#394](https://github.com/dragginzgame/canic/issues/394) owns live Fleet backup.
Do not substitute unrestricted deletion or all-Applied accounting.

The current Canic runner now holds every command guard through reference
publication rather than dropping each before the write. Its existing retention,
lost-final-response replay and active-command regressions pass, together with
strict package library/test Clippy. Evidence remains in
`/tmp/ic-backup-guarded-release/`. This scoped correction does not adopt upstream
custody records or enable live creation; fresh Fleet routing, application
consistency and original paid-effect recovery remain required. The whole
integration batch remains unfinished; no versions or Git effects were performed.

Blob schemas, storage, policy, accounting and installation-memory grants already
belong upstream. The dedicated and embedded hosts retain one Canic memory and
lifecycle owner. Production Root-driven target-bound installation passes on the
delivered 0.22.11 dedicated graph, including controller/target refusals, restart,
lost-response recovery, conflicting rebinding and effect-free replay. The
complete four-artifact Fast build is
`f7aca7dd2fc40c90fcdc1a61d54eaa9c5824268e74551150f273669565830db5`;
the Root journey reuses its verified complete artifacts in 2.69s.
[Blob #61](https://github.com/dragginzgame/ic-blob-storage/issues/61)'s published
endpoint owner now replaces Canic's duplicate declaration block.
Browser/provider, completion verifier, GC, billing and actual application
acceptance remain separate under [#444](https://github.com/dragginzgame/canic/issues/444),
[Blob #31](https://github.com/dragginzgame/ic-blob-storage/issues/31),
[Blob #32](https://github.com/dragginzgame/ic-blob-storage/issues/32),
[Blob #33](https://github.com/dragginzgame/ic-blob-storage/issues/33),
[Blob #34](https://github.com/dragginzgame/ic-blob-storage/issues/34) and
[#494](https://github.com/dragginzgame/canic/issues/494). Framework proof does not
establish provider acceptance or authorize paid probes.

## Focused evidence

Current logs and held inputs: `target/review-validation/upstream-integration-20261012/`.
Session request/retry/conflict and endpoint policy, signer, certificate construction,
token verification/preparation, timer custody, wide totals and u64 projection cases
pass. Scoped Core/Blob warning-denied Clippy and enabled Wasm library checks pass.
Those initial native/Clippy proofs select Host 0.14.2; its later patch to 0.14.3
changed only the four Host selections. The managed and Backup runners were rebuilt
on 0.14.3. All 12 Backup artifact and three layout-custody cases pass on that latest
graph. The first managed timer/metrics attempt caught a missing watchdog initial
schedule argument; it was corrected after the owner finished. Its corrected rerun passes in 10.25s, rebuilding only the runtime probe in
4.62s while reusing both other fixtures. All 15 timer inventory cases pass.
The latest locked independent public client crosses Auth, Backup and Host
type boundaries successfully; the default Wasm library check passes and its
normal/build tree excludes Auth cryptography.
The first direct runtime-probe Wasm check refused the authoritative build-entrypoint
guard; actual canister proofs use the managed Host owner without bypassing it.
Failed/drifted attempts remain distinct; no full workspace or broad gate ran.

Both consumer shapes now have actual current-fixture-graph managed proofs.
The preceding detailed handoff and earlier target-based logs were removed by an
external Cargo clean; retained Wasm artifacts and failed scratch remain. New
completion evidence lives in `.tmp/blob-completion-20261012/` so another Cargo
clean does not erase it.

## Blob completion slice

The two formerly ignored composition journeys now belong to the governed
framework catalogue. A single isolated build worker uses the production CLI,
admits current-source complete cache reuse and supplies finalized manifest-bound
artifacts. Remove the adapter's testing-only dependency; it no longer carries
its own ignored PocketIC test runner. The dedicated, embedded and production
Root journeys pass in 18.05s, 68s and 15.42s. Permissions, two uploader grants,
lost tenant-mutation results, distinct capacity accounting, snapshot fencing,
same-release restoration and embedded counter retention are covered. Production
binding retains controller/target refusals and interruption/exact replay. Its
four-artifact cache reuse takes 2.77s. The final worker run totals 182.57s.

The initial parent-side cache recheck refused the test runner's compiler wrapper;
current-input admission remains in the correctly isolated build owner. Failed
scratch retains that attempt. The exposed timer instruction-total field now
carries `u128` after Timers 0.19; exact Candid/JSON wide-value roundtrip passes.
Existing u64 metric rows omit unrepresentable timer totals. No compatibility
reader or clipping was introduced.

The maintainer explicitly authorized [Blob #61](https://github.com/dragginzgame/ic-blob-storage/issues/61)
in `/home/adam/projects/ic-blob-storage`. Its opt-in contracts macro and standalone
adoption shipped in 0.22.11 at `ebb560f492b6b53925faae8890316fc66bfd6def`.
Canic consumes the actual registry packages through its root catalog and both
private locks, without a Blob source override. The adapter endpoint module is
113 lines rather than 376; the standalone module is 61. Canic retains endpoint
authentication, bounded decoding, actual caller context and the single lifecycle.

Published adoption evidence lives in `.tmp/blob-published-20261012/`. Dedicated,
embedded and production Root journeys pass in 23.80s, 2m03s and 15.12s; the
owning runner totals 177.26s. Both actual 32-method Blob Candid projections pass
strict structural comparison against the exact released standalone contract.
An initial runtime pass exposed missing Candid registrations from an imported
emitter alias. Its run was interrupted and retained; the final adapter uses a
lexical emitter. Both composition cases compare generated Candid against every
callable Wasm endpoint using the existing Host validator. Scoped warning-denied
Clippy passes on the newest root graph. The final source and catalog are
qualified; the earlier compile-only candidate was not delivery evidence. The
catalog and both fixture locks remained fixed throughout these checks. No broad
suite ran.
No Canic commit or publication was performed. Concurrent upstream commits belong
to their owner; this agent did not create them.

Provider/browser, verifier, GC, funding and application acceptance remain the
separate existing obligations. These framework proofs do not close those issues
or make the entire Auth/Backup/Blob release batch push-ready.

## Retained issue fixes and next work

The bounded config-reader fix for [#509](https://github.com/dragginzgame/canic/issues/509)
is applied locally. Host rejects oversized input before all three YAML consumers;
exact limit, missing file, UTF-8, followed/dangling symlink, directory and device
cases pass among all 23 scoped config tests. Host Clippy passes for all targets
and features with warnings denied. Its reported provisioning helper was shortened
without changing the assertions; its owning regression also passes. Both native
macOS jobs now run the config tests; their execution remains pending. Evidence
is retained in `.tmp/issue-509-20261012/`.

These latest Host checks select Auth/protocol-types 0.5.4, Backup 0.20.3,
Blob 0.22.12, Host 0.14.9, Memory 0.35.17, Metrics 0.7.3, Query 0.54.15,
Testkit 0.34.5 and Timers 0.19.3. The final check inputs stayed fixed. Earlier
Blob and Auth proofs retain their recorded graphs; this Host proof does not
relabel their installed artifacts or establish integration completion.

The macOS worker-fixture blocker in [#474](https://github.com/dragginzgame/canic/issues/474)
is corrected locally: the fixture deliberately creates redundant path separators,
then selects one physical path before passing scratch custody. Its previous
trailing-slash `TMPDIR` failure and corrected worker/workspace-runner fixture
passes are retained
under `target/review-validation/macos-scratch-20261012/`, using admitted Testkit
0.34.3 on Linux. Scoped [PR #519](https://github.com/dragginzgame/canic/pull/519)
contains only this fix and compatible draft notes against released main. Its
`baaf95619` source passes both fixtures with released Testkit 0.33.1; native
macOS results are pending in [CI](https://github.com/dragginzgame/canic/actions/runs/38233266774).

[#516](https://github.com/dragginzgame/canic/issues/516) retains the first concrete
provisioning dependency, bounded rejection evidence and exact approved resume
command. Permanent refusals suspend; transport failures back off. Its earlier
managed recovery/permanent-refusal proofs and aligned three-boundary client proof
retain their Auth 0.4.7 / Host 0.14.1 input identity. The installed Toko Root was
not modified, and these fixtures do not constitute application acceptance.
[#517](https://github.com/dragginzgame/canic/issues/517) requires rebuilding direct
consumers on the current exposed family without aliases.

Remaining Auth work includes qualifying the adopted shared signature store and
finishing the session engine's protected authority/transaction contract. Preserve exact
caller/operation records, original retrieval clocks and one certification root.
Complete Auth/Backup integration and their owning recovery proofs before the
requested cleanup slice; guarded reference release remains issue-owned. The
Canic-owned Blob endpoint/composition reduction is complete; application/provider
acceptance remains separate.
The whole integration/release batch is still open, so this slice alone is not a
push-readiness recommendation.

Shared Tooling 0.9.2 is adopted at
`c118062a070a2e238f1eb3989b6fd700f369ce1e`, with 110 verified payloads and
recorded entrypoints. Canic uses the public consumer Make include; paired
formatter commands retain all three workspaces and the custom hook. The unused
generic Cargo installer fixture is retired. Cargo-sort-derives 0.14.1 is prepared
and admitted offline. Focused adoption evidence lives in `.tmp/shared-tooling-091/`;
the directory retains the 0.9.1 preparation and subsequent 0.9.2 refresh separately.
The broader Auth/Backup integration batch remains unfinished. Shipping Rust floor is 1.91.
Sibling repositories remain read-only outside the explicit Blob #61 authority.
Preserve dirty work, archives, failed
scratch and `.canic/local-work/` / incident-repair material. No staging, commit,
version transaction, push, publication, deployment or paid effect ran for the
integration batch. The maintainer-authorized scoped PR above is isolated from it.

## IC Build registry adoption and audit

The maintainer authorized moving generic Testkit build/cache mechanics and
Canic feature-equivalent Cargo grouping to IC Build
([IC Build #1](https://github.com/dragginzgame/ic-build/issues/1)).
Canic now requires crates.io `ic-build 0.1.1`, archive SHA-256
`e6c3b1a1040d38c3f10e9f12fc456e7ba718ceaaa86dc4967ac719b5487e4aeb`,
from published source `dbb17982557174eed29e1a499a2b1a7bb278d5a7`.
The incoming lock already selected this release; raising the minimum requirement
left the lockfile and all unrelated selections unchanged.
The obsolete IC Build sibling-path exception was removed. This audit used the
then-selected Testkit incubation path; current shipping selection is published
as recorded above. No sibling sources were edited by this audit.
The engine preserves v1 cache/stamp/lock bytes and namespaces, returned artifact
lifetime, typed failures, input-change rejection and explicit maintenance.
PocketIC lifecycle stays in Testkit; Canic's complete-release admission, recipes,
credentials, generated inputs and release identities remain consumer policy.
The separate complete-release reuse/finalization path is not replaced by a
generic cache-hit predicate in this batch. IcyDB adoption and native macOS
qualification remain separately scoped. No package versions, refs, tags or
release artifacts are changed by the extraction.

The Host generated-infrastructure input-stability case and all six selected
internal artifact cases pass with registry IC Build 0.1.1 and Host 0.15.1.
Nine direct published-API cases pass for transactional reuse/retention, pruning,
changed-input and malformed-output refusals, nested cache exclusion, real Cargo
feature grouping and structured complete/truncated failure evidence. Duplicate
and empty package names refuse before command-factory effects; an empty overall
list remains an effect-free plan. The real Rust 1.99.0 Wasm fixture passes cold
build, exact reuse, source invalidation and custody of retained old bytes. These
fixtures do not qualify Canic canister deployment. The published artifact engine
is unchanged from 0.1.0; this patch changes grouping admission and diagnostics.

Current evidence lives in `.tmp/ic-build-011-adoption-20261012/`. The manifest
and lock stayed fixed through passing checks at SHA-256
`f291ddc878ced81eac46ddaa406c48409f266a650b3d22fdf2724975282eb343`
and `d1dd5b874b813ef4ce1c9dd17c0a7735b6db6b14b7cca74470860c78c8e10791`.
Dependency inheritance/pinning and scoped diff checks pass. A final optional
cache timing check was terminated while queued behind another build, before
compilation or test execution; it is not counted as passing evidence.

Earlier 0.1.0 evidence and failures remain in `.tmp/ic-build-audit-20261012/`.
The previous Timers 0.21 compilation blocker no longer appears on current source;
the passing 0.1.1 artifact cases supersede that blocked consumer qualification
without relabeling its old inputs. The completed adoption does not qualify the
whole unfinished Auth/Backup/Blob integration batch as ready to push.

Published and consumer-verified grouping findings
[IC Build #3](https://github.com/dragginzgame/ic-build/issues/3) and
[#4](https://github.com/dragginzgame/ic-build/issues/4), plus publication
documentation [#2](https://github.com/dragginzgame/ic-build/issues/2), are closed.
Native macOS qualification and production complete-release convergence remain
with [#1](https://github.com/dragginzgame/ic-build/issues/1); its published-source
[CI run](https://github.com/dragginzgame/ic-build/actions/runs/38288344214) was
queued when inspected. Complete-release admission remains Canic-owned. No broad
gate, PocketIC, commit, own package-version change or release ran.
