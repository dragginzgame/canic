# Current Status

Last updated: 2026-08-23

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the active design, source, validation or changelog material needed for the
current task.

Historical handoffs: [through 2026-06-30](archive/2026-06-30-precompact.md),
[through 0.90.2](archive/2026-07-13-precompact.md) and
[through 0.101.52 Q4](archive/2026-08-12-precompact.md).

## Current Release

- The root `Cargo.toml` is the sole live workspace package-version authority;
  this handoff deliberately does not mirror its value. Immutable local and
  remote tag refs own publication truth, and tag presence determines whether a
  changelog section is an open draft or a published patch.
- The completed role-owned Candid release owns the 161-reason register, typed mappings,
  compact public wire and flat `code + name` release baseline. B1 census/count
  coupling is absent from permanent tests. Its published closeout correction
  makes that released baseline tamper-evident, preserves the four genuine
  caller-continuation values and routes ordinary host failures through the
  prose catalogue. Active checklist:
  [0.102 compact diagnostic codes](../design/0.102-compact-diagnostic-codes/status.md).
- Application-safety and estate sequence: [0.103 role-owned Candid surface](../design/0.103-role-owned-candid-surface/status.md), [0.104 timer ownership plus synchronous lifecycle composition](../design/0.104-ic-timers-consumer-hard-cut/status.md), [0.105 framework-neutral local application authorization](../design/0.105-framework-neutral-local-application-authorization/status.md), [0.106 platform qualification](../design/0.106-fleet-estate-platform-qualification/status.md), [0.107 fresh-Fleet preflight and runtime admission](../design/0.107-fresh-fleet-preflight-and-runtime-admission/status.md), [0.108 Coordinator-backed root funding](../design/0.108-coordinator-backed-root-funding/status.md), [0.109 Fleet-wide ingress admission](../design/0.109-fleet-wide-ingress-admission/status.md), [0.110 reusable estates plus application retirement](../design/0.110-fleet-subnet-canister-estates/status.md), [0.111 stateful Fleet release adoption](../design/0.111-stateful-fleet-release-adoption/status.md) and [0.112 generic Fleet observatory](../design/0.112-fleet-observatory/status.md). External [Prequel Wars](https://github.com/dragginzgame/prequel-wars) replaces the checked-in Skynet App as the flagship demonstration. Other future concepts are [unnumbered ideas](../design/ideas/README.md).
- Release boundary: every pre-1.0 transition is reinstall-only. Every
  Canic-owned canister in a Fleet must come from one admitted release set
  before activation. Same-release interruption recovery, exact retry, backup
  and restore remain required. Scheduled 0.111 is the first explicit
  one-predecessor-to-one-successor exception; no current release is adoptable
  until that line is implemented and published.

## Current Progress

The published 0.108.0 checkpoint contains M0/M1 plus the urgent fresh-Fleet
corrections. The open 0.108.1 draft completes B3-B9/M2-M8: exact registered-
Root admission, full Registry-authority-bound operation identity, fixed and
non-renewing budget/reserve accounting, durable two-sided journals,
accept-once/zero-accept replay, recovery-first Coordinator funding, protected
manual and terminal automatic ICP refill, exact installed-authority recovery,
funding status/metrics, Medic, lifecycle/snapshot fences and explicit funding-
policy generation rotation. The first
human-owned closeout audit rejected the draft because the two funding legs
still used unbounded calls, the ICP refill journal had no lifetime bound, the
PocketIC matrix and B3-B8 evidence were incomplete, and active documentation
overstated readiness. The correction uses bounded calls on both legs, caps the
non-evicting Root refill journal at 4,096 lifetime identities, and adds real
PocketIC single-/multi-Root accounting, non-renewing-cap, uncertain-call,
direct-top-up, production Ledger/CMC replay, reserve-fallback, insufficient-ICP
and rate-denial journeys. The maintainer's first candidate validation exposed
stale CLI/hash/timer expectations and one real pre-activation Root-admission
defect; its follow-up exposed duplicated-match Clippy failures. Those
candidate defects are corrected and focused regressions pass. The complete
maintainer gate must be rerun against the final immutable candidate before a
fresh closeout audit. The 2026-08-23 rerun stopped in `check-invariants`
because the rotation-policy model imported boundary DTOs. The correction keeps
the invariant decision in the model behind one DTO-free named input and moves
only boundary conversion to ops; the exact ten-target invariant gate, focused
rotation consumers and warning-denied changed-package Clippy now pass. The
next maintainer validation completed every serial PocketIC suite but its earlier
ordinary workspace lane reported four failures: five rotation commands were
absent from replay-policy manifests, and host admission/order tests still named
a removed controller-only helper. The open correction classifies all five
durable commands, moves identity evidence to the live operator-funding observer
and makes ordinary failures stop the combined runner before PocketIC. Focused
replay-manifest, live operator-observer, install-order, runner-contract and
warning-denied changed-package checks pass; fake ICP executables are published
from closed staged files so parallel tests cannot race their writers.
The follow-up test-runtime slice keeps every internal PocketIC scenario in one
ordered process, runs its five pure checks before the PocketIC barrier and
preserves direct targeting of each original test. The first two governed cases
remain Fleet deployment restore and autonomous Root removal. The restore proof
uses the process-local active-Fleet baseline; destructive Root removal uses a
fresh exclusive instance because a deleted canister is outside that baseline's
snapshot-reset contract. The canonical Coordinator artifact is
content-addressed through the persistent external-artifact cache, fresh Fleet
setup reports each build/topology/install/join/activation/validation phase and
the runner records PocketIC server RSS/high-water/thread evidence after every
serial suite. Native authorization reuses the captured Fleet only for cases
whose mutations remain inside the complete snapshot/reset contract; the two
HTTP-gateway journeys stay fresh. The cache-populating governed run passed all
22 cases in 293 seconds; the cross-process reuse run passed in 208 seconds,
with the Coordinator phase falling from 17.267 to 1.520 seconds and the fresh
Fleet baseline from 33.601 to 16.138 seconds. Its shared PocketIC server peaked
at 2,229,804 kB RSS and 97 threads. The six-test native-agent target passed in
68.51 seconds after compilation and peaked at 2,519,036 kB and 162 threads.
Serial capacity remains one: these measurements justify the retained process
and cache changes, but one local observation does not prove parallel stability.
Versioning and publication remain pending the complete immutable-candidate
gate, audit verdict and maintainer release workflow.

The 2026-08-22 0.108 design amendments are implemented through B9. Protected
input materializes topology-matched single-Subnet, bounded preview multi-
Subnet and professional multi-Subnet baselines, scales them rationally by
current Registry node count, enforces one grant per 90-day default window and
retains finite non-renewing automatic count/cycle caps. The recommended
standard-13-node one-Root preview is 140T Coordinator creation, 80T reserve,
30T Root creation, 10T Store creation and a two-grant/60T lifetime cap: 180T
in creation amounts before the three exact Cycles Ledger fees, with automatic
ICP disabled. The Fiduciary-backed `recommended` Coordinator
selector is removed; every Fiduciary Coordinator or Root placement requires
an exact adjacent cost acknowledgement and emits retained high-cost evidence
before plan and paid install effects. Corrected policy/hash/plan authority
flows through Registry, stable accounting and generated interfaces before the
sole Root timer runs.

The Toko operator-config follow-up is also implemented in the open draft.
Fleet input now carries only one top-level operator Principal; planning and
installation verify the active ICP identity, derive its relevant ledger
account and query its live cycles balance. Volatile balance/source/time
observations do not perturb the canonical plan digest, and installation
rechecks live sufficiency before the first effect.

The follow-up funding review closes the remaining admission gaps. Fresh-Fleet
infrastructure creation is cycle-only; ICP creation amounts reject before
profile validation and the deferred conversion/rate authority is recorded as
an unnumbered idea. Maximum operator debit now adds one exact Cycles Ledger fee
for every host-created Coordinator, Root and Store. The `canic scaffold
fleet-input` authoring surface resolves selected IC Subnet IDs through
validated Registry evidence, retains explicit node counts as an offline
fallback, shows all scale/round/cap/fee/debit formulas and emits exact integer
funding TOML without reading an identity or balance. Live `canic deploy plan`
remains the install-admission boundary. Focused equality/one-cycle-short, ICP
CLI 1.3 balance-fixture and standard/Toko profile-scaffold checks pass.

The Toko fresh-install reconciliation report is corrected in the same open
draft. A Coordinator accepts a fully validated terminal Root receipt without
having observed the Root's intermediate provisioning counters or requiring
Root completion to follow its passive query intent. The immediate
post-acceptance query also normalizes valid intermediate or terminal progress
to the exact canonical acceptance receipt, and scheduled retry failures emit a
bounded operation/phase/diagnostic record instead of disappearing. Focused
terminal-jump, acceptance-race, restart/replay, forged-receipt and retained
stepwise-path regressions pass.

Accepted CANIC-019 adds one controller-reviewed same-release escape from an
exhausted generation without weakening those finite caps. A no-effect CLI plan
binds the exact installed Fleet, predecessor Registry/generation/policy and
usage, resolved placement evidence, proposed successor authority, zero operator
debit and maximum new exposure. Apply uses one Coordinator-owned durable fence
and idempotent Root prepare/activate receipts. Prior cumulative usage and grant
sequences remain monotonic, automatic effects stay fenced until convergence,
and application Registry state remains owned by its existing lifecycle. The
non-evicting completed history is bounded to 4,096 total Root checkpoint
entries and fails closed when a successor will not fit. Every retained
checkpoint binds the complete plan for exact historical replay. Root activation
recovers across the temporary successor-protected/predecessor-mirror split,
and successor budgets preflight retained fixed-window spend. The corrected
32 MiB Coordinator cell covers the measured 25,315,095-byte worst fragmented
history. Focused unit, stable-capacity, generated-surface and real PocketIC
exhausted-to-successor evidence passes; the complete maintainer validation gate
and human closeout audit remain pending for the final immutable candidate.

The compact diagnostic and role-owned Candid lines are published. Their
read-only closeout audit confirmed the compact wire, typed runtime, host-only
catalogue and Wasm boundary, then closed one dynamic data-loss defect plus
released-baseline, ordinary-rendering and release-reliability defects. One
typed cycles-funding preflight response owns the four genuine caller-
continuation values; the other 64 provisional B1 owner proposals are
reconciled as local typed state, caller-derived data, existing authority or
deliberately dropped operator convenience. The release guard reads the exact
released reason-ledger Git object, including retirement state, and the central
host decoder renders known diagnostics automatically.

B1's 2,895 labels exposed the rejected unreleased 991-row
tuple taxonomy. The accepted frontier is 161 registered causes and ten local
typed families. All
four phases—Register, Map, Cut, Clean and measure—are complete: lossless raw
codes, producer-only registered codes, `Error { code: u16 }`, code-first
`InternalError`, and host-only prose. The flat release guard freezes only
released `code + name`; it neither reads B1 evidence nor freezes row counts,
and the ledger rejects fields beyond its six maintained fields.

The completed 0.103 line owns role methods and autonomous operations. Its B1 evidence
was accepted on 2026-08-17: the immutable `v0.102.2` baseline freezes 207 methods across
representative Root/managed profiles and canonical Coordinator/Store
interfaces, separated into 188 Canic-owned, three external-standard and 16
fixture-owned methods. B2-B7 are complete in the published source: the closed
capability boundary, immutable Overview and bounded shared-request DTOs now
drive the exact managed, Root, Coordinator and Store status dispatchers. Exact
Candid/profile identity survives release metadata, verified Root Store
catalogues, install/Registry replay validation and Component Directory
projection. Root, Coordinator, managed and Store operations resolve their exact
durable IDs through their owning commands rather than a universal operation
store. Representative Runtime-only,
Sharding/AutomaticTopup, Root signer/non-signer and Store builds prove the
corrected B1 protocol is actually pruned:
external artifact metadata selects the exact profile-specific binding before
the first call, `Overview` only verifies it, and
config-derived `AutomaticTopup` rather than mandatory `Runtime` owns top-up
history and public handler reachability. Private automatic-top-up timer,
callback and workflow pruning remains the explicit 0.104 owner. Endpoint
source, authorization/payload attributes, immediate delegates and replay
policy are frozen; existing typed config derivation remains the capability
authority. The corrected six-way review disposes all 188 Canic-owned appearances: 49
role-command variants, 78 role-status variants, two Store byte lanes and 59
private deletions. Every retained row names its exact role-specific variant,
released Rust signature and reviewed executable-caller subset. The normalized
manifest also freezes request/response correlation, sync/async and variant
counts, selector nesting and the atomic B4/B5 caller cuts. Distinct status
concepts stay flat rather than becoming nested family enums; remote composite
phase observations collapse into local operation status and update-only
canister inspection becomes an atomic Root command. DTOs, the exact pruning
matrix, operation ownership and the no-new-timer 0.104 handoff are frozen.
Host/CLI fixtures and then-current application presentation use the role
surface. The obsolete checked-in Skynet demo is removed in favor of external
Prequel Wars; frozen B1 source evidence remains historical. Legacy
shared/non-root/cycle/topology emitters are
deleted, `start_local!` emits one local status method, and the representative
Canic count falls from 188 method appearances to ten. Current fast-profile
Wasm identities are recorded without claiming causal size savings. Raw B1
pre-cut Candid snapshots are no longer retained in the current worktree; the
capture tool derives the immutable normalized register and manifest hashes
from scratch-generated interfaces so historical contracts cannot be mistaken
for live ones. The same cleanup removes five unconsumed legacy build cfgs and
their dead environment/manifest probes; the generated-role cfg catalog is once
again exact and singly owned. The ceremonial default-on `metrics` Cargo
feature is also removed from the facade catalog, all 38 live manifests and
generated/package-contract fixtures; metrics profiles remain config-derived.
The dated 0.103 closeout audit initially found two P0 implementation gaps plus
Store-pruning and active-document/count drift. The correction now authenticates
every Root/Coordinator variant before protected work, requires complete
immutable host/CLI protocol identity before transport, compiles Store lanes
only for Store and its exact Root caller, and reconciles active names/counts.
Focused all-target compilation, warning-denied Clippy, binding/auth/replay
tests and four-role artifact scans pass. The human-owned package bump, complete
release gate, annotated tag and atomic push are complete.

Scheduled 0.104 owns the `ic-timers` consumer/domain async-job recovery hard
cut, a maintained native-timer adoption guide/fixture and the synchronous
framework-neutral lifecycle participant required to compose Canic with IcyDB.
Its exact 0.103.0 source/provider baseline, consumer and native-claim census,
memory-ID-60 disposition and downstream propagation contract are complete as
B1 evidence and were accepted on 2026-08-18. B2 was accepted the same day: the
public application timer macros, facade, handle and transient claims are
deleted without aliases, callbacks consume native provider vocabulary, and
the runtime probe owns its application registrations directly. B3 is accepted.
Memory ID 60 now contains only checked domain
attempt leases and cycle-top-up's exact retry generation; pending timer
commands, schedule ownership, generic recovery deadlines, copied retry streaks
and terminal provider state are deleted. The exact worst-case record encoding
is 589 bytes. B4 moved auth renewal, automatic cycle top-up and placement
acknowledgement to lazy native registrations and exact domain recovery. Its
seven-to-five provider result is a historical B3/B4 development observation;
the final inventory is role-specific. B5 is accepted: the remaining central claim and participant
registries are deleted and pool/lifecycle/snapshot owners retain exact native
custody. The intermediate B2-B6 size and instruction deltas are historical
development observations only because their exact source states were not
preserved. B6 is accepted on locked `ic-testkit 0.8.8`: the maintained native
guide and direct-provider fixture replace the removed facade examples, and
ordinary managed, Root and local start macros accept one safe paired
synchronous lifecycle participant after Canic restoration and before deferred
work. The 46,593-instruction result with zero Wasm- or stable-memory-page
growth is one current two-work-sample observation.

The 2026-08-19 independent closeout audit reopened the line after
`v0.104.1`. Published `v0.104.2` now freezes native registration actions,
prunes automatic-top-up callback/workflow code from capability-disabled
product artifacts and corrects the quantitative evidence authority. Its
immutable source also contains the accepted 0.105 B1/B2 presenter, scope,
authority and pure-policy hard cuts; the active changelog records that early
inclusion rather than leaving it classified as unreleased.

Published `v0.105.0` owns framework-neutral caller-bound scoped local
application sessions directly after 0.104; presentation supplies no semantic
dependency. B1 was captured against exact released `v0.104.1` and is now
reconciled to exact B3 predecessor `v0.104.2`. Its complete
producer/consumer inventory, stable/resource baseline and real PEM-backed
native-agent prepare/retrieve/present journey are retained under
`docs/audits/working/0.105-local-application-authorization/`. The maintainer
accepted caller-derived signed presenter/subject identity and target-local
typed replay-capacity denial on 2026-08-19. B2 now owns the presenter-bearing
token hard cut, canonical application scopes, one verified-authority model and
pure closed policy, all already present in `v0.104.2`. B3 hard-cuts memory ID
34 to bounded caller-bound scoped sessions, proof-consumption replay authority,
one local generation and synchronously reconstructed exact indexes. B4 adds
explicit protected role enablement plus cfg-pruned establish, clear and
caller-self status variants beneath the existing managed role methods. B5 adds
the synchronous facade and token-free generic consumer, converges the proof and
session authority read, and deletes the subject-identity fallback across
managed, Root and Store endpoint generation. B6 adds the separately
Root-authorized paginated audit, bounded aggregate metrics and exact generation
reconciliation. Published `v0.105.0` PocketIC observations keep
one/midpoint/maximum local authorization at 180,797/186,749/187,112
instructions, retain physical stable
memory at 3,329 pages under maximum state and bound one cleanup delivery to 128
records. Controlled equal-path `v0.104.2` -> `v0.105.0` product builds measure
19,054,678 -> 19,236,127 raw bytes, +181,449 and within the 256 KiB ceiling,
with all four Candid hashes and lifecycle export sets unchanged. B7 proves one
native identity can establish independent authority on both compatible managed
targets while their common Root controller remains denied. Both sessions and
exact receipts survive proof expiry and same-release upgrade without duplicate rows; target-local clear,
strict session expiry, bounded cleanup and non-resurrection finish with zero
rows. B2-B7 are complete, the 0.105 implementation batch is closed and
`v0.105.0` is published.

The scheduled 0.106 B1 was accepted on 2026-08-20. Its immutable `v0.105.0` baseline, exact
empty-topology `EmptyRootAdmissions` blocker, complete Q2 normative provenance
matrix and complete current pool/state/scan/timer/encoding/snapshot inventory
are captured under
`docs/audits/working/0.106-fleet-estate-platform-qualification/`. Every Q2
empirical cell remains pending B2; Q3/Q4 acceptance and the bounded local
harness are complete. Accepted protocol `canic-0.106-q3q4-v1` freezes the cohort,
censoring, seven-day horizon, margin and recovery-reserve rules plus the exact
predecessor-built fixture hash and initialized memory observations. Its
proposed operation, physical-asset, concurrency,
fee/refund, reserve and funded-exposure ceilings are frozen; exact external
network, identity and terminal-asset binding is a separate B2 execution gate.
The focused
Root uncertainty journey reuses one exact paid request and recovers its
original Principal on request two. Local 1/8/16/32 creation and empty/installed
reset cohorts, first excess and source/joint/destination controller transitions
also pass. The terminal guard keeps both qualification canisters as unpublished
test-only dependency leaves outside shipped role configuration. Q6 records
unbounded pool failure text, the 64-byte handoff-receipt bound's 67-byte
structural counterexample, unbounded terminal-receipt retention and the absent
canonical receipt-map snapshot payload as accepted 0.110-owned constraints;
0.106 does not repair them. B2 execution is held pending separate exact
authorization for every external effect. The release-tree closeout restores
the exact `v0.105.0` versions of every pre-existing locked package and
classifies the co-delivered 0.108 B1 probe separately as an unpublished
test-only leaf; neither the probe nor its direct test dependency enters a
shipped role or the 0.106 protocol.

The published 0.107 implementation line closes the three deployment-readiness
gaps exposed by the first read-only Toko integration pass: target- and Fleet-
input-complete fresh-Fleet planning, structured NNS catalog inconsistency
diagnostics and a bounded durable runtime whitelist. Its seven batches were
estimated at 10-15 engineering days excluding upstream release latency or
separately approved live-IC qualification. The maintainer accepted B1 on
2026-08-20: it
freezes the exact plan/install grammar and digest, Root-or-controller managed
role surface, memory ID 61, 256-principal/128-page/one-operation bounds and the
smallest typed `ic-query 0.40.1` provenance addition. Test-only maximum
encodings are 8,417 stable bytes, 4,072 status Candid bytes and 101 mutation
Candid bytes. The read-only Toko baseline supplies 175 principals but no
current Canic integration fixture. B2 is complete: the top-level environment
reaches direct `deploy plan`, hidden/global disagreement rejects before
dispatch, and missing or contradictory canonical target authority blocks
before Fleet-catalog lookup. B3 is complete: direct planning now hard-cuts to
the required install Fleet input, common profile and optional finalized
release source, while one pure host compiler owns placement, admission,
initial Component Group and positive-funding validation for both plan and
install. Install resolves target, input and read-only release source before
durable release-build allocation; invalid input/topology tests leave no
release-build directory. B4 is complete: one canonical decision now binds
exact source/artifact authority, placement, derived counts, per-category
funding, maximum operator debit and strictly bounded balance evidence into a
domain-separated SHA-256 digest. Planning renders that decision; install
recompiles it before allocation and binds it through same-release session,
persisted Fleet plan, deployment truth, completion/rejection receipts and
resume comparison. Changed source, balance or expected digest fails before an
effect begins. B5's routing and detailed failure work is complete on published
crates.io `ic-query 0.41.2`, and published `v0.107.2` advances to `0.42.0` for
stable snapshot authority, locked with checksum
`311b60543bc5c09c961abe9612d2bf3e26e99ba8bcadb3c01d043056c544a318`.
Canic uses its modern-first detailed cached/live results and exhaustively
projects
the typed request source/assurance, cache stage/disposition, pinned and
returned Registry-value versions, exact failing endpoint/assurance, completed
record reads, offending subject, code/category and truthful unknown-retry
reason into plan JSON/text plus explicit false effect facts. No fork, error-
string parser, routing fallback, inferred version or guessed retry decision
exists. B6 is complete. Memory ID 61
now owns one bounded canonical runtime-whitelist record, managed role methods
own Root-or-controller add/remove/status variants, configuration is fresh-seed
input only and same-release restoration validates without reseeding. The real
maximum schema remains 8,417 bytes. Focused unit, Candid, source and managed-
artifact checks pass, and the bounded PocketIC journey proves seeding,
controller/Root separation, response-loss exact retry, conflict denial,
immediate removal, restoration and application-session separation. Independent
B7's original proof is complete and reconciled with B5; its historical
read-only Toko snapshot was
`bf14a5d3d89be4335d3da2601e8a60128fde04df` with no Canic integration or
feedback identifiers. Newer downstream feedback was inspected read-only from
Toko Miner HEAD `2af2182f97cb21e220081d49169d6a006eff1adb` while preserving that
repository's existing dirty user work; it is evidence, not authority to edit
Toko Miner. The final audit rechecked the separate dirty Toko Miner worktree at
`4cd7aa8c18e6edde4a9d28a3b4d23709ff542d3e`: CANIC-012 and CANIC-013 are
verified, while CANIC-009 records its first real anonymous/locked installation
exercise and CANIC-011 records its first installed-Fleet mutation/removal/
restoration exercise as exact external blockers. That record satisfies AC13's
blocker alternative without a Toko or live-IC mutation. Published `v0.107.1`
carried the first typed B5 projection, but a subsequent live upstream check
found that `ic-query 0.41.0` could join the current Subnet list against retired
legacy routing authority. Published `v0.107.2` locks
`ic-query 0.41.2`, whose modern-first collector reconstructs the complete
pinned `canister_ranges_*` family and never falls back after a modern-family
error. It also exposes portable canonical Registry-key constants, typed record
subjects and ordinary uncertified-query evidence construction for downstream
fixtures. Canic consumes those builders in its Root-subnet fixture and retains
the returned value version, exact failing endpoint/assurance, completed
Registry reads, shard lower bound and value encoding in plan JSON/text. Old
cache shapes fail closed and require refresh; there is no compatibility reader.
Downstream closeout feedback identified that ordinary direct planning had no
supported way to acquire that evidence in a fresh checkout. The published
correction adds explicit `deploy plan --refresh-catalog`: default planning
remains cache-only, while the opt-in mode may issue public NNS Registry query
calls and update only the private `.canic/ic-query` cache when it is missing or
invalid. It cannot start a build, write deployment state or perform an IC
update call. The authoritative plan consumes only stable Registry version,
catalog digest, assurance and source endpoints; cache path, collection time
and disposition are separate report provenance. The request and transient
refresh state therefore do not perturb plan/install digest parity. Fresh
install uses the same acquisition path automatically when its cache is missing
or invalid.
The same downstream feedback showed that install resolved anonymous, locked or
wrong effective ICP credentials only after expensive builds. The correction
now resolves the effective identity after exact plan recompilation and before
release-build allocation or Wasm preparation, rejects anonymous and
Fleet-operator-mismatched Principals, and reports
`CANIC_ICP_IDENTITY_PASSWORD_FILE` as the non-interactive encrypted-identity
remedy. Later creation-time controller observations remain in place.
The published correction also makes dead code and stale lint expectations hard
failures across every workspace member, narrows internal facade/proc-macro
visibility and removes two redundant dependencies from the test-only IcyDB
composition fixture. Exact generated-code dependency exceptions are documented
and the repository cargo-machete scan is clean. These compile-time changes do
not change generated Canister code. A maintainer-owned complete-validation run
then passed the Fleet deployment-restore and every serial PocketIC lane but
exposed two unrelated unit-fixture races: one placement assertion resampled a
one-second wall clock after the operation, and Linux returned `ETXTBSY` while
one fixture inspected a directly published fake `ic-wasm` executable. The
fixtures now compare returned authority with its durable row and atomically
publish the executable from a closed staging path. The 2026-08-21 closeout
audit then ran the complete `make validate` gate: formatting, repository
invariants, dependency/security checks, locked workspace check, warning-denied
Clippy, ordinary tests and every serial PocketIC lane passed. The audit found
no runtime or release-boundary defect and rejected closeout only because active
documents still called the tagged patch open and retained superseded
authorization/validation wording. This documentation-only correction removes
that AC12 residue. The exact re-audit passed, the maintainer accepted its
`APPROVE 0.107 CLOSEOUT` verdict and explicitly opened 0.108. No external
effect occurred, and held 0.106 B2 effects remain independent and
unauthorized.

A post-publication fresh Toko installation then exposed an operation-identity
collision that blocks Component provisioning before application tests begin:
Fleet activation and Root Component provisioning reused the install operation
ID, so Root status routing could not return the Coordinator-observable record.
The 0.108 correction derives a distinct provisioning phase ID, defers
Root controller/Coordinator observer checks until unique ownership is known,
retains exact release-build Candid sidecars for infrastructure binding and
reports the last correlated `AcceptingRoots` evidence when the bounded host
loop fails. Focused host and control-plane regression tests pass. The
0.108.0 checkpoint now retains the exact release-Wasm sidecar, distinct phase
identity and bounded diagnostics together. The downstream fresh-install rerun
and maintainer-owned release gate remain pending.

The active 0.108 line closes replay-safe Coordinator-backed Root operating
funding separately from the estate budget. The maintainer accepted M0 on
2026-08-21 and continued the sequenced implementation. The complete test-only
M0 proof passes Root accept-plus-receipt trap rollback, post-commit
Coordinator response loss, zero-accept replay/refund and the pre-acceptance
balance API semantic. It also passes a real nested Root-to-Coordinator-to-same-
Root request/retry journey, a bounded current/last-result sequence
counterexample suite and the offline Fleet-catalog/install-journal break-glass
authority proof. The emergency journey covers fee/decimals/rate acquisition,
ledger-transfer response loss and duplicate recovery, CMC-notify response loss
and replay, then terminal completion. M1 recomputed the final 16 KiB
`canic_command` envelope at 42,118,809,000 cycles and freezes both Root
request/retry and automatic-refill floors at 42,200,000,000 cycles on PocketIC
15.0.0 with pinned `ic-cdk 0.20.2`. The final command DTO fits that frozen
bound. Fixed epoch windows charge the reservation-time window, cooldown
starts at first accepted receipt, Draining has one lifecycle-owned funding
fence, and receipt storage stays one current plus one last exact result per
Root. B2/M1 is complete: strict schema-1 Fleet-input policy, central validation
and canonical hashes flow through plan, init, root authority and Registry;
Coordinator genesis and root activation validate independent copies, and the
unreachable generic `canic.toml` refill path is hard-cut. Generated Candid
expands only the protected init/Registry data and adds no funding endpoint. B2
is complete and included in the 0.108.0 checkpoint. B3/M2 through B5/M4 are
complete in the open 0.108.1 draft: Coordinator grants and Root acceptance own
exact durable two-sided journals; sparse topology-scaled policy, non-renewing
caps and explicit Fiduciary acknowledgement flow through immutable authority;
and the sole Root top-up timer resumes retained work before creating a request.
B6/M5 through B8/M7 are complete in the open 0.108.1 draft and do not ship in
0.108.0. They add the one-owner protected ICP refill path, operator and
lifecycle surfaces, real PocketIC qualification and closeout handoff. B9/M8
adds explicit same-release funding-policy generation rotation with one durable
Coordinator fence, bounded retained checkpoints and unchanged application
state; it also remains confined to the open 0.108.1 draft.

Scheduled 0.109 hard-cuts the independent per-canister whitelist into one
Coordinator-owned Fleet admission policy with complete local enforcement
projections. Fleet, Component-spec, Component-instance, Fleet Subnet Root and
explicit standalone-consumer scopes narrow authority; physical Subnet
membership never grants it. The framework-neutral standalone-consumer
contract lets Toko's IcyDB App enforce the same policy without making it a
Canic-managed Component or assuming authorization inheritance.

Scheduled 0.110 owns indexed estates, parallel creation/reset, transfer and
the 10/100/1,000 proof. Opted-in stateful roles must produce an immutable,
bounded application retirement acknowledgement before ordinary uninstall;
forced removal is separately authorized and permanently marked unqualified.
Scheduled 0.111 then owns one stop-the-world exact predecessor/successor
adoption before stateful production claims. Scheduled 0.112 publishes generic
supported observatory views/rendering for downstream Prequel Wars without a
game dependency. Estate-budget replenishment, a product-frontend delivery
handoff, transport, Workers, authentication profiles, blob/archive storage and
Motoko remain ideas.

## Current Decision

Diagnostic registration shipped in `v0.102.2`. Codes identify semantic causes;
typed callers retain handling. Only public, retrievable operator,
durable-evidence or machine-decision boundaries qualify. Projection is explicit;
masked reasons without an independent exact owner stay local, required dynamic
data stays typed, and nonessential context creates no infrastructure. The 991
candidate was replaced rather than retired, and B1 evidence has no allocation
authority. Activation requires one admitted release set; do not add compatibility
decoding, message fallback, diagnostic version or observability machinery.

0.103 B2-B7 source mutation is complete. The maintainer explicitly authorized
B4 mutation on 2026-08-17 and then accepted the bounded 32-Root/nine-Coordinator
variant correction exposed by implementation. Component provisioning now accepts one intent,
self-advances privately and is observed through operation status; Root and
Coordinator command ingress also enforces the selected variant's exact payload
limit. The correction retains scale-out synchronization, Registry
acknowledgement and the two external Root-deletion evidence outcomes, while
Root snapshot reads reuse Registry status, Coordinator removal polls Root
operation status and pre-adoption bytes move to the B5 Store lanes. The atomic
Root/Coordinator and managed/Store caller/fixture cuts, generated role Candid
and exact operation authority now pass focused evidence. Managed profiles
expose only their cfg-selected command/status variants; Store exposes
command/status plus its two admitted byte lanes. Cross-cutting presentation,
legacy-emitter deletion, current-surface residue guards and the count/Wasm
closeout and human-owned release flow are complete. The maintainer accepted
0.104 B1 and B2, then continuation accepted B3-B8. B3 hard-cuts
generic timer recovery into four closed
domain async-job fences, retains a generated exact retry identity only for
cycle top-up and removes every stable provider mirror. Its bounded-state,
property and interruption evidence is accepted; its phase measurements are
historical rather than closeout proof. B4 moves
auth, cycles and placement out of central fixed-job selection while preserving
snapshot custody and exact recovery; its current-graph evidence passes on
the then-locked `ic-testkit` 0.8.7. B5 deletes the remaining pool/lifecycle/
snapshot registries, restores role-pruned authority linkage and passes on
`ic-testkit` 0.8.8. B6 publishes direct native adoption and adds the paired
synchronous lifecycle participant with exact restore-before-participant-before-
defer ordering, Prepared/inactive execution and trap rollback. B7 composes that
participant with exact published IcyDB 0.230.2 and proves one timer provider,
one lifecycle export pair, separate reconstructed rows, rollback and corrected
retry. Published B8 parses the complete Rust ownership set and freezes native
constructors. Published `v0.104.2` also freezes registration method
actions and rejects their disguised or duplicated use across applications,
crates and executable fixtures. It also includes accepted 0.105 B1/B2 ahead of
the remaining 0.105 batches; published `v0.105.0` completes B3-B7 without an
observatory dependency. 0.106 remains evidence-only: repository-local B1 is
accepted and B2 stays held pending its separately approved external run plan.
0.107 B2-B7 implementation and the complete validation gate are finished from
accepted 0.106 B1 without the held B2 external effects. Its AC12 correction
re-audit passed and the maintainer accepted the final closeout verdict.
0.108 has completed 0.107 and its accepted inputs; M0 is accepted, B2/M1
protected policy and the urgent fresh-Fleet corrections form the published
0.108.0 checkpoint. B3/M2 through B9/M8 are implementation-complete in the
open 0.108.1 draft; post-validation candidate corrections and the CANIC-019
amendment have focused passing evidence, while the final maintainer gate and
human 0.108 closeout audit remain next. 0.109 requires completed 0.108 and
explicit B1 promotion; 0.110 requires completed 0.109 plus the accepted,
current 0.106 qualification; 0.111 requires completed 0.110 and an exact
released predecessor; and 0.112 requires accepted 0.111 closeout. Deferred
ideas authorize none of these mutations.

## Validation

The retained `v0.101.53` `CANIC-WASM-001/v3` baseline, exact final-tag
`v0.102.2` artifact identities and focused correction checks
cover ledger generation, released identities, raw/registered separation,
projection, host lookup, compact Candid, typed mappings and generic
current/retired catalogue invariants. Core, control-plane, host, CLI,
Coordinator and Wasm Store targets compile. Representative `app`, Root,
Coordinator and Store release builds pass integrity and bounded absence scans;
all data sections shrink, but only Root shrinks overall, so no causal size claim
is made. Correction-specific unit, Candid, host/CLI rendering, canonical
Coordinator/Store build and warning-denied package checks pass. The complete
2026-08-16 `make validate` remains historical push
evidence and is not rerun during focused development.

The release-reliability foundation removes the contradictory tag-only green
signal, uses Cargo as the sole live package-version authority, adds a post-bump
release candidate guard, gates expensive jobs behind preflight/security and
reports ordinary versus serial PocketIC timing separately. Local validation
and CI's preflight, security and Rust-check jobs now collect every independent
failure inside their barriers, retain complete logs and do not admit expensive
work after a cheap failure.
Workspace tests continue across selected binaries and suites, while the serial
PocketIC group preserves one warm Wasm build state instead of clearing it
between suites. The internal harness now consumes `ic-testkit` 0.8.8's
collect-all Wasm batch report and compatible-spec input snapshot reuse, so one
bad package no longer prevents later independent Wasm acquisitions. Configured
deployment builds collect invalid roles before compilation and ask Cargo to
continue across independent package failures.
Exact release publication
also disables implicit followed-tag pushes; the historical-tag deletion helper
now verifies the remote boundary before removing local refs. A disposable Git
fixture covers remote rejection, exact local/remote deletion and non-
resurrection from the cleaned clone; workflow permissions, fixed runners and
CI ownership are guarded. Targeted actionlint, ShellCheck, release/current-
document guards, release-flow tests and plan-only test-lane checks pass; the
complete suite has not been rerun.

Focused `ic-testkit` adoption checks compile the internal harness,
integration package and Saltz test target under the locked dependency graph.
The payload-limit PocketIC suite passes with structured standalone-pool
outcomes and exact cache paths. A warm three-spec lifecycle acquisition reused
one compatible input snapshot: only the first spec resolved Cargo inputs, the
two later specs reported zero input-resolution time, and the focused lifecycle
test passed.

The current follow-up locks `ic-testkit` 0.8.9, hard-cuts the removed anonymous
diagnostic printer, passes exact controllers into labeled collect-all bounded
status/log reports and aggregates same-tick readiness query failures across
deployment targets. The compact failure report now exposes each diagnostic
target's elapsed time and the total sequential batch time. Failed Wasm entries
provide their caller label, index, typed error, primary phase, partial phase
timings and elapsed time directly to Canic's aggregate error. Wasm progress,
success and failure reporting no longer
depend on a parallel package-label slice, and selected-graph semantic identity
permits reuse across unrelated host-only workspace dependency changes. Version
0.8.8 makes the optional cross-call session's source-immutability assertion
explicit. Version 0.8.9 adds a prepared snapshot for concurrent readers, but
that snapshot is process-local while Canic's governed PocketIC lane crosses
several Cargo test processes. Both optional reuse paths require a genuine guard
over every source, manifest, tool and declared input; Canic does not hold one
and deliberately stays on ordinary per-call validation. A deferred
[immutable cross-process test checkout lease](../design/ideas/immutable-test-checkout-lease/design.md)
now owns the possible runner snapshot, read-only process-tree boundary,
external output roots and future upstream prepared-input service. It is a
performance idea, not a 0.104 release gate or implementation authority. Every
repository PocketIC builder now uses bounded instance construction against one
runner-owned server shared across the serial lane. The runner verifies the
exact binary checksum, bounds port readiness, retains startup output in its
private scratch and owns cleanup; a cheap source guard rejects direct unbounded
startup.
On the locked 0.8.9 graph, all-target checks and warning-denied Clippy pass for
the three direct consumers. The focused artifact-policy tests, locked offline
metadata, release-integrity and current-document guards, changelog governance
and the seven-suite PocketIC plan all pass. No broad workspace or PocketIC
runtime suite was rerun for this dependency-only follow-up.
The underlying 0.8.4 migration passed isolated affected-package compile,
warning-denied Clippy, three artifact tests and the two-test payload-limit
PocketIC suite; that cold run finished in 38.99 seconds. After advancing the
lock to 0.8.5, all three direct consumers pass live all-target compilation and
warning-denied Clippy, and the focused artifact tests pass. An isolated
payload-limit rerun compiled and completed its 34.98-second cold Wasm build,
but a concurrently owned PocketIC process prevented a new server from binding;
the run was stopped without claiming runtime evidence. After advancing to
0.8.6, all three direct consumers pass all-target compilation in 36.03 seconds;
warning-denied Clippy passes for the complete internal/Saltz targets and the
three directly affected Canic integration targets, and all three artifact
tests pass. The wider Canic integration Clippy command reaches a separately
owned 0.104 timer test that is 103 lines against its 100-line lint limit, which
this dependency batch does not alter. The payload-limit suite built its new
semantic-key artifact in 28.52 seconds, and the immediate rerun reused it in
436.6 milliseconds. An intermediate use of 0.8.6's managed spawn path then
failed in about 21 milliseconds because the upstream helper
pre-created its `--port-file`; the exact PocketIC 15 binary exits zero and
silently when that path already exists, while a genuinely absent path proceeds
to bind. `ic-testkit` 0.8.7 repairs that upstream spawn boundary with a private
startup directory and absent server-owned port path. The final Canic flow
starts one runner-owned server with a new port path and uses bounded testkit
`connect` calls. All three direct consumers compile on the locked graph, the
four timer-authority journeys pass in 6.89 seconds with one shared server, and
the runner retains and terminates that exact child on every handled exit before
the scratch cleanup fallback runs. Its numeric direct-child port path also
keeps forced cleanup invocation-scoped after an abrupt runner failure. Exact
and deliberately mismatched server binaries are
accepted and rejected respectively, and PocketIC alignment, locked offline
metadata, the workspace inventory/startup guard, release-integrity contract,
cheap current-document guard, focused ShellCheck, formatting and diff hygiene
pass. The complete suite was not rerun.

Focused 0.104 B2 validation removes the public timer facade and compiles the
runtime probe as a direct `ic-timers` consumer. Warning-denied affected-package
Clippy, 1,122 core unit tests, timer inventory guards, pool ownership tests,
facade/endpoint tests and the four-test timer-authority PocketIC journey pass.
The 24-hour simulated window records zero callbacks for idle capability-pruned
cycle top-up, intent cleanup and log retention. The native application
interval reports two work samples, 25,145 latest/maximum and 50,248 total
instructions with zero Wasm or stable-memory page growth; the retained B1
evidence has no numeric pre-cut sample, so this is not presented as a causal
performance delta. Its phase-size table is historical only because the exact
B2 source state was not retained and its alleged baseline was not reproduced.
The complete suite was not rerun.

Focused 0.104 B3 validation passes 1,118 core library tests, exact state and
role contract checks, timer workflow and pool tests, warning-denied affected-
package Clippy, both source-inventory guards and all four timer-authority
PocketIC journeys. The expired-business-lease fixture commits one trapped
continuation, admits one takeover after expiry and clears only the successor's
exact attempt. The interval remains at two work samples with 50,179 total
instructions and zero memory-page growth; its sub-percent differences from B2
are not a causal performance claim. The watchdog takeover records one 21,503-
instruction scheduler sample and one 51,476-instruction work sample with zero
memory-page growth, for which no B2 numeric baseline exists. The phase-size
table is historical only because the B2 and B3 source states were not retained.
All four product builds and the final instrumentation-free interruption
journey pass. The complete suite was not rerun.

Focused 0.104 B4 validation passes 1,118 of 1,119 core library tests with one
ignored, both source-inventory guards, warning-denied Clippy for core, control
plane, facade, runtime probe and the timer-authority target, the real restored-
Root snapshot/resume journey and all four canonical product builds. All four
timer-authority PocketIC journeys passed on the B4 source. The historical
B3/B4 run recorded a representative inventory change from seven rows to five
because an absent `AutomaticTopup` capability and empty receipt index reserved
no native declarations. The
instruction and size tables are historical only because the exact B3 and B4
source states were not retained; no phase improvement is claimed.
The then-current `ic-testkit` 0.8.7 graph reran all four journeys successfully
in 6.89 seconds through one runner-owned shared server. The complete suite was
not rerun.

Focused 0.104 B5 validation deletes the remaining central native-claim union
and snapshot/recovery participant registries. Intent cleanup, log retention
and Root Canister-pool maintenance retain exact native custody; one Root
watchdog dispatches expired core and pool attempts. Lifecycle deferrals are
direct remove-when-stopped claims, while Root and Coordinator authority
snapshots use distinct exact paths so Coordinator does not link Root owners.
The three timer-authority journeys pass in 6.04 seconds on locked
`ic-testkit 0.8.8`; exact Coordinator snapshot/restore passes in 11.49 seconds,
and the real restored-Root journey proves the pool/watchdog rows are scheduled
live, unregistered while sealed, reconstructed after live resume and absent
from a restored sealed snapshot. Targeted compilation, warning-denied Clippy,
source guards, pool/protocol tests and all nine canonical fast artifacts pass.
The original B4/B5 measurement table is historical only because neither exact
phase source state was retained. The complete suite was not rerun.

Focused 0.104 B6 validation passes both public compile-fail examples, the six
lifecycle-boundary guards, the managed endpoint and protocol-surface guards,
and warning-denied Clippy for every touched facade, fixture, harness and
integration target. Managed Prepared/repeated-upgrade behavior, participant
trap rollback with an unchanged committed module hash, corrected retry and the
real Root participant path pass focused PocketIC checks. The closeout
correction also proves an init-participant trap leaves the same canister empty
through a later round before corrected installation succeeds on that exact
principal. The final four-test timer-authority journey passes in 7.86 seconds
on locked `ic-testkit 0.8.8` and
PocketIC 15. The managed, Root and runtime-probe artifacts retain exact B5
Candid and normalized canister-export sets with one `canister_init` and one
`canister_post_upgrade` each. Its two-sample interval records 46,593 total
instructions with zero memory-page growth. The original B5/B6 Wasm comparison
is historical only because the B5 source state was not retained. The complete
suite was not rerun.

Focused 0.104 B7 validation resolves one `ic-timers 0.6.1` package for Canic
and exact published `icydb = "=0.230.2"`. Normal and trapping composition
artifacts retain identical Candid and exactly one lifecycle export pair. The
focused PocketIC journey passes in 19.74 seconds, proving Prepared and Active
same-release reconstruction, distinct Canic/IcyDB rows, participant rollback
with an unchanged committed module hash and corrected retry. The final
test-only probe is 5,959,481 raw and 1,519,923 deterministic-gzip bytes; it
changes no shipped product role. Locked affected-package compilation,
warning-denied Clippy, the lifecycle payload unit check and the workspace-test
inventory guard also pass. The complete suite was not rerun.

The focused B7 closeout follow-up queries the controller-authorized runtime
status and freezes complete sorted logical timer inventories at six Prepared
and Active install/upgrade checkpoints. Prepared contains Canic log retention
and IcyDB recovery; Active additionally contains Canic intent cleanup. Settled
Prepared recovery is retained idle and unregistered, while newly Prepared and
Active recovery is scheduled and active. Exact equality rejects extra,
duplicate or missing rows and scheduling-state drift. The locked PocketIC
journey passes in 11.59 seconds and warning-denied targeted Clippy passes; no
product canister source or interface changed.

Focused 0.104 B8 validation parses the 45-file Rust ownership set. Published
`v0.104.2` freezes native constructors and registration method
actions and rejects aliases, public re-exports, unclassified files and
duplicate custody. It closes the exact one-provider lock and nine-manifest
declaration set. All sixteen structural provider,
ownership, documentation, raw-access, wait and snapshot guards pass. The four
timer-authority journeys pass in 14.01 seconds, and the exact isolated
measurement journey passes warm in 3.24 seconds. It reports four managed rows,
one scheduled, no top-up row and the exact B6 interval result: two work
samples, 46,593 total instructions and zero memory-page growth. The focused
Root restore journey passes in 47.50 seconds and reports four declared rows,
two scheduled while active and zero while sealed, with no top-up row. Exact
release-identity-bearing observations recorded 19,424,848 raw / 5,030,696
gzip bytes for 0.103.0 and 19,124,317 raw / 4,959,729 gzip bytes for 0.104.0,
but their exact release-build-ID inputs were not retained. They are not
independently reproducible closeout authority. The documented no-ID command
produces 19,424,589 / 5,030,663 for 0.103.0, 19,123,930 / 4,959,656 for
0.104.0 and 19,123,917 / 4,959,773 for 0.104.1; those results are not
canonical release-identity evidence. No controlled causal percentage is
claimed.
Targeted warning-denied Clippy passes for the semantic guard, timer target and
internal Root harness. The lifecycle-boundary journeys, changelog governance
and current-document semantics also pass. The complete suite was not rerun.

The human release gate subsequently exposed one inline async-job workflow test
that reset stable storage directly. Its fixture reset now crosses the test-only
`AsyncJobRecoveryOps` boundary; the exact test and repository layering guard
pass. The subsequent complete gate passed every serial PocketIC lane but
exposed four lifecycle-fixture integration failures plus warning-denied Clippy
on that test-only reset helper. The reset now names its exact stable data type;
the lifecycle probe owns a unique App identity, inherits its schema dependency
from the workspace and exports its public snapshot through Canic's managed
endpoint boundary. The checked-in config inventory now includes all 18
configurations. Each previously failing test, all-target core/probe Clippy and
warning-denied compilation of the affected lifecycle integration target pass.
A later complete gate passed Clippy, every ordinary target and every other
PocketIC lane, then exposed that the newly managed fixture query was correctly
fenced while the Component remained Prepared. The proof now observes Init on
one legitimately activated Canister, uses only role-owned status and module-
hash evidence through a second Canister's Prepared upgrade/trap/retry sequence,
and reads its application snapshot only after activation. The exact focused
PocketIC journey passes warm in 9.77 seconds with both Wasm artifacts reused.
The complete release suite was not rerun after this correction.

The follow-up test-harness correction retains standalone package/profile Wasm
bytes once per test process and returns typed, named role-overview observations
from pooled readiness. The formerly failing restored Component Registry proof
now passes with an 835 ms warm baseline restore. With the then-locked
`ic-testkit` 0.8.2 graph warm, the five-test blob-storage suite completes in
12.45 seconds;
its two Wasm acquisitions take 824 ms in total and report zero builds plus two
reuses. This is an observed 3.50-second, 21.9% reduction from the retained
15.95-second warm run, not an isolated causal benchmark. Its separate cold run
completes in 45.42 seconds, including 32.258 seconds of artifact acquisition.
The four-test timer-authority suite completes warm in 7.18 seconds with 414 ms
of artifact acquisition, zero builds and three reuses; its cold artifact build
is retained separately from that runtime signal.

The exact previously failing active-Registry deployment proof passes in 30.81
seconds warm. Store, Root and issuer Wasm reuse take 956, 731 and 475 ms,
respectively. Its 268.25-second cold run includes approximately 165.995 seconds
of one-time artifact builds. This directly exercises controller-bound,
multi-canister readiness diagnostics without conflating the warm deployment
path with compilation.

No production Canic canister source changed in this follow-up, so there is no
product-role Wasm size delta to claim. The touched `runtime_probe` test fixture
remains exactly 3,673,361 raw bytes; deterministic gzip moves from 910,593 to
910,615 bytes, a 22-byte or approximately 0.0024% increase, while the artifact
hash changes with the build-input fingerprint. That is test-fixture compression
noise rather than a shipped-role size regression.

The completed B2-B8 timer checks establish the public surface, stable domain
recovery, fixed-owner custody, pool/lifecycle/snapshot propagation, downstream
adoption, synchronous lifecycle composition and exact IcyDB qualification.
Semantic ownership and measured closeout are complete.

Focused 0.103 B3 validation builds Runtime-only, Sharding/AutomaticTopup,
Root signer/non-signer and Store profiles through the canonical artifact
builder. Exact positive/negative Candid selectors and referenced types pass,
as do the native reserved-`canic_status` collision, incompatible-feature
rejection, cfg ownership and thin-`start!` guards. The complete suite was not
rerun.

Authorized 0.103 B4 work emits only `canic_command` and `canic_status` for Root
and Coordinator, retains exact domain-owned operation identities, enforces
variant-specific payload bounds and advances Component provisioning through a
private reconciler. The Component advance/publication/activation phase variants
and their replay rows are removed. The corrected register admits 32 Root and
nine Coordinator command variants. The three temporary Root Store-staging
variants and five Coordinator snapshot/removal phase variants are deleted with
their callers; pre-adoption Store staging, participant-specific operation
status and the autonomous Component-provisioning and Root-removal PocketIC
journeys pass focused checks.

Focused 0.103 B5-B7 validation covers exact managed/Store caller and Candid
cuts, Store bootstrap/reverification, host/CLI decoding fixtures, the
then-current application presentation, local-only status emission, the current
endpoint allowlist and all role replay manifests. Seven representative
generated services expose two Canic methods per ordinary role and four for
Store. The four-profile Canic
total is ten instead of 188; representative fast-profile Wasm hashes and sizes
are retained only as current artifact identity because no isolated same-source
pre-cut pair exists. Post-closeout cleanup removes raw pre-cut DID snapshots,
regenerates the three still-configured stale local role artifacts and verifies
all 19 remaining repository/build-local DIDs parse with zero retired Canic
method declarations. The focused 35-test protocol-surface suite passes. The
empty `metrics` feature cleanup also passes locked metadata resolution, core
catalog parity, host package-contract and generated Store-wrapper tests,
facade manifest/documentation guards and CLI medic role-contract fixtures.
The complete suite was not rerun.

The standalone cycle-burn waveform idea completed its bounded B0b pulse and
B0c plateau mainnet calibrations on 2026-08-16. Canister
`w47na-gaaaa-aaaad-qmclq-cai` burned exactly `4 Tcycles` on the frozen public
13-node `verified_application` Subnet and retained more than `1 Tcycle`.
Direct-burn visibility passed, but the public Subnet peak rose only
approximately `0.883 Bcycles/second` and stayed elevated across the observed
tail, rejecting the assumed independent approximately 100-second buckets by
approximately a `45.3x` flattening factor. The later exact `3.6 Tcycle`
host-driven plateau proved that repeated bounded input accumulates cleanly,
raising the observed series approximately `0.990 Bcycles/second`. The
maintainer then explicitly promoted a standalone, non-Fleet executor and inert
mainnet install. The complete decay kernel remains open, so its embedded
`4,531`-second rectangular controller is a dated bounded proposal rather than
a Dashboard-fidelity claim. Exact calibration evidence is retained in the
[B0b calibration report](../audits/working/saltz-b0b-calibration/mainnet-calibration.md).

The integer compiler embeds 45 pre-roll plus 864 waveform amounts at exact
100-second deadlines. After two bounded 1× attempts proved timer/receipt
execution but remained too close to public noise, the maintainer authorized an
exact 10× controlled-signal trial. Authorization digest
`e5977055cf691d29353c6649bd464a821475efd66432ff56ea93d76de419ff8d`
can burn at most `958,052,944,723,000 cycles`, but the staged trial binds only
the first 42 pulses and requires `54,924,680,000,000 cycles` at `Arm`. Exact
schedule/unit checks, warning-denied Clippy, strict extracted Candid and
targeted PocketIC funding, authority, 42-pulse exhaustion, timer-burn and abort
evidence pass. Release Wasm
`2388f3f4e38274999682da7a3525d6fbc41724c073c61d16b7c9b253ebecbfc9`
is installed on `w47na-gaaaa-aaaad-qmclq-cai`. Converting exactly
`49.1 Tcycles` cost `30.15060661 ICP`; the exact top-up produced
`55,080,036,485,836 cycles` before Arm. The 10× run began at
`2026-08-17T00:15:00+02:00`. Sixteen exact receipts burned
`20,504,640,000,000 cycles` with no terminal reason. The owning Subnet's exact
600-second public series rose from `0.389` and `0.508` before the controlled
rise to `1.496`, `3.378` and `5.157 Bcycles/second`. The successive controlled
increases of approximately `0.988`, `1.881` and `1.779 Bcycles/second` passed
the multi-bin proportional-response continuation gate.

The exact remaining top-up was `904,073,274,118,831 cycles`. Its discrete
mint deposited `904,073,274,124,352 cycles` and cost `556.42132824 ICP`; the
canister received only the exact requested top-up. After receipt 17 it held
`937,366,738,334,627 cycles` against `936,266,764,723,000` remaining
intentional cycles, preserving `1,099,973,611,627 cycles` for reserve and
execution. The controller identity retained `4,635.83518221 ICP`.

At `2026-08-17T00:57+02:00`, the maintainer clarified that the intended canvas
was the global Dashboard homepage rather than the owning Subnet graph. Its
approximately `45.9 Bcycles/second` global value exposes a material scope
mismatch: the installed controller was calibrated only against the Subnet's
background and kernel. The standing loss-of-confidence rule therefore caused
an immediate controller Abort before chart start. Terminal status is
`Aborted` / `ControllerAbort`, with exactly 26 receipts,
`33,320,040,000,000 cycles` burned and `925,797,679,907,302 cycles` remaining.
No waveform step executed and no later intentional burn is authorized.

Read-only B0e global qualification is active. The exact seven-day pre-run
global series contains 1,009 ten-minute samples with `37.576 Bcycles/second`
median, `37.983` mean, `53.915` maximum and `4.255 Bcycles/second` 95th-
percentile absolute adjacent change. Replaying a provisional
`100..=150 Bcycles/second` trace over every historical day yields strong
correlations. The 26-pulse controlled rise fits a `4,200.842`-second
display-gain denominator with `R² = 0.999475`. The observed tail begins at
timestamp `1786922200`, one 100-second sample after the first pulse reaches
3,600 seconds of age. Its successive approximately `0.3 Bcycles/second` losses
separate the measured gain from 3,600-second visible support. Ten-second onset
evidence shows attribution beginning about 10 seconds after execution and
becoming complete by about 60 seconds; the controller conservatively leads the
label by one 100-second control step. Remaining pulse expirations are still the
acceptance gate.

The local candidate hard-cuts the obsolete 10× scale into a direct global
contract: `30 Bcycles/second` conservative background credit,
`100 Bcycles/second` floor, `50 Bcycles/second` relief, a `4,201`-second gain
denominator, `3,600`-second support, `100`-second control-grid phase lead and
`500 Bcycles/second` hard rate ceiling. Its exact integer plan is 35 pre-roll
plus 864 waveform pulses totaling `9,481,510,455,119,000 cycles` with digest
`dc1cc6ba53470e0f4abf8045224c8a9bb92516b86e458e9238d4428def3e13d9`.
Simulator/unit checks, warning-denied Clippy, Wasm-target compilation, strict
Candid equivalence and targeted PocketIC funding, exhaustion, exact first
waveform receipt, complete 899-pulse execution and Abort evidence pass. This
code remains inert local evidence; it is not reinstall, funding or Arm
authority.

`Arm` is now an immutable staged authority: it can burn only the 35-pulse
pre-roll. Surplus balance cannot enter the drawing. The first waveform pulse
requires a separate exact-digest `AuthorizeWaveform` command whose current
balance covers the remaining pre-roll, first waveform pulse and retained
reserve. Every later pulse separately preserves that reserve. Focused PocketIC
proves an absent or minimally underfunded continuation rejects, a partially
funded continuation stops before its first unaffordable pulse, Abort prevents
later pulses and a fully funded continuation completes all 899 receipts.

The separately authorized global attempt used that exact
`dc1cc6ba53470e0f4abf8045224c8a9bb92516b86e458e9238d4428def3e13d9`
plan. It began pre-roll at `2026-08-17T02:30:00+02:00`, began the chart at
`03:30`, reached receipt 535 and burned exactly
`5,859,496,546,135,400 cycles`. At `16:36:50`, an unrelated burn on Subnet
`brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae`
added approximately `180 Bcycles/second` to the global canvas while the owning
Subnet remained on the expected trace. The maintainer classified the 24-hour
image as spoiled and explicitly ordered Abort. Independent controller status
confirmed terminal `Aborted` / `ControllerAbort`, 535 receipts and no later
intentional burn authority.

**Preserved mainnet asset:** canister `w47na-gaaaa-aaaad-qmclq-cai` retained
`2,589,936,553,122,558 cycles` at final verification, approximately
`2.590 Pcycles`. The controller identity retained only `0.00010000 ICP`.
Those cycles cannot be converted back into ICP, but they remain useful funding
for later authorized canister work. Do not reinstall, delete or otherwise
dispose of this canister without an explicit plan for that balance.

## Next Action

Published `v0.105.0` closes the framework-neutral authorization line. The
maintainer accepted repository-local 0.106 B1 on 2026-08-20: protocol
`canic-0.106-q3q4-v1` is frozen, and the exact empty-topology blocker plus four
Q6 current-state constraints are assigned to 0.110. The operation,
physical-asset, concurrency, fee/refund, reserve and funded-exposure ceilings
plus bounded creation/reset/controller harness and terminal source/dependency
guard pass. Keep B2 held until a separate exact network, identity and
terminal-disposition authorization exists. In 0.107, B2-B7 implementation,
the prior full validation gate and the exact AC12 re-audit/maintainer
acceptance are complete on `ic-query 0.42.0` stable snapshot authority. The
published 0.108.0 checkpoint remains the downstream fresh-install baseline.
The open 0.108.1 B3-B9 closeout corrections, CANIC-019 policy-generation
rotation, funding-domain/fee correction and Registry/offline profile scaffold
are implemented with focused checks. Establish the immutable corrected
candidate,
rerun the complete maintainer-owned validation gate and then run a fresh
human-owned 0.108 closeout audit. Do not add a production pool contract,
run any remote
qualification effect, version, publish or begin 0.109 before the audit verdict
and maintainer-owned release workflow authorize it.
Do not reopen compact diagnostics with JSON, generic handling metadata,
observability infrastructure, compatibility decoding, B1 test coupling or
retired 991 rows.

For the standalone waveform idea, retain the terminal canister and its
approximately `2.590 Pcycles` without further effects. The completed attempt
proved the controlled trace was visible but also proved the global public
canvas can be spoiled by unrelated Subnet burn. Any transfer, reinstall,
deletion, mint, top-up or new Arm requires an explicit disposition or run plan
and separate mainnet authorization; passing local checks cannot supply it.

Design roots retain authority; supporting evidence lives under `docs/audits/`.
