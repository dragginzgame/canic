# Current handoff — 2026-10-01

## Active work and release boundary

The reported release Clippy failure and false `[CANIC-TEST:E001] ... FAIL failing`
line are repaired in additional unstaged edits. The reinstall fixture now uses
the equivalent inclusive range. Progress uses libtest-aware stderr reporting:
passing self-tests retain expected failures in capture, while failed native tests
and uncaptured PocketIC runs still expose diagnostics. The two runner self-tests
and two rendering tests pass; the captured run contains no failure event, and an
explicit uncaptured run still emits it. The validation-runner shell proof and
`canic-testing-internal --all-targets --all-features` warning-denied Clippy pass.
Evidence: `target/review-validation/progress-capture-*.log`. This closes those
two reported issues; the separate reinstall journey qualification below remains
pending. Existing staged work was preserved, with no version or Git action.

The requested generic endpoint framework work for `ic-blob-storage` is complete
in this repository and uncommitted. Public `on_access_denied = "reject"` keeps
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
the separate Fleet regression below still needs its qualification. The earlier
in-progress macro compile errors are resolved. The sibling repository was read
only; downstream adoption, composed-service/Caffeine qualification and paid
uploads are not qualified or authorized by this work. No version bump, broad
gate, commit, publication, deployment or Cargo cleanup was performed.

The maintainer-selected gate exposed a fixture ordering defect in
`generated_reinstall_recovers_lost_install_and_reaches_working_fleet`: it assumed
a withdrawal must precede every installation, although targets advance in order.
The uncommitted correction accepts either injected reply-loss order, requires
both applicable interruptions and checks exact withdrawal counts after recovery.
Its focused test compiled, but replacement-artifact compilation stopped on unused
`decode`/`reject_access` fields during concurrent macro/runtime edits, before the
corrected assertions ran. Formatting and the scoped diff check pass. Evidence:
`target/review-validation/generated-reinstall-recovery-order.log`. Rerun
`make test-pocketic-case CASE=pic::fleet_registry::baseline::tests::generated_reinstall_recovers_lost_install_and_reaches_working_fleet`
after that work settles; release readiness remains pending this qualification.

The deployment-reliability and completed-Fleet cleanup implementation, prior
direct negative/recovery evidence, propagation and cleanup are complete. Package
versions remain `0.110.48`; both changelog views describe the existing open
`0.110.49` batch. HEAD is the maintainer-created `70a0bc9a4` checkpoint
(`0.110.49`). Concurrent macro/runtime work and this fixture correction are
uncommitted. This diagnosis ran only the focused regression; no broad validation,
version transaction, Git publication or live deployment was performed by the agent.

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

The exact CANIC-188 issued `.48` import has its own
[repair decision](../design/0.110-fleet-runtime-contraction/issued-import-recovery.md)
and [incident instructions](../../scripts/dev/canic188/README.md). Its qualified
candidate and original Root are retained under
`.canic/incident-repairs/canic188/ed3084b6908a04b28effa21e00ec425aaf382d1423849fcbb4f3b12414d61f48/`.
Publishing current packages does not resume that operation. Preserve its original
journal, status, artifacts, ceilings and restoration sequence. Live repair,
convergence, replay and frontend acceptance remain outstanding. A separate local
`.48` import with an issued uninstall is outside that exact repair.

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
is accepted after urgent deployment/recovery qualification and Toko Miner
unblocking, before final 0.110 closeout/blob extraction. It covers retained
Coordinator/Root/Store and child IDs, conservation, one existing Host journal,
controller/reset recovery and retirement contraction. Not started; it does not
delay urgent `.49` publication or authorize live/downstream effects.

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
