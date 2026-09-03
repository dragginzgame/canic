# 0.110 B1 Generated Surface Inventory

Date: 2026-09-03
State: source trace complete; optimized-artifact proof open
Design owner: [0.110 Fleet runtime contraction](../../../design/0.110-fleet-runtime-contraction/0.110-design.md)
Measurement baseline: immutable `v0.110.5` at
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`

## Scope And Evidence Boundary

This ledger inventories Canic-owned build, lifecycle, admission, endpoint,
status, Candid and metrics generation for the eleven canonical roles measured
by `CANIC-WASM-001/v6`. It records both the immutable release baseline and the
explicit current working-tree overlay.

This is source-reachability evidence, not a claim that Binaryen removed every
unselected body. B1 still requires controlled ablations and optimized-artifact
evidence before any source contraction receives byte or function credit. The
canonical App fixtures also define their own test endpoints through
`canic_query` and `canic_update`; those destination expansions contribute to
the measured Wasm but are not Canic's role-selected default protocol.

The valid
[v6 report](../../reports/2026-09/2026-09-03/wasm-footprint-v6.md) measures only
the immutable tag. It does not measure the working-tree overlay recorded below.

## Canonical Role Selection

`apps/test/canic.toml` selects the nine configured roles. Fleet Coordinator
and Wasm Store are built-in infrastructure roles. Capabilities are the exact
result of `role_contract::derive_role_capabilities` or
`built_in_role_capabilities`; metric tiers are selected independently from the
resolved role profile.

| Role | Exact capabilities | Metrics profile and tiers | Start bundle |
| --- | --- | --- | --- |
| `app` | Runtime, AutomaticTopup, Icrc21 | leaf: core, runtime, security | managed |
| `index_hub` | Runtime, ChildProvisioning, Index | hub: core, placement, runtime, security | managed |
| `test` | Runtime, AutomaticTopup, FleetAdmissionProjection, Icrc21 | leaf: core, runtime, security | managed |
| `user_hub` | Runtime, AutomaticTopup, ChildProvisioning, Icrc21, Sharding | hub: core, placement, runtime, security | managed |
| `scale_hub` | Runtime, AutomaticTopup, ChildProvisioning, Icrc21, Scaling | hub: core, placement, runtime, security | managed |
| `index_child` | Runtime | leaf: core, runtime, security | managed |
| `user_shard` | Runtime, AutomaticTopup, DelegatedTokenIssuer, DelegatedTokenVerifier, Icrc21, RoleAttestationVerifier | leaf: core, runtime, security | managed |
| `scale_replica` | Runtime, AutomaticTopup, Icrc21 | leaf: core, runtime, security | managed |
| `root` | Runtime, Root, RootControlPlane, RoleAttestationSigner | root: core, placement, platform, runtime, security, storage | Root |
| `fleet_coordinator` | FleetCoordinator | no generic metrics endpoint | Coordinator |
| `wasm_store` | ChildProvisioning, Runtime, WasmStore | storage: core, runtime, storage | Store |

The Root signer capability is derived because a configured child consumes the
role-attestation cache. The Coordinator deliberately has neither the ordinary
config-driven build script nor the managed lifecycle/status bundle: its Cargo
feature and dedicated start macro select the infrastructure surface. The Store
uses the ordinary build macro's built-in-role exception and its own start
bundle.

## Expansion Spine

### Build-Time Generation

`canic::build!` expands through `__canic_build_internal!` for every configured
role and the Store. The expansion:

- validates package metadata and the canonical build marker;
- parses and validates the selected configuration;
- emits closed capability and metrics `cfg` values;
- writes and exports the compiled role-runtime authority;
- writes the complete compiled config and source only for Root;
- embeds the release-build and protocol-profile identities; and
- enables Candid export only for the dedicated declaration pass.

Fleet Coordinator does not invoke this macro. Its dedicated crate enables only
the `fleet-coordinator-canister` feature and invokes the dedicated runtime
start macro.

### Runtime Entry Generation

| Entry macro | Lifecycle expansion | Inspect-message expansion | Endpoint expansion | Standards |
| --- | --- | --- | --- | --- |
| `start!` on managed roles | `__canic_start_nonroot_lifecycle_core!` | `managed` variant | managed command + managed status | ICRC-10; ICRC-21 only when selected |
| `start!` on Root | `__canic_root_lifecycle_core!` | `root` variant | Root command + Root status bundle | ICRC-10 |
| `start_wasm_store!` | `__canic_start_wasm_store_lifecycle_core!` | `wasm_store` variant | Store runtime bundle | ICRC-10 |
| `start_fleet_coordinator!` | direct synchronous Coordinator init | `fleet_coordinator` variant in the working overlay | Coordinator command + status | none |

Every canonical crate closes with `finish!`, which installs the required marker
and the cfg-selected Candid export module. `start_local!` and its local
lifecycle, generic inspect and local-status expansions are maintained public
surface but are not part of the canonical eleven-role roster.

Each emitted query or update then passes through the `canic_query` or
`canic_update` procedural attribute. That layer emits the public wrapper and
`__canic_impl_*` body, constructs the endpoint-call identity, runs preflight
and access stages, brackets dispatch entry/exit, and preserves Candid metadata.
Payload-limited updates additionally emit a limit-registration constructor and
raw update adapter. These adapters are explicit B1 ablation targets; their
source shape does not establish their optimized cost.

## Managed Protocol Projection

All eight configured non-Root roles receive exactly two Canic-owned protocol
methods: `canic_command` and `canic_status`. They also receive
`icrc10_supported_standards`; `app`, `test`, `user_hub`, `scale_hub`,
`user_shard` and `scale_replica` additionally receive
`icrc21_canister_call_consent_message`.

The generated command union has two common variants, `ConfigureRuntime` and
the controller-only `Observe`, plus only these capability-selected variants:

| Capability | Added command variants | Canonical roles |
| --- | --- | --- |
| FleetAdmissionProjection | `ActivateFleetAdmission`, `OpenFleetAdmission`, `PrepareFleetAdmission` | `test` |
| LocalApplicationAuthorization | `ApplicationSession` | none |
| DelegatedTokenIssuer | `InstallDelegationProof`, `PrepareDelegatedToken` | `user_shard` |
| ChildProvisioning | `RespondCapability` | `index_hub`, `user_hub`, `scale_hub` |

The generated status union has the common variants `Binding`, `CycleBalance`,
`CycleHistory`, `Health`, `Logs`, `Metrics`, `Operation`, `Overview`,
`Readiness` and `Runtime`, plus:

`CycleBalance`, `CycleHistory`, `CycleTopups` and `Metrics` require controller
authority. Human Fleet operators reach managed Component observations through
the Root's controller-only `ObserveCanister` command; they do not become
Component lifecycle controllers.

| Capability | Added status variants | Canonical roles in the working overlay |
| --- | --- | --- |
| FleetAdmissionProjection | `Admission` | `test` |
| DelegatedTokenIssuer | `ActiveDelegationProof`, `DelegatedToken` | `user_shard` |
| LocalApplicationAuthorization | `ApplicationSession`, `ApplicationSessionAudit` | none |
| AutomaticTopup | `CycleTopups` | `app`, `test`, `user_hub`, `scale_hub`, `user_shard`, `scale_replica` |
| ChildProvisioning | `Children` | `index_hub`, `user_hub`, `scale_hub` |

Metrics dispatch is separately cfg-pruned to the tier matrix above. Index,
Sharding, Scaling, delegated-token verification and role-attestation
verification select runtime providers and state but do not add another default
top-level Canic method. Their absence or retention beneath the two union
methods remains an optimized-artifact question.

## Infrastructure Protocol Projection

### Fleet Subnet Root

The Root bundle emits `canic_root_command`, `canic_status` and
`icrc10_supported_standards`. The command union contains:

`AcceptFunding`, `ActivateFleetAdmission`, `ActivateFundingPolicyRotation`,
`AdoptStore`, `BootstrapStore`, `GetOrCreateDelegationProof`,
`HandoffPoolCanister`, `ImportPoolCanister`, `InspectCanister`, `MaintainPool`,
`ObserveCanister`, `OpenFleetAdmission`, `PrepareAuthoritySnapshot`,
`PrepareComponentRegistry`,
`PrepareFleetActivation`, `PrepareFleetAdmission`,
`PrepareFundingPolicyRotation`, `PrepareRoleAttestation`, `PreviewCycleRefill`,
`ProvisionChild`, `ProvisionComponent`, `ProvisionComponents`, `ProvisionPeer`,
`PublishReleaseSet`, `RefillCycles`, `RemoveComponent`, `RemoveRoot`,
`RemoveSubtree`, `RespondCapability`, `ResumeAuthoritySnapshot`,
`ResumeFleetActivation`, `RetryPoolRefill`, `RetryPoolReset`,
`SetCyclesFunding`, `SetFleetStatus`, `SynchronizeComponentDirectories`,
`SynchronizeRegistry`, `UpsertIssuerPolicy` and `UpsertIssuerRenewalTemplate`.

The current status union contains `Admission`, `AuthorityRestore`, `Children`,
`ComponentDirectoryHead`, `ComponentDirectoryPage`, `ComponentRegistry`,
`ComponentRegistryActivePartition`, `ComponentRegistryPartition`, `Config`,
`CycleBalance`, `CycleHistory`, `FleetAuthority`, `FleetState`, `Funding`,
`Health`, `Inventory`, `IssuerRenewal`, `Logs`, `Metrics`, `Operation`,
`Overview`, `Pool`, `Readiness`, `RoleAttestation`, `Runtime` and
`StoreOverview`. `RoleAttestation` is selected by the derived signer
capability. `ComponentRegistryActivePartition` belongs only to the working
overlay, as recorded below. `CycleBalance`, `CycleHistory` and `Metrics`
require controller authority.

### Fleet Coordinator

The dedicated bundle emits only `canic_coordinator_command` and `canic_status`.
Its command union contains `AcknowledgeRootSnapshot`, `ActivateRegistry`,
`ApplyFundingPolicyRotation`, `BeginFundingPolicyRotation`,
`CompleteRootDeletion`, `JoinRoot`, `MutateAdmission`,
`PrepareAuthoritySnapshot`, `PrepareRootDeletionExecution`,
`ProvisionComponents`, `RemoveRoot`, `RequestRootFunding`,
`ResumeAuthoritySnapshot`, `SetRootFunding` and
`StageFundingPolicyRotationRoot`. Its status union contains `Admission`,
`AuthorityRestore`, `Funding`, `Operation`, `Overview`, `Registry`,
`RegistryManifest`, `RegistryVersion` and `RootAcknowledgements`.

### Wasm Store

The Store bundle emits `canic_command`, `canic_status`,
`canic_wasm_store_chunk`, `canic_wasm_store_publish_chunk` and
`icrc10_supported_standards`. Its command union contains `ActivateFleet`,
`InspectTemplate`, `PrepareChunkSet`, `PrepareFleetCredential`,
`ReclaimDeletionCycles`, `RespondCapability`, `RunGc`, `StageManifest`,
`SynchronizeState` and `SynchronizeTopology`. Its status union contains
`Authority`, `Catalog`, `CycleBalance`, `CycleHistory`, `Operation`,
`Overview`, `Storage` and `Template`. `CycleBalance` and `CycleHistory`
require controller authority.

Blob-storage and blob-billing endpoint emitters are explicit public opt-ins.
No canonical role in this roster invokes them. The invocation-only
`application_scope!`, `log!` and `perf!` helpers likewise do not implicitly
add a protocol or lifecycle surface.

## Immutable Baseline Versus Working Overlay

The working tree has five generated-surface differences from `v0.110.5`:

| Surface | Immutable `v0.110.5` | Current working overlay | Measurement consequence |
| --- | --- | --- | --- |
| managed `Children` status | gated by Sharding, so only `user_hub` receives it | gated by ChildProvisioning, so `index_hub`, `user_hub` and `scale_hub` receive it | index and scaling Hub Candid/runtime reachability expands |
| Root active Registry partition | absent | one new `ComponentRegistryActivePartition` request, response and dispatch arm | Root Candid/runtime reachability expands |
| Coordinator inspect hook | generic registered-limit inspection | existing variant-aware Coordinator decoder is connected by `start_fleet_coordinator!` | Candid is unchanged; admission code reachability changes |
| sensitive status authorization | exact cycle and raw metrics variants are publicly readable | managed, standalone-local, Root and Store variants require controller authority | Candid is unchanged; anonymous observability reachability is removed |
| managed Fleet observability relay | no protected path from the human-controlled Root to Root-controlled Components | managed `canic_command::Observe` and Root `canic_root_command::ObserveCanister` carry the bounded shared observability DTO | managed and Root Candid/runtime reachability expands |

The first two changes pre-existed this audit continuation. The Coordinator
wiring correction was made during this inventory: the dedicated decoder and
variant-specific limit function were already generated and enforced again in
endpoint execution, but the start macro had left them unreachable from IC
inspect-message admission. This was an ingress-admission consistency defect,
not an authorization bypass. The two observability rows close the subsequently
identified public telemetry exposure while retaining operator access without
adding human principals as managed Component controllers.

No v6 number may be used as the size or function result for this overlay. A
future immutable-candidate checkpoint must rebuild all affected roles and
compare code-section bytes, total bytes, replica-limited defined functions,
the optimizer-defined cross-check, table entries, instructions, Candid and
protocol-profile identity.

## B1 Routing Result

The source inventory is complete enough to route controlled experiments:

- lifecycle and storage registration belong to B2's direct role wiring;
- endpoint unions, admission adapters, Candid construction/documentation,
  metrics dispatch, commands, status and recovery remain B4 concerns;
- the Store endpoint may schedule deferred GC work because of the IC execution
  boundary, but Store-local GC semantics must remain below that endpoint;
- Root and Coordinator retain their independent projections and protocol
  unions; and
- no large shared protocol enum or generic provider registry is justified by
  this inventory.

B1 is not complete. The repository-owned capability-fixture measurements,
controlled ablations, optimized `1..=N` generic-cohort measurements,
post-`-Oz` attribution and compatible predecessor evidence remain open. The
generic family and fixture are frozen in the
[generic-instantiation cohort](b1-generic-instantiation-cohort.md), and the
source destruction trace is retained in the
[destroyed-state inventory](b1-destroyed-state-inventory.md). B2 stays blocked
until the complete B1 evidence receives maintainer acceptance.
