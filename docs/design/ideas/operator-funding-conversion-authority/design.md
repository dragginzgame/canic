# Idea: Operator Funding Conversion Authority

Date: 2026-08-23
Reviewed: 2026-09-06

## Status

- Classification: deferred, unnumbered idea. It is not a scheduled release or
  implementation authority.
- Need: Fleet creation budgets are cycle-denominated, while an operator may
  eventually want Canic to fund creation from ICP without manually converting
  the required amount first.
- Current boundary: `canic fleet generate` derives desired state from current
  policy, release and estate authority. `canic fleet ensure` plans bounded
  cycle-funded effects and applies only the exact reviewed plan digest.
  Funding admission uses observed balances, fees and bounded execution costs;
  neither command implicitly converts ICP.
- Separation: Root-owned protected ICP refill remains a distinct installed-
  Fleet recovery protocol. It does not authorize host-side Fleet creation or
  convert one funding domain into another during planning.

## Problem

An ICP-denominated creation amount cannot satisfy a cycle-denominated budget
without a bound conversion authority. A planner would otherwise have to
choose an exchange rate, freshness window, fee estimate and safety margin on
the operator's behalf. Mixing ICP and cycles would additionally require two
independent balances and debit totals; adding the raw amounts is invalid.

Operators currently arrange conversion separately and provide cycle-funded
creation budgets. The maintained desired-state and funding authorities live
in [Fleet Ensure](../../../../crates/canic-host/src/fleet_ensure/model/mod.rs).

## Decision Direction

A future opt-in automation may quote and execute operator funding conversion
only if it owns all of the following authority:

1. one exact operator Principal and its separately derived ICP Ledger and
   Cycles Ledger accounts;
2. one exact cycle-denominated desired-Fleet requirement, including every
   per-creation Cycles Ledger fee;
3. a live CMC conversion-rate observation with source, certification status,
   timestamp, bounded expiry and an operator-selected minimum-rate guard;
4. separate ICP and cycles balance observations and separate maximum debit
   totals, including the ICP Ledger transfer fee and any conversion fee;
5. an explicit finite slippage or overfunding margin that cannot be silently
   increased by a policy or later release;
6. one preview receipt binding the input digest, rate evidence, fees, margin,
   destination account and maximum ICP debit;
7. a final pre-effect identity, balance, fee and rate recheck;
8. one durable conversion operation identity with duplicate, response-loss,
   restart and exact-retry reconciliation; and
9. a terminal proof that the resulting cycles balance covers the unchanged
   fee-complete Fleet creation debit before installation begins.

A future quote remains non-admissible without live rate and balance evidence.
Fleet Ensure remains the deployment reconciliation owner. A conversion receipt
must be observed before admitting its resulting cycle budget; the presence of
an ICP balance never grants conversion authority. Exact CLI placement remains
a promotion decision.

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
creation and never converts or values ICP during desired-Fleet admission.
