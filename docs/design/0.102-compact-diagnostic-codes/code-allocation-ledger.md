# Canic 0.102 Permanent Diagnostic Code Allocation Ledger

Date: 2026-08-13

## Status

This B1 document freezes the allocation-ledger contract before any number is
approved. It allocates no code and changes no runtime or wire behavior.

The initial numeric allocation is still incomplete. Consequently there are no
authoritative current rows yet, and no numeric code has ever shipped under the
0.102 protocol to retire. Empty current and retired sets are the only truthful
ledger state until the complete B1 producer inventory receives maintainer
approval.

## Maintained Owners

The implemented ledger will live at:

```text
crates/canic-host/diagnostics/allocations.toml
```

The language-neutral current-code projection will live at:

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
current producer and one active host catalog entry. Retired rows have none of
those active owners.

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
3. every current row has a current producer;
4. every retired row has no current producer, registered constant, active
   catalog entry or current JSON row;
5. allocation only appends the next never-used number after initial approval;
6. direct numeric producer construction outside the central declaration module
   and direct `Error` struct construction outside its one boundary module fail
   mechanically; and
7. the ledger and generated JSON registry are absent from release Wasm.

Protocol anti-resurrection tests for removed fields, variants and decoders are
forbidden. Allocation anti-reuse tests over this permanent ledger are required.
