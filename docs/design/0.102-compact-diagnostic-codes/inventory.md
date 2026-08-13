# Canic 0.102 Diagnostic Inventory

Date: 2026-08-13

## Purpose

This is the active B1 evidence ledger for the compact-diagnostic hard cut. It
records the current source and persistence surface before numeric allocation.
It is not the host catalogue and none of its provisional groupings allocate a
diagnostic code.

## Baseline

- Branch: `main`.
- Commit: `23c0328f78b215580d734ef01b52b35fa3e38ade`.
- Tag: `v0.101.53`.
- Source worktree: clean before 0.102 documentation work.
- Public shape: `Error { code: ErrorCode, message: String }`.
- Maintained public enum leaves: 20.
- Canonical declarations retaining the shape:
  `crates/canic-fleet-coordinator/fleet_coordinator.did` and
  `crates/canic-wasm-store/wasm_store.did`.

The 0.102 design's 25-leaf and 59-endpoint figures came from `v0.98.14`; they
must not be reused as current evidence.

## Source Census

The first conservative source census found:

| Surface | Current count | Interpretation |
| --- | ---: | --- |
| Production-file `InternalError` constructor references | 2,208 | 1,876 in `canic-control-plane`, 332 in `canic-core`; semantic leaves still require grouping |
| Typed `*Error` structs and enums in canister-reachable crates | 113 | Candidate typed mapping owners; not every type crosses the public boundary |
| Explicit `From<...>` conversions into `InternalError` or public `Error` | 39 | Twelve partially typed maps, 13 pass-through wrappers, 12 terminal string-flattening maps and two generic public projections |
| Explicit public-error constructions before inline test modules | 151 in 26 files | Reachable `Error::*` construction and explicit projection sites; convenience definitions and typed `InternalError` routes are additional authorities |
| Production `with_diagnostic_context` call sites | 35 in 15 files | The other two references are the method definition and an inline test; all production calls must disappear or acquire an explicit typed/observability owner in B4 |
| Maintained public enum leaves | 20 | Broad current classes and Wasm Store leaves, not the final 0.102 taxonomy |

The internal-error counts exclude files named `tests.rs` and files under
`tests/`, but Rust inline test modules remain in those conservative totals. The
151 explicit public constructions additionally stop before inline
`mod tests { ... }` bodies. External `mod tests;` declarations do not truncate
their owning production modules. Neither count is a Wasm-size claim.

The exact public-boundary ownership and code-dependent control-flow inventory
is recorded in [public-boundary.md](public-boundary.md). It is narrower than
the complete internal producer ledger but complete for the maintained current
wire boundary.

The complete structural classification of all 39 conversions and 35
production context-appending calls is recorded in
[conversion-context.md](conversion-context.md). It identifies where typed
variants are currently flattened, but does not yet allocate the semantic leaves
inside those types.

The recursive follow-through of the 12 terminal flattening owners is recorded
in [transitive-error-inventory.md](transitive-error-inventory.md). Its union of
54 Canic-owned typed owners and 514 declared variants is the reproducible first
perimeter, not a proposed code count. Expanding the authentication string stop
adds ten owners and 96 non-test structural variants; the conservative
64-owner/610-counted-variant perimeter still includes wrappers and nested
causes. It identifies the string buckets, stringified typed forwarding and
dependency adapters that must be corrected before allocation can be exhaustive.

The first semantic family is recorded in
[configuration-leaves.md](configuration-leaves.md). It proves the TOML/schema
validation path is native-only, bounds the runtime configuration perimeter at
86 producer-reachable exact candidates after its path-specific reachability
cut, and proposes three safe projections. Semantic grouping and observability
approval remain open; no number is allocated.

The second semantic family starts in
[auth-policy-leaves.md](auth-policy-leaves.md). Its base typed surface maps 48
exact authentication/policy candidates and four safe projections and excludes
one unproduced sharding reason.

[auth-string-frontier.md](auth-string-frontier.md) expands and reconciles that
untyped stop. It records ten additional typed owners, 43 direct prose
construction sites, 84 new exact candidates and two new safe projections after
reuse/sediment removal. It also proves that the durable chain-key signing state
currently marks terminal protected-policy failures retryable.

[ic-infrastructure-leaves.md](ic-infrastructure-leaves.md) maps 24 provisional
exact leaves across the owned IC adapters and the pinned call, rejection,
signing-cost and Candid dependency surfaces. It adds four safe projections and
keeps destination-invalid absence as typed evidence before projection.

[bounded-runtime-leaves.md](bounded-runtime-leaves.md) maps 37 current exact
topology, runtime-log, refill, Placement Index and Cashier candidates plus four
safe projections. One projection is shared with configuration. Because 0.102
precedes the 0.107 extraction, the four current Cashier leaves are allocated
now and retired without reuse by that later hard cut.

[runtime-ops-leaves.md](runtime-ops-leaves.md) maps configuration lookup,
protected deployment validation, runtime environment and request/RPC wrapper
ownership. It contributes 18 new exact candidates and two safe projections,
while deliberately reusing existing environment, access and compiled-
configuration meanings.

[fleet-activation-leaves.md](fleet-activation-leaves.md) maps all fresh root,
Store and non-root activation admission plus the protected activation record.
It contributes 30 exact candidates and one safe projection; more than thirty
record-validation prose sites reduce to one durable-state invariant because
their owner, fail-closed action and retry policy are identical.

[storage-registry-leaves.md](storage-registry-leaves.md) maps the ICP-refill,
Placement Index and feature-gated Sharding record owners to 18 exact candidates
and two safe projections. `StorageOpsError` remains transparent.

[fleet-control-plane-leaves.md](fleet-control-plane-leaves.md) maps the Fleet
Registry, Component provisioning plan, Fleet-service binding and shared receipt
hashing owners. It contributes 124 exact candidates and one safe projection,
preserving four aggregate typed causes instead of numbering their formatted
wrappers.

[intent-store-leaves.md](intent-store-leaves.md) maps all 51 live durable intent
variants plus one safe state projection. Request/state-machine conditions remain
actionable while primary/index/metadata contradictions fail closed behind the
masked public state code.

[memory-adapter-leaves.md](memory-adapter-leaves.md) pins the current adapter to
`ic-memory 0.12.3` and its lockfile checksum. It groups 131 known reachable
structural leaves into 54 Canic-owned semantics and adds 20 boundary-specific
unknown leaves for the reachable non-exhaustive enums: 74 exact candidates and
no broad projection.

### Semantic Ledger Coverage

| Family | Provisional exact candidates | Safe projections | Remaining qualification |
| --- | ---: | ---: | --- |
| Runtime configuration/topology | 86 | 3 | Same-semantics grouping and masked owner review |
| Component lifecycle policy | 34 | 4 | Grouping and sediment deletion review |
| Authentication and policy | 132 | 6 | Retry disposition and masked owner review |
| IC infrastructure | 24 | 4 | Calling-context projection review |
| Bounded runtime owners | 37 | 4, one reused | Cashier hard-cut order and masked owner review |
| Cost guard | 7 | 1 | Secondary rollback observability review |
| Access boundary | 20 | 2 new, 2 reused | Dependency-cause observability review |
| Runtime/config/environment/RPC ops | 18 | 2 new, existing projections reused | Typed-cause preservation review |
| Fleet activation | 30 | 1 new | Record-string hard cut and masking review |
| Bounded storage registries | 18 | 2 new | Durable-string and feature review |
| Fleet control-plane compilers | 124 | 1 new | Aggregate-cause and receipt observability review |
| Durable intent store | 51 | 1 new | State masking and interruption review |
| Pinned `ic-memory` adapter | 74 | 0 | Dependency-unknown and version-pin review |
| Top-level Component allocation persistence | 23 new, 3 reused | 1 new | 55 direct constructor sites classified |
| Direct-child reservation/install persistence | 17 new, 11 reused | 0 new | 83 direct constructor sites classified |
| Child commitment and activation persistence | 24 new, 9 reused | 0 new | 73 direct constructor sites classified |
| Component create/install/activation workflow | 45 new, 24 reused | 0 new | 86 direct constructor sites classified |
| Component draining/quiescence/recycling workflow | 32 new, 2 reused | 0 new | 51 direct constructor sites classified |
| Subtree-removal orchestration workflow | 5 new, 1 reused | 0 new | 26 direct constructor sites classified |
| Subtree stop/recycling/protected authority | 22 new, 1 reused | 0 new | 27 direct constructor sites classified; one transparent typed-cause carrier |
| Root draining/final inventory/logical removal persistence | 50 new, 1 reused | 0 new | 73 direct constructor sites classified |
| Store reclamation/publication-binding finalization persistence | 28 new, 4 reused | 0 new | 45 direct constructor sites classified |
| Store deletion/root-deletion preparation persistence | 40 new, 4 reused | 0 new | 61 direct constructor sites classified |
| Final/initial root-inventory persistence | 24 new, 3 reused | 0 new | 35 direct constructor sites classified |
| Root activation/initial-inventory convergence workflow | 9 new, 6 reused | 0 new | 15 direct constructor sites classified |
| Root draining/final-inventory/logical-removal workflow | 22 new, 8 reused | 0 new | 36 direct constructor sites classified; three transparent typed-cause rows |
| Store reclamation/binding-finalization workflow | 0 new, 6 reused | 0 new | 7 direct constructor sites classified |
| Store deletion/root-deletion readiness workflow | 7 new, 9 reused | 0 new | 19 direct constructor sites classified; one transparent two-site transport row |
| Sibling Store adoption workflow | 7 new, 0 reused | 0 new | 7 direct constructor sites classified |
| Component Directory paging/protected member | 15 new, 1 reused | 0 new | 20 direct constructor sites classified; two transparent topology causes |
| Component Directory convergence/runtime status | 30 new, 2 reused | 0 new | 51 direct constructor sites classified; twelve transparent adapter-sediment sites |
| Component peer/protected-allocation workflow | 12 new, 6 reused | 0 new | 23 direct constructor sites classified; two transparent topology causes |
| Component Registry preparation/allocation/create-install workflow | 28 new, 15 reused | 0 new | 55 direct constructor sites classified; eight transparent typed-cause/adapter sites |
| **Current counted subtotal** | **1,095** | **31 distinct** | 1,126 identities; not a numeric allocation |

[ledger-reconciliation.md](ledger-reconciliation.md) reconciles the qualified family
arithmetic and every cross-family reuse. The broad source-document token census
contains 689 unique uppercase tokens; four are explicitly documented notation
or forbidden/unreachable examples, leaving exactly 685 collision-free proposed
identities: 655 exact meanings and 30 additional safe projections.

The first twenty-one direct-constructor passes add 440 exact meanings and
`COMPONENT_REGISTRY_STATE_INVALID`, bringing the qualified set to 1,126 distinct
identities. Existing exact reuses are not counted again.

[projection-ledger.md](projection-ledger.md) aggregates those 31 additional
identities plus five exact leaves reused as projection targets. It names the
required observation class for every masked family and records three remaining
approval gaps: numeric conversion of the guarded recent-failure ring and
call-site-specific IC effect-journal wiring. Cashier uses the guarded numeric
runtime observation until its 0.107 retirement.

[ic-observability-owners.md](ic-observability-owners.md) resolves the IC gap to
17 current call families and their operation-specific recovery/status
authorities, including the missing narrow Store-publication attempt owner. It
requires durable operation records for mutating effects and the guarded runtime
observation only for read-only or bootstrap calls; it rejects a second generic
IC-effect journal.

This symbolic reconciliation does not allocate numbers. The total may still
decrease if the final action/retry review proves two differently named meanings
identical, and every masked exact leaf still requires an approved numeric
observability owner in the complete allocation table.

[direct-constructor-frontier.md](direct-constructor-frontier.md) proves why 685
was not a whole-program total: 2,208 baseline production `InternalError::*`
references require site-level disposition across 101 files. The two Component
Registry modules alone contain 1,154 references. Each site must reuse a
qualified meaning, add a justified meaning or be classified as a transparent
wrapper/native-only/sediment path before numeric allocation.

[component-registry-constructor-leaves.md](component-registry-constructor-leaves.md)
and
[component-registry-workflow-constructor-leaves.md](component-registry-workflow-constructor-leaves.md)
and
[fleet-subnet-root-workflow-constructor-leaves.md](fleet-subnet-root-workflow-constructor-leaves.md)
now close 848 of those references, leaving 1,360 site-level dispositions open.
The Component Registry workflow file is fully classified.

[code-allocation-ledger.md](code-allocation-ledger.md) freezes the permanent
repository-only current/retired allocation contract and the contract for a
generated language-neutral current registry. Because no numeric allocation has
been approved, its current and retired sets are both empty; the 1,126 qualified
symbolic identities above are not ledger rows.

### Current Public Leaves

The exact current enum is:

1. `AuthMaterialStale`
2. `AuthProofExpired`
3. `AuthProofPending`
4. `AuthTokenExpired`
5. `Conflict`
6. `Forbidden`
7. `Internal`
8. `InternalRpcMalformed`
9. `InvalidInput`
10. `InvariantViolation`
11. `NotFound`
12. `OperationIdRequired`
13. `ResourceExhausted`
14. `RootDataCertificateUnavailable`
15. `Unauthorized`
16. `Unavailable`
17. `WasmStoreCapacityExceeded`
18. `WasmStoreChunkMissing`
19. `WasmStoreHashMismatch`
20. `WasmStoreManifestMissing`

Current production-file references to those names are concentrated in 32
files. They include constructors, comparisons, retry classification and metric
projection, so raw reference frequency is not producer cardinality.

## Public Construction Authorities

The maintained public error can currently be produced through five distinct
routes:

1. `canic-core/src/dto/error.rs` owns 14 convenience constructors plus direct
   `new`, the `AccessError` conversion and prose-rich `Display`.
2. `canic-core/src/api/error.rs` converts every unprojected `InternalError`
   from its broad class/origin pair and clones any embedded public error.
3. Runtime authentication and RPC capability paths construct public errors
   directly before an `InternalError` exists.
4. Control-plane Component, peer, publication and Wasm Store paths embed exact
   public errors in `InternalError::public` or return them directly.
5. Generated endpoint macros construct unauthorized and invalid-input errors
   at the endpoint boundary.

The numeric allocation must therefore be based on the concrete typed or
workflow decision at each route, not on the current convenience-constructor
name. Replacing each broad constructor one-for-one would preserve the current
loss of meaning and would not satisfy 0.102.

### Broad Fallback Projection

`canic-core/src/api/error.rs` currently maps unprojected errors as follows:

| Internal class/origin | Current public result |
| --- | --- |
| `Access/*` | `Unauthorized` |
| `Domain/Config` | `InvalidInput` |
| `Domain/other` | `Conflict` |
| `Invariant/*` | `InvariantViolation` |
| `Infra/*`, `Ops/*`, `Workflow/*` | `Internal` |

This fallback is a migration inventory aid only. B4 must replace it with exact
typed mappings; it cannot become the compact-code allocation strategy.

### Host Consumers

The current native consumers are:

- `canic-host/src/icp/response/mod.rs`: central typed Candid result decoder;
- `canic-host/src/canister_protocol/mod.rs`: exact rejection matching;
- `canic-host/src/replica_query/mod.rs`: replica-query result decoding;
- `canic-host/src/fleet_subnet_root_deletion/mod.rs`: unavailable-state
  reconciliation;
- `canic-host/src/install_root/fleet_component_provisioning_install/mod.rs`:
  install transaction rejection handling;
- `canic-cli/src/cycles/mod.rs` and
  `canic-cli/src/cycles/convert/response.rs`: direct code-plus-message cycle
  rendering; and
- `canic-cli/src/inspect/mod.rs`: normal decoder propagation through the host
  response error.

`canic-testing-internal`, Canic-owned PocketIC tests and facade tests also
decode or match the current enum. They are propagation owners, not catalogue
authorities.

## Dynamic Public Context Classification

The current public construction and conversion inventories locate where
messages cross the boundary, but they do not yet classify each dynamic value
interpolated into those messages. Operation IDs, generations, limits, retry
times, principals, Canister identities and conflicting authority values can be
remediation-significant even when their surrounding prose is disposable.

[dynamic-public-context.md](dynamic-public-context.md) defines the required
row shape and four classifications: caller-derivable, sensitive/operator-only,
authoritatively typed, and caller-required but unowned. Its current-source
census is not yet complete. Its first twelve bounded slices classify 117 dynamic
values from the Canic memory-ledger facade, Wasm Store GC, Store
manifest/capacity conversion, explicit Component Registry denials, typed Store
publication causes, delegated-session bootstrap and Store publication
binding/inventory plus Store GC fence, reclamation, binding finalization and
deletion plus the two Store-publication management transports: 53 are
caller-derivable, thirteen are sensitive operator-only, twenty-eight already
have typed owners and twenty-three are caller-required but unowned. The unowned
values
require request-scoped Store capacity/release inspection, guarded
delegated-session capacity status, exact closed-discriminator diagnostics or
root-proxied live GC inspection, operation-scoped Store deletion progress or a
narrow operation-scoped Store-publication attempt status before their messages
are removed. Every category-4 data value must acquire an
approved endpoint-specific typed owner before the public hard cut; a closed
semantic discriminator may instead become an exact registered diagnostic. A
generic text detail or global last-error slot is forbidden.

This is independent of the durable ledger below. Public retrieval ownership
and interruption-recovery ownership must both be proved when the same value
participates in both concerns.

## Durable And Diagnostic Text Classification

The current-source audit corrects one assumption in the design: runtime
bootstrap `last_error` is process-local thread state, not stable memory. The
actual durable failure-text owners are below.

| Owner and field | Persistence/limit | Current decision authority | B1 classification | Required B5 treatment |
| --- | --- | --- | --- | --- |
| ICP refill `IcpRefillRecord.error_message` | Stable, 4,096-byte record; text truncated to 512 characters | `IcpRefillRecordErrorCode` and status, never message parsing | Owned operational text | Retain only if the dynamic ledger/CMC context is still operationally required; keep bounded and prove decisions remain code-only |
| Cycle top-up `CycleTopupEventRecord.error` | Stable, 512-byte record | Typed event status, never message parsing | Owned operational text | Preserve as bounded event context or replace with typed detail; no diagnostic-code recovery from prose |
| Chain-key batch `failure` and issuer `last_failure` | Stable inside unbounded `AuthStateRecord` | Field presence currently participates in readiness/terminal checks | Recovery-significant state | Retain in 0.102 unless a direct typed replacement preserves the exact current state machine with bounded proof; move any broader recovery redesign to a later design line |
| Prepaid-pool asset `Failed(String)` and recycle-reset `Failed(String)` | Stable in unbounded `CanisterPoolAssetRecord` | Typed enum variant controls lifecycle; text is projected to status | Owned operational text | Keep the failure state typed and make retained context explicitly bounded; never classify by text |

Related non-stable or boundary-only text is classified separately:

| Owner | B1 classification | Reason |
| --- | --- | --- |
| Runtime bootstrap `last_error` | Advisory transient context | Process-local, cleared on phase changes and rebuilt by current lifecycle execution |
| Runtime recent-failure summaries/details | Owned operational text | Bounded runtime diagnostic/log projection, not stable recovery authority |
| Pool response reasons | Owned operational text | Projection of typed stable status; not an independent state owner |
| Blob billing `skipped_reason` | Owned operational text pending 0.107 extraction | Application-service response context; 0.102 must not pre-empt the later blob hard cut |
| Scaling-plan `reason` | Owned operational text | Explanatory projection beside typed `ScalingPlanReason`; policy does not parse it |
| Configuration and state-contract validation details | Owned operational text | Host/operator explanation; typed owner determines the failure branch |

The B5 implementation must re-run this ledger immediately before schema work.
No pre-0.102 decoder, migration, fallback parser or text-to-code conversion is
permitted.

B5 is not a general recovery-state redesign. It removes redundant prose,
deterministically clears proven advisory state and preserves bounded owned
operational text. Recovery-significant text is changed only when a direct typed
replacement preserves current behavior without a new state machine; otherwise
the text remains and the redesign is recorded for a later design line.

## Masked Diagnostics And Observability

The current `InternalError` model has no distinct internal numeric identity.
An unprojected invariant/infra/ops/workflow failure is flattened to broad public
`InvariantViolation` or `Internal`, while its only detailed identity is prose.
Consequently no masked internal code can yet have the numeric observability
owner required by 0.102.

The allocation review must assign one of these current owners to every masked
leaf before B2/B4 implementation:

- structured recent-failure entry returned by the guarded status surface;
- existing typed operation/status receipt on the same operation, or correlated
  through its existing retrievable operation ID;
- existing metrics reason enum where it identifies the same leaf; or
- a sufficiently specific safe public code when no internal numeric owner
  exists.

A new generic stable diagnostic log is not implied and must not be invented to
avoid choosing the real owner. An uncorrelated log entry or process-global
“last failure” does not satisfy the masked-code requirement.

## Wasm Baseline

The authoritative method is `CANIC-WASM-001/v3`. Before v3 was introduced, a
fresh v2 run was attempted
from a clean detached linked worktree at the exact baseline commit with:

```text
WASM_AUDIT_PRODUCT_ROOT=/tmp/canic-0102-baseline \
  WASM_AUDIT_DATE=2026-08-12 \
  bash scripts/ci/wasm-audit-report.sh
```

The first attempt stopped before build because the installed tool is
`ic-wasm 0.11.1` while `tool-versions.env` requires `0.11.0`. The checksum-
pinned installer then placed `0.11.0` under `/tmp` without replacing the user's
tool. With the exact pinned tool, the method stopped at its frozen-roster gate:
it expects six roles, while current source reports
`app,test,user_hub,scale_hub,user_shard,scale_replica,root`.

The frozen gate correctly detected product-scope drift. V2 also predates the
0.102 requirement to measure the separately built Fleet Coordinator and Wasm
Store. Historical v2 evidence remains valid but non-comparable. The current
baseline uses the incremented, refingerprinted v3 method with the complete
roster.

### Qualified Development Baseline

To retain useful B1 evidence without misrepresenting the blocked method, four
roles were built from the immutable tag through the same canonical host
builder, offline and with one isolated Cargo target. This is a development
baseline, not a `CANIC-WASM-001` result:

| Role | Class | Raw Wasm bytes | Builder gzip bytes | Functions | Data sections/bytes | Exports |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `app` | representative Component | 3,006,400 | 980,826 | 5,449 | 3 / 236,516 | 26 |
| `root` | Fleet Subnet Root infrastructure | 7,539,746 | 2,430,575 | 10,977 | 3 / 446,252 | 126 |
| `fleet_coordinator` | Fleet Coordinator infrastructure | 3,439,803 | 1,075,598 | 5,136 | 3 / 242,940 | 28 |
| `wasm_store` | Wasm Store infrastructure | 2,597,251 | 855,665 | 5,046 | 3 / 216,224 | 31 |

`ic-wasm 0.11.0 info` accepted each release artifact, and bounded `twiggy top`
plus retained-top inspection completed. The builds left the immutable product
worktree tracked-clean with only permitted `.icp/` output. These values are the
pre-cut reference for B1/B2 development decisions; the retained full audit
below owns release evidence and comparisons.

### Retained V3 Baseline

The fresh full
[CANIC-WASM-001/v3 report](../../audits/reports/2026-08/2026-08-12/wasm-footprint-v3.md)
passes with risk `5/10`. It builds and measures release plus debug artifacts for
all six configured Components and the three infrastructure roles at immutable
tag `v0.101.53`. The release values for the four representative roles exactly
match the development baseline apart from ordinary gzip header variation.

V3 has no compatible predecessor by construction. All later 0.102 material
slice and closeout comparisons must use this v3 method, exact roster and
comparability rules.

## Reproducible Discovery Commands

The source census is reproduced with bounded `rg` scans over `canic-core`,
`canic-control-plane`, `canic-wasm-store` and `canic`, excluding named test
files and test directories. The key patterns are:

```text
InternalError::(auth_material_stale|auth_proof_expired|auth_proof_pending|
auth_token_expired|conflict|domain|forbidden|infra|invalid_input|invariant|
operation_id_required|ops|public|resource_exhausted|
root_data_certificate_unavailable|unavailable|workflow)

(enum|struct) <name ending in Error>

impl From<...> for Error|InternalError

with_diagnostic_context
```

The next inventory step is semantic: split reachable production sites into
leaf families with one action, retry policy, exposure decision and narrow
owner, then record the exact source sites under those families. Raw counts do
not authorize a number.
