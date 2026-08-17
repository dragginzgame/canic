# 0.103 B1 Capability Manifest

Status: accepted B1 evidence decision. This freezes declaration and discovery
semantics only; it authorizes no Candid or runtime mutation.

## Declaration Authority

0.103 introduces no generic capability list, endpoint registry or new config
section. The authoritative set remains the closed
`RoleCapabilityKey` enum. `resolve_role_contract` and
`derive_role_capabilities` in `canic-core::role_contract` derive it from the
validated current config plus built-in role identity, and `canic::build!`
consumes that exact result.

| Capability | Static declaration source |
| --- | --- |
| `Runtime` | implicit for every configured deployable role and the built-in Store |
| `Root` | `[roles.<role>] kind = "root"` |
| `RootControlPlane` | implied exactly by `Root` |
| `FleetCoordinator` | built-in Coordinator identity only |
| `WasmStore` | built-in Store identity only |
| `Scaling` | presence of the role's validated `scaling` config |
| `Sharding` | presence of the role's validated `sharding` config |
| `Index` | presence of the role's validated `index` config |
| `Icrc21` | the role's validated `standards.icrc21 = true` |
| `DelegatedTokenIssuer` | the role's validated `auth.delegated_token_issuer = true` |
| `DelegatedTokenVerifier` | the role's validated `auth.delegated_token_verifier = true` |
| `RoleAttestationVerifier` | the role's validated `auth.role_attestation_cache = true` |
| `RoleAttestationSigner` | a Root when any configured Component or child uses the role-attestation cache |

This reuses the current config syntax exactly. Cargo features remain
implementation availability and never declare a role capability or exported
variant.

## Invalid Combinations

The existing role-contract resolver remains the one build-time rejection
boundary:

- unknown roles and package/config role mismatches fail;
- `FleetCoordinator` and `WasmStore` arise only from their built-in identities;
- `RootControlPlane` cannot exist without `Root`;
- `RoleAttestationSigner` is Root-only and derived, not manually asserted;
- every capability's required Cargo features must be present;
- unknown config fields remain rejected by the typed schema; and
- the existing valid coexistence of `Scaling`, `Sharding` and `Index` is not
  artificially prohibited.

0.103 adds no compatibility interpretation and no stringly capability escape
hatch. B3 must make each unavailable variant, DTO, handler and implementation
path absent rather than returning a runtime unsupported error.

## Compiled Discovery

No existing protected runtime or release surface exposes the exact uncollapsed
`RoleCapabilityKey` set. The host's current `role_capabilities` projection
collapses the four auth capabilities into one `auth` label and omits intrinsic
role capabilities, so it cannot be the B1 discovery authority.

The accepted discovery source is therefore a bounded `Overview` request on the
role's mandatory `canic_status` method. Its response contains:

- the exact role identity;
- the exact closed, lexicographically rendered compiled capability set; and
- the Canic release identity already owned by metadata.

The response is generated from the same build-time resolved capability set
that prunes variants. It is immutable for the installed Wasm, carries no
runtime registration API and adds no method. Host-side config projections may
render the same labels for planning, but they are checked against this typed
source rather than becoming a second authority.

## Reserved Names

When a role emits a Canic control surface, `canic_status` and, when nonempty,
`canic_command` are reserved before Candid generation. An application-owned,
standard or manually declared endpoint with either name is a build error. The
error is independent of macro order; no endpoint is replaced or merged.

## Capability Effect

- `Runtime` supplies only bounded common status variants and the minimum
  command variants required by an actual managed lifecycle or RPC boundary.
- `Root` and `RootControlPlane` supply Root-owned control/status variants;
  workflow phase methods do not survive as variants merely because their code
  is linked.
- `FleetCoordinator` and `WasmStore` select their distinct role-specific
  command/status types.
- auth, scaling, sharding and index capabilities add only their exact typed
  variants and referenced DTOs.
- `Icrc21` retains its externally mandated method rather than becoming a Canic
  command/status variant.
- capabilities never add a Canic method.

The exact old-method-to-variant or private/delete decisions remain owned by
`method-register.tsv`; this manifest does not classify methods by prefix.

## Exact Pruning Matrix

The register's replacement column is the target-variant authority. Capabilities
select from that closed set as follows; a capability with no row adds no public
variant merely to prove that it exists.

| Capability | Exact 0.103 protocol effect |
| --- | --- |
| `Runtime` | Adds `Overview`, `CycleBalance`, `CycleHistory`, `CycleTopups`, `Health`, `Logs`, `Metrics`, `Readiness` and `Runtime` to the configured role's status type, plus `Binding` for managed and Store roles and `RespondCapability` to a role that owns that RPC response. |
| `Root` + `RootControlPlane` | These always coexist and jointly add every Root target in `method-register.tsv` except the signer-gated role-attestation pair and the already listed `Runtime` targets. They are not independently pruned protocol layers. |
| `FleetCoordinator` | Adds `CoordinatorStatusRequest::Overview` plus every Coordinator target in the register. The current Coordinator has no old overview method, so discovery is the one target not represented by an old row. |
| `WasmStore` | Adds every Store target and the two admitted Store data lanes except the already listed `Runtime` targets. |
| `Sharding` | Adds `CanisterStatusRequest::Children` to a managed role. Root child observation is already owned by the Root control-plane pair. |
| `DelegatedTokenIssuer` | Adds `PrepareDelegatedToken`, `InstallDelegationProof`, `ActiveDelegationProof` and `DelegatedToken` to the managed command/status types. |
| `RoleAttestationSigner` | Adds `RootCommand::PrepareRoleAttestation` and `RootStatusRequest::RoleAttestation`. |
| `DelegatedTokenVerifier`, `Index`, `RoleAttestationVerifier`, `Scaling` | Add no 0.103 Candid variant. Their current effects remain local state, validation and workflow reachability. |
| `Icrc21` | Retains only the separately named external-standard method. |

`Operation` is not an independently selectable capability. A role gets its one
local `Operation` status variant exactly when at least one compiled command owns
a durable/asynchronous operation. Removing the last such command removes the
variant and its role-specific operation response type.

## DTO Mapping Rule

The register freezes both the released Rust signature and the target variant.
For a one-to-one target, the current named request and success types in
`rust_signature` remain the target's inner payloads. The common role method
adds only its role-specific request/response enum and the maintained compact
`Error`; it does not wrap a DTO in another generic command family.

The following synthesized boundary types replace shapes that cannot be carried
over literally:

~~~text
RoleOverviewResponse {
    role: CanisterRole,
    capabilities: Vec<RoleCapability>,
    metadata: CanicMetadataResponse,
    bootstrap: BootstrapStatusResponse,
}

OperationStatusRequest { operation_id: [u8; 32] }
OperationReceipt { operation_id: [u8; 32] }

CanisterInspectionRequest { canister_id: Principal }
ConfigStatusResponse { toml: String }
CycleBalanceStatusResponse { cycles: Cycles }
CycleRefillInput {
    operation_id: [u8; 32],
    source_subaccount: Option<[u8; 32]>,
    amount_e8s: u64,
}

LogStatusRequest {
    crate_name: Option<String>,
    topic: Option<String>,
    min_level: Option<Level>,
    page: PageRequest,
}

MetricsStatusRequest { kind: MetricsKind, page: PageRequest }
TemplateLookupRequest { template_id: TemplateId, version: TemplateVersion }
TemplateChunkRequest {
    template_id: TemplateId,
    version: TemplateVersion,
    chunk_index: u32,
}

SetCyclesFundingRequest { enabled: bool }
SetFleetStatusRequest { status: FleetStatus }
PoolCanisterRequest { canister_id: Principal }
PoolHandoffRequest { canister_id: Principal, recipient: Principal }
~~~

`RoleCapability` is the boundary enum with exactly the same 13 variants as
`RoleCapabilityKey`, rendered lexicographically. `Overview` replaces the old
bootstrap/ready/metadata trio; `bootstrap.ready` is the single readiness bit,
so it is not duplicated. The Fleet and pool command enums are flattened into
the explicit Root variants named in the register. The Store chunk read adopts
`TemplateChunkRequest`; the already named publish-chunk request is unchanged.

The only request/response DTOs still open are the four role-specific operation
status response enums and any genuinely new consolidated progress structs they
reference. Their variants must follow the accepted high-level operations, not
the deleted phase list. All other retained inputs and success payloads are now
fixed by the register, its released Rust signature or the synthesized shapes
above.

## Operation Ownership

The operation lane is limited to commands that already have durable phase
status or whose accepted outcome must survive an uncertain paid effect. Every
command target absent from this table is atomic and returns its typed success
payload directly; crossing an `await` alone does not add an operation.

| Role operation | Command variants returning `OperationReceipt` | Status detail payload |
| --- | --- | --- |
| Root Store adoption | `AdoptStore` | `FleetSubnetWasmStoreAdoptionResponse` |
| Root Store bootstrap | `BootstrapStore` | `RootStoreBootstrapResponse` |
| Root Component Registry preparation | `PrepareComponentRegistry` | `RootComponentRegistryPreparationResponse` |
| Root Fleet activation | `PrepareFleetActivation`, `ResumeFleetActivation` | `FleetActivationStatusResponse` |
| Root child allocation | `ProvisionChild` | `RootComponentChildOperationStatus` |
| Root Component allocation | `ProvisionComponent`, `ProvisionPeer` | `RootComponentOperationStatus` |
| Root Component batch provisioning | `ProvisionComponents` | `RootComponentProvisioningStatusResponse` |
| Root cycle refill | `RefillCycles` | `IcpRefillResponse` |
| Root Component removal | `RemoveComponent` | `RootComponentRemovalOperationStatus` |
| Root removal preparation | `RemoveRoot` | `RootRemovalOperationStatus` |
| Root subtree removal | `RemoveSubtree` | `RootComponentSubtreeRemovalResponse` |
| Root Registry synchronization | `SynchronizeRegistry` | `FleetSubnetRootRegistrySyncResponse` |
| Coordinator Component provisioning | `ProvisionComponents` | `FleetComponentProvisioningStatusResponse` |
| Coordinator root removal | `RemoveRoot` | `FleetSubnetRootDeletionResponse` |
| Managed runtime configuration | `ConfigureRuntime` | `ComponentRuntimeOperationStatus` |
| Store Fleet activation | `ActivateFleet`, `PrepareFleetCredential` | `FleetActivationStatusResponse` |
| Store garbage collection | `RunGc` | `WasmStoreGcOperationStatus` |

`PreviewCycleRefill` remains atomic and returns `IcpRefillDryRun`. The existing
Fleet and pool setters, authority snapshot actions, proof/attestation actions,
Store snapshot writes, template control actions and deletion-cycle reclamation
also remain typed atomic commands. Existing domain records and receipts remain
their authority; 0.103 creates no universal operation database.

The four response enums are now fixed to the rows above:

~~~text
RootOperationStatusResponse        = the twelve Root detail variants
CoordinatorOperationStatusResponse = the two Coordinator detail variants
CanisterOperationStatusResponse    = ConfigureRuntime(ComponentRuntimeOperationStatus)
StoreOperationStatusResponse       = FleetActivation(...) | GarbageCollection(...)
~~~

The six new `*OperationStatus` detail structs are projections over existing
domain-owned durable state. Each contains the operation ID, current finite
domain state, last completed durable boundary, optional typed terminal result
and optional compact diagnostic code. They contain no timer handle, deadline,
provider generation, generic action list, prose error or caller-selected phase.

## 0.104 Consumer Handoff

0.103 adds no fixed timer identity, registration kind, provider facade,
`TimerKey`, `AsyncRecoveryOwner` or stable timer record. While 0.103 is the
current release, its self-advancing workflows may use the existing private
lifecycle deferral/recovery seam only to request another bounded run. Durable
demand remains in the operation records named above.

The exact new 0.104 consumer input is four domain participants:

- Root operations: the twelve Root operation details above;
- Coordinator operations: Component provisioning and root removal;
- managed operations: runtime configuration; and
- Store operations: Fleet activation and garbage collection.

0.104 must replace any temporary scheduling call from those participants with
native provider composition and domain-owned retry demand. It may use its one
accepted recovery watchdog to request bounded participant scans, but it may not
copy the operation state into timer storage or create one provider registration
per operation. Atomic commands add no 0.104 consumer.
