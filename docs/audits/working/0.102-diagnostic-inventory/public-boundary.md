# Canic 0.102 Public Diagnostic Boundary Inventory

Date: 2026-08-12

## Status

This is the complete inventory of explicit public diagnostic construction at
immutable baseline `v0.101.53`, plus the exact maintained production-consumer
surface refreshed against pinned current-candidate source
`0750c309104b111fa6f5a1b3355c04fcb38faf71`. It allocates no numeric code. The
larger internal producer ledger remains active because broad `InternalError`
fallback projection can still reach the same public wire.

The boundary audit distinguishes three things that raw `ErrorCode` references
cannot distinguish:

1. a producer that selects a public diagnostic;
2. a consumer that makes a machine decision from that diagnostic; and
3. host-only rendering that preserves a diagnostic without deciding policy.

Any final leaf shared by two sites must still satisfy the stricter semantic
rules in the
[allocation proposal](../../../design/0.102-compact-diagnostic-codes/allocation-proposal.md).

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
| `canic-core/api/blob_storage` | 11 | Current blob lifecycle and Cashier mapping pending the 0.109 hard cut |
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
| `InternalRpcMalformed` | Cashier response mapping in the current blob subsystem | Cover while current; do not preserve after the independent 0.109 hard cut removes its producer |
| `InvalidInput` | Configuration, envelope, proof, replay metadata and request validation | Broad class only; split by owner and caller correction |
| `InvariantViolation` | Publication, decoding and impossible state | Broad class only; most leaves require masking plus numeric observability |
| `NotFound` | Current blob object not live | Cover the current producer; 0.109 determines its later retirement |
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

These twelve current production consumers do more than render an error. The
final allocation and mapping tests must preserve each decision independently.
The source coordinates identify the pinned current-candidate code, not moving
worktree lines.

| Consumer function | Baseline site | Current match | Current decision | Required exact replacement |
| --- | --- | --- | --- | --- |
| `recycle_target_authority` | `crates/canic-control-plane/src/workflow/component_rpc/mod.rs:58-69` | any `Forbidden` from active-member resolution | Attempt durable subtree-removal recovery | Match only the exact member-not-active or missing leaf; unrelated authorization failures must not enter recovery |
| `WasmStoreMetricReason::from_manifest_source_error` | `crates/canic-control-plane/src/workflow/runtime/template/mod.rs:344-359` | three Store leaves, any other public leaf, or no public projection | Select missing-chunk, hash, manifest, Store-call or invalid-state metric reason | Preserve exhaustive Store-specific identities and distinguish non-Store public failures from internal invariant state |
| `WasmStoreMetricReason::from_publication_error` | `crates/canic-control-plane/src/workflow/runtime/template/publication/release/metrics.rs:18-37` | four Store leaves plus broad conflict, invariant and not-found classes | Select capacity, missing-chunk, hash, manifest, Store-call or invalid-state metric reason | Match the exact publication leaves; unrelated broad-class failures must not become publication state |
| `access_error_from_verification` | `crates/canic-core/src/access/auth/token.rs:68-74` | `AuthProofExpired`, `AuthTokenExpired`, or other | Convert to certificate-expired, token-expired or denied access state | Keep the two expiry owners distinct and replace the prose-carrying catch-all with an exact safe access cause |
| `delegated_token_prepare_error_allows_lazy_repair` | `crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs:342-349` | `AuthMaterialStale` or `AuthProofExpired` | Run one lazy root-proof repair, then revalidate replay ownership | Match only repairable delegation-material leaves |
| `IssuerProofInstallError::record_failure` | `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs:232-247` | four auth/input leaves plus catch-all | Classify installation as expired or superseded, proof mismatch or signer rejection | Give every branch an exact typed mapping; unknown codes remain signer rejection |
| `is_retryable_renewal_error` | `crates/canic-core/src/workflow/runtime/auth/renewal.rs:323-333` | `AuthProofPending`, any `Conflict`, any `Unavailable`, or internal infra/ops | Schedule bounded renewal retry | Replace broad class and public matches with the exact retryable renewal leaves |
| `is_retryable_funding_error` | `crates/canic-core/src/workflow/runtime/cycles/mod.rs:340-347` | any public `Conflict`, or internal infra/ops | Retry automatic funding | Match only exact funding-in-progress and transient transport leaves |
| `InternalError::is_public_resource_exhausted` | `crates/canic-core/src/error.rs:159-162` | any public `ResourceExhausted` | Classify a failure as eligible for resource-exhaustion recovery | Replace the broad class with the exact recoverable capacity leaf set |
| `claim_resource_exhaustion_recovery` | `crates/canic-core/src/workflow/runtime/cycles/mod.rs:349-352` | the resource-exhaustion classifier plus an in-memory one-shot fence | Consume one resource-exhaustion recovery opportunity | Preserve the exact eligible leaf set and one-shot budget as separate decisions |
| `query_or_prepare` | `crates/canic-host/src/install_root/fleet_component_provisioning_install/mod.rs:144-165` | any rejected `Unavailable` | Treat status as not prepared and invoke prepare | Match only the exact provisioning-not-prepared leaf |
| `query_optional` | `crates/canic-host/src/fleet_subnet_root_deletion/mod.rs:900-915` | any rejected `Unavailable` | Treat optional terminal status as absent | Match only the exact terminal-receipt-absent leaf; transport or fenced state must remain failures |

The two host `Unavailable` consumers prove that a one-for-one replacement of
the current enum is unsafe: the same broad leaf currently authorizes different
effects. The component recycle and automatic funding consumers show the same
problem for `Forbidden`, `Conflict` and `ResourceExhausted` inside Canisters.

## Transparent Decode And Rendering Consumers

These six production consumers preserve, compare or display a diagnostic
without defining one of the semantic decisions above. They must move to the
host catalogue in B2/B3, but they do not define leaf allocation.

| Consumer function | Baseline site | Current input | Current behavior | Required exact replacement |
| --- | --- | --- | --- | --- |
| `decode_json_result_response` | `crates/canic-host/src/icp/response/mod.rs:56-65` | canonical ICP response containing `Result<T, Error>` | Decode and preserve a rejected endpoint error | Decode arbitrary raw numeric identity losslessly and let the central host catalogue render known codes |
| `CanisterProtocolError::is_rejected_with` | `crates/canic-host/src/canister_protocol/mod.rs:69-78` | one rejected endpoint error and caller-selected current enum leaf | Perform an exact equality predicate for host workflows | Compare raw identities exactly; callers must supply the narrow registered host constant they own |
| `decode_cycle_balance_response` | `crates/canic-host/src/replica_query/mod.rs:34-41` | replica response containing `Result<u128, Error>` | Preserve a Canister rejection separately from transport and decode failures | Preserve arbitrary raw numeric identity and render it through the central catalogue |
| `CyclesCommandError::IcpRefillRejected` | `crates/canic-cli/src/cycles/mod.rs:34-70` | current `ErrorCode` plus message | Retain and render the endpoint rejection | Retain the raw identity only and delegate rich rendering to `canic-host` |
| `decode_icp_refill_response` | `crates/canic-cli/src/cycles/convert/response.rs:60-83` | ICP refill response containing `Result<_, Error>` | Split endpoint rejection from malformed or mismatched terminal response | Preserve the raw rejection identity without copying message or catalogue prose into the CLI |
| `runtime_response_payload` | `crates/canic-cli/src/inspect/mod.rs:357-365` | runtime-status endpoint response through the central host decoder | Propagate a typed host rejection while rendering the successful report locally | Keep endpoint failure rendering centralized and leave runtime-status report rendering independent |

Canic-owned test decoders and assertions remain verification consumers rather
than runtime policy or catalogue authorities. Their complete source update is
part of the atomic B3 contract cut; they do not add semantic rows to this
production-consumer manifest.

The combined production surface therefore contains twelve machine-decision
consumers and six transparent decode/render consumers. Each row is guarded as
structured evidence; this closes the production consumer side of the B1
manifest, but not the much larger exact producer-function mapping.

## Remaining B1 Work

The public boundary and maintained production-consumer surface are now bounded
and mechanically explicit. B1 still requires:

1. binding every provisional exact identity to its exhaustive current producer
   function or finite source-selected adapter set;
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
   [code-allocation ledger](../../../design/0.102-compact-diagnostic-codes/code-allocation-ledger.md).

The next pass starts with typed conversions and `with_diagnostic_context`
callers because they already expose the places where prose is substituting for
identity.
