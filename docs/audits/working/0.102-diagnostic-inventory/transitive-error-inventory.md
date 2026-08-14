# Canic 0.102 Transitive Error-Owner Inventory

Date: 2026-08-12

## Status

This B1 ledger follows every Canic-owned typed error reachable from the twelve
terminal string-flattening conversions recorded in
[conversion-context.md](conversion-context.md). It records structure and
information-loss boundaries only. Declared variants are not diagnostic codes,
and no number in this document is a protocol allocation.

The immutable source baseline is `v0.101.53` at
`23c0328f78b215580d734ef01b52b35fa3e38ade`.

## Method And Result

The inventory started at the twelve `From<T> for InternalError` owners, followed
every transparent aggregate, typed source field and typed reason field in
current Canic source, and stopped at:

- a concrete Canic-owned variant;
- an untyped string or formatter boundary;
- an external dependency error; or
- a type already visited through another root.

The first resulting union contains **54 Canic-owned typed owners and 514
declared variants**. This is the exact declaration count reached before the
inventory encounters an untyped string stop, not a production-reachability
claim or a target of 514 compact codes. It deliberately includes transparent wrapper variants,
nested reason variants, one known unproduced variant and several variants whose
meaning may be shared after action, retry, exposure and ownership are compared.

The inventory is grouped below without double-counting an owner that is
reachable through several roots.

| Frontier | Unique Canic-owned types | Declared variants | Principal owners |
| --- | ---: | ---: | --- |
| Configuration and compiled topology | 10 | 150 | `ConfigError`, TOML/schema reasons, Component/Group/deployment/service topology |
| Pure policy | 8 | 33 | aggregate policy, auth, restore/activation fences, environment, scaling and sharding reasons |
| IC infrastructure adapters | 7 | 20 | release-build, Cycles Ledger, ICP refill, management and NNS Registry adapters |
| Authentication operations | 5 | 27 | validation, signature, scope and expiry beneath `AuthOpsError` |
| Cascade topology and Cashier decoding | 2 | 18 | exact topology-snapshot validation and blob Cashier decoding |
| Operations, runtime and stable-state access | 20 | 251 | provisioning plan, Fleet Registry/service, RPC, environment, memory, activation, intent and placement stores |
| Workflow-local terminal owners | 2 | 15 | ICP refill plus its typed policy reason, and Placement Index |
| **Total** | **54** | **514** | |

The 514 figure is reproducible from enum declarations, but declaration count
alone cannot decide allocation. A transparent `OpsError::FleetRegistry` wrapper
does not receive a second code, while two values currently hidden inside one
`ValidationError(String)` may require different leaves when their caller action
or exposure differs.

The union also does not erase path ownership. The same innermost topology
variant may be reached through configuration initialization, live Fleet
Registry validation or activation. Those paths require separate semantic rows
when their owner, action or recovery differs. An enum variant is allocation
evidence only after its exact conversion path has a current producer.

The authentication string stop has now been expanded separately in
[auth-string-frontier.md](auth-string-frontier.md). It adds ten typed owners and
96 non-test structural variants, producing an expanded perimeter of 64 owners
and 610 counted variants before direct prose constructors and dependency-owned
errors. The original 514 is declaration-based, so the combined number remains
conservative rather than a production-reachability claim. The original 54/514
result remains the reproducible first-pass perimeter; neither total is a
proposed allocation count.

## Ownership Graph

### Configuration And Topology

`ConfigError` reaches:

- `ConfigTomlIssue` through `CannotParseToml.issue`;
- `ConfigSchemaError`, which reaches `CanisterRoleNameIssue`;
- `ComponentTopologyError`;
- `ComponentGroupTopologyError`, which reaches
  `ComponentGroupMemberPathError`;
- `ComponentGroupDeploymentTopologyError`, which reuses Component Group and
  Component topology and reaches `ComponentDeploymentMemberLimitError`; and
- `FleetServiceTopologyError`, which reuses Component deployment and Component
  topology.

The outer aggregate variants are ownership edges, not independent leaf
identities. `ConfigSchemaError::ValidationError(String)` is the major exception:
the current string is standing in for many unrelated schema decisions. There
are 85 current source references outside files named `tests.rs`; B4 must replace
the bucket with typed validation reasons before exhaustive mapping. The count is
a conservative textual reference count and is not a producer or code count.

`ConfigError::RuntimeRootKey(String)` is also untyped. Its one current producer
wraps runtime root-key installation failure and must acquire an owned bootstrap
reason rather than share a generic configuration diagnostic.

### Pure Policy

`PolicyError` is entirely transparent over `AuthPolicyError`,
`AuthorityRestoreEndpointPolicyError`, `EnvPolicyError`,
`FleetActivationEndpointPolicyError` and `ScalingPolicyError`.
`ShardingPolicyError` is a separate terminal conversion; its
`ShardCreationBlocked` variant carries the typed `CreateBlockedReason`.

The aggregate wrappers receive no codes. Fence, disabled-feature, unknown-pool,
grant, subject and bounded-policy failures remain separate where retry or caller
action differs. The seven `IcpRefillPolicyViolation` variants are counted under
the ICP-refill workflow frontier because that is the terminal path that exposes
them.

### IC Infrastructure

`IcInfraError` owns transparent edges to `EmbeddedReleaseBuildError`,
`CyclesLedgerInfraError`, `IcpRefillInfraError`, `MgmtInfraError` and
`NnsRegistryInfraError`. Embedded build parsing further reaches
`ReleaseBuildIdParseError`.

It also accepts three dependency-owned surfaces: `ic_cdk::call::CallFailed`,
`candid::Error` and `ic_cdk::call::CandidDecodeFailed`; management signing
accepts `ic_cdk::api::SignCostError`. These dependency types are not eligible to
become Canic's stable diagnostic registry:

- Candid errors and decode failures retain formatter-owned details;
- call rejection includes provider text and a possibly unknown raw reject code;
- `CallFailed` distinguishes insufficient liquid cycles, local call-perform
  failure and remote rejection; and
- `SignCostError` includes a forward-compatible unrecognized system code.

B4 must translate them at the adapter boundary into a finite Canic-owned typed
surface. Raw reject or decoder text must never determine or enter the public
compact code. Exact typed facts that already drive control flow, including
destination-invalid Canister absence, must remain available to that control
flow before projection.

`NnsRegistryInfraError::Rejected { reason: String }` is another provider-text
boundary. The adapter may retain bounded operational context in an approved
owner, but the diagnostic identity must describe the typed Registry rejection,
not its text.

### Authentication Operations

`AuthOpsError` is transparent over `AuthValidationError`, `AuthSignatureError`,
`AuthScopeError` and `AuthExpiryError`. Scope and the three attestation-expiry
variants are already structurally typed. Four delegated-token
`AuthExpiryError` variants have no production constructor; their meanings are
owned by the maintained delegated-token error enums and the dead variants are
B4 sediment. Three input buckets still lose identity:

- `AuthValidationError::Auth(String)`;
- `AuthSignatureError::ProofInvalid(String)`; and
- `AuthSignatureError::AttestationProofInvalid(String)`.

The string content may contain cryptographic or proof-validation details and
cannot be public allocation input. B4 must introduce safe typed causes at the
producer or deliberately map indistinguishable proof failures to one safe code
while retaining an approved numeric internal observation. The current broad
signature conversion is not evidence that delegation and local-Subnet
attestation failures have the same operational owner.

### Operations, Runtime And Storage

`OpsError` is transparent over configuration, Component provisioning, Fleet
Registry, Fleet-service binding, IC infrastructure, RPC, runtime and storage
owners. Its eight variants receive no independent codes.

The nested graph includes:

- `ConfigOpsError -> ConfigError`;
- `FleetRegistryOpsError -> ComponentTopologyError`;
- `RpcOpsError -> RequestOpsError -> IcInfraError`;
- `RuntimeOpsError -> EnvOpsError | StorageError | MemoryRegistryOpsError`;
- `StorageOpsError -> FleetActivationOpsError | IntentStoreOpsError |
  IcpRefillRecordOpsError | PlacementIndexRegistryOpsError |
  ShardingRegistryOpsError`; and
- `FleetActivationOpsError -> PrepareFleetActivationError ->
  ComponentTopologyError`.

Four aggregate owners destroy typed identity before they reach `OpsError`:

| Owner variant | Current input | Required correction |
| --- | --- | --- |
| `ComponentProvisioningPlanOpsError::Configuration(String)` | formatted configuration compiler error | preserve the typed configuration/topology cause |
| `ComponentProvisioningPlanOpsError::FleetRegistry(String)` | formatted `FleetRegistryOpsError` | preserve the exact Registry variant |
| `FleetServiceBindingOpsError::Configuration(String)` | formatted configuration compiler error | preserve the typed configuration/topology cause |
| `FleetServiceBindingOpsError::Plan(String)` | formatted provisioning-plan error | preserve the exact plan variant |

Those variants must not receive generic fallback codes. They either become
typed source variants or disappear once their source is propagated directly.

The partially typed `TemplateManifestOpsError` boundary is closed separately
in [template-manifest-ops-leaves.md](template-manifest-ops-leaves.md): all
thirteen variants have exact dispositions, ten add meanings and three reuse
qualified Store integrity identities. Its aggregate conversion receives no
code.

`FleetActivationOpsError` has three additional prose buckets: `Encode(String)`,
`InvalidRecord { reason: String }` and `InvalidTransition { reason: String }`.
The latter two currently combine many independent record and state-machine
invariants. Their producer sites must be typed before allocation so an invalid
record cannot accidentally share retry behavior with an invalid transition.

`EnvOpsError::MissingFields(String)` likewise carries a set of missing protected
fields in prose. The decision is finite and must be represented by a typed
missing-field reason or a single leaf only if every missing-field case has the
same owner, exposure and remediation.

[runtime-ops-leaves.md](runtime-ops-leaves.md),
[fleet-activation-leaves.md](fleet-activation-leaves.md) and
[storage-registry-leaves.md](storage-registry-leaves.md) now close those bounded
parts of this graph. They preserve transparent causes, group required
environment fields by identical action, split activation transition strings
where action differs and map the three bounded storage registries.
[fleet-control-plane-leaves.md](fleet-control-plane-leaves.md) closes Fleet
Registry, Component provisioning, Fleet-service binding and shared receipt
hashing without allocating formatted aggregate wrappers.
[intent-store-leaves.md](intent-store-leaves.md) closes the 51-variant intent
owner. [memory-adapter-leaves.md](memory-adapter-leaves.md) closes the pinned
memory dependency boundary with 54 grouped known semantics and 20 explicit
non-exhaustive-enum unknown leaves. The expanded authentication stop is closed
in [auth-string-frontier.md](auth-string-frontier.md). The separate
[direct-constructor-frontier.md](direct-constructor-frontier.md) is now closed
at every mechanical and expanded helper/call-site disposition. The remaining
work in this document is transitive cause/formatter ownership, not an open
direct constructor.

### Memory Dependency Boundary

`MemoryRegistryOpsError` reaches Canic's two-variant `MemoryRegistryError` and
three `ic-memory 0.12.3` runtime surfaces. The dependency path recursively
contains non-exhaustive bootstrap, diagnostic, state, construction, declaration,
range-authority, validation, staging, ledger-integrity, commit-recovery and
stable-cell errors. It also contains opaque codec/CBOR failures and generic
policy errors parameterized by Canic's registry error.

The pinned dependency is authoritative for memory safety, but it is not
authority for Canic's public numeric protocol. B4 must add one exhaustive Canic
adapter over the pinned public variants and include a deliberate dependency-
unknown leaf for each non-exhaustive boundary where Rust requires it. A version
bump must then fail the adapter's focused tests or require an explicit mapping
review; formatting the dependency error is forbidden.

The adapter must preserve at least these distinct actions:

- foreign or unsupported physical memory layout: fail closed, no retry;
- declaration or range-policy rejection: correct linked declarations;
- corrupt or ambiguous durable ledger recovery: fail closed and inspect state;
- bootstrap called with changed identity/snapshot: reject contradictory retry;
- reentrant/unavailable runtime access: bounded retry only where the caller's
  same-release workflow permits it; and
- diagnostic query before bootstrap: report readiness without mutating state.

[memory-adapter-leaves.md](memory-adapter-leaves.md) closes this structural
frontier for the pinned `ic-memory 0.12.3` checksum. It maps 131 known reachable
structural leaves to 54 Canic-owned semantic candidates and gives each of the
20 reachable non-exhaustive enum boundaries its own fail-closed unknown leaf.
The mapping is provisional until maintainer allocation review; direct-
constructor reconciliation is complete. A dependency version/checksum change
reopens it.

### Standalone Terminal Owners

The remaining terminal owners are already bounded:

- `TopologySnapshotValidationError`: 14 exact topology-authority violations;
- `CashierDecodeError`: four response-shape violations scheduled for the 0.108
  blob hard cut; they still require current 0.102 allocation if their producers
  have not actually been deleted before the registry freezes;
- `StorageError`: four runtime-log invariants;
- `IcpRefillWorkflowError`: three direct workflow failures plus seven typed
  `IcpRefillPolicyViolation` reasons beneath `PolicyDenied`;
- `RpcWorkflowError`: seventeen live variants, two unproduced variants and
  source-specific replay codec/receipt failures beneath two string wrappers;
  and
- `PlacementIndexWorkflowError`: four exact configuration/parent/role
  decisions.

[bounded-runtime-leaves.md](bounded-runtime-leaves.md) completes that grouping:
60 current exact candidates, including one adjacent untyped build-network
producer and the complete 27-leaf blob hash/lifecycle/billing/Cashier family
that allocates in 0.102 and retires without reuse under the later 0.108 hard
cut, plus four safe public projections.

[rpc-workflow-error-leaves.md](rpc-workflow-error-leaves.md) separately closes
the RPC owner because its broad codec strings cross shared replay receipt and
response authorities. It adds nineteen exact meanings, reuses existing replay
identities and gives no code to either unproduced variant or broad wrapper.

The later family and dynamic-context ledgers complete their semantic grouping,
public projections and masked observability proposals. Their small size never
authorizes one code per displayed variant automatically.

## Information-Loss Gates

B1 cannot approve numeric allocation until all four gates below are closed.

| Loss kind | Current examples | Required evidence before allocation |
| --- | --- | --- |
| Generic string bucket | schema validation, auth validation/proof, activation record/transition, missing environment fields | finite typed reason at the producer, or proof that every producer has identical action, retry, exposure and owner |
| Stringified typed forwarding | provisioning plan and Fleet-service binding | retain the original typed source and allocate only at its semantic owner |
| Dependency formatter boundary | Candid, IC calls, `ic-memory`, signing | owned adapter variants based on typed fields; explicit handling of non-exhaustive/unknown dependency values |
| Broad public collapse | `Forbidden`, `Conflict`, `ResourceExhausted`, `Unavailable`, generic internal projection | split by existing machine decision and exact retry/recovery semantics recorded in [public-boundary.md](public-boundary.md) |

No `String` value, `Display` result or current public class may be hashed,
parsed or otherwise used to choose a compact code.

## Allocation Consequences

The complete allocation must follow these rules:

1. transparent aggregate and pass-through variants receive no duplicate code;
2. a nested typed reason owns the code when it changes action, retry, exposure
   or remediation;
3. unproduced sediment is deleted rather than numbered;
4. external errors are normalized into finite Canic adapter variants before
   mapping;
5. same-semantics variants may share one code only with an explicit grouped row
   naming every producer; and
6. masked internal leaves require an approved numeric observability owner before
   their safe public projection can be approved.

The 514 declared variants therefore establish the review perimeter. They do
not imply a 512-entry host catalogue, numeric bands or reserved capacity.

## Allocation Handoff

All bounded semantic families, the expanded authentication formatter, direct
constructor frontier, RPC and publication aggregates, dynamic public values
and durable string owners now have dispositions. The reconciled qualified set
is 2,844 exact identities plus 31 projection-only identities.

Numeric assignment remains the final B1 action. It must materialize one
mechanical producer manifest and complete host-catalogue row per retained exact
identity, preserve every explicit reuse and projection, and delete sediment
rather than number it. No runtime mutation starts before maintainer review.
