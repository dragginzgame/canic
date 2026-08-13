# Canic 0.103 Implementation Status

Status: ready for peer review; B1 evidence work approved; B2 implementation held

Design:
[0.103 framework-neutral local application authorization](0.103-design.md)

Status cut: 2026-08-13

## Current Boundary

0.103 is reserved for one framework-neutral bridge from verified Canic token
authority to a bounded synchronous local application decision. Evidence-only
B1 inventory and measurement may begin. It introduces no IcyDB dependency and
authorizes no runtime, public API, Candid, persisted state, package version or
release change.

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

## Approved Direction

Hard-cut the subject-only delegated session into one scoped local application
session. Establish it once from a fully verified token, persist only bounded
verified local authority, and expose one synchronous read-only
`caller + scope -> Allow(subject context) | Deny(reason)` facade.

The consuming application owns the adapter from that Canic decision to any
framework decision. Canic owns no database policy; the framework owns no Canic
token or grant state.

B1 must complete five implementation gates before B2:

1. propagate the selected hard cut that adds a required signed presenter to
   token claims, binds it into the canonical claims hash and proof, requires it
   to equal the current caller in both authorization lanes and removes the old
   presenter-less and subject-equals-caller meanings;
2. inventory and approve the canonical scope hard cut and application-scope
   issuance path;
3. prove the 60-second establishment-proof limit and admitted issuance burst
   cannot exhaust live replay tombstones;
4. freeze one model owner for scope, verified authority, session and replay
   values; and
5. confine caller/time/protected-state acquisition to access/ops while policy
   remains a pure value-to-value decision.

## Release-Batch Tracker

| Batch | Outcome | Owner | Included evidence | Validation | State |
| --- | --- | --- | --- | --- | --- |
| B1 | Current auth/session inventory and frozen generic contract | auth model/ops/access and host inventory | five mandatory decisions, exact predecessor inventory and duplicate-flow report | reproducible `E/A/D/B/H/R/C/M` baseline and explicit decision record | Approved to begin; B2 blocked |
| B2 | Canonical scope, verified-authority projection and pure denial policy | model and policy | scope hard cut, narrowing, binding, replay and replacement policy | focused pure-policy and canonical-scope tests | Blocked on B1 |
| B3 | Hard-cut scoped session state, index, bounds and lifecycle | model, ops and workflow | atomic state, tombstones, capacity, cleanup and synchronous restore | focused state, corruption, retry and reconstruction tests | Blocked on B2 |
| B4 | Standard establish, clear and self-status operations | DTO, endpoints/macros and host configuration | enablement, TTL, current Candid and compact diagnostics | exact surface checks and focused PocketIC operation tests | Blocked on B3 |
| B5 | Public synchronous facade, generic consumer and native guard convergence | access and facade | unchanged-ABI fixture, pure local decision and duplicate removal | focused facade and generic PocketIC consumer tests | Blocked on B4 |
| B6 | Operator inspection, security and performance gates | host/CLI, metrics and audit workflows | protected audit, generation invalidation, bounds and no-secret evidence | inspection, memory, instruction, restore and Wasm measurements | Blocked on B5 |
| B7 | Canic-only qualification, cleanup and closeout | cross-layer release batch | controller separation, clear/expiry, same-release recovery and residue cleanup | focused PocketIC security/recovery matrix and maintained-doc review | Blocked on B6 |

## Promotion Questions

B1 must answer:

1. exact current token/session Candid and source consumers;
2. complete producer/consumer propagation of the required signed presenter,
   including preparation-caller derivation, canonical encoding, signing,
   retrieval, verification, caching, direct authorization and session commit,
   with no presenter-less decoder or first-use bearer fallback;
3. predecessor scope grammar, the proposed hard cut and application-scope
   issuance ownership;
4. accepted active-session, per-subject and replay-binding capacities,
   including the rolling 60-second issuance burst;
5. default and maximum tokenless-session TTL and the maximum complete proof
   lifetime accepted for establishment;
6. exact local authority-generation owner and activation rule;
7. canonical stable snapshot versus derived heap-index representation;
8. one declaration owner for scopes, verified authority, sessions and replay;
9. public method/type names, inactive-status variants and closed denial
   precedence;
10. native proof-bearing/session-bearing policy convergence boundary;
11. current and proposed instruction, stable-byte, heap-byte, restore and
    raw-Wasm costs; and
12. the precise 0.106 authentication-profile reconciliation.

## Next Authorized Action

B1 may inspect and measure the current Canic surface, resolve the five mandatory
decisions, freeze the exact generic contract and return the evidence for review.
B2-B7 remain blocked until that review explicitly authorizes mutation. No
runtime, Candid, stable-state, package-version, changelog-version or downstream-
repository mutation is authorized.
