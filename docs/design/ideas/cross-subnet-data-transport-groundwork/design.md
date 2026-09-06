# Idea: Cross-Subnet Data Transport Groundwork

Reviewed: 2026-09-06

## Status

- Deferred and unnumbered; no implementation or release position is approved.
- Retained need: measure and bound actual Canister-to-Canister application
  calls whose route may cross Subnets.
- Owners: runtime call adapters for one-attempt execution; protected Fleet
  projections for route evidence; application workflows for retry and data.
- Trigger: a concrete application call path and measured transport limitation.
  Database replication and direct client routing do not depend on this idea.
- Repository scope: Canic only. Database repositories remain read-only.

## Current Boundary

The ordinary
[call wrapper](../../../../crates/canic-core/src/ops/ic/call.rs)
supports bounded/unbounded waiting, typed or raw arguments, attached cycles,
typed decoding and call metrics. It does not own Fleet topology, durable
idempotency, database consistency or application retry decisions.

Protected Fleet Directory rows already associate published service members
with their Root and placement. The
[Fleet Registry DTO owner](../../../../crates/canic-core/src/dto/fleet_registry.rs)
defines those projections. Public discovery or a host cache alone cannot
authorize a runtime route.

The current roadmap's Fleet-estate platform qualification is evidence, not
implementation authority for a transport extension.

## Proposed Boundary

Keep one execution path:

1. an authenticated application/workflow selects the exact peer and decides
   retry, consistency and idempotency policy;
2. protected topology resolves source and destination placement where known;
3. pure policy checks explicit wait mode and finite frame/effect limits; and
4. the existing call adapter performs one attempt and records bounded evidence.

Classify a route as same-Subnet, cross-Subnet or unknown. Missing evidence must
not imply same-Subnet placement; contradictory protected placement fails
closed. The Coordinator's Subnet is never a fallback for a service member.

Route classification does not authenticate a peer. Preserve explicit caller,
subject, audience, parent and Subnet bindings at their existing owners.
Unqualified Subnet kind must not enable a kind-sensitive effect.

## Candidate Additions

- An explicit caller-selected bounded timeout, subject to platform limits.
- Request-byte validation before dispatch and successful-response-byte
  validation before typed decoding, using caller-owned protocol bounds.
- Request/response sizes, elapsed time, outcome and route class as bounded
  metrics; avoid unbounded principal/method label growth.
- Platform-provided required call reserve, clearly distinguished from actual
  net cycle consumption.
- Typed distinctions between pre-dispatch failure, rejection and an uncertain
  outcome that may have executed.

A response exceeding the caller's limit may already have incurred execution
and transport cost. Rejecting its decode does not undo the call. A timeout
must never silently trigger a second attempt of a mutation.

Generic and management calls must not acquire implicit Fleet framing,
placement lookups or new limits. Capability RPC remains its existing protected
owner; this extension must not create a parallel RPC framework.

## Evidence Before Scheduling

- Reconfirm the exact pinned CDK and platform timeout, frame, cycle and
  rejection behavior; do not freeze illustrative prices or limits as policy.
- Measure a real need beyond existing call metrics before adding runtime code.
- Prove same-Subnet, cross-Subnet, unknown and contradictory route cases in
  PocketIC with exact service-member and Root placement bindings.
- Test request rejection before dispatch, response rejection before decode,
  uncertain timeout and no automatic retry.
- Show bounded telemetry and Wasm/build cost within the accepted runtime
  contraction budgets.
- Assign final type ownership and a complete implementation/evidence batch.

## Disposition

Retain the bounded transport opportunity. Remove the former release-numbered
plan and speculative database architecture.

Snapshot/log formats, replica freshness, data revisions, promotion, application
authorization, encryption and backpressure need a separate application/data
design if a consumer actually requires them. This idea introduces no database
protocol, state migration or cross-release compatibility.
