# Canic 0.100 Implementation Status

Date: 2026-07-28

- State: implementation in progress.
- Release boundary: reinstall only.
- Implementation started: yes; intermediate Tree identities were released in
  immutable `v0.100.0`.
- Workspace package version: `0.100.29`.
- Latest published release: `v0.100.29`.
- Open patch draft: `0.100.30`; no package-version change has been authorized.
- Open design blockers: none.

The 2026-07-26 design amendment removes the proposed Tree layer. The target is
exactly one Fleet Subnet Root per occupied `(FleetKey, SubnetId)`, with each
root managing multiple Component instances as dynamic multi-level trees. A
Component Spec declares one direct Component role and a flat catalog of every
potential descendant role/Wasm. Concrete parentage is root-owned Registry
state. A Component creation operation receives a root-allocated
`ComponentInstanceId` only when the root durably reserves it before any paid
Canister effect.

Different Fleets may each own an independent Fleet Subnet Root on the same
physical Subnet. Root uniqueness, authority, admissions, Stores and limits
remain Fleet-scoped.

The amendment separates immutable Component Topology, root-local Component
admissions, each root's active release set and the root-local Wasm Store
Catalog. The host builds one qualified Component/Component-Child artifact
union per `ReleaseBuildId` and projects an exact release-set manifest for each
root.

The latest design amendment separates flat artifact admission from dynamic
runtime parentage. A Project Hub creates a Project Instance as its direct child
on the same root/Subnet; that Project Instance may create its Ledger, Machine
and further children. Each child records its immediate parent, but the Fleet
Subnet Root remains sole controller, Registry owner and lifecycle executor.
Every root admitted for the Project Hub Spec stores the complete
Hub/Instance/Ledger/Machine potential-Wasm catalog once. Released 0.100.9
hard-cuts the Component Spec/Topology canonical shape. The maintained pre-1.0
schema/domain identifier remains v1 and has no prior-shape decoder.

Canic infrastructure now has its own exact three-entry artifact manifest for
the Coordinator, Fleet Subnet Root and Wasm Store. The host directly installs
the Coordinator and roots; each root alone bootstraps its local Store from the
verified infrastructure artifact.
The Coordinator is not part of a Component hierarchy and is not installed
through a Wasm Store.

Released 0.100.1 removed the staged Tree declaration layer from
source, generated bootstrap, host/CLI projections, scaffolding, fixtures and
active guidance. It retains no Tree identity or compatibility alias. Runtime
authority is still transitional until the later binding, admission and
Registry slices replace the 0.99 root model.

## Slice 1 — Freeze Current Authorities and Component Topology

- [x] Record the complete live Registry, Directory, cascade, lifecycle,
  bootstrap and role-package
  [authority inventory](0.100-authority-inventory.md).
- [x] Record the intermediate Tree declaration implementation and its bounded
  validation.
- [x] Remove `TreeSpecId`, `TreeGroupId`, `TreeId` and their public exports.
- [x] Replace `[tree_specs.*]` and `[tree_groups.*]` with flat
  `[component_specs.*]`.
- [x] Replace `canic app role attach --tree-spec` with
  `--component-spec <component-spec>` and no alias.
- [x] Require exactly one Component role and a flat potential child-role/Wasm
  catalog per Component Spec.
- [x] Enforce positive Spec maxima and the 4,096-Component-instance Fleet
  bound.
- [x] Reject `root`, `service` and `component` child kinds,
  `owner_component` and nested Component declarations.
- [x] Add `ComponentSpecId` and the canonical `ComponentInstanceId` type.
- [x] Derive and freeze the canonical bounded Component Topology.
- [x] Hard-cut the canonical Component Spec/Topology v1 encoding to add then
  remove initial/direct-depth authority, retaining only the flat
  potential-Wasm catalog plus exact role-to-role spawn grants.
- [x] Validate bounded spawn-grant parents, targets, completeness and
  per-parent ceilings while allowing recursive role capabilities.
- [x] Validate bounded peer-Component provisioning-grant targets, cycles and
  per-requester/root ceilings.
- [x] Replace the temporary environment Component Spec selector with protected
  `ComponentBinding`.
- [x] Freeze `SubnetId`, Coordinator authority, `FleetSubnetRootBinding`,
  Component admissions, root limits and protected Component/child bindings.
- [x] Bind every Component Child to its immediate Component-tree parent so
  protected identity can represent arbitrary runtime depth.
- [ ] Hard-cut Fleet Root to Fleet Subnet Root.
- [ ] Hard-cut local `SubnetRegistry` and `SubnetDirectory` to root-owned
  per-Component `ComponentRegistry` and `ComponentDirectory`.
- [x] Split Fleet and Component Directory provenance.
- [ ] Prove no prior-release transition reader or decoder exists.

## Slice 2 — Topology-Admitted Artifacts and Fresh Root Installation

- [x] Revise config/bootstrap/host projections to Component Specs.
- [x] Canonicalize every Component Spec and freeze its topology hash.
- [x] Distribute positive per-root Component Spec admissions whose sum does
  not exceed the Fleet ceiling.
- [x] Freeze the exact three-role Canic Infrastructure Artifact Manifest
  model, compiler and canonical digest.
- [x] Populate and persist that manifest from the qualified complete-build
  outputs.
- [x] Freeze the exact qualified application artifact-union model, compiler
  and canonical digest under one release-build and Fleet-wide topology
  identity.
- [x] Populate and persist that application artifact union once from the
  qualified complete-build outputs.
- [x] Freeze exact Spec-scoped Fleet Subnet Root Release-Set Manifest
  projection and canonical digests.
- [x] Materialize and persist one projected release-set manifest per planned
  root.
- [x] Freeze one immutable pre-effect Fleet install-plan boundary from exact
  resolved Coordinator/root placement, admissions, limits and funding input.
- [x] Resolve strict operator input to independent exact Coordinator and Fleet
  Subnet Root placements, admissions, configured limits and funding.
- [x] Record the Coordinator's resolved facts in its creation journal.
- [x] Record every root's resolved facts in its creation journal.
- [x] Freeze exact initial creation funding in the immutable plan before any
  external effect.
- [x] Bind every Coordinator creation intent and effect to that frozen
  funding.
- [x] Bind every root creation intent and effect to that frozen funding.
- [x] Install the Coordinator from empty state.
- [x] Install and independently verify every planned root behind its runtime
  `Prepared` fence.
- [x] Bootstrap each root's local topology-admitted Wasm Store.
- [x] Commit the genesis Fleet Registry.

## Slice 3 — Fleet Registry and Root Lifecycle

- [x] Implement canonical snapshot commits with Fleet Component Spec and root
  rows.
- [x] Enforce one Fleet Subnet Root per occupied `(FleetKey, SubnetId)`.
- [x] Prove another Fleet may independently use the same physical Subnet.
- [x] Implement root `Joining`.
- [x] Implement the initial atomic all-root `Active` transition.
- [ ] Implement root `Draining` and `Removed`.
- [x] Install initial roots behind the runtime `Prepared` fence.
- [ ] Enforce Spec, admission, root, topology, limits, active-release-set and
  tombstone rules.

## Slice 4 — Component Lifecycle, Mirrors and Directories

- [x] Prepare one empty root-local Component Registry authority against the
  exact Store, active Registry Mirror and Fleet Directory.
- [x] Implement durable root-local `ComponentInstanceId` allocation.
- [x] Implement admitted direct Component Canister creation through the root.
- [x] Install and independently verify an admitted direct Component from its
  exact root-local Store under an immutable `ComponentBinding`.
- [ ] Implement same-root grant-checked peer Component provisioning while
  retaining causal origin without parentage.
- [ ] Implement authenticated parent-to-root child effects at arbitrary depth.
- [ ] Make the Fleet Subnet Root the required lifecycle controller and retain
  authoritative idempotent receipts.
- [ ] Resolve lifecycle artifacts only through the active release set.
- [x] Implement bounded Fleet snapshot synchronization once per root.
- [x] Atomically activate the Fleet Registry Mirror and Fleet Directory.
- [x] Store logical Component Registries in one bounded root-local collection
  with independent per-Component heads.
- [x] Commit normalized top-level Component rows with a principal index and
  terminal operation receipt.
- [ ] Store normalized Component Registry rows with principal, parent/role,
  count and operation-journal indexes.
- [x] Derive the first ownership-preserving Component Directory head from its
  exact committed Registry partition.
- [ ] Derive ownership-preserving Component Directories with compact heads and
  revision-bound pagination.
- [ ] Run subtree removal as durable post-order traversal and partition
  mutation serialization by Component instance.
- [x] Distribute exact Directories directly from the root to a committed
  Component with target-local retention, independent observation and a
  terminal root receipt.
- [x] Activate a Directory-prepared Component runtime from its exact retained
  authority, reconcile uncertain response, independently observe target
  `Active` and retain a terminal root runtime receipt.
- [x] Hard-cut generic cascade, credential-generation and Fleet-activation
  mutations from managed application Components while retaining the separate
  Wasm Store bundle.
- [x] Promote an exactly runtime-active Component Registry partition to
  `Active`, derive its next Directory revision and synchronize that current
  authority to the target before root activation.
- [ ] Apply the same direct distribution boundary to registered Component
  Children as descendant creation lands.

## Slice 5 — Recovery and Closeout

- [ ] Qualify interruption and exact retry.
- [ ] Prove one Component operation cannot block an unrelated Component.
- [x] Activate initial roots only after active-release-set Store and final
  topology synchronization.
- [ ] Hard-cut the terminal Fleet catalog from one root principal to the
  Coordinator principal and publish it only after complete terminal evidence.
- [ ] Expose compact root-local known-created, not-deletion-confirmed Canister
  count summaries without enumerating every Component Registry member.
- [ ] Add `canic info subnets <fleet> [--json]` with one canonical row per
  occupied physical Subnet, exact Fleet-owned Canister counts and fail-closed
  Coordinator/root evidence validation.
- [ ] Qualify restore fencing and role-package boundaries.
- [ ] Measure Wasm boundaries and remove stale authority.

## Current Batch

Released 0.100.1 parses only flat
`[component_specs.*]`, compiles all Component and potential child roles into
bootstrap input, builds their union through host projections and exposes
`--component-spec` attachment with no old flag. Root is implicit
infrastructure outside Specs. Top-level Component roles are unique as
Component declarations, but a role may also appear in another Spec's flat
potential-descendant catalog. The same declared descendant artifact may occur
in several catalogs without a role-only lookup choosing an owner.

Released 0.100.1 defined but did not allocate durable `ComponentInstanceId`
values. It compiles each validated config into a bounded canonical
Component Topology, freezes domain-separated golden Spec hashes and root-local
topology digests, and exposes strong `SubnetId`, Coordinator/root authority,
root limits, admission and Component/child binding contracts. Component
aggregate descendant, Registry-byte and cycles-funding limits compile from finite
config defaults or exact overrides.

The host topology planner accepts resolved root principals, physical Subnets,
limits and positive per-Spec capacities, then derives hashes/digests itself.
It canonicalizes root/admission order, enforces App/Fleet binding, complete
Fleet admission coverage and one root per Fleet/Subnet pair, while independent
Fleets may reuse one physical Subnet. Root bootstrap and managed release-role
projection now consume compiled topology instead of raw Component Spec
configuration.

Released 0.100.2 defines the infrastructure artifact family independently from
application release sets. Its compiler admits exactly one Fleet Coordinator,
Fleet Subnet Root and Wasm Store entry from one `ReleaseBuildId`, derives
raw/gzip lengths and hashes from matching bytes, canonicalizes entry order and
freezes deterministic manifest bytes and digest.

Released 0.100.3 adds a qualified infrastructure build-output contract and
immutably persists the canonical manifest under its durable
`ReleaseBuildId`. Loading revalidates canonical bytes and path identity. Exact
same-release retry is idempotent before or after release-build finalization;
conflicting evidence, unsafe artifact paths and first publication after
finalization fail closed. A normal complete install build now requires its
durable release-build identity before Cargo execution.

Released 0.100.4 freezes the separate application artifact-union
compiler. It requires the exact topology role set in both pre-build targets
and qualified build outputs, binds canonical evidence to one
`ReleaseBuildId` and the Fleet-wide Component Topology digest, and rejects
package, path, representation, build or topology drift. Exact per-root
release-set projection retains every Component Spec and child-role
authorization entry. Repeated or byte-identical artifacts count only once
against the root's Wasm Store byte limit without deduplicating those
authorization entries.

Released 0.100.5 derives the exact application targets from the
validated complete-build snapshot, qualifies the current raw/gzip outputs
against them and immutably persists the canonical union under its durable
`ReleaseBuildId` before finalization. Loading revalidates canonical bytes,
release-build path identity and the Fleet-wide topology. Exact retry remains
idempotent before or after finalization, while conflicts, unsafe artifact
paths and first publication after finalization fail closed.

Released 0.100.6 adds the immutable pre-effect `FleetInstallPlan`.
It accepts already-resolved Coordinator/root Subnets, positive creation
funding, admissions and limits; derives canonical pre-creation root topology
without fabricated Canister principals; and projects every root manifest from
the finalized application union. Root manifests publish first and the
Fleet/network/release-build-bound plan publishes last. Exact and interrupted
retry is idempotent, while conflicts, noncanonical documents, unsafe files,
identity drift and topology drift fail closed. Different Fleets retain
independent plan paths and may use the same physical Subnets.

Released 0.100.7 hard-cuts Component Spec and Component Topology encoding to
version 2, compiles exact initial-child cardinality and bounded non-parent
provisioning grants, and projects grants incoming to admitted target Specs
without importing requester admission or artifacts.

Released 0.100.8 freezes passive Fleet Registry snapshot, manifest and
version contracts plus a bounded domain-separated canonical encoding.
Epoch-one/revision-one genesis contains the complete immutable Component Spec
set and zero roots. Snapshot validation admits incremental partial `Joining`
rows while enforcing the configured App, exact protected Coordinator
authority, canonical physical-Subnet order, unique root principals, one active
release build, root topology/admissions/limits and aggregate Fleet admission
ceilings. Durable commits and root transitions remain pending.

Released 0.100.9 hard-cuts fixed-depth Component Topology v2 to v3.
Specs now compile one Component role plus a flat catalog of every potential
descendant Wasm. `initial_instances` is removed, aggregate
`maximum_children` becomes `maximum_descendants`, and every protected child
binding records its immediate parent. Child-role `maximum_instances` is
replaced by an explicit role-to-role spawn grant with a positive
`maximum_instances_per_parent`; `maximum_descendants` caps the whole tree and
defaults to 20,000. A child may request another admitted child in the same
Component tree/Subnet only through its exact grant, while the root retains
every controller, funding, Registry, artifact-selection and lifecycle effect.
Descendant roles may own scaling, sharding and Placement Index pools. The
default Registry allowance is 16 MiB, and the design requires normalized
indexed storage, compact revision-bound Directory pages, durable post-order
subtree removal and per-Component concurrency.

Released 0.100.10 adds the separate strict `--fleet-input <path>` operator
document. It carries Coordinator Subnet selection, every exact root Subnet,
root-local Component admissions, complete root limits and positive creation
funding. The normal installer resolves it, freezes all root release sets and
the multi-root Fleet install plan, then stops before effects.

Released 0.100.11 adds the genuine built-in Fleet Coordinator runtime,
protected Registry genesis and controller-only Registry queries. The host
builds and immutably persists the exact Coordinator, Fleet Subnet Root and
Wasm Store artifacts under the same release build before finalization.

Released 0.100.12 replaces the pre-effect Coordinator guard with a canonical
installation journal. The host durably records intent, creates the Coordinator
on its exact planned Subnet with its exact funding method, installs its
qualified Wasm and verifies the live module hash, complete Registry genesis,
manifest and version. A post-verification fence still prevents any legacy
root effect.

Released 0.100.13 replaces that root-effect fence with one immutable journal per
planned Fleet Subnet Root. In canonical plan order, the host durably records
each exact placement and funding effect, creates and installs the qualified
root Wasm, then verifies the live module hash, empty `Prepared` activation
status and protected Fleet/Coordinator/Subnet/admission/limit/release-set
authority. Only after all roots are verified does it validate the complete
Fleet root-binding set and reach the next explicit fence. The obsolete
single-root activation journal and its root creation, cycles and catalog-write
paths are hard-cut; one small Fleet install session retains only shared
identity and finalized release evidence.

Released 0.100.14 extends each root journal through exact release-set staging,
root-owned creation of one implicit local Store, exact publication and
independent `StoreVerified` evidence. Host and runtime canonical manifest
shapes must serialize to identical bytes. The root binds every staged artifact
to its protected build, topology, admissions, package and Store limit, and the
live Store Catalog must equal the complete ordered admitted role set. Exact
retry reuses the same Store and evidence while both root and Store remain
`Prepared`.

Released 0.100.15 extends every root journal through `RegistryJoinVerified`. The
Coordinator atomically compare-and-commits each exact `Joining` row with a
durable response receipt; exact retry returns the original response even after
later Registry revisions. The host verifies every canonical pre- and post-join
prefix and stops only after the complete all-`Joining` snapshot, manifest and
version agree with locally recomputed authority.

Released 0.100.16 hard-cuts the maintained Component Spec, Component Topology and
root-install journal schema identifiers to v1 under the pre-1.0 reinstall-only
rule. It extends every root journal through `RegistrySyncVerified`: a prepared
root reverifies its exact Store, fetches and validates the complete
all-`Joining` Coordinator snapshot, durably stages it before acknowledgement,
then records the Coordinator's exact idempotent `(root, version)` receipt. The
host independently re-queries every root and requires the Coordinator's
complete canonical acknowledgement set for the planned roots and version.

Released 0.100.17 atomically compare-and-commits the complete acknowledged
all-`Joining` Registry to all-`Active`. The Coordinator requires every current
root's exact acknowledgement, stores a response-idempotent activation receipt
and clears superseded acknowledgements in the same commit. A separate v1 host
journal freezes the complete source and target Registries before mutation,
then independently verifies the live all-`Active` Registry, manifest and
version.

Released 0.100.18 extends every root journal through
`RegistryMirrorActivationVerified`. Each prepared root reverifies its exact
Store, fetches and validates the final all-`Active` Coordinator snapshot,
derives its Registry-version-bound Fleet Directory and atomically replaces
the private all-`Joining` candidate with one exclusive active
mirror/Directory record. The host independently re-queries every root and
accepts recovery across the Coordinator transition only when the exact
deterministic all-`Active` state is reproduced.

Released 0.100.19 extends every root journal through
`ComponentRegistryPreparationVerified`. Each root independently reverifies its
Store, active Registry Mirror, protected all-`Active` root row and matching
Fleet Directory before committing one empty Component Registry authority.
Exact retry returns the same authority; conflicting preparation fails closed.
The Registry starts at allocation sequence one with zero reserved or committed
Components, descendants and charged Registry bytes. Fresh root bootstrap no
longer creates configured roles or rebuilds the retired role-based
Directories, so no Component can bypass its future durable Registry binding.

Released 0.100.20 adds the first admitted top-level Component operation phase.
Under the prepared Registry authority, the root independently reverifies its
exact Store, all-`Active` Registry Mirror, protected root row and Fleet
Directory before reserving a nonzero operation ID. Pure policy derives the
Spec role/hash and deterministic root-local `ComponentInstanceId`, enforces
root, Spec, managed-Canister and Registry-byte capacity, then atomically
commits the operation record with the advanced allocation sequence and
reserved count. Exact retry and read-only status reproduce the same durable
reservation; conflicting intent and capacity exhaustion fail closed.

This patch intentionally performs no Canister creation, install, cycle
transfer, `ComponentBinding` commitment or Directory publication. The
operation remains `Reserved`, committed Component and descendant counts stay
zero and the root remains runtime-`Prepared`.

Released 0.100.21 compacts the reinstall-only stable-memory assignment into
consecutive control-plane IDs 10-19 and core IDs 30-62, with reserved growth
through 29 and 99 respectively and application memory beginning at ID 100.

Released 0.100.22 continues one reserved operation through exact Store-bound
empty-Canister creation. The root reverifies its protected authority, active
Registry Mirror/Fleet Directory and exact Store, resolves the reserved
Component role artifact and configured initial cycles, then durably freezes
that evidence, the root as sole controller, replay cost settlement and maximum
terminal Registry-byte charge before the management call. The monotonic
record is `Reserved → CreationIntent → Created`; an unresolved intent is never
blindly repeated, while exact `Created` retry returns the original principal.

Released 0.100.23 continues the created operation through exact Store-backed
installation. Managed application init hard-cuts the retired copied
environment/role and Directory payload to an immutable root admission plus
`ComponentBinding`. Durable
`Created → InstallIntent → Installed → Verified` progress binds the qualified
raw-Wasm digest, exact gzip chunk source and target identity before the
management call. Live verification requires the sole root controller, the
exact gzip-payload status hash and the target's retained binding query.
Interrupted retry advances an already observed exact install without
repeating it, retries only after proving the target remains empty and fails
closed on unavailable or mismatched code.

Released 0.100.24 atomically commits an exactly verified operation. The root
reverifies the protected root, Store, Registry Mirror, Fleet Directory,
topology, caller, live module, sole controller and retained binding before one
mutation inserts the normalized top-level row, principal index, terminal
operation receipt and advanced counters. Each Component partition owns
revision one and a domain-separated content hash independent of its root-local
peers.

The same authority derives the first ownership-preserving Component Directory
head with the exact binding, source root, partition revision/hash and retained
synchronization observation. Exact retry returns the original allocation,
partition and Directory. Installation now reserves the maximum allocation,
partition and index footprint before its paid effect; commitment replaces
that reservation with exact encoded bytes. The Component and root remain
runtime-`Prepared`.

Released 0.100.25 distributes one committed Component's exact active Fleet
Directory and ownership-preserving Component Directory head directly from the
root. The commitment freezes a domain-separated hash of that combined
authority. The target retains its protected `ManagedCanisterBinding`, complete
authority and hash in the existing Fleet activation cell and advances only
from `AwaitingDirectory` to `DirectoryPrepared`.

The root reverifies the Store, active Registry Mirror/Directory, committed
partition, live module, sole controller and retained binding before the call.
It then compares the target's complete response, independently re-queries the
target and commits a fixed-size terminal receipt. An uncertain call is
reconciled from target status before exact retry. Conflicting retained
authority fails closed. The Component Registry row, Component runtime and
root runtime all remain `Prepared`.

Released 0.100.26 activates that Directory-prepared Component only from its exact
protected binding, install operation and retained Directory-authority hash.
The target atomically retains the full original activation Directory
separately from later current Directory revisions, records its original
activation time, advances the shared runtime fence and schedules durable
application init arguments only once. It retains no fabricated cascade or
credential evidence. Application Components hard-cut those old Store-era
mutations from their exported bundle. The root commitment retains the
original prepared partition's encoded-byte evidence so later revisions cannot
change earlier operation responses.

The root reconciles an uncertain activation call from target status,
independently re-queries the exact active receipt and then records its
fixed-size terminal runtime receipt. Exact activation retry returns the
original response, and replay of Directory preparation reconstructs the
original `DirectoryPrepared` response even after the target has progressed.
The Component Registry row and Fleet Subnet Root intentionally remain
`Prepared`.

Released 0.100.27 atomically promotes that exact runtime-active Component's
Registry partition from revision-one `Prepared` to revision-two `Active`,
replaces exact stable-byte accounting and retains a separate immutable
membership receipt. It derives and sends the corresponding current Directory
only after the root-local commit. The already-active target accepts only the
next revision under the same owning Component/source root, a later
synchronization observation and non-regressing Fleet authority; its original
activation Directory remains unchanged.

The root reconciles an uncertain synchronization call through target status,
independently re-queries the exact current authority and commits the terminal
membership bit last. Current Registry/Directory queries expose revision two,
while exact retries of commit, Directory preparation and runtime activation
reconstruct their revision-one responses. The Fleet Subnet Root remains
runtime `Prepared`.

Released 0.100.28 seals the exact ordered initial Component allocation inventory
only after every operation is committed, every partition is `Active`, all
Directory/runtime/membership receipts are terminal and stable counters and
encoded bytes match the complete set. The domain-separated inventory hash
binds immutable allocation, release-set, Registry and Directory evidence to
the protected Fleet activation operation.

While the root remains `Prepared`, the seal blocks new reservations without
blocking exact retry. Root activation revalidates the frozen inventory,
independently queries each target for its exact active current Directory,
records aggregate convergence, advances the root runtime and commits a
terminal root-runtime receipt before bootstrap readiness. Exact activation
retry reconciles an already-active root into the same receipt. Dynamic
allocations resume after activation and cannot rewrite the initial inventory.

Released 0.100.29 hard-cuts role-attestation prepare/get admission from the
removed `SubnetRegistry` predicate to current active Component Registry
identity. The root requires its own runtime to be `Active`, resolves the caller
through the protected principal index and active partition, and binds subject,
role and physical Subnet to that exact Component. Retrieval revalidates active
membership and the caller-bound prepared proof. Verification derives its
expected physical Subnet from protected Component/root authority rather than
the legacy environment root-principal alias. The real Coordinator/root/Store
journey now carries its Registry-created issuer through issuance, claim
rejection and issuer guard metrics without reviving the retired cached
static-role fixture.

Open 0.100.30 rebases the delegated-auth instruction-audit scenarios on that
same real lifecycle. Root proof provisioning, issuer delegated-token
preparation and project-hub token verification now use fresh
Coordinator/root/Store topology plus active Registry-allocated Components.
The method is versioned to v3, and its composite fingerprint includes the
authoritative fixture, canister packages and configs so obsolete fixture
evidence cannot be selected as a baseline.

## Next Action

Maintain checked known-created/not-deletion-confirmed Canister counters under
the root-local Component Registry authority and expose compact
controller-only root summaries. Then add
`canic info subnets <fleet> [--json]`: discover the Coordinator from the
terminal catalog, query its current root rows, fan out only the compact
summaries and fail closed instead of reporting a partial Fleet total.
