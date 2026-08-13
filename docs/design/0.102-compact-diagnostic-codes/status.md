# Canic 0.102 Implementation Status

Date: 2026-08-13

## Status

- State: evidence-only baseline and diagnostic inventory are active. No public
  error shape, numeric assignment, stable record or runtime behavior has
  changed.
- Evidence baseline: clean `main` tag `v0.101.53` at
  `23c0328f78b215580d734ef01b52b35fa3e38ade`. The active constructor
  reconciliation is pinned separately to current-candidate control-plane/core
  source content at `0750c309104b111fa6f5a1b3355c04fcb38faf71`.
- Release boundary: 0.102 is reinstall-only and is not rolling-compatible with
  pre-0.102. Every Canic-owned canister in a Fleet must be installed from one
  admitted release set before activation; matching host/CLI callers and
  regenerated external bindings move with that boundary. Same-release retry,
  backup, restore and interruption recovery remain required.
- Design gate: direction is approved and evidence-only B1 work may continue.
  Mutating batches B2-B6 remain blocked until the complete producer,
  dynamic-public-context and durable-state inventories plus the initial
  allocation ledger, host catalogue and projection table receive maintainer
  approval.
- Release checkpoint: at the maintainer's explicit request, `0.102.0` closes
  the completed operator-performance and CLI-diagnostic outcomes together with
  this reviewable B1 evidence snapshot. B1 remains active after the checkpoint;
  no incomplete diagnostic behavior is represented as released.
- Measurement gate: the fresh current-source canonical Wasm baseline passes;
  every later material cut and closeout must remain comparable with v3.

## Release-Batch Plan

Implementation slices are smaller than these release batches. Compiler fallout,
fixture propagation, documentation, generated surfaces and changelog maintenance
remain part of the batch whose behavior caused them.

| Batch | Bounded outcome and owner | Included direct evidence and fallout | Focused validation | Surface impact | Status |
| --- | --- | --- | --- | --- | --- |
| B1 | Current diagnostic authority and Wasm baseline; whole program | Exact public/internal producer inventory, dynamic public-message ownership, durable-string classification, operation-correlated masked-code owners, proposed current/retired allocation ledger and representative canonical Wasm measurements | Inventory consistency checks, current-source scans and `CANIC-WASM-001` or an explicitly qualified successor | Evidence only; no runtime or wire change | Active |
| B2 | Prose-free runtime identity and host catalogue; `canic-core`, `canic-host`, `canic-cli` | Distinct raw and registered code types, approved current allocations, permanent current/retired ledger, exhaustive host metadata with typed disposition, compact formatting, CLI lookup, generated language-neutral current registry and release-Wasm absence proof | Targeted core/host/CLI tests, current/retired allocation and catalogue bijection tests, direct-construction guards and role-scoped Wasm inspection | Adds the current diagnostic lookup surface; public endpoint error wire remains unchanged | Pending |
| B3 | Fleet-atomic public diagnostic hard cut; runtime facade, Candid, host and test owners | Replace the public enum-plus-message record with one `nat16`, update every owned endpoint, canonical/generated declaration, decoder, helper and fixture, delete the old shape without aliases, and require one admitted release set before Fleet activation | Exact Candid-shape guards, representative endpoint decode tests, mixed-release activation rejection, host failure rendering and focused Wasm remeasurement | Breaking public error contract; no mixed pre/post-0.102 Fleet operation | Pending |
| B4 | Code-first internal propagation and security projection; `canic-core` and control plane | Remove owned diagnostic prose/context concatenation, exhaustively map typed causes, retain explicit internal/public identities, prove masked-code observability and remove canister-only derived prose | Targeted typed mapping, projection, authorization and retry-decision tests plus subsystem Wasm measurements | Internal runtime representation and operator diagnostics | Pending |
| B5 | Bounded durable diagnostic ownership; stable model and lifecycle owners | Remove redundant prose, deterministically clear proven advisory state, preserve bounded owned operational text, and add a direct typed replacement only when current recovery behavior remains unchanged; defer broader state-machine redesign and retain its owned text | Record-level canonical encoding tests and the smallest lifecycle/PocketIC recovery journeys required by fields actually changed | Current stable schema only; reinstall boundary unchanged | Pending |
| B6 | Whole-program hard cut and measured closeout; all owners | Runtime/Candid/generated/config/doc residue removal, final canonical measurements, downstream source-change guide, responsibility audit and deletion of temporary inventory tooling not admitted as recurring authority | Targeted residue and governance guards followed by maintainer-owned full release validation | One maintained compact diagnostic protocol | Pending |

Six batches match the design's actual dependency boundaries: evidence and
allocation authority must precede scaffolding; scaffolding must precede the
atomic public cut; internal and stable ownership then hard-cut independently;
the final batch proves the complete absence and footprint properties.

## Current Batch: B1

B1 is complete only when all of the following are reviewable together:

1. the initial `v0.101.53` public-error, internal-conversion and host-consumer
   inventories are reconciled against one exact current release-candidate
   source snapshot;
2. every dynamic public-message interpolation is classified as
   caller-derivable, sensitive/operator-only, authoritatively typed or
   caller-required but unowned, and every unowned required value has a proposed
   endpoint-specific typed owner;
3. every durable diagnostic-derived string is classified as redundant prose,
   advisory context, recovery-significant state or owned operational text;
4. every proposed leaf has one meaning, class, origin, typed host disposition,
   public projection and, where masked, a retrievable operation-correlated
   numeric observability owner;
5. every proposed number is nonzero, unique across the permanent current and
   retired allocation ledger and backed by a current producer when active;
6. the current host catalogue and generated language-neutral registry are
   bijective with the proposed current allocation rows;
7. representative leaf, Fleet Subnet Root and Wasm Store artifacts have a
   reproducible current-source baseline; and
8. the maintainer can approve or amend the complete inventories and initial
   allocation before B2 makes them authoritative.

## Current Evidence

- The public `Error` still contains `ErrorCode` and `String`; the current enum
  has 20 maintained leaves, not the design's historical 25-leaf estimate.
- `InternalError` still owns class, origin, `String` and an optional complete
  public error, and `with_diagnostic_context` still concatenates prose.
- Canonical infrastructure Candid still exposes the enum-plus-message shape.
- Host protocol rejection matching still consumes `ErrorCode` directly.
- `CANIC-WASM-001/v3` passes at immutable tag `v0.101.53` with the complete six-
  Component plus three-infrastructure release/debug roster and risk `5/10`.
  Historical v2 evidence remains valid superseded history and is not comparable
  with v3. Exact measurements and the preceding four-role development check are
  recorded in [inventory.md](inventory.md).
- [allocation-proposal.md](allocation-proposal.md) recommends dense monotonic
  codes, unpadded `E<decimal>` rendering and nine host-only broad classes. These
  choices and the later complete leaf table remain maintainer review gates.
- [code-allocation-ledger.md](code-allocation-ledger.md) now freezes the
  repository-only permanent current/retired ledger contract and the contract
  for a generated language-neutral current registry. No allocation or generated
  registry exists yet, so both ledger sets remain empty until B1 approval.
- The normative design now distinguishes lossless raw decoded identities from
  registered producer identities, makes typed host disposition authoritative
  for host automation, requires operation-correlated evidence for masked
  errors, and defines the one-release-set Fleet activation boundary. These are
  design constraints, not implementation claims.
- [public-boundary.md](public-boundary.md) proves 151 explicit public
  constructions in 26 production files and records every current
  code-dependent machine decision. In particular, broad `Forbidden`,
  `Conflict`, `ResourceExhausted` and `Unavailable` matches require semantic
  splits before the compact cut.
- [dynamic-public-context.md](dynamic-public-context.md) now classifies the
  first 138 individual values across the Canic memory-ledger facade, Wasm Store
  GC, shared manifest/capacity conversion, explicit Component Registry denials,
  typed Store publication causes, delegated-session bootstrap and Store
  publication binding/inventory plus Store GC fence, reclamation, binding
  finalization and deletion plus the two publication management transports.
  Sixty-six are caller-derivable, sixteen are sensitive operator-only,
  thirty-one already have exact typed owners and twenty-five are caller-
  required but unowned. They require request-scoped Store capacity/release
  inspection,
  guarded delegated-session capacity status, exact closed-discriminator
  diagnostics, root-proxied live GC inspection or operation-scoped Store
  deletion/publication-attempt progress. Component RPC and Runtime Introspection
  have zero-row closures for explicit dynamic public-error constructions. Ten
  additional
  runtime-template values are explicitly excluded from the public-context count
  because their registered resolver has no current in-repository caller; they
  remain constructor-frontier sites rather than disappearing from coverage.
  Every dynamic publication GC invalid-state field and nested publication
  transport cause is now classified; their static invariant/cause branches
  remain allocation work. The transitive auth formatter remains an explicit
  open subfrontier.
- [conversion-context.md](conversion-context.md) classifies all 39 explicit
  typed conversions and all 35 production context-appending calls. Twelve
  terminal conversion owners currently lose their transitive typed causes to
  one formatted broad error and are the next allocation input.
- [component-policy-leaves.md](component-policy-leaves.md) assigns provisional
  semantics to all 34 live Component policy variants, identifies four safe
  projection leaves and excludes one unproduced error variant plus four unread
  input fields as sediment to delete rather than number.
- [transitive-error-inventory.md](transitive-error-inventory.md) follows all 12
  terminal flattening roots to a union of 54 Canic-owned typed owners and 514
  declared variants. It separates transparent wrappers from generic string
  buckets, stringified typed forwarding and dependency-owned error boundaries;
  these are an allocation perimeter, not 514 proposed codes.
- [configuration-leaves.md](configuration-leaves.md) separates native-only
  TOML/schema authoring errors from the runtime path. Exact-path analysis cuts
  46 of its initial 132 internal candidates, leaving 86 producer-reachable
  candidates plus three safe projections for semantic grouping. An exposed
  enum variant alone is not allocation evidence.
- [auth-policy-leaves.md](auth-policy-leaves.md) and
  [auth-string-frontier.md](auth-string-frontier.md) map 132 provisional exact
  authentication/policy candidates and six safe projections. The expanded stop
  contains ten additional typed owners, 96 non-test structural variants and 43
  direct prose construction sites, but wrapper reuse and current-path sediment
  reduce it to 84 additions. The durable chain-key signing state currently
  collapses retryable management failure with terminal protected-policy
  failure; its typed disposition and numeric durable owner are required in
  B4/B5.
- [ic-infrastructure-leaves.md](ic-infrastructure-leaves.md) maps the complete
  seven-owner IC infrastructure graph plus pinned call, rejection, signing-cost
  and Candid dependency boundaries to 24 provisional exact leaves and four safe
  projections. Destination-invalid absence remains a typed pre-projection fact.
- [bounded-runtime-leaves.md](bounded-runtime-leaves.md) maps 37 exact topology,
  runtime-log, refill, Placement Index and current Cashier candidates plus four
  safe projections. The four Cashier leaves are allocated in 0.102 and their
  numbers retire without reuse when 0.107 removes the producers.
- [cost-guard-leaves.md](cost-guard-leaves.md) maps seven exact reservation
  leaves and one safe projection, deletes the redundant public-kind classifier
  and keeps rollback failure as a secondary typed observation rather than a
  combined diagnostic.
- [access-leaves.md](access-leaves.md) splits 28 production denial construction
  sites into 20 new exact candidates, two safe projections and six explicit
  reuses of existing auth semantics. Typed verifier, configuration and Registry
  failures survive masking instead of becoming foreign-caller denials.
- [runtime-ops-leaves.md](runtime-ops-leaves.md) maps configuration lookup,
  protected deployment validation, environment access and request/RPC wrapper
  ownership to 18 new exact candidates and two safe projections. Required-field
  and root-access meanings are reused rather than duplicated.
- [fleet-activation-leaves.md](fleet-activation-leaves.md) maps fresh activation
  admission and the protected activation record to 30 exact candidates and one
  safe projection. Its free-form record/transition reasons are grouped only
  where owner, action and retry policy agree.
- [storage-registry-leaves.md](storage-registry-leaves.md) maps ICP-refill,
  Placement Index and Sharding record failures to 18 exact candidates and two
  safe projections; the aggregate storage wrapper receives no code.
- [fleet-control-plane-leaves.md](fleet-control-plane-leaves.md) maps the Fleet
  Registry, provisioning-plan, Fleet-service binding and shared receipt-hash
  owners to 124 exact candidates and one safe projection. Four formatted
  aggregate variants become typed cause edges rather than codes.
- [intent-store-leaves.md](intent-store-leaves.md) maps all 51 live durable
  intent variants and one safe state projection, separating actionable
  request/state conditions from masked index and metadata contradictions.
- [memory-adapter-leaves.md](memory-adapter-leaves.md) pins the adapter to
  `ic-memory 0.12.3` and its checksum, groups 131 reachable known structural
  leaves into 54 Canic-owned semantics and adds 20 boundary-specific unknown
  leaves for the reachable non-exhaustive enums.
- [ledger-reconciliation.md](ledger-reconciliation.md) reconciles every family
  subtotal and cross-family reuse. A broad census finds 689 uppercase tokens;
  four are documented notation or forbidden/unreachable examples, leaving the
  expected 685 collision-free proposed identities.
- [projection-ledger.md](projection-ledger.md) aggregates all 31 currently qualified additional
  safe projections and five exact leaves reused as projection targets. It
  names proposed numeric observation owners and leaves IC effect call-site
  ownership and the current string-coded recent-failure ring as explicit
  approval gates. Cashier uses that guarded numeric owner until 0.107 retires
  its codes.
- [ic-observability-owners.md](ic-observability-owners.md) maps 17 current IC
  call families to their operation authority or guarded runtime status and
  identifies the missing narrow Store-publication attempt owner. Mutating calls
  keep their exact code in their operation-specific durable authority with
  retrievable correlation; no parallel generic IC-effect journal is proposed.
- [direct-constructor-frontier.md](direct-constructor-frontier.md) finds 2,208
  production `InternalError::*` references in 101 files after excluding test
  source and inline test tails. The two Component Registry modules contain
  1,154 references. Site-level reconciliation of this frontier is required
  before the qualified symbolic set can become an allocation.
- [component-registry-constructor-leaves.md](component-registry-constructor-leaves.md)
  and
  [component-registry-workflow-constructor-leaves.md](component-registry-workflow-constructor-leaves.md)
  and
  [fleet-subnet-root-workflow-constructor-leaves.md](fleet-subnet-root-workflow-constructor-leaves.md)
  and
  [component-provisioning-constructor-leaves.md](component-provisioning-constructor-leaves.md)
  and
  [component-provisioning-workflow-constructor-leaves.md](component-provisioning-workflow-constructor-leaves.md)
  and
  [fleet-coordinator-constructor-leaves.md](fleet-coordinator-constructor-leaves.md)
  and
  [fleet-coordinator-receipt-invariant-frontier.md](fleet-coordinator-receipt-invariant-frontier.md)
  and
  [fleet-coordinator-root-deletion-constructor-leaves.md](fleet-coordinator-root-deletion-constructor-leaves.md)
  and
  [fleet-coordinator-deployment-ledger-constructor-leaves.md](fleet-coordinator-deployment-ledger-constructor-leaves.md)
  and
  [fleet-coordinator-workflow-constructor-leaves.md](fleet-coordinator-workflow-constructor-leaves.md)
  and
  [canister-pool-constructor-leaves.md](canister-pool-constructor-leaves.md)
  and
  [canister-pool-workflow-constructor-leaves.md](canister-pool-workflow-constructor-leaves.md)
  and
  [root-store-bootstrap-constructor-leaves.md](root-store-bootstrap-constructor-leaves.md)
  and
  [root-bootstrap-store-state-constructor-leaves.md](root-bootstrap-store-state-constructor-leaves.md)
  classify 2,053 effective sites across top-level and direct-child allocation, install,
  commitment and activation plus top-level draining, quiescence and recycling
  and subtree-removal orchestration/effect ops/workflow plus root draining,
  final-inventory and logical-removal persistence plus Store reclamation,
  publication-binding finalization, Store deletion and root-deletion
  preparation plus final/initial root-inventory persistence and its activation
  convergence workflow plus root draining, final inventory, logical removal,
  root summary, Store reclamation/binding finalization, Store deletion and
  root-deletion readiness, sibling Store adoption, Component Directory
  paging/protected-member, Directory convergence/runtime-status, peer/
  protected-allocation, Registry preparation/allocation/create-install
  workflow, direct Component Directory/protected-status persistence and top-
  level Component draining/removal transition and protected-validation
  persistence plus subtree fence/advancement, leaf stop/deletion and
  membership, Directory and leaf-finalization persistence plus protected
  subtree authority/history validation plus top-level commitment, Directory/
  runtime and membership activation, Fleet-service refresh and remaining
  generic Registry/accounting/hash persistence plus Component Group
  acceptance/member progress, protected request/persisted-phase validation,
  result/Directory integrity, cursor advancement, member authority, hashing,
  commit mapping and complete Coordinator-authenticated provisioning
  orchestration plus Coordinator genesis/join/snapshot, Component provisioning,
  service publication, Directory/runtime evidence and root lifecycle plus the
  dedicated root-deletion transition and durable-history owner plus Scale Out
  reservation, activation, reconstruction and retired-receipt replay plus
  Coordinator workflow admission, fences and transparent propagation. The
  first Canister pool range closes Store/import initialization, reset,
  recycling and claims plus autonomous creation through explicit rollover,
  handoff, Store deletion and shared helpers plus maintenance, import and
  recoverable refill workflow plus root Store manifest, artifact and catalog
  bootstrap, root Subnet discovery and sibling Store adoption state. The
  seventy-three passes add 1,678 exact meanings and one Registry-
  state projection while reusing existing policy and earlier-slice identities. The
  complete Component Registry and Component provisioning ops/workflow files are
  classified. The Coordinator parent file's 154 direct constructors are
  mechanically and semantically closed across all 154 direct constructors and
  all 235 parent-file receipt-invariant calls. The root-deletion module's 21
  direct and 10 hidden calls and deployment ledger's 2 direct and 47 hidden
  calls are also closed. All 292 shared-funnel calls and the 12-site Coordinator
  workflow now have dispositions. All 69 Canister pool ops references are
  classified across three consecutive ranges, as are all 17 pool workflow and
  refill references.

The currently qualified ledgers contain 2,333 provisional exact candidates and
31 distinct additional safe projections: 2,364 symbolic identities. Their names
are collision-free and all known reuse inside that qualified subset has been
deducted. They do not yet cover the direct-constructor frontier and therefore
are not an allocation or authority for the next number.

These observations establish work to inventory. They are not proof of a size
win and do not authorize numeric assignments.

## Next Action

Reconcile the effective constructor frontier, continuing with remaining Wasm
Store and Mirror/Directory synchronization owners.
Coordinator ops and workflow are
closed, including all 292 hidden receipt calls. Component Registry
is mechanically closed at all 800 ops and 354
workflow constructors; Component provisioning is closed at all 177 ops and 56
workflow constructors. The original 2,208-reference census is now an effective
2,499-site frontier because one generic Coordinator adapter expands to 292
static call-site meanings across three files.
Then proceed by authority and external-effect risk. In parallel
within evidence-only B1, complete
[dynamic-public-context.md](dynamic-public-context.md) and link every public
message interpolation to its approved ownership classification. Continue the
dynamic ledger with remaining explicit runtime constructions, the transitive
auth formatter and transitive Component Registry messages. Link every constructor
site to an existing
exact meaning, a newly justified meaning or an explicit non-code disposition.
Only then assemble the complete current allocation, permanent-ledger,
host-catalogue and projection tables with
[allocation-proposal.md](allocation-proposal.md) for maintainer approval. Do
not begin B2 or expose a second diagnostic protocol.
