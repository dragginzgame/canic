# Toko Miner upstream feedback — 2026-09-11

## Current refresh — 2026-09-13

Read-only inspection now finds CANIC-169, CANIC-168 and new evidence within CANIC-166.
The downstream adoption report records .15 adoption and focused local evidence
for 163/164/167; 165 remains available for downstream fixture conversion. The
earlier .14/.15 publication and readiness statements below retain their dated
scope, rather than describing this current worktree.

- **CANIC-169 — agreed, implemented and qualified.** PendingReset and Failed
  configured pool assets project as Retained, keeping their funding under the
  existing reconciliation owner. Qualification also exposed and corrected the
  handoff from a fully applied reset to its next same-operation review. Exact
  completed install evidence survives the pause and retry; the duplicate-funding
  guard remains unchanged. See [qualification below](#canic-169-qualification).
- **CANIC-168 — agreed, implemented and qualified.** Implicit `sccache` discovery
  does not prove compiler startup. The host now probes direct compiler startup
  and the implicit wrapper before each actual Cargo compilation, preserves
  original process evidence and points to the existing `RUSTC_WRAPPER=`
  override. Explicit wrapper choices bypass this probe; no build retry or
  fallback is added. Qualification and final readiness are recorded in the
  [current handoff](../../../../status/current.md).
- **CANIC-166 — agreed, source-entry correction implemented.** The downstream .15
  staging-plan attempt reports missing `publication_attempts` on all 30
  retained journal effect rows. Ordinary planning now diagnoses journal failures
  through the existing raw evidence inspector; explicit reinstall calls that
  inspector before executable journal admission. No missing counters are
  defaulted and no old journal is made executable. All 192 native Fleet Ensure
  cases pass, including the 30-row omission shape, nonzero retained counters,
  unchanged source bytes, review rejection without live evidence and interrupted
  handoff recovery. The exact copied source also exposed a completed bootstrap
  receipt without `fixtures`; a private receipt projection verifies its original
  action hash only within the Applied prefix and cannot produce an executable
  action. The complete copied source now reaches the recovery diagnostic.
- **CANIC-014 — agreed, transaction correction implemented.** The governed bump
  now generates a visible version/source/date/gate snapshot and explicitly
  frames the preserved human handoff as pre-transaction history. Sixteen
  release-flow cases, scoped Clippy and ShellCheck pass. Historical tags are
  unchanged; next-tag verification and package publication remain outstanding.
- **Role-specific memory initialization — parked.** The maintainer deferred
  this optional optimization under design ideas. It does not delay the current
  memory batch or expand CANIC-168.

Sources read: `../toko-miner/docs/upstream/canic.md` (169, 168 and the updated 166
body) and its September 13 release-adoption report. Toko Miner remains read-only.

The ignored historical `.12` journey failed setup before runtime effects:
its archived 3,738 source files match the immutable commit, but the current
host correctly rejects its old package graph. Retired fixture-manifest and
bootstrap response shapes also prevent simply bypassing that check. The
obsolete executable branch is removed under the maintainer's test-cleanup
instruction; its original installed-source proof remains in the
[September 8 report](../2026-09-08/activation-feedback.md#final-installed-source-proof).
Current production version/protocol checks remain intact. The maintained
current-runtime recovery journey and source/adoption tests remain. This is
explicit retirement of a stale historical harness, not fresh historical-runtime
qualification. Failed setup log: `/tmp/canic166-installed-recovery.log`.

CANIC-141 is now implemented through its requested **reject-before-effects**
alternative. A scalar management fee may authorize direct and Root pool
creation on only one exact subnet per operation. Compilation and retained apply
reject mixed targets with `MixedSubnetCreationFees`; pending Root creations
participate, while reused/reinstalled identities and idle fully supplied Roots
do not. The generator test now rejects fresh mixed placement and retains its
single-subnet creation/retry path. Native checks cover direct/mixed-Root
rejection without paid mutations, unchanged retained evidence, pending work and
unrelated retained identities. All 197 Fleet Ensure tests and affected
host/testing Clippy pass. Logs: `/tmp/canic141-fleet-tests.log` and
`/tmp/canic-feedback-final-clippy.log`. The native governed-catalogue test also
passes with the explicit `governed-pocketic-tests` feature and exact selector
(`/tmp/canic-feedback-inventory.log`); it checks membership/order without
starting a replica. The first invocation omitted that feature and matched zero
cases, so it is not counted as validation. This deliberately does not claim exact
mixed-subnet fee support; even differently located subnets with numerically
equal reviewed fees are rejected. No live fee lookup or fee value is inferred.

### Remaining requested work and release boundary

**Promoted:** the maintainer explicitly selected all four requests for 0.110
on 2026-09-13. The active OP1–OP4 tracker supersedes the earlier release-position
restriction below. All four Canic implementation batches are now ready in open
.16; the tracker records their separate native, live and package evidence.

The maintainer asks for all remaining feedback to be handled. These are the
promoted product outcomes, now implemented in Canic with adoption still separate:

| Feedback | Bounded outcome and owner | Required evidence / boundary |
| --- | --- | --- |
| CANIC-010 | Host/CLI plan and reconcile one admitted top-level Component through the existing Root lifecycle. | Exact placement/release binding, lost allocation/install replies, capacity and authority rejection, terminal effect-free replay; [design](../../../../design/0.110-fleet-runtime-contraction/0.110-design.md#op1-operator-component-lifecycle-canic-010). |
| CANIC-008 | Read-only host export of terminal App/Fleet routing and generated frontend-binding identities. | Unknown/nonterminal role rejection, stale binding hashes, bounded non-secret manifest and one independent frontend consumer; [design](../../../../design/0.110-fleet-runtime-contraction/0.110-design.md#op2-frontend-handoff-canic-008). |
| CANIC-002 | Host-first bounded Fleet observatory using protected status and terminal inventory. | Field ownership, freshness, failure isolation, partial-status truthfulness and measured cost; avoid an every-role renderer; [design](../../../../design/0.110-fleet-runtime-contraction/0.110-design.md#op3-host-first-fleet-observatory-canic-002). |
| CANIC-017 | Public developer host owns a persistent disposable multi-subnet Fleet and gateway. | Two-Root placement, browser discovery, restart/time/shutdown, bounded resource/cache use and explicit fidelity limits; [design](../../../../design/0.110-fleet-runtime-contraction/0.110-design.md#op4-persistent-multi-subnet-development-fleet-canic-017). |

That order keeps ordinary operator lifecycle ahead of frontend/observability
integration and the broader local process owner. The maintainer explicitly
scheduled these four in 0.110 on September 13. OP1–OP4 are implemented and
qualified; see the [local Fleet guide](../../../../features/operations/local-development-fleet.md)
for the final batch and packaging limitations. No new minor is needed. Role-specific memory initialization stays
parked under its separate instruction.

CANIC-165's Toko fixture conversion and CANIC-162's downstream allocation
measurements require downstream adoption/work. CANIC-166's live review and
recovery require exact environment/effect authority. They remain distinct from
Canic source completion and cannot be marked done by mutating the read-only
sibling or by this local test cleanup. The narrowed 139 and adoption records for
163/164/167 do not reopen implementation. The refreshed ledger ends at 169.

### CANIC-169 qualification

The real platform-to-policy regression reproduces the released
`InvalidProtocolStep("pool-reconcile-gllqn-eyk")` before the projection correction
and passes after it. It covers PendingReset, Failed, Ready, Claimed and Workload
at balances 900, 1,050 and 1,100 against an ordinary minimum of 1,000 and recovery
target of 1,100, with separately bounded observation/update burn. These fifteen
cases verify the exact funding amount, Root/lifecycle binding and unique owner,
including an asset above the ordinary minimum but below its recovery target.

The live regression exposed a second failure after the three Applied installs:
the existing planning branch still classified a paused Full reinstall as an
unfinished reset, returning its old plan. Planning now admits the existing next
review only after validating the complete journal, exact action hashes and
recorded install versions. It retains completed reinstall evidence before
replanning. Applying the paused reset reports `SuccessorReviewRequired` without
rechecking obsolete preparation seals or repeating installs. Incomplete, Issued,
wrong-action, wrong-operation and missing-version evidence fails the native
admission regression.

The governed PocketIC case
`pic::fleet_registry::baseline::tests::generated_mixed_topology_and_ready_reserve_recover_one_reviewed_operation`
passes in 760.95s (776s runner). It uses a disposable six-pool-asset Fleet,
spends the Hub's real cycles down to roughly 323.4B before reset, retains the
three Coordinator/Store/Root installation receipts across interruption and
retry, and reviews one Root/PendingReset funding action. A lost withdrawal reply
reconciles without a second paid withdrawal. A further review covers Store and
Component effects with zero additional debit. Both reviews keep the reset's
operation identity and contain no install/create actions. Final checks prove
exact operator debit, bounded native-cycle conservation, five Workloads and one
Ready reserve, wiped user rows/restored system rows, no repeated installs,
effect-free replay and a second explicit wipe with a new operation identity.

Qualification identities:

- Canic base: `8907442e028ebe44f47090783c39f64bcf940b04` (`v0.110.15`) plus
  the preserved open .16 worktree and this correction; no package version bump.
- Rust 1.98.1, PocketIC 16.0.0, native ICP 1.5.0; locked offline dependencies.
- Cargo.lock SHA-256:
  `d4a42ddbaf9a0b90099a3bbdeb90f65f5aa3d74cc1d78c5f88dd14a2999d1488`.
- Initial fixture build:
  `a4c128728412f11837b79ce8562e3115451db387e17361b79b4f15d02cbb36ae`;
  selected replacement:
  `33888cd229a38503c7d4d2ce3fd53c48e003dd5856473bef2165c5a67889010a`.
- Underfunded reset operation:
  `5ec71578d99b0925e5eba7031ee383ad901068d31e11e68737627e42f67e9478`;
  original reset plan:
  `804fc9e10e6a19996fac30b95fdb75d08124889eefbe9f3e3dc0514641e1dcc5`.
- Source inventory: `/tmp/canic169-source-qualified.json`, SHA-256
  `c597ef5a71f9892bb41798f9a4a446c6247845e695ea06bb3e8a445ecc9c95aa`;
  all 1,791 recorded source/build inputs were unchanged across the passing run.

Logs: `/tmp/canic169-reinstall-qualified.log`,
`/tmp/canic169-fleet-native.log` (199 passed, two existing ignored cases), and
`/tmp/canic169-clippy.log` (affected host/internal-testing/Hub libraries and tests).
Earlier live runs found the missing review handoff and an incorrect test input
for terminal replay; neither is counted as a passing full proof. A newly
reviewed funding plan requires its own reviewed input on terminal replay.

This is current Canic fixture qualification, not a replay of Toko's fourteen
retained pool assets or a live recovery of its recorded operation. Toko's source,
plan, journal, request and selected build remain untouched. The in-repository
.16 batch and changelogs are ready for the maintainer-selected release flow;
downstream adoption and exact live review/apply remain separate.

### CANIC-166 qualification boundaries

The maintained current-release reinstall journey passes in 666.84s (681s runner),
including selected replacement artifacts, controller rejection, a lost install
reply, full convergence, terminal conservation and effect-free replays. All
1,741 recorded Rust/TOML/Candid/shell/lock/Make inputs remained unchanged during
that run. Log: `/tmp/canic166-current-reinstall.log`; inventory:
`/tmp/canic166-current-runtime-source.json`. This run precedes the final private
completed-receipt projection; that later read-only boundary is qualified by the
source regressions and exact copied evidence below, not by an assertion that
the historical installed-activation journey passed.

The native source tests cover both 30-row variants (missing publication counters
alone and the missing completed-bootstrap fixture field), nonzero retained
attempt counts, valid Issued/Applied state substitutions, hash/identity drift,
effect-free rejection without live reset inventory and every durable adoption
handoff interruption. Logs: `/tmp/canic166-fleet-tests-final.log` and
`/tmp/canic166-source-entry-final.log`.

The local diagnostic probe uses the freshly built Canic host against a Canic
scratch copy of the actual Toko source: three operation documents, three
infrastructure Wasms and eleven referenced chunk objects. All 17 original and
copied files retain their exact SHA-256s. It returns
`RetainedActivationReviewRequired` for operation
`aed7d8545e4722930c1b500fde05b87c4ba07e4e63088ebbd54593d7b2ab8895`
and journal plan reference
`5714f60182f51fff270478361b9dafd96222c417cb084ccf67e2a5118b419407`.

| Original document | SHA-256 |
| --- | --- |
| Plan | `22618ec59cf19283ae62edb5808ddbe54bf22426b553045cae9f16fd0151d89b` |
| Journal | `46bf831e876bcd0d9134b529236e390c100f60f770c938179af5723422d2f91f` |
| State | `155a523c1bcd084e3bc35eff8704de6610494c1ceffdb9cfd1a17430202a958f` |

Probe log: `/tmp/canic166-retained-source-probe.log`; complete source digests:
`/tmp/canic166-toko-source-hashes.json`; isolated copy:
`.tmp/canic166-retained-source-review`. This is exact local source recognition,
not a live authority review or staging recovery result. No sibling edit, live
effect, version transaction or Git publication ran.

Final host all-feature library/test Clippy and the four source-entry regressions
pass after the completed-receipt correction. Layering, formatting, diff,
document-semantics and changelog checks pass. The complete in-repository .16
batch and changelog are ready for the governed release flow; the live recovery
boundary and historical qualification limits above remain explicit.

## Earlier September 11–12 disposition

Maintainer-selected Canic-only work. The downstream ledger was refreshed
read-only on 2026-09-12 and now ends at CANIC-167; section bodies were checked as well as the summary table.
Package versions remain 0.110.14 with an open 0.110.15 draft. Existing
CANIC-160/164 work is preserved. No sibling mutation or live Fleet mutation ran;
the later maintainer-selected observation-only assessment is recorded below.

## Disposition and sequence

| Feedback | Canic disposition | Next boundary |
| --- | --- | --- |
| CANIC-167 | Agreed and implemented: final build summaries show exact code/data bytes and profile; selected-role provenance retains final measurements for fast and release. | Publication and downstream adoption; no new limit or inferred executable headroom. |
| CANIC-166 | Agreed; ordinary resume needs an actionable diagnosis. The exact reported plan reference already belongs to CANIC-157's supported bounded partial-activation recovery. Current code now distinguishes an inspectable source from other unreadable evidence. | Use the existing explicit recovery review against fresh exact authority; staging admission/execution is not established by this local work. |
| CANIC-165 | Canic provisioning implementation qualified: actual Store import, later Shards, replacement grants, backoff, funding, direct retirement, held-reply recovery and complete generated Fleet apply/reinstall. The completed feature joins the .15 draft. | Publication and downstream conversion; matched size/import-cost receipts for Translation and Game Shard against compact embedding remain downstream work. |
| CANIC-163 | Implemented and qualified as the selected explicit-intent extension in .15: completed source authority is distinct from selected target artifacts. | Publication and Toko wrapper adoption; changed/identical builds, lost Root responses and completed-digest replay are covered by the [Canic proof](canic163-selected-build-reinstall.md). |
| CANIC-164 | Implemented and qualified in the existing .15 batch. | Publication, downstream adoption and live attribution remain separate. |
| CANIC-160 | Shared/concurrent observation corrections are qualified in .15. | Comparable real Toko deployment timing remains downstream evidence. |
| CANIC-161/162 | Existing diagnosis and protected/public observation improvements stand. No Canic-owned application loop or measured memory saving is established. | Application-loop evidence and fresh installed allocation attribution. |
| CANIC-087/139 | Maintainer accepted the narrowed boundary: exact unchanged-release reuse and verified intermediate caching. Existing no-op/extraction/first-build evidence stands. | Cross-release-identity finalized-Wasm reuse is dropped; release binding stays intact. No replacement runtime-identity design is needed for this feedback. |
| CANIC-141 | Real mixed-subnet fee limitation, previously explicitly deferred. | Keep behind the selected recovery/provisioning work; retain exact fee accounting. |
| Earlier Confirmed labels | Published fixes and retained evidence already cover 007/132/133/135/136/137. | Downstream status labels do not reopen implementation without a new failing case; see the [prior triage](../2026-09-10/toko-feedback-triage.md). |

CANIC-166's diagnostic is a bounded follow-up to a published operator recovery
path and extends the same .15 draft. CANIC-165 now joins that draft after its
complete generated apply/funding qualification; it remains a distinct product
extension in the changelog. No new minor is opened. Its
[design checkpoint](../../../working/canic165-fixture-provisioning/design.md)
records owners, identity, source access, consistency requirements and sequenced
proofs. CANIC-163 was implemented as its own complete operator batch after the
166 diagnostic and read-only staging review, then consolidated into the same
open .15 draft. Its source/target authority and runtime evidence remain explicit.

The maintainer relayed downstream steering later in this same session: narrow
139 as above, finish scoped validation, prioritize 163 next and keep 165
separate. CANIC-166's existing recovery review should be assessed read-only
before 163; staging does not depend on implementing 163. Any resulting review
still requires exact reset-scope and cycle-accounting assessment before apply.
That observation-only assessment succeeded using a Canic-owned scratch copy of
operation documents/artifacts and isolated ICP identity settings. The
[staging assessment](canic166-staging-review.md) records exact stop/stop/start
preparation scope, complete 24-asset inventory and the conservative 219T
execution allowance against 392.571T observed native cycles. No effect was
applied, all original/copied inputs remain unchanged, and temporary copied
identity material was removed. This admits a review, not a completed recovery.

## CANIC-166 source finding

Read-only inspection of the downstream retained files finds:

- a Full plan and InProgress journal with the same plan reference
  `5714f60182f51fff270478361b9dafd96222c417cb084ccf67e2a5118b419407`;
- an Applied prefix followed by one Issued effect, with no successor phases
  or funding reviews;
- omitted `recovery_review` and `reinstall` fields in the source plan; and
- zero declared new funding, operator debit, unavoidable fees and scheduled
  transfers in that source plan's conservation bounds.

These local observations agree with the source already investigated in
[CANIC-157](../2026-09-08/activation-feedback.md#installed-source-observations-2026-09-09).
They do not revalidate deployed controllers, cycle balances, installed modules,
callback settlement or complete physical inventory. No protected record is
republished here beyond the already recorded plan reference.

The existing source inspector treats source bytes and exact action hashes as
evidence. It does not deserialize a predecessor plan into executable current
authority. The existing explicit `--reinstall` branch bypasses ordinary input
resume, assesses that evidence, and requires fresh live recovery admission.
The existing installed-source PocketIC proof deliberately omits the same two
review fields and covers preservation, controller drift, stop/restart,
lost-install-response recovery, conservation and effect-free replay.

The new ordinary-plan diagnostic delegates to that inspector only after strict
read failure. It preserves the underlying error. An exact local match yields
`RetainedActivationReviewRequired` with operation, journal plan reference and
source-document digest; a failed inspection yields `RetainedPlanUnreadable`.
Neither outcome creates a replacement review, edits active documents, invokes
the platform, or weakens plan integrity. CLI reinstall help now describes both
supported review cases. The
[operator runbook](../../../../features/operations/fleet-ensure.md#unreadable-retained-plan)
explains the prerequisites and limits.

## Qualification

| Check | Result | Evidence |
| --- | --- | --- |
| Host unreadable/source evidence, plan integrity, retained desired and interrupted adoption | 5 pass; exact source bytes preserved, no provisioning mutation, bad evidence cannot request the supported review | [native log](canic166-evidence/native.log) |
| CLI Fleet authority selection, parsing and rendering | 15 pass | [CLI log](canic166-evidence/cli.log) |
| Recursive command ordering/example bound and bare-command help | 2 pass | [CLI log](canic166-evidence/cli.log) |
| Host/CLI library, binary and test Clippy, all features | Pass with warnings denied | [lint log](canic166-evidence/clippy.log) |
| Changed Rust formatting, whitespace, local document links, current-document semantics and .15 draft preflight | Pass; zero document-layout advisories | Local targeted checks |

The [qualification record](canic166-qualification.json) binds retained log
checksums and distinguishes the native diagnostic proof from live recovery.
The existing runtime recovery evidence is reused within its original scope;
this diagnostic change does not alter source inspection, recovery admission,
effect execution, journalling or conservation. No broad suite or new PocketIC
run is needed to claim the diagnostic behavior. Actual staging recovery remains
unperformed and CANIC-166 is not recorded as live-resolved.

The complete .15 CANIC-160/163/164/166 batch and changelog are ready for release
review. The [163 qualification](canic163-selected-build-reinstall.md) adds 26
focused host/CLI/help tests, affected-package Clippy and a successful 770.29-second
mixed-Fleet changed/identical-build proof. CANIC-165 implementation, Toko wrapper
adoption and live staging recovery remain separate. No version mutation,
publication or broad gate ran.


## Feedback refresh and CANIC-167 — 2026-09-12

Read-only inspection of Toko Miner's ledger, fixture-provisioning handoff and
September 12 scan finds one new issue, CANIC-167. Its reporting request is sound:
raw/gzip size does not identify executable code size, and the host already owns
section measurement. Current application pins remain independent of Canic's
unpublished worktree. No remote publication or live staging state was rechecked.

The final application, infrastructure and selected-role tables now include
`PROFILE`, `CODE (B)` and `DATA (B)`. The existing selected-role provenance path
records required `payload.final_wasm_metrics` for raw/gzip, code/data and defined
functions. Both surfaces reuse the finalization owner's parser against emitted
artifacts; they do not depend on an optional optimizer result. Exact profile,
artifact hashes, release identity and finalization remain with their existing
owners. The current v1 record hard-cuts without missing-field defaults. No new
headroom projection or installation ceiling is introduced.

The refreshed CANIC-165 handoff correctly keeps complete generated deployment
separate from consumer qualification. Its funding/backoff and retirement list
lags newer local evidence; those results retain their original scope rather
than reopening already qualified work. The design now records matched downstream
code/data/custom/raw/gzip, row/read equality and import/final-validation cost
receipts for Translation and Game Shard, using current compact embedding as the
baseline. Both exact suspended-effect cases and generated apply/funding remain
Canic work. No Toko importer or IcyDB source was changed. CANIC-166 still needs
fresh reviewed staging recovery execution, without another local recovery owner.

The [CANIC-167 check record](canic167-build-metrics.json) binds the targeted host,
CLI and Clippy logs and changed source digests. Native examples independently
vary code and data, cover both profiles, match table/evidence counts to exact
section payloads and artifact sizes/hashes, and preserve input bytes. Malformed
Wasm and absent gzip fail measurement. This qualifies reporting, not a measured
Toko size reduction, runtime import cost or production deployment.


Qualification passes: 21 focused host tests, 26 CLI build tests and affected
host/CLI library, binary and test Clippy with warnings denied. Formatting,
whitespace, document semantics, local links and the .15 draft preflight pass.
Final native checks preceded only the const/visibility lint cleanup; the final
Clippy result binds that source. The CANIC-167 reporting batch and .15 changelog
surfaces are ready for release review. CANIC-165 remains an incomplete separate
batch, and the combined worktree is not ready to publish as complete provisioning.
