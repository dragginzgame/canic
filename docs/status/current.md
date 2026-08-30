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

## Release Evidence Contract

Release truth comes from the workspace package version, dated root and detailed
changelogs, annotated Git tag and release commit, complete published package set,
and the governed validation marker at the end of this file. The version bump
transaction owns that marker and binds the candidate version, exact validated
source and gate. This handoff deliberately does not restate a mutable "latest
release" version, tag object or release commit in narrative prose.

This closes the recurring release-evidence defect tracked downstream as
`CANIC-014`: future releases cannot make this handoff contradictory merely by
advancing beyond a manually copied latest-release sentence. Version-specific
sections below describe their named published batches and remain historical
facts; current source-development state comes from Git and the worktree rather
than a manually rotated status marker.

Published 0.109.27 advances the composed lifecycle test fixture to
published IcyDB `0.249.1`. All six IcyDB packages remain confined to the test
canister/schema graph, and Canic's published packages remain IcyDB-free.
It also corrects two `canic-host` PocketIC proofs that had drifted into the
parallel ordinary lane: generated pool-recovery and Toko-shaped Fleet Ensure
qualification now use the bounded shared-server serial lane, whose CI job owns
the required Wasm target.

`CANIC-094` is part of the same published support batch. An exact predecessor A
sealed to requested successor C still rejects a later successor D; the seal is
not retargetable. The rejection is now a typed
`SealedSuccessorConvergenceRequired` result carrying both C and D release-build
and Root artifact identities. It directs the operator through retained-C plan
and apply, terminal C proof, D generation and fresh D planning as separate
review boundaries.

`CANIC-095` then blocked the authorized retained-C apply before its first
effect because ICP CLI 1.3.0 omits `canister_version` from status JSON. The
published correction preserves the pre/post reinstall-version invariant and
obtains the exact missing value from a typed management-canister
`canister_status` response. Its
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

Downstream application of that reviewed plan exposed `CANIC-096`: after three
same-identity infrastructure reinstalls and two Starts completed, the
`replan_required` boundary retained cycles but discarded its exact projected
Principal/topology state. The open 0.109.28 batch writes that state before the
nonterminal journal marker, so a fresh process can continue from the same live
Root-owned estate without manual reconstruction.

`CANIC-097` then showed that ICP CLI 1.3.0's version-less ordinary status could
repeat all three proved reinstalls despite their applied journal records. The
correction retains an exact operation/action-hash/pre-version proof and checks
the current Principal, Root/parent, kind and desired module independently.
Directly observed canisters retain their live module. A Root-owned Store may
use the exact content-bound journal proof only while the same Root reports the
same Store Principal under the retained topology. No version is defaulted or
inferred. Changed or incomplete retained authority returns a typed conflict
before an install instead of silently suppressing or repeating the effect.

## Open 0.109.30 CANIC-100 Hard Cut

Published `v0.109.29` closes the retained E132 deployment blocker described
below. The next source batch removes the separate pre-1.0 compatibility residue
identified by the downstream hard-cut audit.

The maintained state manifest now describes only the current v1 state contract:
domain ownership, storage, memory IDs, record/snapshot identity, restore order
and current lifecycle invariants. Generic support windows, migration policies,
migration edges and migration-path/upgrade-test audit logic are removed rather
than deprecated. The diagnostic state manifest and audit remain read-only
current-contract tools.

Runtime introspection and Component Group canonical graph identity are reset in
place to schema/domain v1. No decoder accepts their former values. A narrow
executable-source guard rejects new Canic-owned schema, protocol, manifest,
format, wire or config versions above 1 and named migration/legacy readers. The
guard's synthetic permitted case keeps external-tool versions and immutable
audit-method revisions outside product compatibility policy.

This is `CANIC-100`; it is required before 0.110 promotion but is independent of
the already-published downstream deployment unblock.

## Published 0.109.29 CANIC-098 Correction

Published `v0.109.28` was the immutable predecessor. The retained downstream
journal proves its Coordinator reinstall Applied, followed by one predecessor
`AdoptStore` intent that synchronously rejected with typed E132 and retained no
receipt. Store and Root were not reinstalled and no paid or identity-changing
effect followed.

The narrow correction keeps controller preparation separate from successor
protocol adoption. It accepts only the exact rejected request and typed
diagnostic, preserves the immutable action/hash sequence, marks the old
operation `replan_required` without claiming adoption succeeded, and retains
the proved Coordinator reinstall. The mandatory predecessor Root and Store
module identities come from exact live management observations when retained
topology lacks those hashes. Any retained hash that is present must match the
live observation exactly; a missing live hash or a live reviewed-successor hash
fails closed and no value is defaulted. The fresh exact-controller plan
schedules no Coordinator install, funding or debit; it reinstalls Store then
Root and only then adopts Store through the initialized successor Root.
Wrong-controller evidence remains a typed pre-effect blocker; automatic
controller repair is not a release gate for this downstream estate.

The production-shaped regression begins with both retained module hashes
absent, applies that fresh plan, reaches terminal cycle conservation and
immediately replays from a fresh process with zero effects. B9 source is
preserved separately and is not present in this patch. No 0.110 or later work
may delay validation, publication or downstream adoption of this correction.

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

The isolated `CANIC-098` source passes host formatting, its exact retained-E132
journey and warning-denied `canic-host` library/test Clippy. The journey omits
both retained predecessor module hashes, requires exact live observations,
applies the fresh Store/Root plan, performs successor-only adoption, conserves
cycles and immediately replays without effect. No broad workspace or PocketIC
gate was run during this implementation pass.

The production-shaped retained-estate journey now applies three exact
infrastructure reinstalls and two Starts, persists a nonterminal state, and
replans from a fresh process using ICP CLI 1.3.0-shaped status without a
version, a Root-owned Store and two zero-ledger `PendingReset` imports. The
successor contains no Coordinator, Root or Store install, no new funding and
no operator debit. Focused operation/action/version and
Principal/Root/parent/kind/module negatives pass, as do the remaining typed
transition, terminal convergence and effect-free replay portions of the same
journey. No broad gate was run during coding.

Targeted warning-denied `canic-host` all-target Clippy passes. The exact
predecessor-response decoder regression passes, including fail-closed rejection
of an incomplete response. The production-shaped retained-estate
generator/ensure journey passes with distinct A, B and C, all authority
negatives, exact old response bytes, generation C and an ordinary successor
plan carrying no creation, funding, transfer or deletion. Formatting and diff
hygiene, layering, changelog governance, release-draft preflight and the
current-document semantics guard pass. The evidence intentionally excludes a
broad workspace or PocketIC gate during coding.

For the published 0.109.27 support batch, locked all-target compilation and
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

`CANIC-030` is closed without a new Canic guard or product surface. The retired
demo's externally funded canister remains documented in its
[immutable disposition audit](../audits/reports/2026-08/2026-08-24/saltz-mainnet-calibration.md),
including its last verified cycles, controller condition and requirement for a
separately authorized external disposition. Keeping that evidence is necessary;
making every future Canic release validate a retired one-off asset would be
stale product coupling.

Current non-runtime maintenance closes the source-level `CANIC-048` test
portability defect. The backup special-artifact rejection fixture now uses a
FIFO instead of an AF_UNIX socket, retaining the non-regular-file safety proof
without inheriting `SUN_LEN`. The exact test passes under the ordinary test
scratch path and an intentionally overlong `TMPDIR`; no production backup or
restore behavior changes.

Current host maintenance also closes `CANIC-036` across every maintained ICP
process-launch shape. Captured-output and inherited-terminal launches now use
one bounded retry primitive for transient Linux `ETXTBSY`; all other launch
errors return immediately and inherited stderr streaming/capture remains
unchanged. Both held-writer regressions pass, and warning-denied `canic-host`
library/test Clippy is clean. The removed output-to-file launcher is not
restored.

## Next Authorized Action

Complete the targeted `CANIC-100` hard-cut batch and hand it to the
maintainer-selected validation/version/publication flow. Do not modify Toko
Miner from this repository. Downstream may adopt published `v0.109.29` and
resume its exact retained operation independently. B9 remains separate, and no
0.110 implementation begins before the human-owned 0.109 closeout.






<!-- canic-release-validation: version=0.109.29 source=baf6b319b00e6369fd3e6790454ee48819bcf234 date=2026-08-30 gate=complete -->
