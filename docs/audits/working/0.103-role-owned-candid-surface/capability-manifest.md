# 0.103 B1 Capability Manifest

Status: accepted B1 evidence decision. This freezes declaration, profile-
binding and correlation semantics. B2/B3 are complete in the unreleased
worktree; this evidence file itself grants no broader Candid or runtime
mutation.

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
| `AutomaticTopup` | any validated Component or child using the compiled role has `topup` configured; never implied for Root, Coordinator or the built-in Store |
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
- `AutomaticTopup` is non-Root and config-derived; an absent `topup` policy
  cannot retain its DTOs, status variant or active runtime timer owner. The
  private registration/callback code hard cut remains 0.104-owned;
- `RootControlPlane` cannot exist without `Root`;
- `RoleAttestationSigner` is Root-only and derived, not manually asserted;
- every capability's required Cargo features must be present;
- unknown config fields remain rejected by the typed schema; and
- the existing valid coexistence of `Scaling`, `Sharding` and `Index` is not
  artificially prohibited.

0.103 adds no compatibility interpretation and no stringly capability escape
hatch. B3 must make each unavailable variant, DTO, handler and implementation
path absent rather than returning a runtime unsupported error.

## Compiled Profile Identity And Binding Bootstrap

No existing protected runtime or release surface binds the exact uncollapsed
`RoleCapabilityKey` set to the generated Candid. The host's current
`role_capabilities` projection collapses the four auth capabilities into one
`auth` label and omits intrinsic role capabilities, so it cannot select a
profile-specific binding.

The accepted bootstrap model is external manifest selection. One canonical
protocol-profile digest binds:

- the exact Canic release identity;
- the exact role identity;
- the exact closed, lexicographically ordered compiled capability set; and
- the generated Candid SHA-256.

Its current-form encoding is fixed as SHA-256 over the ASCII domain
`canic.protocol-profile.v1`, followed by the release identity and role as
`u32` big-endian byte length plus UTF-8 bytes, the capability count as `u32`
big-endian, each lexicographic capability name in the same length-prefixed
form, and the raw 32-byte Candid digest. The Candid digest covers the exact
maintained generated DID bytes after canonical extraction formatting. No JSON,
display rendering or unordered collection is a digest authority.

The build artifact and accepted release-set/Directory metadata carry that
identity beside the Wasm identity. Before its first role call, the host or CLI
selects the exact generated full binding by protocol-profile digest. A static
inter-canister caller may use only the generated request/response fragment for
its one owned variant after protected metadata proves that the exact target
profile contains it. That fragment comes from this manifest's correlation
table; it is not a generic superset. Missing or mismatched profile evidence
fails before dispatch; trial decoding, fallback bindings, dynamic method
probing and runtime negotiation are forbidden.

Every role retains a bounded `Overview` status request. Its response contains
the exact role, exact compiled capability set, Canic release identity and
protocol-profile digest produced from the same build-time authority. This is a
post-selection verification surface, not a bootstrap mechanism. B2/B3 must
extend the existing artifact/release metadata so the external selector and
`Overview` are bidirectionally checked against the same generated profile.

## Reserved Names

When a role emits a Canic control surface, `canic_status` and, when nonempty,
`canic_command` are reserved before Candid generation. An application-owned,
standard or manually declared endpoint with either name is a build error. The
error is independent of macro order; no endpoint is replaced or merged.

## Capability Effect

- `Runtime` supplies only universally available local runtime observations and
  the minimum command variants required by an actual managed lifecycle or RPC
  boundary. It does not own automatic top-up, refill, treasury or funding
  policy merely because those features concern cycles.
- `AutomaticTopup` owns top-up event observation and its public handler. It is
  also the exact decision that 0.104 consumes to prune private automatic-
  funding registration, callback and workflow reachability; 0.103 does not
  duplicate that timer-mechanics hard cut.
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
| `Runtime` | Adds `Overview`, `CycleBalance`, `CycleHistory`, `Health`, `Logs`, `Metrics`, `Readiness` and `Runtime` to the configured role's status type, plus `Binding` for managed roles and exact `Authority` for Store. `CycleHistory` is local balance observation, not funding history. |
| `ChildProvisioning` | Adds `RespondCapability` only to an exact non-Root role with at least one nonempty `spawn_grants.<parent-role>` entry. The built-in Store retains the same response through its distinct Store command surface. Leaf and ordinary top-level Component roles compile no non-Root capability responder or its request/response codec graph. |
| `AutomaticTopup` | Adds `CanisterStatusRequest::CycleTopups` only to a configured managed role with an explicit `topup` policy, together with its referenced DTO and public handler reachability. Root, Coordinator and the implicit Store cannot acquire it. The same frozen decision is 0.104's private timer/workflow pruning input. |
| `Root` + `RootControlPlane` | These always coexist and jointly add every Root target in `method-register.tsv` except the signer-gated role-attestation pair and the already listed `Runtime` targets. They are not independently pruned protocol layers. |
| `FleetCoordinator` | Adds `CoordinatorStatusRequest::Overview` plus every Coordinator target in the register. The current Coordinator has no old overview method, so profile verification is the one target not represented by an old row. |
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
    protocol_profile_digest: [u8; 32],
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

PoolMaintenanceResponse = Maintained
    | MaintenancePaused { reason }
    | Created { canister_id }
    | RefillWaitingForCycles { available, creation_amount }
    | RefillPending { operation_id, uncertain_result }
    | RefillBlocked { operation_id, failure }
    | ResetReady { canister_id }
    | ResetFailed { canister_id, reason }
PoolImportResponse = Imported { canister_id }
    | ResetFailed { canister_id, reason }
PoolRefillRetryResponse { previous_operation_id: [u8; 32] }
PoolResetRetryResponse { canister_id: Principal }
PoolHandoffResponse { canister_id: Principal, recipient: Principal }
~~~

`RoleCapability` is the boundary enum with exactly the same 14 variants as
`RoleCapabilityKey`, rendered lexicographically. `Overview` replaces the old
bootstrap/ready/metadata trio; `bootstrap.ready` is the single readiness bit,
so it is not duplicated. The Fleet and pool command enums are flattened into
the explicit Root variants named in the register. The Store chunk read adopts
`TemplateChunkRequest`; the already named publish-chunk request is unchanged.

The only role request/response DTOs still open are the four role-specific
operation status response enums and any genuinely new consolidated progress
structs they reference. Their variants must follow the accepted high-level
operations, not the deleted phase list. Artifact metadata must additionally
carry the profile fields frozen above. All other retained inputs and success
payloads are fixed by the register, its released Rust signature, the
synthesized shapes above and the correlation rule below.

## Request And Response Correlation

The B1 mapping is normalized rather than repeating response facts across 207
rows:

- `method-register.tsv` owns the old method, exact target request variant,
  execution mode, authority/payload bound, replay policy and released Rust
  request/success signature;
- the operation table below is the exact operation-owner relation; and
- this section owns the closed response correlation.

For a status selector `X`, only the same role's `StatusResponse::X` may be
returned. For an atomic command `X`, only `CommandResponse::X` carrying the
named success payload admitted by the register may be returned. Where one old
command enum was flattened, each new request variant admits only its matching
old response arm or an explicitly narrowed response shape; it cannot return the
former broad response family. Every command in the operation table returns
only `CommandResponse::OperationAccepted(OperationReceipt)` on acceptance and
is observed through the owning role's `StatusRequest::Operation` response. All
failures use the maintained compact `Error` boundary.

The three released multi-command inputs are narrowed exactly:

| Request variant | Only admitted success response |
| --- | --- |
| `RootCommand::SetCyclesFunding` | `RootCommandResponse::SetCyclesFunding(SetStateResponse<bool>)` from the old `CyclesFundingEnabled` arm |
| `RootCommand::SetFleetStatus` | `RootCommandResponse::SetFleetStatus(SetStateResponse<FleetStatus>)` from the old `Status` arm |
| `RootCommand::PreviewCycleRefill` | `RootCommandResponse::PreviewCycleRefill(IcpRefillDryRun)` from the old `DryRun` arm |
| `RootCommand::RefillCycles` | `RootCommandResponse::OperationAccepted(OperationReceipt)`; terminal `IcpRefillResponse` is operation status |
| `RootCommand::MaintainPool` | `RootCommandResponse::MaintainPool(PoolMaintenanceResponse)` |
| `RootCommand::RetryPoolRefill` | `RootCommandResponse::RetryPoolRefill(PoolRefillRetryResponse)` from the old `RefillRetryScheduled` arm |
| `RootCommand::ImportPoolCanister` | `RootCommandResponse::ImportPoolCanister(PoolImportResponse)` |
| `RootCommand::RetryPoolReset` | `RootCommandResponse::RetryPoolReset(PoolResetRetryResponse)` from the old `ResetQueued` arm |
| `RootCommand::HandoffPoolCanister` | `RootCommandResponse::HandoffPoolCanister(PoolHandoffResponse)` from the old `HandedOff` arm |

The bounded B4 correction retains four additional direct correlations that
carry independent authority or outcome evidence rather than private phase
selection:

| Request variant | Only admitted success response |
| --- | --- |
| `RootCommand::SynchronizeComponentDirectories` | `RootCommandResponse::SynchronizeComponentDirectories(RootComponentDirectorySynchronizationResponse)` |
| `CoordinatorCommand::AcknowledgeRootSnapshot` | `CoordinatorCommandResponse::AcknowledgeRootSnapshot(FleetSubnetRootSnapshotAcknowledgement)` |
| `CoordinatorCommand::PrepareRootDeletionExecution` | `CoordinatorCommandResponse::PrepareRootDeletionExecution(FleetSubnetRootDeletionExecutionResponse)` |
| `CoordinatorCommand::CompleteRootDeletion` | `CoordinatorCommandResponse::CompleteRootDeletion(FleetSubnetRootDeletionResponse)` |

The exact participating Root reads the complete Registry through
`CoordinatorStatusRequest::Registry` and derives its manifest and version
locally. That read adds no selector. Initial Store bytes use the admitted Store
control/data lanes under the B5 pre-adoption authority; they do not return to
the Root command union.

Both dispatchers use exhaustive request matches with no wildcard. Focused tests
prove every permitted request/response pair and reject unrelated response arms.
In particular, synchronous commands cannot return an operation receipt,
asynchronous commands cannot return misleading terminal synchronous success,
and one status selector cannot return another selector's page. The register,
operation table and this correlation rule together are the exact B1 columns
`request variant`, `response variant(s)`, `authority`, `payload bound`,
`execution mode`, `replay policy` and `operation owner`; no second divergent
207-row ledger is introduced.

## Operation Ownership

The operation lane is limited to commands that already have durable phase
status or whose accepted outcome must survive an uncertain paid effect. Every
command target absent from this table is atomic and returns its typed success
payload directly; crossing an `await` alone does not add an operation.

| Role operation | Command variants returning `OperationReceipt` | Status detail payload |
| --- | --- | --- |
| Root Store adoption | `AdoptStore` | `FleetSubnetWasmStoreAdoptionResponse` |
| Root Store bootstrap | `BootstrapStore` | `RootStoreBootstrapResponse` |
| Root Component Registry preparation | `PrepareComponentRegistry` | `RootComponentRegistryStatusResponse` |
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
