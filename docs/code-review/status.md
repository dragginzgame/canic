# Code review implementation status

## Findings summary

The original review found **401 distinct, non-refuted findings**, after merging
67 duplicates and excluding 8 refuted reports.

| Original severity | Findings |
| --- | ---: |
| Critical | 1 |
| High | 44 |
| Medium | 105 |
| Low | 220 |
| Info | 31 |
| **Total** | **401** |

| Current tracked status | Findings |
| --- | ---: |
| Addressed and qualified | 31 |
| Awaiting finding-by-finding disposition | 370 |

The 31 qualified original IDs are a conservative minimum. The other 370 include
unchecked, partial and unreconciled work; they are not 370 confirmed current
bugs. Severity describes the original review snapshot. Additional fixes do not
increase the qualified count until reconciled with their original IDs.

Last updated: **2026-10-03**. Scope: Canic implementation and qualification;
Toko Miner is downstream feedback and read-only context.

The maintainer reports **0.110.51 published** and packages are **0.110.51**.
The accepted **FR1 Fleet release-to-capacity batch is active and unfinished**.
Completed Host/CLI/Backup corrections are recorded in the open **0.110.52** draft;
incomplete FR1 remains in root `Unreleased`. Its restored foundations are qualified;
evidence below remains scoped to the checks actually run.
This page tracks progress; it is not release authority or evidence of publication.

## Review sources

- [Original review export](<Canic Code Review.html>) — preserved source findings.
- [Implementation tracker and counted IDs](../design/0.110-fleet-runtime-contraction/status.md#accepted-code-review-corrections--2026-09-30) — detailed R1–R8 ownership and traceability.
- [Current session handoff](../status/current.md) — latest execution evidence and concurrent work.
- [Deployment reliability audit](../audits/reports/2026-09/2026-09-30/deployment-reliability.md) — separate 18-finding audit; its snapshot findings are historical, not current dispositions.
- [0.110 release notes](../changelog/0.110.md) — included behavior changes, distinct from validation and publication status.

## Recent delivery status

FR1 shared replay discovery is now in progress: controller-only pages retain
expired uncertainty and original effect/accounting identities, with bounded
stable records and compact responses. Three discovery tests, 150 replay regressions
and affected-package/governed-journey Clippy pass. Fixture refresh/verification
pass; the real paid-grant journey passes (325.14s, 414s runner), as does the
Coordinator/provisioning journey (142.87s, 143s runner).
Host's provisioning reader now allows the complete Root reply type table; all
28 selected Host tests and affected lint pass, with a real-wire decoder proof
added to the passing Root journey. Bounded Host receipt collection now covers
Coordinator and selected Roots; its new native/signed-wire tests and production
Root decoder proof pass (33 native tests; signed-query proof 2.78s; paid-grant
proof 36.09s). Coordinator public DTO/canonical Candid propagation passes five
contract tests and affected lint. DTO round trips, fixture verification and the
public-request Coordinator journey pass (98.39s; 222s runner), along with scoped
format/document/whitespace checks. Evidence is retained under
`target/review-validation/fr1-host-receipts-*` and `fr1-replay-canonical-*`.
Live producer quiescence, custody handoff, account-recovery qualification,
execution/resume/CLI, whole-Fleet proof and retirement contraction remain.
This extends paid-owner discovery;
it adds no completed original-review finding or push-readiness claim.

FR1 provisioning discovery now pages both retained Root journals independently
of active pointers, preserving original identity, exact stage and outstanding
delivery without resuming work. Its 24 provisioning and three Directory native
tests pass, along with affected-package and governed-journey Clippy. Host now
collects the original pages under custody/Registry checks; all 27 selected Host
release tests and Host Clippy pass. Fixture refresh/verification and the signed-query
Host PocketIC proof pass (2.17s, 4s runner). The exact interrupted-to-terminal Root
PocketIC proof passes (30.46s, 51s runner), including controller denial, replay and
unchanged balances. This is partial discovery, not complete settlement
or push readiness; the original-review count remains 31/401.

FR1's Host pool collector is implemented: bounded signed reads preserve each
Root's exact obligations, with custody/Registry checks and no partial result on
refusal. After space was freed, all 22 selected release-ops native tests and Host
library/test Clippy pass; four native tests cover pool evidence. The signed-query
PocketIC case passes in 1.77s (4s runner), including failure after the first of two
Roots succeeds, replay and unchanged balances (`fr1-host-pool-*` logs).
Provisioning/Directory journal discovery is the next in-progress owner.
Membership/authentication remains with the other session.
This partial FR1 step adds no original-review closure count.

FR1 now exposes controller-only Root pool evidence independently of new-work
admission. Held Store/source IDs, exhausted import authority and progress,
creation uncertainty and pending handoff remain observable without mutation.
All 62 selected pool tests, affected-package Clippy and fixture refresh/verification
pass. The exact import PocketIC case passes in 88.42s (170s runner), covering
both custody paths, retained progress, authorization, replay and unchanged balances
(`fr1-pool-census-*`, removed by subsequent external target cleanup). Other obligation owners and complete FR1
execution remain unfinished. This
partial slice leaves the original-review closure count at 31/401.

FR1's bounded Host collector now includes existing Coordinator funding status,
with exact Root membership/policy/lifecycle checks. It preserves pending grants,
reservation windows, terminal history and policy rotation alongside Root evidence.
Completed Coordinator history alone is not pending work; lost-reply work on
Root remains visible. All 44 selected Host release tests, warning-denied Host
library/test Clippy and the extended signed-query PocketIC case pass (1.29s;
72s runner). Evidence: `target/review-validation/fr1-coordinator-evidence-*`.
Runtime/Wasm source is unchanged; full FR1 execution remains incomplete.
This Host-only continuation adds no original-review closure count.

FR1 funding assessment now distinguishes historical receipts, known unissued or
refused transfers, unresolved effects and refund residual review. It preserves
pending Coordinator work and all original account evidence. The supporting Core
fix persists transfer uncertainty across lost replies, retains uncertain spending
reservations and prevents fee changes while the original debit is unresolved.
The required current-record field follows pre-1.0 reinstall-only policy. Core's
86 refill tests, 42 Host release tests and 17 Root funding selections pass;
affected-package governed Clippy, fixture refresh and both exact PocketIC cases
pass (Ledger/CMC 229.71s; Host 0.87s). Logs are
`target/review-validation/fr1-funding-assessment-*`; the current handoff records
qualification details.
This partial FR1 work adds no original-finding closure count.

FR1 now has a Host collector for every selected Root's funding pages, with signed
network/operator binding, bracketed certified custody and Registry observations,
bounded decoding/collection and no partial result after refusal. Five native
tests, Host library/test Clippy and the extended certified-ownership PocketIC case
pass (0.88s; 70s runner); evidence is `target/review-validation/fr1-host-funding-*`.
This preserves evidence for later reconciliation, not producer quiescence or
settlement authority, and adds no original-review closure count.

FR1's bounded Root funding census is qualified: 84 Core refill tests, 10 Root
funding tests, affected-package all-feature and governed-feature Clippy,
embedded peer refresh/final verification and the
exact real Ledger/CMC PocketIC case pass (226.87s; 243s runner).
`target/review-validation/fr1-funding-census-pocketic-verified.log` records the
successful case. Retained exhausted notifications, refund/expiry evidence and
historical accounts remain discoverable; controller denial and query replay
preserve balances. This is observation only: complete obligation collection,
quiescence, account recovery, execution/CLI and whole-Fleet proof remain. No
additional original-review finding is counted from this partial FR1 slice.

Two more low findings are fixed: `cli-core-5` rejects colliding canister ID export
variables before either shell or JSON output; `host-icp-network-9` permits exact
local reset of an instance that exceeds load traversal limits or contains
interior symlinks. The instance root itself must remain a real directory, link
targets remain untouched, and terminal replay preserves its receipt. Seven
`info_env` tests and three native reset regressions pass; retained evidence is
`target/review-validation/low-review-*.log`. Host/CLI all-feature library/test
Clippy passes with `--no-deps` and warnings denied. Both corrections extend the .52 draft;
FR1 remains unfinished.

Two simple original findings are fixed: `host-icp-network-10` accepts additive
ICP CLI fields in balance, snapshot inventory and known visibility output;
`backup-persistence-12` accepts equivalent checksum hex casing during artifact
verification and restore-preview validation. Required fields, known visibility
variants, hash syntax and actual artifact integrity remain checked. Focused
native evidence is retained as `target/review-validation/simple-review-*`.
The wider ICP selection passed 72 tests but two unrelated HTTP listener tests
were denied by the sandbox; narrower affected selections pass.
All 66 selected Host/Backup tests and Backup library/test Clippy pass. Host
library/test Clippy passes with `--no-deps`; dependency-inclusive lint encounters
the then-in-progress Core refill panic-doc warning. That warning is now corrected
and dependency-inclusive FR1 lint passes. The .52 draft includes both corrections;
FR1 remains unfinished.

The service-authority denial now identifies the receiving canister's missing
active authority and the required Fleet service ID, instead of suggesting a caller
admission problem. Its guard and diagnostic code are unchanged. All 28 Core
access tests, Core library Clippy and the embedded fixture refresh pass;
`service-authority-message-*` logs retain the evidence. The .51 notes include
this correction; it does not diagnose or repair Toko's reported deployment binding.

“Qualified” means the recorded targeted checks passed for their tested source
and dependency state. It does not mean published, live-deployed, or accepted by
Toko. Evidence paths below are local retained logs under
`target/review-validation/`; the linked handoff gives detailed results.

| Work | Current disposition | Evidence / remaining boundary |
| --- | --- | --- |
| FR1 restored after .50 | **Restored and requalified foundations; batch unfinished.** Durable read reservations, authenticated ownership inventory and release fencing are integrated with the .50 fixes. Spent read reservations alone do not block explicit reset; uncertain effects still do. | 74 native tests, four exact PocketIC cases, affected-package Clippy and refreshed embedded peer pass. `fr1-resumed-*`. Account settlement, role quiescence, executor/CLI, whole-Fleet proof and retirement contraction remain. No release command is exposed. |
| FR1 declared Ledger accounts | **Implemented observation boundary; batch unfinished.** Bounded signed queries read exact declared accounts, including zero balances. Default-subaccount normalization prevents false drift and duplicate counting. | 30 selected Host native tests, Host Clippy and exact account PocketIC proof pass (0.72s). `fr1-accounts-*`. Role-owned account discovery, recovery-artifact qualification and paid-effect settlement remain; these results add no original-review closure count. |
| Operator Component CLI fixture drift | **Fixed and qualified.** Typed journal, state and topology constructors include the required recovery field and catch added record fields at compilation. Production validation is unchanged. | Exact public CLI PocketIC case passes in 23.18s; runner 39s plus successful cleanup. Governed-feature library/test Clippy passes with warnings denied. `operator-cli-fixture-{pocketic,clippy}-final.log`. The prior complete run failed; no new full-suite pass is claimed. |
| CANIC-192: preserve selected deployment inputs | **Implemented and qualified.** Desired, policy, inventory and identity survive apply, resume, import and successor review. Changed destinations reject before effects; equivalent paths are accepted. | 82 CLI tests, 23 Host tests, affected-package Clippy and public CLI PocketIC reset/recovery/offline replay pass. `canic192-*`; PocketIC 481.48s. The binaries predate the concurrent ic-memory 0.15.2 bump. |
| CANIC-191 / RD1: reset unfinished installations | **Implemented and qualified.** Current artifacts and physical custody replace predecessor-completion requirements, including malformed application state. | Partial bootstrap/activation/import reset, uncertain-effect fences, conservation and replay are qualified. `canic191-*`, `release-split-*`, and the later CANIC-192 journey. Live Toko adoption remains separate. |
| CANIC-190: exhausted Host attempts | **Implemented and qualified.** `fleet recover-attempts` grants two additional attempts per reviewed exhausted resource after exact-digest approval. | Original counters, baselines, ingress and Root spending limits remain intact. Native partial-publication/replay checks and exhausted signed-handoff PocketIC recovery pass. `r2-*`. Exhausted Root spending authority still requires cycle-safe reset. |
| R2: Root import call reduction | **Implemented and qualified for this bounded outcome.** Confirmation reuses its final status sample before the next mutation. | Eight running sources use 105 calls; placement-drift refusal/recovery uses 106. Minimum/recommended review allowances are 105/224. `r2-*`. This does not close all R2 findings or qualify cross-advance placement caching. |
| Deployment and release guard cleanup | **Implemented and qualified.** Incidental files and surplus cycles no longer cause false refusals; release checks validate authority and behavior rather than source spelling. | 17 selected Rust regressions, focused Clippy, release-integrity fixtures, ShellCheck and document/inventory checks pass. `deployment-guards-*`. Custody, unexplained deficits and uncertain paid effects remain protected. |
| Gitleaks removal | **Complete at maintainer request.** Remove the scanner, installer, pins, exclusions, local/CI gate and mandatory audit scan requirement. | Targeted shell/workflow lint, authority/catalog/document checks and print-only validation dispatch pass. Remaining release-tool fixtures pass with the unrelated commit-creating tag fixture excluded. `gitleaks-removal-*`. |
| Rust 1.99 and ordinary-test fallout | **Implemented and qualified.** Toolchain pins, lint cleanup, optional IcyDB selection, isolated runner inputs and stale native fixtures are corrected. | Full Canic-owned Clippy was explicitly run for the toolchain update; subsequent corrections have targeted evidence. `rust199-*`, `ordinary-fallout-*`. IcyDB drift remains an optional local-consumer limitation, not a Canic release blocker. |
| ic-memory adoption | **0.15.3 selected and lifecycle-qualified.** Default diagnostics preserve configured bootstrap; the embedded peer and provenance are refreshed. | Earlier 35 memory, 70 receipt, four ABI/identity tests and Core Clippy qualify 0.15.2. The 0.15.3 public-API probe, final embedded verifier and current-graph managed-component PocketIC journey pass (166.24s). `ic-memory-0152-*`, `upstream-feedback-recheck-memory.*`, `lock-reconcile-*`. |
| Deployment reliability audit, findings 1–18 | **Implementation follow-through recorded in the handoff.** Includes complete failure reporting, test selection, bootstrap budgets, held-source funding, build reuse, packaging and automation. | Findings 9/10 have installed-package and embedded-fixture journeys; finding 17 has command-array/phase evidence, extended by CANIC-192. Native macOS execution, formal audit closeout and downstream live acceptance are separate qualifications. Do not count these 18 again as original-review closures. |

## Published .50 CI follow-up — 2026-10-02

A later native inventory failure identified the embedded-peer reproduction test
as an unclassified ignored test. Its exact identity is now registered; all four
native runner/inventory checks pass, preserving recovery ordering and unique
ownership. Evidence: `target/review-validation/ci-embedded-inventory-registration.log`.
Completed CI/build/auth/diagnostic changes are consolidated in the existing .51
changelog draft; incomplete FR1 stays in root `Unreleased`. No broad suite or
additional PocketIC journey was run for the registration correction.

The maintainer-requested size tuning is implemented: `fast` uses ThinLTO and eight
code-generation units across workspace and generated infrastructure profiles.
The real test Root's code section falls from 9.30 to 8.98 MiB with identical
Candid; 12 targeted native tests and Host Clippy pass, and the embedded peer is
refreshed. Existing downstream applications need the updated profile in their
own workspace manifests. This does not close the portability issue below or add
an original-review closure. `target/review-validation/fast-profile-*` and
`fast-root-{before,after}.*` retain the evidence.

Published `.50` [CI run 37004277856](https://github.com/dragginzgame/canic/actions/runs/37004277856)
exposed two corrected setup defects: missing macOS Binaryen runtime libraries and
missing `cargo-edit` for ordinary release-guard tests. Installer behavior fixtures,
the release-candidate fixture and shell/workflow lint pass locally. The third
failure now has a qualified repair: the embedded allocation peer builds from a
private source copy using a synthetic test-only version and remapped paths,
preserving exact external locked dependencies. Three focused tests include real
byte reproduction across source directories and a private release-version
transaction, plus refusal of broken source. Internal Testing Clippy and the exact
public managed-component PocketIC lifecycle pass (229.76s; 382s runner).
Evidence: `target/review-validation/ci-embedded-*`; the current handoff records
the artifact hash and exact qualification scope. Actual GitHub and native macOS
reruns remain unqualified; no additional original-review closure is counted.

Root issuer configuration work (AF1) is now complete. Its refreshed embedded peer
matches the settled combined source, and the standalone verifier plus both native
fixture evidence tests pass. The interrupted combined-build check is resolved;
evidence is in `ci-embedded-{native,verify}-settled.log` under the same directory.
The earlier lifecycle and byte-reproduction timings describe the preceding peer;
AF1 owns qualification of the new auth behavior. No expensive journey was repeated.
The complete FR1 batch remains unfinished, so this CI repair is not a whole-batch
push-readiness verdict.

## Upstream feedback recheck — 2026-10-02

Read-only review of sibling checkouts, release source and retained feedback.
Canic's manifest and registry lock select all four versions below. No upstream
files or dependencies were changed by this recheck.

| Dependency selected | Feedback disposition | Remaining boundary |
| --- | --- | --- |
| ic-memory 0.15.3 | **Fixed.** Default export, commit-recovery and both doctor helpers no longer construct the runtime before bootstrap. | Fresh public-API probe confirms typed `NotBootstrapped` followed by successful configured 16-page bootstrap for all four helpers and a control. `upstream-feedback-recheck-memory.{rs,log}`. |
| ic-query 0.44.2 | **Upstream export-alias fix now selected.** Manifest and lockfile agree after correcting the stale 0.44.1 entry. Host and CLI preserve the new history-cache progress event. | Both dependency gates pass; 10 Host and three CLI catalog tests, affected-package Clippy and the managed-component lifecycle case pass. Canic does not yet opt its caller-owned source into cross-process history caching. |
| ic-testkit 0.10.4 | **Classifier fix retained.** Generic channel-closure and quoted refusal text no longer count as dead PocketIC transport. | Prior 0.10.3 build and three public-API regressions pass: `testkit-0103-{build,probe}.log`. 0.10.4 removes the experimental teardown patch; production PocketIC remains unchanged. The hang and original Busy/tick cause remain unresolved. |
| ic-timers 0.8.1 | **No new runtime defect found.** Release changes are tooling, lint annotations and documentation; registration/deadline behavior is unchanged. | Retained four-case timer evidence remains relevant. Upstream handoff still calls 0.8.1 unpublished despite its release commit and Canic registry selection: documentation drift only. |

The latest dependency combination passes the exact public managed-component
lifecycle journey, embedded verifier, scoped Host/CLI tests and Clippy, and both
dependency gates. Evidence is in `target/review-validation/lock-reconcile-*`;
this is targeted qualification, not a full workspace run. Query's own upstream
tests remain separate evidence, and Canic disk-history adoption remains open.

## Remaining accepted work

| Owner / batch | Work still open |
| --- | --- |
| R2 — Host, CLI, Control Plane | Remaining import/operation recovery findings, older unknown outcomes and Root-cap disposition. Host attempt recovery is now qualified; do not continue tracking that specific gap as unimplemented. |
| R3 — funding and conservation | Remaining funding accounting and authority work beyond qualified signing admission, grant replay and native-credit corrections. |
| R4 — placement and recycling | Caller replay, issuer and funding authority isolation. Existing placement/recycling and allocation-bound recovery evidence does not close these remaining findings. |
| R5 — Backup, CLI, Core | Complete uploads, consistent capture and remaining release/restore authority. Filesystem locking and retention corrections are qualified; fresh live backup remains unavailable. |
| R6 — background drivers | Driver ownership and trap recovery; typed platform-unavailable retry classification is already qualified. |
| R7 — convergence | Operation-specific authority across unrelated Fleet changes, mirror acknowledgements, rotation and activation fences. |
| R8 / FR1 — retirement and release | Complete paid-obligation and account collection, cycle/ICP accounting, quiescence, execution/recovery, CLI, whole-Fleet proof and retirement contraction. Parked foundations are now restored and requalified after .50; preserve their original recovery bundle. |
| CS1 — simplification | Accepted simplification work follows FR1; its direct evidence, propagation and cleanup remain outstanding. |
| Qualification and triage | Native macOS results, remaining execution-boundary work, and finding-by-finding disposition of the full original review. R1's bounded quota outcome is already qualified. |

The [implementation sequence](../design/0.110-fleet-runtime-contraction/status.md)
owns detailed acceptance and dependencies. Completing every review finding is
not a prerequisite for shipping a bounded corrective batch. Final 0.110 closeout
must be explicitly requested and accepted before starting 0.111.

## Next checkpoints and maintenance

1. Keep the recorded current-graph qualification distinct from earlier version
   evidence when dependencies or runtime sources change again.
2. Keep incomplete FR1 in root `Unreleased` until its coherent outcome is complete;
   do not assign one patch per restored helper. Publication and live deployment
   remain separate maintainer-selected actions.
3. Continue the accepted remaining sequence, including resumed FR1 and CS1, using
   the detailed tracker rather than treating historical snapshot findings as new bugs.
4. Reconcile completed work with original finding IDs before changing the 31/401
   count. Record duplicates, partial fixes and superseded findings explicitly.

For each completed batch, update this file's date, disposition, evidence and
remaining boundary alongside the current handoff. Keep detailed test output in
retained logs and exact finding IDs in the implementation tracker. Do not turn
this descriptive status file, its wording or its counts into a release gate.
