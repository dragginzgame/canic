# Canic 0.105 Implementation Status

Date: 2026-08-19

## Status

- State: B1 accepted and B2 complete; B3 is paused on the reopened 0.104
  predecessor gate. Existing 0.105 work remains preserved.
- Outcome: one framework-neutral bridge from verified Canic token authority to
  a bounded synchronous local application decision.
- Review state: the lifetime, client-access, predecessor,
  lifecycle-composition and revocation decisions are reproduced in the
  retained B1 evidence. The maintainer accepted caller-derived presenter and
  subject identity plus target-local typed replay-capacity denial.
- Runtime impact: B2 hard-cuts the presenter-bearing token, canonical
  application scope, verified authority projection and pure authorization
  decisions. B1 itself added only test qualification and evidence.
- Implementation approval: paused. B3-B7 remain blocked until the eventual
  published 0.104 corrective release is reconciled.
- Surface posture: enabled managed roles add establish/clear command variants
  and one application-session status variant to the fixed 0.103 role surface.
  Infrastructure roles cannot enable the capability; no session-specific
  method exists. Caller-self and protected operator status variants authorize
  independently before session state is read.

Design:
[Framework-neutral local application authorization](0.105-design.md)

## Scheduled Boundary

0.105 is scheduled immediately after the eventual accepted 0.104 closeout. B1 was captured
against exact released tag `v0.104.1`, peeled commit
`464c186d9d82112d1ea4c7bdb1f47bcd5e5224a5`; the 2026-08-19 audit has since
reopened that predecessor boundary. The retained B1/B2 work introduces no IcyDB
dependency and makes no runtime, public API, Candid, persisted-state,
package-version or release mutation.

The retained 0.101.53 evidence recorded this predecessor shape:

- delegated tokens are verified locally against Fleet, role, subject, scope
  and time;
- `auth::authenticated(scope)` obtains the token from Candid argument zero;
- delegated sessions retain subject, issue/expiry and bootstrap fingerprint,
  but not verified local scopes or issuer context; and
- session lookup is bounded but is not yet the proposed read-only indexed
  foreign-guard contract.

That evidence is historical input only. The current
[B1 inventory and decision record](../../audits/working/0.105-local-application-authorization/README.md)
and [resource baseline](../../audits/working/0.105-local-application-authorization/resource-baseline.md)
replace it for 0.105 promotion review.

The read-only IcyDB 0.226 design and Explorer feedback are requirements evidence
only. Canic will not modify IcyDB, import it or name its SQL/schema surfaces in
Canic contracts.

The authorization contracts compose at the function level. The preceding
[0.104 synchronous lifecycle-participant contract](../0.104-ic-timers-consumer-hard-cut/0.104-design.md)
lets one managed application restore Canic and IcyDB under one lifecycle
owner. This line consumes that seam but adds no framework dependency or
lifecycle authority of its own.

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
   authorization lanes, derives subject from the same caller and removes the
   old presenter-less and caller-nominated-subject meanings; the authorized
   mutating batches perform the propagation, not B1;
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
repository and is not a 0.105 promotion dependency.

## Release-Batch Tracker

| Batch | Outcome | Owner | Included evidence | Validation | State |
| --- | --- | --- | --- | --- | --- |
| B1 | Current auth/session inventory and frozen generic contract | auth model/ops/access and host inventory | seven mandatory decisions, complete presenter-propagation inventory, authority-generation transition table, separate proof/session capacity evidence, exact predecessor inventory and duplicate-flow report; no runtime mutation | reproducible `E/A/D/B/H/R/C/M` baseline, native-agent acquisition journey and explicit decision record | Accepted 2026-08-19 |
| B2 | Canonical scope, verified-authority projection and pure denial policy | model and policy | scope hard cut, narrowing, binding, replay and replacement policy | focused pure-policy and canonical-scope tests | Completed 2026-08-19 |
| B3 | Hard-cut scoped session state, index, bounds and lifecycle | model, ops and workflow | atomic state, proof-expiry-independent active sessions, exact-retry receipt, tombstones, capacity, cleanup and synchronous restore | focused state, proof/session-expiry, corruption, retry and reconstruction tests | Paused on reopened 0.104 |
| B4 | Standard establish, clear and self-status variants | DTO, managed-role dispatch/macros and host configuration | enablement, protected default/maximum TTL, current role Candid and compact diagnostics descended from 0.102 | exact variant/surface checks and focused PocketIC operation tests | Blocked on B3 |
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
9. public command/status variant and type names, their managed-role integration,
   inactive-status variants and closed denial precedence;
10. native proof-bearing/session-bearing policy convergence boundary;
11. exact browser-neutral preparation, retrieval and current proof-presentation
    journey for a native agent identity, plus the intended target-local
    establishment inputs that B7 will qualify after implementation;
12. current and proposed instruction, stable-byte, heap-byte, restore and
    raw-Wasm costs; and
13. the precise declarative-authentication-profile reconciliation.

## Accepted B1 Decisions

The request no longer nominates a subject. Preparation derives both signed
`presenter` and signed `subject` from the authenticated caller. Verification
requires `subject == presenter == current transport caller`; no bearer,
different-subject or compatibility lane remains.

Replay admission is target-local. The target retains every live consumed proof
and denies fresh growth with a typed capacity result at 256 live proofs per
subject or 4,096 per Canister. It never evicts live authority and does not
misrepresent issuer-local preparation limits as a Fleet-wide quota.

## Next Authorized Action

Preserve B1/B2 and the existing B3 working state without further successor
mutation. Resume B3 only after published `v0.104.2` is reconciled as the exact
predecessor. Do not add compatibility records, live
tombstone eviction or a second authority generation.
