# Canic 0.102 Public Diagnostic Boundary Inventory

Date: 2026-08-12

## Status

This is the complete current-source inventory of explicit public diagnostic
construction and code-dependent control flow at immutable baseline
`v0.101.53`. It allocates no numeric code. The larger internal producer ledger
remains active because broad `InternalError` fallback projection can still
reach the same public wire.

The boundary audit distinguishes three things that raw `ErrorCode` references
cannot distinguish:

1. a producer that selects a public diagnostic;
2. a consumer that makes a machine decision from that diagnostic; and
3. host-only rendering that preserves a diagnostic without deciding policy.

Any final leaf shared by two sites must still satisfy the stricter semantic
rules in [allocation-proposal.md](allocation-proposal.md).

## Explicit Construction Coverage

There are 151 explicit public `Error::*` constructions or projections before
inline test modules in 26 production files. They are distributed as follows:

| Owner | Sites | Current responsibility |
| --- | ---: | --- |
| `canic-control-plane/api/template` | 1 | Store-facing API state conflict |
| `canic-control-plane/ops/fleet_service_peer` | 4 | Cross-root Fleet-service requester admission |
| `canic-control-plane/ops/storage/template` | 3 | Wasm Store typed manifest/chunk/hash/capacity and GC state |
| `canic-control-plane/workflow/component_registry` | 29 | Component authority, Directory synchronization and runtime activation |
| `canic-control-plane/workflow/component_rpc` | 4 | Component capability parent/recycle authority |
| `canic-control-plane/workflow/runtime/template` | 5 | Store calls, response decoding, manifest lookup and publication mapping |
| `canic-core/api/auth` | 17 | Delegated-token API and session admission |
| `canic-core/api/blob_storage` | 11 | Current blob lifecycle and Cashier mapping pending the 0.108 hard cut |
| `canic-core/api/error` | 5 | Broad fallback projection, not five final leaf meanings |
| `canic-core/workflow/cost_guard` | 2 | Invalid permit and exhausted protected-operation budget |
| `canic-core/workflow/ic/icp_refill/replay` | 7 | ICP-refill replay conflict and quota decisions |
| `canic-core/workflow/placement/allocation` | 2 | Placement operation identity conflicts |
| `canic-core/workflow/rpc/capability` | 15 | Envelope, proof and replay validation |
| `canic-core/workflow/rpc/request` | 12 | Root capability authority and execution decisions |
| `canic-core/workflow/runtime/auth/prepare` | 28 | Auth request admission and replay decisions |
| `canic-core/workflow/runtime/auth/provisioning` | 4 | Root delegation installation call failures |
| `canic` endpoint macros | 2 | Controller-only memory ledger and disabled metrics tier |
| **Total** | **151** | |

This count deliberately excludes:

- the public convenience-constructor definitions themselves;
- typed `AccessError`, policy-error and Store-error conversions;
- `InternalError::auth_*`, `operation_id_required` and
  `root_data_certificate_unavailable` routes;
- the broad `InternalError` class/origin fallback; and
- native host decoders and control-flow consumers.

Those are independent mapping authorities and remain in the complete ledger.
The 151 sites prove the explicit public boundary is bounded; they do not imply
151 final leaves.

## Maintained Wire Leaves And Disposition

The current 20-leaf enum is an inventory input, not the proposed allocation:

| Current leaf | Current semantic reach | Required 0.102 disposition |
| --- | --- | --- |
| `AuthMaterialStale` | Missing, superseded or policy-invalid delegation material | Split wherever repair action or owner differs; preserve explicit lazy-repair decisions |
| `AuthProofExpired` | Expired delegation certificate/proof | Preserve expiry as a typed decision; do not merge with token expiry |
| `AuthProofPending` | Proof generation or provisioning still in progress | Preserve bounded retry semantics |
| `AuthTokenExpired` | Delegated token expired | Preserve remint action separately from proof repair |
| `Conflict` | Replay, operation, lifecycle, Registry, Directory, GC and funding states | Broad class only; split by owner, retry and operation identity behavior |
| `Forbidden` | Authenticated denial, missing membership, authority mismatch and disabled policy | Broad class only; split before any recovery consumer is converted |
| `Internal` | Encoding, invalid response and unprojected infra/ops/workflow failures | Broad class only; internal leaves need explicit safe projection and observability |
| `InternalRpcMalformed` | Cashier response mapping in the current blob subsystem | Cover while current; do not preserve after the independent 0.108 hard cut removes its producer |
| `InvalidInput` | Configuration, envelope, proof, replay metadata and request validation | Broad class only; split by owner and caller correction |
| `InvariantViolation` | Publication, decoding and impossible state | Broad class only; most leaves require masking plus numeric observability |
| `NotFound` | Current blob object not live | Cover the current producer; 0.108 determines its later retirement |
| `OperationIdRequired` | Missing replay identity in several unrelated command families | Split by owning command family because origin and remediation differ |
| `ResourceExhausted` | Quotas, capacities, overflow and funding policy | Broad class only; preserve capacity-specific machine decisions |
| `RootDataCertificateUnavailable` | Certified-query proof requested without a data certificate | Already narrow; retain only for this exact query-context failure |
| `Unauthorized` | Failed endpoint/controller authentication and broad access fallback | Broad class only; split by safe caller action and authentication owner |
| `Unavailable` | Pending state, disabled service, transport failure and missing status | Broad class only; split before retry or fallback consumers are converted |
| `WasmStoreCapacityExceeded` | Store canonical byte/capacity rejection | Preserve as a Store-owned capacity leaf |
| `WasmStoreChunkMissing` | Referenced Store chunk is absent | Preserve as a Store-owned missing-chunk leaf |
| `WasmStoreHashMismatch` | Store chunk or manifest hash disagreement | Split only if producer remediation differs; retain typed publication metrics |
| `WasmStoreManifestMissing` | Referenced Store manifest or binding is absent | Preserve the exact Store lookup/publication distinction if actions differ |

None of these names or groupings reserves a numeric identity.

## Code-Dependent Machine Decisions

These current consumers do more than render an error. The final allocation and
mapping tests must preserve each decision independently:

| Consumer | Current match | Current decision | Required semantic boundary |
| --- | --- | --- | --- |
| `canic-control-plane/src/workflow/component_rpc/mod.rs` | any `Forbidden` from active-member resolution | Attempt durable subtree-removal recovery | Match only the exact member-not-active/missing leaf; unrelated authorization failures must not enter recovery |
| `canic-core/src/access/auth/token.rs` | `AuthProofExpired`, `AuthTokenExpired` | Convert to certificate-expired or token-expired access state | Keep the two expiry owners distinct |
| `canic-core/src/workflow/runtime/auth/prepare/mod.rs` | `AuthMaterialStale` or `AuthProofExpired` | Run one lazy root-proof repair, then revalidate replay ownership | Match only repairable delegation-material leaves |
| `canic-core/src/workflow/runtime/auth/provisioning/mod.rs` | four auth/input leaves | Classify installation as expired/superseded, proof mismatch or signer rejection | Give every branch an exact typed mapping; unknown codes remain signer rejection |
| `canic-core/src/workflow/runtime/auth/renewal.rs` | `AuthProofPending`, `Conflict`, `Unavailable`, or internal infra/ops | Schedule bounded renewal retry | Replace broad conflict/unavailable matching with exact retryable leaves |
| `canic-core/src/workflow/runtime/cycles/mod.rs` | any public `Conflict`, or internal infra/ops | Retry automatic funding | Match the exact funding-in-progress/transient leaves only |
| `canic-core/src/workflow/runtime/cycles/mod.rs` through `InternalError::is_public_resource_exhausted` | any public `ResourceExhausted` | Consume one resource-exhaustion recovery opportunity | Replace the broad-class predicate with the exact recoverable capacity leaf set |
| `canic-host/src/install_root/fleet_component_provisioning_install/mod.rs` | any `Unavailable` | Treat status as not prepared and invoke prepare | Match only the exact provisioning-not-prepared leaf |
| `canic-host/src/fleet_subnet_root_deletion/mod.rs` | any `Unavailable` | Treat optional terminal status as absent | Match only the exact terminal-receipt-absent leaf; transport or fenced state must remain failures |
| Store template/publication workflows and metrics | Store leaves plus broad conflict/invariant/not-found | Select missing-chunk, hash, manifest, capacity or invariant metric/recovery reason | Preserve exhaustive Store-specific identities and stop collapsing unrelated broad leaves |

The two host `Unavailable` consumers prove that a one-for-one replacement of
the current enum is unsafe: the same broad leaf currently authorizes different
effects. The component recycle and automatic funding consumers show the same
problem for `Forbidden`, `Conflict` and `ResourceExhausted` inside Canisters.

## Rendering-Only Consumers

The central ICP response decoder, replica-query decoder, `canic inspect` and
cycles CLI preserve or display the current error without authorizing a state
transition. They must move to the host catalogue in B2/B3, but they do not
define leaf semantics.

`canic-cli` currently stores `ErrorCode` plus message in
`CyclesCommandError::IcpRefillRejected`. The compact cut replaces that pair
with the raw diagnostic identity and host rendering; CLI code must not gain a
second copy of catalogue prose.

## Remaining B1 Work

The public boundary is now bounded and its machine decisions are explicit.
B1 still requires:

1. grouping all reachable internal constructors and typed conversions by
   actionable invariant;
2. classifying every dynamic value currently interpolated into a public message
   through [dynamic-public-context.md](dynamic-public-context.md), with an
   endpoint-specific typed owner for every caller-required unowned value;
3. assigning each resulting leaf a provisional label, class, narrow origin,
   public projection, typed host disposition and action;
4. naming a retrievable, operation-correlated numeric observability owner for
   every masked leaf;
5. proving the complete leaf table covers this boundary plus the native-only
   internal inventory before assigning numbers; and
6. proposing the initial permanent current/retired allocation rows and
   language-neutral current registry under
   [code-allocation-ledger.md](code-allocation-ledger.md).

The next pass starts with typed conversions and `with_diagnostic_context`
callers because they already expose the places where prose is substituting for
identity.
