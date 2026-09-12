# Toko Miner upstream feedback — 2026-09-11

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
| CANIC-165 | Separate provisioning extension in progress. Actual Store import, later Shards, replacement grants, backoff, automatic funding and direct retirement are qualified within their recorded scope. | Exact Root grant/revoke await and held actual-Store reply interruption, then complete generated Fleet apply/funding. Downstream matched size/import-cost receipts cover both Translation and Game Shard against compact embedding. |
| CANIC-163 | Implemented and qualified as the selected explicit-intent extension in .15: completed source authority is distinct from selected target artifacts. | Publication and Toko wrapper adoption; changed/identical builds, lost Root responses and completed-digest replay are covered by the [Canic proof](canic163-selected-build-reinstall.md). |
| CANIC-164 | Implemented and qualified in the existing .15 batch. | Publication, downstream adoption and live attribution remain separate. |
| CANIC-160 | Shared/concurrent observation corrections are qualified in .15. | Comparable real Toko deployment timing remains downstream evidence. |
| CANIC-161/162 | Existing diagnosis and protected/public observation improvements stand. No Canic-owned application loop or measured memory saving is established. | Application-loop evidence and fresh installed allocation attribution. |
| CANIC-087/139 | Maintainer accepted the narrowed boundary: exact unchanged-release reuse and verified intermediate caching. Existing no-op/extraction/first-build evidence stands. | Cross-release-identity finalized-Wasm reuse is dropped; release binding stays intact. No replacement runtime-identity design is needed for this feedback. |
| CANIC-141 | Real mixed-subnet fee limitation, previously explicitly deferred. | Keep behind the selected recovery/provisioning work; retain exact fee accounting. |
| Earlier Confirmed labels | Published fixes and retained evidence already cover 007/132/133/135/136/137. | Downstream status labels do not reopen implementation without a new failing case; see the [prior triage](../2026-09-10/toko-feedback-triage.md). |

CANIC-166's diagnostic is a bounded follow-up to a published operator recovery
path and extends the same .15 draft. CANIC-165 is not folded into that release
as an unfinished feature, and no new minor is opened. Its
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
