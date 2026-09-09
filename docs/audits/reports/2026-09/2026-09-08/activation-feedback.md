# Activation and observation feedback: CANIC-157–160

The maintainer accepted review and implementation on 2026-09-08, explicitly
asking to avoid architectural drift. Work extends the open 0.110.13 draft;
package versions remain 0.110.12. Earlier OD1/RI1/RF2 changes are preserved.

## Sanity review and scope

| Report | Finding | Bounded disposition |
| --- | --- | --- |
| 157 | Root/Store install IDs derive from the operation; module equality does not establish matching activation authority. Root prerequisites currently finish independently, and deliberate reinstall requires terminal convergence. | Check exact activation authority before dependent effects; support reviewed recovery of the reported partially activated estate through existing Ensure/reset owners. Preserve or reconcile issued effects before supersession. |
| 158 | Root's existing private provisioning callback reschedules every error after one second. Host stall detection does not bound that callback. | Retain bounded attempt/backoff state in the existing operation and classify at the failure owner. Suspend active retries requiring review; preserve uncertain-effect reconciliation. |
| 159 | Protected Root provisioning status omits the originating activation failure; the outer diagnostic can hide a nested rejection. | Retain one bounded typed failure, with exact stage/target/operation and retry category; carry it through existing protected status and host output. Diagnostic timestamps must not count as progress. |
| 160 | Configured observations are serial, but a scoped snapshot and bounded terminal-inventory reads already exist. The reported timings do not attribute cost. | Measure stages and remote calls first. Reuse existing observation/concurrency ownership for any demonstrated improvement, preserving freshness and deterministic failure precedence. |

These are operational correctness and observability corrections. They do not
authorize compatibility generations, authority migration, another scheduler,
another inventory/cache, a recovery command family, public internal errors,
unbounded failure history, or preservation of application rows across a reset.
Unknown transport results must not be classified as permanent identity conflicts.

The installed-release recovery boundary is material: 0.110.12 gates authority
snapshot commands/status while Root is Prepared, and its active private retry
claim is not a quiescent snapshot. Merely relaxing the host's Converged check or
adding an endpoint to a replacement Wasm cannot qualify that repair. The exact
in-progress failure must be exercised against immutable installed surfaces;
new-runtime retry/diagnostic evidence is a separate qualification.

## Delivery sequence and evidence

One activation correction covers 157–159 across runtime, control plane, host,
CLI, current Candid and the existing recovery fixtures. A separate measurement
slice covers 160 and must report measured limits rather than promise a speedup.
Both stay in the accepted open patch. Required evidence includes exact mismatch
rejection, transient recovery, bounded permanent failures, lost-response safety,
reviewed reset from partial activation, conservation, complete readiness,
effect-free replay, protected access and observation freshness/error ordering.

## Implemented behavior and qualification

The host retains prerequisite authority across desired changes and checks the
exact Root operation before a dependent Store installation. A reviewed plan
that also reinstalls that exact Root supplies the coherent closure instead.
Native cases cover desired-refresh rejection, refusal of a different Root as
the witness, and admission of the exact reviewed closure. These are preventive
checks; they do not qualify the installed partial-state repair below.

Root retains one typed latest failure in its existing provisioning record.
Retries use exponential delays capped at 60 seconds; a decoded rejection of
the exact Store activation binding requires review. Unknown transport results
continue bounded retries without changing issued receipts. Work progress
clears backoff; changing diagnostics does not count as progress. A stale callback
cannot replace newer failure evidence, clear a newer permanent failure, or
create another retry chain after another callback has recorded its result.

Protected status preserves catalog/activation stage, target, operation,
diagnostic, retry category and Root's original failure time through Coordinator
status and host output. The public error contract remains bounded. Failed
Store catalog verification is attributed before the later activation query;
these are distinct possible origins, rather than an assumption that every
preparation failure comes from the activation query.

Final working-tree evidence:

| Check | Result | Log |
| --- | --- | --- |
| Core/control-plane provisioning | 37 core and 26 control-plane cases pass, including stable-state round-trip, delay cap, receipt invariance, exact replay and diagnostic-time separation | `/tmp/canic158-159-native5.log` |
| Host/CLI Fleet regressions | 171 host and 52 CLI cases pass; two existing PocketIC cases remain ignored in the native tier | `/tmp/canic157-160-host-cli3.log` |
| Transient Store failure, final fixture | PASS, 25.57s test / 26s runner; catalog origin, protected access, bounded retry intervals, retained receipt hash, same-operation recovery, terminal readiness and effect-free replay | `/tmp/canic158-159-transient5.log` |
| Permanent Store identity mismatch | PASS, 22.36s test / 33s runner; Root expects A while Store retains B, exact origin survives Coordinator projection, acceptance replay and 20 simulated minutes leave failure/receipts unchanged and Root inactive | `/tmp/canic158-159-permanent4.log` |
| Six affected packages, all targets/features, warning-denied Clippy | PASS; the final fixture-only change also passes its owning package's all-target/all-feature Clippy | `/tmp/canic157-160-clippy6.log`, `/tmp/canic157-160-clippy7.log` |
| Canonical Coordinator Candid regeneration | PASS from the compiled profile, including original failure timestamp and catalog stage | `/tmp/canic159-candid6.log` |
| Protocol and changelog contracts | 36 protocol cases and changelog governance pass; the feature-enabled Coordinator read-contract equality case also passes | `/tmp/canic157-160-contracts.log`, `/tmp/canic159-coordinator-contract.log` |

Formatting, whitespace and the current-document guard pass. No broad workspace
or release gate was run. These are working-tree checks, not a published
release-validation receipt.

The permanent fixture installs the two conflicting identities directly. Its
earlier attempt to reinstall Root immediately hit replica install-code limits
before reaching the assertion; it is not recovery evidence. Earlier transient
runs exposed premature injection and missing catalog attribution. Only the final
passing cases above qualify the maintained behavior. Earlier RI1/RF2 paid-effect
lost-response and conservation evidence remains scoped to those journeys; the
new activation tests do not replace an actual partial-estate reset proof.

## Observation measurements

The paired configured-observation fixture holds three infrastructure reads at
50 ms per logical remote attempt and varies the retained pool size. Both sides
use the existing scoped snapshot. Durations include fixture/process overhead.

| Retained pool assets | Serial before | Bounded after | Remote attempts, before/after |
| --- | ---: | ---: | ---: |
| 0 | 163 ms | 55 ms | 3 / 3 |
| 8 | 221 ms | 114 ms | 4 / 4 |
| 24 | 220 ms | 111 ms | 4 / 4 |
| 96 | 220 ms | 111 ms | 4 / 4 |

Logs: `/tmp/canic160-before.log`, `/tmp/canic160-after2.log`. The 24-asset case
improves this stage by about 49.5%; it does not establish whole-plan or mainnet
speedup. Flat call counts confirm the existing pool snapshot already amortizes
repeated reads. The implementation reuses the bounded collector with at most
four independent reads, drains issued batches on error, preserves configured
error precedence, and expires snapshots on success and failure. CLI timing
events report stage, elapsed time, logical remote attempts and success on stderr.

## CANIC-157 installed recovery qualification

The reported in-progress 0.110.12 estate cannot use the converged same-release
wipe: its journal is not Converged, and its Prepared Root rejects the existing
authority-snapshot command/status lane. A private retry claim also prevents
treating the source as quiescent. Adding an endpoint to new Wasm does not alter
those installed capabilities. Same-Wasm response-loss recovery additionally
needs an exact deployment witness; a canister-version increase can reflect
another management change. The existing new-runtime witness is not available
on that installed Root.

The current plan contract also has newly required reset/recovery fields. The
old InProgress journal shape itself is not the incompatibility; old plans and
their hashed actions cannot simply be relabeled as current reviewed authority.
Source inspection hashes the retained document bytes and checks the issued
protocol prefix without decoding a predecessor plan into executable current
authority. The old plan hash is a journal reference, not a verified current
plan digest. The implementation below archives the truthful InProgress source
and requires a real corrected Root module; its installed-release proof remains
outstanding.

### Installed source observations, 2026-09-09

Read-only protected queries using the existing `toko-miner-mainnet` identity
and explicit `https://icp.net` network confirmed that the installed Root's
Component Registry and pool status remain readable while Prepared. The source
operation is `aed7d8545e4722930c1b500fde05b87c4ba07e4e63088ebbd54593d7b2ab8895`;
`5714f60182f51fff270478361b9dafd96222c417cb084ccf67e2a5118b419407` is its
retained plan hash, not its operation identity. The journal has an Applied
protocol prefix, an Issued provisioning action, and an unissued readiness tail.
The infrastructure action lists are empty.

Root `2ydug-eaaaa-aaaab-qhfca-cai` reports a sealed initial inventory with three
committed Components, six managed descendants, nine known created canisters,
zero reservations, and inactive Root runtime. Its activation identity is
`a1847d70363e13f3ec14e638bd137feab5f302653edcba049bb382b2f285915a` and inventory
hash is `f80800239de7740fdca5e11e72397ff1d6723ebfbba014c2d0513e4d7830b248`.
The complete pool page reports one Store and 24 non-Store assets: nine Workload
and 15 Ready. Its physical maximum is 24 and minimum Ready reserve is five;
creation, handoff, reset, claimed, recycling and failed work are absent from
that page. These counts describe Toko's observed estate, not the neutral
19-Workload/five-Ready qualification fixture.

The raw observations are retained locally in
`/tmp/canic157-installed-root-registry.json` and
`/tmp/canic157-installed-root-pool.json`. These are protected, uncertified
queries. They do not establish management controller sets, fresh asset cycle
balances, callback settlement or reset authority. No update call, payment,
stop, reset or sibling edit was performed against this deployment.

### Settlement proof and capacity correction

Inspection of immutable `v0.110.12` found that refill checks capacity before
awaiting the Ledger fee, while intent admission did not recheck it. An older
callback can therefore resume after other work fills the physical pool. The
current source now checks capacity atomically when admitting a new creation
intent; exact replay retains its already reserved slot.

A completed `MaintainPool` response is insufficient as a settlement barrier:
the five-minute maintenance lease permits takeover without cancelling the
older callback. Do not add a maintenance action whose success is treated as
proof that every earlier callback finished.

Targeted evidence for this follow-up:

| Boundary | Result | Local log |
| --- | --- | --- |
| Pool capacity, reservation replay and existing refill/reset behavior | 24 all-feature control-plane pool cases pass | `/tmp/canic157-pool-capacity.log` |
| Source-document integrity, refusal without source mutation, pool inventory completeness, existing Fleet behavior and explicit reset input selection | 173 all-feature host and 52 CLI Fleet cases pass; two existing PocketIC cases remain ignored in the native tier | `/tmp/canic157-source-host-cli.log` |
| Affected-package hygiene | Control-plane and host all-target/all-feature Clippy pass; the CLI test-length annotation is corrected and its all-target/all-feature Clippy passes. Formatting, diff whitespace and current-document semantics pass. | `/tmp/canic157-source-clippy2.log`, `/tmp/canic157-source-clippy3.log` |

The source observer now participates in a bounded, single-Root recovery through
the existing Ensure owner. Preview retains a separate review without replacing
the active source documents. Apply checks the reviewed digest and fresh
authority, archives exact plan/journal/state bytes, commits local adoption intent,
and finishes both file replacements before remote effects. Recovery accepts only
the exact before/after document combinations; a completed adoption cannot roll
back subsequent journal progress. The source journal remains an archived
InProgress record, never a manufactured Converged record.

Admission binds source artifact hashes to retained topology and actual
management observations, exact controllers and paid inventory, the same
operator/Ledger, unchanged Root Ledger balances and source native debit within
the retained bound. Source documents are opaque evidence, not executable old
plans. Three separate reviews cover stop/restart preparation, corrected Root
reinstall and remaining Full Ensure convergence. The preparation's completed
current documents are retained by exact hash before the Root-reset review.
Ordinary continuation, initializers, install-version reconciliation, physical
imports and the existing current Root history witness own the later effects.

The completed preparation uses actual management stops of Coordinator and Root,
then restarts the unchanged Root for fresh inspection. The IC management stop
boundary waits for outstanding responses before reporting Stopped
([management canister reference](https://docs.internetcomputer.org/references/management-canister/)).
The proof exercises that boundary on the affected runtime and requires complete
fresh inventory before the separate Root-reset review. Changed membership,
pending paid work, missing authority or a stop timeout prevents reset. A full-pool
query alone never supplies settlement evidence. Multi-Root partial recovery is
refused; cross-Root settlement ordering is outside this bounded repair.

### Final installed-source proof

`installed_partial_activation_recovers_through_reviewed_reinstall` builds the
controlled source roles from immutable commit
`95c6caa03867cd68b3bc57ba88c02b961251460b` (`v0.110.12`) in the Canic-owned
`.tmp/canic157-installed-source`. Before and after artifact construction, all
3,738 archived source files match that commit. The local integrity record is
`/tmp/canic157-source-integrity.json`; its artifact map records the files present
in the qualification workspace, including builder support artifacts.

The test installs the old Root with activation A and actually reinstalls Store
with B. It executes the source protocol prefix, retains an Issued provisioning
effect, and observes Published Root with activated Components but inactive Root
runtime. The application-neutral estate has one Workload and one Ready asset.
The source plan deliberately omits newer review fields and remains opaque
source evidence; no Converged source journal is manufactured.

The production recovery then proves:

- preview preserves source plan/journal bytes; controller drift rejects before
  their replacement, and restoring exact authority permits the same review;
- actual Coordinator/Root stops, unchanged-Root restart, fresh complete inventory,
  preparation conservation and effect-free preparation replay;
- a separate corrected-source Root reinstall, an actual lost install response,
  fresh adapter reconstruction, exact reconciliation and effect-free replay;
- a separate Full Ensure review completes infrastructure, imports and control
  plane through the existing continuation owner (19 effects), reaches readiness,
  retains exactly both pool assets and their Root-only controllers, bounds total
  native debit and preserves Root/operator Ledger accounts; and
- immediate Full-plan replay performs no effects.

The proof found a host defect before the remaining Coordinator reset: same-Wasm
history reconciliation required the requested module even before a changed-Wasm
install. The reviewed witness now retains the prior module too. Unchanged prior
module plus no later deployment permits execution; exact requested deployment
proves completion; unrelated module/history remains a typed conflict. Three
native history regressions cover the maintained distinctions. Root-reset review
also reports its whole-estate allowance and uses the same fresh observation for
admission and initial conservation.

| Final boundary | Result | Local evidence |
| --- | --- | --- |
| Installed 0.110.12 partial-activation recovery | PASS, 731.72s test / 810s runner; test time includes 301s corrected-artifact build, runner includes 77s native compilation | `/tmp/canic157-installed-recovery4.log` |
| Direct host Fleet Ensure owner | 167 pass, two existing PocketIC cases ignored in the native tier | `/tmp/canic157-final-host.log` |
| CLI Fleet regressions | 52 pass | `/tmp/canic157-final-cli.log` |
| Prior/requested module history | Three pass; included in the host total | `/tmp/canic157-history-regression.log` |
| Host, CLI and fixture library/test Clippy with the governed fixture feature | PASS, warnings denied | `/tmp/canic157-final-clippy3.log` |
| Changelog governance and current-document semantics | PASS; one changelog case and zero document-layout advisories | `/tmp/canic157-final-changelog.log`, `/tmp/canic157-final-docs.log` |
| Local source replacement/crash boundaries and broader Fleet-related checks | Earlier 174 pass; exact before/after file combinations, drift rejection and completed-marker replay remain covered | `/tmp/canic157-recovery-host3.log` |

Earlier failed runs do not qualify recovery: the first used the wrong fixture
query endpoint, the second exposed the corrected history defect, and the third
caught display-text parsing of Candid Nat in the new fixture assertion. The
final run uses the existing operation-status endpoint and direct numeric
conversion. No failed run is reported as a successful repair.

The test is an explicitly ignored historical-source qualification, separate
from the ordinary current-runtime catalogue. An exact targeted PocketIC selector
now includes the explicitly selected ignored test. With the verified source
snapshot above, reproduce it with:

```sh
CANIC_ACTIVATION_SOURCE="$PWD/.tmp/canic157-installed-source" RUSTC_WRAPPER= \
make test-pocketic-case \
  CASE=pic::fleet_registry::baseline::tests::installed_partial_activation_recovers_through_reviewed_reinstall
```

The retained successful run used a private copy of the same governed runner
with `--ignored` for this exact case; the maintained exact selector now supplies
`--include-ignored`. Both select exactly one named test. Native checks and the
PocketIC result are working-tree evidence, not a published validation receipt.
The maintained runner's exact-case dry plan selects that test with
`--exact --include-ignored`; Bash syntax and targeted ShellCheck pass with the
repository's configured exclusions.

This single-Root proof complements the earlier complete 24-asset RF2 and public
process-metrics qualification; it does not claim Toko's live 9-Workload/15-Ready
estate was changed. Publication, artifact rebuild and downstream rollout remain
separate. The tracker still ends at CANIC-160 after the final read-only refresh.
The four reports are sane and the complete accepted 0.110.13 batch is ready for
push/release review. The root and detailed changelogs are ready for the governed
version transaction; package versions remain 0.110.12. No Git publication,
version transaction, broad gate, deployment or sibling mutation was performed.
