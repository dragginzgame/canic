# Canic 0.102 Permanent Diagnostic Code Allocation Ledger

Date: 2026-08-13

## Status

This document freezes the allocation-ledger contract and records its B2
materialization. B2 changes no public wire behavior.

The maintainer approved dense rows `1..=991` for 960 exact handling contracts
and 31 safe public projections on 2026-08-16. The checked-in ledger is now
permanent numeric authority and contains 991 authoritative current rows. There
are no retired rows in the initial allocation. The approved rows cover all
3,929 producer-qualified observations exactly once.

## Maintained Owners

The implemented ledger lives at:

```text
crates/canic-host/diagnostics/allocations.toml
```

The language-neutral current-code projection lives at:

```text
crates/canic-host/diagnostics/current-codes.json
```

Both are host/repository assets. Neither path may be imported, generated into
or packaged with a canister crate. Representative release-Wasm inspection must
prove both complete assets absent.

`allocations.toml` is permanent identity authority. `current-codes.json` is a
deterministic current-only projection generated from the active host catalog
and checked against the ledger; it is not a second allocation authority.

## Required Allocation Row

Every ledger row contains at least:

| Field | Meaning |
| --- | --- |
| `code` | Permanent nonzero `u16` identity |
| `label` | Current or former symbolic identity |
| `status` | Exactly `current` or `retired` |
| `summary` | Current or last operator summary |
| `catalog_owner` | Active host owner for a current row; absent when retired |

Current rows additionally have one registered runtime constant, at least one
current producer, one active host catalog entry, one canonical semantic
condition and the stable handling key that justified allocation. Any number of
equivalent producers may reference the same current row. Retired rows have
none of those active owners.

## Retirement Transition

Retirement updates one existing row from `current` to `retired`. It may update
the summary to the last maintained wording, but it never changes the number or
former label and never deletes the row. The same coherent slice removes:

- every runtime producer;
- the registered runtime constant;
- the active rich catalog entry; and
- the current-code JSON row.

Host lookup may still render the retired identity from this ledger. That keeps
numeric logs, receipts and operator evidence diagnosable. It does not accept or
decode the removed pre-0.102 enum-plus-message Candid contract.

## CI Contract

CI must prove:

1. all current and retired numbers are globally unique and nonzero;
2. current ledger rows, registered constants, active catalog entries and
   current JSON rows are bijective;
3. every current row has at least one current producer, every maintained
   producer observation maps to exactly one current internal row or an explicit
   non-diagnostic disposition, and equivalent producers map many-to-one;
4. every retired row has no current producer, registered constant, active
   catalog entry or current JSON row;
5. allocation only appends the next never-used number after initial approval;
6. direct numeric producer construction outside the central declaration module
   and direct `Error` struct construction outside its one boundary module fail
   mechanically;
7. the ledger, generated JSON registry, producer-coverage frontier and
   producer-to-code mapping are absent from release Wasm; and
8. the initial allocation is not four digits, every singleton has a reviewed
   split rationale and no code exists solely because a producer is in another
   module, role or subsystem; and
9. every current code has exactly one canonical runtime declaration path and
   no producer-local alias or duplicate constant.

Protocol anti-resurrection tests for removed fields, variants and decoders are
forbidden. Allocation anti-reuse tests over this permanent ledger are required.
