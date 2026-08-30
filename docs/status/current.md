# Current Status

Last updated: 2026-08-30

## Purpose

This is the compact handoff for the active Canic source and roadmap. Read this
file first, then follow only the linked design, audit or implementation owner
needed for the task.

Historical handoffs:

- [through 2026-06-30](archive/2026-06-30-precompact.md);
- [through 0.90.2](archive/2026-07-13-precompact.md);
- [through 0.101.52 Q4](archive/2026-08-12-precompact.md);
- [through published 0.109.12](archive/2026-08-26-pre-root-repair-hard-cut.md);
  and
- [pre-reorientation 0.109.24 handoff](archive/2026-08-30-pre-roadmap-reorientation.md).

## Published Release Truth

`v0.109.26` is the immutable maintained release. The annotated tag object
`aeeb7b4624aec64a1ad2e4815d78494d643889ed` peels to release commit
`1f99da17b8627cd482c6d8677f5490d4a3a0964a`; workspace packages, `main` and
tracked `origin/main` agree. Its complete validation marker binds source
`75b243f429c07b011fa11689947480169866ed00`.

The former handoff incorrectly called 0.109.24 an unversioned draft and 0.109.23
the maintained release. That recurring release-evidence defect is downstream
`CANIC-014`. This handoff now treats structured version, tag, package and
validation records as release authority; narrative is a summary only.

`0.109.27` is an open changelog draft for the current source batch. It is not a
versioned workspace, tag, published package or deployment.

The current support slice advances the composed lifecycle test fixture to
published IcyDB `0.249.1`. All six IcyDB packages remain confined to the test
canister/schema graph, and Canic's published packages remain IcyDB-free.
It also corrects two `canic-host` PocketIC proofs that had drifted into the
parallel ordinary lane: generated pool-recovery and Toko-shaped Fleet Ensure
qualification now use the bounded shared-server serial lane, whose CI job owns
the required Wasm target.

`CANIC-094` is part of the same open support batch. An exact predecessor A
sealed to requested successor C still rejects a later successor D; the seal is
not retargetable. The rejection is now a typed
`SealedSuccessorConvergenceRequired` result carrying both C and D release-build
and Root artifact identities. It directs the operator through retained-C plan
and apply, terminal C proof, D generation and fresh D planning as separate
review boundaries.

`CANIC-095` then blocked the authorized retained-C apply before its first
effect because ICP CLI 1.3.0 omits `canister_version` from status JSON. The open
batch preserves the pre/post reinstall-version invariant and obtains the exact
missing value from a typed management-canister `canister_status` response. Its
agent route calls `aaaaa-aa` while setting the install target as the effective
canister ID required by the IC interface. The selected ICP environment supplies
the resolved API URL and root key. The exported selected identity is
Principal-checked, and its PEM buffer is zeroed after identity construction.
The response boundary decodes the IC interface's exact `version : nat64` field
and projects it into Canic's internal `canister_version`; it does not ask the
management canister for a nonexistent `canister_version` field. The fallback
binds module and version from one response rather than joining two snapshots.
It neither defaults nor infers a version. If either observation boundary fails,
the typed diagnostic confirms no install ran and directs resume of the same
reviewed plan after controller/management access is restored.

## Published 0.109.26 Correctness Batch

Published 0.109.25 closes `CANIC-091`: a retained schema-1 state with cycle
evidence but no Principal/topology maps can review and apply one exact
management-bound Root Start before protected child observation.

Downstream review then exposed `CANIC-092`. Production had live stopped
predecessor A, retained desired release/artifact B and newly requested finalized
release/artifact C. The first 0.109.26 candidate incorrectly required its
generator authority for C to equal retained desired B.

The exact corrected prerequisite was subsequently reviewed and applied once.
The Root now runs predecessor A, but its protected pool response legitimately
predates the current `recovering_ledger` field. `CANIC-093` exposed that both
post-Start generation and ordinary successor planning still decoded that exact
predecessor response as current release C and therefore stopped before any
further effect.

The published `canic-host` batch established this corrected invariant:

1. management-observe A with exact Root Principal, Subnet, controllers and
   stopped state;
2. atomically retain one typed generator authority binding A to the exact
   current Fleet ID and newly requested finalized release/successor C;
3. independently load C's finalized current-release and infrastructure
   manifests and re-read the exact manifest-bound raw Root Wasm;
4. use retained desired B only for stable Fleet and Root identity, leaving its
   bytes unchanged and never loading B's old release manifest or Root artifact;
5. embed the authority into a content-addressed plan scoped only to same-ID
   Root Start, with zero install, replacement, creation, funding, transfer,
   fee or operator-debit authority;
6. reject missing/tampered authority, wrong release or successor, and changed
   live predecessor identity before a plan or effect;
7. apply the Start once and make lost-response/terminal replay effect-free; and
8. after the Root runs, select a narrow predecessor pool-status projection only
   when the sealed A-to-C authority, exact live module and finalized successor
   artifact all match;
9. normalize only the semantically absent predecessor `recovering_ledger`
   count, while requiring every other response field and keeping current Roots
   on the current Candid/DTO path; and
10. retain the observed Store/pool identities and cycles through generation C,
    then produce an ordinary reviewed successor plan with no creation, funding,
    transfer, deletion or operator debit.

The production-shaped regression keeps A, B and C distinct. It proves
deterministic authority retention, finalized release and raw-Wasm verification,
one zero-debit Start without B's release manifest/artifact, unchanged
desired/state ownership and effect-free replay;
missing/tampered authority, wrong release/successor, Fleet, Principal, Subnet,
controller, predecessor-module and runtime drift reject. Exact predecessor
response bytes decode only under that authority; missing fields other than the
one known absent inventory count fail closed. The same journey completes
generation C and ordinary no-apply successor planning without paid or
identity-changing authority.
Targeted validation evidence is recorded below; no broad workspace or
PocketIC gate is run during coding.

## Safety State

The retained downstream evidence reports:

- default ICP identity restored to anonymous;
- desired Fleet authority byte-identical;
- the exact Root Start applied once under its reviewed prerequisite and replayed
  without a second effect;
- 84,279,333,025 cycles of measured Root execution burn, with zero funding,
  transfer, fee or operator debit;
- the same Root Principal now running predecessor module A;
- finalized release artifacts and the sealed A-to-C authority retained;
- no downstream state/archive edit or synthetic topology authority;
- no later canister, controller, cycle, database, catalogue or frontend effect;
  and
- no checksum, optimizer, size or authority bypass.

Canic does not authorize or perform downstream effects from this repository.
The Canic regression creates the new digest-bound Root-start authority only in
its disposable fixture directory.

## 0.109 Closeout State

0.109 remains open. Functional Fleet-wide admission is retained, but the
[binding post-implementation complexity audit](../audits/release-lines/0.109-post-implementation-complexity-audit.md)
still has `closeout_verdict: fail` and no accepted immutable superseding pass.

Required order:

1. finish and publish the current IcyDB test-support slice through the
   maintainer-selected release flow;
2. adopt that exact release downstream and complete CI, release preparation,
   reviewed no-effect planning and terminal/effect-free replay evidence;
3. close B8 with the maintained public operator/adoption loop;
4. execute B9 pure simplification: localize decisions, decompose the three
   gravity wells, retain this handoff below 250 lines and freeze a bounded
   PocketIC time/RSS/process/case envelope;
5. reconcile and complete B10's already-published managed-App qualification
   surface without adding runtime authority;
6. rerun the canonical methods on one immutable candidate; and
7. obtain the maintainer-requested and accepted 0.109 closeout verdict.

No 0.110 implementation begins before that human-owned closeout.

## Reoriented Roadmap

Toko Miner is the primary read-only real-application steering source. Canic
retains repository-owned fixtures and never gains a Toko or IcyDB production
dependency.

| Line | Accepted purpose | State |
| --- | --- | --- |
| [0.110](../design/0.110-fleet-runtime-contraction-and-stateful-safety/status.md) | Contract release builds, endpoint/runtime code, control-plane/operator paths and validation; then add stateful-retirement safety | Accepted reorientation; blocked on 0.109 closeout and promotion |
| [0.111](../design/0.111-stateful-fleet-release-adoption/status.md) | One exact whole-Fleet stop-the-world predecessor/successor transition under inherited budgets | Accepted; blocked on 0.110 closeout and promotion |
| [0.112](../design/0.112-bounded-multi-fleet-estates/status.md) | Indexed estates, an ordinary reserve Fleet and one same-Subnet single-asset cross-Fleet transfer | Accepted reorientation; blocked on 0.111 closeout and promotion |
| [Fleet Observatory](../design/ideas/fleet-observatory/status.md) | Host/downstream-first passive observation without an assumed every-role runtime protocol | Deferred unnumbered idea |

### 0.110 Steering Facts

- `CANIC-014`: release truth must be structured; handoff prose is not an
  independent publication authority.
- `CANIC-087`: eliminate release-LTO declaration links and serial compatible
  runtime links while preserving canonical artifact/determinism gates.
- `CANIC-090`/`CANIC-091`/`CANIC-092`/`CANIC-093`: a prerequisite effect may
  short-circuit unavailable protected observation only under exact management
  and retained module authority plus mandatory post-effect revalidation; any
  narrow predecessor response projection remains exact-module/release bound and
  cannot become permissive current decoding.
- Endpoint-heavy Toko evidence: Binaryen has converged; shared non-generic
  wrappers and role pruning must supply at least 350 KiB useful current-profile
  code-section headroom, with 500 KiB preferred.
- Managed roles already expose their application package version, Canic
  framework version and IC `canister_version`, but the Fleet list currently
  projects only Canic version and module hash. 0.110 B4 owns a host-only
  verified per-role version inventory: semantic versions are operator labels,
  exact observed hashes remain deployment authority, and unknown or conflicting
  mappings fail visible rather than being guessed.

0.110 does not inherit unresolved 0.109 work. It makes the accepted reductions
durable budgets and adds only stateful retirement as a new safety capability.

### Deferred Scope

Adaptive creation/reset lanes, transfer batches, broad automatic estate
funding and 1,000-canister qualification are unscheduled. The former 0.112
runtime Observatory is deferred because a new cross-role projection plus
HTML/JSON adapters on every role conflicts with current size and complexity
evidence.

## Validation State

Targeted warning-denied `canic-host` all-target Clippy passes. The exact
predecessor-response decoder regression passes, including fail-closed rejection
of an incomplete response. The production-shaped retained-estate
generator/ensure journey passes with distinct A, B and C, all authority
negatives, exact old response bytes, generation C and an ordinary successor
plan carrying no creation, funding, transfer or deletion. Formatting and diff
hygiene, layering, changelog governance, release-draft preflight and the
current-document semantics guard pass. The evidence intentionally excludes a
broad workspace or PocketIC gate during coding.

For the open 0.109.27 support slice, locked all-target compilation and
warning-denied Clippy pass for the composed IcyDB fixture and schema. The exact
timer/dependency graph guard resolves all six IcyDB packages at `0.249.1`, one
`ic-timers 0.7.0` provider and no production Canic consumer. The governed
targeted PocketIC lifecycle journey passes install, startup recovery, upgrade,
retained state and timer-custody restoration in 134 seconds with a 404,468 kB
server high-water mark. No broad workspace suite was run.

The focused retained-estate generator regression extends its distinct live A,
retained B and sealed C identities with later release D. It proves that D fails
before a protected predecessor query, receives exact typed C/D diagnostic
fields and cannot change the retained A-to-C authority. The exact regression
passes in 0.60 seconds after a 26.38-second incremental compile, and
warning-denied `canic-host` library/test Clippy passes in 9.82 seconds. No broad
gate was run.

The exact ICP CLI 1.3.0-shaped versionless-status regression passes in 0.01
seconds. An independently declared fixture encodes the canonical management
response field `version : nat64`; production decoding projects version 42 and
the same-response module hash into Canic's install evidence. It rejects when
both sources are unavailable and proves the obsolete generic ICP CLI management
call is absent. The adjacent agent-boundary regression passes and asserts the
management destination and target effective canister ID independently.
The reinstall replay regression passes in 0.07 seconds: its first version
observation failure leaves the exact journal `in_progress` with zero effects,
then the same digest records the pre-version, installs once, proves a newer
terminal version and produces an effect-free terminal replay. No broad gate
was run. After the routing correction, warning-denied `canic-host` library/test
Clippy passes in 13.92 seconds and `canic-cli` all-target compilation passes in
52.61 seconds.

`CANIC-034` is already closed by the maintained fresh-estate creation graph:
each Root pool asset is funded directly by its reviewed Cycles Ledger creation
action with exact creation and Ledger fees, so no Root-ledger bootstrap or
parallel funding authority is needed. `CANIC-087` remains sequenced to 0.110
B2 and is not pulled across the human closeout gate.

## Next Authorized Action

Finish targeted review of the open 0.109.27 support batch, including the
CANIC-094 successor-order diagnostic and CANIC-095 typed install-version
transport. Do not modify Toko Miner from this repository. After a maintained
successor is adopted downstream, resume only the unchanged authorized C plan
if its retained digest still verifies, reach terminal convergence, reuse the
finalized D build, generate and review a fresh D plan, then prove immediate
replay effect-free. Only then begin 0.109 B9 simplification and its superseding
audit.




<!-- canic-release-validation: version=0.109.27 source=ef3acc17c865a939d52f3d8376e8725abe598ba6 date=2026-08-30 gate=complete -->
