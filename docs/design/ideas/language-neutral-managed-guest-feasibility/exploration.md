# Exploration: Managed Guests And A Rust Database Service

Reviewed: 2026-09-06

## Status

Exploration only. The [managed-guest feasibility idea](design.md) owns the
bounded next question. This note preserves a possible application use case;
it approves no ABI, Mops package, database service or external repository work.

## Two Independent Needs

A Motoko application might need:

- ordinary managed Component lifecycle under Canic; and
- access to a separately deployed Rust database service.

The first requires managed-guest conformance and qualified build evidence.
The second requires an application-owned service contract. Neither implies a
rewrite of Canic, IcyDB or an existing Rust product in Motoko.

Wasm installation alone supplies neither contract.

## Possible Database Boundary

If a concrete consumer needs an IcyDB-backed service, keep the database runtime
in Rust. Its owning application or external repository would define typed,
bounded Candid operations and an optional Motoko client.

The service owns schema, query limits, authorization, consistency, storage,
indexes and database recovery. Canic owns only the managed infrastructure
contract: exact artifacts, placement, lifecycle and protected bindings.

Root owns local Component lifecycle and Component Registry state. Coordinator
owns the Fleet Registry. A Motoko caller may request only the child operations
permitted by its exact registered binding; application parenthood does not
make it a controller or a Fleet authority.

## Questions A Real Consumer Must Resolve

- Is the database a top-level Component or an admitted child, and who owns its
  application authorization and quotas?
- Which concrete reads and writes cross the Candid boundary? Do not expose
  unrestricted controller SQL as an application convenience.
- What are the operation identity, expected revision and durable result for
  each mutation?
- How does an interrupted caller reconcile a write that may already have
  committed?
- What are response/page limits, backpressure, latency and cycle budgets?
- Which same-release backup/restore ordering and data-consistency guarantees
  does the application need?

An inter-canister call is not a cross-canister transaction. Database and
application operation receipts must describe actual commit authority; a Canic
lifecycle receipt cannot stand in for a database result.

## Disposition

Keep this use case separate from the feasibility proof. The former complete
SDK, configuration and database topology proposal is retired. A successful
guest proof does not establish an IcyDB service or production adoption.

No sibling repository mutation, cross-release state preservation or product
rewrite follows from this note. If no consumer can specify a bounded service
API and its operating cost, do not build a generic database service.
