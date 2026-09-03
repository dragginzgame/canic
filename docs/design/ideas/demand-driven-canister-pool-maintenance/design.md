# Idea: Demand-Driven Canister-Pool Maintenance

Date: 2026-09-03

## Status

- Classification: deferred, unnumbered idea. It is not a scheduled release or
  implementation authority.
- Need: an Active Fleet Subnet Root currently polls Canister-pool maintenance
  every 30 seconds even when no work can be performed. One local observation
  recorded approximately 959 million instructions across 312 no-work checks.
- Priority: non-blocking efficiency and observability cleanup. Current pool
  correctness, recovery and mainnet refill behavior take precedence.
- Ownership: the Root Canister-pool workflow owns proactive maintenance; the
  shared async-job recovery watchdog remains a separate reliability owner.

## Current Boundary

Every Active Root schedules a retained after-completion pool-maintenance timer.
Each pass may reconcile a pending creation, reset one pending asset, stop for
draining, report a healthy configured minimum, or start a refill when Ready
capacity is below that minimum.

Automatic creation is intentionally available only for the `ic` build network
because it uses the IC-mainnet Cycles Ledger. On `local`, a capacity deficit
therefore produces a maintenance-paused result, but the timer currently recurs
after completion and repeats the same check 30 seconds later.

Local maintenance is not wholly redundant. Initial imported assets may still
need reset and inspection, explicit reset retry can introduce new work, and
recycled assets retain same-operation recovery requirements. A future change
must not disable the complete workflow merely because automatic creation is
unavailable.

## Decision Direction

### Local Networks

Make proactive pool maintenance quiesce after all actionable local work has
settled. The maintained direction is:

1. retain the initial activation pass so imported `PendingReset` assets can be
   inspected and prepared;
2. continue bounded passes while a pending reset, recyclable asset transition
   or other locally executable pool operation exists;
3. stop recurrence when the pool is healthy or its only unmet demand would
   require unavailable automatic creation;
4. re-arm maintenance when an explicit command or durable state transition
   introduces actionable local work; and
5. reconstruct that exact demand after lifecycle or authority-snapshot restore
   without reinstating permanent idle polling.

The local terminal state should be observable as intentional quiescence, not a
failed or silently missing timer.

### IC Mainnet

Do not disable autonomous maintenance on `ic`. A mainnet Root must still notice
that Ready capacity fell below its configured minimum and reconcile the exact
Cycles Ledger creation journal through success, retry, uncertain response or
terminal blockage.

A later implementation may also make healthy mainnet maintenance
demand-driven. Pool allocation, reset, recycle, activation and restore paths
already expose candidate events from which to arm bounded work. Promotion must
first prove that every capacity-reducing transition is covered and that lost
responses, traps and restarts cannot leave refill demand dormant. A slower
periodic safety reconciliation may remain justified if event-complete arming
cannot be proved.

### Recovery Watchdog

The 30-second async-job recovery watchdog is not the proactive pool timer. It
also protects Root issuer renewal, automatic cycle top-up, placement-receipt
acknowledgement and expired Canister-pool maintenance attempts. Network-aware
pool quiescence must not disable, merge with or weaken that watchdog. Any later
watchdog optimization requires its own reliability design and evidence.

## Non-Goals

This idea does not:

- enable automatic Cycles Ledger Canister creation on local networks;
- change the configured pool minimum, maximum or per-asset cycle amount;
- remove explicit local import, reset, retry or recycle behavior;
- weaken intent-before-effect journalling, lost-response reconciliation,
  bounded paid effects or immediate replay safety;
- change Fleet activation, draining or authority-snapshot fences; or
- claim that an idle mainnet timer is free merely because its refill capability
  is valid.

## Evidence Required Before Promotion

- a local activation journey with multiple imported assets that reaches Ready
  capacity before the maintenance timer becomes quiescent;
- an underfilled local pool that stops repeating the unavailable automatic-
  creation check;
- explicit local reset retry and recycle journeys that re-arm only the required
  bounded work and quiesce again;
- lifecycle and authority-snapshot restoration of both quiescent and active
  maintenance demand;
- unchanged periodic recovery-watchdog ownership and expired-attempt takeover;
- mainnet automatic refill from a healthy pool through allocation-created
  demand, including uncertain response and exact replay;
- a mainnet no-work measurement that informs whether event-driven arming or a
  slower safety cadence is warranted; and
- timer-inventory and instruction evidence showing that local idle pool checks
  no longer accumulate while required recovery remains live.

Promotion requires a concrete owner, accepted release position, complete batch
plan and explicit maintainer approval. Until then, local Roots retain the
existing polling behavior and mainnet Roots retain autonomous refill.
