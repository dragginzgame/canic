# Canic 0.102 Public Projection Ledger

Date: 2026-08-13

## Status

This B1 ledger aggregates every current non-self projection proposed by the
semantic family audits. It allocates no code. A projection is admissible only
when its exact internal identity has the numeric observation named here before
the safe public identity is returned.

The current source has one general guarded owner:
`CanicRuntimeStatus.recent_failures`, backed by the heap-only bounded
`RecentFailureOps` ring. Its current `code: String` is not yet a compliant
numeric owner; B2/B4 must hard-cut that field and every writer to
`DiagnosticCode`. Pre-readiness paths additionally need the existing lifecycle
log/trap path to emit the exact numeric identity. A durable workflow record is
preferred when the diagnostic affects interruption recovery.

For every masked result, the exact code must live on the same retrievable
operation/status record or be correlated through that record's existing
operation ID. A global or uncorrelated log is not an observability owner. The
guarded recent-failure ring is admissible only when the maintained guarded
status surface returns that bounded observation and no durable operation
authority exists.

## Additional Safe Projection Identities

These are the 31 projection-only identities in the currently reconciled
2,875-identity qualified set. The direct-constructor, RPC workflow-error,
Template-manifest and complete publication-workflow audits add no further
projection. Dynamic cause/formatter ownership is closed; final allocation
review may merge only explicitly proved same-semantics rows and may not invent
a generic projection.

| Safe public projection | Exact mapped surface | Proposed host class | Required exact numeric observation |
| --- | --- | --- | --- |
| `RUNTIME_CONFIGURATION_INVALID` | 84 compiled-configuration leaves plus three runtime lookup leaves; typed configuration causes retain their own identity | `Invariant` | lifecycle numeric log before readiness; guarded recent-failure ring afterward |
| `RUNTIME_CONFIGURATION_CONFLICT` | contradictory second runtime initialization | `Conflict` | lifecycle numeric log and guarded recent-failure ring |
| `RUNTIME_CONFIGURATION_UNAVAILABLE` | runtime configuration absent and refill build-network identity absent | `Unavailable` | guarded recent-failure ring; lifecycle numeric log when readiness is unavailable |
| `COMPONENT_ALLOCATION_AUTHORITY_INVALID` | four protected root-topology/admission contradictions | `Invariant` | guarded recent-failure ring on the root |
| `PEER_COMPONENT_REQUESTER_UNAUTHORIZED` | invalid protected peer requester/root binding | `Forbidden` | guarded recent-failure ring on the target root |
| `COMPONENT_CHILD_AUTHORITY_INVALID` | protected Component/parent/Spec/limit, Store/module source, creation/install authority and persisted allocation-record contradictions | `Invariant` | guarded recent-failure ring on the root |
| `COMPONENT_CHILD_PARENT_UNAUTHORIZED` | protected parent belongs to another Component tree or reservation | `Forbidden` | guarded recent-failure ring on the root |
| `AUTH_ROOT_ISSUER_BINDING_INVALID` | Fleet mismatch and immutable issuer-policy identity mismatch | `Forbidden` | guarded auth recent-failure entry |
| `AUTH_ROOT_ISSUER_POLICY_INVALID` | six malformed protected issuer-policy decisions | `Invariant` | guarded auth recent-failure entry |
| `RUNTIME_ENVIRONMENT_INVALID` | incomplete protected environment and conflicting root authority | `Invariant` | lifecycle numeric log before readiness; guarded recent-failure ring afterward |
| `AUTH_PROOF_INVALID` | four base proof failures plus 21 reconciled certificate/chain-key proof failures | `Unauthorized` | guarded auth recent-failure entry; durable batch code when the proof belongs to a signing batch |
| `AUTH_CHAIN_KEY_SIGNING_FAILED` | three protected signer header/key/signature contradictions | `Invariant` | durable chain-key signing batch diagnostic and terminal disposition; guarded status projection |
| `AUTH_CHAIN_KEY_MATERIAL_STALE` | six protected policy/epoch/key-floor failures | `Unavailable` | durable signing batch when applicable; otherwise guarded auth recent-failure entry |
| `RUNTIME_RELEASE_BUILD_INVALID` | three embedded release-build identity failures | `Invariant` | lifecycle numeric log before readiness; guarded recent-failure ring afterward |
| `IC_PLATFORM_RESPONSE_INVALID` | four unrepresentable Ledger/management response values | `Invariant` | owning workflow journal/receipt when one exists, otherwise guarded recent-failure ring |
| `IC_PLATFORM_PROTOCOL_INVALID` | request Candid encode and response Candid decode failure | `Invariant` | owning workflow journal/receipt when one exists, otherwise guarded recent-failure ring |
| `IC_PLATFORM_EFFECT_FAILED` | ten management/NNS/call failures that cannot safely expose target or raw reject detail | `Unavailable` | exact owning effect journal before/after external effects; guarded recent-failure ring for read-only calls |
| `RUNTIME_LOG_STATE_INVALID` | four runtime-log counter/sequence/time contradictions | `Invariant` | guarded recent-failure ring; never the contradicted runtime log itself |
| `ICP_REFILL_RESPONSE_INVALID` | two refill value/decimal response contradictions | `Invariant` | durable refill operation diagnostic plus guarded recent-failure ring |
| `BLOB_CASHIER_RESPONSE_INVALID` | four current Cashier decode contradictions plus `TopUpWithoutCycles` after Canic attached admitted cycles | `Invariant` | guarded recent-failure ring before projection; allocate in 0.102 and retire without reuse in the 0.108 hard cut |
| `COST_GUARD_CONFIGURATION_INVALID` | five protected cost-manifest/accounting failures | `Invariant` | owning intent/cleanup diagnostic for durable work; guarded recent-failure ring otherwise |
| `ACCESS_DEPENDENCY_UNAVAILABLE` | typed environment, verifier, Registry or runtime dependency cause | `Unavailable` | preserve and record the nested exact code in the guarded access recent-failure entry |
| `ACCESS_CONFIGURATION_INVALID` | malformed static service guard or empty access expression | `Invariant` | lifecycle numeric log and guarded access recent-failure entry |
| `COMPONENT_DEPLOYMENT_CONTEXT_INVALID` | ten protected deployment-context mismatches | `Invariant` | guarded recent-failure ring on the validating runtime |
| `RPC_RESPONSE_INVALID` | closed internal request/response variant mismatch | `Invariant` | guarded recent-failure ring on the receiving runtime |
| `FLEET_ACTIVATION_ADMISSION_INVALID` | 19 fresh root/Store/topology admission failures | `InvalidInput` | lifecycle numeric log before the runtime becomes ready |
| `ICP_REFILL_STATE_INVALID` | four durable refill-record/index semantic contradictions | `Invariant` | guarded recent-failure ring; never write diagnostic evidence into contradicted refill state |
| `SHARDING_STATE_INVALID` | assignment-count underflow and overflow | `Invariant` | guarded recent-failure ring; never the contradicted Sharding Registry |
| `COMPONENT_PROVISIONING_RECEIPT_INVALID` | receipt encode and byte-count failure | `Invariant` | owning provisioning/publication journal plus guarded recent-failure ring |
| `INTENT_STATE_INVALID` | 39 primary/index/metadata/schema contradictions | `Invariant` | guarded recent-failure ring; never the contradicted intent store or index |
| `COMPONENT_REGISTRY_STATE_INVALID` | initial-inventory, byte-ledger, physical-count, install-intent, precharge, receipt and partition/index contradictions | `Invariant` | guarded recent-failure ring; never the contradicted Component Registry record |

The classes above are proposed host-catalogue metadata for the safe public
identity. They do not overwrite the independently catalogued class of a masked
exact code and never enter release Wasm.

## Exact Identities Reused As Projections

Five exact candidates are also safe projection targets and therefore are not
part of the 31 additional identities:

| Exact/public identity | Masked exact input | Required exact numeric observation |
| --- | --- | --- |
| `COMPONENT_ALLOCATION_CAPACITY_EXHAUSTED` | Component allocation count overflow | guarded recent-failure ring |
| `COMPONENT_SPEC_ALLOCATION_CAPACITY_EXHAUSTED` | per-Spec allocation count overflow | guarded recent-failure ring |
| `PEER_COMPONENT_CAPACITY_EXHAUSTED` | peer-allocation count overflow | guarded recent-failure ring |
| `COMPONENT_CHILD_PARENT_ROLE_CAPACITY_EXHAUSTED` | per-parent role count overflow | guarded recent-failure ring |
| `FLEET_ACTIVATION_STATE_INVALID` | activation-record encode failure, invalid timestamp and evidence-hash failure | lifecycle numeric log or guarded recent-failure ring, chosen before returning the projection |

The public capacity identities tell the caller what resource is unavailable,
while the internal overflow identities still prove broken accounting. No
caller may interpret a capacity projection as proof that the corresponding
counter is valid.

## Approval Gaps

The projection names and mappings are now finite, but three observability
decisions remain implementation gates:

1. replace the guarded recent-failure ring's string code with a numeric code,
   expose it through the guarded status owner and prohibit summary text from
   carrying classification;
2. approve and instrument the call-site owner map in
   [ic-observability-owners.md](ic-observability-owners.md), without adding a
   competing generic effect journal or losing retrievable operation-ID
   correlation; and
3. preserve all 27 current blob identities through 0.102 allocation and retire
   their numbers without reuse when 0.108 removes their producers.

These are not reasons to broaden the public error. If an individual path has no
approved numeric owner, its exact code must become safely public or the path
must be hard-cut before allocation.
