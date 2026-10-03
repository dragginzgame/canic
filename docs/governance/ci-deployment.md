# CI and Deployment Governance

This document is the authoritative workflow policy for commands, git,
versioning, releases, and deployment-adjacent automation.

## Commands

- Format: `cargo fmt --all`
- Check: `make check`
- Lint: `make clippy`
- Test: `make test`
- Build: `make build`
- Repository invariants: `make check-invariants`
- Shell automation lint: `make shellcheck`
- Complete local validation: `make validate`
- Eligible non-runtime patch validation and bump: `make patch-fast`
- Release-cadence advisory: `make release-cadence`

Primitive targets perform only the operation they name. They do not configure
Git hooks, format before checking, or invoke unrelated invariant, feature,
lint, build, or test targets. `make validate` is the explicit composition
boundary for the complete local workflow.

`make validate` has four sequential barriers. The first runs every independent
formatting, repository-invariant, dependency and shell check, then
reports their complete failure set. Workspace checking and Clippy start only
when that barrier passes. The control-plane feature matrix starts only after
compile and warning-denied Clippy pass; the complete test graph starts only
after that feature matrix passes. A deterministic compiler or warning failure
therefore never starts the feature matrix or leaves PocketIC running. Targets
within each barrier remain sequential so independent Cargo processes do not
contend for the same build graph. Complete failed-target logs are retained under
`target/validation-failures/`; the terminal summary repeats bounded failure
detail and the exact failed target list.

Every validation target also retains its raw output and a `timings.tsv` record
under a unique `target/validation-runs/` directory, including successful runs.
Records distinguish target, result, elapsed seconds and log path. The log path
is printed before work begins, so interrupted output remains discoverable;
an interrupted target may have a partial log without a completed timing row.
Nested validation barriers have separate directories. These logs are diagnostic
evidence, not reusable release-validation receipts.

## Command Authority

An unambiguous maintainer instruction in the current conversation authorizes
the exact Git, version, release, publication or deployment action it names.
Natural language is enough; automation must not require a magic phrase, a
second confirmation or a hand-executed command. The active repository,
version, Fleet and environment context may resolve the target when only one is
possible. Ask once when the target or external effect remains genuinely
ambiguous. Generic continuation, readiness and audit requests do not authorize
external effects.

Before an authorized network effect, retain the checks that prevent actual
damage: exact network and identity, exact reviewed plan digest, maximum debit,
cycle conservation and duplicate-effect protection. If those facts still
match the reviewed plan, do not add another ceremony gate. A changed digest,
environment, identity, debit bound or destructive disposition requires a new
plan or maintainer decision.

Release-blocking guards validate machine-relevant facts, not editorial prose.
Exact checks are appropriate for structured records, identifiers, versions,
digests, schemas, executable command ownership and required file/link
presence. They must not freeze explanatory sentences, line wrapping,
illustrative values, a full heading inventory or ordinary readiness narrative.
If a documentation fact must drive automation, represent it as a dedicated
machine-readable field. Documentation gates remain lightweight and must not
turn wording cleanup into a failed compile/test release cycle.

Release preflight checks authority records and exact tool pins. Native tests
parse CI YAML and maintained Rust production syntax; executable owner tests
check release sequencing, source-bound validation, checksums, redaction and
cleanup. Equivalent shell implementations, YAML layouts, comments and negative
test inputs do not require synchronized source-text guards. Audit-method
fingerprints remain available through the explicit `make audit-method-catalog-gate`
lane, outside deployment and release validation, so editorial audit changes do
not block shipping.

Ordinary parallel Rust suites retain libtest's default output capture. Passing
tests therefore do not print expected panic hooks or fixture chatter as live
validation errors; Cargo emits captured output for a failed test. Long-running
governed PocketIC suites keep live progress output. The workspace test runner
retains complete command output under `target/test-runs/` and prints each log
path before execution. Request, observation, artifact-cache and structured timing
traces stay out of live output; a failed command prints the last 100 trace lines. Complete
logs survive invocation-owned scratch cleanup, including partial logs from
interrupted runs. Expected rejected requests inside passing cases do not trigger
trace output. Document-layout preferences
and drift in transitive informational advisory inventories are warnings. Missing
required authority documents, known vulnerabilities, yanked dependencies and
unmaintained direct dependencies remain blocking.

The ordinary CI job also installs the internal Rust toolchain's
`wasm32-unknown-unknown` target and checksum-bound `ic-wasm`. Host build-cache
tests fingerprint that sysroot and compile small declaration fixtures; artifact
preflight tests resolve the actual Fast-profile toolchain. These prerequisites
do not start PocketIC or require the full deployment-tool installation.

Direct Cargo and Make-based work within this checkout share the repository
`target/`. Repository Cargo configuration also directs standalone audit fixtures
there by default. When `sccache` is available
and no explicit `RUSTC_WRAPPER` is set, Make selects it through the repository
wrapper and disables Rust incremental compilation so compiler results remain
cacheable. The wrapper gives the persistent cache server a stable
`.tmp/sccache-runtime/` socket and temporary directory; it never inherits an
invocation-owned `test-runtime.*` directory that cleanup removes. Cache infrastructure
errors fall back quietly to the original compiler command. Set
`CANIC_SCCACHE_VERBOSE=1` to print cache fallback diagnostics while investigating
cache availability; ordinary builds avoid repeating them for every crate. Genuine compiler
failures retain their diagnostics and exit codes without another compile attempt;
cache-management commands retain their own failure status. Without a
wrapper, Make leaves Cargo's profile defaults intact: local dev/test work may
remain incremental while `release` and `fast` artifacts stay non-incremental.
Explicit `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL` and `RUSTC_WRAPPER` values
remain authoritative. Canic artifact builds keep incremental compilation
disabled for deterministic Wasm output. Installed `canic build` leaves compiler
wrapper selection to Cargo and the operator's explicit configuration.
Direct `scripts/ci/run-with-test-scratch.sh` invocations also select the stable
repository cache wrapper when sccache is available and `RUSTC_WRAPPER` is unset.
They preserve explicit wrappers, including an empty value, and leave the selected
incremental-compilation setting intact. The cache server must outlive disposable
test scratch without retaining that scratch as its temporary directory.
Do not run a second Canic Cargo/check/test process against the same repository
`target/` during validation. Cargo will serialize parts of those graphs on its
build-directory lock while both processes still compete for CPU and memory;
changing source or `Cargo.lock` underneath the validating process can also turn
an otherwise quick immutability assertion into a late failure. Read-only plan
inspection remains safe while the owned validation finishes.

Native development builds and their inherited test profile use line-table debug
information. This preserves file/line backtraces while avoiding full type and
variable records in large native executables. Optimization, debug assertions and
overflow checks retain their existing settings. For debugger sessions requiring
local variables and parameters, set `CARGO_PROFILE_DEV_DEBUG=2` for development
builds or `CARGO_PROFILE_TEST_DEBUG=2` for tests. Changing debug mode rebuilds the
affected native artifacts. Fast and Release Wasm profiles remain unchanged;
downstream workspaces own their own Cargo profiles.

CI uses the same runner for its preflight, security and Rust-check jobs. Tool
installation and version verification remain immediate prerequisites, after
which each job reports every independent policy, security or compile-check
failure in one run. Every repeated failure-detail line carries its exact
`[ERR:<target>]` owner and is rendered red on an interactive terminal.
High-confidence compiler, panic, test and Make failure lines receive that same
decoration as they stream; decoration never changes the target result. The raw
command output remains in `latest.log`; the directly promptable, aggregated
failure excerpt is retained separately as `latest-errors.log`. Every expensive
compile and test job still requires both cheap gate jobs to pass. Each runner
invocation first syntax-checks and executes a private immutable copy of itself;
an unrelated edit to the workspace script during a long test cannot splice new
shell text into the process after the tests have completed.

CI cancels superseded pull-request runs. Main-branch runs have distinct concurrency
identities and retain their own results even when another commit arrives. Release
accounting must name the exact source revision for local validation, CI results
and package qualification; a later passing branch run is not evidence for an
earlier package. Publication does not wait for or manufacture CI evidence.

The repository owns one `pre-commit` hook, configured by `make install-dev` or
`make install-hooks`. It runs only `make fmt`; it does not run tests, Clippy,
builds, validation, versioning, commits, or pushes. A partially staged file
rejects before formatting because formatting the working copy cannot prove the
staged snapshot. After successful formatting, the hook refreshes the index only
for files that were already staged and tracked files that were clean before the
formatter changed them. It never stages pre-existing unstaged edits and rejects
if formatting changes such a file. Therefore `git add .` followed by
`git commit` commits the formatted snapshot without a second staging pass, while
unrelated unstaged content remains byte-for-byte unchanged. `make fmt-check`
remains in validation and CI so hook bypass does not weaken the release
boundary.

`make test` executes the release-lane integration tests recorded in the guarded
workspace test inventory. New integration targets must declare their release
lane, execution class and suite before the gate accepts them. External consumer
composition uses the explicit `integration` lane and is reported as unselected
by normal release/PocketIC runs. Run the IcyDB composition qualification with
`make test-pocketic-case CASE=icydb_lifecycle_composition` when its published
dependencies share Canic's memory runtime. This includes the IcyDB-backed
provisioning journeys within that target. It is a test consumer, not a deployed
Canic dependency. Its independent upstream dependency schedule never requires
alignment with Canic or blocks Canic upgrades, push readiness or publication.
Do not chase or wait for matching IcyDB releases, or change dependency versions
solely to align this optional local test consumer. Record unavailable composition
qualification separately and continue Canic-owned validation.
Default workspace build, check, Clippy and test commands exclude the optional
IcyDB schema/probe packages. The integration package's `external-composition`
feature gates its IcyDB dependency and test target; the explicit composition
command enables it. Clippy checks all targets/features in other maintained
packages and all default targets in this integration package. Full Canic-owned
validation does not require the optional consumer to compile.
The production Wasm dependency graph still requires exactly one memory runtime.
Complete and PocketIC-only runs first verify the checked-in embedded allocation
peer against its current producer inputs, before test suites or server startup.
Changed inputs use the existing build cache for byte qualification; stale bytes
fail immediately with the explicit refresh command. This check never rewrites
the fixture. Narrow targeted, ordinary and fast lanes do not inherit it; the
owning lifecycle proof retains its own verification.
Ordinary tests
retain libtest's default parallelism. PocketIC suites remain ordered, with two
isolated internal workers after the source-bound recovery barrier. Each worker
executes its own cases serially. Complete and PocketIC-only runs compile every selected serial
suite before starting the shared server. Preparation and execution use the same
package, feature and target selectors; a compilation failure stops before any
PocketIC case. The ordinary-test barrier still leads complete runs, and narrow
targeted, ordinary and fast lanes do not inherit the full serial preparation.
This moves compilation failures forward; it does not skip cases or claim a
shorter successful run. After every serial suite the runner reports the shared server's
current resident memory, resident high-water mark and thread count from the
release-supported Linux process boundary. `make test-wasm` is the fast lane and
runs only its classified release-surface integrations; it does not run workspace
unit/bin tests or PocketIC. The complete and ordinary lanes retain the unit/bin
coverage. `make test-runtime-fast` selects that same fast integration lane.
Fleet tests use synthetic, application-neutral topologies sized to exercise
the relevant contract. A complete generated journey owns fresh provisioning,
interruption and replay; focused recovery tests prepare their real canister
preconditions without repeating that entire journey. Application deployment
sizes belong in downstream qualification. Native cycle balances sampled across
IC calls or simulated-time advances must allow bounded execution and idle-storage
charges. Use an explicit observed-cost or reviewed test burn allowance, smaller
than the payment whose absence or duplication the test must detect. Keep grant
receipts, payment identities, policy limits and ledger accounting exact; replay
must not credit the recipient again. Exercise normal charging deliberately when
qualifying this allowance, rather than relying on incidental scheduler timing. Capacity arithmetic and rejection
boundaries remain covered independently of expensive deployment cardinality.
Retained-estate reinstall and completed-source reset each use two Workloads and
one Ready asset. Underfunding both Workloads must exhaust replacement capacity
and require an explicitly reviewed successor; exact import identities, controller
drift, lost replies, conservation and terminal replay remain real-canister proofs.
Native policy checks own the 19-Workload/five-Ready arithmetic boundary. Neither
reset journey is a deployment-scale qualification.
An explicitly selected CANIC-188 incident qualification exercises nine Workloads
and fifteen Ready assets through the same completed-source public CLI journey.
It is ignored by ordinary test selection and excluded from the governed release
catalogue. Native policy tests separately exercise mainnet call-cost bounds;
the incident-sized local journey does not establish live incident completion.
The mixed-topology journey owns the changed-build wipe, application-row reset,
interruption and replay proof. The small retained-estate journey owns a second
same-build reset with a distinct operation, lost identical-Wasm response,
conservation and replay. Native supplementary funding uses one real blocked
child claim with nonzero Ledger fees; its withdrawal and receipt-loss recovery
also cover the native funding pause without a separate synthetic-minimum estate.
The default gate excludes audit cohort-size experiments and arithmetic for their
external qualification budgets. Product Ledger/CMC replay, funding, import/reset
and controller-routing proofs remain required. Completed-source receipt and
manifest inspection uses the maintained schema and canonical hash owners;
fixtures must not rewrite current records into a predecessor format.
Release-test artifact identity separates native producer inputs from canister
inputs through the existing Cargo resolver and sealed-artifact cache. Standalone
host `cfg(test)` Rust modules may be omitted when they are not embedded producer
inputs; inline tests, production sources, Cargo manifests and locks remain bound.
Ambiguous source inclusion retains the complete host input set. The complete
producer snapshot, including excluded test files, must remain unchanged during
acquisition and publication. This reuses artifacts only, never test results.
The ordinary integration inventory joins workspace unit/lib/bin coverage in
one Cargo invocation, using explicit integration target names. This preserves
the workspace feature graph through the ordinary tier instead of recompiling
dependencies for a package-scoped integration pass. Integration target names
must not cross inventory selection classes. Pure internal fixture tests join
the same invocation. The internal library test binary excludes the
stateful Fleet catalogue unless its governed PocketIC feature is selected;
normal fixture-library consumers retain their configured fixture surface.
In a complete run, the later governed host PocketIC stage selects the same
workspace library/binary Cargo graph as ordinary tests, avoiding a second host
harness caused solely by package-scoped dependency feature resolution. Its
`governed_pocketic_` filter and ignored/serial selection run only the host proofs;
other harnesses select no tests. The ordinary-failure barrier and internal
catalogue ordering remain. PocketIC-only and exact-case runs retain the scoped
host graph because they have not built the ordinary workspace graph.
Timing output calls this
`libtest-parallel` to distinguish parallelism inside one Cargo invocation from
concurrent suite execution. When Make selects `sccache`, the runner reports
request/hit/miss, uncacheable-call and cache-error deltas, retains the server
through the complete two-hour test envelope and uses a 40 GiB local cache.
Missing or malformed statistics and a server reset are reported as unavailable.
The Rust checks, ordinary tests and PocketIC CI jobs install checksum-pinned
sccache and restore a separate 2 GiB disk cache per job. Cache saves also run
after test failures; cancellation skips them. Keys bind the platform, toolchain
and tool pins, with per-source snapshots and same-job fallback. sccache validates
individual compiler inputs after restore; the archive is never validation evidence.
Cargo target caching stays disabled in CI to bound disk use. Final PocketIC
artifacts retain their independent exact-input validation and cache.
Cargo continues across independently selected ordinary test binaries and records
their failures before returning one nonzero result. Serial PocketIC commands
retain failures across independent binaries and suites, preserving logs, timing
summaries and invocation cleanup. Compile errors and empty selections fail before
the full PocketIC tier starts. Failure excerpts include the assertion and case
identity after bounded trace context.
A failed ordinary tier is a hard barrier in the combined local
runner: it reports all ordinary failures and skips the serial PocketIC tier.
Plan-only inventory resolution still enumerates both tiers, and the explicit
PocketIC-only mode remains independently runnable. The governed internal harness
runs source-bound activation-reset recovery before starting two isolated workers.
One worker runs the short lifecycle contracts first, then Fleet deployment
restore, autonomous Root removal and the remaining regressions in catalogue
order, keeping their process-local baseline. The other runs the independent published managed-App and Component
Group lifecycle cases before the complete Fleet journeys. This balances the
retained timing baseline without adding processes or sharing mutable estates.
Partition membership derives from the registered cases. Native checks reconcile
compiled libtest identities with the registered journeys, native selectors and
explicit opt-in proofs. They require unique identities, nonempty selections,
per-worker catalogue order and the recovery prefix. Build and compiler-cache unit tests live in separate
modules so assertion-only edits do not invalidate the fixture artifact producer;
production inputs and concurrent-change checks remain complete.

Workers execute the parent's already compiled binary, with independent servers,
ports, native ICP shims and invocation-owned scratch. Only validated immutable
artifact caches and their locked build targets are shared. Each worker reports
case progress and its slowest cases. A case panic stops its process. An exact
completed-prefix report permits only the unexecuted suffix to run in a fresh
process, server and scratch. Passed and failed cases are never retried; any
failure keeps the overall result failed. Missing, malformed or inconsistent
reports stop that worker. Independent workers and later suites finish. The
source-bound recovery prerequisite remains a barrier for its dependent cases.
Completion and handled interruption stop owned process groups, join them and
clear their scratch. Exact single-case selection stays serial.
The measured two-case qualification uses identical warmed artifacts and retains
success, cancellation and interruption evidence. It does not establish the
elapsed time of a complete release gate.

The restore proof uses the process-local baseline; destructive Root removal uses
an exclusive fresh instance because deletion is outside snapshot reset. Ordinary
validation executes the feature-gated native runner/cache tests, Host local-Fleet
native tests and workspace doctests, including compile-fail public contracts.
Stateful cases stay under the serial PocketIC owner. The lane
clears transient heavy Wasm targets once before its integration-suite group and
once at invocation cleanup, retaining Cargo freshness between ordered suites.
The ignored instruction-audit target shares the runtime Cargo invocation for
compile coverage. Only its explicit audit runner executes the audit; release
validation does not report it as a separately completed PocketIC audit.
CI may run the ordinary and PocketIC lanes in separate jobs; it must not
add concurrency beyond the two internal workers without replacing this measured
policy. Cheap source/governance preflight and security jobs gate the Rust checks
job. That job runs formatting and warning-denied Clippy before the control-plane
feature matrix. Ordinary tests, PocketIC tests and the release-profile build all
depend on the completed checks job, so no expensive lane starts while a quick
compiler or lint failure is still discoverable.

The governed PocketIC runner resolves one repository-pinned server binary,
verifies its exact checksum even when `POCKET_IC_BIN` was supplied by the
caller, then starts one shared server in the invocation-owned private scratch
immediately before the serial PocketIC lane. The runner admits a numeric port
within 30 seconds, retains bounded stdout/stderr for startup failure and gives
the process a two-hour idle and hard lifetime. It retains the exact child PID,
stops and waits for it on every handled exit, and leaves invocation-scratch
cleanup as a crash-safety fallback bound to the numeric direct-child port path.
A failed suite prints bounded tails from both server streams next to its own
retained log. `ic-testkit` 0.9.0 owns the corrected bounded managed-server
primitive for one Rust process; Canic keeps a runner-owned server because the
serial lane still crosses the internal harness and several integration-test
processes. Repository fixtures use
testkit connect mode with their own 30-second instance-construction deadline.
Direct PocketIC test commands outside the governed runner must supply
`CANIC_POCKET_IC_SERVER_URL`; they fail immediately when it is absent rather
than spawning an implicit or unobservable child process.

Local governed tests retain content-addressed Wasm and sealed release-artifact
sets under `target/test-artifacts` and reuse the shared incremental Wasm target.
Each Local/Ic fixture compiler target has an 8 GiB whole-target cleanup threshold
and seven-day idle expiry, checked at most hourly under ic-testkit's build lock.
The observed Local target exceeds 4 GiB, so the former threshold could discard
useful compiler state. The new threshold leaves headroom; it reserves no disk
space and is not a hard cap during compilation. Clearances and maintenance
failures appear in normal output with the affected target; retained/skipped
maintenance remains verbose-only. Exact artifact verification is independent
of this mutable compiler cache policy.
The PocketIC lane selects an installed native ICP CLI matching the repository
pin before starting its server. `CANIC_TEST_ICP_BIN` explicitly selects that
executable; otherwise an already-native `icp` on PATH is retained. If a launcher
shadows it, the runner selects the native installation under
`ICP_CLI_INSTALL_DIR` or `${CARGO_HOME:-$HOME/.cargo}/bin`. A missing, non-native
or mismatched selected tool fails before Cargo. `make install-dev` installs the
pinned native CLI. Selection creates only a private scratch symlink and changes
PATH for the test invocation; other tools and ordinary/plan-only lanes retain
their existing resolution. Production transport and command version checks
remain intact.

Tests must keep plans, journals, identities and PocketIC state invocation-local;
only immutable build products whose source, configuration, toolchain and output
set are transactionally verified may cross invocations. Use `make clean-wasm`
only for deliberate cache/storage maintenance, not as a routine response to a
test failure. A focused PocketIC regression uses `make test-pocketic-case CASE=<selector>`.
The selector is an exact internal Rust test path or a classified `canic-tests`
integration target such as `native_agent_delegation`. This delegates to the
existing `targeted-pocketic` runner, with the pinned shared server, private
scratch, serial execution and reusable artifacts. An integration selector runs
that target's tests; an internal Rust path selects one exact test. The exact-path
selector verifies its registered test identity before execution and rejects a
zero-test match. An omitted
selector fails before creating scratch or starting Cargo. Use these focused
commands while fixing failures; `make patch` and `make release-patch` remain
complete release validation, not the default development feedback loop.

## Explicit Cargo Cleanup

Release and push targets retain Cargo artifacts. This lets a following
`make publish` reuse the exact build cache populated by validation instead of
performing a clean package compilation immediately after the release push.
Failed validation, version, stage, commit, tag, push or package publication
also retains those artifacts for diagnosis and retry.

The primitive `release-push` target remains limited to readiness verification
and the atomic network update; version-only and one-shot release targets do not
infer local cleanup. Cargo cleanup is an explicit storage-maintenance action:
use `make release-clean` for fail-soft cleanup after all intended package work,
or `make clean` when cleanup failure should be returned to the caller. Neither
command changes release authority or replays a network effect.

## Development Slices and Validation Tiers

A code slice is a small, focused implementation unit chosen for reviewability
and safety. It is not a release patch by default.

Release grouping, continuation and handoff readiness are governed by
[delivery cadence governance](delivery-cadence.md). A minor has no minimum
release count and planned design cadence should normally publish no more than
12 releases; necessary post-publication correctness, security, recovery and
operator-regression fixes may exceed that guideline. An implementation slice
is not automatically a release.

Default development cadence:

- Choose batch boundaries by complete outcomes rather than elapsed time.
- Keep individual code slices focused by concern, module, or invariant.
- Combine compatible implementation, direct evidence, propagation and cleanup
  slices into the current planned release batch and open patch draft.
- Keep routine compile, lint, fixture and documentation fallout in that batch;
  do not turn it into another patch release.
- Maintain the changelog by default when a meaningful code or behavior batch
  is complete. Reuse an existing untagged patch draft; otherwise prepare the
  next patch draft according to the [changelog policy](changelog.md).
- A changelog draft version is documentation planning, not a package-version
  bump. Release version files remain owned by the human release flow.

Validation is tiered:

- Automated coding work runs only the smallest targeted format, test, lint, or
  compile commands that exercise the touched code and relevant invariant.
- Before implementation closeout, account for every changed Rust target in the
  accepted batch, including unit tests, integration tests, examples and feature-
  gated fixtures. Linting a dependent package checks dependency libraries, not
  those dependencies' test targets; a passing test run does not establish Clippy
  coverage. Select each affected owning package/target explicitly and use the
  release lint feature selection and `-- -D warnings`. Group compatible affected
  packages in one invocation with `--keep-going` to collect independent failures;
  use `--all-targets --all-features` for those packages when their affected target
  set spans library and test surfaces. This remains package-scoped validation,
  not authorization for workspace-wide gates.
- For changed feature-gated imports or tests, also compile/run the affected
  target in package isolation with its ordinary runner feature selection and
  each directly affected role selection. `--all-features` and workspace feature
  unification can hide missing gates; they do not replace those narrow checks.
  Use the governed scratch wrapper when fixtures create Cargo consumers, so
  repository-local workspace discovery is covered before deployment validation.
- After a validation failure, inspect the complete retained failure log, correct
  all reported in-scope defects, and rerun the affected target set together.
  Closeout evidence records package, target and feature selection for the final
  source; do not infer lint coverage from dependency compilation or an earlier
  source snapshot.
- Human/CI batch validation may add wider package checks when cross-cutting
  behavior warrants them.
- The maintainer-directed deployment/version/release flow chooses whether the
  complete gate or the governed fast patch lane is appropriate.

For documentation-only governance changes, use docs-appropriate validation such
as formatting, whitespace, link-shape review, and `git diff --check`. Do not run
code test suites unless code files changed or the maintainer asks for them.

Release-line-specific validation matrices may further classify existing checks
for a bounded release line. Use
[docs/operations/release-validation-matrix.md](../operations/release-validation-matrix.md)
as the current matrix for slice close-out, implementation close-out, RC
promotion, and final release/tag validation. The matrix interprets this
governance policy for the active release line; it does not override the git,
versioning, or release boundaries in this document.

The sole supported host and Rust target authority is the
[supported host and target matrix](supported-platforms.md). Installer branches
outside a declared and validated cell do not create support claims.
macOS on Apple Silicon and Intel is required Host/CLI support. Its outstanding
native build and filesystem qualification must be tracked as support work;
Linux-only CI success does not establish macOS validation. Follow the matrix's
qualification requirements before claiming macOS checks have passed.

## Git Boundary

Automated agents may stage, commit, tag and push when the maintainer explicitly
requests those actions in the current conversation. The instruction need not
use prescribed wording and may authorize the normal sequential release command
as one action. Without that instruction, agents remain read-only for Git
publication and may inspect state with commands such as `git status`,
`git diff`, `git log`, and `git show`.

Do not rewrite history or tags. Do not revert user changes unless explicitly
requested.

## Versioning and Release

Version mutation, tagging, publication and push require an explicit maintainer
instruction, but an automated agent may execute them once instructed. A direct
request such as “publish 0.109.15” is sufficient authority for the normal
version, stage, commit, tag, push and package-publication sequence in the named
repository. Do not ask the maintainer to repeat it or run intermediate commands
by hand. Do not infer the same authority from “continue,” “finish,” “ready to
push,” an audit request or a request to prepare a candidate.

The normal complete patch path is `make patch`, followed by
`make release-stage`, `make release-commit`, and `make release-push`; the
one-shot form is `make release-patch`. Minor and major releases use their
corresponding commands.
Before patch validation and version mutation, `make patch` prints the
read-only `make release-cadence` advisory. The advisory reports when the next
release would exceed the soft 12-release minor-line guideline but never blocks
or expands the maintainer's release authority.
The complete and fast Make version targets delegate to one fail-fast release
validation owner. Before the gate or receipt reuse, it checks source cleanliness,
the planned release notes, freshly observed remote ancestry and availability of
the planned release tag. Diverged branches, occupied tags and failed remote
observations stop before compilation or PocketIC. It propagates any
nonzero validation or eligibility result immediately, checks cleanliness again,
and proves that `HEAD` is unchanged before granting version-mutation authority.
The complete lane runs the same explicit `make validate` workflow. It does not
mutate source formatting; the pre-commit hook handles routine formatting, while
validation's `make fmt-check` catches bypassed hooks. Any failed target leaves
the version unchanged. After a complete gate succeeds and the clean immutable
source revision is rechecked, the lane atomically retains a local exact-HEAD
success receipt under `target/release-validation/`. If versioning, remote
readiness or another later release-only step fails, rerunning the same complete
release command rechecks the cheap preflight and reuses that receipt instead of
rerunning compilation or PocketIC. A source commit change or missing receipt
requires validation again. An executable release-integrity regression proves
that a failed gate cannot invoke the bump script and that an exact successful
receipt resumes version mutation without a second validation. The underlying
bump script rejects direct invocation without the private validation marker
supplied by the owner.
The root `Cargo.toml` is the sole live workspace package-version authority;
ordinary status and planning prose must not act as a parallel package-version
source. Current and committed version queries must use the shared pinned
`cargo-get` reader; release scripts must not maintain parallel manifest
parsers. The governed bump is the one exception: after validating one exact
clean source commit, it seals an `Unreleased` detailed changelog entry with the
release date or preserves its existing valid ISO date, then writes one generated
`release-validation.json` containing schema `1`, exact release version, validated
source commit, release date and `complete` or `fast` gate. The transaction restores
both receipt contents and prior absence if any later bump step fails. Status and
planning documents are ordinary descriptive handoffs; release scripts do not
parse, mutate or stage them. The receipt records validation, not successful
tagging or package publication.
Immediately before changing version
files, the bump transaction fetches the current `origin` branch, requires it
to remain an ancestor of the validated local source, and requires the exact
planned release tag to be absent remotely. The release commit may then
differ from the validated source only in the enumerated version, lock,
installer, changelog and structured receipt surfaces. The cheap current-document
semantics gate still rejects volatile
"latest published" and manual release-truth prose elsewhere. After staging,
`make release-commit` runs the fast
post-bump `make release-candidate` guard before committing or tagging. That
guard verifies the sealed changelog, rejects non-release changes in the
release transaction, and checks locked offline Cargo
metadata, uniform workspace package versions and the installed-CLI default
without repeating the already completed full source validation. Current status
text is informational and never gates candidate admission or package publication.
The release lane owns validation; the publication guard does not infer validation
from an editable handoff.

### Fast non-runtime patch lane

`make patch-fast` is a governed alternative only when the current workspace
version has an exact immutable published tag and every attributable change
after that tag is confined to documentation/governance or the
release tooling that owns this lane. Runtime, build, package, protocol,
generated, Candid, configuration and product-fixture changes reject before
version mutation. `make release-patch-fast` performs the same eligible gate and
then uses the normal stage, candidate, commit, tag and atomic push path.

The fast lane verifies the immutable tag's structured validation receipt,
requires that receipt or its fast-release chain to retain a complete validated
release ancestor, and checks ancestry, diff hygiene, current-document and
release-matrix semantics. It runs
the release integrity and release-flow checks when tooling changed. Dependency
resolution changes require the complete lane; matching version prefixes do not
establish unchanged behavior. It deliberately skips workspace tests
and PocketIC. The structured receipt records `gate: "fast"`; it is not evidence
that `make validate` ran on that patch. The reader admits only schema-1 receipts
from the exact annotated tag, bound to its version and an ancestor source.
Tags without this receipt require the complete release lane. No prose/marker
fallback or automatic backfill of historical validation is supported.

Use the fast lane for a documentation/governance correction or release-tooling
correction whose production source and dependency resolution are unchanged.
Any ineligible path, missing receipt, non-ancestor tag or targeted failure
falls back to the complete path; there is no override flag.

When an accepted release batch declares an exact downstream pre-publication
qualification gate, freeze one clean source commit before running that gate.
Build the candidate executable from that commit and run the declared no-effect
preflight before version mutation or publication. A source change invalidates
the downstream result and requires a new candidate commit; release-only
version surfaces do not change the qualified source. Run focused checks while
editing, the one declared production-boundary journey before review, and the
complete `make validate` gate once through the normal maintainer-directed version target
after the source candidate is frozen. Publication may proceed only when the
downstream preflight and normal release gate both identify that unchanged
source candidate.

The test target allocates one private repository-owned
`.tmp/test-runtime.<suffix>` directory. It clears only that scratch on success,
ordinary failure or handled interrupt. Before removing it, cleanup forcibly
stops only a detached PocketIC server whose exact `--port-file` is a direct
child of that invocation's scratch; this avoids the upstream server's late
socket-teardown panic without touching another invocation's server. Cleanup
never sweeps a shared path or another concurrent invocation's scratch. Canic
scripts must clean their own temporary files; explicit cleanup must not sweep
unrelated repository scratch or global `/tmp` content.
Before its final atomic network update, `make release-push` verifies the exact
release commit/tag pair from committed `HEAD`, refreshes the current `origin`
branch, requires fast-forward ancestry and rejects any conflicting remote tag.
An idempotent retry may observe the exact same annotated tag object. It does
not format, compile, test, validate, or clean. A successful parent one-shot
release also retains the validated Cargo artifacts for subsequent packaging or
publication. Local
staged, unstaged and untracked changes neither block the push nor join it; they
remain local. The release version is read from `HEAD`'s committed `Cargo.toml`,
so a later local manifest edit cannot redirect tag selection. Test scratch has
already been removed by the test invocation that owned it. Release push
explicitly disables implicit followed-tag publication and sends both the
current branch ref and the exact workspace-version tag ref in one atomic push,
so the tag is still sent
when the branch commit is already present remotely. No fallible local cleanup
step can change or revoke a successful push, and atomic push prevents a branch-
only or tag-only remote update. A transport interruption can still make the
remote outcome uncertain and must be resolved by inspecting the remote refs
before retrying.

The historical-tag deletion helper removes remote refs before local refs and
verifies both requested boundaries. Deleted annotated tags remain present in
other clones until those clones remove them. A later `git push --tags` or
`git push --follow-tags` from such a clone republishes them and must not be
used; the exact release push is the maintained tag-publication path.
GitHub Actions intentionally does not run a separate tag-only workflow. The
new `main` release commit owns one CI result containing preflight, security,
MSRV, Rust checks, ordinary tests, serial PocketIC tests and the conditional
release-profile workspace build. A green tag must never coexist with a red CI
result for the same source merely because the tag ran a weaker job graph.
For one-shot releases, the maintainer or an explicitly authorized agent may run
`make release-patch`, `make release-patch-fast`, `make release-minor`, or
`make release-major`, which perform those steps in order.
Minor and major release commands do not add an interactive confirmation after
the maintainer has already issued the explicit command.

Publishing first re-runs the release-candidate guard, then verifies that every
crate in the governed publish order exposes the same workspace version before
declaring the package set available. A successful subset or library/CLI split
is never reported as a complete Canic release.

The publication manifest boundary is shared with the cheap release-integrity
gate and reads locked offline Cargo metadata without compiling a Rust test.
It checks normal/build dependencies, including renamed, optional and
target-specific dependencies, against unpublished local workspace members.
Development-only dependencies remain outside that publication boundary.
An exact registry version already observed during the current publish invocation
does not need another lookup. Packages skipped by `PUBLISH_FROM` still require
observation before the complete set is declared available. These observations
are invocation-local and do not authorize a later run.

Publication retains individual preflight, registry lookup, package verification/
upload and propagation logs with elapsed seconds and exit codes under
`target/publication-runs/`. A lookup miss before publication is expected and is
distinguished from a failed publish step by its stage name. Both commands print
their timing directories automatically. `make clean` removes these diagnostics
with the other Cargo artifacts; normal release/publish commands retain them.

Tags are immutable.

The dependency-risk inventory also runs on a weekly read-only schedule so a
new advisory is visible even when the repository receives no source push.

## Environment Selection

- `ICP_ENVIRONMENT` selects the target ICP CLI environment.
- If unset, it defaults to `local`.
- Canic automation should target environments declared in `icp.yaml`.
- Use `ICP_ENVIRONMENT` for Make/script defaults and `canic --environment <name>`
  for one-off CLI commands.
- Do not use DFX-era network variables as the Canic automation selector.

## Automation Language Boundary

Do not add Python code, `.py` scripts, Python build helpers, Python test
helpers, or Python CI glue to this repository.

Prefer Rust for durable tooling. Use shell only when a small wrapper is
sufficient.

## Continue From Here

- [Review delivery cadence](delivery-cadence.md)
- [Read the testing rules](../../TESTING.md)
- [Browse all documentation](../README.md)
- [Back to the main README](../../README.md)
