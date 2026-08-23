# Idea: Operator Funding Conversion Authority

Date: 2026-08-23

## Status

- Classification: deferred, unnumbered idea. It is not a scheduled release or
  implementation authority.
- Need: Fleet creation profiles are cycle-denominated, while an operator may
  eventually want Canic to fund creation from ICP without manually converting
  the required amount first.
- Current boundary: Fleet creation funding is cycle-only. `canic scaffold
  fleet-input` reports exact cycle amounts and Cycles Ledger creation fees;
  `canic deploy plan` admits only a live cycles balance that covers that exact
  fee-complete maximum debit.
- Separation: Root-owned protected ICP refill remains a distinct installed-
  Fleet recovery protocol. It does not authorize host-side Fleet creation or
  convert one funding domain into another during planning.

## Problem

An ICP-denominated creation amount cannot satisfy a cycle-denominated profile
floor without a bound conversion authority. A planner would otherwise have to
choose an exchange rate, freshness window, fee estimate and safety margin on
the operator's behalf. Mixing ICP and cycles would additionally require two
independent balances and debit totals; adding the raw amounts is invalid.

The current hard cut therefore rejects every `kind = "icp"` infrastructure
creation amount before topology-profile admission. Operators explicitly
convert ICP to cycles first and submit cycle-funded creation values.

## Decision Direction

A future opt-in automation may quote and execute operator funding conversion
only if it owns all of the following authority:

1. one exact operator Principal and its separately derived ICP Ledger and
   Cycles Ledger accounts;
2. one exact cycle-denominated Fleet-input requirement, including every
   per-creation Cycles Ledger fee;
3. a live CMC conversion-rate observation with source, certification status,
   timestamp, bounded expiry and an operator-selected minimum-rate guard;
4. separate ICP and cycles balance observations and separate maximum debit
   totals, including the ICP Ledger transfer fee and any conversion fee;
5. an explicit finite slippage or overfunding margin that cannot be silently
   increased by a profile or later release;
6. one preview receipt binding the input digest, rate evidence, fees, margin,
   destination account and maximum ICP debit;
7. a final pre-effect identity, balance, fee and rate recheck;
8. one durable conversion operation identity with duplicate, response-loss,
   restart and exact-retry reconciliation; and
9. a terminal proof that the resulting cycles balance covers the unchanged
   fee-complete Fleet creation debit before installation begins.

The offline scaffold may eventually explain a quote, but it must remain
clearly non-admissible without live rate and balance evidence. `canic deploy
plan` remains the install-admission boundary and must never infer permission to
convert from the presence of an ICP balance.

## Required Evidence Before Promotion

- exact ICP CLI, ICP Ledger, Cycles Ledger and CMC protocol/version baseline;
- certified or otherwise explicitly classified conversion-rate authority;
- equality, one-unit-short, fee-change, rate-expiry and slippage-bound tests;
- mixed-account and wrong-identity denial before any transfer;
- response-loss and duplicate-notification replay without duplicate debit;
- restart between ICP transfer, CMC notification and cycles observation;
- exact refund/error classification with no optimistic unknown-effect retry;
- clean separation from Root runtime refill budgets and journals; and
- an explicit maintainer-approved release position and implementation batch.

Until promotion, Canic accepts only cycle-funded fresh-Fleet infrastructure
creation and never converts or values ICP during Fleet-input admission.
