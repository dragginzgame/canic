# Idea: Replay-Safe Estate Budget Replenishment

Date: 2026-08-18
Reviewed: 2026-09-06

## Status

- Classification: deferred, unnumbered idea. It is not a scheduled release or
  implementation authority.
- Need: reviewed deployment funding is bounded, while continued growth may
  eventually exhaust the exact Root estate Cycles Ledger account.
- Sequence: reconsider after accepted bounded-estate qualification and real
  depletion/cost evidence. Repeated reviewed plans remain the baseline; this
  idea does not schedule autonomous funding or amend the estate roadmap.
- Separation: this is distinct from existing Coordinator-backed Root operating
  funding and from every application/player economy.

## Decision Direction

Current Fleet Ensure can fund a bounded deployment forecast through one
reviewed `FundEstate` action. This deferred idea is narrower: a future optional
protocol may replenish one exact registered root estate Cycles Ledger account
for demand that arises after the terminal reviewed deployment plan. It must:

1. derive the destination account from protected root/Fleet authority, never
   from an application payload;
2. reserve the exact per-root/Fleet budget before transfer;
3. persist intent before the external effect;
4. reconcile duplicate calls, uncertain transfer and acknowledgement loss
   through one exact operation identity and receipt;
5. expose pause, exhaustion, reserved, transferred and unresolved status;
6. keep Coordinator treasury, root operating balance, estate Ledger balance,
   retained-asset cycles and application/player balances separate; and
7. deny application commands and application balances direct Canic treasury
   authority.

An application may calculate or recommend an infrastructure allocation, but a
separate operator/Fleet authority must admit and execute the deposit. The
application cannot choose the destination account or convert game balances
into infrastructure authority.

## Required Evidence Before Promotion

- current Coordinator/Root funding and estate-accounting contracts;
- measured estate depletion and an operational need beyond repeated reviewed
  Fleet Ensure plans;
- exact source authority and destination account derivation;
- immutable per-root/Fleet limits and reserve policy;
- Cycles Ledger duplicate/fee/`TooOld`/uncertainty contract;
- response-loss, restart and acknowledgement-loss recovery;
- distinct operator, Coordinator, root and application threat model; and
- explicit maintainer-approved release position and batch plan.

Until promotion, unplanned post-deployment growth pauses and requires a new
reviewed Fleet Ensure plan. Operators must not bypass its durable funding intent
with an out-of-band deposit. Continuous autonomous replenishment remains
deferred.
