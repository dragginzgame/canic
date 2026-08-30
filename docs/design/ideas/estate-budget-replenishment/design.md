# Idea: Replay-Safe Estate Budget Replenishment

Date: 2026-08-18
Roadmap reconciled: 2026-08-30

## Status

- Classification: deferred, unnumbered idea. It is not a scheduled release or
  implementation authority.
- Need: the 0.112 pre-funded estate model is sufficient for bounded
  qualification but a continuously growing Fleet can exhaust the exact root
  Cycles Ledger account.
- Sequence: review only after the pre-funded 0.112 journey is qualified and
  its real funding/cost evidence is accepted.
- Separation: this is distinct from 0.108 Coordinator-backed root operating
  funding and from every application/player economy.

## Decision Direction

A future optional protocol may replenish one exact registered root estate
Cycles Ledger account from a Fleet-authorized infrastructure source. It must:

1. derive the destination account from protected root/Fleet authority, never
   from an application payload;
2. reserve the exact per-root/Fleet budget before transfer;
3. persist intent before the external effect;
4. reconcile duplicate calls, uncertain transfer and acknowledgement loss
   through one exact operation identity and receipt;
5. expose pause, exhaustion, reserved, transferred and unresolved status;
6. keep Coordinator treasury, root operating balance, estate Ledger balance,
   retained-asset cycles and application/player balances separate; and
7. deny every player, game command or Galactic Credit path direct Canic
   treasury authority.

An application may calculate or recommend an infrastructure allocation, but a
separate operator/Fleet authority must admit and execute the deposit. The
application cannot choose the destination account or convert game balances
into infrastructure authority.

## Required Evidence Before Promotion

- completed 0.108 and 0.112 funding/accounting contracts;
- measured estate depletion and replenishment need from the pre-funded v1
  journey;
- exact source authority and destination account derivation;
- immutable per-root/Fleet limits and reserve policy;
- Cycles Ledger duplicate/fee/`TooOld`/uncertainty contract;
- response-loss, restart and acknowledgement-loss recovery;
- distinct operator, Coordinator, root and application threat model; and
- explicit maintainer-approved release position and batch plan.

Until promotion, operators pre-fund the immutable maximum exposure, pause
creation on insufficient estate balance and make an explicit deposit before
resuming.
