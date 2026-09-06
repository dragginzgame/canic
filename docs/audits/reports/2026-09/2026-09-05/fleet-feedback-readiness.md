# Fleet feedback correction readiness

The accepted `CANIC-007` and `CANIC-132`–`CANIC-138` correction batch is
**ready for release approval**. Its bounded outcome is reviewed Fleet launch,
funding, recovery and explicit pre-1.0 reinstall through the existing Ensure
owner. Whole-Fleet evacuation applies only when deleting the Fleet and is not
a release prerequisite for this batch. No source defect remains identified in
that accepted scope.

Custody, the pinned predecessor owner, compatibility readers and superseded
Store-adoption plan handling are removed. The final generated changed-release
journey reaches a working Fleet after a lost install response and passes both
full-plan replay checks. Package versions remain 0.110.7; the open patch draft
is 0.110.8. Release approval and the maintainer-selected governed release gate,
versioning and publication remain outstanding.

## Closeout handoff

- **Changed:** complete initial supply and bounded Ensure continuation; shared
  funding-policy admission; exact native/Ledger accounting and repair; selected
  authorization storage; explicit management-bound Root reinstall followed by
  a reviewed full plan. Runtime data may be discarded, while cycles and asset
  control remain accounted for.
- **Final evidence:** 138 Fleet host tests, 47 selected Fleet CLI tests, five
  canonical Coordinator Candid tests and scoped warning-denied Clippy pass.
  The generated changed-release PocketIC journey passes in 522.04s. The final
  Coordinator fee-preflight/Ledger lost-reply regression passes in 142.74s.
  Earlier supply, funding, authorization and performance proofs are retained
  below with their checkpoint boundaries.
- **Propagation:** canonical Candid includes required Ledger receipts and the
  maintained command/status variants. Plan JSON includes the exact current
  scope and reinstall bindings. CLI progress and reports distinguish a
  completed prerequisite from a terminal full Fleet. Operational docs and the
  open changelog describe that same contract.
- **Evidence applicability:** the final host policy/observation and fixture
  changes precede the final host, CLI, Clippy and PocketIC passes. No DTO or
  Candid change followed the five conformance passes. Initial closeout changed
  only documentation and evidence. A subsequent explicit `HashSet::default()`
  correction in the conformance test passes
  [targeted all-feature Clippy](artifacts/fleet-reinstall/protocol-surface-clippy.log).
  Its comparison behavior is unchanged; no PocketIC rerun is required.
  The subsequent release attempt exposed two additional test-target lints:
  oversized JSON fixture setup and late PocketIC fixture disposal. Both are
  corrected without changing assertions or IC calls.
  [Combined affected-package Clippy](artifacts/fleet-reinstall/affected-package-clippy.log)
  passes for all targets/features of `canic`, `canic-host` and `canic-tests`,
  with warnings denied and `--keep-going`.
  The [changed JSON regression](artifacts/fleet-reinstall/json-report-test.log)
  also passes. The fixture-disposal-only edit does not require repeating the
  existing PocketIC behavior proof. Earlier Clippy selection had checked
  dependency libraries without their test targets; the
  [validation procedure](../../../../governance/ci-deployment.md) now requires
  explicit affected-target coverage before closeout. The full release gate
  remains outstanding.
  [Check records and selected source digests](artifacts/fleet-reinstall/checks.json)
  bind the retained logs and capture the reviewed source for subsequent changes;
  they are not an immutable release-validation receipt.

| Classification | Remaining action |
| --- | --- |
| Release blockers within this correction batch | None identified. The implementation, affected evidence and propagation are complete. |
| Release process | Maintainer approval and the governed release gate/version/publication flow. No broad gate was pre-run during coding. |
| Downstream qualification | Toko must adopt the immutable release and rehearse its real App, configuration and controller topology before live use. |
| Optional follow-up | Complete Coordinator native evacuation/deletion and whole-Fleet terminal conservation, needed only for whole-Fleet deletion; CANIC-139 whole-release build reuse; further deployment-performance work. |
| Separate accepted work | B1 footprint qualification and human acceptance retain their own tracker. This correction does not accept B1, close 0.110 or begin 0.111. |

The subsequent release-test correction updates the reservation hash vector for
the current `asset_recipient` field and verifies that redirecting the recipient
changes the hash. Config discovery validates all discovered files, uniqueness
and required fixtures without a fixed aggregate count. The Coordinator policy
manifest now declares the existing `Retire` operation's replay and value-transfer
guards; its workflow, persisted transfer and receipt contracts are unchanged.
All [32 targeted core regressions](artifacts/fleet-reinstall/core-contract-tests.log)
pass, and [core library/test Clippy](artifacts/fleet-reinstall/core-contract-clippy.log)
passes with all features and warnings denied. These final-source checks resolve
the three reported `canic-core --lib` failures; the complete release gate still
needs to finish.

A later test-only correction removes a wall-clock second-boundary race in the
placement-index recovery assertion. The first panic poisoned the shared seam
lock, causing eleven secondary failures. The test now checks the returned time
against observed start/end bounds and retains exact asset and removal checks.
All [64 affected placement/auth/intent tests](artifacts/fleet-reinstall/core-timing-tests.log)
and [all-feature core library/test Clippy](artifacts/fleet-reinstall/core-timing-clippy.log)
pass. Production source and the shared test-lock policy are unchanged.

The subsequent feature-documentation and Ledger cost-guard failures are also
corrected. The facade README documents the current opt-in local application
authorization feature. The guard inspects `create_canister` and `transfer`
signatures for mandatory `&CostGuardPermit` arguments instead of counting
permit spellings across their shared file. Both
[affected integration targets](artifacts/fleet-reinstall/integration-contract-tests.log)
pass (14 tests), and the changed
[cost-guard target passes all-feature Clippy](artifacts/fleet-reinstall/cost-guard-clippy.log).
The broader Clippy attempt was interrupted by removal of shared Cargo artifacts;
the focused retry passed. Runtime and package manifests are unchanged.

## Toko Miner adoption checks

1. Pin the published Canic release, build a sealed current App release, and
   reconcile staging policy with the generator's complete Workload count plus
   its independent Ready reserve. Canic's retained topology proof covers
   nineteen Workloads plus five Ready assets; Toko must validate its own current
   configuration and limits.
2. Verify the exact selected environment, operator, infrastructure controllers,
   Root-owned assets, cycle accounts and generated seed. Start a current-schema
   host operation; old release journals and application state are not migrated.
3. Rehearse generation, review/apply of `root_reinstall_prerequisite`, then
   review/apply of the subsequent `full` plan. A terminal prerequisite is not
   application readiness. Confirm all declared Components and descendants,
   application initialization, required authorization capabilities and Ready
   reserve reach the expected state.
4. Interrupt and resume the same operation with its exact retained digest.
   Confirm no duplicate paid effects or reinstalls, account for native and
   Ledger balances, and verify original-plan and newly planned replay issue
   zero effects. Validate application behavior after its intentional state reset.

The local fixtures qualify their exact controller and topology contracts, not
arbitrary installations or mainnet timing. Full deletion of a retained Toko
installation requires separate applicability and conservation evidence if it
is requested; it does not block reinstall adoption. Toko remains read-only in
this Canic session.

## Completed focused evidence

The completion audit checks the refreshed downstream criteria by outcome.
Publication, downstream adoption and live effects remain outside this
in-repository goal; they are not represented as completed by these proofs.

| Requirement group | Current evidence and disposition |
| --- | --- |
| CANIC-007 1–5 | Exact plan-owned Root Ledger authority and native/Ledger accounting; the four-Workload refill crosses lost transfer and creation replies, terminal conservation and effect-free replay. Application allocation still requires the trusted operator/Fleet review boundary. |
| CANIC-132 1–4 | Protected creation margin, first-observed balances and exact creation receipts; four Workloads refill four Ready assets at the 1.9T floor, with bounded repair of existing Failed assets qualified separately. |
| CANIC-133 1–5 | Complete protected lifecycle inventory and pending creation own capacity exactly once; typed pre-effect rejection and full-capacity four-Failed repair are covered by focused policy and production-host proofs. Repair authorizes existing assets and does not invent free capacity or replacement creates. |
| CANIC-134 1–5 | Complete for the accepted reinstall correction: generation, reviewed reset, lost-response recovery, working Fleet reconstruction, conservation and both effect-free replays pass. Custody and predecessor readers are removed. Complete whole-Fleet deletion is optional follow-up under the maintainer's final scope. |
| CANIC-134 6 | Downstream disposable rehearsal is required before live supersession. No Canic fixture is claimed as that adoption evidence. |
| CANIC-135 / CANIC-136 | Synchronous selected-store restoration, corruption rejection, enabled/disabled Root contracts, native delegation recovery, Candid and focused artifact evidence are retained. The runtime source trees matched the controlled measurement copy at that checkpoint; this is not B1 acceptance or a broad B2/B3 gate. |
| CANIC-137 1–3 | Generation and Ensure invoke the runtime funding authority; Medic delegates to Ensure's same validator. The four/five-grant boundary is covered by runtime and host tests. |
| CANIC-137 4 | Relevant Canic source qualification is retained; managed downstream adoption remains downstream work. |
| CANIC-138 1–6 | Structural capacity admission, complete up-front 5+5 and 19+5 supply, bounded reconciliation and successor phases, progress events, lost replies and both replay checks pass. Up-front complete supply is the explicitly permitted fresh flow; local autonomous refill is not added. |
| CANIC-138 7 | The complete controlled five-plus-five comparison passes in both modes: apply-to-readiness falls 27.86%, and apply through both replay checks falls 33.47%, with the same 53 effects, conservation and recovery assertions. See the [measurement boundary and raw evidence](fleet-feedback-performance.md). This is one local pair, not a mainnet latency guarantee. |

| Outcome | Evidence |
| --- | --- |
| Nineteen Workloads plus five Ready assets use the actual generator | Current-schema production-adapter journey: 1092.56s; complete recursive supply, original-plan continuation, lost replies, terminal conservation and both replay checks; predates the separately qualified Start/incident cuts |
| Five Workloads plus five Ready assets converge through one reviewed operation | Latest candidate production-adapter journey: 564.53s; lost controller/reset replies, terminal conservation and immediate effect-free replay |
| Complete five-plus-five observation-reuse comparison | Candidate 564.53s and disabled-reuse baseline 826.02s, both passing; measured apply-to-readiness 422.149s versus 585.206s, excluding setup |
| Four active Workloads refill four Ready reserves | Production-adapter journey: 377.41s; 1.9T floor, exact fee rejection, lost transfer/creation replies, four unique creation receipts and replay |
| Four Workloads plus four Failed assets repair at maximum eight | Production-adapter journey: 349.17s; reviewed native withdrawals and Root resets, lost replies, no Root-account transfer or additional creation, all four Ready and replay |
| Underfunded imports recover without indefinite balance polling | Production-adapter journey: 365.44s; reviewed native balance bounds, lost withdrawal/reset replies, terminal conservation and replay |
| Selected issuer storage restores before readiness | Seven governed native authorization/delegation cases pass; corrupt ID 66 fails lifecycle restoration and valid proof survives same-release restoration |
| Auth-free Root excludes delegation | Real Root activation and replay pass; controlled copied-Toko-Root measurement removes 333,288 code-section bytes (5.15%) |
| Protected funding policy has one semantic owner | Generation, Medic and Ensure use runtime admission; focused tests accept four grants and reject five |

The first generated journey reached terminal inventory but rejected the
fixture's relocated config because copying only its TOML lost relative App
package paths. The fixture now retains the complete unchanged source layout
under its isolated workspace and validates every declared package and role
contract before host effects. Scoped Clippy passes and the complete rerun
above qualifies that correction; production authority checks remain intact.

The reserve-only terminal-accounting defect was reproduced after all four
reserves became Ready. The corrected accounting binds each creation to the
reviewed Root budget and exact protected receipt without demanding an
unrelated host provisioning action. The current 138 focused Fleet host tests and
scoped warning-denied Clippy pass.

The auth measurement's named optimized companions do not have byte-identical
executable sections to the canonical Wasms. Canonical sizes and hashes are
reported separately; this is not full B1/B2 acceptance. Downstream adoption,
disposable deployment qualification and immutable publication are outstanding.

The final focused contract checkpoint passes 26 role-contract tests, four
runtime funding-policy tests and 39 Medic library tests. Exact commands are in
the [focused check record](artifacts/feedback-completion/checks.json). These
checks predate the new retirement changes.

## CANIC-134: qualified reinstall and optional deletion follow-up

Explicit reinstall must discard state while retaining cycle and asset control,
without old endpoint readers or a temporary artifact. Full evacuation is only
for deleting the Fleet. Generation now admits a changed Root module through
management observations, and Ensure compiles its reviewed reset prerequisite.
A subsequent full plan owns reconstruction and bounded protocol convergence.
Pool-policy drift and status errors no longer select implicit resets.
The canonical Coordinator Candid has been refreshed from Rust declarations.

The complete [generated changed-release journey](fleet-reinstall-journey.md)
now passes in 522.04s. Its three-action Root reset survives an actual lost install
response and adapter reconstruction. A subsequent reviewed full plan completes
18 effects across bounded phases, reaches one Workload and one Ready asset,
preserves the 1B Root Ledger balance and exact controllers, bounds native debit,
and proves both original-plan and newly planned effect-free replay. No previous
host records, old endpoint reader or temporary Wasm are used.

The [Root management reinstall checkpoint](artifacts/fleet-retirement/root-reinstall-pocketic.log)
passes in 347.63s including cold artifact builds. An active disposable Root gets
fresh Fleet initialization authority; its Principal, controllers, ten pool
assets, Store control and 1B Ledger balance remain controlled. No Ledger transfer
occurs and native debit remains below the test's 5T bound. This uses the same
Root artifact with fresh initialization; it does not qualify a changed-release
host journey, complete Fleet reconstruction, or host interruption/replay.

The current Root/Store retirement already returns excess native cycles to the
Coordinator. Root deletion preparation now has an in-progress Ledger transfer
intent and receipt, owned by that same operation. Optional whole-Fleet deletion work includes
complete Coordinator native evacuation/deletion and whole-Fleet terminal
conservation. Returned pool assets remain under exact operator control.
The Ledger receipt is now carried in the Coordinator readiness request and
Root terminal preparation record. The focused [Root model regression](artifacts/fleet-retirement/root-model.log)
and [Coordinator authority regression](artifacts/fleet-retirement/coordinator-authority.log)
pass, covering restoration, exact transfer identity, receipt replay and rejection
of a receipt for another destination. The [Root Ledger lost-reply checkpoint](artifacts/fleet-retirement/root-pocketic.log)
passes: one transfer empties the Root account and retains the exact receipt.
The final [Coordinator preflight checkpoint](artifacts/fleet-retirement/coordinator-preflight-pocketic.log)
passes in 142.74s: a rejected fee bound leaves the valid request usable, exactly
two transfers evacuate Root then Coordinator Ledger accounts, both lost replies
reconcile, replay is effect-free, and returned assets remain controlled after
Root deletion. The Coordinator's native balance and final deletion are outside
that proof. There is no separate custody artifact, upgrade command or
old-protocol owner.

Current implementation cannot retroactively add retirement capability to the
installed 0.110.6 modules. Toko live applicability remains unknown, and downstream
disposable rehearsal remains necessary before live effects. This deletion
limitation is not a reinstall prohibition. Current Fleet generation and Ensure
remain the deployment owner.

See the [hard-cut design disposition](fleet-hard-cut-review.md). Historical
custody logs below are retained only as evidence of the removed candidate.

## Retained evidence

- [Genuine unmodified predecessor retirement](artifacts/feedback-activation/canic134-predecessor-removal.log)
- [Named-operator predecessor retirement and custody](artifacts/feedback-activation/canic134-predecessor-custody-operator.log)
- [Historical test-only extension](artifacts/feedback-activation/canic134-predecessor-custody-probe.rs)
- [Historical removed-custody recovery and corruption](artifacts/feedback-activation/canic134-custody-pocketic.log)
- [Custody model boundaries](artifacts/feedback-activation/canic134-custody-model-tests.log)
- [Updated Fleet host regressions](artifacts/feedback-activation/canic134-host-regressions.log)
- [Custody and affected-package Clippy](artifacts/feedback-activation/canic134-final-clippy.log)
- [Generated nineteen-plus-five](artifacts/feedback-activation/generated-nineteen-recovery.log)
- [Focused Fleet host regressions](artifacts/feedback-activation/fleet-host-regressions.log)
- [Native authorization/delegation recovery](artifacts/feedback-activation/native-auth-recovery.log)
- [Scoped warning-denied Clippy](artifacts/feedback-activation/generated-package-clippy.log)
- [Automatic five-plus-five](artifacts/feedback-activation/automatic-five-plus-five.log)
- [Four-Workload reserve refill](artifacts/feedback-activation/four-workload-reserve-recovery.log)
- [Four-Failed reserve repair](artifacts/feedback-activation/four-failed-reserve-repair.log)
- [Funded import recovery](artifacts/feedback-activation/funded-failed-pool-recovery.log)
- [Reserve accounting failure before correction](artifacts/feedback-activation/four-workload-reserve-accounting-before.log)
- [Generated package-location failure](artifacts/feedback-activation/generated-package-location-before.log)
- [Controlled auth measurement](auth-feedback-toko-root.md)
- [Genuine predecessor artifact verification](artifacts/feedback-activation/predecessor-artifacts.json)

The predecessor and custody artifacts above describe removed candidates only.
Current reinstall qualification uses Canic-owned fixtures with no predecessor
reader. Retained Toko artifacts are source-audit evidence and do not become a
downstream-owned dependency of Canic's release gate.
