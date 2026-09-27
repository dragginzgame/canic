# Deployment and test latency audit

Date: 2026-09-27. Published base: `0.110.44`; current working batch: `.45`.

## Implemented follow-up: cache attribution and worker qualification

The accepted continuation adds one quiet `CANIC-CACHE` event per completed
literal-zero fixture cache acquisition. It retains upstream coordination/content/
namespace lock waits, input capture, lookup, caller build, output validation,
publication, materialization, maintenance and total durations independently.
Process, optional worker and parent phase identify the owner; build duration is
null on a cache hit. Successful consoles suppress these records, complete logs
retain them, and bounded failure excerpts include them.

The mixed-topology helper's replay groups now have nested `same_plan_terminal_apply`,
`fresh_terminal_plan` and `fresh_terminal_apply` spans. Their parent group remains;
the tests still perform each operation and retain all existing assertions.

One same-binary warmed comparison exercised an actual regular-group completed-reset
case and a journey-group refill case through the production worker launcher:

| Boundary | Serial | Two workers |
| --- | ---: | ---: |
| Complete selected commands | 262.03s | 145.26s |
| Completed-estate reset case | 141.44s | 142.32s |
| Two-Workload refill case | 114.38s | 114.65s |

Both cases pass, including recovery and terminal replay. The selected pair uses
**44.6% less wall time**, with less than one second change in either case. Every
measured fixture acquisition was a verified hit: serial acquisitions took
2.30–2.34s and parallel acquisitions 2.27–3.07s. One parallel acquisition waited
0.51s for coordination ownership. Cold warm-up acquired the source/reset/refill
recipes in 146.72s, 39.57s and 207.69s, overwhelmingly caller build time; it is
excluded from the comparison.

The private comparison harness initially omitted the compiler wrapper on its
parallel path, producing a different recipe and rebuilds. That cohort was stopped
with owned-process cleanup and excluded. The measured parallel rerun explicitly
uses the same repository wrapper as serial. An earlier missing-server-path setup
attempt ran no selected test. Evidence retains these distinctions. A sibling Toko
qualification was active during initial warm-up, and this single comparison does
not control all machine load.

Four native timing tests pass, including a real file-cache miss/hit and retained
parentage. Quiet-runner success/failure tests, scoped warning-denied internal
Clippy, formatting, ShellCheck and whitespace checks pass. The two selected real
cases qualify the cache event and launcher; they do not execute the newly split
mixed-topology replay sections, which have compile and existing Span-owner coverage.
The full catalogue and its balance remain unmeasured. No scheduling or case
membership changed in this continuation, and no full-suite speedup is claimed.

[Qualification evidence](cache-worker-qualification.json) binds the binary, source,
logs, cache fields and timings. Raw traces are copied under
`.tmp/cache-worker-audit45/` outside target cleanup. Both `.45` changelogs include
the diagnostic change; the complete current batch remains ready for review and
the chosen release gate. Package versions stay `.44`; changes are uncommitted.
The read-only audit below records the preceding investigation.

## Feedback check

Read Toko Miner's current `docs/upstream/canic.md` and Canic's open GitHub issues
and recent comments. No new actionable request follows the already implemented
CANIC-183 receipts, CANIC-150 named progress and CANIC-160 transient-read retry
corrections. Toko is qualifying published `.44`, which excludes those dirty `.45`
changes. CANIC-176's matched application-build measurement and downstream adoption
remain open. The `ic-testkit` PID request is delivered in `0.10.1`.

This audit reads source and retained evidence. It does not execute deployment,
build a new matrix, run a suite or modify downstream repositories. Extracted
timings, source hashes and evidence limitations are in
[deployment-test-latency-evidence.json](deployment-test-latency-evidence.json).

## What the 55 minutes means

The retained local `.44` report records **3,304 seconds (55m04s)** for the test
stage, of which **2,934 seconds (48m54s, 88.8%)** is internal PocketIC. Everything
else totals 370 seconds. This is neither complete release-command duration nor
Toko deployment duration. The original `target/validation-runs` logs have since
been cleaned; this audit retains the prior report's result rather than claiming
to rederive it from missing raw logs.

The current `.45` two-worker runner and fixture consolidation were implemented
after that measurement. The controlled refill/repair pair is 188s serial versus
114s parallel; the complete default `.45` suite has not been timed. Applying
that pair's 39.4% reduction to the whole release would be unjustified.

The separate [`.44` CI PocketIC job](https://github.com/dragginzgame/canic/actions/runs/36327889446/job/108645249435)
was cancelled after 60 completed cases and an unfinished native-funding journey.
It is useful for case attribution but cannot establish a full-suite duration.
It also contains cold Wasm builds and has different hardware/cache conditions
from the local 55-minute run.

## Findings and recommended sequence

### 1. Qualify and balance the two workers against the actual catalogue

`pic::governed_suite::worker_groups` puts registry, Coordinator and lifecycle
cases on one worker and eight Fleet journeys on the other, after the source-bound
recovery barrier. The first group is not a short-test group: its 56 cases consumed
**2,573.94 seconds (42m54s)** in the serial `.44` CI trace. The recovery prefix
took another 435.49 seconds. Selected large cases were:

| Case | Observed CI duration | Current group |
| --- | ---: | --- |
| Completed-estate reset | 333.76s | Registry/Coordinator/lifecycle |
| Fleet deployment restore | 205.89s | Registry/Coordinator/lifecycle |
| Initial-child failure and same-claim recovery | 195.34s | Registry/Coordinator/lifecycle |
| Published managed-App support | 168.17s | Registry/Coordinator/lifecycle |
| Autonomous Root removal | 156.25s | Registry/Coordinator/lifecycle |
| Four initial Shards | 378.19s | Fleet journeys |

These old durations identify where to investigate. They do not prove current
imbalance: `.45` changes some cases, the remaining CI journey suffix is incomplete,
and compilation is included in some case spans. The first qualification should
report both worker wall times, case durations, build/cache work and memory peaks
from the same candidate. Keep two processes initially. If the current partition
is skewed, redistribute independently owned fixture groups using those measurements;
preserve within-group dependencies, the recovery barrier and complete membership.
Some registry tests use a pooled baseline, so moving arbitrary individual tests
may introduce cold setup and erase the saving.

Acceptance: exact registered membership, no repeated or omitted cases, recovery
first, cancellation/cleanup on either failure, and a matched slow-case pair in
both partitions. Use the maintainer's next selected release gate to measure the
whole suite; this audit does not authorize or start that gate.

### 2. Separate artifact waiting from execution before changing scheduling again

The existing cold parallel trace contains a **379.24s `artifact_cache_lookup`**
span and a separate 267.02s build/seal span. The helper in
`baseline/tests/release_artifacts/mod.rs` measures all of
`prepare_artifact_cache` as lookup. `ic-testkit::ArtifactCacheTimings` already
exposes coordination, content and namespace lock waits. Canic currently reports
an aggregate cache duration and maintenance details at this boundary.

The next small diagnostic change should retain those existing timing fields in
the quiet trace, alongside worker/case identity and cache outcome. A long lookup
then becomes attributable to locking, verification or other preparation. The
379s observation alone does not establish which category dominates. Only then
decide whether distinct cold recipes should be prepared once before execution or
grouped differently. Preserve exact input/output checks and retained cache handles.

CI also sets `cache-targets: false` for every Rust-cache job. That intentionally
leaves native target compilation outside that cache; ordinary CI took 967s while
the prior local ordinary stage took 78s. Those totals include different work/cache
conditions. Profile native compile cost separately and evaluate a bounded compiler
cache against runner disk limits before retaining large target directories.

### 3. Audit repeated observation and setup within the expensive journeys

Current warm traces provide more useful execution evidence than cold CI totals:

| Current fixture | Case duration | Completed management-status request events | Inclusive status-request time |
| --- | ---: | ---: | ---: |
| Mixed topology, changed-build reset | 355.66s | 475 | 105.96s |
| Small reinstall including same-build repeat | 234.61s | 426 | 94.13s |

Request durations can overlap and nest; they are not additive wall-time savings.
These counts cover several plans, interruptions, resumes and terminal checks.
They do not prove that 475 or 426 reads can be removed.

`IcpEnsurePlatform` already has scoped read reuse, invalidation at effects and
backoff, fresh install-preparation observations, and bounded independent reads.
The useful next investigation is an exact repeated-key trace *within each existing
read-only scope*. Any reuse must bind the same network, caller, subject and
authority and expire before mutations, retries and new invocations. Broadening
cache lifetime across an install or handoff would weaken the audited flow.

The mixed fixture's 27.75s `initial_terminal_replay` span contains same-plan apply,
new plan, second apply and assertions. Its 25.54s `reset_terminal_replay` similarly
includes fresh planning. Split these existing spans before diagnosing replay
latency. Preserve same-operation replay and fresh-plan validation as separate
contracts. If repeated fresh-plan coverage is consolidated, keep it in an exact
current case that still asserts controller authority, conservation and no effects.

The four-Shard test creates ten pool assets: five workloads plus five Ready
reserves. Inspect whether all five Ready assets are necessary for its *activation*
invariant or whether the reserve policy forces them. A smaller dedicated activation
fixture is a candidate, not an established saving. Keep multi-child ordering,
sealed Root activation and replay in PocketIC; move pure capacity arithmetic only
to native tests. The `.44` CI case includes cold compilation, so 378s is not its
removable execution cost.

### 4. Avoid expensive work after an ordinary CI failure

`tests-ordinary` and `tests-pocketic` both depend only on `checks` in CI. The local
runner's ordinary-test failure barrier therefore does not span those jobs. On
this run, ordinary tests failed at 15:22:45 UTC; PocketIC continued until
cancellation at 16:36:23 UTC, over 73 minutes later. This was useful read-only
evidence for the audit but wasted qualification work for a candidate already red.

Consider making PocketIC depend on successful ordinary tests, or moving the
cheap graph/inventory guards before the expensive jobs while retaining their
ordinary coverage. The first option delays green-run PocketIC startup; the second
needs a maintained targeted selector. Choose based on desired time to green versus
cost after failure. A dependency change alone does not shorten a successful run.

## Deployment flow assessment

The Toko live evidence identifies **991.33s** for eight Release Wasms, **83.626s**
for desired generation, and **1.056s** for ordinary terminal replay. The frontend
publisher separately spent **371.147s** reinstalling/syncing assets. Reset apply
and resume lack complete wall-time receipts in the released CLI; the pending
CANIC-183 correction supplies that measurement boundary for the next required
deployment. These figures cannot be summed into an end-to-end deployment duration.

The supported flow remains preparation/review, current build and generation,
reviewed reset apply, exact-digest resume when needed, terminal conservation and
replay, followed by the frontend publication owner. Preserve original evidence,
typed authority, intent-before-effect, bounded funding and lost-response
reconciliation. The audit found no basis to remove those checks or claim a new
deployment-speed improvement. The strongest measured application cost is build
work; continue the matched current-source build measurement described in
[the operator feedback report](toko-operator-feedback.md#canic-176-disposition).
Frontend upload performance belongs to the downstream publisher.

## Outcome

Prioritize worker balance and artifact-wait attribution before adding workers or
cutting more recovery tests. Then target repeated setup/observations with exact
coverage ownership. The 55-minute baseline warrants this work, but it does not
measure pending `.45`. This audit changes documentation/evidence only and adds no
new release gate or claim that the entire suite is faster. Existing `.45`
corrections remain ready for review and the maintainer-selected release gate.
