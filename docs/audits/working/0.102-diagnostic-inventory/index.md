# Canic 0.102 Implementation Status

Date: 2026-08-15

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
- Published checkpoints: `v0.102.0` at
  `e6dfd7d2d212f9fce4b1b16caba33d8062e3461d` and `v0.102.1` at
  `86763c5f16478e2e548e2059e5efaa963bf9a966`. They preserve reviewable B1
  evidence without representing incomplete diagnostic behavior as released.
- Open work is the unversioned remainder of the B1 allocation-authority batch.
  It changes no runtime, Candid, stable state or package version.
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
4. every producer observation maps to one reviewed canonical condition and
   handling contract or explicit non-diagnostic disposition, with equivalent
   producers sharing a code;
5. every compressed code has one meaning, class, semantic origin, typed host
   disposition, public projection and, where masked, a retrievable operation-
   correlated numeric observability owner;
6. every proposed number is nonzero, unique across the permanent current and
   retired allocation ledger and backed by a current producer when active;
7. the current host catalogue and generated language-neutral registry are
   bijective with the proposed current allocation rows;
8. representative leaf, Fleet Subnet Root and Wasm Store artifacts have a
   reproducible current-source baseline; and
9. the maintainer can approve or amend the complete inventories, many-to-one
   map and initial
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
- The design's
  [allocation proposal](../../../design/0.102-compact-diagnostic-codes/allocation-proposal.md)
  recommends dense monotonic
  codes, unpadded `E<decimal>` rendering and nine host-only broad classes. These
  choices and the later compressed code table remain maintainer review gates.
- The design's
  [code-allocation ledger](../../../design/0.102-compact-diagnostic-codes/code-allocation-ledger.md)
  now freezes the
  repository-only permanent current/retired ledger contract and the contract
  for a generated language-neutral current registry. No allocation or generated
  registry exists yet, so both ledger sets remain empty until B1 approval.
- The normative design now distinguishes lossless raw decoded identities from
  registered producer identities, makes typed host disposition authoritative
  for host automation, requires operation-correlated evidence for masked
  errors, and defines the one-release-set Fleet activation boundary. These are
  design constraints, not implementation claims.
- [public-boundary.md](public-boundary.md) proves 151 explicit public
  constructions in 26 production files and records the exact maintained
  production-consumer surface: twelve code-dependent machine decisions and six
  transparent decode/render consumers at the pinned current-candidate source.
  In particular, broad `Forbidden`, `Conflict`, `ResourceExhausted` and
  `Unavailable` matches require semantic splits before the compact cut.
- The targeted inventory guard now distinguishes typed evidence from exact
  source anchoring. All 2,864 exact provisional identities name a symbolic
  source anchor in a structured owner cell, and the empty debt set is
  fingerprinted so later source drift cannot silently reopen it. This is a
  producer-manifest boundary, not proof of runtime reachability by itself. The
  first closed family is the twenty-identity access
  boundary, whose exact function/branch anchors have their own completeness
  check. The twelve-identity authority-restore family is also complete at this
  level, including the typed endpoint-policy fence and eleven newly anchored
  persistence/workflow decisions. The twenty direct-prose authentication
  identities now name their exact attestation, proof-retrieval, retention,
  verifier and chain-key configuration functions and have a separate
  family-completeness check. Closing the remaining verifier/signer test-key and
  feature-disabled crypto sites makes all 151 authentication identities
  complete at this symbolic-anchor level. The thirteen runtime-auth renewal/
  admission identities and eleven RPC/runtime-crypto identities also have
  exact function/branch evidence and independent family guards; shared
  identities that were already anchored do not inflate the global progress
  count. The twenty-one prepare-replay identities and eight prepare/
  provisioning identities now close the adjacent request, decision, retained-
  receipt, codec, capacity, remote-proof and issuer-install boundaries with
  their own guards. The eleven core chain-key batch/approval identities now
  close their validation, canonicalization, planning and installation
  branches under a separate guard. A bounded five-identity Canister creation/
  pool helper subset also names the funding-overflow, immutable-configuration
  and missing-asset producers. All ten pool initialization, reset and claim
  identities name their exact transition functions under another guard; this
  is joined by all twenty-one autonomous-creation intent, paid-attempt,
  terminal-evidence, adoption, commit, retry, cancel and rollover identities.
  All ten exclusive-handoff identities bind their begin/completion authority,
  asset-state and terminal-receipt branches. Store deletion, configuration,
  cost/adoption helpers and recycling settlement close the remaining direct
  range: all 56 meanings across 69 pool constructor sites are symbol-addressed.
  The seventeen adjacent workflow constructor sites reference eighteen exact
  identities; after three ops reuses, all fifteen net-new maintenance, import,
  handoff and recoverable-refill meanings are symbol-addressed too.
  All twenty-two root/Store bootstrap meanings now bind their exact manifest,
  topology, artifact, staged-authority and live-catalog branches under a family
  guard; two topology adapters remain transparent typed-cause propagation.
  The adjacent root-Subnet/sibling-Store state ledger anchors all nine exact
  identities—one access reuse plus eight net-new meanings—while preserving the
  typed Registry discovery failure as transparent propagation.
  The twenty-one Store-lifecycle and fifty Fleet Mirror coverage labels now
  bind their exact producer functions as well. Their Store-client and
  Coordinator-result adapters remain transparent typed-cause propagation and
  therefore do not create wrapper codes.
  The adjacent Component Directory/Fleet-service peer boundary also binds all
  forty-two exact meanings to its synchronization and protected-requester
  functions. Its typed Component-binding adapter remains transparent, while
  the already-qualified exact count-overflow projection is reused.
  The durable Component Directory synchronization journal now binds all
  fifty-five coverage meanings to exact progress, acceptance, retry, intent,
  terminal and stable-commit functions. Its unreachable placement commit
  variants remain explicit non-diagnostic dispositions.
  Fleet-activation/scaling plus the five small core ops owners now bind their
  thirteen referenced coverage labels as well. Typed configuration adapters
  and the two impossible scaling shapes remain explicit non-allocating
  dispositions.
  The five adjacent small workflow owners bind their six referenced topology
  and deadline labels; their cost-guard and capability adapters remain
  transparent typed propagation.
  The final small-adapter family binds all twenty-seven candidate/reuse
  identities to exact producers. Its twenty-eight coverage labels include the
  exact Fleet-activation state identity reused as a projection; five typed
  adapters remain transparent, two impossible states remain code-free and the
  twenty-three net-new anchors alone reduce global debt.
  The Component runtime owner now binds all seventy-four exact preparation,
  synchronization, activation, Directory-validation and hashing identities to
  concrete functions. Sixty-four are net-new meanings, ten are exact reuses,
  seventy-two reduce global debt and three storage adapters remain transparent.
  The root-issuer/delegation-batch family likewise binds its sole reused
  certificate-TTL identity to the exact mapper while keeping five typed Fleet,
  storage and policy edges transparent; it creates no duplicate code and does
  not reduce already-closed global debt.
  Root/non-root lifecycle orchestration likewise reuses its two already-
  anchored access/environment identities and source-addresses all nine
  transparent memory, environment, configuration and startup edges without a
  lifecycle wrapper code.
  Runtime coordination, restore and activation bind all twelve referenced
  identities too. Ten reuse existing access/restore/activation anchors; only
  the resumable-refill upgrade fence and credential-bundle capability fence
  reduce global debt, while three memory/storage adapters remain transparent.
- [dynamic-public-context.md](dynamic-public-context.md) now classifies the
  first 656 individual values across the Canic memory-ledger facade, Wasm Store
  GC, shared manifest/capacity conversion, explicit Component Registry denials,
  typed Store publication causes, delegated-session bootstrap and Store
  publication binding/inventory plus Store GC fence, reclamation, binding
  finalization and deletion, the two publication management transports and
  remaining Store module-resolution/internal-client paths, the typed Fleet-
  service peer binding cause, the core plan/Registry typed funnels, runtime
  authentication build-contract roles, prepare-admission authority, prepare
  replay state, issuer provisioning, root-issuer policy, runtime lifecycle,
  restore, activation, environment, Fleet-service binding, topology cascade,
  ICP-refill policy, intent-storage adapters, placement allocation and
  Component runtime canonical hashing, scaling policy and small receipt/
  configuration/RPC/cost adapters plus final root-state, activation,
  Sharding/reservation and cycles-recovery context, role-attestation/root-
  signature proof handling, delegated-token/chain-key verification, trust-
  anchor/signer configuration, durable chain-key batch failures, typed auth
  scope/time fields, nested audience/canonical/certificate fields, issuer-proof
  signature buckets, Component Registry workflow adapters, Component Registry
  canonical encoders, the complete Component Registry capacity/accounting
  formatter group, every Component Group provisioning formatter, the two Fleet
  Subnet Root inventory-count formatters, Coordinator hashing/time evidence and
  the maintained blob-billing public adapter and Component endpoint access
  predicates plus access proof decoders/verifiers, dependency/service guards,
  the non-billing blob facade, every Auth API terminal-mapper route, the generic
  public projection, the two shared replay cleanup-context helpers, the
  local-intent/placement-acknowledgement public values, live RPC authority/
  funding/replay scalars and the expanded replay codec/root recovery context.
  Two hundred eighty-seven are caller-derivable, 67 are sensitive operator-only,
  234 already have exact typed owners and 68 are caller-
  required but unowned. They require request-scoped Store capacity/release
  inspection, guarded delegated-session/local-intent capacity and request-
  scoped cycles-funding preflight status, exact closed-discriminator
  diagnostics, root-proxied live GC inspection or operation-scoped Store
  deletion/publication-attempt progress plus operation-scoped Component/root
  Registry byte projections and provisioning group-placement/per-Spec
  capacity projections. Component RPC and Runtime Introspection
  have zero-row closures for explicit dynamic public-error constructions. Ten
  additional runtime-template values are explicitly excluded from the public-context count
  because their registered resolver has no current in-repository caller; they
  remain maintained surfaces with explicit constructor dispositions rather
  than disappearing from coverage.
  The next ranked 67-site configuration-authoring formatter cluster is also
  closed with zero rows: 66 sites are gated through the native/test-only
  validation module and `LogConfig::validate` owns the identically gated final
  site. They remain native `ConfigSchemaError` prose awaiting B4 ownership
  relocation, not Canister runtime diagnostics.
  Every dynamic publication GC invalid-state field and nested publication
  transport cause is now classified; their static invariant/cause branches
  remain allocation work. The delegated-token and chain-key proof formatters
  are classified, including one unproduced generic string proof-cause lane to
  remove. Trust-anchor and signer builders share one required closed field
  model; dependency parser and crypto prose is discarded. The durable batch
  failure string is also split into Registry-staleness, exact signer and
  per-issuer install authorities. The 105-row authentication dynamic formatter
  is closed; four unproduced `AuthExpiryError` variants are B4 sediment while
  their maintained delegated-token meanings remain allocated. The following
  transitive Component Registry subfrontier is also closed below.
- The first Component Registry transitive slice closes all nine remaining
  workflow `format!` sites after the previously classified public denials. It
  also corrects the shared Store-artifact diagnostics from child-only labels to
  generic top-level/child Component meanings without changing the symbolic
  count. The next slice classifies all fourteen ops canonical-encoder causes;
  exact encoder sites select the code and dependency prose is discarded. The
  capacity/accounting slice then closes the remaining twenty-six ops formatter
  sites and finds twenty-two missing operation-specific projected-byte owners.
  It also finds three formatted and two adjacent static post-precharge ceiling
  checks that are protected accounting contradictions, not ordinary capacity
  exhaustion. The direct ledger now gives those sites five distinct
  reservation-invalid identities and the guarded Registry-state projection.
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
  [auth-string-frontier.md](auth-string-frontier.md) map 151 provisional exact
  authentication/policy candidates and six safe projections. The expanded stop
  contains ten additional typed owners, 96 non-test structural variants and 43
  direct prose construction sites, but wrapper reuse and current-path sediment
  reduce it to 84 additions. The durable chain-key signing state currently
  collapses retryable management failure with terminal protected-policy
  failure; its typed disposition and numeric durable owner are required in
  B4/B5.
- [core-auth-constructor-leaves.md](core-auth-constructor-leaves.md) closes all
  39 core token/delegation and chain-key batch/Registry constructors. Twenty-six
  are exact reuse, typed dispatch or unreachable sediment; the remaining
  thirteen reduce to eleven new exact chain-key state meanings and one
  transparent signer wrapper.
- [runtime-intent-rpc-execution-constructor-leaves.md](runtime-intent-rpc-execution-constructor-leaves.md)
  closes all 19 runtime-intent and authorized root RPC execution constructors.
  It adds fourteen exact meanings, reuses three intent-store identities and
  records the pre-effect deadline/recovery condition explicitly.
- [rpc-workflow-error-leaves.md](rpc-workflow-error-leaves.md) closes all 19
  declared RPC workflow variants plus every source-specific replay decoder
  failure. It adds nineteen exact meanings, reuses generic replay authorities,
  distinguishes terminal from staged response absence and removes two
  unproduced variants instead of numbering them.
- [template-manifest-ops-leaves.md](template-manifest-ops-leaves.md) closes all
  13 manifest-storage variants. It adds ten exact meanings, reuses three Store
  integrity identities and requires one exact request-scoped rejected-byte
  projection instead of a generic detail or last-error slot.
- [publication-binding-release-leaves.md](publication-binding-release-leaves.md)
  closes the twelve non-transport binding/release/chunk constructions as
  thirteen predicates. It adds seven exact meanings, reuses six Store
  identities and exposes the combined GC-fence/release-absence branch that B4
  must split. The 56 GC and two transport constructions remain open.
- [publication-gc-error-leaves.md](publication-gc-error-leaves.md) closes all
  56 GC lifecycle constructions as 64 source-semantic dispositions. It adds
  sixty exact meanings, reuses two existing lifecycle identities and one new
  catalog identity at two later sites, and leaves only two typed publication
  transport constructions open.
- [publication-transport-error-leaves.md](publication-transport-error-leaves.md)
  expands the final two publication wrappers through eleven typed IC leaves per
  management surface. It adds twenty-two exact meanings, no wrapper identity
  and no projection, completing `PublicationWorkflowError` source semantics.
- [rpc-authorization-runtime-auth-constructor-leaves.md](rpc-authorization-runtime-auth-constructor-leaves.md)
  closes all 12 RPC authorization and runtime authentication contract
  constructors. It adds nine exact parent/caller/build-capability meanings and
  reuses two existing identities across three sites.
- [runtime-auth-renewal-admission-constructor-leaves.md](runtime-auth-renewal-admission-constructor-leaves.md)
  closes all 15 delegated-proof renewal and prepare-admission constructors. It
  adds ten exact progress, retry, configuration and attestation-authority
  meanings, reuses three auth identities and retains one typed policy edge.
- [runtime-auth-prepare-replay-constructor-leaves.md](runtime-auth-prepare-replay-constructor-leaves.md)
  closes all 32 authentication prepare-replay constructors. It adds twenty-one
  exact request, replay-decision, receipt-state and response-reconstruction
  meanings shared across both prepare commands.
- [runtime-auth-prepare-provisioning-constructor-leaves.md](runtime-auth-prepare-provisioning-constructor-leaves.md)
  closes all 17 prepare-orchestration and issuer-provisioning constructors. It
  adds three exact retention/install-completion meanings, reuses five existing
  identities and retains nine typed edges.
- [runtime-auth-root-issuer-batch-constructor-leaves.md](runtime-auth-root-issuer-batch-constructor-leaves.md)
  closes all six root-issuer policy and delegation-batch constructors without
  adding a meaning. One existing TTL identity is reused and five typed policy/
  storage edges remain transparent.
- [runtime-root-nonroot-lifecycle-constructor-leaves.md](runtime-root-nonroot-lifecycle-constructor-leaves.md)
  closes all 11 root/non-root lifecycle constructors without adding a meaning.
  Two build/environment identities are reused and nine typed lifecycle edges
  remain transparent.
- [runtime-coordination-restore-activation-constructor-leaves.md](runtime-coordination-restore-activation-constructor-leaves.md)
  closes all 17 runtime coordination, restore and activation constructors. It
  adds four exact upgrade/restore/capability meanings, reuses nine lifecycle
  identities and retains three typed edges.
- [ic-infrastructure-leaves.md](ic-infrastructure-leaves.md) maps the complete
  seven-owner IC infrastructure graph plus pinned call, rejection, signing-cost
  and Candid dependency boundaries to 24 provisional exact leaves and four safe
  projections. Destination-invalid absence remains a typed pre-projection fact.
- [bounded-runtime-leaves.md](bounded-runtime-leaves.md) maps 60 exact topology,
  runtime-log, refill, Placement Index and complete current blob candidates
  plus four safe projections. All 27 blob leaves are allocated in 0.102 and
  their numbers retire without reuse when 0.109 removes the producers.
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
  subtotal and cross-family reuse. A broad census finds 712 uppercase tokens;
  four are documented notation or forbidden/unreachable examples, leaving the
  expected 708 collision-free frontier-checkpoint identities. Later source-
  semantic expansion is added explicitly rather than rewriting that census.
- [projection-ledger.md](projection-ledger.md) aggregates all 31 currently qualified additional
  safe projections and eight exact leaves reused as projection targets. It
  names proposed numeric observation owners and leaves IC effect call-site
  ownership and the current string-coded recent-failure ring as explicit
  approval gates. Cashier uses that guarded numeric owner until 0.109 retires
  its codes.
- [ic-observability-owners.md](ic-observability-owners.md) maps 17 current IC
  call families to their operation authority or guarded runtime status and
  identifies the missing narrow Store-publication attempt owner. Mutating calls
  keep their exact code in their operation-specific durable authority with
  retrievable correlation; no parallel generic IC-effect journal is proposed.
- [direct-constructor-frontier.md](direct-constructor-frontier.md) finds 2,208
  production `InternalError::*` references in 101 files after excluding test
  source and inline test tails. The two Component Registry modules contain
  1,154 references. Site-level reconciliation now closes every mechanical
  reference and all 2,514 effective helper/call-site dispositions. This is not
  yet permission to allocate while dynamic-context and allocation gates remain.
- [inventory.md](inventory.md) closes the current Canister stable failure-text
  census. Fifteen explicit persisted string fields reduce to six failure-text
  fields and nine non-failure protocol/authority/log fields. The six classify
  exactly as two recovery-significant auth fields and four owned operational
  fields; none are redundant stable prose or advisory stable state. Auth needs
  bounded typed failure state in B5, while the two unbounded pool reason
  payloads need explicit bounded operational context. No decision parses any
  of the six strings.
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
  and
  [wasm-store-lifecycle-constructor-leaves.md](wasm-store-lifecycle-constructor-leaves.md)
  and
  [fleet-registry-mirror-constructor-leaves.md](fleet-registry-mirror-constructor-leaves.md)
  and
  [component-directory-peer-constructor-leaves.md](component-directory-peer-constructor-leaves.md)
  and
  [core-plan-registry-adapter-constructor-leaves.md](core-plan-registry-adapter-constructor-leaves.md)
  and
  [core-auth-constructor-leaves.md](core-auth-constructor-leaves.md)
  and
  [runtime-intent-rpc-execution-constructor-leaves.md](runtime-intent-rpc-execution-constructor-leaves.md)
  and
  [rpc-authorization-runtime-auth-constructor-leaves.md](rpc-authorization-runtime-auth-constructor-leaves.md)
  and
  [runtime-auth-renewal-admission-constructor-leaves.md](runtime-auth-renewal-admission-constructor-leaves.md)
  and
  [runtime-auth-prepare-replay-constructor-leaves.md](runtime-auth-prepare-replay-constructor-leaves.md)
  and
  [runtime-auth-prepare-provisioning-constructor-leaves.md](runtime-auth-prepare-provisioning-constructor-leaves.md)
  and
  [runtime-auth-root-issuer-batch-constructor-leaves.md](runtime-auth-root-issuer-batch-constructor-leaves.md)
  and
  [runtime-root-nonroot-lifecycle-constructor-leaves.md](runtime-root-nonroot-lifecycle-constructor-leaves.md)
  and
  [runtime-coordination-restore-activation-constructor-leaves.md](runtime-coordination-restore-activation-constructor-leaves.md)
  and
  [environment-component-rpc-service-binding-constructor-leaves.md](environment-component-rpc-service-binding-constructor-leaves.md)
  and
  [cascade-refill-intent-storage-constructor-leaves.md](cascade-refill-intent-storage-constructor-leaves.md)
  and
  [authority-restore-placement-allocation-constructor-leaves.md](authority-restore-placement-allocation-constructor-leaves.md)
  and
  [icp-refill-replay-constructor-leaves.md](icp-refill-replay-constructor-leaves.md)
  and
  [component-runtime-constructor-leaves.md](component-runtime-constructor-leaves.md)
  and
  [component-directory-synchronization-ops-constructor-leaves.md](component-directory-synchronization-ops-constructor-leaves.md)
  and
  [fleet-activation-scaling-constructor-leaves.md](fleet-activation-scaling-constructor-leaves.md)
  and
  [core-small-ops-constructor-leaves.md](core-small-ops-constructor-leaves.md)
  and
  [small-workflow-constructor-leaves.md](small-workflow-constructor-leaves.md)
  and
  [final-small-adapter-constructor-leaves.md](final-small-adapter-constructor-leaves.md)
  classify 2,514 effective sites across top-level and direct-child allocation, install,
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
  bootstrap, root Subnet discovery and sibling Store adoption state plus Store
  cycle reclamation, module resolution, typed internal calls, inventory and
  physical deletion progress plus root-local Fleet Registry Mirror Joining,
  acknowledgement, active storage, snapshot and monotonic transition authority.
  Root-level Component Directory synchronization and cross-root Fleet-service
  peer authority are also closed. Core Component provisioning-plan and Fleet
  Registry typed conversion adapters are closed without new wrapper identities.
  Core token/delegation and chain-key batch/Registry ops are closed with exact
  auth reuse, typed dispatch, explicit sediment and eleven new state meanings.
  Runtime intent and authorized root RPC execution are closed with fourteen new
  meanings and three intent-store reuses. RPC authorization and runtime
  authentication contracts are closed with nine new meanings and two existing
  identities reused across three sites. Delegated-proof renewal and prepare
  admission are closed with ten new meanings, three auth reuses and one typed
  policy edge. Authentication prepare replay is closed with twenty-one new
  meanings shared across both prepare commands. Prepare orchestration and
  issuer provisioning are closed with three new meanings, five reuses and nine
  typed edges. Root-issuer policy and batch approval add no meaning, reuse one
  TTL identity and retain five typed edges. Root/non-root lifecycle adds no
  meaning, reuses two and retains nine typed edges. Runtime coordination,
  restore and activation add four meanings, reuse nine and retain three typed
  edges. Environment, Component-RPC lifecycle and Fleet-service adapters add
  nine meanings, reuse two and retain six typed edges. Cascade, ICP-refill and
  intent-storage adapters add no meaning, reuse eleven and retain six typed
  edges. Authority restore and placement allocation add twenty-three meanings.
  ICP-refill replay/cost/IC adapters add two meanings and reuse thirteen.
  Component runtime adds sixty-four exact authority/codec meanings, reuses ten
  and retains three typed storage edges. Component Directory synchronization
  ops add fifty-five meanings, reuse one cursor identity and exclude one
  unreachable placement-only commit branch. Fleet activation/scaling and the
  small ops/workflow adapters add fourteen meanings, reuse five, retain six
  typed/public edges and remove two impossible plan shapes. The final small
  adapters add twenty-three meanings, reuse four identities across six sites,
  retain five typed edges and remove two impossible branches. The ninety-eight
  passes add 2,048 exact meanings and one Registry-
  state projection while reusing existing policy and earlier-slice identities. The
  complete Component Registry and Component provisioning ops/workflow files are
  classified. The Coordinator parent file's 154 direct constructors are
  mechanically and semantically closed across all 154 direct constructors and
  all 235 parent-file receipt-invariant calls. The root-deletion module's 21
  direct and 10 hidden calls and deployment ledger's 2 direct and 47 hidden
  calls are also closed. All 292 shared-funnel calls and the 12-site Coordinator
  workflow now have dispositions. All 69 Canister pool ops references are
  classified across three consecutive ranges, as are all 17 pool workflow and
  refill references. All 22 remaining Wasm Store lifecycle constructors are
  classified, including four transparent typed-cause sites. All 32 Fleet
  Registry Mirror constructors are classified, including two transparent
  remote-diagnostic adapters. All 26 Component Directory synchronization and
  Fleet-service peer constructors are classified, including one transparent
  typed-topology adapter. All 26 core plan/Registry constructors are classified
  as transparent typed conversions.

Source-to-ledger reconciliation corrected a nineteen-observation omission at the
maintained delegated-session helper boundary. A mechanical table-set check also
found one earlier prose-arithmetic undercount of an already materialized exact
candidate; it did not add another semantic label. The currently qualified
ledgers therefore contain 2,864 exact provisional identities and 31 distinct
additional safe projections: 2,895 coverage labels. The nineteen added labels
do not collide with the preceding set. Producer qualification expands those
labels into 3,898 exact entries plus the 31 projections. The complete guarded
allocation and host-catalogue proposal maps all 3,929 observations onto 991
review rows; the label and qualified-observation sets remain coverage evidence,
not allocation counts.

Every one of the 2,895 labels is now present in an explicit identity column;
the former configuration, activation, Fleet compiler, protected-deployment,
intent-store, memory-adapter and request-dispatch prose lists are structured
rows. `crates/canic/tests/diagnostic_inventory_ledger.rs` derives exclusively
from those columns, proves that all 31 projections are members of the set and
pins the sorted label set. A second check proves that every one of the 2,864
exact provisional identities occurs in at least one row with structured typed producer,
source, decision, dependency-boundary, site or call evidence. The function and
consumer manifests are now closed, while the derived numeric register remains
a maintainer-review proposal rather than runtime allocation authority.

The complete composition register establishes a 3.96:1 coverage-to-code
reduction and stays below the four-digit rejection gate. It does not authorize
runtime numeric assignments before maintainer approval.

## Next Action

The producer, direct-constructor, transitive formatter, 656-value dynamic
context and current durable-string inventories are closed. The original 2,208
references expand to 2,514 effective helper/call-site dispositions and the
closed label frontier is 2,864 exact provisional identities plus 31
projections. Its producer-qualified review frontier is 3,929 observations.

Review the complete guarded many-to-one register in the allocation proposal.
It maps the closed frontier onto 960 exact-condition contracts plus 31 safe
public projections and proposes dense codes `1..=991`; the source frontier and
register remain repository-only. Rerun the stable failure-string census
immediately before B5 mutation. Do not begin B2 or expose a second diagnostic
protocol before the maintainer approves or corrects that allocation.
