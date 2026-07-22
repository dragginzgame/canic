# Receipt-Backed Intent Adapter Handoff

## Purpose

This document is the downstream integration contract for Canic's
receipt-backed intent API. It is intentionally narrower than the 0.90 design:
it describes how one trusted domain adapter binds an external receipt to one
Canic reservation without adding a generic receipt or recovery framework.

Canic owns durable reservation state, exact identity-binding enforcement,
capacity accounting, and compare-and-set settlement. A downstream adapter
owns request and identity derivation, authorization, the external call,
receipt storage, receipt validation, and domain responses.

## Required Flow

For every first entrance or retry, the adapter must:

1. Authenticate and validate the complete domain request.
2. Derive one deterministic `OperationId`, `PayloadBinding`, resource key,
   quantity, and absolute replay deadline from that immutable authorization.
3. Call `ReceiptBackedIntentApi::begin_or_load` before the external effect.
4. Execute the effect only for `Created`.
5. Validate any returned or later observed domain receipt.
6. Construct `TerminalEvidence` only from that validated receipt.
7. Call `ReceiptBackedIntentApi::settle_if_pending` with the revision returned
   by `Created` or `ExistingPending`.

There is no `await` inside either Canic mutation. The adapter performs the
external call between begin and settlement.

The replay deadline is required and may not move forward on retry. It must not
exceed the currently verified authorization expiry. Canic accepts an absent
operation only while `now < replay_deadline_ns`, with at most 24 hours
remaining. Exact retained operations are loaded before that temporal decision,
so pending and terminal results remain observable at and after their deadline.

## Resource Namespace Ownership

Application-owned local and receipt-backed intent resource keys must not begin
with `canic:`. That prefix is reserved for Canic runtime authority, including
placement allocation, cost guards, and pool import. Consumer begin operations
reject reserved keys with `InvalidInput`; consumer load, settlement, commit,
and rollback also reject an existing Canic-owned record before mutation.

Use a domain-owned prefix such as `mint:`, `uploads:`, or `app:placement:`.
Ordinary words such as `placement:`, `cost:`, and `pool_import:` are not
reserved without the leading `canic:` namespace.

## Begin Decisions

| Canic result | Adapter decision |
| --- | --- |
| `Created` | Execute the external operation once with the bound ID. |
| `ExistingPending` | Perform one domain-owned, targeted recovery action. |
| `ExistingCommitted` | Return or reconstruct the stored success. |
| `ExistingRolledBack` | Return or reconstruct the stored no-effect result. |
| `BindingConflict` | Reject without making an external call. |
| `ReplayWindowClosed` | Reject the absent operation without making an external call. |
| `ReplayWindowTooLong` | Reject the adapter deadline without making an external call. |
| `CapacityExceeded` | Reject this new operation without changing state. |
| `StoreCapacityReached` | Reject this new operation without changing state. |

An external transport, decode, callback, or local-settlement error does not
prove that the external operation was not submitted. The reservation remains
pending until the adapter has validated applied or durable no-effect evidence.

## Evidence Boundary

Before constructing `TerminalEvidence`, the adapter must validate all
domain-specific facts, including:

- the actual source canister;
- the caller-scoped operation identity;
- the canonical payload binding;
- the receipt schema and terminal decision; and
- the effect reference or durable no-effect reason represented by the
  fingerprint.

Canic compares the source, decision, and fingerprint for exact terminal replay.
It does not decide whether a domain receipt is authentic or sufficient.
Contradictory terminal evidence is an error and must not be converted into a
different settlement attempt.

## Focused Conformance Fixture

`crates/canic-tests/tests/pic_receipt_backed_intent.rs` exercises the public
Canic facade through the sole `intent_authority` test canister. The fixture
exposes explicit begin and settlement decisions instead of collapsing them
into an optional record. Its focused PocketIC proof covers:

- creation and exact pending replay;
- changed payload, resource, or deadline rejection;
- shared-resource capacity rejection;
- not-found, stale-revision, and binding-conflict settlement;
- pending state across a same-Wasm upgrade;
- committed and rolled-back settlement;
- exact terminal replay;
- contradictory evidence preserving the first terminal state; and
- released capacity after durable rollback.

The seed-based fixture endpoints are test scaffolding, not a downstream API.
Downstream code should use the typed Canic facade operations directly and keep
its domain receipt types in its own repository.

## Canonical Placement Consumer

Canic's placement workflow is the first in-repository consumer. Scaling,
directory, and sharding all use one `PlacementAllocationWorkflow` that:

- derives a deterministic operation, payload binding, and capacity resource;
- reserves live capacity through `ReceiptBackedIntent` before root RPC;
- reuses that operation as the root provision replay identity;
- settles only after the owning placement registry records the child;
- rolls back only after a known provisional child is disposed; and
- acknowledges the committed root placement receipt after local settlement;
- carries the exact 32-byte operation ID through capability transport; and
- retains terminal intent evidence as a bounded durable acknowledgement queue
  until the response-idempotent root acknowledgement succeeds.

Committed root placement receipts survive request expiry until that
acknowledgement. The queue drain resumes after init and upgrade and deletes only
the exact terminal intent after receipt release; settled aggregate capacity is
unchanged. A directory claim with no known child is never released on age
alone. Recovery resumes only when the matching durable intent exists; untracked
uncertainty remains fail closed.

Placement resources use the exact internal shape
`canic:placement:<64 lowercase hex>`. The acknowledgement drain accepts only
that shape; a consumer resource with a similar ordinary prefix is never part
of the placement queue.

Other Canic systems may use this adapter when they allocate a child through
root and have one authoritative registry transition that can prove membership
or disposal. Systems that transfer value, upgrade or recycle existing
canisters, or already own a domain-specific replay/cost receipt do not fit this
contract and must retain their existing authority.

## Downstream Adoption

Toko is the first planned downstream consumer. Its developers can implement
the mint adapter after the Canic release is published. That implementation
must remain Toko-owned and add focused tests for caller-scoped receipts,
receipt validation, deterministic no-effect evidence, ambiguous-call recovery,
cancellation fencing, and retirement of any co-authoritative local settlement
row.

Canic publication does not certify those downstream properties. It provides
and tests the generic reservation and settlement contract on which they rely.
