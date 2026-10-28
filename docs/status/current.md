# Current handoff — 2026-10-28

## Accepted work and authority

The maintainer explicitly requests releasing Canic 0.112.0 and publishing its
package family. This includes the canonical source release and its required
qualification, version preparation, commit/tag and atomic main/tag push, followed
by `make publish`. It does not select an application or authorize a live Fleet
operation. PocketIC performance work is deferred until after publication.

The reviewed repair batch and focused qualification corrections are committed.
Local `main` was fast-forwarded from `f768a5519edb35512912dd73fce45936155edc6e`
to `ec817f2e20f5002a88def1a522f7ffe3ac046ee9`; the latter remains available on
`repair/release-qualification`. Versions still read 0.111.0; 0.112.0 notes are open.
The accepted Blob/Wire hard cut remains governed by the
[closeout plan](../audits/working/0.112-push-readiness/plan.md) and
[migration contract](../design/0.112-standalone-blob-service-extraction/wire-hard-cut.md).

## Current release attempt

The first authorized `make release-minor` stopped before version mutation after
4m43.920s. The release-integrity gate's synthetic build-environment fixture
inherited the enclosing release's `RELEASE_SOURCE`, then compared that real SHA
with its substituted Git identity. No 0.112 release plan was saved, tag created,
push performed or package published. The release lock was released normally.
Logs: `target/session-release-0112/attempt.TWDfr4/release.log` and `release.exit`.

The build-environment, validation-lane and preflight fixtures now clear inherited
`RELEASE_SOURCE` before selecting synthetic identities. Production source fences
and explicit mismatch regressions remain unchanged. Both directly affected lane
fixtures pass under the actual source SHA; the full release-integrity gate passes
with the outer release variables. Scoped ShellCheck passes with repository
options and the referenced tool catalog. Evidence:
`target/session-release-0112/attempt.TWDfr4/fixture-recovery-final.log`.
The narrow correction and changelog updates require a local commit, then a fresh
canonical minor release. Do not use resume: this failed gate saved no attempt.

## Qualification and implementation evidence

Complete non-publishing `make release-verify` passed on clean `ec817f2e2` in
90m48.263s. Detailed evidence is retained in
`target/session-qualification/final-report.md`, `attempt-6.log`,
`attempt-6-selection.txt` and `attempt-6.exit`. Earlier failed attempts and their
focused recoveries remain alongside them. The previous detailed handoff is
preserved in Git at `ec817f2e2:docs/status/current.md`.

That complete pass includes the exact restored-Root inventory/allocation case
(2m36s), maximum application initialization (1m16s), Coordinator replay (1.85s),
mixed-topology recovery (11m46s) and source-bound activation reset (4m52s).
Internal PocketIC took 65m10s and public runtime PocketIC 9m14s, inside the total.
These passes do not explain historical activation-conflict causation.

Implemented owners: shared preparation/qualification groups, governed build
selection, selected generated-source admission, exact package/source boundaries,
and existing release recovery. Generated source lives in configuration-keyed
`target/canic-packages`; Cargo output defaults to `target/cargo`. Physical output
and source overlaps, including symlink aliases, reject. Root lookup canonicalizes
configuration aliases. Snapshot rejection tests retain typed transport rejection
and unchanged-state assertions. Initialization admits the existing maximum input
within bounded codec accounting without increasing application bytes or authority.

The supported non-publishing sequence is `make prepare-validation SCOPE=release`,
review/commit, then `make release-verify`. No manual exports are required.
Make selects Rust/Cargo 1.99.1, pinned sccache 0.19.0, the existing wrapper,
version-isolated runtime and incremental=0; supported explicit overrides remain
validated. Qualification forces offline Cargo. Release preflight prepares all
three locked graphs and exact tools. No source-only receipt bypass exists.
Shared Tooling latest-stable maintenance: [#319](https://github.com/dragginzgame/shared-tooling/issues/319).

## Boundaries and next action

Commit the focused fixture correction and run `make release-minor`; only after
success run `make publish`. Observe exact remote refs and registry versions.
Stop on failed gates or uncertain external outcomes and retain all artifacts.
Once version preparation creates a saved plan, use exact-version
`make release-resume VERSION=0.112.0` after inspecting its state and remote refs.
Never remove an occupied lock based only on restricted PID visibility.

Fresh hosted setup and native macOS remain unverified. Historical October 27
`ActivationResetAdoptionConflict` remains unexplained; logs are
`/tmp/canic-release-0112-attempt{4,5}.log`. Current passes do not disprove it.
Preserve controllers, bounded spending, cycle conservation, release fences and
same-operation recovery. Snapshot-owned Shared Tooling files remain unchanged.

`make release-minor` releases source; `make publish` publishes packages. CLI
installation, application build, Fleet deployment and live verification are
separate. No live deployment is verified. Publication provenance hardening,
broader consolidation and deployment orchestration remain deferred.

## Isolated persistent receiver contribution — 2026-10-28

The maintainer selected persistent receiver integration first under
[Canic #491](https://github.com/dragginzgame/canic/issues/491). This isolated topic
branch is based on released `5c236ab130c0d63892ef281b45bdc3fa353e012f`; the normal
Canic checkout and its pending work are preserved. Auth/protocol types are locked
to published 0.19.4. No package version or release operation is selected.

The internal receiver component retains the complete restored logout ledger,
separate budgets and clock fencing. All 40 focused session-storage tests, normal
native/Wasm receiver graphs and package/test Clippy pass. These are native storage
and compile proofs; no installed upgrade, protected Root enrollment or production
endpoint is qualified. The production facade remains closed. See the
[component contract](../features/authentication/README.md#persistent-receiver-component).
The owning issue retains enrollment, endpoint/lifecycle and installed acceptance.

## Parent PR CI qualification repairs

PR #701's first hosted run passed preflight, security, MSRV and main checks but
exposed three qualification gaps. The Auth lock change made the embedded peer
stale. The base also contains a macOS CLI fixture using an unavailable rustix
symbol and an ordinary CI job which never prepares the release metadata editor
that its release fixtures admit offline. The setup failure was a missing prepared
store, not permission to weaken ownership checks.

The CLI fixture now uses native POSIX mkfifo with mode 0600. The ordinary job
explicitly prepares and checks the existing pinned editor. The existing refresh
example regenerated the peer and structured evidence; its normal verifier and
corruption/source-change regression pass. The Linux FIFO case, all 17 release-flow
cases, CLI package/test Clippy, workflow lint and formatting pass. Native macOS
and a complete hosted rerun remain pending; no full local suite was run.
Evidence: the Auth workspace's `target/canic-integration-20261028/parent-ci-*.log`.
Owning acceptance remains with Canic #491/#515/#551/#563.

## Enrollment follow-up on the persistent receiver branch

The next isolated contribution adds the bounded snapshot codec, Root producer,
opaque direct-Root pull ticket and persistent monotonic receiver installer. It
reserves a Wire identity but mounts no Root or issuer-registration endpoint.
Source logic is qualified by 15 selected enrollment cases (nine new), two codec/
schema cases, 40 storage cases, 11 descriptor cases, 27 role-catalog cases, base/
receiver/Root-only native/Wasm checks and warnings-denied Core/Contracts Clippy.
Evidence is in the Auth workspace's `target/canic-integration-20261028/`.
These do not qualify actual IC response authentication or installed lifecycle.

The parent qualification repairs are retained in PR #701; its fresh hosted
rerun remains separate from local component evidence. The normal Canic checkout
remains untouched. Root endpoint mounting, registration and installed acceptance
remain in the owning issue.

The enrollment source also refreshed and verified its own embedded allocation
peer after rebasing onto the parent qualification repairs. Its Core/Contracts
Rust sources and lock selection are unchanged from the passing focused component
evidence; the generated peer/evidence bind the final capability catalog.
