# Idea: Cross-Subnet Data Transport Groundwork

Date: 2026-07-26

## Status

- Classification: deferred, unnumbered idea. Its former working number was
  `0.106`; it is not a scheduled Fleet-expansion predecessor.
- Former review status: **TBD; proposed groundwork for maintainer review.**
- Release boundary: reinstall only. 0.106 installs one current call surface
  and consumes no prior-release state or Wasm.
- Readiness: not implementation-approved. The route, measurement and
  one-attempt call boundary are decided below, but Slice A must qualify the
  pinned platform surface and settle final type ownership first. The open
  database decisions near the end do not block that groundwork; they must be
  resolved before a later data protocol is implemented.
- Sequence: this is the 0.106 design line. It follows:
  - [0.99 App/Fleet identity and terminology](../../archive/0.99-app-fleet-identity-and-terminology-hard-cut/0.99-design.md);
  - [0.100 Fleet coordination and Registry synchronization](../../archive/0.100-multi-subnet-fleet-coordinator-and-registry-synchronization/0.100-design.md);
  - [0.101 composable Component deployment and Fleet service publication](../../archive/0.101-fleet-authoritative-service-provisioning-and-publication/0.101-design.md);
  - [0.102 compact diagnostic codes](../../archive/0.102-compact-diagnostic-codes/0.102-design.md);
  - the released [0.105 local application authorization design](../../archive/0.105-framework-neutral-local-application-authorization/0.105-design.md);
  - the released [0.104 synchronous lifecycle-participant contract](../../archive/0.104-ic-timers-consumer-hard-cut/0.104-design.md); and
  - the evidence-only [0.106 Fleet-estate platform qualification](../../0.106-fleet-estate-platform-qualification/0.106-design.md), which supplies no transport authority.
- Dependency posture: the Fleet-aware route model depends on the protected
  topology and Fleet service bindings designed by 0.100 and 0.101. The
  generic call-wrapper measurements can be implemented independently once the
  final 0.99 call surface is known.
- Protocol posture: this line introduces no database replication protocol,
  database wire schema or writable replica.
- Product-path posture: 0.106 is not required for client-directed application
  routing. A client may call its selected application Canister directly and
  present chain-key-backed credentials for local verification. This groundwork
  matters only when one Canister workflow actually calls another Canister and
  the route may cross Subnets.
- Versioning posture: there is one current call surface. No generation
  suffix, negotiation, compatibility decoder or parallel call wrapper is
  introduced.
- Repository scope: Canic only. IcyDB and other database repositories remain
  read-only downstream integrations.
- Independence posture: this transport idea neither requires nor extends 0.105
  application sessions. Transport routes carry explicit Canister-to-Canister
  authority; they do not substitute for an ingress application subject.
- Lifecycle posture: 0.106 neither requires nor extends the 0.104 synchronous
  participant seam. Its call wrapper remains ordinary application/runtime work
  after lifecycle restoration.

## Summary

0.106 prepares Canic for a later data path in which one read-only replica or
materialized-view Component communicates with one canonical writable service
Component.

It does **not** implement that data path.

The groundwork has five responsibilities:

1. Know the calling Canister's own Subnet without an extra network call and
   verify it against protected Component, root or Coordinator placement.
2. Resolve one exact configured Authority, Replica or PoolMember endpoint from
   a published Fleet service, join its
   `(component, fleet_subnet_root, canister_id)` to
   `FleetSubnetRootDirectoryEntry.placement_subnet`, then classify the route
   as same-Subnet or cross-Subnet.
3. Extend the existing one-attempt Canic call wrapper with explicit timeout,
   caller-supplied frame limits and route-observation support.
4. Measure route-sensitive latency, bytes, outcomes and the platform-provided
   call-cost reserve without inventing a per-call billing claim.
5. Prove the behavior on real same-Subnet and cross-Subnet PocketIC routes
   before a database protocol is designed on top.

The eventual direction remains:

~~~text
read-only Component
  → bounded, authenticated, idempotent request
  → canonical writable service Component commits
  → canonical data revision advances
  → read-only service obtains and atomically activates a bounded result
~~~

That text is a direction, not a wire contract. 0.101 owns service ID, mode and
configured member identity. The database direction uses an AuthorityReplica
service; snapshot versus log transport, database revisions, Replica readiness,
direct authorization and refresh semantics remain later decisions. The
Coordinator Subnet is not part of this route derivation. A canonical service
may happen to share that physical Subnet, but its placement still comes from
its owning Fleet Subnet Root.

## Why a Separate Groundwork Line

The existing `canic::api::call::Call` is an ordinary IC call builder. It
provides:

- bounded or unbounded response waiting;
- Candid and raw argument encoding;
- optional attached cycles;
- typed response decoding; and
- current inter-Canister call metrics.

It deliberately does not replace Canic's protected capability RPC and does
not own topology, retry policy or durable operation recovery.

That is the correct primitive boundary, but it is insufficient evidence for a
future data path:

- the wrapper records target and method count but not route class;
- it does not expose a caller-selected bounded timeout;
- it does not accept or enforce protocol-specific request and response limits;
- it does not record request bytes, response bytes or elapsed time;
- it does not expose the platform's required call-cost reserve;
- it cannot distinguish an observed cross-Subnet delay from an ordinary call;
  and
- it has no rule preventing a future caller from adding unsafe implicit
  retries around a timed-out mutation.

Adding database semantics directly to that primitive would collapse
transport, topology, consistency and recovery into one layer. 0.106 instead
makes the primitive measurable and gives Fleet workflows one protected route
input.

## Platform Facts and Constraints

The IC routes an ordinary inter-Canister update call by target principal. The
caller does not choose a local or cross-Subnet transport and does not need to
know the target's Subnet for the call to work.

The route still changes important operating characteristics:

- same-Subnet calls avoid XNet streaming and normally have lower latency;
- cross-Subnet calls require additional consensus and XNet routing rounds;
- a same-Subnet request may be larger than a cross-Subnet request;
- composite-query calls are same-Subnet-only and cannot provide the eventual
  cross-Subnet database path;
- cross-Subnet transmission cost grows with bytes; and
- execution and storage cost depend on the replication factor of the Subnet
  executing or storing the work.

0.106 must not hard-code current cycle prices. Platform prices and Subnet
membership may change. It records measured bytes and the current
platform-provided call-cost reserve, while host tooling may apply the current
price schedule when presenting estimates.

The current documented request ceilings are 10 MiB for a same-Subnet call and
2 MiB for a cross-Subnet call. A replicated-execution response is limited to
2 MiB regardless of the request route. Those are platform limits, not suitable
Canic protocol-frame targets.

The repository's pinned `ic-cdk 0.20.2` exposes the required primitives:
`ic_cdk::api::subnet_self`, bounded-wait `change_timeout`, `Call::get_cost`,
raw successful response bytes and clean/non-clean rejection classification.
0.106 qualifies and wraps those current primitives; it does not invent a
parallel transport API.

## Architectural Boundary

The maintained stack is:

~~~text
application or Canic workflow
  → consistency, authorization, idempotency and retry decision
  → protected Fleet route resolution
  → one-attempt policy (wait, request limit, response limit)
  → ordinary Canic call wrapper
  → IC call transport
~~~

Each layer has one responsibility.

### One-Attempt Call Wrapper

The ordinary Canic call wrapper:

- encodes one request;
- validates the encoded request against its caller-supplied limit before
  dispatch;
- retains the caller-supplied response limit for raw-reply validation;
- records one attempt;
- dispatches exactly one IC call;
- records one result; and
- decodes or returns the response.

It does not:

- discover Fleet topology asynchronously;
- know about Component Specs, Component instance IDs or Fleet service roles;
- poll the NNS Registry before a call;
- retry;
- assign an operation ID;
- decide whether a mutation is safe to repeat;
- persist a receipt;
- interpret a database revision; or
- transform application data.

### Fleet Route Resolution

A Fleet workflow may attach route context only from protected Canic state.
Route context determines limits and observability; it never grants endpoint
authorization. The generic wrapper receives already-resolved attempt policy;
it does not interpret a Fleet Directory or Fleet Registry.

### Database Workflow

A later database workflow will own:

- the mutation or refresh request;
- authorization of the exact peer;
- idempotency and expected data revision;
- durable pending intent and response receipt;
- timeout reconciliation;
- page, snapshot or log semantics; and
- atomic activation of received data.

None of those behaviors is implemented in 0.106.

The expected data-plane call is direct from the calling read-only service
Canister to the published canonical writable service Canister. In transport
terms those are the caller/call source and callee/call target. In data terms
they are the read-only consumer and canonical data authority. A Fleet Subnet
Root distributes protected topology and owns local platform lifecycle effects;
it is not an implicit data proxy. Any future relay must be an explicitly
admitted Component role with its own protocol.

## Route Model

The semantic route classification is:

~~~rust
pub enum CallRoute {
    SameSubnet,
    CrossSubnet,
    Unknown,
}
~~~

The exact final Rust owner and visibility remain a Slice A decision, but these
three states are required.

`Unknown` is not an error disguise and must not be guessed from latency. It
means Canic lacks current qualified target-placement evidence at dispatch.

### Source Subnet

The caller obtains its own Subnet synchronously from the current IC system API
through `ic_cdk::api::subnet_self()`.

The existing asynchronous NNS Registry lookup for the calling Canister must
not be retained as the normal per-call self-discovery path once the current
system API is admitted and qualified.

Canic compares the runtime value with the narrowest protected placement owner:

- a Component has its exact protected `ComponentBinding`;
- a Component Child has its exact protected `ComponentChildBinding`;
- a Fleet Subnet Root has its Fleet Registry Mirror and protected
  `FleetSubnetRootBinding`;
- the Fleet Coordinator uses `FleetCoordinatorBinding.coordinator_subnet`; and
- a generic Canister without qualified placement can obtain its runtime Subnet
  but cannot construct a qualified Fleet route context.

When runtime `subnet_self()` contradicts the applicable protected state:

- the route is not silently reclassified;
- protected identity is not rewritten;
- Fleet-managed cross-Subnet work fails closed; and
- diagnostics direct the operator to placement or migration recovery.

### Target Subnet

For a Fleet-managed call, the target Subnet comes from the narrowest protected
owner:

- a configured Fleet service endpoint resolves one exact member of
  `FleetDirectoryService.members`, validates its purpose against the service
  mode, then resolves that member's root principal to one `Active`
  `FleetSubnetRootDirectoryEntry` and its `placement_subnet`;
- a Fleet Subnet Root resolves from its exact `FleetSubnetRootEntry`;
- the Fleet Coordinator resolves from
  `FleetCoordinatorBinding.coordinator_subnet` for Coordinator control-plane
  calls only;
- the Fleet Subnet Root-owned Component Registry for a locally managed
  Component or Component Child; or
- later qualified placement evidence owned by the protocol introducing that
  target.

The locally activated Fleet Directory retains the service's exact Fleet
Subnet Root and that root's placement, so managed Canisters can derive route
policy without calling the Coordinator. It remains a discovery projection:
its provenance must match protected Fleet state, and it never becomes
endpoint authorization. A separately supplied public Fleet or Component
Directory response is only an untrusted hint.

The Coordinator Subnet is not a fallback service placement. Equality between
a service root's Subnet and the Coordinator Subnet is ordinary co-location;
inequality is ordinary independent placement.

The NNS Registry's `get_subnet_for_canister` lookup remains useful for
operator diagnostics, installation qualification and reconciliation. 0.106
does not add one NNS lookup before every application call.

### Subnet Kind and Cloud Engine Qualification

The
[0.100 Subnet topology amendment](../../archive/0.100-multi-subnet-fleet-coordinator-and-registry-synchronization/0.100-design.md#physical-subnet-kind-and-topology-evidence)
preserves the IC-native
`SubnetKind::{Application, CloudEngine, System, Unknown}`, the host-side
registry-version-bound provider observation and the eventual protected
`SubnetPlacement`. `IC mainnet` is network scope, not a kind. `SubnetKind` is
independent of route:

~~~text
same Subnet ID + any observed kind       → SameSubnet
different Subnet IDs + any observed kind → CrossSubnet
~~~

Kind does not authorize the peer and cannot turn a cross-Subnet route into a
same-Subnet route. It qualifies which platform call effects are available
after the route is derived. In particular, 0.106 must not assume that a Cloud
Engine supports every combination of cross-Subnet attached cycles, bounded
wait and unbounded wait available between NNS-managed Subnets. The exact
current platform behavior must be qualified before implementation; unsupported
effects reject before dispatch or use a separately designed NNS-managed Subnet
proxy with its own authority, funding and receipt model.

The `.icq` cache and `ic-query` reports remain host-only. A runtime never
consults them before a call. Until the later protected-placement cut
distributes `SubnetKind` through bindings and Fleet Directory rows, runtime
kind is unqualified and cannot enable a kind-sensitive effect.

### Route Derivation

Given qualified source and target Subnet identities:

~~~text
source == target  → SameSubnet
source != target  → CrossSubnet
missing evidence  → Unknown
contradiction     → fail closed for Fleet-managed work
~~~

The caller cannot supply a raw `SameSubnet` label that relaxes a limit. Public
application code may provide an untrusted topology hint for labels only, but a
hint never permits a larger payload, shorter recovery path or weaker
authorization.

Two Canisters belonging to different Fleets may classify as `SameSubnet`.
Route equality says nothing about Fleet membership or permission; without an
explicit cross-Fleet protocol their authority check still rejects.

### Placement Changes

An IC call continues to route by principal if the target Canister moves.
Therefore:

- route classification is operational evidence, not Canister identity;
- correctness must not depend on a same-Subnet-only wire format;
- an old route observation may affect a metric label but cannot authorize a
  peer;
- protected Fleet placement contradictions fail closed until topology is
  reconciled; and
- every route-capable protocol selects limits safe for a cross-Subnet route,
  even when source and target are currently co-located.

## Provisional Route Context

The eventual operations-layer input should carry a named qualified context
rather than loose booleans:

~~~rust
pub struct QualifiedCallRouteContext {
    pub source_placement: SubnetPlacement,
    pub target_placement: SubnetPlacement,
    pub evidence: QualifiedCallRouteEvidence,
}

pub struct QualifiedCallRouteEvidence {
    pub source: ProtectedSourcePlacement,
    pub target: ProtectedTargetPlacement,
}

pub enum ProtectedSourcePlacement {
    Component {
        binding: ComponentBinding,
    },
    ComponentChild {
        binding: ComponentChildBinding,
    },
    FleetSubnetRoot {
        binding: FleetSubnetRootBinding,
    },
    Coordinator {
        binding: FleetCoordinatorBinding,
    },
}

pub enum ProtectedTargetPlacement {
    FleetServiceMember {
        fleet_directory: FleetDirectoryProvenance,
        service: FleetDirectoryService,
        member: FleetDirectoryServiceComponent,
        root: FleetSubnetRootDirectoryEntry,
    },
    Component {
        binding: ComponentBinding,
    },
    ComponentChild {
        binding: ComponentChildBinding,
    },
    FleetSubnetRoot {
        binding: FleetSubnetRootBinding,
    },
    Coordinator {
        binding: FleetCoordinatorBinding,
    },
}
~~~

This shape is illustrative, not yet a public or stable DTO. `CallRoute` is
derived from `source_placement.subnet` and `target_placement.subnet`; kind is
not independently caller-set and does not participate in Subnet equality.
`FleetServiceMember` must prove `member` is one exact member of
`service.members`, that its purpose is admitted by `service.mode`, and that the
root entry matches that member's root. A caller cannot substitute another
member or another placement on the same root.
Unresolved evidence does not construct a qualified context; the route-aware
workflow returns `Unknown` or a typed missing-evidence failure according to
its policy.

The final type must satisfy:

- Fleet identity comes from exact protected binding or Fleet Directory
  provenance, not a duplicated loose field;
- evidence binds distinct current protected source and target observations;
- a Fleet-managed source and target bind the same exact Fleet unless a later
  protocol explicitly introduces cross-Fleet authority;
- unknown evidence cannot be represented as trusted;
- a local-registry route cannot claim a remote placement;
- route context is not serialized into every application payload merely for
  metrics; and
- no public caller can forge evidence that changes safety policy.

## Call Effect and Wait Policy

Route and effect are separate dimensions.

The semantic effect classes are:

~~~rust
pub enum CallEffect {
    Observation,
    ReceiptBackedMutation,
    UnreconciledMutation,
}
~~~

The final names remain TBD, but the distinction is required.

### Observation

An observation may use bounded wait when:

- the endpoint is logically read-only;
- repeating the request has no authoritative side effect; and
- a timed-out result is safe to request again.

The 0.100 Registry head and snapshot calls are the model: authenticated update
calls that are operationally repeatable.

### Receipt-Backed Mutation

A mutation may use bounded wait only when its owning workflow provides:

- one durable operation or idempotency identity;
- one canonical request hash;
- exact expected authority and data revision;
- a way to retrieve or reproduce the committed result; and
- a recovery path for `SYS_UNKNOWN` or interruption after dispatch.

The wrapper does not create those guarantees.

### Unreconciled Mutation

The wrapper must never retry an unreconciled mutation. Selecting bounded wait
for one requires an explicit caller decision and returns an unknown-outcome
failure on timeout. A later database design should reject this class for
canonical data mutation.

### Timeout

0.106 adds an explicit bounded timeout to the Canic wrapper rather than relying
only on the CDK default. Timeout values belong to the operation policy, not to
the route alone:

- a cross-Subnet route may justify a different operational threshold;
- same-Subnet congestion can still exceed a local expectation;
- a timeout does not cancel a dispatched call; and
- timeout never proves that the callee did not commit.

The pinned CDK silently caps `change_timeout` at its current maximum and treats
zero as a real, usually immediate timeout rather than unbounded wait. Canic
must validate a nonzero timeout no larger than the qualified pinned maximum
before constructing the call instead of relying on either behavior. No default
database timeout is selected in 0.106.

The current CDK also distinguishes clean rejects, where the callee did not
execute, from non-clean rejects with an unknown outcome. The wrapper preserves
that typed distinction. It still performs no retry; the owning workflow
decides whether a clean rejection is safe to repeat.

## Frame-Limit Policy

The generic call wrapper accepts explicit per-attempt limits:

~~~rust
pub struct CallFrameLimits {
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
}
~~~

There is no global 128 KiB limit on every Canic call. Management Canister
operations, Wasm chunk upload and existing protocols retain their own
qualified limits. The caller's operation policy must supply nonzero limits no
higher than the applicable platform ceiling. The generic wrapper validates
those values and the encoded frames but does not choose the policy.

0.106 uses this provisional value only for its Fleet data-call qualification
fixture:

~~~rust
const PROVISIONAL_MAX_FLEET_DATA_FRAME_BYTES: usize = 131_072;
~~~

This is 128 KiB and safely below the current 2 MiB cross-Subnet request and
replicated-response ceilings. Matching the 0.100 Registry snapshot size is a
convenient initial test value, not evidence that a later database protocol
should use the same limit. That protocol must select and freeze its own
request and response limits after measuring its exact envelope and workload.

Each supplied limit applies separately to:

- the exact encoded Candid request argument bytes; and
- the exact successful response bytes before semantic decoding.

The provisional fixture limit does not authorize a 128 KiB application payload
inside an envelope. A later protocol must subtract its exact envelope overhead.

The request is rejected before dispatch when its encoded bytes exceed the
request limit. A response exceeding the response limit is rejected before
semantic decoding and recorded as a contract violation. The receiving endpoint
must independently use a bounded/raw endpoint contract capable of rejecting
oversized input without first allocating an unbounded Candid vector.

No 0.106 endpoint splits, chunks, compresses or rejoins data. The qualification
proves that the wrapper can enforce an explicit protocol policy safely below
the current cross-Subnet platform ceilings.

## Cost and Timing Evidence

The wrapper must distinguish estimates from observations.

### Cost Reserve

The current CDK call builder exposes the amount of cycles the caller must hold
above its freezing threshold for the call. That value includes:

- method-name and request-byte cost;
- response-transmission reservation;
- callback-execution reservation; and
- explicitly attached cycles.

Unused reservation may be refunded. It is not the exact final per-call burn.
0.106 therefore names the value `required_cycles_reserve`, not `cost`,
`charged_cycles` or `actual_cycles`.

The wrapper does not infer the target's execution or storage cost from the
caller's balance. Target-side Canister metrics remain the authority for target
consumption.

### Timing

For one attempt, the wrapper records:

- dispatch time;
- reply or rejection observation time;
- elapsed nanoseconds;
- wait mode and configured timeout; and
- whether the result was reply, deterministic rejection or unknown outcome.

Elapsed time is an operational observation, not proof of route. A slow call is
not classified as cross-Subnet and a fast call is not classified as local.

### Bytes

The wrapper records:

- exact encoded request argument bytes;
- exact successful response bytes;
- attached cycles; and
- route class.

Method-name bytes may be recorded separately where needed to reconcile the
platform reserve calculation.

### Metric Cardinality

The existing target-and-method counter remains useful for focused diagnostics
but grows with topology. New route-sensitive metrics must use bounded
dimensions:

- `same_subnet`, `cross_subnet` or `unknown`;
- bounded/unbounded wait;
- effect class;
- outcome class; and
- fixed latency and byte buckets.

The wrapper must not create a durable per-call metric row, unbounded target
history or dynamic error-label table.

Correctness evidence such as an idempotency receipt belongs to the owning
workflow, not the metric subsystem.

## Failure Model

0.106 requires typed distinctions for:

- protected source-Subnet mismatch;
- missing target-placement evidence;
- contradictory target-placement evidence;
- request frame too large;
- response frame too large;
- invalid timeout configuration;
- insufficient required cycles reserve;
- deterministic IC rejection;
- bounded-wait unknown outcome;
- response decoding failure; and
- instrumentation overflow or invariant failure.

If 0.102 is implemented first, these failures receive current compact
diagnostic codes. If 0.106 is implemented first, they remain typed producers
and are included in the later 0.102 inventory. There is no parallel diagnostic
shape.

## Authorization and Security

Route information is never authorization.

The eventual canonical writable-service endpoint must independently validate:

- exact `FleetBinding`;
- current Fleet authority epoch;
- canonical `FleetServiceBinding`, including service ID, role, Component Spec,
  AuthorityReplica mode, member-placement policy and the exact
  Authority-purpose member, owning Fleet Subnet Root and principal;
- the target's exact Component and root binding where relevant;
- allowed caller Component, caller role and caller Canister;
- request operation identity;
- expected service-owned data revision; and
- payload or command hash.

The caller principal remains the transport identity. A claimed source Subnet
inside an application payload is not trusted merely because it matches the
route observation.

Directory entries are lookup data, not administrator, issuer or caller
authority. Component membership, co-location and IC controller status alone
do not grant database mutation. The Coordinator publishes the target binding
but is not the database parent, controller or data-plane proxy.

0.106 does not add encryption. Inter-Canister messages use the IC's replicated
message-routing system. A later protocol must separately decide whether
application-level confidentiality is required for data replicated onto a
different Subnet.

Chain-key-backed credentials remain suitable for direct client-to-Canister
authorization, but they do not eliminate the need for an explicit
Canister-to-Canister caller and operation policy when a later replication
workflow is introduced.

## Stable State and Recovery

The one-attempt wrapper owns no durable pending-call table.

0.106 may persist:

- protected qualified local Subnet identity through the existing Fleet
  binding owner;
- bounded aggregate metrics through their current owner, if current metrics
  persistence requires it; and
- a protected last route contradiction diagnostic where needed for readiness.

It must not persist:

- every call attempt;
- every target principal;
- an unbounded latency history;
- a speculative database cursor;
- an application payload;
- a generic retry queue; or
- an operation receipt without an owning workflow.

An interrupted observational call is repeated by its workflow. An interrupted
mutation is not repeated unless its later protocol has durable idempotency and
reconciliation evidence.

## Interaction With Earlier Designs

### 0.99

0.99 provides immutable Fleet identity and the forward rule that receiving a
request is not commitment. 0.106 does not weaken the Fleet-scoped identity or
reuse an operator-facing name as transport authority.

### 0.100

0.100 remains the sole owner of Fleet Registry head/snapshot transport,
Registry revision and root mirror activation. 0.106 may reuse its:

- protected Component/Spec/root placement evidence;
- exact `FleetRegistryVersion`;
- bounded observational retry principles; and
- authenticated update-call posture.

0.106 does not add Registry deltas or reinterpret
`FleetRegistryVersion` as an application-data revision. It also does not treat
the Coordinator Subnet as a default target. Reusing 128 KiB in the
qualification fixture does not couple later data frames to the Registry
snapshot bound.

### 0.101

0.101 publishes each canonical `FleetServiceBinding` with an exact mode and
complete same-Spec configured member set: one Authority plus zero or more
Replicas, or one or more PoolMembers. 0.106 resolves an exact selected
member's target Subnet only by joining that member's root principal to the
exact active `FleetSubnetRootDirectoryEntry` under matching Fleet Directory
provenance. The selected endpoint's Fleet Subnet Root retains platform
lifecycle and Component Registry authority; the Coordinator remains
publication authority only.

A configured Replica binding proves topology and purpose, not synchronized
application data or read eligibility. 0.106 does not:

- provision another service;
- create, modify or own a service mode or member binding;
- transfer service ownership;
- move application data; or
- make a Directory entry writable.

### 0.102

0.102 owns the maintained compact diagnostic representation. 0.106 adds only
typed failure producers and does not introduce transport error prose or a
second error envelope.

## Explicit Non-Goals

0.106 does not define or implement:

- a change to IcyDB code or endpoints;
- a canonical-writer/read-only-copy database protocol;
- snapshot export or import;
- change-data capture or an application log;
- database cursor or service-owned data revision;
- data-page ordering;
- direct database-to-database authorization;
- Fleet Subnet Root proxying;
- Coordinator proxying;
- push versus pull refresh;
- Authority/Replica provisioning or binding mutation;
- read routing;
- lag or freshness guarantees;
- application-level compression or encryption;
- replica promotion or failover;
- Fleet-authoritative service replacement or relocation;
- conflict resolution or multiple writers;
- application-data migration;
- automatic retries for arbitrary calls; or
- a generic distributed transaction framework.

0.106 also does not change client-directed application routing or local
chain-key credential verification. A Fleet may use 0.100/0.101 without ever
using this transport groundwork.

The maintained term for the future data authority is
**Fleet-authoritative writable service**. “Call source” remains reserved for
the calling Canister in route classification and does not imply data
authority. The writable service's application-data authority remains distinct
from the Coordinator's Fleet-topology authority.

## Implementation Slices

These are tentative work slices, not patch versions or protocol generations.

### Slice A: Current-Surface and Platform Qualification

1. Inventory every current Canic call builder, protected capability call and
   inter-Canister metric owner.
2. Freeze the exact current CDK support for:
   - `subnet_self`;
   - bounded timeout;
   - raw request and response bytes;
   - required call-cost reserve;
   - clean/non-clean rejection classification; and
   - the pinned maximum bounded timeout.
3. Confirm the exact PocketIC support for multiple Subnets and route
   placement.
4. Reconcile the existing asynchronous self-Subnet Registry lookup with the
   direct system API.
5. Qualify the exact current Cloud Engine behavior for cross-Subnet attached
   cycles, bounded wait and unbounded wait.
6. Decide final type ownership and visibility without adding a second call
   facade.

This slice changes no call behavior.

### Slice B: Route Evidence

1. Add the three-state route classification.
2. Resolve source Subnet from the system API and verify it against protected
   local identity.
3. Resolve every exact Fleet service member target through member binding →
   active Fleet Subnet Root → placement Subnet, never through Coordinator
   placement.
4. Retain the exact protected source and target `SubnetKind` when the 0.100
   placement cut has supplied it.
5. Treat unqualified generic targets or kinds as `Unknown`.
6. Add contradiction diagnostics without rewriting protected state.

### Slice C: One-Attempt Wrapper Groundwork

1. Add explicit bounded timeout support.
2. Add encoded request and raw response byte accounting.
3. Add explicit request/response limits without changing unrelated call
   surfaces.
4. Qualify the provisional 128 KiB Fleet data-call fixture.
5. Expose or record `required_cycles_reserve`.
6. Preserve exactly one dispatch per `execute`.
7. Keep retries and receipts outside the wrapper.

### Slice D: Route-Aware Observability

1. Record elapsed time and outcome by bounded route dimensions.
2. Separate estimate, reservation and observed consumption terminology.
3. Add readiness diagnostics for protected placement contradiction.
4. Prove the added metadata and metric dimensions remain bounded.

### Slice E: Same-Subnet and Cross-Subnet Qualification

1. Install two Canic Canisters on one PocketIC Subnet.
2. Install equivalent Canisters on different PocketIC Subnets.
3. Exercise bounded observation, unbounded call and timed-out call paths.
4. Verify route classification never depends on elapsed time.
5. Verify the same explicit provisional fixture limits on both routes.
6. Record request/response bytes, reserve and latency evidence.
7. Prove no automatic retry occurs after an unknown outcome.

### Slice F: Closeout and Replication Input

1. Publish the measured route and call evidence.
2. Remove temporary probes that do not become maintained tests.
3. Freeze the exact call-groundwork contract.
4. Produce a bounded input inventory for the later database transport design.
5. Confirm no database, replica or application-data behavior entered 0.106.

## Validation Strategy

### Unit and Structural Tests

- route derives only from source and target Subnet equality;
- unresolved placement produces `Unknown`;
- contradiction is distinct from unknown;
- untrusted hints cannot relax limits;
- route does not grant authorization;
- request size is checked before dispatch;
- response size is checked before semantic decoding;
- the generic wrapper has no implicit Fleet-sized default;
- explicit timeout is passed only to bounded calls;
- invalid zero or over-maximum timeouts reject before CDK construction;
- one `execute` performs at most one dispatch;
- clean and non-clean rejects remain distinguishable;
- `required_cycles_reserve` is not labelled as actual cost; and
- metric labels and buckets are bounded.

### PocketIC Tests

- verified same-Subnet placement classifies `SameSubnet`;
- verified different-Subnet placement classifies `CrossSubnet`;
- generic unresolved placement classifies `Unknown`;
- protected local-Subnet contradiction fails closed;
- Fleet service Authority, Replica and PoolMember targets each resolve through
  their own exact active Fleet Subnet Root, whether or not that root shares the
  Coordinator Subnet;
- another service member or another placement on the same root cannot be
  substituted for the selected member;
- another Fleet's root on the same physical Subnet cannot satisfy that service
  binding;
- Fleet Subnet Root and Coordinator control-plane targets resolve from their
  distinct protected placements;
- bounded timeout produces an unknown outcome without a second dispatch;
- identical requests use the same explicit fixture limits on both routes; and
- ordinary IC routing succeeds without a separate local/XNet call API.

PocketIC timing and cycle accounting are test evidence for Canic behavior, not
a mainnet performance or price claim.

### Repository Checks

- targeted formatting and Markdown checks;
- exact link validation;
- targeted tests for the call wrapper and metrics owners;
- strict Clippy for changed Rust packages if implementation begins;
- layering and public-surface guards;
- Candid drift checks only if a public endpoint changes; and
- `git diff --check`.

## Groundwork Completion Criteria

0.106 groundwork is complete only when:

1. the ordinary call wrapper still dispatches one transparent IC call;
2. a Canister obtains its source Subnet without a per-call NNS lookup;
3. every Fleet-managed service member endpoint resolves through its published
   Fleet Subnet Root and that root's exact active placement, never through
   Coordinator placement;
4. route is exactly same-Subnet, cross-Subnet or unknown;
5. route classification cannot authorize a caller or relax a bound through an
   untrusted hint;
6. source or target placement contradiction fails closed, and Subnet kind
   never changes ID-based route equality;
7. explicit bounded timeout is supported;
8. the encoded request and raw successful response are measured against
   explicit per-attempt limits;
9. unrelated generic/management calls receive no implicit Fleet frame limit,
   while the provisional 128 KiB fixture is either qualified or replaced by
   measured evidence before implementation approval;
10. required cycle reserve is distinguished from actual consumption;
11. latency, bytes and outcomes are observable by bounded route dimensions;
12. no call-wrapper path retries automatically;
13. same-Subnet and cross-Subnet PocketIC journeys pass;
14. no database, replica, snapshot, log, cursor or application-data protocol
    has been introduced;
15. client-directed routing remains independent of 0.106; and
16. the later database design receives an explicit unresolved-decision and
    evidence inventory.

## Open Decisions Before Database Transport

These are intentionally not answered by 0.106:

1. Which state inside each 0.101 same-Spec Replica is transferred, rebuilt or
   intentionally omitted?
2. Does a read-eligible Replica expose the complete database surface or a
   bounded role-specific projection?
3. Is initial refresh a complete snapshot, ordered pages or a database-native
   export?
4. Is incremental refresh a log, change stream, snapshot replacement or
   application command replay?
5. Which service-owned type defines the canonical data revision and checked
   overflow behavior?
6. Is synchronization pull, push notification followed by pull, or a bounded
   combination?
7. How are idempotency keys, expected data revision and committed result
   receipts encoded?
8. What exact caller Component, caller role, caller principal and canonical
   writable target may exchange data?
9. What proves snapshot or log completeness and ordering?
10. How is a partially received transfer resumed or abandoned?
11. What freshness and lag evidence is public, protected or operational?
12. How are storage growth, cycle budget and backpressure bounded?
13. Is compression worthwhile after instruction and cycle measurement?
14. Does any application data require encryption beyond IC message routing?
15. How does backup/restore preserve the data cursor without permitting
    rollback?
16. What separate later design would fence and promote a replica, if promotion
    is ever admitted?

The Fleet Registry revision must not answer item 5. Registry revision describes
Fleet topology; application data needs a service-owned revision or cursor.
The later protocol begins with direct service-to-service transport. If
measurement justifies a relay, it must declare a dedicated admitted Component
role; neither the Fleet Subnet Root nor Fleet Coordinator becomes an implicit
application-data proxy.

## Expected Follow-On

After 0.106, a separate design may define one read-only replica or read-model
protocol:

~~~text
0.106
  “Can Canic classify, bound, measure and safely attempt this route?”

Later database transport
  “What exact revisioned bytes move, how are they authenticated and resumed,
   and when may a read-only service atomically expose the refreshed result?”
~~~

That later design must begin with one canonical writable Authority and its
configured same-Spec Replica Components published by 0.101. It must not
include promotion, failover or multiple writers unless those are separately
approved.

## Platform References

The platform assumptions were rechecked on 2026-07-25 against:

- the [ICP inter-Canister call guide](https://docs.internetcomputer.org/guides/canister-calls/inter-canister-calls/),
  including bounded/unbounded waiting and cross-Subnet latency;
- the [ICP protocol overview](https://docs.internetcomputer.org/concepts/protocol/),
  including local queues and XNet routing;
- the [ICP execution-error reference](https://docs.internetcomputer.org/references/execution-errors/),
  including the distinct request and replicated-response payload ceilings;
- the [ICP cycle-cost reference](https://docs.internetcomputer.org/references/cycle-costs/),
  including inter-Canister transmission and Subnet-size scaling;
- the pinned [`ic-cdk 0.20.2` call builder](https://docs.rs/ic-cdk/0.20.2/ic_cdk/call/struct.Call.html),
  including bounded timeout, raw response access, rejection classification and
  required call-cost reserve; and
- the [ICP canister-migration guide](https://docs.internetcomputer.org/guides/canister-management/canister-migration/),
  including `get_subnet_for_canister` for placement verification.

Current prices, operational latency and Subnet assignments are not frozen by
this document.
