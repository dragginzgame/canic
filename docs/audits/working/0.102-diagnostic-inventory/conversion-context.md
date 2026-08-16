# Canic 0.102 Conversion And Diagnostic-Context Inventory

Date: 2026-08-12

## Status

This B1 ledger classifies every current explicit conversion into public
`Error` or `InternalError`, plus every production caller of
`with_diagnostic_context`, at immutable baseline `v0.101.53`. It allocates no
numeric code. Variant-level leaf assignment remains a later B1 gate.

## Typed Conversion Boundary

There are exactly 39 explicit conversions. Their current structure falls into
four disjoint groups.

### Partially Typed Maps: 12

These conversions already inspect at least one typed variant or preserve a
typed public error, but several still collapse distinct variants into one broad
leaf and generate prose with `to_string()`:

| Conversion owner | Current behavior | Required B4 treatment |
| --- | --- | --- |
| `PublicationWorkflowError` | Maps variants to four Store leaves plus broad conflict, invariant and unavailable | Closed across binding/release, all 56 GC constructions and both typed management transports; B4 maps only the qualified exact leaves and transparent typed causes |
| `TemplateManifestOpsError` | Maps selected variants to Store leaves and flattens the remainder to `Ops` prose | Closed in [template-manifest-ops-leaves.md](template-manifest-ops-leaves.md): ten exact additions, three reuses and no wrapper identity |
| `AccessError -> Error` | Preserves proof expiry, token expiry and broad denial | Replace denial with exact safe access leaf selected before conversion |
| `AccessError -> InternalError` | Same expiry split; broad denial becomes class/origin plus prose | Preserve the typed access cause without reconstructing identity from display text |
| `ComponentAllocationPolicyError` | Groups 17 variants into invalid, exhausted, invariant, unavailable and forbidden | Allocate by action, retry and authority meaning; preserve peer/root decisions |
| `ComponentChildAllocationPolicyError` | Groups 18 variants into five broad public classes plus invariant; one variant has no producer | Preserve parent, grant, activity, authority and capacity decisions independently; delete the unproduced variant |
| `AuthSignatureError` | Splits proof unavailable, invalid proof and missing data certificate | Separate delegation proof and attestation proof validation where remediation or exposure differs |
| `IntentStoreOpsError` | Makes total-record capacity public and delegates every other variant to storage | Preserve exact capacity semantics; map delegated variants at their typed owner |
| `RpcWorkflowError` | Makes disabled funding, operation ID, funding policy and in-progress public; flattens the rest | Closed in [rpc-workflow-error-leaves.md](rpc-workflow-error-leaves.md): nineteen exact additions, generic replay reuse, two codec wrappers and two sediment variants |
| `RpcOpsError` | Preserves a remote public rejection; delegates local request failures | Keep the remote code unchanged and map local typed variants without formatting |
| `ShardingWorkflowError` | Distinguishes policy from invariant but owns an untyped invariant string | Replace the string variant with typed invariant causes before code mapping |
| `CostGuardReserveError` | Preserves an embedded storage error; flattens every other reserve variant | Exhaustively map permit, quota, cycle reserve and arithmetic failures |

### Pass-Through Wrappers: 13

These conversions deliberately delegate to another conversion. They need no
independent code merely for wrapping, but the terminal owner must remain
exhaustive:

- `ScalingPolicyError -> PolicyError`;
- `ConfigOpsError -> OpsError`;
- `AuthValidationError -> AuthOpsError`;
- `AuthScopeError -> AuthOpsError`;
- `AuthExpiryError -> AuthOpsError`;
- `RequestOpsError -> RpcOpsError`;
- `EnvOpsError -> RuntimeOpsError`;
- `MemoryRegistryOpsError -> RuntimeOpsError`;
- `IcpRefillRecordOpsError -> StorageOpsError`;
- `StorageOpsError -> OpsError`;
- `PlacementIndexRegistryOpsError -> StorageOpsError`;
- `ShardingRegistryOpsError -> StorageOpsError`; and
- `RuntimeOpsError -> OpsError`.

B4 must retain these only when the wrapper is a real ownership boundary. It
must not allocate duplicate diagnostics for both wrapper and terminal owner.

### Terminal String-Flattening Conversion Owners: 12

These conversions currently discard all typed variant identity into one broad
class/origin and `to_string()`. Several rows are transparent aggregate enums,
so this is the terminal conversion frontier, not a claim that only the listed
outer variants require analysis:

| Terminal typed owner | Current projection |
| --- | --- |
| `ConfigError` | `Domain/Config` |
| `ConfigSchemaError` | `Domain/Config` |
| `PolicyError` | `Domain/Domain` |
| `ShardingPolicyError` | `Domain/Domain` |
| `IcInfraError` | `Infra/Infra` |
| `AuthOpsError` | `Ops/Ops` |
| `TopologySnapshotValidationError` | `Invariant/Ops` |
| `CashierDecodeError` | `Ops/Ops` pending any promoted standalone blob-service hard cut |
| `OpsError` | `Ops/Ops` |
| `StorageError` | `Invariant/Storage` |
| `IcpRefillWorkflowError` | `Workflow/Workflow` |
| `PlacementIndexWorkflowError` | `Domain/Workflow` |

These 12 conversion owners are the first variant-level allocation input. B1
must recursively follow every transparent nested typed cause, and it must split
generic `String` catch-all variants before B4 where those strings currently
carry distinct decisions. Neither the current broad projection nor derived
display text may decide the compact diagnostic.

### Generic Public Projection: 2

`From<&InternalError> for Error` and `From<InternalError> for Error` both use
the same `internal_error_to_public` fallback. They do not own independent
leaves. B4 replaces this class/origin guess with the exact public code already
stored by the code-first internal error.

## Diagnostic Context Boundary

There are 35 production `with_diagnostic_context` calls in 15 files. The
method definition and one inline unit-test call account for the two additional
textual references in the original census.

| Owner | Calls | Current appended context | Required code-first disposition |
| --- | ---: | --- | --- |
| cascade state/topology | 5 | Child principal, multiple-child failure and topology/cycle-reconciliation stage | Preserve the primary code; record bounded per-child numeric failure through the existing runtime diagnostic owner rather than chaining prose |
| cost-guard workflow | 2 | Permit completion or recovery failure appended to the protected-operation failure | Preserve the primary failure; give cleanup/recovery its own typed numeric observation |
| generic replay workflow | 2 | Cleanup or receipt recovery failure appended to the original error | Preserve original replay decision; retain secondary recovery failure in its operation owner |
| ICP-refill execution/replay | 6 | Commit, cleanup, recovery and operation context | Map exact refill/replay state; use the durable typed refill record for recovery-significant evidence |
| placement acknowledgement | 2 | Root resolution and acknowledgement context | Map the exact placement stage rather than carrying a free-form prefix |
| RPC handler/replay/non-root cycles | 7 | Request identity plus cleanup, response commit and funding recovery context | Preserve exact replay/funding code; write secondary operation failure to its typed receipt/status owner |
| delegated-auth prepare/provisioning | 4 | Recovery commit stage or raw issuer call/response cause | Map prepare state exactly; never expose raw transport/reject text, and retain masked numeric cause through approved auth observability |
| fleet activation | 2 | Failed root call and activation stage | Preserve exact root/stage identity in the activation journal or structured runtime diagnostic |
| runtime intent/bootstrap | 5 | Intent abort and four bootstrap index-rebuild stage labels | Allocate exact restoration/intent leaves; stage names belong in the host catalogue, not runtime-owned strings |
| **Total** | **35** | | |

The code-first design has no generic secondary-cause vector. When a cleanup or
recovery failure follows a primary failure, the primary diagnostic remains the
returned result. The secondary failure must either already have a typed
operation/status owner, be written as a numeric structured runtime diagnostic,
or be intentionally discarded after a documented best-effort cleanup. It must
not be folded into the primary identity.

## Allocation Consequences

This inventory imposes four gates on the full leaf table:

1. every reachable variant in the transitive typed graphs rooted at the 12
   terminal string-flattening conversion owners has an explicit row or is
   deliberately grouped with variants having identical action, retry,
   exposure and owner;
2. every partially typed map is exhaustive and no broad current wire class is
   treated as final identity;
3. pass-through wrappers neither erase a terminal variant nor allocate a
   duplicate wrapper code; and
4. every removed context call names the primary code and any retained numeric
   secondary observability owner.

The two Component policy types are expanded in
[component-policy-leaves.md](component-policy-leaves.md). The complete
transitive perimeter behind the 12 terminal owners is now recorded in
[transitive-error-inventory.md](transitive-error-inventory.md): 54 unique
Canic-owned typed owners and 514 declared variants, including wrappers and
nested reasons. The next B1 step is to turn that bounded perimeter into
semantic family tables. Numbers remain unassigned until those tables also
reconcile with the direct public boundary.
