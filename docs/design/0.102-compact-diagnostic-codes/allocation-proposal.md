# Canic 0.102 Diagnostic Allocation Proposal

Date: 2026-08-13

## Status

This document proposes the allocation policy for maintainer approval. It does
not allocate a code and must not be imported by runtime code.
[ledger-reconciliation.md](ledger-reconciliation.md) records the initial
collision-free 685-identity family subset and the first seventy-nine constructor/
semantic-call passes, bringing current qualified coverage to 2,467 identities, while
[projection-ledger.md](projection-ledger.md) aggregates every masked public
mapping in that subset and its proposed numeric observation.
[direct-constructor-frontier.md](direct-constructor-frontier.md) records the
remaining site-level coverage gate. The reviewed complete allocation table will
replace this proposal as the B1 authority.
[code-allocation-ledger.md](code-allocation-ledger.md) freezes the permanent
current/retired ledger contract; its current and retired sets remain empty
until that complete allocation is approved.

## Recommendation

Use one dense, monotonic numeric space:

- `0` is invalid and never allocated;
- the initial approved leaves receive `1..=N` with no gaps;
- a later current producer receives the next never-used number;
- removing the final producer changes its permanent ledger row to `retired`;
- numbers do not encode class, origin, severity, retry policy or protocol
  generation.

This is preferable to subsystem bands. The host catalogue already owns exact
origin and meaning, while bands would either reserve unused identities or force
future subsystem growth into misleading ranges. Dense assignment also makes it
obvious that every number corresponds to a real current leaf at the initial
cut.

The compact runtime format should be the ASCII letter `E` followed by the raw
unsigned decimal value with no zero padding, for example `E1` or `E1203`.
Parsing accepts only the raw decimal form or that exact uppercase prefix. It
does not accept labels, lowercase aliases, signs, whitespace or historical
message text.

## Initial Ordering

The complete initial ledger should be sorted by:

1. `DiagnosticOrigin` catalogue order;
2. symbolic label; and
3. producer owner when two candidate rows still collide.

Numbers are then assigned once in that order. The ordering is an initial review
tool, not a promise that later labels remain alphabetically adjacent: new codes
append, retired rows remain present and retired numbers never move.

## Leaf Boundary

Two producers share a leaf only when all of these are identical:

- narrow owning subsystem;
- failure meaning;
- safe public exposure;
- caller/operator action;
- retry behavior; and
- machine decision.

Different interpolated values do not create leaves when the action is the same.
For example, a mismatched expected and observed Registry revision remains one
revision-mismatch diagnostic; the typed expected/observed values belong in the
status or receipt that owns them. Conversely, capacity exhaustion and malformed
input never share a leaf merely because both currently use one convenience
constructor.

The 1,150 constructor sites in the two largest Component Registry modules must
therefore be grouped by actionable invariant, not allocated one number per
line. Each group still lists every producer function so coverage is exhaustive.

## Host Classification

The proposed host-only broad classes are:

- `InvalidInput`;
- `Unauthorized`;
- `Forbidden`;
- `NotFound`;
- `Conflict`;
- `ResourceExhausted`;
- `Unavailable`;
- `Invariant`; and
- `Internal`.

They are catalogue metadata, not wire fields or canister lookup tables. A
class does not replace the leaf identity and cannot be used to guess an unknown
code.

Every current catalog row also has one typed host disposition:
`DoNotRetry`, `ExactRetry`, `RetryAfterStateChange`, `BoundedRetry` or
`Reconcile`. Labels are presentation-only. Host automation may consume the
typed disposition; runtime policy continues to use its owning typed state and
exact numeric constant rather than a catalog lookup.

The initial narrow origin families are:

- canister input and endpoint dispatch;
- delegated authentication and role attestation;
- RPC capability and replay;
- App configuration and compiled topology;
- runtime bootstrap, activation and restore fencing;
- Component provisioning, Registry and Directory;
- Fleet Registry, Coordinator and Fleet-service publication;
- Wasm Store publication and garbage collection;
- prepaid Canister inventory and lifecycle effects;
- cycles funding, cost guard and ICP refill;
- stable storage and canonical decoding; and
- current blob-storage billing/lifecycle producers pending the independent
  0.108 extraction.

The final `DiagnosticOrigin` variants should be as narrow as operational action
requires. These families are inventory headings, not reserved numeric bands.

## Required Row Shape

Every approved leaf row contains:

| Field | Meaning |
| --- | --- |
| code | Nonzero permanent `u16` identity |
| label | Stable host-only symbolic label |
| status | `current` in an approved producer row; retirement preserves the row as `retired` |
| class | One host-only broad class |
| origin | Narrow host-only subsystem owner |
| disposition | One typed host retry/reconcile/permanent behavior |
| summary | Concise host/operator meaning; retained as the last summary after retirement |
| producer owners | Every current function or exhaustive typed conversion producing the leaf |
| public projection | Same code or one explicit safe public code |
| observability owner | Structured numeric log, receipt/status field or `public` when no masking occurs |
| action | One concise operator/caller action |
| exposure note | Why the leaf is safe or why it is masked |

The complete row set must be mechanically checked against the native-only
runtime inventory before B2. A path or line number alone is not identity;
producer function and typed variant names remain the review anchors when lines
move.

## Public Projection Rules

- Input, authorization, admission, capacity and retry-state leaves are public
  when their exact identity reveals no private topology or stable contents.
- Corrupt stable state, cryptographic internals, private topology mismatches,
  raw platform rejects and build/file paths receive an internal code plus one
  explicit safe public code.
- A masked internal code is not admitted until its numeric observability owner
  is named, already exists in the same coherent slice and is attached to the
  same retrievable operation/status record or correlated through an existing
  retrievable operation ID.
- If there is no approved internal observability owner, allocate a safe but
  sufficiently specific public leaf instead of making the failure opaque.

## Decisions Requested

B1 needs maintainer approval of three choices before the complete numbers are
frozen:

1. dense monotonic codes rather than semantic numeric bands;
2. compact runtime rendering as unpadded uppercase `E<decimal>`; and
3. the nine host-only broad classes above.

Approval of this policy does not approve an incomplete leaf list. The complete
producer-to-leaf table remains a separate B1 review gate before B2 changes
runtime code.
