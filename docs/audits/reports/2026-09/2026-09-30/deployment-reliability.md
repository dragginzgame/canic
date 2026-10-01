# Deployment reliability audit — 2026-09-30

Canic's deployment difficulty has several causes: fixtures drift from production
contracts; the runner hides independent failures; some intended tests never run;
and build/deployment boundaries still contain real defects. Relaxing authority,
cycle conservation or exact replay checks would not repair these causes.

`make test` runs native tests and disposable PocketIC deployments. It does not
deploy a live Canic Fleet. This audit follows both that validation path and the
Host/CLI path used by downstream deployments.

## Identity, scope and limits

This is a `code_trace` with retained execution evidence, reviewed by Codex.
The primary method is `CANIC-RELEASE-INTEGRITY-001/v1`; its formal result is
`partial`, validity `valid`. This requested operational review is broader than
the method's automation scope but does not complete its security/secret-scan,
native-platform or artifact-promotion proof. Build and package findings retain
their Host/publish owners. It is not a minor closeout, numerical trend comparison,
new successful validation receipt or assertion-by-assertion review of all Rust.

Frozen source is `3e38927f47e2e55fed8a864b51ca3a58731dd544`, package version
`0.110.48`, release anchor `v0.110.48`. The two pre-existing changes at audit
capture are the composed readiness test correction in `pic/lifecycle.rs` and its
open `.49` changelog note. Source-tree/product-tree/lockfile/toolchain identities,
method fingerprint, reviewed-file hashes and downstream snapshots are retained
in [deployment-reliability.json](deployment-reliability.json). The product-tree
hash describes committed source; it does not claim the dirty patch is committed.

No full test/validation, package build, publication, network request, native macOS
run or live deployment was executed for this audit. Previous targeted readiness
qualification is separately identified below. Retained logs do not establish a
pass for unexecuted cases. Manual P1 findings below need independent review before
formal audit closeout; no second reviewer or waiver is claimed.

The maintainer selected **Toko Miner only** as the downstream scope. It has dirty
work, including its resolver change. No sibling builds, deployment commands,
dependency updates or retained-operation changes ran. Preliminary inspection of
other local checkouts is excluded from the findings and acceptance plan.
The other developer's reported staging operation is not available in this
checkout; its live balances, completion and recovery remain unverified.

## End-to-end coverage map

| Boundary | Traced owner and evidence | Conclusion |
| --- | --- | --- |
| Local release admission | `Makefile`, `run-release-validation-lane.sh`, `check-fast-patch-eligibility.sh`, candidate/index/push guards | Complete validation has staged barriers and a source-HEAD receipt; post-validation delta checks and failure transaction handling remain incomplete. |
| CI | `.github/workflows/ci.yml`, support matrix | Separate ordinary/PocketIC jobs and native Mac jobs exist; release runs can be cancelled and Mac runner tooling is not qualified by a Wasm build. |
| Test selection and process lifecycle | `run-workspace-tests.sh`, `run-pocketic-workers.sh`, `pic/mod.rs`, `pic/workers/mod.rs`, integration TSV | Recovery barrier, process isolation and retained logs exist; coverage and complete failure feedback have gaps. |
| Fixtures and startup | `pic/lifecycle.rs`, public managed fixtures, bootstrap/import/reset cases, retained failure logs | Several failures are stale test assumptions, not random infrastructure failures; the manually embedded Root peer has no automated freshness proof. |
| Build and reuse | `canister_build/artifact`, `cache`, `reuse`, `fleet_package`, role package admission | Exact finalized-build reuse is useful. Generated lock retention and guessed artifact filenames can still select unexpected inputs/outputs. |
| Readiness and bootstrap | CLI `fleet/bootstrap`, Host `workflow/readiness`, `policy/infrastructure_bootstrap`, registration/recovery ops and workflow | Early authority/funding observations and same-operation supplement exist; fresh admission can still accept an insufficient execution ceiling. |
| Import and activation | `workflow/capacity_import/review`, import policy, bootstrap convergence | Custody and completed-import fences are meaningful; original balance selection can prevent repairing insufficient held-pool headroom. |
| Reinstall and replay | CLI/Host `clean_reinstall`, completed-reset public CLI journey, registration recovery test | Current-build review preserves original paid-effect authority and terminal replay; phase completion must remain distinct from Fleet completion. |
| Packaging and installation | `publish-workspace.sh`, installed/packaged CLI and Wasm probes | Extracted-package compilation is distinct evidence; no installed-package deployment journey closes the full operator path. |
| Consumer orchestration | Toko Miner Make, staging/local/reinstall/toolchain/qualification scripts and CANIC-185–189 feedback | Toko Miner maintains substantial parsing/phase glue. Managed application qualification and a complete Fleet reset are different boundaries. |

The production path traced is build/finalize → generate/readiness → review →
apply retained infrastructure → register → import held capacity → converge
workloads → verify terminal conservation → effect-free replay. Clean reinstall
selects the current build and complete physical inventory, then delegates those
phases. Frontend/application readiness remains a downstream responsibility.

## Findings

All findings are `open` unless a different disposition is stated. Severity is
impact, not proof strength. Locations and named functions refer to the hashed
snapshot; source confirmation is not a new execution reproduction. Original
review identifiers below are deduplication links, not newly fixed findings.

### 1. Independent failures are suppressed, and short contract failures arrive late

`CANIC-110-TESTING-001` — P2, confirmed, `operational_risk`; owner: Testing.

`pic/mod.rs::run_selected_governed_test_cases` catches the first panic, discards
its payload and breaks. `run-pocketic-workers.sh` exits on the first failed worker
and kills the other worker. `run-workspace-tests.sh::run_test` then skips all
remaining serial suites. `worker_groups` appends short lifecycle contract cases
after long registry and Coordinator cases. Failure excerpts select the last 100
trace lines rather than the failing assertion and exact case.

The September 30 runs therefore exposed failures serially: managed-child
settlement, low-reserve preparation and composed IcyDB readiness. The latest
outer failure took 1,483.84 seconds; its actual assertion is at line 4573 of
`target/test-runs/20260930T171959Z-37470.lhZry1/1.log`. The repaired direct-ingress
case runs in 4.23 seconds in its targeted warmed invocation. These durations
have different scopes; they are not a claimed 350-fold speed improvement.

Remedy: preserve real dependency barriers, move independent short contracts early,
retain per-case identity/assertion/rerun details, let independent workers finish,
and resume unexecuted cases in fresh owned processes after a case panic. Do not
continue blindly with a potentially damaged shared fixture or retry a failed case
until it turns green. Report every failure and every skipped dependency. Acceptance
includes cancellation, bounded process cleanup, exact membership and no repeated
completed cases. This extends the September 27 latency audit rather than claiming
its measured two-worker optimization was ineffective.

### 2. Feature-gated tests are outside every normal execution selection

`CANIC-110-TESTING-002` — P2, confirmed, `evidence_gap`; owner: Testing/Host.
Duplicate sources: `tests-ci-2`, `tests-ci-7`.

The ordinary lane omits `governed-pocketic-tests`. The governed lane selects only
the ignored wrapper. That wrapper calls registered cases, not all libtest cases.
`managed_projection_fences_then_opens_and_restores`, the generated-release-cache
test, five `release_artifacts/host_inputs` tests, worker-selection tests and runner
meta-tests are outside that catalogue. Checking the catalogue against its own
partitions cannot detect an unregistered test.

Host `local-fleet` is enabled by the internal governed feature, where Host is only
a dependency. Its own ordinary test binary lacks that feature; the later Host
lane selects ignored `governed_pocketic_` tests. Its non-ignored allocation,
symlink/lock and interrupted-startup tests are consequently not selected there.

Remedy: give pure tests an ordinary compiled owner, explicitly classify every
stateful case, and compare discovered tests to registered/explicitly excluded
identities. Execute the small Host feature-specific native lane. Derive membership;
do not add an aggregate-count pin or run all PocketIC cases concurrently.

### 3. Full serial selections can pass with zero matching tests

`CANIC-110-TESTING-003` — P2, confirmed, `evidence_gap`; owner: Testing.
Duplicate source: `tests-ci-1`.

`run-workspace-tests.sh` performs exact `--list` validation only for targeted
Rust-path mode. Full internal/Host selections accept Cargo's success status even
if a selector has drifted to zero matches. A guard that checks the selector's
shell spelling does not prove it still names a compiled test.

Remedy: validate nonempty discovered selections before starting expensive work
and reconcile executed identities at completion. Some unrelated harnesses in the
workspace Host graph intentionally select zero tests: validate the owning
selection, not every individual harness's count.

### 4. Compile-fail documentation contracts have no maintained test lane

`CANIC-110-TESTING-004` — P2, confirmed, `evidence_gap`; owner: facade/Testing.
Duplicate source: `tests-ci-5`.

Every normal Cargo test invocation selects `--lib`, `--bins` or `--test`; no
Make/CI runner invokes doctests. The `application_scope!` and synchronous
lifecycle-participant compile-fail examples are therefore not exercised by
`make test`. Clippy does not establish those rejection contracts.

Remedy: add a narrow facade doctest/compile-contract lane before PocketIC, with
positive examples as well as invalid participant/scope cases.

### 5. Fresh bootstrap still clamps an insufficient budget into an approved plan

`CANIC-110-HOST-BOOTSTRAP-001` — P1, high, `product_defect`; owner: Host policy.
Related accepted work: R2/R3 and the registration incident; not the same defect as
the now-implemented retained-operation recovery supplement.

`policy/infrastructure_bootstrap::finish` calculates a reserve and then uses
`accumulator.execution_burn.min(spendable)` at line 433. It retains that smaller
value as the immutable execution ceiling. Actual registration actions and their
cost are compiled by `workflow/infrastructure_bootstrap::advance` only after all
initial effects are applied; `registration::require_budget` then rejects the
shortfall. The original reported 237T/154T incident matches this path.

The new supplement provides safe recovery and a useful shortfall diagnostic. It
does not prevent a fresh review from entering the same situation. The review
currently distinguishes available money from required execution insufficiently.

Remedy: derive/quote initialization plus successor work before destructive
admission, retain the full required bound, and either review the necessary funding
or reject with exact deficits. Never silently lower required work to fit funds.
Keep the supplement for already-issued effects. Qualification must include a
fresh underfunded review with zero mutations and the existing retained-operation
recovery/replay case. Exact reserve calibration remains to be measured; do not
replace conservative bounds with arbitrary smaller constants.

### 6. Bootstrap-held pool balances can become an unrepairable import prerequisite

`CANIC-110-HOST-IMPORT-001` — P1, high, `product_defect`; owner: Host import.
Duplicate source: `host-ensure-capacity-4`; belongs to accepted R2.

Bootstrap compilation skips Pool targets. Later import review selects the original
bootstrap source sample or terminal Root sample before considering fresh samples.
Import policy requires sampled balance to cover readiness floor plus debit.
Bootstrap convergence additionally requires exact original imported cycle values.
An originally insufficient held source therefore remains insufficient in the
review even after funding; ordinary workload convergence waits for that import.

Remedy: check every held source's import headroom before bootstrap effects and
provide an explicitly reviewed same-operation additional-funding observation
where required. Preserve original custody, debit history and conservation; do not
replace historical observations or simply delete the equality checks. Qualify
the insufficient-source/top-up case and exact replay. No live instance was tested
in this audit.

### 7. Generated infrastructure locks retain independent dependency state

`CANIC-110-HOST-BUILD-002` — P2, confirmed, `product_defect`; owner: Host build.
Duplicate source: `host-release-set-3`.

`fleet_package::materialize` writes a separate workspace and copies the parent
`Cargo.lock` only when the generated lock is absent. The copy is not atomic.
Subsequent parent dependency corrections need not update an existing generated
lock. The later locked build uses that generated graph; an exact build cache can
faithfully retain an unintended older dependency graph.

Remedy: choose one explicit generated-lock derivation owner, refresh atomically
when the selected parent resolution changes, and bind the resolved generated
locks to provenance/cache inputs. Test a parent-only dependency update, interrupted
materialization and unchanged-input reuse. Do not delete retained release bytes.

### 8. Application artifact selection guesses filenames and shares output names

`CANIC-110-HOST-BUILD-003` — P1, high, `product_defect`; owner: Host build.
Duplicate sources: `host-canister-build-1`, `host-canister-build-2`.

`canister_build/artifact` groups roles by Cargo workspace but executes all
declaration batches before extraction and all runtime batches before finalization.
They share the selected target directory. `declaration_inputs` and
`built_canister_wasm_path` derive `<package_name_with_underscores>.wasm`; the
metadata target's actual name is not used. No cross-workspace output-name
collision rejection appears in spec resolution.

A custom `[lib] name` can leave the expected path missing or stale. Equal package
names in distinct accepted workspaces can overwrite the common uplifted output
before finalization. This audit confirms the selection path, not a newly deployed
wrong-Wasm reproduction.

Remedy: capture Cargo compiler-artifact output per package/target/batch and retain
those exact bytes before another batch can overwrite them, or isolate output
roots. Cover custom lib names, equal names in separate workspaces, stale files
and final role/Candid identity. A cache checksum alone cannot repair wrong source
selection.

### 9. Packaged CLI qualification stops before the deployment boundary

`CANIC-110-PUBLISH-001` — P2, confirmed, `evidence_gap`; owner: CLI/Host/package.

`verify-packaged-downstream-cli.sh::run_probe` lists an App and roles, inspects a
role and prints Ensure help. `v1-readiness-smoke.sh` scaffolds/attaches roles and
evaluates evidence; its Fleet operation is also help. Installed CLI blob/auth
proofs use mocked ICP boundaries. The packaged Wasm proof builds actual artifacts
but does not deploy the installed CLI's Fleet journey. The internal reset journey
calls `canic_cli::run` in the workspace graph.

These are valid, different tests. Together they do not establish an isolated
installed-package build → review → apply → interruption → resume → terminal replay.
The package probes are separate manual Make targets, absent from `make validate`
and the automatic publish script. Current governance does assign them to RC/final
accounting; the defect is incomplete executable coverage/accounting, not a claim
that the existing smoke promised live mutation.

Remedy: one small generated consumer using extracted current packages and an
installed CLI against owned PocketIC, with exact deployed artifact identity and
JSON phase/completion assertions. Reuse it in an explicitly selected package lane;
avoid duplicating every internal Fleet scenario or adding it to every narrow edit.

### 10. The public embedded Root test Wasm still needs a freshness owner

`CANIC-110-TESTING-005` — P2, confirmed, `evidence_gap`; owner: facade/Testing.
Duplicate source: `tests-ci-13`.

The managed Component Group fixture embeds
`crates/canic/src/testing/managed_component_group/fixture/sharding_root_stub.wasm`.
The recent manual rebuild fixes its allocation/Directory ordering. Its README
records a hash and build procedure, but no maintained runner proves that embedded
bytes correspond to current fixture source and required protocol dependencies.

Remedy: retain compact structured source/tool/lock/artifact identity and qualify
the source-to-embedded-artifact relationship when those inputs change. Keep the
small public-fixture behavior test early. Do not rebuild this Wasm on every
unrelated test or pin explanatory README sentences.

### 11. Core publication preserves an obsolete verification exception

`CANIC-110-PUBLISH-002` — P2, confirmed, `operational_risk`; owner: publication.
Duplicate source: `tests-ci-6`.

`publish-workspace.sh` adds `--no-verify` for `canic-core` and its regression test
requires that flag. The `.21` changelog attributes it to the former
`canic-testkit` dev-dependency publication cycle. Core's current dev dependencies
are criterion, futures, k256 and syn; that justification no longer applies.

Remedy: remove the exception and verify the packaged Core before publication.
If a current packaging failure exists, fix or record that actual failure. This
audit did not run a package build and does not assert it already succeeds.

### 12. A failed release commit can still create the release tag

`CANIC-110-RELEASE-001` — P2, confirmed, `product_defect`; owner: release scripts.
Duplicate source: `tests-ci-3`.

The Make `release-commit` shell recipe separates `git commit` and `git tag` with
`;` and does not enable shell errexit. A failed commit can therefore tag the
previous HEAD. The later push guard rejects the mismatched release, limiting the
effect, but the next release attempt encounters an already-existing local tag.

Remedy: make successful commit a prerequisite for tag creation and verify the
resulting release identity. Qualify with fake Git failure outcomes; agents must
not create real commits or tags to test it.

### 13. Allowed release paths are mistaken for allowed release content

`CANIC-110-RELEASE-002` — P1, high, `product_defect`; owner: release admission.
Duplicate source: `tests-ci-11`.

Candidate and index guards permit any `Cargo.toml`/`Cargo.lock` path changes after
the validated source. Candidate metadata checks uniform versions, not a semantic
version-only manifest/lock delta. A dependency, feature or profile edit in an
allowed file can consequently travel with the version transaction without the
guard proving it was part of the validated candidate.

Remedy: compare structured before/after manifests and lock entries, permitting
only the governed version transaction's exact mutations. Retain the existing
source-bound receipt. Do not add prose markers or require unrelated worktree
cleanliness at push time. Qualification must reject a non-version manifest edit
inside an otherwise legitimate release diff.

### 14. The fast lane admits behavior-changing dependency updates

`CANIC-110-RELEASE-003` — P2, confirmed, `governance_conflict`; owner: release admission.
Duplicate source: `tests-ci-10`.

`validate_compatible_lock_patch` permits changed version/checksum lines when the
strings before the final dot match. It does not establish unchanged behavior,
monotonic advancement, or valid pre-release/0.0.x compatibility. The lock-change
lane runs advisory/metadata/native compilation but no behavioral or Wasm test.
That is broader than AGENTS' non-runtime/non-build/non-package fast boundary.

Remedy: keep the fast lane for genuinely non-behavioral changes. A changed
dependency resolution needs its affected behavior/package qualification or the
maintainer-selected complete lane. Do not make every documentation change run
PocketIC to compensate for this separate loophole.

### 15. Main-branch CI can cancel a release's only complete result

`CANIC-110-RELEASE-004` — P2, confirmed, `operational_risk`; owner: CI.
Duplicate source: `tests-ci-9`.

Workflow concurrency uses workflow/ref with unconditional `cancel-in-progress`.
A subsequent main push can cancel the preceding release's tests and conditional
release build. The publish script does not wait for that CI result. This is a
source-confirmed possibility, not a claim that a specific cancelled release is
currently published without all required evidence.

Remedy: cancel obsolete PR runs while preserving each release commit's required
result, and make publication accounting explicit for the exact source/package
identity. Preserve useful local receipt reuse.

### 16. macOS support does not yet extend through the full Make runner

`CANIC-110-TESTING-006` — P2, high, `evidence_gap`; owner: Testing/platform.
Related original findings: `repro-build-1`, `host-icp-network-11` are separate
already-corrected Rust/filesystem issues.

`run-pocketic-workers.sh` requires `setsid` and Bash `wait -n -p`;
`run-workspace-tests.sh` uses associative arrays/mapfile. These are undeclared
requirements for the stock macOS shell/tool environment. Native Mac CI compiles
Host/CLI, runs selected tests, installs tools and builds Wasm; it does not execute
the full worker launcher. The support matrix already marks that evidence pending.

Remedy: implement portable owned process-group lifecycle or explicitly install
and select the required maintained shell/tool implementation, then qualify
success, worker failure, interruption and scratch cleanup on both Mac architectures.
Do not claim native macOS qualification from this Linux source review.

### 17. Downstream wrappers depend on human output and internal phase records

`CANIC-110-CLI-AUTOMATION-001` — P2, confirmed, `operational_risk`; owner: CLI
contract, with separately owned downstream adoption.

Toko Miner's `managed-output.sh` parses `Release build:`, `plan_sha256:`,
`terminal:` and `plan_scope:` lines, then reads internal plan/journal JSON to
decide completion. Ordinary local convergence implements up to 20 plan/apply
passes. Reinstall already uses JSON, but retains its own next-phase state and
parses Canic's continuation; staging still uses the human report path.
`local_fleet.py::next_action` checks exact tokenized `next_command` arguments and
knows three internal phase shapes; `local-reinstall.sh` permits up to 40 phase
calls and reads the internal journal on nonzero exit to recognize replan. This
fails closed, but every contract change can require coordinated wrapper changes.
Canic's full-App build has no JSON result option; single-role `--provenance`
requires a role and does not replace the full-build output contract.

Remedy: finish one structured CLI build/operation result with exact phase,
operation/review identity, completion and next action. Keep approval an explicit
consumer decision; do not execute arbitrary `next_command` text with `eval`.
Test the consumer shape in Canic's generated workspace, then record downstream
adoption separately. No sibling edits are authorized by this audit.

Toko Miner's published `.48` CLI versus resolver-3 manifest conflict is the known
CANIC-189 publication boundary;
the exact resolver restriction is already removed in current Canic source.

`qualify-canic-adoption.sh` builds real application Wasms and exercises the
published managed Component Group fixture. That proves application lifecycle,
caller fences and composed state, not a full multi-source Fleet import. The local
candidate script proves source/path integration and compilation, not installed
registry-CLI deployment. Existing Canic incident qualification does retain the
nine-workload/fifteen-spare public-CLI reset case; its existence must not be erased
by calling all coverage synthetic. Complete the missing package/consumer boundary
around that evidence instead of adding another unrelated happy-path fixture.

### 18. Some guards and current release documents still freeze obsolete assumptions

`CANIC-110-RELEASE-005` — P3, confirmed, `documentation_drift`; owner: release docs/guards.

The package/installed smoke tests require the exact Ensure explanatory sentence.
The release-integrity guard compares the entire release-push recipe and requires
implementation spellings in helpers despite also having behavioral fixtures.
Some spelling checks concern meaningful exact identifiers; those should not be
removed indiscriminately. Others constrain refactoring without proving behavior.

Current release-matrix/package documents still describe two prose blob inventory
gates and generated-wrapper fallback/canonical-sibling proof paths that the
maintained generated Fleet proof no longer uses. The matrix also describes
cross-release compatibility evidence without the current reinstall-only boundary.
These are active guidance, distinct from deliberately retained historical reports.

Remedy: remove stale current claims; test parsed commands/results and structured
records. Retain document/link presence and real authority identifiers. Delete
obsolete paths and their anti-resurrection checks together; do not create more
release gates that police narrative wording.

## Existing corrections and boundaries worth preserving

- Resolver admission: exact Cargo resolver restriction removed; generated Canic
  packages use resolver 3. Do not restore a resolver policy to conceal a fixture
  or packaging problem.
- Bootstrap recovery: exact supplementary review, funding and additional finite
  allowance preserve the original journal/applied prefix. Keep the retained
  operation; pool import waits for bootstrap completion.
- Managed child fixture: allocation reply waits for actual installation and
  Directory identity. The production placement rejection was correct.
- Low-reserve fixture: retry only typed busy-owner unavailability; authority
  conflict remains fatal.
- Composed startup: replace three fixed ticks with bounded typed IcyDB readiness;
  fail on terminal startup failure. Both exact cases pass, in 4.23s/9.35s, and
  affected-package all-target/all-feature Clippy passes. This is the only new
  test-source correction accompanying this audit.
- Negative Candid cases: shared quiet parsing removes unsolicited stderr while
  retaining returned errors. Do not filter real Candid errors out of logs.
- Immutable artifact reuse, scoped build locks, owned scratch, intent-before-effect
  journalling, exact controller/caller binding, finite paid budgets, terminal cycle
  conservation and effect-free replay protect real invariants. Keep them.

## Implementation sequence and acceptance

1. **Reliable feedback and selection:** findings 1–4. Produce an authoritative
   per-case inventory/result, expose every independent failure, classify actual
   dependencies and keep short contracts early. Use native selector/runner tests
   plus a small real two-worker failure/isolation proof. Avoid a new broad suite
   during implementation.
2. **Fresh deployment admission:** findings 5–6. Quote the full operation before
   destructive effects; preserve the accepted original-operation recovery path.
   Run focused native budget tests and the exact underfunded-bootstrap/import
   recovery journeys. Distinguish shortfall from authority corruption everywhere
   touched.
3. **Artifact and Toko Miner fidelity:** findings 7–10 and 17. Correct generated
   lock/output ownership, qualify the embedded peer and add one disposable
   installed-package journey consuming structured outputs. Model the Toko Miner
   nine-workload/fifteen-spare estate and its actual phase/result consumption;
   keep a small package smoke separate from the representative large reset proof.
   Downstream adoption
   remains a separate read-only handoff until explicitly authorized.
4. **Release/host cleanup:** findings 11–16 and 18. Remove the obsolete exception,
   fix commit/tag sequencing and semantic release deltas, tighten fast-lane
   eligibility, retain release CI and qualify the Mac launcher. Use fake external
   commands for release regression tests; never exercise real Git publication.

These are coherent follow-up slices, not a demand for one patch per finding or
for all 401 historical review findings to block the urgent incident correction.
The audit does not change the `.49` publication boundary automatically. It does
supersede the assumption that one repaired late fixture proves the entire new
deployment-reliability batch is ready.

On the maintainer's next explicitly selected full gate, retain all case outcomes,
worker wall times, cache/build time and first-failure latency. Compare equivalent
source/cache conditions before claiming a throughput gain. A successful exact
case establishes that case; a completed package proof establishes its documented
package boundary; neither is a full deployment or release verdict.

## Original feedback and unresolved verification

Document delivery checks pass: reviewed source hashes and local file links,
unique finding identities, scoped Rust formatting, whitespace and current-document
semantics. The two existing layout warnings remain advisory. The focused
`canic --test changelog_governance` Cargo test passes (one test); its log is
`target/review-validation/deployment-audit-changelog.log`. It verifies the open
notes, not any new deployment finding. Invocation-owned scratch was removed;
release builds and retained operations remain intact.

The embedded review's 476 raw records include duplicates/refutations; its retained
401-finding summary must not be replaced by that raw count. The duplicate mappings
above reconfirm deployment-relevant entries only. They do not increase the
previous 27-fixed minimum. The remaining R2–R8 runtime findings, unrelated auth,
backup, blob, timer and concurrency paths were not individually re-adjudicated.

For formal completion, retain an independent review of manual P1 findings,
current Mac results, package-execution evidence and the remote retained-operation
outcome. No secrets/unsafe-code/reproducibility closeout, live IC fee measurement
or general all-project compatibility claim is made. The requested source audit
and remediation sequence are complete within the explicit trace map; the
implementation batch and formal release-integrity qualification remain open.
