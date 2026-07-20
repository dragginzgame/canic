# 0.95 Timer Authority And Scheduling Consolidation - Status

Last updated: 2026-07-20

## Current State

0.95 is active. Slice A is released as `v0.95.0` against the exact
`v0.94.14` anchor: every
production timer, lifecycle deferral, retry scheduler, public timer form, and
bounded host wait is inventoried and dispositioned. The public hard-cut
surface, scheduler arbitration rules, owner trigger model, service bounds, and
closeout gate are frozen.

Released `v0.95.1` completes Slice B. All current canister timer owners
route through one common workflow and one one-shot-only IC platform boundary.
Request sequence and generation arbitration, schedule-during-run merging,
after-completion recurrence, consuming cancellation, live status, and the
public hard cuts are implemented and validated.

Released `v0.95.2` begins Slice C with the finite local-intent owner.
One lifecycle-rebuilt stable index contains only finite expiry deadlines;
bounded callbacks follow its exact earliest deadline, while TTL-free intents
leave the process unregistered and idle. The open `0.95.3` batch removes the
pool maintenance interval, makes `pool:pending` the sole event/retry owner, and
corrects intent invariant failure to stop rather than self-retry. Placement
acknowledgement remains separate; log retention is blocked on the count-
authority correction admitted below.

The accepted design now includes a maintainer-approved duration amendment.
Cadences are no longer retained merely because the audit recorded them.
Durations must be semantic zero, an authoritative deadline, bounded retry
policy, explicit safety observation, or application-supplied. Local invariant
failures stop failed instead of polling. Each built-in owner must freeze a
compact decision record covering its inputs, formula, lower and upper bounds,
advance/reset events, failure window, idle cost, and evidence. Pool, placement,
and auth do not inherit their old minute or 30-minute values; cycle safety must
derive its bounds from balance/headroom evidence before Slice D implementation.

## Immutable Baseline

- Release anchor: `v0.94.14`.
- Source commit: `7d5cca4fceae1cb29644b3c1de12cf6a576e0503`.
- Source tree: `2c5155fc8ebbf7a69066f50b4cf1810b264b0071`.
- Product-tree hash:
  `5599ed0e0f6e77b197e63cc4d3bd5bce0ce166ca8390c40f4a87203b89779ce2`.
- Cargo.lock SHA-256:
  `0263c0acf3a2fdd34017ceab6ef528f0d1ab352bf3d1a08a2f1ad1de19f99823`.
- Rust toolchain: `rustc 1.97.0 (2d8144b78 2026-07-07)`.
- Workspace package version at anchor: `0.94.14`.

## Slice A Evidence

The canonical report is
[0.95 timer authority Slice A](../../audits/reports/2026-07/2026-07-20/0.95-timer-authority-slice-a.md).

- Direct IC timer access: one production owner.
- Timer/process families: 14 rows, all dispositioned.
- Bounded host waits: two rows, both retained outside the canister scheduler.
- Public forms: retain cancellable one-shot and after-completion interval;
  remove both unused guarded forms and public raw-CDK access.
- Empty-root baseline: ten background callbacks and 72,303 timer-callback
  instructions in the first 60 minutes and 31 seconds; seven log/pool wakes
  had no authoritative work.
- Permanent source inventory guard: added and targeted for every 0.95 slice.

## Slice B Evidence

- Common control: nine deterministic arbitration and checked-overflow tests
  pass inside the 915-test `canic-core` library run.
- Public behavior: one PocketIC journey proves consuming cancellation,
  exactly-once one-shot execution, after-completion recurrence without missed-
  tick replay, and truthful live status.
- Structural boundaries: both timer-inventory/direct-owner tests and both
  lifecycle synchronous/deferral tests pass; the layering guards pass.
- Boundary projection: runtime DTO Candid/Serde and passive-DTO guards pass,
  and all 17 focused CLI inspect tests consume the new two-dimensional status.
- Strict Clippy passes for all targets in `canic-core`, `canic`,
  `canic-control-plane`, and `canic-cli`; it also passes for the focused
  `canic-tests` journey and `runtime_probe` fixture.
- No stable state, configuration schema, dependency, host wait, backup,
  restore, receipt-reclamation, or Cargo package version changes are present.

## Slice C Intent Evidence

- One stable derived index at core-runtime memory allocation 44 is ordered by
  cleanup deadline and intent identity and rebuilt before timer startup.
- Local-intent and cost-guard workflow authorities maintain that index across
  reserve and terminal transitions; direct production cost-guard mutation is
  structurally limited to the workflow owner.
- Released `v0.95.2` processes at most 32 due intents, continues through a new
  timer message when more are due, retries storage failure after one minute,
  and stops when the index is empty. The open `0.95.3` correction removes that
  invariant self-retry and stops failed while preserving durable evidence.
- PocketIC proves exact scheduling, upgrade reconstruction, finite capacity
  release, TTL-free retention, and truthful active/idle runtime status.
- Intent record and receipt schemas, public Candid, configuration, dependencies,
  and Cargo package versions are unchanged.

## Slice C Pool Evidence

- The open `0.95.3` batch hard-cuts the `pool:maintenance` key and its permanent
  30-minute cadence. Root lifecycle reconstructs only `pool:pending` from
  durable pending-reset rows.
- Empty roots retain `pool:pending` as `unregistered + idle` with zero
  executions. A focused maintained-root PocketIC topology journey proves the
  runtime projection and absence of the removed maintenance row.
- Known producer work and bounded ten-row continuation schedule immediately.
  Only local-build importability failure retries, through the frozen
  1/2/4/8/16/30-minute progression; production IC builds cannot enter that
  probe failure.
- Unexpected policy variants and intent cleanup storage/deadline contradictions
  stop as invariant failures. Durable pool and intent evidence remains for
  lifecycle or operator recovery.
- No stable schema, memory allocation, configuration, dependency, or Cargo
  package version changes are present.

## Finding Index

| Finding | Severity | State | Owner |
| --- | --- | --- | --- |
| `CANIC-095-TIMER-001` async interval overlap | P1 | fixed in released 0.95.1 | common timer workflow |
| `CANIC-095-TIMER-002` stale guarded slot/lost reschedule | P1 | fixed in released 0.95.1 | common timer workflow |
| `CANIC-095-TIMER-003` false live timer status | P2 | fixed in released 0.95.1 | common timer workflow/runtime projection |
| `CANIC-095-TIMER-004` unnecessary idle wakes | P2 | intent fixed in released 0.95.2; pool fixed in open 0.95.3; log remains | log, pool, intent workflows |
| `CANIC-095-TIMER-005` unrelated full scans | P2 | intent fixed in released 0.95.2; placement remains | intent and placement ops/workflows |
| `CANIC-095-TIMER-006` competing mechanics/lifecycle paths | P2 | fixed in released 0.95.1 | timer workflow and lifecycle facade |
| `CANIC-095-TIMER-007` unreachable configured root self-refill | P1 | accepted for Slice D | cycle/top-up workflow |
| `CANIC-095-TIMER-008` log count authority contradicts disposition | P2 | accepted for later isolated Slice C batch | log storage/ops/workflow |

No other product finding is admitted to 0.95 without a design amendment and
reproducible timer-owner evidence.

## Frozen Implementation Order

1. Slice B: fixed identities, sequence/generation arbitration, automatic slot
   consumption, cancellation, after-completion recurrence, live status, one
   lifecycle facade, and public hard cuts.
2. Slice C: log age deadlines, pool events/retries, intent expiry index, and
   placement acknowledgement queue/index.
3. Slice D: independent cycle sample/top-up owners, reachable configured root
   self-refill, exact delegated-proof renewal deadline, comparative costs, and
   cumulative closeout.

Receipt replay/reclamation, Toko integration, dependency work, backup/restore,
and general cleanup remain out of scope.

## Next Action

Release the validated open `0.95.3` pool batch with its intent invariant-stop
correction. Then take placement acknowledgement as the next independent owner.
Keep log retention separate until bounded count and age mutation have one
canonical authority.
