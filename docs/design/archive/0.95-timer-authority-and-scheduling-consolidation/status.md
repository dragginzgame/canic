# 0.95 Timer Authority And Scheduling Consolidation - Status

Last updated: 2026-07-21

## Current State

0.95 is closed at `v0.95.10`. Slice A was released as `v0.95.0` against the
exact `v0.94.14` anchor: every
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
leave the process unregistered and idle. Released `v0.95.3` removes the
pool maintenance interval, makes `pool:pending` the sole event/retry owner, and
corrects intent invariant failure to stop rather than self-retry. Released
`v0.95.4` gives placement acknowledgement a durable terminal-only index and
pending-only bounded recovery. Released `v0.95.5` completes Slice C with
append-owned count enforcement and exact deadline-driven log age retention.
Released `v0.95.6` begins Slice D by separating event-owned cycle history from
one configuration-gated automatic-funding safety owner. Released `v0.95.7`
makes that owner nonroot-only, hard-cuts automatic root ICP
conversion, reduces the maximum observation gap to one hour, and binds
successful child requests to parent funding cooldown. Released `v0.95.8`
replaces the remaining fixed auth recurrence with exact durable refresh,
in-flight, expiry, and typed retry deadlines. Released `v0.95.9` addresses the
closeout audit's topology/funding race, checked cycle-deadline requirement, and
obsolete role-attestation timer route. Released `v0.95.10` fixes the final
measured same-round hierarchy ordering defect and records the cumulative
owner/cost evidence.

The immutable closeout evidence is
[0.95 release-line closeout](../../audits/release-lines/0.95-closeout.md).

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
  and stops when the index is empty. Released `v0.95.3` removes that
  invariant self-retry and stops failed while preserving durable evidence.
- PocketIC proves exact scheduling, upgrade reconstruction, finite capacity
  release, TTL-free retention, and truthful active/idle runtime status.
- Intent record and receipt schemas, public Candid, configuration, dependencies,
  and Cargo package versions are unchanged.

## Slice C Pool Evidence

- Released `v0.95.3` hard-cuts the `pool:maintenance` key and its permanent
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

## Slice C Placement Evidence

- Released `v0.95.4` adds core-runtime allocation 45 as a derived index
  containing only terminal placement operation identities. Lifecycle rebuilds
  it from canonical receipt-backed intents before scheduling begins.
- Empty indexes remain `unregistered + idle`. Producers and lifecycle recovery
  schedule immediate exact work; successful 32-row pages continue through new
  timer messages without scanning unrelated receipt consumers.
- Root transport/ops failure stops after one call and follows the frozen
  1/2/4/8/16/30-minute progression. Public root rejection and local/index
  contradictions stop failed with durable evidence retained.
- The progression permits at most 52 failed calls in its first 24 hours and 48
  per day at the cap. New terminal evidence advances the deadline; successful
  acknowledgement resets backoff.
- Unit coverage proves exact selection, rebuild, corruption rejection, removal,
  classification, and bounds. A maintained scaling PocketIC topology proves
  real root acknowledgement drains back to idle.

## Slice C Log Evidence

- Released `v0.95.5` hard-cuts the append-only allocations 31 and 32 and
  their full-history rewrite. One ordered stable map at allocation 35 now owns
  current runtime-log records and exact oldest-row removal.
- Append owns both entry-byte truncation and the count limit. At steady state
  it evicts at most one displaced row; lowering the compiled count below
  retained history clears non-authoritative logs in one bounded hard cut.
- Age retention schedules only `oldest.created_at + max_age_secs + 1s`, which
  preserves the existing strict cutoff. Each callback removes at most 256 due
  rows and continues through a separate timer message only while more are due.
- With the default `max_age_secs` unset, `log_retention:run` remains
  `unregistered + idle` with zero callbacks rather than waking 144 times/day.
  Local configuration or storage contradictions stop failed.
- Runtime-log Candid and configuration shapes are unchanged. The state
  manifest now declares one `runtime_log` v1 domain; retired append-only log
  history has no migration, alias, fallback, or parallel reader.

## Slice D Cycle Evidence

- Released `v0.95.6` replaces `cycles:tracking` with one independent
  `cycles:topup` owner. Disabled roles record their lifecycle observation and
  remain `unregistered + idle`; no history-only callback remains.
- Released `v0.95.7` limits the owner to configured nonroot roles,
  which await the existing parent-funding request. Root ICP conversion remains
  an explicit operator action through the maintained guarded endpoint and CLI.
  The automatic hub workflow, threshold field, resumable lookup, and operation
  identity are removed as a hard cut.
- Above threshold, the next check derives from headroom and the greater of
  twice observed burn or a threshold-per-day floor. The accepted safety window
  is one minute to one hour after reserving five minutes for funding and five
  minutes of margin; observations older than 12 hours use the floor.
- Work is due at or below threshold. A completed grant cannot be followed by
  another request before the configured parent cooldown or the one-minute
  observation floor. Transport/ops failure and in-progress conflict use the
  1/2/4/8/16/30-minute progression. A typed resource-exhaustion response gets
  one one-minute recheck between successes; repeated exhaustion,
  authorization, and configuration rejection stop instead of polling.
- The parent remains the single abuse-control authority for direct-child
  admission, kill switch, replay exclusion, cost guard, request clamping,
  cumulative child budget, cooldown, and available balance. The child timer
  cannot bypass those controls or trigger unbounded ICP conversion.
- Released `v0.95.9` preserves that fail-closed admission while reconciling the
  existing child owner when authoritative topology arrives after placement.
  No authorization class becomes retryable, and no polling, grace delay,
  duplicate timer, or extra history write is introduced. Cycle deadline
  multiplication and addition now return typed invariant failure on overflow.
  The initial topology recovery event is consumed once per runtime start so
  unrelated later topology changes cannot restart terminal capacity or policy
  rejection repeatedly.
- Released `v0.95.10` preserves insufficient parent capacity as the public typed
  `ResourceExhausted` cause and permits one one-minute recovery attempt between
  successful grants. The measured nested shard then succeeds after its parent
  refills in the same timer round. A second exhaustion stops failed, so no
  capacity-polling loop or weakened parent control is introduced.
- History is written at lifecycle and funding boundaries. Each observation
  purges at most 128 expired balance rows and 128 top-up-event rows under the
  unchanged seven-day window. Candid and stable shapes are unchanged; the
  automatic-only `min_hub_cycles_before_refill` config key is removed.

## Slice D Auth Evidence

- Released `v0.95.8` gives `auth_renewal:run` one deadline reconstructed from the
  current registry-bound chain-key batch and root-owned issuer renewal state.
  No template or disabled auth leaves it `unregistered + idle`.
- Prepared and signed batches continue immediately. A persisted `Signing`
  batch waits until its exact expiry after lifecycle restart because its
  external signing outcome is unknown; Canic does not issue a blind duplicate.
  Installing and retryable batches retain their exact durable retry deadline.
- With no in-flight batch, the next deadline is the earliest installed
  refresh or `next_attempt_after`. Missing state or a template, policy,
  registry epoch, registry hash, or installed-certificate mismatch is due now.
- Transport and ops failures use the 1/2/4/8/16/30-minute bounded progression.
  Remaining active-proof validity can only shorten the retry. Terminal public
  rejection, encoding/response contradiction, overflow, and due-without-
  progress stop failed and preserve the typed cause in recent diagnostics.
- Partial issuer installation is no longer reported as timer success. Exact
  successes remain committed; failed issuers remain on the same durable batch
  for bounded retry.
- The two-issuer PocketIC journey validates one signature, valid installed
  proofs, the exact root-owned refresh deadline, and idle reconciliation after
  both templates are disabled. No Candid, configuration, stable-record,
  dependency, or Cargo package version change is present.

## Finding Index

| Finding | Severity | State | Owner |
| --- | --- | --- | --- |
| `CANIC-095-TIMER-001` async interval overlap | P1 | fixed in released 0.95.1 | common timer workflow |
| `CANIC-095-TIMER-002` stale guarded slot/lost reschedule | P1 | fixed in released 0.95.1 | common timer workflow |
| `CANIC-095-TIMER-003` false live timer status | P2 | fixed in released 0.95.1 | common timer workflow/runtime projection |
| `CANIC-095-TIMER-004` unnecessary idle wakes | P2 | intent fixed in released 0.95.2; pool fixed in released 0.95.3; placement fixed in released 0.95.4; log fixed in released 0.95.5; cycle history fixed in released 0.95.6; root top-up fixed in released 0.95.7; auth fixed in released 0.95.8 | log, pool, intent, placement, auth, and cycle workflows |
| `CANIC-095-TIMER-005` unrelated full scans | P2 | intent fixed in released 0.95.2; placement fixed in released 0.95.4 | intent and placement ops/workflows |
| `CANIC-095-TIMER-006` competing mechanics/lifecycle paths | P2 | fixed in released 0.95.1 | timer workflow and lifecycle facade |
| `CANIC-095-TIMER-007` unreachable configured root self-refill | P1 | audit assumption corrected in released 0.95.7: automatic root refill was obsolete, so the flow and its config surface are hard-cut; manual ICP conversion remains | cycle/top-up and ICP-refill workflows |
| `CANIC-095-TIMER-008` log count authority contradicts disposition | P2 | fixed in released 0.95.5 | log storage/ops/workflow |
| `CANIC-095-TIMER-009` auth renewal polls and erases timer outcomes | P1 | fixed in released 0.95.8 | auth renewal ops/workflow and common timer workflow |
| `CANIC-095-TIMER-010` child funding can stop before parent topology admission | P1 | fixed in released 0.95.9 | topology and cycle workflows |
| `CANIC-095-TIMER-011` cycle deadlines saturate instead of failing closed | P2 | fixed in released 0.95.9 | cycle workflow |
| `CANIC-095-TIMER-012` obsolete role-attestation timer routing survives without an owner | P2 | fixed in released 0.95.9 | build, lifecycle, and runtime startup |
| `CANIC-095-TIMER-013` nested child capacity rejection can become terminal before its parent refills in the same timer round | P1 | fixed in released 0.95.10 | RPC typed-cause mapping and cycle workflow |

No other product finding is admitted to 0.95 without a design amendment and
reproducible timer-owner evidence.

## Frozen Implementation Order

1. Slice B: fixed identities, sequence/generation arbitration, automatic slot
   consumption, cancellation, after-completion recurrence, live status, one
   lifecycle facade, and public hard cuts.
2. Slice C: log age deadlines, pool events/retries, intent expiry index, and
   placement acknowledgement queue/index.
3. Slice D: independent cycle sample/nonroot-top-up owners, manual-only root
   ICP conversion, exact delegated-proof renewal deadline, comparative costs,
   and cumulative closeout.

Receipt replay/reclamation, Toko integration, dependency work, backup/restore,
and general cleanup remain out of scope.

## Next Action

0.95 is closed at immutable `v0.95.10`; its nested-funding and 24-hour idle
regressions remain in the maintained test driver. Continue through the bounded
0.96 receipt contract without reopening this line for unrelated cleanup.
