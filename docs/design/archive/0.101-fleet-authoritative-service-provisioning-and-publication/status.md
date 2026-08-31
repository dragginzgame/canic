# Canic 0.101 Implementation Status

Date: 2026-08-12

## Status

- State: implementation has the bounded layer-neutral identity vocabulary,
  strict configuration-only Component Group declarations, independent
  deployment flattening with bounded placement envelopes and worst-case Spec
  demand, exact deployment/include/leaf Fleet-service purpose resolution and
  strict canonical Fleet-service target validation and bounded inert labels
  inherited across deployment/include/leaf paths, plus bounded reduction-only
  limits resolved against exact flattened members and immutable Component
  Spec ceilings and one schema-v1 semantic digest over the canonical group,
  flattened-deployment and Fleet-service-target sections. The managed runtime
  ABI now validates and retains an immutable ordinary-or-group-member context
  and exposes it to application policy. One bounded canonical fresh
  provisioning plan now binds the exact Registry version, active roots,
  release sets, placement IDs, complete flattened members and effective
  limits; the strict Fleet input and Registry also carry the immutable
  per-root group-placement ceiling. A selected root now validates and durably
  accepts its exact Coordinator-authenticated batch, retains permanent
  placement reservations and an exact restart-safe acceptance receipt, and
  fences unrelated allocation and ordinary removal while that aggregate
  authority is live. The root now advances that accepted batch one canonical
  placement member at a time through the existing root-local
  `ComponentInstanceId` allocator, retaining a hash-bound O(1) cursor and
  exact response-loss reconciliation with the Component Registry. After every
  identity is reserved, the same aggregate command claims one
  oldest-sufficient Ready prepaid Canister per canonical member through the
  ordinary Component creation journal and derives the exact protected
  `GroupMember` runtime context from the accepted plan. A second hash-bound
  O(1) cursor reconciles response loss without another claim. After every
  Canister is claimed, the root installs one canonical member at a time through
  the ordinary Store-backed install journal, retaining and independently
  observing its exact grouped context behind `Prepared`; a third hash-bound
  cursor reconciles response loss. After every member is installed, the root
  commits one exact `Prepared` Component Registry partition at a time through
  the ordinary commitment authority; a fourth hash-bound cursor reconciles
  response loss only after the allocation receipt, grouped Registry-byte
  ceiling and current Fleet Directory agree. After the fourth cursor becomes
  terminal, one local-only step re-reads every exact allocation and partition
  and freezes a group-partitioned `Provisioned` result containing each member's
  path, Spec, purpose, limits, binding and Registry head under one
  domain-separated terminal receipt. A focused PocketIC root proves the
  complete accept/reserve/claim/install/commit/provision/replay path against one
  canonical Project Hub group placement and the continued Directory/runtime
  fence afterward. A Coordinator-ready pure compiler now revalidates the
  complete plan and every exact root receipt, then derives canonical
  Authority/Replica and Active Pool bindings with mode, density, spread and
  global identity uniqueness checked before mutation. The canonical Fleet
  Registry now carries the complete service set under strict ordering,
  mode, placement, active-root admission and global identity validation, and
  the Coordinator has one closed atomic Registry-plus-receipt commit/replay
  primitive. The Coordinator now validates and durably freezes the complete
  fresh-install plan before effects, exposes compact exact status and replay,
  revalidates that authority after restart and fences grouped root lifecycle.
  It now journals and dispatches one exact canonical root acceptance at a
  time, reconciles response loss by exact root replay and retains each
  authenticated acceptance under the shared canonical receipt hash before
  reaching `RootsAccepted`. The host durably freezes the complete canonical
  compiled App provisioning configuration before Coordinator creation, and
  the standalone Coordinator persists and revalidates that exact authority
  without source TOML, embedded App config or runtime `ConfigOps`. The
  Coordinator now advances only completely accepted roots through their
  existing bounded journals, retains every authenticated terminal
  `Provisioned` receipt and recompiles the complete service topology before
  `ComponentsProvisioned`. A new exact command then derives that service set
  only from durable authority and atomically commits the canonical Fleet
  Registry snapshot, exact publication receipt and
  `ServiceTopologyPublished` operation state. The Coordinator now journals one
  exact selected-root publication call at a time. Each root synchronizes that
  published Registry, derives service-aware Fleet, Component and Component
  Group Directories, delivers and independently verifies every exact prepared
  Component authority, then freezes one terminal `Published` receipt. The
  Coordinator authenticates every receipt before reaching
  `DirectoriesConfirmed`. Each selected root now composes the existing
  runtime-activation and Registry-membership journals to activate one exact
  grouped Component per bounded command, verifies the resulting current
  Directory, and advances only after that Component is `Active`. After every
  planned Component is active, the root seals and revalidates initial
  inventory, prepares or resumes its Fleet runtime and freezes a terminal
  receipt chained to its prior `Published` receipt. The Coordinator persists
  one exact root call intent at a time, validates compact activation evidence
  in canonical root order and reaches `RuntimesActivated` only after every
  initial root is active. The host now persists every explicit initial
  placement-to-root-Subnet assignment before effects, resolves real root
  principals from the exact all-Active Registry and journals Coordinator
  preparation, every exact advance, terminal runtime evidence and catalog
  publication as one response-loss-safe transaction. Catalog publication is
  unreachable before `RuntimesActivated` and must match the exact published
  service Registry. Status cardinality is exact, replay accepts only the sole
  canonical one-revision service successor, initial atomic root batches fit
  their immutable pool minimum/import Ready target and every initial root has
  one canonical batch, including an empty batch when it owns no initial group
  placement. Newly installed root and Store runtimes are independently
  verified `Prepared`. The former direct empty-root host activation path is
  removed; empty roots cross the same Coordinator-owned Directory and runtime
  transaction instead.
  Once the final root reaches terminal runtime activation, the Coordinator
  atomically materializes one canonical deployment ledger from the protected
  configuration, fresh plan and exact terminal root receipts. Each deployment
  retains its placement IDs and root assignments, protected maximum and
  policy, and the next never-reused ordinal; the committed placement vector is
  the sole current-count authority. Earlier phases require no ledger, and
  terminal corrupt or incomplete ledger state fails closed. One monotonic
  scale-out plan may now reserve exact new placement ordinals, accept every
  selected active-root batch and advance the current root one canonical member
  at a time through the existing response-loss-safe Component identity
  journal. The root binds Active-runtime work to its exact sealed initial
  inventory. Once every identity is reserved, the same bounded journal may
  claim one exact prepaid Canister per canonical member through the existing
  pool authority. Once every Canister is claimed, it may install one canonical
  member at a time through the existing Store-backed lifecycle journal while
  retaining the exact grouped runtime context. Once every member is installed,
  it may commit one exact `Prepared` partition at a time through the existing
  Component Registry journal. A fully committed root may now freeze its exact
  group-partitioned terminal receipt; the Coordinator retains every selected
  root receipt in canonical order and reaches `ComponentsProvisioned` only
  after all are terminal. The Coordinator now derives the complete next
  Fleet-service set from that exact operation and its source Registry, then
  appends all Replica or PoolMember additions in one canonical Registry
  revision while rejecting Authority additions or any change to published
  members. Each publication retains an independent exact receipt, so initial
  and scale-out Registry history remains reconstructible across restart.
  Ordinary-only scale-out records the same `ServiceTopologyPublished` boundary
  without a Registry mutation. The Coordinator now drives the plan's exact
  selected plus affected-existing-root confirmation set in canonical order.
  Every root durably synchronizes the published Fleet Registry and its exact
  existing affected service Components before selected roots publish their
  prepared batch Directories. Root-local intent precedes each Component call,
  uncertain responses reconcile against independently observed runtime and
  Registry evidence, and a later Component head covers an in-flight required
  head only under the exact published Fleet Directory. The Coordinator reaches
  `DirectoriesConfirmed` only after every barrier root is terminal. It then
  journals activation only for selected roots. Each root persists whether the
  batch began on a fresh or already-active runtime; an active root activates
  only the new Components and reuses its sealed initial-inventory evidence
  without preparing or rewriting the root runtime. Exact terminal receipts
  reach `RuntimesActivated` and atomically append the new placements to the
  canonical deployment ledger. A later monotonic increase now atomically
  retires that terminal journal into bounded compact exact-replay history,
  installs the next validated journal and revalidates the complete deployment
  ledger from fresh authority plus every retired receipt. Historical exact
  prepare, status and terminal advance retry survive restart without retaining
  another complete plan; reused identities and corrupt history fail closed.
  Managed application endpoints now have one positive
  `deployment::is_service_authority(...)` guard backed by the fully validated
  current runtime and Directory authority. Only the exact matching Active
  Authority purpose passes; Replica, PoolMember and both ordinary forms fail.
  Every dynamically created descendant now receives its owning top-level
  Component's exact protected deployment context and applicable Component
  Group Directory at every lifecycle phase. Root reservation and later retry
  validation enforce the deployment's effective descendant, Registry-byte and
  per-parent spawn ceilings rather than falling back to the Component Spec
  maxima. Cross-root peer provisioning now accepts only the IC-authenticated
  raw caller that the target root derives as one exact top-level member of the
  expected Fleet service from its fully validated current Registry Mirror and
  matching Fleet Directory. It retains that remote Component identity,
  owning-root authority, Registry version and independently compiled
  requester-Spec-to-target-Spec grant in the ordinary allocation journal;
  every later lifecycle step revalidates the current member and exact grant.
  Same-root callers continue through local Component Registry proof, while a
  child, forwarded caller, wrong service, inactive root or caller-supplied
  identity cannot substitute for either proof. A focused Project hierarchy
  journey now uses application-owned durable operation identities to create a
  Project Instance under its Hub and role-qualified Ledger and optional
  Machine children under that Instance. Exact retries preserve the original
  Canisters, Directory rows preserve immediate parents, all descendants remain
  on the root's Subnet and the Coordinator Registry is unchanged. A focused
  canonical-plan proof now assigns two deployments of one Project Hub group to
  distinct root/Subnet bindings while retaining one shared Component Spec and
  Spec hash. The protected entries preserve the 10,000-instance Spec grant in
  one placement and the exact 2,000-instance reduction in the other;
  substituted limit authority rejects.
- Q4 qualification: the disposable three-application-Subnet Toko journey is
  complete. It provisions database A/B/C Authority and Replica members,
  differently limited reused Project Hub deployments, packed and spread
  ActivePool members, two interrupted scale-outs, three Project Instances,
  three Ledgers, one Machine and an isolated second Fleet sharing one physical
  Subnet. The exact supported envelope and Wasm measurements are recorded in
  [qualification report](../../audits/release-lines/supporting/0.101-fleet-authoritative-service-provisioning-and-publication/qualification.md).
- Release boundary: reinstall only.
- Implementation status: complete; Q5 closeout is released in `0.101.53`.
- Dependency: completed 0.100 qualified independently host-installed
  Coordinator/root/Store infrastructure, Fleet Subnet Root, Component Spec,
  root-local Component identity, topology-admitted sibling Wasm Store,
  prepaid-Canister inventory and Registry architecture, including flat
  potential-Wasm catalogs and multi-level dynamic Component trees plus
  separate runtime and Registry membership activation, revision-bound
  current-Directory convergence and inventory-bound Fleet Subnet Root runtime
  activation.
- Closeout gate: Q5 completed the whole-program hard cut, sediment removal,
  generated/configuration/stable-memory review and final responsibility/size
  accounting in the
  [closeout report](../../audits/release-lines/supporting/0.101-fleet-authoritative-service-provisioning-and-publication/closeout.md).
  Application-data replication
  remains a separate later design and is not an implementation blocker for
  0.101 topology, purpose or discovery contracts.

## Remaining Release-Batch Plan

The 0.101 line predates current
[delivery cadence governance](../../governance/delivery-cadence.md) and has
already exceeded the normal 6-10-release planning range. Historical tags stay
immutable. The remaining work is consolidated prospectively into five
outcome-based batches; individual checklist proofs, CI fallout and changelog
repairs do not create additional release boundaries.

| Batch | Bounded outcome and owner | Included direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| Q1 | Scale-out recovery and ActivePool qualification; Coordinator/root | Packed and spread membership, atomic publication, canonical Directory barrier, limit rejection, forced interruption/retry and no health/readiness claims | Coordinator state-machine tests plus focused root/PocketIC evidence where an IC call is required | Complete in released `0.101.49` |
| Q2 | Fleet isolation and physical-effect boundaries; Coordinator/root/host | Two co-located Fleets, independent Store installation/adoption, ordinary prepaid-pool claim/retry and first-excess frozen scale bounds | Focused host/root tests and the smallest required PocketIC journeys | Complete in released `0.101.50` |
| Q3 | Durable document and policy ownership; host/control plane/core | Cross-document backup/restore consistency, typed provisioning-journal decomposition, pure initial-placement policy and shared mechanical durable I/O | Focused schema/replay tests and package Clippy | Complete in released `0.101.51` |
| Q4 | Real topology and supported-envelope qualification; integration harness | Disposable Toko-shaped topology, cross-Subnet placement, dynamic descendants, shared-Subnet second Fleet and measured supported envelope | Focused multi-Subnet PocketIC/disposable-environment journey and [size/capacity report](../../audits/release-lines/supporting/0.101-fleet-authoritative-service-provisioning-and-publication/qualification.md) | Complete in released `0.101.52` |
| Q5 | Hard cut, sediment removal and closeout; whole program | Layer cleanup, obsolete surfaces, install execution/performance cleanup, Candid/generated/config/stable-memory residue, responsibility/size accounting and final closeout corrections | Targeted residue and installer guards followed by maintainer-owned complete release validation | Complete in released `0.101.53` |

`Q1` is complete in the released `0.101.49` batch. In addition to the packed and
spread Coordinator proof, a real two-application-Subnet PocketIC journey adds
the second Project Hub, restarts the Coordinator after every durable advance,
replays the same command without duplicate placement or membership and keeps
the Hub, its new Instance and that Instance's Ledger on the selected root's
Subnet. `Q2` is complete in the released `0.101.50` batch with direct co-
located-Fleet, Store-controller, pool-replay and first-excess evidence. `Q3`
is complete in released `0.101.51`: restored Coordinator state retains
one exact service/placement authority, the provisioning journal has separated
typed owners, placement policy is pure and the four host installation
journals share only mechanical durable-document operations. Individual fallout
inside a batch does not create another patch boundary. `Q4` is complete in the
released `0.101.52` batch: the three-Subnet Toko-shaped journey and shared-
Subnet second Fleet pass with exact restart/replay, protected 10,000/2,000
Project Hub limits and the measured envelope in
[qualification report](../../audits/release-lines/supporting/0.101-fleet-authoritative-service-provisioning-and-publication/qualification.md).
`Q5` is complete in released `0.101.53`. Its cleanup
reconciles current installation, recovery, layering and release-authority
documentation; archives the oversized session handoff; and removes CI guard
dependencies on incidental job counts, step adjacency and exact Make recipe
text. Fresh install now checks
live authority before compilation, retains its dedicated Cargo target for exact
retry, preserves the complete post-build safety gate and presents interactive
activity plus grouped padded summaries instead of unbounded receipt and finding
streams. Compatible configured roles now share Cargo workspace/profile builds
and workspace metadata/catalog validation while retaining package-selected role
graphs. The exact nine-role Toko fixture materializes all eleven configured and
infrastructure artifacts in 186.98s on its first fixture-specific fast build
and 60.54s on exact retained-cache retry. Network-mutating operator install
timing is not inferred from that artifact build; the Q4 PocketIC journey owns
the actual multi-Subnet runtime evidence.
The same cleanup audited rendered CLI help and reduced verbose install, App, build,
deploy and evidence after-help blocks to representative examples plus concise
scope or safety notes. The removed install prose no longer invents a whole-
Fleet update flow that the CLI does not expose. `canic build <app>` now exposes
the complete artifact build directly: it batches the configured Fleet Subnet
Root with every attached Component role once per Cargo workspace/profile, then
builds Fleet Coordinator and Wasm Store. Its output classifies Components as
application artifacts and all three platform canisters as infrastructure. The
optional role selector remains focused on configured custom builds and role-
scoped provenance. Retained-cache
measurement exposed Coordinator's unconditional debug-Candid build as 82.00s
of the 111.30s Infrastructure phase. Coordinator now joins Wasm Store in owning
a checked-in canonical DID, and ordinary builds copy and embed both contracts;
the repeated smoke reduces Infrastructure to 8.51s and the complete six-
artifact build from 147.15s to 44.94s. The closeout ownership pass has also
extracted Coordinator root-deletion entry operations, identity/retry lookup,
cycle-target and hash authority, and durable-history validation from the
accumulated Registry/provisioning module without changing its stable or public
contracts.
Canonical provisioning-plan byte encoding is now a separate focused owner from
semantic authority validation, with unchanged frozen plan and root-batch hash
vectors. The control-plane feature gate independently compiles the Coordinator,
Fleet Subnet Root and Wasm Store roles instead of inferring root isolation from
the combined host consumer. The final
[closeout report](../../audits/release-lines/supporting/0.101-fleet-authoritative-service-provisioning-and-publication/closeout.md)
accounts
for production growth, every remaining large owner, stable memory, generated
surfaces, permitted terminology and freshly executed focused evidence.

0.101 creates a fresh Fleet with composable compile-time Component Groups.
Nested group declarations flatten to direct Components under exact Fleet
Subnet Roots. Inclusion emits one occurrence per member path and does not
implicitly execute another deployment or deduplicate equal Specs. The same
Component Spec may provide one Fleet service Authority and several
cross-Subnet Replicas or several members of an ActivePool. Bounded
`FleetServiceId` is separate from role, and typed mode/member purpose is
protected independently of inert labels. One reusable service-bearing group
may receive an Authority assignment at its singleton deployment and a Replica
assignment where another group includes it.

Fresh installation provisions configured initial group placements and
gives each concrete copy a stable never-reused
`ComponentGroupPlacementId`, and atomically publishes every service's complete
mode-compatible member set. Placement policy can pack several copies of one
deployment on one root or spread them across roots, subject to per-deployment
density/spread and immutable aggregate root limits. After installation, an
authenticated administrator may monotonically add exact placements on
pre-installed, pre-admitted roots from the same fresh Fleet installation.
0.101 does not add a root after the initial all-root activation. Toko's
one-cell-per-root choice and maximum of ten are example policy values, not
protocol limits.

Separate deployment IDs may reference the same Component Group while applying
different reduction-only limits to exact flattened member paths. The
Component Spec remains the absolute envelope. A deployment may narrow
`maximum_descendants`, `maximum_registry_bytes` and exact role-to-role
spawn-grant ceilings, but may not raise them, add grants or replace component
configuration. Every placement and later scale-out of one deployment inherits
the same protected effective limits. This permits one Project Hub deployment
with a 10,000-instance Hub-to-Instance grant and another on a different root
and physical Subnet with a 2,000-instance effective grant without duplicating
its group, Spec, role or Wasm.

The design now also carries the corrected high-cardinality Toko path. Every
project-data-cell placement contains one Project Hub PoolMember beside the
database Replicas. The Hub asks its own root to create Project Instance direct
children, and a Project Instance asks the same root to create its Ledger and
optional Machine children. Every child binding records the exact immediate
parent while the root remains sole lifecycle authority. The Coordinator is not
on this path. The Hub's `project_id -> ComponentChildBinding` map is
an application-owned Placement Index that agrees with, but does not replace,
protected Canic parentage or the root-derived Component Directory. The Hub and
Project Instance use distinct explicit spawn grants for their respective
child roles.

0.101 also closes 0.100's deferred cross-root peer-Component requester proof.
The target root derives one exact top-level Fleet-service member from the
IC-authenticated raw caller and its current Fleet Registry Mirror, requires
the matching Fleet Directory projection, then independently enforces the
compiled requester-Spec-to-target-Spec Component Provisioning Grant.
Membership proves identity, not permission. Children, ungrouped Components,
forwarded callers and caller-supplied bindings cannot use this path.

0.101 does not consume a 0.100 installation, preserve existing Canisters,
synchronize application data, choose load-balancer health, scale in, promote a
Replica or create roots during scale-out. Grouped Components and their roots
remain fenced from the ordinary 0.100 removal paths while placement or service
references exist.

Fresh 0.101 installation inherits the 0.100 infrastructure manifest,
independent host installation of the Coordinator, every root and every sibling
Store, reciprocal root/Store verification and sole-root Store adoption.
Component Group placement does not change that installation ownership and
reuses the ordinary prepaid-Canister claim plus the root-owned 0.100 Cycles
Ledger refill. A Component request cannot create a physical Canister or use a
paid fallback when no `Ready` asset exists; root maintenance independently
restores the configured pool minimum.

The current complete Fleet-service member vectors and affected-root
confirmation barrier are retained only for the measured initial envelope. A
later reinstall-only large-Fleet design may hard-cut them to versioned
partitions and proof-carrying root-local projections, optionally distributed
through bounded Coordinator Workers, while keeping the Coordinator the sole
Fleet policy writer.

## Slice 1 — Composition and Purpose Contracts

- [x] Add bounded `ComponentGroupSpecId`, `ComponentGroupDeploymentId`,
  `ComponentGroupPlacementId`, `FleetServiceId`, member IDs and canonical
  member paths.
- [x] Compile nested Component Group declarations as a bounded acyclic graph.
- [x] Flatten every deployment completely before planning.
- [x] Preserve each distinct member-path occurrence and distinguish inclusion
  from independent deployment.
- [x] Resolve every Fleet-service leaf through exactly one typed purpose
  assignment on its deployment/include/leaf path.
- [x] Reject unused purpose assignments and orphan service occurrences or
  targets.
- [x] Add typed `Ordinary` and `FleetServiceMember` purpose with Authority,
  Replica and PoolMember variants.
- [x] Validate AuthorityReplica and ActivePool target/member invariants.
- [x] Validate service-wide member density/spread independently of deployment
  placement policy, including concrete fresh-plan and selected-root
  assignments.
- [x] Add bounded inert deployment labels that cannot alter authority.
- [x] Compile bounded reduction-only deployment-member limits against exact
  flattened paths and immutable Component Spec envelopes.
- [x] Add the reinstall-only protected Component deployment runtime contract,
  hard-cut managed init/status to retain it, validate exact compiled grouped
  projections and expose the retained purpose to application policy.
- [x] Derive each `GroupMember` context from the accepted root plan, verify it
  against the Component Group Directory and enforce its exact effective limits
  throughout root allocation and descendant lifecycle.
- [x] Derive one semantic protected configuration digest over groups,
  deployments and service targets.
- [x] Remove singleton-Spec and sole-root-admission service assumptions.
- [x] Validate worst-case Spec demand, placement density/spread and the
  zero-placement/non-Authority versus singleton-Authority count rules.
- [x] Measure and freeze the initial supported root, Component, placement,
  service-member, plan, Registry and Directory envelope. The Q4 report records
  the executed topology and canonical byte sizes while retaining the existing
  first-excess protocol ceilings.

## Slice 2 — Root Plans and Provisioning

- [x] Freeze one canonical root/group-placement/member plan shape.
- [x] Require strict Fleet input to assign every initial placement ordinal to
  one explicit root Subnet, persist the complete canonical assignment before
  effects and resolve only the exact live Registry root principal/binding.
- [x] Carry every member's canonical effective limits through plan hashing,
  root acceptance, protected runtime context, descendant reservations and
  durable receipts.
- [x] Reserve monotonically increasing, never-reused placement ordinals before
  root calls.
- [x] Bind every placement to one exact eligible Fleet-owned root while
  permitting repeated roots within placement policy.
- [x] Require every flattened Spec in that root's immutable admissions,
  Component Topology, active release set and Wasm Store Catalog before durable
  acceptance.
- [x] Enforce each root's immutable aggregate group-placement ceiling across
  accepted and provisioned state. Permanent exact-retry-safe reservations count
  once and the terminal transition preserves that same accounting authority.
- [x] Extend the complete current root-limit contract without dropping
  Registry, Store, prepaid-pool or cycles-funding authority.
- [x] Reuse canonical root-local `ComponentInstanceId` allocation,
  prepaid-Canister claim and platform lifecycle, failing closed when no Ready
  imported, recycled or automatically created asset exists. The grouped root
  journey uses the ordinary identity, claim, Store-backed install, Registry
  commit, Directory and runtime-activation journals through terminal replay.
- [x] Reuse 0.100's bounded root-owned Cycles Ledger refill and permanent
  uncertain-expiry fence; keep raw management `create_canister` and Component
  paid fallback absent. Grouped provisioning consumes only Ready pool assets;
  root maintenance remains the single automatic refill authority.
- Deferred beyond 0.101: authenticated minimum-balance/fixed-top-up policy and
  bounded overfunding warnings are owned by the accepted 0.114 Canister-estate
  design rather than this topology release.
- Deferred beyond 0.101: any non-IC ledger funding design must separately prove
  exact-Subnet, guaranteed-response and expired-uncertainty fences before it
  can become a maintained funding path.
- [x] Derive a cross-root top-level requester from the raw caller's exact
  current Fleet Registry service binding and matching Fleet Directory, then
  independently require the compiled peer-Component grant.
- [x] Accept same-root child requests from any exact registered
  Component-tree node through an exact role-to-role spawn grant and without a
  Coordinator operation.
- [x] Bind every descendant to its exact immediate parent while retaining the
  owning top-level Component binding.
- [x] Keep new Components runtime `Prepared`.
- [x] Persist group-partitioned Component Registry evidence and one aggregate
  idempotent root receipt.
- [x] Dispatch each persisted root batch from the Coordinator only after an
  exact pre-call intent, reconcile response loss by exact retry and retain the
  authenticated canonical `Accepted` receipt in plan order.
- [x] Advance every accepted root through its existing bounded root-local
  cursors under one durable Coordinator pre-call intent, retain exact terminal
  `Provisioned` receipts in canonical plan order and recompile the complete
  service topology before `ComponentsProvisioned`.

## Slice 3 — Service Topology and Directories

- [x] Resolve exactly one Authority plus zero or more same-Spec Replicas for
  AuthorityReplica, and one or more same-Spec Pool members for ActivePool. The
  pure compiler consumes every exact root `Provisioned` receipt, canonicalizes
  the complete member set and rejects missing, substituted, nonterminal,
  duplicate or placement-policy-invalid evidence before Registry mutation.
- [x] Publish each service's complete initial mode-compatible member set in one
  Fleet Registry revision. The Coordinator derives the complete set only from
  its durable plan and canonical terminal root receipts, then commits the
  Registry snapshot, exact publication receipt and
  `ServiceTopologyPublished` operation state atomically. A service-free plan
  records the boundary without inventing a Registry revision. Exact retry and
  restart reconstruct the committed result, while stale final-root commands
  cannot initiate publication.
- [x] Project exact service ID, mode and purpose-bearing member bindings
  through Fleet Directory.
- [x] Derive one root-local Component Group Directory per placement without
  introducing group parentage or lifecycle authority.
- [x] Send exact Fleet, Component and Group Directories before activation.
- [x] Activate each Component runtime under its exact prepared Directory,
  then promote its Component Registry partition to `Active` and synchronize
  the resulting revision-bound current Directory before sealing initial
  inventory and activating the root. The Coordinator reaches
  `RuntimesActivated` only after every selected root returns exact terminal
  evidence.
- [x] Drive fresh installation from the host through one durable
  prepare/advance transaction, reconcile uncertain updates through passive
  status, and publish the terminal Fleet catalog only after exact
  `RuntimesActivated` and published-Registry evidence.
- [x] Freeze the exact Directory-confirmation roots: all initial roots for
  fresh install, and selected plus every affected existing service-member root
  for scale-out. Fresh-install confirmation verifies every initial root from
  the canonical plan. Scale-out now synchronizes the published Fleet Registry
  and every existing affected service Component on each exact barrier root,
  then publishes the prepared batch on selected roots before the Coordinator
  reaches `DirectoriesConfirmed`.
- [x] Require Replica purpose to fail application database write-authority
  checks.
- [x] Require PoolMember purpose to grant no implicit leadership, health or
  consistency.
- [x] Preserve that write fence for service-sensitive descendants through
  their exact owning top-level Component without making descendants group
  members or Fleet services.

## Slice 4 — Explicit Group Scale-Out

- [x] Persist each deployment's placement IDs, exact root assignments,
  protected maximum and next ordinal. The canonical placement vector is the
  sole current-count authority. It is materialized atomically with terminal
  fresh-install runtime evidence and rederived during stable-state
  validation; scale-out reservation and append remain separate later steps.
- [x] Accept only monotonic desired-count increases with exact unused
  placement IDs on active roots within density, spread and aggregate limits.
  The Coordinator now freezes and atomically reserves the exact increase,
  checks all configuration-derived root capacity and advances only those
  reserved batches through its existing response-loss-safe root-acceptance
  journal. Each selected active root rechecks its exact Mirror, protected
  configuration and release set, Store artifacts, Component/group capacity
  and Ready pool capacity before accepting. The current accepted root may now
  reserve each planned Component identity and then claim one exact prepaid
  Canister per canonical member. Fully claimed members may then install through
  the exact Store-backed journal, then commit one canonical `Prepared`
  partition through the root-local Component Registry. Every selected root may
  now freeze its exact terminal receipt, and the Coordinator advances only
  after retaining those receipts in canonical order. The Coordinator now
  publishes the complete service additions atomically, completes the exact
  selected-plus-affected Directory barrier, activates only the selected new
  batches and appends their placements under terminal root receipts. A later
  increase atomically retires the completed journal into bounded compact
  history before installing the next validated plan; stable validation
  reconstructs the deployment ledger from every exact receipt.
- [x] Require every eligible scale-out root to belong to the complete root set
  installed and activated by the same fresh Fleet installation.
- [x] Enforce each affected service's complete member density/spread policy
  after every addition.
- [x] Provision only new placements and retain exact retry identity.
  The protected plan contains only the new placements and exact preparation
  retry is durable. Root acceptance now retains exact pre-call intents and
  authenticated receipts for only those placements. The Coordinator now
  journals each current-root advance and the root allocates one canonical
  member identity at a time, retaining the exact allocation across response
  loss and restart. Once every identity is reserved, the root uses its existing
  pool claim journal to retain the exact Canister principal across response
  loss. Each fully claimed member may then install with its exact grouped
  context and replay the same Store-backed operation after interruption.
  Fully installed members may then commit their exact Registry partition with
  response-loss replay. Each fully committed root now freezes a terminal result
  without releasing its aggregate runtime fence, and the Coordinator reaches
  `ComponentsProvisioned` only after every selected root is terminal. All
  Replica or PoolMember additions then publish in one exact Registry revision;
  ordinary-only operations retain the same receipt boundary without a Registry
  mutation. The exact selected plus affected-existing-root Directory barrier
  now reaches `DirectoriesConfirmed`. Selected roots then activate only their
  new batch while retaining pre-existing root activation and sealed inventory;
  terminal receipts atomically commit the new placement vector. Repeated
  scale-out now retains compact terminal prepare/status/advance replay across
  restart without preserving another full plan, rejects operation-ID reuse and
  begins the next placement range only in the same atomic rollover commit.
- [x] Append all Replica and PoolMember bindings from one scale operation
  atomically. The complete target set is compiled from the exact source
  Registry and terminal selected-root receipts, existing authority and members
  are immutable, and exact restart replay retains every publication receipt.
- [x] Fence grouped Components and their roots from ordinary drain/removal.
  The Component Registry operation boundary rejects grouped Components before
  ordinary draining and rejects any persisted grouped ordinary-draining state.
  The Coordinator now fences only a root named by a nonempty operation batch,
  committed placement or Fleet-service binding, leaving unrelated empty roots
  lifecycle-open. The accepted design now freezes one nonexpiring,
  noncancellable Coordinator reservation as the atomic winner against plan
  preparation. Coordinator prepare/status persistence, domain-separated hash
  validation, exact retry and both plan/reservation orderings are implemented.
  The root independently fetches the retained reservation from its protected
  Coordinator, accepts later unrelated Registry revisions only while the
  exact Active target row remains unchanged, rechecks local grouped authority
  after the call and durably binds the canonical reservation hash before its
  one-way fence. The Coordinator requires that same retained hash and current
  target row at publication. Local and publication exact retry survive
  response loss and later revisions. Qualified grouped removal remains a later
  design, not a 0.101 operation.
- [x] Reject scale-down, placement reuse, Authority-group scaling, live root
  creation and admission expansion.

## Slice 5 — Recovery and Qualification

- [x] Exercise deterministic Coordinator interruption boundaries for fresh
  provisioning and scale-out. Fresh root acceptance/provisioning already
  reconciles exact lost-response intent. Scale-out now reloads durable state
  with affected-root synchronization, selected-root synchronization,
  selected-root publication and runtime activation each in flight, then
  requires the exact typed reconcile call before recording the response.
- [x] Prove backup/restore cross-document consistency. A complete packed and
  spread ActivePool snapshot is cleared, restored and revalidated with its
  exact Fleet Registry service members, placement IDs/root mappings, terminal
  replay evidence and next ordinal. Independent next-ordinal and Registry-
  member substitutions fail closed without mutating restored state.
- [x] Prove Component Topology, group flattening, admission,
  active-release-set, Wasm Store, effective member limits, placement,
  service-mode, purpose, label and authority boundaries. Focused compiler,
  canonical-plan and Store-backed root journeys cover the individual
  rejection boundaries; the Q4 topology qualification composes them through
  exact protected runtime authority.
- [x] Prove several placements of one deployment may share a root without
  identity or Component Group Directory ambiguity. One durable two-placement
  root journey now retains distinct member-operation, Component, Canister,
  placement-provenance and placement-receipt identities through Registry
  commitment and Directory derivation; reordered placement evidence rejects.
- [x] Prove the durable Coordinator path can atomically add one same-Spec
  ActivePool member while retaining packed and spread placements. The initial
  two members span two roots; ordinal 2 packs onto the first root, both roots
  cross the canonical Directory barrier, the selected runtime activates, the
  placement ledger settles and exact terminal status replays after restart.
  The next placement on that full root rejects before durable mutation.
- [x] Prove two Fleets remain isolated when their roots share one physical
  Subnet. A focused PocketIC journey installs two Coordinators, roots, sibling
  Stores and disjoint root-owned prepaid inventories on one application
  Subnet. Each Store ends root-controller-only; foreign Store mutation and
  foreign-Fleet Registry synchronization reject while both independent
  Registry joins succeed.
- [x] Prove cross-root peer provisioning requires the exact current raw-caller
  Registry member, matching Directory projection and independent
  requester-Spec-to-target-Spec grant, while stale, forwarded, child and
  caller-supplied identities reject.
- [x] Prove independent host Store installation/adoption and ordinary
  prepaid-Canister claim/retry remain the sole infrastructure and Component
  effect paths used by grouped provisioning. Host journals retain separate
  root and Store creation/install/verification tracks plus the exact adoption
  receipt; live adoption observes the final root-only controller set. Grouped
  provisioning delegates to the ordinary root-local claim/install journals,
  and exact claim replay returns the same sole workload asset.
- [x] Prove the first excess value for every currently frozen initial scale
  bound rejects before mutation or network effects. Exact tests cover Fleet
  Component maxima, group graph/declaration/inclusion/flattening bounds,
  deployment/member/reduction and service-target bounds, plan root/
  confirmation/placement/entry counts, placement envelopes and protected
  initial root Component/group-placement capacity. Q4 still owns measurement
  and freezing of the supported Registry/Directory/Toko operating envelope;
  those values are not invented by Q2.
- [x] Prove configured Replica discovery never claims data readiness,
  promotion or failover and configured PoolMember discovery never claims
  health, load-balancer eligibility or consistency. Fleet Registry and Fleet
  Directory service DTOs carry only exhaustively checked configured topology;
  application policy owns every runtime observation and decision.
- [x] Prove local Project Hub -> Project Instance -> Ledger/Machine
  provisioning, exact immediate-parent bindings and Coordinator-free retry.
- [x] Prove two deployments reuse one Project Hub group with distinct
  reduction-only 10,000/2,000 Hub-to-Instance ceilings on different roots.
- [x] Decompose the fresh-install Component provisioning journal into
  separately readable typed transition/schema, authority validation and
  durable-document persistence owners. The maintained schema version, pretty-
  JSON-plus-newline encoding, file and lock names, byte bound and exact retry
  behavior remain unchanged.
- [x] Move initial Component Group placement compilation and semantic policy
  out of generic Fleet-install-plan persistence into one focused pure owner.
  Plan persistence now maps its typed policy rejection without owning density,
  spread, admission, pool or root-capacity rules.
- [x] Inventory every host installation journal and consolidate mechanically
  identical no-follow read, lock, canonical encode, atomic publication and
  exact-retry plumbing behind a narrow durable-I/O owner while retaining
  distinct typed domain state machines. Coordinator, root-install, Registry-
  activation and Component-provisioning journals now reuse bounded canonical
  encoding, no-follow reads and exact replacement reconciliation; their locks,
  paths, schemas, authority checks, create conflict rules and phase machines
  remain domain-owned.
- [x] Reconcile current installation, recovery, identity, layering and release-
  authority documentation with maintained behavior. The compact handoff now
  carries only current state, stale 0.100 install boundaries and removed CLI
  examples are gone, and CI documentation guards reject the superseded claims
  instead of requiring them.
- [x] Give both built-in infrastructure source crates checked-in canonical
  Candid contracts. Ordinary Coordinator and Wasm Store builds copy and embed
  those contracts without debug recompilation; explicit refresh regenerates
  either DID, missing canonical contracts fail closed, and generated fallback
  wrappers extract without treating ignored sidecars as source truth.
- [x] Audit every 0.101-touched module for mixed layer ownership, long or mixed
  authority predicates, forwarding namespaces, dead re-exports, unused
  dependencies and completed implementation scaffolding; split or delete each
  confirmed concentration.
- [x] Hard-cut every obsolete phase, field, endpoint, protocol adapter, stable
  record/index, feature edge, fixture, example and active-document reference;
  do not retain aliases, fallback decoders, migration or compatibility paths.
- [x] Recheck Candid, generated surfaces, CLI/configuration guidance, stable-
  memory ownership and Wasm dependency isolation after removal.
- [x] Publish the final stale-occurrence inventory and responsibility/size
  diff, with an evidence-backed explanation for every remaining large module
  or mechanically duplicated path that cannot safely be reduced.
- [x] Complete all mandatory sediment-removal and design closeout checks.

## Completion

- [x] The Toko journey provisions database A, B and C Authorities on one root.
- [x] The same database group is reused inside a nested project-data-cell
  group to provision same-Spec database A, B and C Replicas plus one Project
  Hub PoolMember on at least two other roots.
- [x] One project-data-cell scale-out resumes exactly across forced
  interruption. The three-Subnet qualification journey restarts the
  Coordinator after every durable advance; exact replay retains the new cell,
  its service members and every later local descendant on the selected root's
  Subnet.
- [x] The local Project Hubs provision at least three Project Instance
  children across their application roots; every Project Instance creates one
  Ledger and exactly one creates its optional Machine.
- [x] Two deployments reuse one Project Hub group on different roots with
  distinct protected effective spawn-grant ceilings and no duplicated Spec.
- [x] A same-Spec ActivePool packs multiple stable placements on one root,
  spans at least two roots and publishes one atomic scale-out addition in the
  real disposable multi-Subnet journey.
- [x] The initial supported Fleet/service envelope is measured and does not
  claim ten-thousand-Subnet qualification.
- [x] All design criteria and required journeys pass.
- [x] No Tree identity, runtime Group Canister, nested Component declaration,
  Component Child group/service target, delegated lifecycle authority,
  singleton-Spec restriction, application/prior-release adoption,
  prior-release transition or compatibility path survives. Fresh-install
  adoption of the separately installed sibling Wasm Store remains the required
  0.100 infrastructure-controller transition.
- [x] The mandatory cleanup report accounts for production growth, deleted
  authority paths, module ownership, durable-I/O consolidation, generated
  surfaces, stable allocations and every permitted historical-only residue.
- [x] Current status and changelog record the final evidence.
