# Canic 0.103 Implementation Status

Status: peer-review lifetime, client-access, predecessor, lifecycle-composition
and revocation feedback incorporated; B1 evidence work approved; B2
implementation held

Design:
[0.103 framework-neutral local application authorization](0.103-design.md)

Status cut: 2026-08-14

## Current Boundary

0.103 is reserved for one framework-neutral bridge from verified Canic token
authority to a bounded synchronous local application decision. Evidence-only
B1 inventory and measurement may begin. It introduces no IcyDB dependency and
authorizes no runtime, public API, Candid, persisted state, package version or
release change.

B1 evidence may run alongside 0.102 evidence. No mutating 0.103 batch may begin
until 0.102 is accepted and complete. B4 must consume the exact released 0.102
diagnostic representation, not its current proposal.

The current 0.101.53 implementation remains unchanged:

- delegated tokens are verified locally against Fleet, role, subject, scope
  and time;
- `auth::authenticated(scope)` obtains the token from Candid argument zero;
- delegated sessions retain subject, issue/expiry and bootstrap fingerprint,
  but not verified local scopes or issuer context; and
- session lookup is bounded but is not yet the proposed read-only indexed
  foreign-guard contract.

The read-only IcyDB 0.226 design and Explorer feedback are requirements evidence
only. Canic will not modify IcyDB, import it or name its SQL/schema surfaces in
Canic contracts.

The authorization contracts compose at the function level. Combined runtime
qualification with IcyDB or another lifecycle-owning framework additionally
requires the separately qualified
[0.104 synchronous lifecycle-composition seam](../0.104-framework-neutral-synchronous-lifecycle-composition/0.104-design.md).
The deferred optional block in `canic::start!` is not that seam, and 0.103 does
not implement, depend on or claim it.

## Numbering Reservation

Inserting 0.103 and the independent 0.104 lifecycle-composition line originally
moved the former provisional 0.103-0.111 designs to 0.105-0.113. The later
evidence-only 0.105 Fleet-estate platform qualification preserves the
peer-reviewed 0.103/0.104 identities and moves only those provisional lines to
0.106-0.114:

| Current line | Design |
| --- | --- |
| 0.105 | Fleet-estate platform qualification |
| 0.106 | Cross-Subnet data transport groundwork |
| 0.107 | Coordinator Workers |
| 0.108 | Declarative authentication profiles |
| 0.109 | Standalone blob-service extraction |
| 0.110 | Coordinator-backed root funding |
| 0.111 | Optional encrypted Canister snapshot archives |
| 0.112 | Language-neutral managed-guest feasibility |
| 0.113 | Skynet Fleet observatory |
| 0.114 | Fleet Subnet Canister estates |

Published package versions, historical changelogs, audit evidence and archived
handoffs are not design reservations and were not renumbered.

## Approved Direction

Hard-cut the subject-only delegated session into one scoped local application
session. Establish it once from a fully verified token, persist only bounded
verified local authority, and expose one synchronous read-only
`caller + scope -> Allow(subject context) | Deny(reason)` facade.

The establishment proof and retained session have separate clocks. A proof's
complete verified lifetime is at most 60 seconds. Protected role configuration
selects a positive default and maximum local-session TTL, both no greater than
1,800 seconds. The request may narrow that local maximum. A proof must be live
at first commit, but its expiry does not end or extend the committed session.
Replay tombstones remain tied to proof expiry; active-session capacity is
qualified separately for the longer occupancy window.

Caller clear rejects anonymous and removes that caller's retained session
record whether active, expired or stale. It remains idempotent when no record
exists and never removes a live replay tombstone. B1 must freeze the normative
generation-transition table before mutation: Fleet, role, issuer,
granted-scope and reduced-maximum-TTL changes advance generation; verifier
disablement and subject inadmissibility deny immediately; default-TTL changes
affect future sessions only.

The consuming application owns the adapter from that Canic decision to any
framework decision. Canic owns no database policy; the framework owns no Canic
token or grant state.

B1 must complete seven evidence gates before B2:

1. inventory and freeze complete propagation of the selected hard cut that adds
   a required signed presenter to token claims, binds it into the canonical
   claims hash and proof, requires it to equal the current caller in both
   authorization lanes and removes the old presenter-less and
   subject-equals-caller meanings; the authorized mutating batches perform the
   propagation, not B1;
2. inventory and approve the canonical scope hard cut and application-scope
   issuance path;
3. prove the 60-second establishment-proof limit and admitted issuance burst
   cannot exhaust live replay tombstones, then separately prove the protected
   session default/maximum contract, active-session capacity and 30-minute
   staleness ceiling;
4. freeze one model owner for scope, verified authority, session and replay
   values;
5. confine caller/time/protected-state acquisition to access/ops while policy
   remains a pure value-to-value decision;
6. freeze the complete authority-generation transition table, including
   verifier, Fleet, role, issuer, granted-scope, default-TTL, maximum-TTL and
   subject-admissibility changes; and
7. freeze and prove a browser-neutral authenticated proof-acquisition journey,
   including a native client using a PEM-backed agent identity without giving
   Canic access to the private key.

The generic application-guard contract does not make Canic the only guard
provider. A maintained bounded static-principal allowlist is recommended as
downstream IcyDB-owned work so a plain IcyDB Canister can support Explorer
without adopting Canic. That work is independent, remains outside this
repository and is not a 0.103 promotion dependency.

## Release-Batch Tracker

| Batch | Outcome | Owner | Included evidence | Validation | State |
| --- | --- | --- | --- | --- | --- |
| B1 | Current auth/session inventory and frozen generic contract | auth model/ops/access and host inventory | seven mandatory decisions, complete presenter-propagation inventory, authority-generation transition table, separate proof/session capacity evidence, exact predecessor inventory and duplicate-flow report; no runtime mutation | reproducible `E/A/D/B/H/R/C/M` baseline, native-agent acquisition journey and explicit decision record | Approved alongside 0.102 evidence; mutating work blocked |
| B2 | Canonical scope, verified-authority projection and pure denial policy | model and policy | scope hard cut, narrowing, binding, replay and replacement policy | focused pure-policy and canonical-scope tests | Blocked on B1 |
| B3 | Hard-cut scoped session state, index, bounds and lifecycle | model, ops and workflow | atomic state, proof-expiry-independent active sessions, exact-retry receipt, tombstones, capacity, cleanup and synchronous restore | focused state, proof/session-expiry, corruption, retry and reconstruction tests | Blocked on B2 |
| B4 | Standard establish, clear and self-status operations | DTO, endpoints/macros and host configuration | enablement, protected default/maximum TTL, current Candid and exact released 0.102 compact diagnostics | exact surface checks and focused PocketIC operation tests | Blocked on B3 and 0.102 closeout |
| B5 | Public synchronous facade, generic consumer and native guard convergence | access and facade | unchanged-ABI fixture, pure local decision and duplicate removal | focused facade and generic PocketIC consumer tests | Blocked on B4 |
| B6 | Operator inspection, security and performance gates | host/CLI, metrics and audit workflows | protected audit, generation invalidation, bounds and no-secret evidence | inspection, memory, instruction, restore and Wasm measurements | Blocked on B5 |
| B7 | Canic-only qualification, cleanup and closeout | cross-layer release batch | native-client multi-target establishment, controller separation, proof/session expiry, clear, same-release recovery and residue cleanup | focused PocketIC security/recovery matrix and maintained-doc review | Blocked on B6 |

## Promotion Questions

B1 must answer:

1. exact current token/session Candid and source consumers;
2. complete producer/consumer inventory and frozen propagation plan for the
   required signed presenter, including preparation-caller derivation,
   canonical encoding, signing, retrieval, verification, caching, direct
   authorization and session commit, with no presenter-less decoder or
   first-use bearer fallback;
3. predecessor scope grammar, the proposed hard cut and application-scope
   issuance ownership;
4. accepted active-session and per-subject capacities for the 30-minute
   occupancy window, separately from replay-binding capacity for the rolling
   60-second proof-issuance burst;
5. exact protected default and maximum tokenless-session TTLs, each no greater
   than 1,800 seconds, and the maximum complete proof lifetime accepted for
   establishment;
6. exact local authority-generation owner and the complete transition table for
   verifier, Fleet, role, issuer, granted-scope, default-TTL, maximum-TTL and
   subject-admissibility changes;
7. canonical stable snapshot versus derived heap-index representation;
8. one declaration owner for scopes, verified authority, sessions and replay;
9. public method/type names, inactive-status variants and closed denial
   precedence;
10. native proof-bearing/session-bearing policy convergence boundary;
11. exact browser-neutral preparation, retrieval and current proof-presentation
    journey for a native agent identity, plus the intended target-local
    establishment inputs that B7 will qualify after implementation;
12. current and proposed instruction, stable-byte, heap-byte, restore and
    raw-Wasm costs; and
13. the precise 0.108 authentication-profile reconciliation.

## Next Authorized Action

B1 may inspect and measure the current Canic surface, resolve the seven mandatory
decisions, freeze the exact generic contract and return the evidence for review.
B2-B7 remain blocked until 0.102 is accepted and complete and that review
explicitly authorizes mutation. No runtime, Candid, stable-state,
package-version, changelog-version or downstream-repository mutation is
authorized.
