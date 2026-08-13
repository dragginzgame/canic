# Canic 0.103 Implementation Status

Status: planning only; no implementation authorized or started

Design:
[0.103 framework-neutral local application authorization](0.103-design.md)

Status cut: 2026-08-13

## Current Boundary

0.103 is reserved for one framework-neutral bridge from verified Canic token
authority to a bounded synchronous local application decision. It introduces
no IcyDB dependency and authorizes no runtime, public API, Candid, persisted
state, package version or release change.

The current 0.101.53 implementation remains unchanged:

- delegated tokens are verified locally against Fleet, role, subject, scope
  and time;
- `auth::authenticated(scope)` obtains the token from Candid argument zero;
- delegated sessions retain subject, issue/expiry and bootstrap fingerprint,
  but not verified local scopes or issuer context; and
- session lookup is bounded but is not yet the proposed read-only indexed
  foreign-guard contract.

The read-only IcyDB 0.226 design is requirements evidence only. Canic will not
modify IcyDB, import it or name its SQL/schema surfaces in Canic contracts.

## Numbering Reservation

The previous provisional Canic 0.103-0.111 designs have been renumbered
0.104-0.112 without changing their intended order or implementation status:

| Current line | Design |
| --- | --- |
| 0.104 | Cross-Subnet data transport groundwork |
| 0.105 | Coordinator Workers |
| 0.106 | Declarative authentication profiles |
| 0.107 | Standalone blob-service extraction |
| 0.108 | Coordinator-backed root funding |
| 0.109 | Optional encrypted Canister snapshot archives |
| 0.110 | Language-neutral managed-guest feasibility |
| 0.111 | Skynet Fleet observatory |
| 0.112 | Fleet Subnet Canister estates |

Published package versions, historical changelogs, audit evidence and archived
handoffs are not design reservations and were not renumbered.

## Key Decision Proposed

Hard-cut the subject-only delegated session into one scoped local application
session. Establish it once from a fully verified token, persist only bounded
verified local authority, and expose one synchronous read-only
`caller + scope -> Allow(subject context) | Deny(reason)` facade.

The consuming application owns the adapter from that Canic decision to any
framework decision. Canic owns no database policy; the framework owns no Canic
token or grant state.

## Release-Batch Tracker

| Batch | Outcome | State |
| --- | --- | --- |
| B1 | Current auth/session inventory, generic integration contract and `E/A/D/B/R/C/M` baseline | Pending maintainer approval |
| B2 | Canonical scope, verified-authority projection and pure denial policy | Blocked on B1 |
| B3 | Hard-cut scoped session state, index, bounds and lifecycle | Blocked on B2 |
| B4 | Standard establish, clear and self-status operations | Blocked on B3 |
| B5 | Public synchronous facade, generic consumer and native guard convergence | Blocked on B4 |
| B6 | Operator inspection, security and performance gates | Blocked on B5 |
| B7 | PocketIC qualification, cleanup and closeout | Blocked on B6 |

## Promotion Questions

B1 must answer:

1. exact current token/session Candid and source consumers;
2. exact scope grammar and byte/count limits;
3. accepted active-session, per-subject and replay-binding capacities;
4. default and maximum tokenless-session TTL;
5. exact local authority-generation owner and activation rule;
6. snapshot versus derived-index representation;
7. public method/type names and closed denial reasons;
8. native proof-bearing/session-bearing policy convergence boundary;
9. current and proposed instruction, stable-byte, restore and raw-Wasm costs;
   and
10. the precise 0.106 authentication-profile reconciliation.

## Next Authorized Action

No implementation is authorized. After maintainer approval, B1 may inspect and
measure the current Canic surface, freeze the exact generic contract and return
the evidence for review. B2-B7 remain blocked until that review explicitly
authorizes mutation.
