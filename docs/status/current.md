# Current handoff — 2026-10-27

## Scope and authority

The maintainer now explicitly requests completing preparation, committing the
reviewed repair batch and running non-publishing qualification in this session,
with diagnosis and in-scope fixes on failure. This supersedes the earlier
no-commit/no-qualification boundary. Push, release, publication and deployment
remain unauthorized. Qualification must retain the clean-tree fence and must not
race source edits. Evidence for this attempt belongs to `target/session-qualification/`.

Canonical `make prepare-validation SCOPE=release` passed in 96.666s. The pinned
cargo-edit 0.13.12 replaced ambient 0.13.13; its release-profile build took 48.06s.
No lockfiles or package versions changed. Commit review found and corrected the
macOS workflow's missing `prepare-build-environment` call before native tool
builds. Its existing setup fixture now checks ordering and preparation failure
propagation; this remains simulated evidence, not a fresh hosted macOS pass.

The earlier final review of the preserved repair/composition batch completed. At
that point the maintainer allowed concrete fixes and one non-publishing attempt.
No staging, commits, version changes, tags, pushes, publication or deployment were
performed during that earlier review. Preserve the dirty tree and all retained evidence.
The repair batch starts from `f768a5519edb35512912dd73fce45936155edc6e`; versions remain 0.111.0,
with 0.112.0 notes open. The accepted Blob/Wire hard-cut scope remains in the
[closeout plan](../audits/working/0.112-push-readiness/plan.md).

## Implemented batch and final review

- Host shares build-environment normalization with internal artifact production.
  Unknown/compiler/offline inputs remain bound; credentials and owned execution
  plumbing are withheld from both child commands and input fingerprints.
- Configuration-keyed generated source remains in `target/canic-packages/`.
  Repository Cargo/Make output is `target/cargo/`; Host's unset-output default is
  `target/canic-wasm/`. Application fingerprints exclude unrelated generated source;
  selected source/locks remain actual inputs. Explicit output overrides remain
  effective. Both the namespace and selected package directory are checked for
  physical overlap, including nested symlink aliases, before materialization.
  `clean-wasm` owns generated-source cleanup; compiler cleanup uses Cargo selection.
- Both packaged-downstream proofs now discover archives through Cargo metadata's
  `target_directory`, including relative/absolute overrides and paths with spaces.
  Developer paths, package guards and active instructions agree. CI rust-cache has
  target caching disabled; its separate sccache, test-artifact and diagnostic paths
  remain intentional. No old output cache was migrated or removed.
- One preparation owner supplies release/preflight/security/checks prerequisites,
  including all three locked graphs, Wasm std, exact editor and consumer tools.
  Security and Wasm crypto checks no longer fetch dependencies. Direct checks need
  explicit preparation. The standalone test runner retains its own locked fetch.
- Make qualification groups share local/CI membership; intentional coverage and
  scheduling differences are explicit in policy. No required product checks were
  removed. Generic Shared Tooling producer fixtures stay upstream; snapshot,
  consumer wiring, identity, package-boundary and failure-barrier checks remain.
- Fresh qualification has no SHA-only receipt shortcut. Exact-attempt recovery
  stays with the release runner. Alternate staging reads the adapter's roster,
  including both private consumer locks. CI retains bounded diagnostics/timings.
- Snapshot prepare/resume is admitted under Snapshot, while Release still rejects.
  The installed test checks `CanisterError` plus `CanisterCalledTrap` directly for
  allocation refusal and verifies unchanged inventory. It does not unwrap an
  expected transport refusal or relax production fences. Its stale lint expectation
  was removed; the final rejection assertions have a focused helper.

## Evidence and qualification result

`target/final-review/` contains source manifests, tool
paths/versions/hashes, logs, and the prior detailed handoff. The exact admission
source hash is `7e10c534529c6742edcd4d389cf4847b0cc1c5d3470d3ce84baa39f5243e2b67`
(`admission-source.json` in that directory); `final-source.json`
records subsequent documentation-only handoff changes. These are diagnostic records,
not release receipts or permission to reuse test results for changed code.

| Targeted check on reviewed code | Result |
| --- | --- |
| Offline Host `cargo test --locked -p canic-host --lib generated_` via scratch wrapper | 19 passed, 6.84s tests; includes actual Make input admission, selected-source invalidation, config isolation and nested alias refusal |
| Package output-path fixture, real Cargo on tiny local packages | Passed for both proofs: default, relative and absolute outputs; no publication |
| Wasm crypto fixture + real offline locked-graph check | Passed; missing-cache failure propagates; 12 canonical roles admitted |
| Make composition fixture and scoped shell lint | Passed |
| Host library Clippy + internal-testing Clippy with governed PocketIC tests | Passed; 7.14s and 1.89s respectively |
| Shared Tooling integrity | 139 owned files unchanged, snapshot 0.26.23 |
| Exact snapshot-restore PocketIC case, final reviewed code | **1 passed**, 35.56s test; runner 109s = 60s native preflight + 37s PocketIC stage + 12s other overhead |

Exact case command:
`CARGO_NET_OFFLINE=true make test-pocketic-case CASE=pic::fleet_registry::baseline::tests::restored_root_preserves_its_inventory_but_cannot_allocate`.
The final log is `pocketic-reviewed.log` in the evidence directory. Installed checks
cover prepare/live resume/restore, exact inventory, controller rejection, timer
fencing, restored resume refusal and allocation refusal. Earlier artifact-overlap
and helper failures remain historical evidence; they are not current failures.

The single complete attempt was `CARGO_NET_OFFLINE=true make release-verify`.
Inspection confirmed it calls the canonical owner with `complete` and no bump
argument. It **stopped at `ensure-clean`**, exit 2, in 0.06s. No qualification stage
or tool-admission stage ran. No clean-tree bypass or second complete run occurred.
The dirty tree is the specific admission blocker. Full qualification has not passed.

The earlier runtime tests used Rust/Cargo 1.99.1, `target/cargo`, offline Cargo and
ambient sccache 0.18.0. The intermediate preparation review installed 0.17.0;
that was historical evidence, not the final selection. The maintainer subsequently
approved latest stable 0.19.0. Its upstream archive digest and Linux wrapper/server
compatibility passed focused checks. Make now selects the managed executable,
version-isolated runtime, wrapper and incremental defaults itself. The existing
preparation owner installs/admit these selections; qualification checks the actual
selected binary, rejecting mismatched explicit overrides. The ambient binary and
global servers remain untouched. Shared Tooling latest-stable maintenance policy
is tracked in [#319](https://github.com/dragginzgame/shared-tooling/issues/319).
Native macOS remains untested; simulated cold-host selection covers the existing
Rust installer's prerequisite ordering and separate invocation environment.
Metadata editor 0.68.1 and locked Testkit selection checks previously passed.

Preparation evidence in `target/preparation-review/` records 507 external identities
in the three fetched locks and zero extra identities in 12 existing generated
graphs. With only those archives and existing registry index metadata, all three
workspace graphs (all features), all 12 generated graphs, both nested role-metadata
regressions and real generated-input admission passed offline. No ambient extracted
source, credentials or Cargo configuration were copied. This is current-graph
evidence, not fresh hosted setup or a promise about future fixture dependencies.

The supported non-publishing sequence is
`make prepare-validation SCOPE=release`, maintainer review
and commit, then `make release-verify`. The Make verification target forces Cargo
offline itself; the underlying lane script and direct `make validate` do not.
Preparation honors an explicit offline setting and cannot fill missing caches
with it enabled. Cargo offline does not prohibit advisory-database network access.
No full preparation or qualification was run in that earlier follow-up.

## Remaining boundaries and next action

The correction batch is ready for commit review, **not yet release-qualified**.
The newly authorized session will commit the reviewed batch before selecting
`make release-verify` again; prerequisite preparation has passed and no manual
environment exports are needed. Do not substitute direct
`make validate` to evade the source fence. Only non-publishing qualification and justified in-scope recovery are authorized.
Fresh hosted-runner setup remains unverified. Historical October 27
`ActivationResetAdoptionConflict` failures in attempt four remain unexplained;
selected passes do not disprove them. Preserve authority checks and diagnose recurrence.
The prior snapshot-resume E25 failure in attempt five now has native and installed
passing evidence. Historical logs remain `/tmp/canic-release-0112-attempt{4,5}.log`.

Host-wide checks found no competing Canic build. The release lock is absent; only
historical 0.110/0.111 recovery records were previously observed. Nothing was removed.
`make release-minor` releases source; `make publish` publishes packages. CLI install,
application build, Fleet deployment and live verification are separate. No live
deployment is verified. Publication provenance hardening, broader script consolidation
and deployment orchestration remain deferred. Preserve controllers, bounded spending,
cycle conservation, release fences and same-operation interruption recovery.

## Environment ownership follow-up evidence

| Command/check | Tested state and result | Remaining uncertainty |
| --- | --- | --- |
| `make prepare-build-environment`; `make check-build-environment` | Current 0.19.0 pin, real Linux checksum installer and runtime admission passed | Not full prerequisite preparation |
| `bash scripts/ci/test-build-environment.sh` | Current Make/preparation/lane owners: separate calls, defaults, explicit overrides, wrong-version and symlink refusal passed with substitute tools/gates | Simulated macOS, no hosted proof |
| `bash scripts/ci/test-pinned-ci-tools.sh`; `bash scripts/ci/test-sccache-wrapper.sh` | Pin/argv/failure propagation and compiler fallback contract passed | No full workload performance claim |
| Release lane, preflight and qualification-group focused shell fixtures | Preserved source fences, ordering, offline selection and shared gate membership passed | Full qualification remains unrun after the earlier dirty-tree refusal |
| Development/release Make recipe fixtures; focused ShellCheck; CI actionlint | Current adapters and changed automation passed | Substituted installers/gates, not hosted execution |
| Make-selected sccache 0.19.0, two real tiny Rust library builds | One compilation, one cache miss, one cache hit; byte-identical artifacts, no compiler/cache write failures; isolated server stopped | Linux compiler-cache compatibility only, not workspace qualification or a measured workspace speedup |

Upstream 0.19.0 isolated server/start/stats/stop, Rust 1.99.1 version query,
actual initial-connection failure and wrapper fallback evidence is retained in
`/tmp/canic-sccache-019-review/`. These passes do not resolve fresh hosted setup
or the historical activation conflict. No full qualification, release, publication
or deployment ran during that earlier follow-up.

The continuation's real compilation evidence is retained in
`target/environment-review/` (selected environment, cache statistics, probe and
server shutdown log). It exercised Make's default binary/wrapper/incremental
selection with explicit temporary cache/runtime isolation, without touching shared
Cargo outputs. No additional integration defect was found in this focused review.

## Authorized qualification attempt and recovery

The repair batch was committed as `ad66b647366b6e11f50ce537d74e64ecbd226256`
on local branch `repair/release-qualification`. Its clean-tree
`make release-verify` attempt stopped at Clippy after 108.756s: the generated-input
regression exceeded the 100-line function limit (144 lines). No later qualification
stage ran. Tool and dependency admission passed; Clippy's stage time was 61s.

Extracting fixture setup, Make output discovery and generated-directory cleanup
preserves all assertions. The final focused Host all-target/all-feature Clippy
passes (33.70s); the exact generated-input regression passes (1 test, 6.62s after
112s native compilation). Logs are in `target/session-qualification/`. The first
helper extraction still exceeded the limit and failed narrowly before the final
cleanup extraction; no complete retry occurred between those focused checks.
The corrected source must be committed and qualified afresh. No push or release
has occurred. Fresh hosted setup and the historical activation conflict remain
unresolved.

The second complete attempt, at `8e745713c`, stopped after 269.636s at the
release-tools fixture: it inherited Make's versioned runtime while expecting its
own temporary runtime. Formatting, shell lint, security and the other invariant
gates passed. Both copied-wrapper fixtures now clear inherited runtime selection;
the wrapper's cleanup assertion also refuses a missing directory instead of
accepting empty output from a failed find. Both exact fixtures with inherited
runtime selection, scoped ShellCheck, and the entire affected
`CARGO_NET_OFFLINE=true make release-integrity-contract-gate` passed. That gate's
publication checks use substitutes and had no registry effects. Evidence remains
in `target/session-qualification/`; no complete retry was used to diagnose this fix.

The third complete attempt at `d8a53b6a0` passed every pre-test stage and stopped
after 665.245s on one ordinary Host fixture. It supplied Make's CARGO_TARGET_DIR
as a command override as well as a spec-owned output; strict input admission
correctly rejected the duplicate ownership. The fixture now filters that command
override while retaining the spec output and every contamination/invalidation
assertion. Its exact Make-environment regression passes (0.09s after 49.33s build),
and Host all-target/all-feature Clippy passes (17.83s). Production admission is
unchanged. Other ordinary/internal/local-Fleet/doc groups completed, but the failed
ordinary barrier correctly prevented all serial PocketIC execution in that attempt.
