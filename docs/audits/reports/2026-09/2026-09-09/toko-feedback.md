# Toko Miner feedback qualification — 2026-09-09

The four requested investigations have concrete evidence. They are not four
closed acceptance gates: the corrected metrics candidate passes its frozen
ceiling narrowly, downstream runtime adoption remains pending, and staging
recovery has only been reviewed.
The Canic candidate also corrects an originating-error propagation gap found by
this qualification. No version, Git publication or staging effect was performed.

## Source and environment

Canic starts from tagged `v0.110.13`,
`5250d69186047bc373db7cdc7b7680c3aad031d0`, with the open BF2/BF3 build changes.
The CLI candidate SHA-256 is
`2b2ab6be2cc06bb72d679ad017679a0057ca441c6dab422c1c0ad7ae27806934`.
It predates the additional Coordinator reporting fix described below.

Toko Miner was frozen from `f077590a4d2ab4fb85a901a5afab416c2e51dc36` plus its
then-current working tree. It uses Canic runtime 0.110.13 and IcyDB 0.257.3.
Concurrent gameplay edits in the real checkout are not silently substituted
into this evidence. The controlled source inventory is retained in the
[first-build report](build-reuse-first-build.md).

The disposable Fleet uses `/tmp/toko-feedback-fleet`, local gateway port 18014,
and its own XDG identity directories. The maintainer's local network and identity
store are separate. Metrics uses independent native PocketIC fixtures. Files
under `toko-feedback-evidence/` have hashes and original paths in
[manifest.json](toko-feedback-evidence/manifest.json).

## CANIC-149: same-release local reinstall

The downstream wrapper is wired to Canic's preparation/reset flow. It retains
one operation, desired digest and release identity through interruption. A local
acknowledgement receipt survives completion until startup succeeds. Shell
contracts cover changed desired input, foreign reset intent, lost review reply,
retry and acknowledgement. The final cleanup also permits explicit full
`local-reset` to discard that local receipt and checks retained reinstall inputs
before touching the local network. The maintainer approved the cleanup, which
is now applied and passes its focused shell checks.

The real fixture grows to **24 pool assets plus three infrastructure canisters**,
including nine Workloads and 15 Ready assets. Its original seed contains only 15
pool slots. The release is
`43e2cf9b9258fea38f9c10d000621fe28a3f179a0baa2f72ddacd70c22e24b4f`.
The reset operation is
`f3e99f5f119474de0804fefb68962c1595c580ebf1fb32db7d357762c40dbf21`,
with complete reset plan
`dc27eb7d9f53145d13322a1ab9d056326f20b51eb3497bba51ffc2f86726a403`.

Qualification:

1. Enrol a real user, creating User Hub enrolment, User Shard membership and
   Game Shard robot state.
2. Lose the successful Coordinator install response, then interrupt before Root
   installation. Resume the same retained operation.
3. Stop only the disposable Store after provisioning acceptance. Capture Root
   and Coordinator protected failure state; the host retains its stalled work.
4. Restart Store and resume. All 59 effects converge. The physical set of all
   **27 canister identities is identical** before and after. Pool roles can move
   between those retained identities; per-role addresses are not guaranteed.
5. Query every Game Shard: old caller has no robot. Query every User Shard:
   typed `PrincipalConflict` indicates no old membership. Re-enrol the same
   caller using the same request operation ID: the new user ID differs.
6. Replay the completed reset while acknowledgement is pending. The new robot
   and membership remain byte-for-byte equal. There are exactly **three
   infrastructure installs total**, one per infrastructure canister across all
   attempts. Acknowledge the operation and verify the local receipt is removed.

The initial successful terminal observation accounts for 706.781T starting,
zero received funding, 907.887B measured execution burn and 705.873T controlled
remaining. Later read/replay observations update measured burn; they do not
repeat reset effects. The published cycle-conservation equation remains valid.
This exercises the actual wrapper functions, not the full Vite startup command;
frontend acknowledgement ordering has shell-contract coverage.

An earlier 17-canister trial was invalidated by this session funding another
fixture through the same operator during its terminal check. Canic correctly
refused the changed operator balance. It is not counted as a product failure or
successful qualification, and its journal was not edited to manufacture success.

## CANIC-148/153: metrics qualification

All nine nodes and five application roles were measured. Checks cover cycle
publication, anonymous process aggregates, timer work, memory extents, protected
observer denial, cached history and gauge observations without counter deltas.
The existing 80-row Game Shard scenario passes after explicitly completing
Translation catalogue initialization before jumping the clock by sample periods.
Peak instructions are 16,062,610 for Translation, at most 10,679,876 for Game
Shard and below 2,660,000 for the other roles. The earlier unready Translation
trial measured 21,434,209; the readiness change is a fixture correction, not a
runtime performance fix or proof of startup-load budget compliance.

The ceiling fixture adds real IcyDB entities in a disposable source copy, queries
exact primary keys, and includes normal observer entities in the 64-entity total.
Entity names use the real schema maximum of 64 bytes. The actual collector and
producer publish **194 rows**, with maximum public name length 96 bytes; no
synthetic serialized report is timed in the producer. Setup-only endpoints are
not proposed for deployment in Toko Miner.

The unchanged producer exceeds the **20,000,000 instruction** limit: maxima are
22,057,905, 21,922,187 and **22,151,949** across the Game Shards. Its release is
`e5bf66de85a86928ccb19d2d96a842e9b780c2337e89c43aef7aa2c54b3a89fa`.
An exact-capacity metric-name allocation experiment reduces the maximum to
**21,745,176**, still a failure. Its release is
`ac5d24cebbb2cbf135bbb36e1f11d9adbf3590dc3da645b36acaf202df21e1b6`.
The budget and source completeness check were not relaxed. The allocation
experiment was not adopted; the later index correction below supersedes this
candidate result.

Increasing the actual source to 65 entities preserves the prior application
cache and refreshes independent families during that callback. Published Canic
then terminates the timer: `invariant_failure` overrides its next schedule in
ic-timers. Resetting the source to 64 leaves the cached data Stale. This is a
separate, confirmed recovery defect.

The Canic correction classifies rejected optional sampling as retryable. The
existing governed `timer_authority` integration target passes all eight tests,
including repeated rejection, continued cycle history, recovery, suspension and
same-release restoration. Its command is:

```sh
RUSTC_WRAPPER= CARGO_NET_OFFLINE=true make test-pocketic-case CASE=timer_authority
```

A disposable Toko build patches only `canic-core` to that current source. Its
release is `8bfd44f0ebbb3a85240a0b8185b8aae2119e4177a14378c9c748c2d9b87259ac`.
The actual 64→65→64 source sequence passes on **all three Game Shards**: rejected
application rows and timestamps/history stay unchanged, independent Cycles,
Operations and Performance refresh, protected access stays denied, then fresh
194-row publication and gauge history resume. The focused test takes 25.96s.
Strict all-target/all-feature core Clippy and warning-denied Clippy for the
changed integration target pass. This candidate also includes the unadopted
allocation experiment; its peak remains
21,745,173 instructions. Recovery success does not satisfy the separate budget.

Reproduction fixtures and the exact test body are retained as
`metrics-ceiling-fixture.patch`, `metrics-allocation-experiment.patch`,
`metrics-ceiling-test.rs.txt` and `metrics-isolation-test.rs.txt` in the evidence
directory. Apply only to a disposable copy, build with the exact release profile,
and set `TOKO_MINER_QUALIFICATION_RELEASE_BUILD_ID` and
`TOKO_MINER_QUALIFICATION_ARTIFACTS_DIR` when running the named ignored
`canister_toko_miner_user_hub` test. Fixture setup endpoints must never ship in
the application. The candidate core patch is a qualification override, not a
changed downstream pin.

The subsequent [history-index correction](metrics-history-index.md) uses the
unchanged downstream producer and uninstrumented Canic runtime. It passes
300 sample periods (including the full 288-slot window and rollover), then all
three 64→65→64 recovery sequences, at **19,963,567** maximum instructions.
The 20M threshold remains unchanged; headroom is **36,433 instructions (0.18%)**.
This qualifies the frozen fixture, not arbitrary application activity, startup
load, concurrent downstream changes or a published/adopted runtime.

## CANIC-139: controlled first build

A copy with no target or dependency records builds eight artifacts in **355.98s**.
The next unchanged invocation reuses all eight in **3.40s**. All 26,768 inventoried
source entries are unchanged; dependency records increase from zero to 14 and
all 143 newly recorded control-plane source paths were captured before compile.
The [structured result](build-reuse-downstream.json) and linked report preserve
hashes and commands. This validates the candidate mechanism. The original failed
historical invocation has no retained before/after inventory, so its exact cause
is not retrospectively asserted. Publication/adoption and changed-role reuse
remain distinct work.

## CANIC-154–159: retained estate, recovery and origin

Explicit retained seed generation observes all 27 physical identities, including
assets beyond the original seed. Attempting to ordinary-ensure changed authority
on the running Root is refused with E132 before effects. Restoring exact accepted
input permits the reviewed same-release wipe, which independently captures all
24 pool assets. This is not a compatibility or authority-migration path.

During the controlled Store outage, Root remains Accepted and records Provisioning
code **66**, Backoff, exact operation/target and a 60-second retry deadline.
Two observations retain the same receipt while consecutive failures advance
from 10 to 12 over roughly 120 seconds. Coordinator instead reports outer code
132 / RootProvisioning with **no origin**. This exposes a real reporting gap:
the generic wait filter suppressed all Provisioning failures.

The Canic correction preserves that failure while Root is Accepted. It continues
to suppress Coordinator-dependent publication waits, avoiding a publication
cycle. Exact diagnostic, operation, target, retry category and original timestamp
are checked by the focused control-plane regression. Warning-denied
all-target/all-feature control-plane Clippy passes. The full disposable runtime
journey used published 0.110.13 and demonstrates the defect; it does **not** claim
a deployed runtime proof of the new correction.

The subsequent [Canic runtime regression](../2026-09-10/canic159-runtime.md)
now proves the correction in PocketIC: an Accepted-phase Store outage propagates
all origin fields to Coordinator, then the same operation reaches runtime
activation, clears the failure and replays without updates. This closes the
candidate-runtime proof gap without claiming downstream adoption or staging
recovery.

The combined local run proves retention, interruption, bounded retry observation,
terminal conservation and completed replay. It does not exercise every possible
funding tranche or prove the actual staging recovery.

## Staging review: no effects applied

The [exact JSON review](toko-feedback-evidence/toko-feedback-staging-recovery-review.json)
was prepared in the disposable copy using named `toko-miner-mainnet` authority.
A restrictive transport permits status reads and only typed Root
`InspectCanister` calls for the retained assets; that endpoint delegates solely
to management `canister_status`. Installs, transfers and stop/start operations
were refused by the transport. Actual staging journal/desired files were not
modified.

- Fleet: `staging / toko-miner-staging-001`.
- Candidate release: `65773024643542bea6a0115d1a189bea64d37283e6cd46db714021c8297a90f2`.
- Review scope: `reinstall_preparation`; **zero effects applied**.
- Plan digest: `aecbab3d33f9a1247cc46e3c4610901d61ddaa91d902cd6c063d36a871265283`.
- Proposed actions: stop Coordinator `jesds-iaaaa-aaaar-qcbgq-cai`; stop then
  start Root `2ydug-eaaaa-aaaab-qhfca-cai`. Store is unchanged by preparation.
- Reviewed estate: 24 retained pool assets; zero creations.
- New funding/operator debit/fees: zero. Observed controlled balance:
  458,841,010,958,386 cycles. Conservative maximum execution burn:
  219,000,000,000,000 cycles; expected remaining 239,841,010,958,386.

This is approval material for preparation only. The dependent full reset review
must be obtained and assessed after preparation; this report does not preapprove
unknown later actions or funding. The proposed artifact set is the frozen source,
not a claim that current concurrent gameplay changes are deployed.

## Review handoff

The [approved six-file downstream patch](toko-miner-followup.patch) is applied.
[Base/proposed hashes and application record](toko-miner-followup.json) retain
its exact reviewed inputs and outputs: all six bases matched immediately before
application and all six proposed hashes matched immediately afterward. Shell
syntax, ShellCheck and both focused shell contracts pass in the real checkout.
The local-preflight/full-reset cleanup reuses existing operation ownership;
no new mode, schema, compatibility path or duplicate reset flow was added.

The earlier automatic-review refusal was resolved by explicit maintainer
approval. The downstream ledger and required scan log record application;
concurrent gameplay edits remain intact. The patch includes no metrics
allocation experiment, fixture endpoints, dependency update or deployment.

The full requested acceptance set remains open for downstream runtime adoption
and staging recovery/funding qualification. The frozen metrics ceiling now
passes with minimal headroom. Canic's open 0.110.14 notes include BF2/BF3 and the
confirmed runtime corrections, ready for scoped release review. No broad
validation, version change or Git publication ran.
