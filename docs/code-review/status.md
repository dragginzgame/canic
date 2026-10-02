# Code review implementation status

Last updated: **2026-10-02**. Scope: Canic implementation and qualification;
Toko Miner is downstream feedback and read-only context.

The maintainer reports **0.110.50 published**; main retains its release tag and
complete validation receipt, and packages are **0.110.50**. The accepted **FR1
Fleet release-to-capacity batch is resumed and unfinished**, with no next patch
allocated. Its restored foundations and current embedded peer are requalified;
evidence below remains scoped to the checks actually run.
This page tracks progress; it is not release authority or evidence of publication.

## Review coverage

The original review contains **401 findings**. The implementation tracker has
**27 distinct original finding IDs explicitly counted as addressed and qualified**.
That is a conservative minimum, not a current total of every fix. Recent work
below has additional evidence but has not all been reconciled against original
IDs. Do not interpret the other 374 entries as 374 confirmed remaining defects,
or add issue counts and test counts to the closure total.

- [Original review export](<Canic Code Review.html>) — preserved source findings.
- [Implementation tracker and counted IDs](../design/0.110-fleet-runtime-contraction/status.md#accepted-code-review-corrections--2026-09-30) — detailed R1–R8 ownership and traceability.
- [Current session handoff](../status/current.md) — latest execution evidence and concurrent work.
- [Deployment reliability audit](../audits/reports/2026-09/2026-09-30/deployment-reliability.md) — separate 18-finding audit; its snapshot findings are historical, not current dispositions.
- [0.110 release notes](../changelog/0.110.md) — included behavior changes, distinct from validation and publication status.

## Recent delivery status

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
4. Reconcile completed work with original finding IDs before changing the 27/401
   count. Record duplicates, partial fixes and superseded findings explicitly.

For each completed batch, update this file's date, disposition, evidence and
remaining boundary alongside the current handoff. Keep detailed test output in
retained logs and exact finding IDs in the implementation tracker. Do not turn
this descriptive status file, its wording or its counts into a release gate.
