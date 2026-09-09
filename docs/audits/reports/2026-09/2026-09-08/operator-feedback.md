# Toko Miner operator feedback

Date: 2026-09-08. Baseline: tagged `v0.110.12`, commit `95c6caa03`.
Scope: read-only inspection of `toko-miner/docs/upstream/canic.md`, especially
reopened CANIC-042 and new CANIC-149/150. Only Canic is modified.

The feedback's CANIC-147 draft reference is now superseded by tagged 0.110.12,
which contains public history. Toko Miner's application producers, sampling-cost
qualification and live adoption remain downstream work for CANIC-147/148.

## OD1 diagnostic correction

CANIC-042 was reproducible in source: the first four prefix-matched lines could
be consumed by bare `Caused by:` markers while discarding the actual offline
checkout error. The host now retains the outer error and deepest causes within
four bounded lines and 768 characters. Cause continuations survive; source
excerpts are omitted, local role/workspace paths scrubbed, URL/assignment tokens
redacted, and unrecognized output receives a fixed fallback. The I/O error kind
or JSON category/location remains available for command-launch and malformed
metadata failures without echoing private source data. The shared typed
finding and evidence phase remain the authority for config/build/qualification,
state resolution and Medic. Medic now recommends dependency/cache resolution
and `cargo fetch --locked` before its locked offline evidence query.

CANIC-150 was also confirmed: the human report exposed unlabeled arithmetic,
raw funding-domain quantities and per-canister effect counts alone. Text now
names the planning allowances and measured terms, formats optional and required
cycle quantities consistently, lists ordered action kinds and distinguishes
host create actions from Root-funded pool creations. No plan, journal, progress
DTO or JSON schema changes. No executor or financial policy changes.

Final source qualification:

| Check | Result | Log |
| --- | --- | --- |
| `cargo test --locked -p canic-host --lib cargo_evidence_failure` | 5 pass | `/tmp/canic-feedback-host-tests.log` |
| `cargo test --locked -p canic-cli --lib fleet::tests` | 12 pass | `/tmp/canic-feedback-cli-tests.log` |
| `cargo test --locked -p canic-cli --lib medic::role_contract::tests` | 1 pass | `/tmp/canic-feedback-medic-tests.log` |
| `cargo clippy --locked -p canic-host -p canic-cli --all-targets --all-features --keep-going -- -D warnings` | Pass | `/tmp/canic-feedback-clippy.log` |
| `cargo test --locked -p canic --test changelog_governance` | 1 pass | `/tmp/canic-feedback-changelog.log` |

Tests use the governed scratch wrapper and explicitly select the owning library
for host/CLI behavior. Changed-file formatting, link presence and
`git diff --check` pass.

The complete OD1 source batch is ready to push and the open 0.110.13 changelog
is ready for the maintainer-selected release flow. Package versions remain
0.110.12. No broad validation, version transaction, Git publication, deployment
or sibling mutation was performed. No PocketIC is required for these diagnostic
changes. A new downstream invocation demonstrating the complete readable plan
and live stderr stream remains downstream evidence;
this source work does not claim that deployment run.

## CANIC-149 accepted follow-up boundary

Baseline gap: the public CLI could not request an unchanged-Wasm Fleet wipe.
`LiveCanister.reinstall_required` is observation-internal; production adapters
set it false. `reviewed_reinstall_canisters` admits only infrastructure, and the
native same-module test injects that flag on a mock Coordinator. The generated
PocketIC reinstall journey uses different release artifacts. None establishes
a complete unchanged-Wasm application database reset.

Accepted outcome: one operation-scoped wipe intent under the existing Fleet
Ensure owner, separate from permanent desired state. CLI parsing only submits
the intent. Workflow and pure policy derive its complete dependency closure;
ops persist exact reviewed authority and perform existing single-step effects.
The plan digest binds a fresh operation identity, unchanged release hashes,
environment/network, Principals, controllers, topology, maximum cycle debit
and complete reset closure. Issued install records bind the pre-install
management version before the effect. An unchanged release is not
evidence that a new requested reset has already happened.

Before any state is erased, inventory the Root-controlled estate and all
required Hub/Shard descendants with exact authority and cycle balances. The
plan must account for every retained asset across framework reinitialization.
Reset ordering and pool reconciliation must reconstruct framework authority and
authored application fixtures before reporting a terminal full Fleet. For an
unchanged topology, preserve the network and controlled Principal set without
unnecessary create/delete effects; explicitly bind any pool role reassignment.
This is same-release operational work, not cross-release identity or state
compatibility.

Persist intent and pre-install canister versions before paid effects. Lost
responses must reconcile those exact effects; resume keeps the original
operation and never repeats an acknowledged or proved reset. Reject a distinct
wipe while an operation is incomplete. Terminal replay is effect-free. Another
explicit wipe allocates a new operation; ordinary ensure after either wipe
remains effect-free. No application-owned install loop, synthetic Wasm change,
observation injection or new lifecycle owner satisfies this contract.

Treat public intent, authority closure, execution/reconciliation, diagnostics,
fixtures and cleanup as one implementation batch. Required qualification starts
from a converged Fleet with real user rows through the production adapter and
CLI boundary, proves those rows disappear and current fixtures return, records
retained identities/controllers and conservation, interrupts before and after
install responses, and demonstrates both replays and a second deliberate wipe.
Use the existing governed PocketIC journey owner with an application-neutral
Hub/Shard data fixture. Toko Miner then separately adopts the released seam and
qualifies its local startup and mapping refresh. Full local reset remains its
current available workaround. The maintainer accepted CANIC-149 as RI1 after
OD1 on 2026-09-08. Implementation and production-adapter qualification are
complete in the open 0.110.13 batch; CANIC-141 remains deferred.

### RI1 implementation and qualification

The CLI submits the explicit intent through the host Fleet Ensure workflow.
Preparation uses the current authority-snapshot commands and the existing
journal. Its completion is `prepared`, so current-Fleet consumers cannot mistake
sealed authorities for a playable Fleet. A second review with the same wipe ID
captures the complete pool, including assets outside the original seed, and
retains that normalized input only in the reviewed plan. Root pool reset and
ordinary provisioning rebuild the current topology and installation fixtures.

Before effects, management observations bind infrastructure modules,
controllers and placement; mainnet placement uses the validated routing
catalogue. Each pool asset must have exact Root controller authority. Before
Root loses its records, every captured asset is reinspected. Full completion
and replay verify the entire physical Principal set and directly recheck every
retained controller set, including the Ready reserve.

Same-Wasm recovery uses the issued pre-install version plus the latest
management deployment history: reinstall mode, module hash and operator must
match. Each reviewed install binds an exact Root witness and its Candid hash.
The controller-only Root command relays a replicated management history call,
including while sealed or Prepared. The host verifies the target, current
module and controllers before classifying the latest deployment. An older
history entry permits retry after ordinary version advances; a newer foreign
or mismatched change fails closed. This follows the upstream
[management-canister contract](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/#ic-method-canister_info).
Roots seal before Coordinator so outstanding Root funding can finish first.

The neutral Hub and Shard fixtures store actual user rows in an application-owned
stable BTreeMap. Their allowed memory range is explicit, and the existing
post-restoration installation hook seeds authored row `0 = 7`. The governed
mixed-topology journey creates and modifies rows before each wipe, crosses the
public CLI for the first request, interrupts before and after an install reply,
and checks row erasure, fixtures, exact estate retention, cycle accounting,
replays and a second intentional wipe.

Final native qualification:

| Check | Result | Log |
| --- | --- | --- |
| Host Fleet Ensure library tests | 154 pass, 2 existing PocketIC cases ignored in the native lane | `/tmp/canic149-host-tests4.log` |
| Core role-command replay contracts | 13 pass | `/tmp/canic149-replay-tests.log` |
| Fleet CLI behavior | 13 pass | `/tmp/canic149-cli-tests.log` |
| CLI help ordering and example bounds | 2 pass | `/tmp/canic149-help-tests.log` |
| Root protocol contracts | 6 pass | `/tmp/canic149-protocol-tests.log` |
| Changelog governance | 1 pass | `/tmp/canic149-changelog-tests.log` |
| Governed mixed-topology PocketIC journey, including two deliberate wipes | Pass: 1,764.30s test; 1,841s targeted tier; 1,842s runner | `/tmp/canic149-pocketic5.log` |
| All-target/all-feature Clippy, warnings denied, for core/facade/host/CLI/internal fixtures | Pass | `/tmp/canic149-clippy10.log` |

The native history cases cover exact upstream Candid variant labels, wider
wire schemas, optional change details, foreign actors, wrong hashes/modes,
ordinary version movement and controller drift. The generated-estate case
also qualifies normalization of extra physical assets outside the original
seed. Current document checks and changed-source formatting pass.

The final production-adapter journey passed both intentional wipes. Before
Root's first reinstall, the fixture interrupts after the authority inspections,
performs another real Root inspection and resumes the retained operation. This
qualifies replicated history reconciliation after ordinary version advancement.
Coordinator interruptions before installation and after the real install reply
also recover. Exactly three infrastructure reinstalls occur per wipe; completed
replays and ordinary ensure issue none. The first request crosses the public
CLI, the second receives a distinct operation ID, and a conflicting unfinished
request is rejected. Both wipes erase the user rows and restore only authored
fixtures, retain all physical Principals/controllers/cycle accounts, and pass
terminal conservation with five Workloads and one Ready reserve.

The complete combined OD1/RI1 source batch is ready to push and its open
0.110.13 changelog is ready for the maintainer-selected release flow. Broad
validation, versioning, Git publication and deployment were not performed.

Toko Miner adoption, local startup/mapping refresh and measured improvement
against full network reset remain downstream qualification. The PocketIC proof
uses Canic's application-neutral Hub/Shard fixtures; no downstream files were
changed. Package versions remain 0.110.12 and the combined draft is 0.110.13.

## Accepted follow-ups CANIC-151–156

The maintainer accepted all six after triage. The completed RF2 follow-up scope
extends OD1/RI1 in the same 0.110.13 draft.

CANIC-152's CLI 1.4.0 / launcher v15 alignment passes an isolated managed-network
probe: startup, ping, local faucet, detached creation, JSON status, settings,
installation and post-install status/module verification. The disposable network
was stopped. Evidence: `/tmp/canic152-probe.log` and the ignored
`.tmp/canic-152-probe` script/config. Existing live networks were not changed.

CANIC-154 rejects known omitted estate identities before Root reset. Retained
terminal evidence identifies omissions; the reviewed seed and matching policy
remain the import authority. CANIC-155 separates the base execution allowance
from an affordable continuation reserve, retains immutable protocol prefixes,
and requires review when the next effect exceeds authority or budget. CANIC-156
shows the shared policy's known recovery funding and pending-discovery boundary,
then reports exact action targets, kinds, bounded additional debit and the next
review command. Existing receipts and the operation ID survive these pauses.

The expanded real journey creates a 27-canister estate through the production
adapter, records its terminal evidence, rejects a deliberately incomplete
8-import seed, and validates all 24 imports before a changed-release Root reset.
It does not simulate historical 8-to-24 managed growth or change desired inputs
between the Root prerequisite and Store installation. Six actual Workload
balances are reduced before the reviewed recovery baseline. Infrastructure
planning succeeds below the whole-continuation allowance, exposes all six known
top-ups, and refuses their unreviewed debit after the two infrastructure effects.
A fresh reviewed phase recovers a lost funding response; later activation remains
explicitly reviewed with zero additional debit. Completion proves 19 Workloads,
five Ready reserves, all exact controllers, three total infrastructure reinstalls,
exact aggregate Ledger debit, bounded native burn, retained Root Ledger funds,
no additional creations and both completed-plan and fresh-plan zero-effect replay.
The lost Root install response also recovers through a reconstructed adapter.

This live test found and corrected a false pre-apply drift rejection: advisory
funding amounts and the affordable reserve change with ordinary observed burn.
The comparison now normalizes those estimates while retaining funding targets,
fees, margins and effect authority. Its focused regression also rejects changed
targets/fees and excessive observed movement. Unaffordable-first-phase and
immutable affordable-prefix regressions remain in the native selection.

CANIC-151 deliberately keeps exact build configuration as a caller-validated
precondition. The portable fixture and topology manifest cannot infer every
embedded runtime setting. The matching-build recipe requires retained source
bytes, finalized manifests and role-artifact verification before constructing
PocketIC. CANIC-153 projects bounded anonymous owner aggregates through the
existing sampler and history. Native cases cover all four process metric owners;
the Wasm fixture records through the existing public Wasm-store API and checks
that owner, ICC, timer and memory projections through the actual public cache.
No private metric API or production test hook was introduced.

Final focused evidence:

| Check | Result | Log |
| --- | --- | --- |
| Host/CLI Fleet selections | 167 host and 51 CLI pass; 2 existing host PIC cases excluded | `/tmp/canic151-156-host-cli4.log` |
| All-feature public metrics/history native selection | 18 pass | `/tmp/canic151-156-metrics2.log` |
| Timer PocketIC target | 8 pass; 23.46s test / 27s runner | `/tmp/canic151-156-timer3.log` |
| Retained 27-canister changed-release PocketIC journey | Pass; 1,030.37s test / 1,060s runner | `/tmp/canic151-156-reinstall4.log` |
| All-target/all-feature Clippy for the six affected packages, warnings denied | Pass | `/tmp/canic151-156-clippy5.log` |

The five-family probe measures 19,277,344 first-sample, 18,182,874 repeat-sample
and 18,959,614 maximum scheduled-tick instructions, below the unchanged
20-million bound. It checks anonymous public reads, protected denial, default-off
publication, history, source timestamps/units and family-failure isolation.
The Canic participant uses matching embedded source; this is not a new Toko Miner
producer measurement. Downstream application qualification remains separate.

The accepted OD1/RI1/RF2 batch and open 0.110.13 changelog are ready for
push/release review. Scoped Clippy, formatting, changelog governance and document
checks pass. Package versions remain 0.110.12.
No broad validation, Git publication, deployment or downstream edits occurred.

## Later tracker additions

A read-only refresh found CANIC-157–160 after acceptance of 151–156. These new
reports concern Root/Store activation identity across changed desired inputs,
bounded autonomous retry, protected originating-error evidence, and measured
planning latency. They remain proposed separate follow-up work. This batch
refreshes the complete seed before Root reset and preserves a single desired
identity thereafter; it does not claim to fix or qualify 157's mid-operation
identity change, nor the independent retry/diagnostic/latency outcomes.
