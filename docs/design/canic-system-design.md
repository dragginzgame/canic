# Canic system design

Status: accepted architectural basis; implementation is working toward M2, with narrow M3 autonomy and preservation paths qualified. See the [current qualification scope](../../qualification/README.md) and [dated evidence](../qualification/history.md). Retiring milestone names from test packages does not declare a milestone gate complete.
Date: 2026-11-14. Domain/configuration consistency reviewed against the implemented CLI; see the [review evidence](../qualification/history.md#domain-and-configuration-review).

Naming: Canic is this fresh rewrite. Legacy Canic is the predecessor now hosted
as [canic-legacy](https://github.com/dragginzgame/canic-legacy). Historical package
names and source paths below identify that predecessor as observed, not current
rewrite dependencies.

Canic should let a Rust application team build and operate a multi-canister IC application without writing its own infrastructure management system. It should make the first useful capability inexpensive to adopt, keep application decisions with the application, and remain understandable after years of change.

The maintainer treats old Canic as an intensely iterated design document spanning four years and four generations of LLMs. Its code records experiments, needs, failure modes and accumulated decisions. An existing mechanism has no presumption of inclusion in Canic. This freedom also applies to this design and the proposed crate layout.

Start from user outcomes and derive the smallest coherent system that achieves them. Preserve a safety requirement when its underlying threat, resource obligation or recovery need still exists; rederive its implementation. Historical guards can encode obsolete assumptions as well as important protections. The working direction is a small composable runtime and host planning with explicit effect authority. Physical control-plane topology remains a design decision. Persistent production use is a required destination; a reinstall-only prototype must never be advertised as that destination.

This document covers the whole product surface identified in Canic's current manifests, public modules, CLI catalog, feature guides, active designs and twelve idea topics. Coverage means every identified responsibility receives a reasoned disposition, including removal, delegation or deferral. It does not create a feature-parity backlog. It is not a claim that every historical behavior is desirable, every source line has been audited, or an unbuilt implementation is complete. The coverage register and source record make omissions and disagreements reviewable.

Read by concern: [product contract](#1-product-contract), [architecture](#4-architecture-and-dependency-direction), [durable operations](#7-durable-operations-and-uncertainty), [funding](#9-funding-and-economics), [persistent lifecycle](#13-persistent-releases-checkpoints-and-recovery), [delivery plan](#16-delivery-plan-with-deletion-gates), [capability coverage](#17-complete-capability-disposition-register), [paused ideas](#18-deferred-ideas-and-paused-slices), and [decisions for review](#19-decisions-required-before-implementation).

## 1 Product contract

### Users and outcomes

An application developer should be able to start with an ordinary Rust canister, add one capability, and understand its memory, lifecycle, permissions and cost. An operator should be able to review changes, run them, close the terminal, resume elsewhere with the required custody, and distinguish success from an unknown result. A governance-controlled application should be able to approve the same exact effects without an operator key acquiring additional authority.

The core journeys are:

1. Inspect externally built artifacts without installing a Canic runtime.
2. Build a fresh application and establish a working single-subnet Fleet.
3. Add another subnet and grow shared or dedicated workers within explicit capacity and funding limits.
4. Continue operating during operator absence, subject to finite delegated authority.
5. Diagnose an incomplete operation and resume its original work without paying twice.
6. Compose Auth, IcyDB, Blob or another runtime without duplicate lifecycle or memory ownership.
7. Capture and restore an application-consistent checkpoint when the application supplies the necessary protocol.
8. Eventually change a persistent production installation while preserving explicitly promised identity, state and outstanding obligations.
9. Release disposable capacity, reclaim controlled resources, or destroy an installation under a separately reviewed disposition.

Application identity, tenancy, routing semantics, data schemas, consistency, business transactions, user balances and governance remain application-owned. Canic supplies infrastructure capabilities, not a database replication system, application economy or universal distributed transaction manager.

### Hands-on delivery from the start

The team must be able to use each delivered increment and feed experience into the next one. The first deployment is a Hello World application on the IC mainnet, with a local rehearsal of the same supported path. Mainnet experimentation begins before the stateful consumer, autonomous growth and production lifecycle are complete. Passive inspection is useful as an intermediate increment, but does not complete M1.

The first hands-on loop is: obtain the tool and example, build, review the target/controllers/artifact and finite cycle budget, deploy, call the application, inspect its status, change the greeting, and deploy the changed executable. The second deployment must explicitly state whether it preserves the principal and state or creates a fresh disposable installation. Unsupported upgrades must not become an implicit reinstall. Retained canisters, balances and the next maintenance or disposition action remain visible after the experiment.

Deliver this through the product CLI and documented public interfaces, not a private deployment script. Each increment includes a runnable example, exact setup and usage instructions, supported networks, visible limitations and a repeatable team exercise. Team feedback on setup, errors, deploy time, costs and operation status informs the next scope; record actionable feedback in owning GitHub issues. Keep the previously delivered journey runnable as capabilities grow, and document any explicit experimental breaking change.

M0 resolves only the contracts needed for the first effects and records later decisions at their owning gates. Neither the full governance profile nor adoption by the proposed Toko consumer blocks the operator-controlled Hello World loop. Exact authority, bounded spending and truthful uncertainty still apply to its real effects. Mainnet availability describes where the experiment runs; persistent production support remains the M7 claim. Actual execution selects a concrete signer, target and budget; the design does not supply spending authority or credentials.

### Smallest stateful application journey

The next design witness is a persistent application with one entry canister and a shared worker on one subnet. Build and install it, serve an authenticated application operation, grow to a second worker under a finite budget, interrupt a management call, recover its original outcome, and inspect what happened. M2 must include meaningful application state, one actual stateful supplier such as IcyDB, and explicit endpoint, memory and timer composition. By M3, a second executable must preserve the promised application invariant and an operation awaiting reconciliation. This narrow rehearsal does not claim general migration or production readiness.

The application owns logical user routing and data semantics. Begin with explicit canister declarations. Introduce groups, role catalogs, a global directory, warm pools or cross-subnet coordination only when a concrete journey needs them. The 2026-11-15 decision selects a separate per-fleet Wasm Store for managed deployments; its contract is in section 12.

For each mechanism, record the need, failure being prevented, simplest candidate, state/effects introduced and acceptance witness. Compare direct host-controlled installation with a subnet-local executor before selecting the first managed topology. Autonomous growth requires a surviving on-chain effect owner; a permanently running operator process is not an equivalent substitute. Choose one initial topology after this comparison rather than implementing speculative deployment modes.

### Accepted developer and operator journeys

The maintainer's 2026-11-14 workflow review admits the following product outcomes.
These refine the destination; they do not claim that today's CLI implements them
or require replacing the existing mainnet demos before their successor qualifies.

- Define the app's artifacts, roles and permitted relationships once. Instantiate
  that definition in several fleets with distinct networks, identities and budgets.
- Generate a small buildable Rust application with one application canister
  controlled by one Root/management owner. Support a persistent local development
  loop and the same reviewed deployment semantics on mainnet. The selected next
  topology adds a per-fleet Wasm Store controlled by Root. A Coordinator remains
  conditional; the currently shipped starter has no Store yet.
- Build, review and converge the complete selected fleet through one plan/apply
  path. Repeating a completed convergence has no mutation effects. Individual
  canister changes use the same owners and evidence.
- Preserve cascading discovery and make scaling and sharding optional composable
  building blocks. No particular growth workload is selected yet; the existing
  profile consumer supplies the first bounded extension, not a Toko port.
- Treat a subtree as an operational scope: it could represent an entire project.
  Support reviewed inspection, drain/suspend/resume and persistent upgrade journeys
  without affecting siblings outside that scope. This does not add a new App/Fleet
  ownership level or import Toko-specific data semantics.
- Operators supply cycles manually. Canic may distribute those cycles within
  explicit grants; ICP conversion is excluded, not an optional adapter backlog.
- Keep initial operator handover simple: retain configuration, exact artifacts and
  journals, then provision the receiving operator's signer authority separately.
  This is the working default, not a promise of reconstruction from on-chain data
  after losing the journals. Application frontend integration remains deferred;
  the existing read-only fleet visualization stays in scope.

Deliver these as small usable increments within M2/M3. The existing reusable
app definition and Root-plus-app journey extend to Root/Store/app through the
same whole-fleet convergence owner. Qualify discovery and subtree state propagation
before broadening growth or subtree upgrades. No new command aliases, generic
workflow engine or package catalogue follows merely from this scope decision.

### Adoption levels

| Level | Useful result | Required commitment |
| --- | --- | --- |
| Passive tools | Artifact comparison, build evidence, diagnostics and cost reports | Host tools and supplied inputs; no controller or runtime changes |
| Composed runtime | Endpoint guards, lifecycle participation and selected service integrations | Explicit local memory, endpoints and runtime participants |
| Managed Fleet | Provisioning, placement, funding, artifact distribution and recovery | Exact governance/controller assignments and durable operations |
| Governed production | Persistent releases, complete recovery and admitted governance integration | Qualified lifecycle and application readiness contracts |

These are documented dependency boundaries, not four parallel implementations. A capability is independently adoptable only after a real consumer proves that its dependency graph and integration obligations match the claim.

## 2 Design decisions and limits

| Decision | Recommendation | Consequence |
| --- | --- | --- |
| Product organization | Organize by responsibility and state owner | Do not recreate a universal core containing every service |
| Fleet topology | Managed fleets select Root and a separate versioned Wasm Store; derive further infrastructure from concrete needs | Root owns effects, Store owns artifact bytes; Coordinator remains conditional |
| Application runtime | Only explicitly selected endpoint, binding and lifecycle support | A leaf does not link provisioning, estate accounting or host tooling |
| Execution | Domain-specific durable operations sharing small mechanical primitives | No generic workflow interpreter or universal journal schema |
| Authorization | Separate infrastructure authority, optional Fleet admission, application identity and resource permission | Passing one check never implies the others; Fleet filtering is explicitly selected |
| Configuration | Compile a small set of typed inputs into one normalized plan | Defaults and derived capabilities cannot become competing writable authorities |
| Persistence | State belongs to the domain that can explain and repair its invariants | Views and caches cannot silently acquire authority |
| Protocol | Wire for owned contracts initially; fixed external protocols remain external | Transport selection stays out of domain decisions |
| Scheduling | Durable demand, bounded workers and independent recovery wakeups | Quiescence is observable and cannot strand unfinished effects |
| Extensibility | Explicit compile-time composition and narrow adapters | No service locator, runtime plugin registry or blanket capability engine |
| Compatibility | No compatibility layer with the old Canic implementation | Existing Canic adoption is a separate future undertaking |
| Production lifecycle | Design a qualified persistent release boundary before claiming production support | Experimental reinstall remains a distinct operating mode |

The old repository's sequencing and release gates describe its delivery history. They do not dictate Canic's architecture. Toko's adopted engineering baseline still governs this repository. Optional Candid selection for owned endpoints would require an explicit change to the current shared Wire policy before implementation; keeping the design seam does not grant that exception.

### Complexity acceptance

Every proposed subsystem must identify a concrete user journey, its one authoritative state owner, its public contract, its external effects and its recovery proof. Each persistent field must have a writer, reader, invariant and retention rule. Each abstraction needs actual distinct callers. A trait is justified by a real alternate implementation or test boundary, not by possible future extensibility.

A vertical change includes callers, tests, generated interfaces, operator diagnostics, documentation and removal of superseded code. No batch closes with two active implementations, an unused replacement, or a permanent forwarding facade. Package count and line count are evidence, not success criteria.

### Mechanisms excluded from the starting architecture

- A universal Core with mandatory endpoint/workflow/policy/ops/model/storage/view layers.
- A required Coordinator plus Root plus Store installation for the smallest application.
- Independent role, feature, capability and runtime settings that control the same service selection.
- Framework takeover of application lifecycle or hidden initialization when mounting a service.
- Mandatory Fleet-wide whitelist distribution for ordinary application authentication.
- Separate recovery owners for equivalent commands, or a universal workflow interpreter absorbing every domain.
- Executable rebuilds solely to represent changed approval or qualification evidence, without a demonstrated executable-binding requirement.
- Repair of obsolete installation metadata before erasing disposable state when current custody and obligations can be established directly.
- Copies of independent Auth, Blob, Backup, storage or scheduling engines inside Canic.
- Cross-Fleet transfers, alternate codecs and elaborate cache optimizations as prerequisites for the first working application.

Reintroducing one requires a concrete need and a fresh design decision. Important invariants such as bounded spending, explicit authority and truthful uncertainty must be shown directly; the number of historical guards is not evidence of safety.

## 3 Domain model

The hierarchy is **App → Fleets → Canisters**. One logical application may have
multiple fleets, each a deployment on an explicitly trusted network with its own
canister identities and retained operations. Fleet names are scoped to their app;
for example, `profile/mainnet` and `profile/staging` are deployments of `profile`.

An app owns code and application semantics; a fleet owns one deployment's selected
network, canister membership, management policy and bounded effect authority.
Several fleets may use the same network and executable. One fleet may eventually
span several subnets; neither network nor subnet is an app or fleet identity.
Canister roles describe responsibilities inside that deployment, not additional
ownership levels. An external controller or service is a relationship, not an
implicitly owned fleet member. Within a declaration, the same canister under the
same exact network binding cannot belong to two fleets. Shared services require
explicit external references when that consumer need is admitted.

Preserve the useful legacy distinction between a canister's application role,
its immediate parent in the application tree, and its IC controller set. A
manager remains a manager when it controls children. Routing from an entry to a
worker does not itself establish parentage. Current concrete fleet declarations
may carry optional `role` and `parent` slot references; parents must exist in
the same fleet and the tree must be acyclic. These fields support inspection
before deployment as well as the deployed view. They are descriptive, not
spawn grants or controller policy, and do not modify an existing approved plan.
Applying topology through autonomous management still requires the explicit
admission and custody contracts below.

Canister trees contain only canister nodes. Root appears at the top of that
tree, with its application descendants below it. Show an external operator's
controller relationship separately as metadata, not as an indented ancestor
of Root. This presentation rule applies to terminal output, visualizations and
documentation; the controller relationship remains explicit without mixing
identity principals into the application tree.

Names select scope; they do not grant controller rights or prove ownership. A
configured principal is a declaration, a certified controller read is an
observation, and an approved operation supplies bounded effect authority. These
must remain distinct even when displayed together. The current CLI has no rename,
fleet-transfer or identity-migration operation: editing a name does not retag an
approved plan, move a journal, renew an allowance or move a deployed canister.

| Concept | Meaning and owner |
| --- | --- |
| Application specification | Logical component types, relationships, service exports, permitted child patterns and readiness contracts; application-owned |
| Fleet | One deployed application instance on one explicitly trusted network |
| Fleet policy | Governance-selected placement, admission, funding, resource bounds and recovery authority |
| Component specification | A reusable artifact/configuration contract; not a deployed canister |
| Component allocation | Logical instance plus a unique allocation generation, physical canister, executor and artifact binding |
| Physical asset | A controlled canister with observed location, controllers, module, balances and custody status |
| Service directory | Derived routing projection of committed component allocations, with freshness and generation |
| Artifact | Exact executable bytes and associated schema, build and capability identities |
| Deployment plan | Immutable proposed effects and bounded authority over exact inputs and observations |
| Operation | One retained business intent and its reconciliation history |
| Funding grant | A finite budget over a named source, destination derivation, purpose and validity policy |
| Observation | Evidence with subject, source, time or generation and confidence; never an implicit instruction |

A principal is insufficient as the identity of a reusable allocation. Every callback, directory binding, shard assignment and removal request that can outlive reuse binds the allocation generation. A late success for an old allocation may settle its accounting but cannot affect a replacement.

Fleet, Component, Root and related names are working vocabulary for discussing the source material. They do not freeze the public API or require a corresponding Rust type. Begin with explicit canisters and their relationships. If repeated structure justifies groups, compile them into the same deployment graph; a group gains no canister, treasury, lifecycle or controller. Roles and capability catalogs likewise require demonstrated classification or compilation needs and never grant caller permission.

Keep desired topology, admitted operations and observed state distinct. A desired count is not a reservation. An observed canister is not an adopted asset. A directory row is not custody. A successful request submission is not completion.

## 4 Architecture and dependency direction

```mermaid
flowchart TD
    User[Operator or external governance] --> CLI[CLI and automation clients]
    CLI --> Host[Host planning and observation library]
    Host --> Authority[Selected management authority]
    Authority --> App[Application canisters]
    App --> Composition[Application-owned composition root]
    Composition --> Runtime[Small Canic runtime]
    Composition --> Adapters[Selected service adapters]
    Adapters --> Services[Independent Auth Blob IcyDB services]
    Host --> Suppliers[Independent build query and backup libraries]
```

Arrows show interaction and composition, not blanket permission or a fixed canister count. Authenticated commands and retained grants define permission. Subnet execution and global coordination refine the management authority when the workload requires them.

### Logical modules and possible packages

Start with Contracts, Host and CLI for the passive/planning journey; retain `canic` for the small application runtime as the installed journey develops. Pure planning starts inside Host. Separate host and Wasm dependency graphs from the beginning. These are responsibility boundaries, not a requirement for one crate or canister per row:

| Boundary | Owns | Must not own |
| --- | --- | --- |
| Contracts | Bounded IDs, public commands/results, semantic digests and published protocol declarations | Filesystem, timers, storage engines, policy decisions |
| Planning | Pure compilation, placement constraints, effect proposals and validation | Live discovery or paid execution |
| Application runtime | Installed binding, endpoint admission, lifecycle participation and local service discovery | Fleet planning, estate ledgers, host identity keys |
| Coordinator | Global policy generations, membership and grant issuance | Every child continuation, rendering or per-leaf metric history |
| Subnet executor | Physical assets, local allocation, management operations and local budget reservations | Application schemas or arbitrary application routing |
| Artifact service | Content-addressed upload, integrity, chunk access and retention | Deployment approval or application readiness |
| Host | Discovery, build integration, plan review, durable ingress custody, reconciliation and reports | A second authoritative copy of remote domain state |
| CLI | Parsing, help, formatting, exit status and invocation | Hidden business policy or a separate recovery algorithm |
| Adapters | Canic-specific composition around independently owned services | Copies of supplier engines or supplier DTOs |

Use direct functions and domain modules first. A module may contain its transition function, record schema and effects together when that makes its ownership legible. Thin endpoint adapters authenticate and decode; a domain transition validates current state and decides the next action; an effect adapter performs one external attempt; its completion returns to the same domain. Do not force every request through eight directories or mirror every DTO into a view, model and storage type without a representation reason.

### Infrastructure roles

The selected next managed starter has one Root/management owner controlling a Wasm Store and one application canister. The Store is an infrastructure role, separate from application allocation cardinality. Retain an explicit external controller for Root's maintenance and recovery; show that principal as controller metadata outside the canister tree. Qualify the exact graph before deployment. Across subnets, globally scarce budgets and membership changes may justify a coordination owner, while executors retain local custody and finite delegated authority. A Coordinator, when selected, must not be needed for every continuation of previously admitted local work.

The experimental `canic-root` now qualifies one or two direct greeting children locally,
using the same bounded management engine as the profile manager. Its controller
is the external operator; its created application's sole controller is Root.
Lifecycle mounting and application readiness remain explicit in each canister.
The CLI now generates a standalone greeting workspace whose artifact passes that
same installed qualification. Managed-child CLI plan/apply now pins Root code,
retains exact admission attempts and recovers completion through replicated
observations. One child upgrade is now locally qualified, including persistent
application state and interrupted callbacks. The runtime now qualifies two permanent child slots, each with a bounded history
for eight serial upgrades and independent continuations. Both share finite cycle
and artifact custody budgets. The installed CLI qualifies both children through
creation, independent upgrades, recovery and fleet views. Root maintenance
compares complete custody for both slots, including Empty. General trees and
subtree operations remain unqualified. See the
[managed-child evidence](../qualification/history.md#managed-child-cli-approval).

The maintainer's 2026-11-15 decision selects a separate, dynamic, versioned Wasm Store per managed fleet. Root must not be the durable artifact repository. Root retains approved immutable references, reservations and effect evidence; the Store retains verified bytes, versions and retention pins. This separates artifact growth and release history from Root's authority journal and maintenance. Section 12 owns the contract and migration gates. Direct host bootstrap and passive artifact tools remain usable without installing a fleet. Artifact possession never supplies deployment approval authority.

External governance retains the controller hierarchy it approves. A governance integration may place the same executor logic inside an existing infrastructure owner only after composition proves one lifecycle and one effect owner. No hidden recovery key or universal operator bypass is acceptable.

Later sections use Coordinator, executor and Store to name these responsibilities. Store is selected for managed fleets; Coordinator remains conditional. Names do not import the legacy implementations.

The existing mainnet topology is recorded in M0: the operator directly
controls Hello World, the profile entry/shared worker, and the profile manager;
only the manager's autonomous worker is controlled by that manager. This is not a
fleet-wide executor or a general Root. Its artifact bytes are retained in the
manager; those deployments predate the separate Store decision. Moving their
custody requires the qualified transition below, not an automatic migration.

### Repository and workspace boundary

Keep one repository and one product workspace initially. CLI depends on Host, Host depends on passive Contracts, and the application runtime depends on Contracts and selected suppliers. Host communicates with deployed roles through contracts without depending on their runtime implementations. Add role crates when the topology requires separately compiled artifacts, and adapter crates with their first complete consumer journey.

The independent qualification workspace isolates test-only graphs. Separate Host into another workspace only for demonstrated dependency/toolchain incompatibility; a separate repository requires an independent ownership or delivery need. Release groups can evolve independently within one repository. Dependency direction, Wasm builds and downstream consumer tests enforce the runtime boundary.

This compilation view is distinct from the interaction diagram; arrows mean dependency:

```mermaid
flowchart LR
    CLI --> Host
    Host --> ManagementContracts[Management contracts]
    Role[Selected infrastructure implementation] --> ManagementContracts
    App[Consumer application] --> Runtime[Small Canic runtime]
    App --> Supplier[Selected stateful supplier]
    Runtime --> RuntimeContracts[Contracts used by the runtime]
    Host --> HostSuppliers[Host suppliers]
```

Contracts are organized by the protocol a consumer actually speaks. A runtime consumer must not import an omnibus provisioning, treasury, backup and administration command surface. Shared identities can remain small shared types. Independent native Host and Wasm consumption justify a package boundary as soon as both exist; they do not require separate repositories.

## 5 Configuration and operator experience

Separate three categories of input by writer; this does not require three files or a large configuration language:

- Application specification: component artifacts, logical dependencies, permitted children, service contracts and optional participants.
- Deployment policy: network trust, topology targets, placement restrictions, budgets, admission and governance.
- Local execution settings: credential references, paths, tool configuration and presentation. Secrets never enter canister configuration or artifact metadata.

The application specification is reusable across fleets: fleets reference its
logical components rather than copying artifact/role/child definitions. Concrete
allocations, network trust, placement, funding and controller assignments remain
fleet-owned. The generated starter and whole-fleet planner consume this same
model. Migration from today's concrete slot declarations must preserve retained
principal bindings and plan digests; editing source configuration cannot amend an
already approved operation. Reusable app definitions do not require a generic
group catalogue or independently deployed group canisters.

The compiler validates the explicit deployment graph, normalizes sets and produces typed desired state. Service selections have one source of truth; any features or capability descriptions are derived from it. Group expansion is added only if reusable groups are selected. Unknown keys, impossible combinations, unavailable services, contradictory placement and unbounded effects are errors before network mutation. Static policy and mutable deployment policy each have one owner; any override must be explicitly modeled as a narrowing operation.

Keep semantic configuration identity separate from source-file identity. Reformatting a file should not change its meaning; exact source bytes still belong to build evidence when a build can consume them. After approval, the exact compiled plan and selected schema are immutable.

The implemented unversioned declaration has reusable definitions at
`apps.APP.canisters.SLOT` (role, parent and optional Wasm input) and existing-ID
bindings at `apps.APP.fleets.FLEET.canisters.SLOT`. Every fleet resolves the shared
definitions; its bindings cannot override topology or artifacts. Network and
journal path remain fleet-owned. Wasm paths resolve beside the config; exact
bytes are retained only when planning. The generated starter now has a bounded
whole-fleet planner for one new Root and one new application. General spawn policy,
larger fleet funding policy, adoption and upgrade convergence remain unimplemented.
The default journal is `.canic/apps/APP/fleets/FLEET/` beside the config.
An existing shared journal may remain in place, but observations and operation
selection must distinguish its fleets by exact retained names or known target
and network evidence. Unassigned uncertain records remain visible as such.

Reject ambiguous selections, duplicate JSON names, conflicting physical
membership and upgrades outside the selected fleet before retaining a new plan.
Approval still binds the exact network, target, artifact, controller and budget;
mutable config cannot change those inputs during apply or recovery. These local
selection checks are not an authorization boundary against an operator who
already controls the signing identity. `--no-config` uses explicit retained
plans; it does not bypass signature, controller, deadline or evidence checks.

### Approval and admission contract

One canonical approval record binds its schema, network trust identity, normalized plan digest, approver and authority generation, validity policy, effect graph, artifact/configuration identities and cumulative resource ceilings. An effect names its operation ID, prerequisites, permitted caller, target reference, request commitment and result schema. Preview and execution consume this same representation.

A target reference is either an exact principal/allocation generation or a typed output of a named approved effect. For example, `create_worker_1.result.canister` may become allocation generation G on subnet S with controllers C, within creation budget B, and receive artifact H and configuration K. Admission validates those constraints; the retained creation result resolves the reference exactly once. The principal is recorded as evidence under the original digest. It cannot be substituted by editing the approved plan. An unexpected result is held for reconciliation, not silently installed or published.

| Input | Admission treatment |
| --- | --- |
| Network trust, artifact hash, selected protocol/schema | Exact binding; mismatch requires a new approval |
| Controller set and authority generation | Exact prerequisite or explicitly approved transition with retained evidence |
| Newly created principal | Result of the named approved effect, validated against its constraints |
| Fees, attached cycles and funding | Approved ceilings, cumulative exposure and stated freshness policy; no automatic budget increase |
| Capacity and local inventory | Current authoritative reservation at admission, not the host's earlier observation |
| Diagnostic observations | Evidence only unless explicitly promoted to a safety prerequisite |

The executor validates its current authority and atomically reserves its owned capacity, funding and evidence space before admitting an effect. Competing operators cannot both consume the same resources. Revalidate relevant fences before each later dispatch; remote controller changes still require provider evidence and reconciliation. A last-minute host reread does not remove these races.

An autonomous grant is separate from a deployment plan. It permits bounded future instantiations of an exact component specification under a named parent allocation/generation, policy generation, destination derivation and finite capacity/funding allowances. It is not an informal extension of a completed deployment approval. Activation uses the evidence chain in section 12.

### Current command contract

Keep the shipped vocabulary canonical. `canic app init APP --fleet FLEET --network
NETWORK` adds a new app and its first fleet, creating the config if missing and
preserving other apps if present. Existing app names are refused.
`--app APP --fleet FLEET` selects an existing
deployment; a selector may be omitted only when its choice is unique. See the
[runnable configuration journey](../../README.md#initialize-an-app-and-fleet).

| Journey | Commands |
| --- | --- |
| Initialize and build | `canic app init APP --fleet FLEET --network NETWORK`, `canic artifact build --package PACKAGE`, `canic artifact inspect WASM_FILE` |
| List local declarations | `canic app list`, `canic fleet list`, optionally `--app APP` for fleets |
| Add another deployment declaration | `canic fleet init FLEET --app APP --network NETWORK` resolves the app definitions with no existing-ID bindings, without creating canisters or journals |
| Define reusable canisters | `canic app canister add SLOT --app APP [--role ROLE] [--parent SLOT] [--wasm PATH]`, `canic app canister list --app APP`; app-owned topology and artifact inputs, shared by all fleets |
| Bind and list deployment identities | `canic fleet canister bind SLOT --app APP --fleet FLEET --id PRINCIPAL`, `canic fleet canister list --app APP --fleet FLEET`; observation declarations only, never controller or deployment approval; unknown definitions, occupied bindings and conflicting membership refuse |
| Review and execute a starter fleet or one deployment | `canic fleet plan --controller PRINCIPAL` selects Root-plus-app creation or its next child upgrade; `--canister SLOT` selects one deployment; `canic fleet apply DIGEST` resumes that exact approval |
| Investigate or continue the original operation | `canic fleet status DIGEST`, `canic fleet apply DIGEST` |
| Observe | `canic fleet show`, `canic fleet show --watch`, `canic fleet view REPORT` |
| Exercise the greeting or inspect the wallet | `canic fleet call DIGEST`, `canic wallet PRINCIPAL` |

Only structured output (`--json`) is global. App/fleet selection and journal
arguments belong to deployment actions, not artifact inspection, saved-report
viewing or wallet queries. Without a canister, Wasm override or target,
`fleet plan` consumes the selected fleet's two declared Root/app artifacts. It
refuses other trees, existing bindings and competing individual plans rather than
deploying a partial fleet. Its immutable approval names the future Root as the
typed output of its approved creation. Root and child signing/evidence remain
with the same existing effect owners; the journal lock covers their composition.
Replanning identical desired inputs returns original authority, including after
expiry. Existing single-canister planning remains explicit: a configured creation
names an unbound slot, while an upgrade
names an admitted principal and its previous module hash. Named creation uses the
app-defined Wasm input when present and refuses a competing CLI override; otherwise
an explicit `--wasm` remains required. `fleet call` currently
exercises the hello demo by its completed operation digest; it does not promise a
generic application call interface. See the [command contexts](../../README.md#command-contexts).

Wider admission, funding, capacity, restore and integration commands have no
reserved spelling or implementation backlog merely because their responsibilities
appear in this design. Admit their consumer need first; do not add aliases or a
parallel command hierarchy speculatively.

Read-only commands must clearly disclose paid observations where applicable; planning is mutation-free, not necessarily cost-free at every remote provider. Automatic discovery has bounded calls and bytes. Preparation never silently installs tools inside validation.

Every mutating operation requires a human-readable preview and a structured JSON form binding the same digest. JSON stdout is machine output; progress uses stderr, and experimental schema changes must be documented. Errors need a typed cause, operation identity, effect certainty, current owner, and admissible next action. Resuming means continuing original authority; it never means inventing a new retry allowance. Broader error/preview contracts remain qualification requirements where the current one-canister CLI does not yet implement them.

Remove synonyms unless they serve genuinely different audiences. Do not retain `ensure`, `deploy`, `apply` and an internal repair command as independent orchestration owners. An eventual shorthand may call the same plan/apply flow.

## 6 Authority and security

### Separate decisions

1. Network trust: identify the intended network and its trust anchor; local discovery cannot silently replace a production key.
2. Infrastructure authority: establish the controller or delegated governance right to perform this exact management action.
3. Optional Fleet ingress admission: where selected, determine whether a caller may enter a protected application surface under the installed policy generation.
4. Application authentication: establish the issuer and subject, distinct from the transport caller.
5. Application authorization: decide whether that identity may act on the specific resource.

Fleet membership never grants resource access. An application session never grants treasury or controller authority. Discovery never grants either. Local endpoint checks remain mandatory even when an early ingress filter exists.

Each Canic-owned managed method declaration supplies access class, payload bounds, protocol identity and an explicit public/protected classification. One declaration generates the endpoint adapter, interface catalog and permissible inspection predicate, using Platform's generic IC entrypoint machinery and Wire's codec functions under the [endpoint ownership boundary](#endpoint-ownership-and-actor-composition). Duplicate methods, missing access decisions and incompatible mounted names fail compilation. `inspect_message` is only a cheap early filter; it cannot replace update authorization or reserve resources. Its platform restrictions are defined by the [IC system API](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/).

### Policy distribution

Ordinary application authentication and authorization do not require a Fleet-wide whitelist. When a deployment needs an additional infrastructure admission policy, assign it one canonical owner and explicit propagation semantics. A Coordinator can serve that role when present. Local projections remain derived state.

A consumer requiring coordinated revocation may justify bounded fence/prepare/activate/open transitions with fixed participant membership. Activation waits for the required generation; an unavailable participant remains a visible blocker. A consumer accepting bounded staleness needs a documented freshness and expiry contract instead. Choose the needed promise before implementing either mechanism; neither is a compulsory framework-wide default. After each asynchronous effect, validate relevant retained authority and execution fences before advancing.

For fenced revocation, status distinguishes `Requested`, `Fencing`, `Completed` and `NewGenerationOpen`. Completion requires every required participant's acknowledged durable fence plus the selected treatment of already-admitted work. An unreachable participant may still serve the old policy: a blocked transition does not mean revocation has completed. Opening the next generation requires the separate activation acknowledgements. Settling old external effects never reopens revoked discretionary authority.

### Controller and recovery graph

The selected next starter has an explicitly trusted operator recovery authority R controlling Root/management owner E, which controls the application and Wasm Store canisters. R is an external principal with retained host custody, not a canister tree node. R performs E's maintenance; its power to replace E is explicit transitive trust. Installed CLI witnesses qualify named Store bootstrap under an existing bound Root, whole-fleet bootstrap before application admission, and one Root-owned Store replacement followed by Root and child upgrades. This is local qualification, not mainnet maintenance evidence. The direct-host comparison establishes bootstrap cost and authority; it cannot satisfy autonomous operation without a surviving on-chain effect owner.

| Role | Controllers and management-effect caller | Upgrade owner and surviving journal | Failed migration, restoration or retirement custody |
| --- | --- | --- | --- |
| Application | E; E performs management effects | E retains the application upgrade operation | E holds allocation identity, grants and receipts outside the application; R can repair E |
| Subnet executor E | R; R performs E maintenance effects | R's authenticated host custody retains E's upgrade operation; E's domain evidence is preserved or exported before stopping | R retains inventory and outstanding exposure; loss of that evidence blocks new effects, not a fresh allowance |
| Coordinator, if selected | R; R performs maintenance | R retains the upgrade journal; executors retain their acknowledged delegation liabilities | R retains reconciliation evidence; unavailable delegations remain encumbered |
| Per-fleet Wasm Store | E; E performs Store maintenance | E retains the maintenance operation and immutable references outside Store; Store preserves catalog, bytes, upload progress and pins | Retained host backup/export permits exact reconstruction; unavailable or missing bytes block dependent effects without losing Root's journal |

A role is never the sole surviving executor or journal owner for the operation that stops or replaces it. Ingress-result custody and its retention horizon must be demonstrated for R as well as E. An operator cannot resolve missing evidence by declaring an uncertain operation failed.

The proposed governance profile is hierarchical: governance controls infrastructure owners whose approved code controls descendants. For an SNS profile, ordinary registered infrastructure dapps have SNS Root as controller; their descendants are not assumed to be individually registered dapps. Current ordinary dapp registration removes other controllers outside testflight ([SNS Root source](https://raw.githubusercontent.com/dfinity/ic/master/rs/sns/root/src/lib.rs)). Consequently E cannot be assumed to remain co-controller of every registered application.

M0 proves the direct-operator management/recovery path needed for the first deployment. Before implementing a governance integration, demonstrate that its selected approval/execution path can upgrade and recover an infrastructure owner with durable custody outside the affected canister. The direct-operator fixture alone does not prove that. If the consumer requires direct SNS registration of each application, SNS's supported controller path becomes the effect owner and requires a separate architecture decision before that integration. There is no fallback operator bypass. Full SNS lifecycle qualification remains M7; it does not gate the early operator-controlled mainnet experiment.

### Threat model

Qualification must cover unauthorized ingress and canister callers; malicious payloads and oversized replies; compromised application canisters; stale or forged observations; host-file replacement and corrupted journals; concurrent operators; replay; changed controllers; exhausted budgets; and interrupted authority rotation. A compromised controller can replace code or state: Canic cannot promise protection from a party that has that power. Document recovery-controller trust explicitly.

Revocation prevents new authority. It does not erase a possibly issued paid effect. Minimal reconciliation authority must remain available to settle the original effect without permitting new discretionary spending. Recovery access is explicit and narrowly scoped.

## 7 Durable operations and uncertainty

Distributed effects require durable domain records; they do not require a general workflow engine. Use a small shared vocabulary while retaining each domain's actual state machine:

```text
Proposed -> Authorized -> Reserved -> ReadyToIssue -> Issued
Issued -> Confirmed -> Settled
Issued -> Uncertain -> Reconciling -> Confirmed | ProvenNoEffect | Blocked
Proposed/Authorized/Reserved -> Cancelled, only if no effect may have issued
Settled -> retained terminal receipt -> bounded archival/compaction
```

`Issued` means an attempt may have reached its target. A lost callback, timeout, killed process or expired lease is not proof of failure. A local commit cannot atomically commit a remote effect. The adapter must declare what evidence can prove completion, non-execution or continuing uncertainty, consistent with the [IC interface contract](https://docs.internetcomputer.org/references/ic-interface-spec/).

Every external mutation retains, before dispatch, its operation ID, owner, exact request commitment or replay bytes, target/network, authority generation, reservation and provider-specific duplicate identity. Separate:

- business operation identity, which survives retries;
- transport attempt identity, when another attempt is genuinely admissible;
- worker lease generation, which fences late local completions.

A lease permits another worker to reconcile; it does not permit a second debit. If the provider has no usable deduplication or outcome query, the operation remains blocked for a documented resolution instead of pretending to provide exactly-once execution. Expired provider replay windows do not justify changing timestamps and resubmitting a payment.

Store current state and the minimum durable evidence required for reconciliation. Use append-only records where custody demands them, not event sourcing for every field. Terminal receipt retention has explicit replay horizons; compaction must preserve deduplication and financial obligations for those horizons. Unknown effects cannot be garbage-collected as old work.

Host custody and remote authority have different records. The host retains signed ingress and received evidence needed to resume; remote domains retain reservations and accepted effects. They reference one business operation rather than reconstructing independent business decisions. A new host may continue only after custody transfer or sufficient authenticated reconstruction; a copied filename is not that proof.

The current Host operation model selects one explicit action with its complete
required authority: direct creation/upgrade, Root-owned child creation/upgrade,
or Root maintenance with the full ordered child custody. Fleet children form one
ordered collection of permanent identities. The selected runtime supports one
or two direct children; that capacity does not justify separate child/sibling
branches throughout planning, execution and presentation. Retained-format codecs
preserve existing approval digests, journal locations and signed attempts while
the working model changes. Replanning cannot remap names to different slots.

### First effect contracts

The table specifies required evidence, not provider guarantees already demonstrated. M0 pins the actual provider/API and qualifies the first managed path. Ledger effects apply only to the selected cycle-funding/creation provider; no ICP conversion adapter is planned. Each adapter retains exact request bytes or commitment, caller, network, target, authority, amount and transport identity before dispatch. A Canic operation ID provides no provider deduplication by itself.

| Effect and initial API | Wait and retained identity | Success evidence | Non-execution evidence and permitted recovery |
| --- | --- | --- | --- |
| Creation: management `create_canister` from E | Unbounded response wait candidate; retained attempt and creation constraints; no assumed provider idempotency key | Returned principal durably associated with the approved output; verified custody and placement | Definitive provider refusal before creation permits a new authorized attempt. Lost result remains charged/blocked unless authoritative evidence recovers that exact principal; inventory similarity is insufficient |
| Cycle funding: management `deposit_cycles` | Unbounded response wait candidate; exact destination and attached amount, attempt and refund record | Retained successful completion under the API's credit semantics and cycle accounting | Retained rejection/refund evidence interpreted under the provider contract; balance deltas alone never authorize replay |
| Installation: management `install_code` or selected chunked install API | Unbounded response wait candidate; target generation, mode, exact module/configuration and attempt | Retained completion, verified current module hash and accepted installation binding | Current hash alone does not identify which attempt initialized state. Reconcile the binding and execution evidence; never repeat initialization or reinstall merely after a timeout |
| Controllers: management `update_settings` | Unbounded response wait candidate; exact old/new sets and transition operation | Retained completion and authoritative new-controller evidence available to surviving custody | Reobserve through an authorized controller; losing read authority is not failure proof. Never erase the outgoing sole custodian before the successor is confirmed |
| Ledger transfer: selected ledger's transfer API | Declare selected transport wait; retain exact source/subaccount, destination, amount, fee, memo and provider timestamp/duplicate tuple | Correlated block/transaction receipt or provider duplicate response identifying the original transfer | Replay only the identical request within the provider's qualified duplicate horizon. After expiry, query admissible evidence or remain blocked; never refresh timestamp to manufacture a retry |

Creation result custody is an M0 go/no-go experiment: interrupt dispatch, callback and durable result retention separately. If the chosen API cannot reconstruct a lost principal, either choose a qualified recoverable provider path or explicitly accept bounded blocked exposure. Do not advertise automatic creation recovery without this proof. The early successful recovery witness must name an interruption the selected provider can actually reconcile.

The experiment must identify durable host custody before submission, the executor's local commit of intent/reservations with outgoing-call enqueueing, the separate commit retaining the returned principal, and a later failure that cannot erase it: a queued call acts only after non-trapping return, and a trapping callback discards its own writes ([IC system API](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/), [execution properties](https://docs.internetcomputer.org/references/message-execution-properties/)).

Request size, attached-cycle amount, worker concurrency and retry authority are bounded independently of response-wait semantics. IC bounded-wait calls can return `SYS_UNKNOWN` and lose attached cycles, including refunded cycles; unbounded-wait calls have an accepted-plus-refunded conservation guarantee but different liveness costs ([IC execution properties](https://docs.internetcomputer.org/references/message-execution-properties/)). Receiver deduplication does not recover transport-lost cycles. Select and test wait policy per effect, expose possible loss as exposure, and never turn "one attempt" into a universal bounded-wait setting.

### Exhaustion and recovery reserves

Before dispatch reserve the operation record, maximum retained evidence, capacity/funding exposure and enough protected operating resources for the qualified reconciliation path. Unknown effects continue consuming those allowances. When unresolved obligations hit their bound, refuse new discretionary work while retaining original reconciliation authority. Evidence retention is not silently shortened to admit more demand.

Recovery receives protected worker opportunities, storage and operating reserve; ordinary child demand cannot spend the last resources needed to record or settle an issued payment. Define these budgets against the supported outage/reconciliation window. They cannot guarantee progress through indefinite provider failure: report a reserve shortfall before exhaustion and remain truthfully blocked. Qualification fills the uncertainty budget, observes refusal of new work, then proves existing work can still record evidence and settle. A watchdog alone proves only a wakeup.

### First vertical journey

Fresh Fleet creation proceeds in this order:

1. Compile specifications, select exact artifacts and establish network/signing authority.
2. Collect bounded placement, inventory, fee and funding evidence; form an immutable preview.
3. Approve its digest and retain host custody before the first dispatch.
4. Establish management authority and artifact delivery for the selected topology under exact bootstrap operations. Bootstrap a global owner first only when dependent executors require its grants.
5. Reserve capacity and funding before allocating or claiming component assets.
6. Verify artifact bytes and install bindings, initialize participants, then obtain application readiness.
7. Publish the application bindings needed for this journey and satisfy its selected ingress policy. Global directory or Fleet-wide admission transitions apply only where explicitly selected.
8. Return a terminal receipt and verified deployment binding; export a frontend-specific binding only when the consumer needs it. Reapplying the completed plan produces no new effects.

A supplied-canister bootstrap follows the same identity, custody and verification rules. Capacity import cannot implicitly establish infrastructure. Failure at any stage has a status owner and an original operation to reconcile.

## 8 Allocation placement and scale

Applications request capacity with a component specification, parent allocation, placement constraints and an idempotency key. Canic chooses only among admitted capacity. Allocation reserves both logical capacity and the physical resources needed for completion before dispatch; settling or releasing reservations is part of the same owner transition.

Executors own bounded indexed asset tables. Useful states are held, preparing, ready, reserved, installed, draining, uncertain and released; exact variants should follow the implemented operations rather than preserve old names. Counters and reverse indexes update atomically with their authoritative rows. Full scans are bounded repair/diagnostic operations, never the normal allocation path.

### Application allocation patterns

Retain these four application patterns as admitted product requirements. They
describe cardinality and assignment, independently of physical subnet placement,
application role, immediate parentage and IC controller authority. They are
optional compositions over the same allocation, funding, readiness, discovery
and upgrade owners; their names do not require four runtime engines or preserve
legacy configuration syntax.

| Pattern | Meaning and example | Required identity and maintenance behavior |
| --- | --- | --- |
| Singleton | At most one live allocation of a declared role per concrete parent; for example, a project's settings canister | Concurrent creation resolves the same admitted slot. Upgrade preserves its identity and application state. A singleton under each project is not a Fleet-wide singleton. |
| Scaling workers (legacy `replica`) | Bounded interchangeable workers serving equivalent requests | Retain worker identities and capacity accounting. Qualify bounded rollout, drain and readiness before routing new work. The name grants no data replication, leader election or failover semantics. |
| Shards | Many application keys share each capacity-bounded canister; for example, user records across user shards | Retain each key's exact allocation through growth and upgrades. Application lifecycle preserves shard contents; Canic does not silently move existing keys when capacity changes. |
| Keyed instances (legacy `instance` / placement index) | Each admitted key selects one dedicated canister within its declared scope; for example, one project canister per project ID | Repeated resolution returns the same allocation. Concurrent creation shares retained pending work. Upgrade preserves the key binding, principal and application state. |

Scope singleton and keyed uniqueness by the owning parent allocation and declared
role or assignment policy; keyed instances additionally bind the application key.
The same key in a different parent or Fleet is not automatically the same instance.
A Fleet-wide uniqueness promise needs an explicit authoritative owner and cannot
be inferred from a role name or a per-parent limit. Application authentication and
admission precede key allocation; bound key size, retained claims, instance count
and funding exposure. Fresh caller identities must not create unbounded demand.

A pending or uncertain claim remains bound to its original operation; timeout
alone cannot authorize another child. Resolution of an unavailable assignment
reports unavailable until explicit qualified recovery. Generation-bound claims
and callbacks prevent late success or release from modifying a replacement.
A removed allocation invalidates its routes. Releasing an old shard binding must
not decrement a replacement's counts. Recheck capacity at assignment commit to
handle concurrent completions, while replay of an existing key keeps its original
assignment even when the shard is full. Allocation reuse and data movement remain
separate, explicitly qualified operations.

### Physical placement

The eventual placement choices are same subnet as a named parent or dependency,
a selected admitted subnet, and bounded distribution across admitted subnets.
Keep hard locality requirements distinct from preferences: an unsatisfied hard
constraint produces a typed refusal, never silent remote placement. Distribution
needs explicit count, per-subnet density and spread bounds. Spreading canisters
does not itself replicate application data or provide failover. Qualify each
selected arrangement with its actual effect owner and provider before supporting
it; this design does not claim all combinations currently work.

Same-subnet work is the preferred execution path. Protected placement classifies a route as same-subnet, cross-subnet or unknown. Unknown is never assumed local. Directory paging must carry a generation or comparable consistency marker; mixed pages cannot masquerade as one complete inventory. Large dynamic Fleets require indexes by owner, subnet, role and unfinished operation, including children created after initial deployment.

Shared tenancy is a first-class qualification workload. Do not equate one logical user with one canister. Placement policy accepts application capacity signals without requiring Canic to understand user records. Subnet outages, pressure and lack of funding cause bounded queueing or typed refusal; no hidden unbounded backlog.

### Delivery and upgrade qualification

The four patterns are the intended product scope; their implementations enter
incrementally with a named consumer and bounded fixture. Today the Root
journey qualifies up to two fixed child slots and bounded serial upgrades, not general singleton
admission, scaling, sharding or keyed-instance APIs. Keep those limits visible;
do not add unused configuration variants or a general pool catalogue in advance.

Each pattern's useful journey includes creation, resolution, bounded growth where
applicable, upgrade and interrupted-operation recovery. Before extending to many
children, qualify repeatable maintenance without resetting spent or uncertain
authority. An approved rollout binds its target allocations and artifact versions,
finite concurrency and failure policy; concurrent growth cannot silently add
unapproved upgrade targets. Ordering and mixed-version protocol compatibility
come from the application. Partial completion retains per-target evidence and
resumes exact operations; it is not an atomic subtree upgrade or automatic rollback.
Singletons, keyed instances and shards preserve bindings and application invariants;
interchangeable workers still require the application's drain/readiness contract.
Unlimited recursive spawning, automatic rebalancing and data replication remain
outside these four patterns.

Multi-Fleet operation initially means isolated Fleets inspected by the same host. Cross-Fleet movement is not required for multi-subnet operation. Controlled cycle disposition between Fleets remains a separate experiment with its own physical proof and budget.

### Delegated capacity and reclamation

For each approved global limit L, keep non-overlapping accounting categories: unallocated capacity U, unused outstanding delegated allowances D, committed/reserved usage C, and uncertain issued usage Q. Maintain `U + sum(D) + C + Q <= L`; moving one unit between categories never counts it twice. Retired usage returns to available capacity only after the corresponding release proof. Coordinator projections may lag, but cannot treat missing reports as available capacity.

Each delegation has a unique identity, executor generation, finite allowance and validity policy. E atomically moves its unused allowance into reserved or uncertain usage when admitting work. Reclaim only after an acknowledged durable fence preventing further admission under that delegation and reconciliation of all issued effects. Return proven unused capacity; retain committed and uncertain liabilities. Expiry can prohibit new issuance but cannot prove earlier non-execution. An unreachable executor's allowance stays encumbered.

For a limit of 100 delegated as 60 to A and 40 to B, losing contact with A creates no new capacity for B. A replacement inherits the predecessor's outstanding exposure and cannot receive a fresh allowance that ignores it. The same principle applies to funding grants. M4 must exercise lost fence acknowledgements, delayed creations and executor replacement.

## 9 Funding and economics

Use distinct types and ledgers for cycles, fees, grants and observations. Separate operator assets, Coordinator treasury when selected, executor operating reserve, estate creation accounts, canister balances and application/player balances. Reservations are liabilities, not expenditure. Deposited cycles are not all consumed cost. Read-only public ICP balance/address inspection does not introduce an ICP spending or conversion owner.

Funding follows the selected custody graph: external treasury or governance grants finite authority to the owner that actually executes payments. A subnet executor can fund its managed children; an additional Coordinator treasury is justified only by a real global funding need. Creation funding remains distinguishable from operating funds even if both accounts share an owner. Applications may express demand; they cannot select arbitrary infrastructure destinations or mint treasury authority.

A grant specifies source authority, derivable destinations, purpose, maximum cumulative debit, fees, validity policy, per-request bounds and revocation behavior. Reservation occurs before payment. New policy generations do not automatically reset cumulative spending limits. Balance thresholds and rate limits cannot substitute for a lifetime or explicitly renewed exposure bound.

### Autonomous operation

Retain prompt demand-triggered funding for active components plus coarse bounded coverage for idle components. Define the required runway against observation cadence, expected burn, outages and maximum replenishment delay. Observe and expose starvation risk before reserves are exhausted. Idle sweeps are paged and resumable; no global scan is required per child request.

Estate replenishment uses the same grant and transfer owners with a distinct purpose and account. It must not be implemented as a second treasury or queue. Exhausted authority pauses growth and reports the required renewal. A new operator-approved grant may extend future work but cannot overwrite an uncertain old transfer.

ICP conversion is excluded by the maintainer's 2026-11-14 decision. Operators
manually supply cycles to the explicitly identified ledger account or native
canister balance required by the selected path; these are different balances.
Canic reports the destination and shortfall and pauses when funded authority is
exhausted. It never purchases cycles, converts ICP or falls back to another
account. Bounded parent-to-child cycle distribution remains in scope. Previously
recorded external conversions remain historical evidence, not a Canic feature.

### Conservation

At each transfer boundary, reconcile opening controlled value plus admitted inflow against closing controlled value, accepted outflow, observed consumption/fees and explicitly permitted discard. Separate currencies and accounting windows. Report uncertainty as a liability with its evidence owner, never as a plug in the arithmetic.

Deletion requires both data-disposition authority and settled financial/external obligations. For retained empty canisters, preserve controlled native cycles and verify custody; evacuation solely to delete and recreate them is unnecessary. A source that must transfer its own cycles requires executable, reviewed machinery and physical qualification. The old capsule proposal supplies constraints, not an already proven primitive.

## 10 Runtime lifecycle memory and scheduling

The application owns one lifecycle composition root. It declares participants, their checked memory grants, restore order, readiness prerequisites and suspend/restore hooks. Endpoint mounting adds endpoints only. Linking a crate cannot start a timer, allocate a private manager, create an issuer or silently grant permissions.

Startup synchronously restores required local state and installs authority fences before scheduling fallible work. Async initialization advances a durable readiness operation with explicit failure and retry. An installed module is not automatically ready for publication. A failed participant cannot be hidden behind an apparently healthy framework status.

Keep lifecycle state distinct from operation progress and scheduler activity. A component may be initializing, ready, draining or suspended while a particular management operation is uncertain. Fleet-wide state changes use a retained target generation and participant acknowledgements; observing the Root in a new state does not prove every descendant has converged. Draining rejects new discretionary work while allowing original-effect reconciliation. Suspension reconstructs explicit participant state on resume rather than treating a restarted timer as application readiness.

### Cascading discovery and subtree operations

Adapt the legacy behavior as three narrow responsibilities: authenticated
topology/direct-child projection, locally readable role/service discovery, and
operational-state propagation. They share committed allocation identities, not
one universal mutable snapshot. Directory lookup returns all exact role matches
in deterministic path order within its explicit scope; a unique-target request
refuses ambiguity. Discovery neither grants access nor proves live readiness.

Propagate validated bindings to affected descendants after admission/activation;
do not require applications to wire every principal manually. Preserve exact
parent, receiver, role, allocation generation, revision and content commitments.
Exact replay is harmless; conflicting or stale projections refuse. Dynamic
children join through the same publication owner. Global directories, Coordinator
and legacy Store-specific topology transport are not prerequisites.

A subtree operation binds its selected root, tree revision, complete bounded
target set, action and operation generation. Authorize the request at its owner
and authenticate each propagation hop; neither a role nor an IC controller edge
substitutes for application parentage. Fence overlapping transitions and new child
admission while capturing the target set. Later children must inherit the active
restrictions before publication; they cannot silently enlarge an approved upgrade
plan. Sibling subtrees outside the selection remain unaffected.

Retain per-target pending/applied/unconfirmed outcomes and reconciliation failures.
Bound fanout, payloads and report detail without reporting omitted targets as
complete. Retry the original operation; recheck its generation after awaits so an
older pause cannot overwrite a later resume, even if a state value repeats.
Recovery/control endpoints remain usable during application suspension. IC-level
stop/start is a separate management effect, not a synonym for logical suspension.
Subtree upgrades use section 13's persistent release and management-effect custody;
a state cascade cannot install code or imply an atomic distributed upgrade.

First qualify two sibling branches, a grandchild, repeated roles, a failed/lost
reply and retry, stale delivery, upgrade recovery and concurrent child admission.
The [legacy evaluation](../qualification/history.md#cascading-and-workflow-scope-review)
records source/history evidence and the differences needing new qualification.

### Memory and scheduling

Memory ownership uses IC Storage allocations and stable names, not copied hardcoded IDs. Compile and validate one allocation manifest. Optional roles do not open unused stores during startup, diagnostics or sampling. Measure fresh physical allocation as well as logical usage. Reopening state must validate its schema, allocation owner and bounds before granting access.

Use one qualified executor/timer provider instance in a composed canister. The scheduler owns wakeups; the domain owns durable demand. Each recovery-critical task retains its operation and attempt fence. A bounded independently rearmed watchdog can recover abandoned work, but does not create new business authority. Stale callbacks cannot clear successors.

Demand-driven pool maintenance is the baseline direction: activation, allocation, reset and recovery transitions re-arm actionable work. Healthy or unsupported local creation demand becomes observable quiescence. A coarse safety sweep may remain if complete event coverage is unproved. Measure its cost separately from productive work and preserve recovery reliability.

Entropy comes from the independent entropy owner. Security challenges requiring fresh post-intent randomness cannot use buffered seeds or expanded streams. Hosts own readiness, cancellation and bounded retry; snapshot restoration reconstructs volatile generators and prevents old replies from populating a new lifecycle instance. No predictable fallback is permitted.

## 11 Authentication and service composition

IC Auth owns credential, verification, session and replay semantics. Canic owns issuer enrollment under Fleet authority, exact receiver configuration and endpoint/lifecycle integration. An authenticated application identity is `(issuer, subject)`; the browser or calling canister remains a separate transport identity. Do not invent a parallel session store or resurrect removed token modes for convenience.

Declarative profiles compile roles, scopes, issuer selection, expiry and receiver requirements into one normalized manifest. They do not create a new authority system. Clients request profile identities; protected discovery identifies the permitted issuer. Renewal cannot widen grants. Revocation, stale projections, lost replies, device logout and clock behavior are acceptance cases, not fields left for an adapter to guess.

Do not assume synchronized cross-canister clocks. Preserve upstream signed deadlines and explicitly documented local-time refusal semantics. Production readiness is established by actual enrollment, issuance, browser transport, receiver admission and logout journeys—not by compiling an adapter.

| Integration | Supplier owns | Canic owns |
| --- | --- | --- |
| Auth | Credential/session engine and protocol | Fleet enrollment, endpoint guards and runtime composition |
| Blob | Upload/storage/accounting/certification and service semantics | Explicit service mounting, memory grants, lifecycle and funding bindings |
| IcyDB | Data model, database consistency, persistence and database readiness | Host lifecycle, endpoint admission, memory and entropy integration |
| Backup | Snapshot artifacts, transfer/custody machinery and generic capture/restore progression | Fleet membership freeze, controller authority and application acceptance |
| IC Relay (optional) | Durable channels, batching, acknowledgements, bounded request/result primitives and selected worker orchestration | Endpoint composition, admitted peer/route bindings, lifecycle participation and application workload authority |
| Metrics and logging | Shared measurement and log machinery | Bounded labels, role-specific collectors and operator presentation |
| Build/query/host utilities | Generic platform and host mechanics | Product artifact selection, trust, plan and deployment semantics |
| ICRC | Required external standards and representations | Explicitly selected application adapter and guards |

Qualify the selected service arrangement, embedded or separately deployed. If both arrangements are eventually supported, they must use the same independently owned service engine, with each arrangement qualified for its actual composition boundaries. Supplier packages remain usable without Canic. Direct supplier types are preferred where identity is shared; duplicate DTOs require an actual boundary. Pin and qualify type/runtime identities that must agree. Do not force unrelated tools and product packages to advance as one dependency generation.

### Embedded Blob composition witness

The first optional adapter is a module in `canic`, behind its `blob` feature.
It mounts three bounded Platform/Wire endpoints with captured caller identity,
using the independent supplier's Command, Inspection and management contracts.
The application explicitly grants Blob's declared Storage lease alongside its
catalog, creates with its actual principal, reopens with `Service::open`, and
calls the service synchronously. Mounting contributes no storage bootstrap,
timers, manager selection or activation.

The installed asset-catalog witness in Canic's qualification workspace owns two
users, four lifetime catalog slots and publication policy. Checked user paths may
invoke local application authority; public uploader paths preserve the actual
caller. Conflicts/capacity precede Blob mutation. Later persistence or reply
encoding failure traps, preserving atomic rollback of both owners. Blob retains
all reference/accounting and maintenance decisions. Closed admission survives
upgrade until activation of the exact attempt; readiness never activates it.

Current Host and Root effects use empty installation/upgrade arguments and have
no approved Blob drain/stop/reopen/activation sequence. That is an explicit
integration gap, not permission to weaken the supplier's hooks. The local witness
uses explicit installation and method-bound unit upgrade arguments with a trusted
controller and a separately configured manager. Fleet lifecycle support must
extend the existing approval/effect owners and bind those arguments and the exact
maintenance attempt before this app can join the managed fleet journey. No parallel
Blob deployment CLI or recovery engine is selected. See the
[current witness and input evidence](../demos/blob.md). This does not close the
provider journey, certificate, settlement or sustained-history gates.

## 12 Protocols transport and artifact identity

Domain transitions consume logical bounded values. Endpoint adapters select and invoke supplier-owned codecs, and call adapters own a single attempt. Application operations own retry and consistency. A decode failure after a remote mutation must not trigger a second request using another codec.

### Endpoint ownership and actor composition

Accepted direction, 2026-11-14: Canic owns endpoint composition for explicitly
mounted actor capabilities. Platform owns generic IC entrypoint macros. Wire
owns message encoding and validation without knowledge of deployed endpoints.
This is the implementation destination, not a claim that the migration has shipped.

| Owner | Responsibility | Boundary |
| --- | --- | --- |
| IC Wire | Bounded framing, serialization, decoding and validation of caller-supplied protocol identities | No exported-method catalogue, endpoint mounting, IC entrypoint macros, authorization or lifecycle |
| IC Platform / Runtime | Generic IC entrypoint macros, exports, execution context, argument handling, guard invocation and reply machinery | Explicit codec/guard hooks; no Wire-specific endpoint policy or Canic actor model |
| Canic | Compose selected actor adapters; bind exported names, protocol contracts, access rules and readiness requirements; derive the managed endpoint catalogue | Delegate codecs to Wire and IC execution to Platform; no copied codec, executor or universal Core |
| IC Testkit Wire | Optional native encode/call/decode convenience over Testkit's simulator client | Test-only; no Canic runtime, endpoint catalogue, installation, lifecycle or retry policy |

Suppliers expose ordinary typed capabilities and remain usable without Canic.
The application explicitly mounts a Canic adapter for a selected capability;
Canic binds its handler to an exported name, passive protocol contract and access
decision. Platform generates the actual IC entrypoint and invokes the selected
hooks. Wire encodes and validates bytes. Endpoint mounting adds no implicit
timers, storage allocations, lifecycle participants or management authority;
those remain explicit at the application's composition root.

Keep protocol types, stable method IDs and directional limits passive and usable
by hosts and tests without a runtime dependency. An exported method name and a
Wire method ID are distinct: multiple exports may intentionally use one protocol
contract. Wire may check that supplied ID without knowing any exported function.
Canic derives endpoint views from the mounted declarations rather than maintaining
a second authoritative catalogue. Any Canic-specific declaration macro belongs
in Canic and emits Platform hooks; it must not implement IC execution itself.
Use the smallest declaration mechanism demonstrated by the selected actor adapters.

Retire the separate `ic-wire-endpoint` and endpoint-macro implementation after
inventorying and migrating its supported consumers. Canic consumes Wire's codec
API directly through its composition path, rather than adding a guard hook to
Wire Endpoint as the long-term solution. Non-Canic consumers can compose
Platform's explicit codec hooks directly. Preserve Wire's independent codec and
value-schema functionality; removing endpoint ownership is not a frame-format change.

The first Canic migration covers the shared management endpoints mounted by Root
and the profile manager, including creation and upgrade. Remove superseded
handwritten codec wrappers while preserving export names, exact bytes, explicit
IDs/limits, authorization-before-copy ordering, rejection versus trap behaviour,
one dispatch/reply path and synchronous/async callback rollback boundaries.
Qualify valid and malformed calls, wrong identity/direction, oversized frames,
unauthorized callers and relevant reply-encoding failures through installed
endpoints. Supplier retirement follows migrated caller evidence; no package
rename, compatibility facade or replacement generic adapter framework is required.

Track the implementation in [Canic #2](https://github.com/dragginzgame/canic/issues/2),
supplier retirement in [Wire #61](https://github.com/dragginzgame/ic-wire/issues/61),
generic hooks in [Platform #164](https://github.com/dragginzgame/ic-platform/issues/164),
and test adapter alignment in [IC Tooling #359](https://github.com/dragginzgame/ic-tooling/issues/359).

The first local Canic implementation now explicitly mounts Root and profile-manager
management methods through `canic_management::mount_endpoints!`, using the existing
Platform hooks and Wire codec API. Its [installed evidence](../qualification/history.md#endpoint-composition-ownership-decision)
covers admission and reply rollback. This fixed management adapter does not yet
supply general actor declarations or catalogue generation; supplier retirement
remains separate work.

### Protocol and artifact contracts

Separate four identities: logical protocol/schema, transport encoding, canonical signed or hashed bytes, and stable storage representation. Changing one does not silently change the others. Owned Canic contracts use Wire initially; external management, ledgers and standard interfaces retain their required encodings. Optional Candid selection is a contingency requiring policy approval and two complete qualified builds, with no runtime negotiation or fallback.

A call contract declares request/reply limits, wait policy, attached-cycle maximum and effect certainty. Check request size before dispatch and reply size before decoding. An oversized reply may still follow a completed remote effect. Metrics use bounded outcome/route/method classes, not arbitrary principal labels. Generic calls do not acquire mandatory Fleet lookups.

Build records bind exact sources, selected lock graph, toolchain, features, target, relevant environment, generated inputs and final artifacts. Read executable paths from Cargo artifact messages, not guessed filenames. Generated infrastructure entrypoints stay small and output under build storage; applications do not maintain copies of canonical infrastructure source.

### Per-fleet versioned Wasm Store

Selected on 2026-11-15. Store implements versioned publication, chunk custody,
quota changes, durable pins and bounded management-cache preparation. Root
qualifies reference-backed effects/custody, controller-mediated publication and
replicated Store observation after handover. Fleet Host/CLI publication works
with an already-bound Store; named Store bootstrap now combines direct creation,
installation, controller handover and replicated readiness in the same retained
operation. Creation selects the approved Root's certified subnet, retained before
signing; changed placement blocks recovery. Store observation establishes
initialized state through Root's actual call path, alongside separate certified
code and sole-controller checks. No application child slot is reserved.
Whole-fleet planning now includes an optional declared Store, bound to the same
Root creation output and completed before application admission. Its separate 1T
creation debit appears in the total approval; no app slot is consumed. Retained
Store slots cannot be added or removed through replan; a changed executable selects
a separate maintenance approval after completed work.
Fresh fleets with a declared Store publish both selected application artifacts
before either child admission. The immutable release label is the application
canister name plus the complete content hash; manifests and future Store creation
output are bound by the original fleet approval. Changed child artifacts first
produce an independent publication approval through existing plan/apply; after
publication completes, replan approves installation separately. A completed
release remains reusable after reviewed Root maintenance; this does not renew
unfinished publication authority under different Root code. The existing
bounded serial upgrade chain retains target, code, source and slot authority.
Retained inline fleets do not migrate implicitly. Host Root maintenance binds the
Store identity/code and every child's original creation and historical upgrade
delivery, including retention pins and cache deadlines. Replicated full-custody
observations must match before and after the original direct install attempt;
legacy inline observations cannot substitute. Store authority/code/placement and
child identity/code/controllers are independently checked. Only completed
maintenance advances Root code authority for subsequent work. One Host-driven
Store upgrade now has an installed CLI witness; inline-to-Store migration remains
unqualified. See the
[current scope](../demos/wasm-store.md). The full contract below still requires
pin release, reclamation and fleet integration. Each managed
fleet owns a separate Store with a persistent, dynamically extensible catalog.
Adding artifacts or versions must not require rebuilding or reinstalling Root or
Store. The first implementation serves one subnet. Its canister tree is:

```text
Root
├─ wasm-store
└─ app
```

The external operator controls Root and is displayed separately. Root owns
management effects and deployment approval. Store owns artifact publication,
storage and delivery; it cannot authorize an application upgrade. Root's durable
records contain references and evidence, not Wasm payloads or a mirrored catalog.
Bounded transient transfer buffers are allowed; they are not a second repository.
Root's explicit publication endpoint forwards one typed prepare/chunk/publish/observe
command under actual controller authority and a bounded dispatch deadline. Store
independently checks Root control and retains immutable publication outcomes. Root
keeps no publication catalog or retry journal. A failed Root reply after dispatch
cannot roll back Store; the operator retains the exact command and reconstructs
progress by replicated Store observation. Publication never grants install authority.

| Concern | Required contract |
| --- | --- |
| Artifact identity | Immutable content digest, exact length, encoding, ordered chunk hashes and build/protocol metadata. Specify stored-byte and installed-module hashes separately where encoding changes them. |
| Release versions | A logical artifact name plus an explicit version resolves to one immutable manifest. Repeating identical publication is idempotent; assigning different bytes to that version is refused. Paginated history survives selection of a newer release. |
| Selected release | A mutable, authenticated selection points to a published version. Selection changes are retained and do not deploy anything. Plan review resolves it once to the exact manifest/hash and Store binding; apply never resolves a moving “latest” again. |
| Upload | Authenticated, bounded, resumable chunk staging with reserved capacity. Verify each chunk and the complete ordered payload before atomic publication. Interrupted uploads remain incomplete and cannot be installed; identical committed chunks can be reused. |
| Dynamic capacity | Store owns explicit byte, artifact, version, staging and request/work limits with visible usage/headroom. Authorized limit changes preserve data and require no reinstall; platform ceilings still apply. No fixed two-version array, whole-catalog payload rewrite or inherited 512 KiB child/approximately 1 MiB fleet product ceiling. |
| Retention | Root obtains an idempotent durable pin for the exact approved operation before relying on availability. Pin acquisition and release have retained outcomes. Pending, uncertain and recovery-required artifacts cannot be evicted; rollback candidates have an explicit retention policy. Expiry or a lost release reply alone does not prove that a pin can be removed. |
| Deletion | Reject deletion of pinned content. Removing a selection does not delete its release. Explicit bounded reclamation removes eligible bytes while retaining version/hash history and an availability/tombstone record; it cannot permit reuse of a version for different bytes. |
| Observation | Show versions, selected and deployed hashes separately, upload progress, pins/blockers, bytes/headroom and delivery-cache readiness. Catalog listings read metadata rather than scanning artifact payloads. |

Stable Store bytes are authoritative; the IC management chunk store is a
rebuildable installation cache. Verify cached chunk presence and restore only
missing chunks from the exact pinned artifact. Root remains the caller of
`install_chunked_code`. The IC requires its chunk-store canister to share the
target's subnet, and the caller to control that store or be the store itself.
Root controlling both Store and application satisfies the selected graph. See the
[management-canister specification](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/#ic-method-install_chunked_code).
Multi-subnet fleets retain one logical version authority but must qualify
subnet-local delivery and its controller graph before use; a remote Store is not
automatically a usable management chunk source. Do not duplicate release policy
or make Root the durable cross-subnet payload repository.

Bootstrap uses the existing reviewed host/effect path to establish Root and Store
under exact hashes, typed creation outputs and bounded budgets, then verifies
Store readiness before admitting dependent application effects. Store bootstrap
and replacement must not depend on fetching their only recovery executable from
that same unavailable Store. Host retains the exact infrastructure executables
and the selected recovery export. Store maintenance preserves catalog, payloads,
staged uploads and pins across two executables; Root retains the maintenance
journal outside Store. A Store outage blocks dependent installation, not normal
application calls or access to Root's retained operation status.

The first preservation observation is a bounded replicated read of one release's
progress and all retention obligations on its shared artifact. It uses existing
Store metadata and Root's typed controller/deadline route; former pin owners and
other labels remain visible. This adds no mirror catalog or maintenance owner.
Separate reads are not an atomic catalog snapshot. Store's persistent admission
gate closes publication, quota, pin and fresh cache work while retaining read
access. It binds a monotonic sequence, operation and actual controller; only the
same attempt/owner can activate it. Reopening storage during upgrade leaves the
gate unchanged. Delayed closure cannot re-close an activated attempt, and stale
activation cannot reopen a successor. Cache continuations retain their admission
generation and cannot continue through a close/reopen cycle. Already-dispatched
remote cache work may still finish: closed admission is not proof of drained calls.
Root forwards this fixed contract without an installation effect or catalog copy.
Its explicit maintenance mounts now retain one immutable approval outside Store,
fence new child/publication mutations and raw gate changes, close Store and issue
stop once. Intent and call enqueue share the local dispatch commit; separate
callback faults qualify recovery through observation without repeating stop.
Root replacement preserves that record. Stopped status under the approved old
code and sole Root control establishes the drain boundary, even when observed
after the dispatch deadline. This does not rely on canister-version arithmetic.
The CUJ6 journal loads recognized predecessors without inventing maintenance
authority; existing full child-custody reads refuse a pending Store attempt so
Host Root maintenance cannot silently omit it.

Before issuing stop, Root captures a bounded ordered metadata commitment over
the closed Store catalog, one release per continuation. Every page checks the
same closed attempt and captured owner. Root retains quota/counters, a digest,
cursor and count; it never mirrors the catalog. Aliases, incomplete-upload
progress and shared pins participate. Strict key progression, unchanged counters,
matching release count and an explicit end page prevent incomplete enumeration.
Concurrent responses to one cursor can commit only once, and Root replacement
preserves progress. This is a metadata preservation boundary, not payload hashing
or a cache snapshot. Old CUJ5 attempts load without this evidence; already-stopped
records and Stores without the new observation cannot acquire it by assumption.

The explicit installation request supplies the approved replacement bytes, bounded
to one 1 MiB management-cache chunk for this first infrastructure path. Host keeps
their durable copy. Root records one upload and one exact chunked-install request,
including the original sender version, without persisting Wasm. Lost upload replies
recover through cache observation; installation recovery verifies the approved
replacement under sole Root control. Issued upload/install/start/activation effects
are never replayed. All fresh dispatch retains the original deadline and operator
authority; post-deadline observations cannot grant fresh restart authority.

Root starts the replacement with admission still closed and repeats the same
bounded scan. Complete matching metadata is required before Verified. Explicit
activation then opens only the exact Store attempt. The installed two-executable
witness additionally compares all stable bytes, including payloads and pending
uploads; runtime metadata comparison is not a full payload integrity scan.
Completion releases mutation fences but retains the operation as historical
evidence. Child-only custody must refuse even this completed record. An explicit
controller-only Root custody read returns both child slots, complete artifact
references and the Store operation synchronously, without effects or a Store call.
Host Root-maintenance approvals can select that operation and require exact
preflight/postflight equality through the existing direct installation path.
The operation's captured owner, deadlines, catalog commitments and uncertain
requests survive unchanged; maintaining Root does not renew Store authority.
Older approvals keep their existing read contract and cannot omit a new operation.
Whole-fleet planning selects this richer approval from authenticated retained
Store-workflow completion; public diagnostics cannot supply it. Subsequent child
artifact sources bind the completed Store attempt and new module. Their complete
Root observation must retain that exact attempt, operator, Store code and Ready
preservation evidence; no old-method probe or fallback is allowed.
No drain observation alone authorizes installation.

Existing fleet plan/apply/status/reconcile now qualify this installed path locally.
A changed Store artifact requires completed child and publication work, with no
other artifact change in the same approval. Host retains one original signed
admission and bounded continuations; Root remains the sole management-effect owner.
Completion points to an admitted signed Ready observation rather than duplicating
the runtime journal. Apply permits at most 32 continuations per invocation and
4096 observations per operation. Reaching a bound retains blocked exposure.
One attempt remains the limit, with no reset, cancellation or successor history.
Live use, larger infrastructure replacements, explicit repair of blocked attempts
and repeatable maintenance need their own admitted consumer paths.

The public Root maintenance query is a diagnostic projection of that same journal,
not another evidence owner. Host selects it from retained Store bootstrap authority
and a known Store identity, preserving the exact network binding. Missing endpoints
and foreign Store reports remain unknown. Fleet views never advance the operation;
an uncertified completed report establishes neither completion custody nor current
Store admission or readiness.

Host reconciliation must not call the effectful generic maintenance advance.
The separate typed recovery contract captures the admitted phase and observes
only already-issued outcomes or the post-install catalog. It may retain evidence
after dispatch expiry, but cannot issue a fresh close, stop, upload, install,
start or activation. Reuse the same Root journal and observation implementation;
do not create a second workflow or renew operation authority.

Deploying an older retained version is a new approved upgrade subject to the
application's schema and lifecycle compatibility checks. Artifact availability
does not make application-state rollback safe.

### Deployment latency and Store transition

Upload each distinct artifact once per fleet, reuse it across children and
upgrades, and rebuild only missing management-cache chunks. Keep bounded transfer
work separate from management-effect concurrency; faster publication must not
introduce competing installs. Measure cold bootstrap, cold upload, warm reuse,
upgrade and interrupted-resume paths separately: wall time, bytes transferred,
management calls, cycles, build time, scheduled waiting and readiness observation.
The acceptance witness includes an unchanged artifact reused by a second child
with no payload re-upload while the verified cache remains available.

The installed publication witness records cold interrupted-upload and warm
version-reuse command timings plus retained chunk payload totals. Its unique-chunk
accounting includes deduplication within a single Wasm. This is a local publication
baseline, not a full deployment or mainnet benchmark; build, bootstrap, scheduled
waiting, installation and cycles still need separate measurements. See the
[qualification guide](../../qualification/README.md) for the existing runner and
report location.

New fleet and individual managed plans dispatch once approved prerequisites are
ready. They bind earliest dispatch to plan creation and expire ten minutes later;
bootstrap, publication and admission consume that same immutable window. Host has
one admission-cutoff calculation: equality with plan creation uses expiry, while
an explicit future timestamp retains pre-dispatch admission. Historical approvals
already have future timestamps and keep that original behavior. No new serialized
mode, field or workflow is required. Deliberate future scheduling remains explicit.
Root accepts an elapsed earliest-dispatch timestamp before original expiry through
the same approval, pin, journal and timer. Creation retains its ten-minute interval
bound; runtime upgrade retains its one-hour bound while Host plans select ten minutes.
Installed local witnesses cover runtime boundaries and the public CLI's default
fleet bootstrap and later child upgrade across Store recovery and Root maintenance.
Older Root executables may reject the new timing: select an explicit future schedule
or separately reviewed Root maintenance first. No endpoint probing, fallback or
deadline renewal is authorized. Existing plans keep exact timestamps, digests and signed attempts;
no performance change silently extends authority. Report phase timings before
claiming a speedup; an extra canister can increase cold-bootstrap cost.

Inline Root deployments retain their 512 KiB per-artifact and 1,048,512-byte
combined custody limits. The Store-backed engine qualifies immutable
references, pin/callback recovery and preserved upgrade history through the same
effect owner. Fleet Host/CLI can publish an explicit immutable release through
an already-bound Root-owned Store using the existing plan/apply/status/reconcile
owners; this grants no installation authority. The approval binds both modules,
principals and exact bytes, with separately retained read-only recovery. Named
child planning now selects a completed publication operation, freezes its immutable
manifest and Store code, and reuses the existing Store-backed creation/upgrade
contracts. Observations validate complete source and pin custody. Fresh whole
fleets publish content-hash releases, converge serial child upgrades and preserve
full Store custody during Root maintenance. One Host-driven Store replacement
qualifies subsequent Root and child upgrades. Inline-to-Store migration and
successor Store maintenance remain pending. See the
[current implementation scope](../demos/wasm-store.md). First prove fresh
Root/Store/app creation, multiple coexisting versions, interrupted upload/resume,
hash/version conflicts, quota exhaustion, cache reconstruction, pin/deletion
races, operator-absent installation and independent Root/Store maintenance.
Adapt the applicable legacy regression cases recorded in the
[Store decision evidence](../qualification/history.md#versioned-wasm-store-decision-2026-11-15).
Integrate through the existing fleet plan/apply/status owners, not a second
deployment engine or an unqualified set of parallel CLI commands.

Existing Root-held artifacts move only through an explicitly qualified custody
transition: verify exact Store bytes and pins, preserve every operation's original
authority and evidence, then prove the upgraded owner uses those references
before retiring its payload storage. Unfinished/uncertain old work blocks a
transition unless that exact state has a preservation proof. Never rewrite old
approvals, reset storage or reinstall to bypass migration. Fresh fleets can use
the new contract before existing-fleet migration is supported.

### Direct calls and optional IC Relay

Support direct platform `call()` and an optional `ic-relay` adapter as explicit application communication choices. Direct calls remain the starting path. Relay batches logical messages over physical IC calls and adds retained delivery state; it is an embedded supplier library, not a required relay canister or a replacement for the management API. Applications select it for named traffic where batching or durable delivery earns its storage and scheduling cost. Applications using only direct calls must not acquire Relay's dependency graph, queues or timers.

Keep these APIs semantically explicit: a direct call has a reply/reject boundary; Relay submission admits retained work whose delivery, business execution, result readiness and consumption are separate observations. Transport acknowledgement does not prove application completion or exactly-once external effects. Keep bounded Wire contracts and application-owned operation identities. Never silently fall back between paths after a timeout, rejection or decode failure; uncertain effects retain their original recovery contract.

Canic composes the supplier through the application's lifecycle root: one admitted Storage owner, one shared Timers epoch and one physical-call owner. Restore the exact queues, reservations, peer bindings and worker declarations before admitting traffic or starting workers. Inactive applications stay inactive. Authenticate the actual peer at receive and preserve that identity for deferred authorization; recheck current permissions and original deadlines before business effects. Relay does not gain deployment, spending or Fleet authority. Select and qualify one compatible supplier graph; the historical integration guide's version pin is not this rewrite's dependency decision.

The first Relay pilot uses two demo canisters and one bounded application command, alongside an explicit direct-call version. It must preserve the same application invariant and qualify enqueue/journal, business/reply and consumption/progress commit boundaries, stopped peers, lost acknowledgements, duplicate delivery, backpressure, queued revocation and upgrades with retained work. Management creation, installation and funding stay on their existing effect contracts. The pilot extends the fleet view with labelled direct/Relay edges and bounded queue, result and recovery observations.

Compare physical calls per completed logical operation, cycles, instructions, throughput, end-to-end latency, retained bytes and idle cost under declared payload sizes, concurrency, batch limits and subnet placement. Include sparse traffic and failures as well as saturated batches; compare equivalent completion and durability guarantees, identifying any additional direct-path journal cost. Orders-of-magnitude efficiency is a workload hypothesis to measure, not a universal speedup or an existing Canic result.

Relay's [adoption contract](https://github.com/dragginzgame/ic-relay/blob/50ef15f286830f1c03cc85f0804f23a622bd09bc/docs/integration/canic-adoption.md), inspected on 2026-11-13, describes experimental supplier primitives and prior consumer evidence. It explicitly leaves managed integration unqualified; legacy Canic references and historical storage upgrade pairs do not qualify this rewrite. The optional module is admitted to the design now. Implement its narrow pilot when the multi-canister application can exercise it, without making Relay a prerequisite for M0 closure or the direct-call journey.

### Separate code identity from deployment approval

The selected starting model is reusable code artifacts plus an authenticated installation binding committed at initialization and activation. A release approval binds exact module hashes, configuration, protocol and intended installations. Qualification evidence has its own digest bound by approval. Changing evidence alone does not require changing executable bytes. Passive metadata is not executable attestation.

The activation transition is `InstalledUnpublished -> BindingAccepted -> Ready -> Published`. The management-effect owner (E in the first fixture) verifies the current module hash through authoritative management evidence, matches it to approval and retains the install result. The installed runtime admits configuration only from its declared installation authority, binding network, resolved principal, allocation generation, configuration digest, protocol and lifecycle generation. The application then supplies readiness for that binding. The publication owner commits the directory/frontend projection only with the approval, resolved target evidence, module verification, binding acknowledgement and application readiness naming the same generation. Changed controllers, module or generation invalidate pending activation evidence. Continuing integrity assumes the explicitly trusted controllers do not replace code outside this contract.

Structural Wasm finalization is excluded from the initial product. A future executable-binding requirement would require a new design decision and replacement plan; it is not a second deployment mode to implement alongside approval bindings.

Artifact publication is atomic and content-addressed. Verify uploads/chunks before installation and bind retention to unfinished operations. A cache can reproduce known bytes but cannot supply missing approval or qualification. Passive inspection preserves externally supplied bytes and reports unknown provenance honestly.

## 13 Persistent releases checkpoints and recovery

### Experimental and production contracts

Canic starts as an experimental system, with preservation guarantees limited to explicitly qualified paths, including the M3 rehearsal. Cross-release state preservation is not implemented by this document. Legacy Canic state, principals, journals and authority are not imported implicitly. Experimental status does not authorize reinstalling to resolve an uncertain effect or discarding state promised by a qualified path.

Before persistent production use, accept and qualify a new release lifecycle contract with these properties:

- State, identity, controller authority, outstanding effects and application availability have explicit preservation requirements.
- Infrastructure and application schema compatibility are assessed separately.
- The application declares readiness and any data migration; Canic cannot infer either from successful installation.
- In-flight operations either settle under their original authority or have a proven successor reader and execution contract.
- A release fences affected work, checks the exact prerequisite state, performs bounded steps and retains resumable progress.
- Mixed-version communication is supported only for explicitly tested adjacent contracts and transition windows.
- Rollback is allowed only when code, data and external obligations make it safe; otherwise recovery proceeds forward.

The first candidate production strategy is a bounded maintenance-window release with affected mutations quiesced, rather than universal rolling upgrades. It reduces the initial compatibility surface. Required availability may force rolling transitions later; decide that from an actual consumer. New product protocol generations are introduced only after that policy decision, never as speculative pre-1.0 lanes.

### Checkpoint and restore

A checkpoint selects an exact live membership generation, including dynamic children. Freeze topology and relevant external producers; ask every participating application owner to reach its declared consistency boundary. Record checkpoint identity and management-effect custody outside the state that will be restored. Individual snapshots are not proof of a consistent distributed checkpoint.

Capture must prove complete artifact transfer and immutable publication. Physical execution status and application admission are distinct: a stopped canister cannot run ordinary acceptance methods, and snapshot execution metadata includes global-timer state ([IC management interface](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/)). The restore protocol is:

1. Persist a recovery fence in application state before capturing the checkpoint. It must cover public work, timers, callbacks and every producer of external effects. Record the checkpoint and fence evidence outside the restored state.
2. Validate network, approved code/schema, controllers and complete artifacts; physically stop the target and load the fenced checkpoint.
3. Physically start the canister into that already-fenced state. Restored timer state cannot authorize ordinary effects. A host promise to call recovery immediately after start is insufficient.
4. Permit only narrowly authorized reconciliation, participant reconstruction and application post-load acceptance. Preserve original paid attempts and compare restored claims against surviving authority.
5. Obtain a fresh lifecycle generation from the surviving authority, accept it, then explicitly reopen normal work. Failed acceptance leaves the application fenced.

Outstanding Blob/funding/auth obligations may prevent reopening. Even a checkpoint captured with no unfinished operations may be unsafe: later completed effects still exist after restoration. Source-reference release requires settled effects and an accepted terminal outcome.

### Authority must survive application restoration

The initial restore boundary is application-only while its control plane remains authoritative. Coordinator/executor snapshot rollback and whole-Fleet historical restoration are excluded until a separately qualified procedure identifies authority and evidence outside the entire rollback set. Merely asking a restored Coordinator cannot establish freshness. Forward infrastructure upgrades still require state preservation and surviving upgrade custody; they do not authorize historical rollback.

| Historical state restored | Required surviving authority or reconciliation |
| --- | --- |
| Grant consumption and capacity allowances | Control-plane liabilities and receipts remain current; spent or uncertain allowance never becomes unused |
| Admission generation and revoked identity | Current policy owner must reauthorize the recovered lifecycle; old admission does not reopen work |
| Allocation-generation counters | Control plane issues unique generations outside the application checkpoint; reuse never repeats an identity |
| Operation receipts and duplicate state | Surviving effect records dominate restored claims; replay uses original provider identity or stays blocked |
| Lifecycle and entropy state | Fresh authorized lifecycle invalidates old callbacks/generators; reconstruct qualified entropy before use |

Before checkpoint support, classify each authoritative field by rollback set, surviving witness and retention horizon. If evidence needed to distinguish post-checkpoint completed work has expired, that checkpoint cannot reopen the affected authority. Application-owned external effects require their own surviving reconciliation evidence; Canic cannot infer them from infrastructure records. Retained checkpoint lifetimes and receipt retention must agree.

The legacy Canic live backup CLI was unavailable in the research baseline; the new design must not imply it can be copied as a working baseline. Canic consumes the upstream engine and qualifies its production provider. Backup, release upgrade, destructive reinstall and disaster recovery remain distinct operations with explicit authority.

### Release to reusable capacity

Adopt the paused FR1 outcome as a first-class managed operation. Retain complete inventory and custody outside retiring canisters before clearing any authority owner. Stop new producers, settle uncertain effects, obtain application data-discard approval, transfer controller custody before wiping a sole controller, then reset and verify empty assets. Preserve observed cycles and controlled ledger accounts.

A surviving same-subnet executor may receive ready capacity. Otherwise assets remain explicitly held by the operator until fresh bootstrap. An executor cannot place itself in its own pool. Recommissioning is fresh admission; old receipts or missing historical state are not automatic prerequisites. Bypass only irrelevant historical metadata, never unresolved spending or custody.

Physical deletion is separate, explicit and irreversible. It requires a disposition and conservation proof; whole-Fleet release must not silently substitute deletion for a failed reset.

## 14 Observation diagnostics and developer tools

Keep Observatory, rendering, comparisons and aggregation on the host. Canisters expose bounded facts, counters, operation summaries and paged inventory. Separate public metrics from protected authority, topology, logs and treasury details. Certificate or authenticated-update requirements depend on whether a result is merely displayed or used to authorize an effect.

Every status states observation age/generation and completeness. An unreachable subnet is unknown, not empty or healthy. Dynamic inventory must include autonomous children. A terminal initial deployment plan is not a perpetual inventory source. Correlate plans, local operations, remote effects and readiness with durable operation IDs.

### Demo fleet visualization from the first multi-canister increment

The first runnable multi-canister demo must include a small browser-based fleet view, delivered with that increment and no later than M2. This is an admitted team-feedback requirement. A local host-served, read-only view is sufficient initially; it uses the same inventory and operation evidence as the CLI and requires no additional infrastructure canister.

Show the actual deployed demo fleet: canister names, principals and roles; distinguish controller relationships from application call/routing relationships. Selecting a canister shows observed lifecycle/readiness, installed module identity, cycles where available, and related deployment or recovery operations. Label the network, observation time and evidence source; missing, stale and unreachable observations remain visibly distinct from healthy or failed states. Do not infer routing relationships or successful effects from a diagram layout.

The team exercise deploys the demo, opens the view, invokes the application, and inspects a worker installation or upgrade. The interrupted-effect exercise must expose the pending/uncertain operation and its eventual retained outcome. As M3 adds autonomous workers, refreshing the same view must discover them and preserve visibility of blocked exposure. Document the launch command and repeatable local/mainnet exercise for each supported deployment path; a static mock diagram does not satisfy the gate.

Keep the initial view limited to these observations. Protected observations retain their access boundary, and browser payloads exclude keys and signed requests. Broader dashboards, historical metrics and application frontend bindings remain separately consumer-gated; this view does not acquire deployment or funding authority.

### Measurements and developer projections

Measure per-role total Wasm and code-section size, validator function count, initial and steady stable-memory allocation, instruction cost, idle timer burn, cross-subnet bytes, queue depth and recovery latency. Measure physical canister overhead and retained reserves separately from logical tenants and consumed cycles. Numeric targets are set against a workload before implementation; historical limits and prices are not universal current constants.

Frontend export is a read-only projection from verified deployment bindings. It names network, application, exact endpoints, protocol and artifact identity, excludes secrets, and cannot make an unready component appear published.

Persistent local development uses the same compiled contracts and effect transitions with an explicitly qualified local provider. Starting does not reset data; reset is separate. Model unavailable creation/funding capabilities explicitly. A local fixture standing in for a Ledger or governance canister proves only that fixture boundary, not live-platform support.

## 15 Qualification and operating budgets

Tests should prove user-visible behavior and recovery boundaries, not preserve the old implementation. Use pure transition and property tests for local state/accounting, native tests for host persistence and artifact admission, and PocketIC for installed lifecycle, management effects and inter-canister behavior. Include a bounded mainnet Hello World exercise in M1 and repeat hands-on exercises as the supported surface grows. Live execution uses explicitly selected authority and budgets; normal automated tests do not spend mainnet funds. Additional live qualification targets named platform gaps. A successful mainnet greeting proves that deployment/call path, not all failure recovery or persistent production guarantees.

| Scenario | Required evidence |
| --- | --- |
| First hands-on deployment | A team member follows public instructions to build, review, deploy and call Hello World locally and on mainnet, inspect its receipt/cost exposure, then deploy a changed greeting with explicit identity/state semantics |
| First Fleet | Build, bootstrap, provision, readiness, publication and terminal no-effect replay |
| Lost responses | Interrupt before/after durable intent, dispatch, reply retention and settlement; resume original effect |
| Concurrent actors | Duplicate commands, competing reservations, late callbacks, changed policy/controllers and allocation reuse |
| Resource limits | At limit and first excess for records, bytes, queues, decoding, capacity and debit; full uncertainty budget refuses new work while protected recovery still settles existing obligations |
| Security | Wrong network/caller/subject/audience/scope/generation; malformed input; mounted endpoint bypass |
| Multi-subnet growth | Local execution, protected placement, partial subnet failure, delegated allowance fencing/reclamation and truthful revocation completion |
| Funding | Exhaustion, fee/rate change, duplicate payment, notification loss and nonzero uncertainty |
| Composition | Auth plus database plus Blob with one memory/lifecycle/timer owner; isolated upstream use |
| Runtime recovery | Trap, instruction exhaustion, stale lease, restore, quiescence and demand rearming |
| Reuse | Complete custody inventory, reset interruption, original-obligation settlement and cycle preservation |
| Early persistence | By M3, two actual executables preserve the M2 application's invariant and retained uncertain-operation evidence |
| Persistent production | Broader release/migration cases, interrupted infrastructure upgrade, surviving custody and accepted restart |
| Checkpoints | Multiple participants, dynamic membership, complete transfer, physical stop/load/start into a persisted fence; post-checkpoint spent/revoked authority cannot revive |
| Governance | Real supported approval path, changed artifact/target rejection and no operator bypass |

The OpenChat-class workload is the strategic qualification target, not a chat product to implement. Include shared MultiUser-style workers, user indexes, groups and communities, mixed tenancy, multiple subnets, sustained growth, local funding and governed effects. Exclude the legacy one-User-canister model from this target, following the retained developer feedback. Reported estate sizes and fees in the old idea are historical input, not current benchmark facts.

Use a small synthetic fixture for failure injection and topology, alongside a named application consumer. The maintainer selected the **Canic profile demo** on 2026-11-13: keep the journey within Canic and keep it simple. Its application owns a stable user ID, immutable owner mapping, private profile and revision, with IC Storage as the first stateful supplier. Pin its inputs and measure the standalone baseline before expanding managed composition. The earlier Toko user-shard candidate is deferred research, not an implementation or measurement gate. Passing a synthetic fixture alone does not establish this hands-on application's adoption or complete M2.

Establish the first deployment baseline in M0 and capture the team's actual M1 experience. Establish the stateful consumer baseline before its implementation and compare equivalent guarantees at M2 and each subsequent expansion. Report advantage, parity, gap, tradeoff or unmeasured. Count application infrastructure removed, integration code added, new permissions, extra canisters, operator interventions and ongoing supplier maintenance. M8 validates accumulated broader value; it is not the first adoption measurement. A smaller diff is not automatically lower burden.

For the first composed consumer, retain a reproducible comparison with and without the selected capability: application source and explicit imports/composition; transitive dependency graph; generated endpoint catalogue; incremental total Wasm, code-section size and function count; initial stable-memory allocation; idle background activity; additional infrastructure canisters and operating duties. Hold toolchain, workload and optimization choices constant and explain unavoidable differences. Set budgets from that baseline before implementation. The Wasm graph must exclude Host, provisioning and estate accounting. These artifacts qualify the small-runtime claim independently of the interaction diagram.

Record accepted workload bounds before claiming scale: occupied subnets, physical assets, logical tenants, active operations, directory rows, peak calls, payload distributions, retention, outage window and funding exposure. Exercise increasing sizes and contention to find limits. Synthetic inventory records alone do not prove the same number of running canisters.

## 16 Delivery plan with deletion gates

These are implementation milestones after design acceptance, not a version allocation or authorization to release the old repository. Each is a coherent outcome with its own narrow checks and a team-usable result. Deliver runnable increments within milestones instead of withholding the CLI until the architecture is complete. M0 produces a concrete walkthrough and small executable provider experiments; M1 completes the first product deployment loop. Do not create a package or framework solely to prepare for a later milestone.

The [M0 decision and evidence record](../qualification/history.md) scopes the initial investigation. Section 17 governs admission: a later milestone names a possible delivery boundary, not permission to implement every integration with a documented owner.

The first fleet CLI finish line is M2: initialize an app/fleet, build and review
one plan for its declared tree, apply it, repeat with no unnecessary effects,
inspect progress and recover supported interruptions through CLI commands,
then upgrade while preserving the application's invariant. The same definition
must serve a second isolated fleet. The initial whole-fleet path now deploys a
new Root with one or two direct children, resumes the same authority, and derives its next child
upgrade from retained completion, retaining at most eight upgrades per child. Matching
artifacts report no change against
retained completion, not freshly observed network state. A separate Root-only
maintenance plan now preserves the full completed journal through the direct
installation owner, then advances the Root binding for future child plans.
The replacement must support the controller-only full-custody observation.
One retained single-upgrade predecessor is admitted by exact module hash with
separate authenticated creation/upgrade preflight reads and the same full-custody
postflight. Unknown older owners, unfinished work and combined Root/child changes
are refused; a missing endpoint never authorizes a fallback.
Unbounded upgrade history, existing-fleet adoption and larger trees remain open. `fleet reconcile`
requests recovery of an exact retained creation/upgrade approval on a compatible
Root, retaining original effect limits and ordered signed-attempt custody.
Maintenance of active/uncertain work, other old-owner transitions and Root replacement
still require separately qualified controller paths. These are remaining
implementation requirements, not claims made by the existing command names.
Subtree administration and the optional allocation patterns in section 8 extend
this boundary in M3; each selected pattern includes its maintenance proof.

| Milestone | Deliverable | Completion and subtraction gate |
| --- | --- | --- |
| M0 First deployment contract and baseline | Specify the Hello World walkthrough and finite budget; choose its topology/controller graph, approval representation and provider contracts; identify the later stateful consumer and preservation requirements | Exercise the selected operator management path and creation-result custody in small fixtures; classify unavoidable uncertainty; defer unrelated governance/composition decisions to their owning implementation gates |
| M1 Hands-on Hello World | Deliver usable CLI/artifact inspection and a narrow build, plan, apply, call and status journey locally and on mainnet, then deploy a changed greeting | Team follows public instructions; exact approval and retained effect custody, explicit identity/state semantics, bounded spending and truthful status; one parsing/artifact identity and operation path; passive preview alone is insufficient |
| M2 Composed managed application | Reusable app definition, generated Root/Store/app starter and persistent local loop; versioned artifact custody; whole-fleet convergence; real consumer state, stateful supplier composition, cascading discovery and runnable fleet visualization | Same app definition serves two fleets; repeated convergence has no mutations; Store publication, retention, maintenance and warm-reuse evidence from section 12; exact authority and interrupted-effect recovery; explicit memory/timer ownership and first adoption comparison; no second provisioning path |
| M3 Autonomous bounded operation and early persistence | Finite grants, composable growth, subtree drain/suspend/resume and narrow persistent subtree upgrade with surviving operation evidence | Sibling isolation, descendant acknowledgements, concurrent growth fencing, operator absence, late replies and recovery after upgrade; no duplicate retry/funding owner; live Rust futures need not migrate |
| M4 Multi-subnet scale, when admitted | Consumer-required coordination, dynamic inventory, shared tenancy and explicit delegation reclamation | Admit the consumer's multi-subnet need first; unreachable executor cannot release allowance; fence/reconciliation and selected revocation semantics are proven; no central round trip per local continuation |
| M5 Expanded composition, when admitted | Complete the selected Auth, Blob or frontend journeys building on M2's composition | Admit each integration's consumer need separately and qualify its selected arrangement; supplier independence; no copied engine or second lifecycle owner |
| M6A Reusable capacity | Independently qualify release/reset and retained custody | Original obligations settled, late callbacks fenced and cycles conserved; checkpoint qualification does not gate this outcome |
| M6B Application checkpoint/restore | Independently qualify consistent checkpoint provider and fenced recovery | Surviving control-plane authority, explicit stop/load/start/accept/open and no resurrection of post-checkpoint spent authority; no whole-Fleet rollback claim |
| M7 Production lifecycle and governance | Complete maintenance-window lifecycle and selected governance profile qualification | Broader migration, infrastructure recovery and actual governed effects using early contracts; production claim permitted only here |
| M8 Broader adoption value | Representative additional consumers and accumulated baseline comparisons | Measured net responsibility removed, including suppliers; reject extensions without demonstrated value |

Record M7 preservation requirements during M0 and exercise their narrow persistence path by M3. Complete governance-specific feasibility before implementing that integration, without making it a prerequisite for the direct-operator experiment. M1's direct public greeting does not require Store, Coordinator, Auth or IcyDB. M2 extends the existing effect path with the selected per-fleet Store and stateful composition; the Store decision does not retroactively invalidate historical M0/M1 evidence. There is no throwaway second deployment engine.

Every later milestone extends a runnable team journey and updates its example, operator instructions and limitations. M6A and M6B are separately reviewable outcomes; shared custody principles do not couple their completion gates. Expansion depends on preceding hands-on and consumer evidence: if M2 adds burden without a credible reduction, revise the architecture before building the larger management surface. Reorder optional adapters for demonstrated consumer needs while retaining dependency and recovery proofs.

No mechanical port of `canic-core`, `canic-host` or their tests is a milestone. Bring forward behavior requirements and independently author the new implementation. The original repository remains intact as evidence and a separate product until an explicit replacement decision. Migration of existing deployments is not silently included.

### Preventing the next sediment layer

Maintain one current architecture document, current executable examples, generated reference from actual schemas, and a small decision log for consequential alternatives. Historical designs become immutable evidence and lose current authority. Each feature guide names its supported maturity and links to the executable acceptance journey.

For each milestone, review: new durable state, public concepts, recovery branches, generated surface, dependencies, idle cost and code removed from the planned ownership graph. A second writable source of truth is a design defect until justified. A warning suppression, compatibility alias or fallback cannot substitute for completing a cut. Delete abandoned prototypes in the same batch that selects a direction.

Use Toko's tooling and upstream implementations for generic mechanics. Keep targeted validation evidence with the owning change. Do not turn exact prose, directory names or historical task counts into product safety gates. Test structured invariants and actual behavior.

## 17 Complete capability disposition register

Section numbers identify architectural ownership, not a promise to implement every historical capability. The source record identifies the inventory inputs. The initial product starts with the M1 Hello World deployment loop and grows into the M2/M3 stateful consumer path; wider qualification milestones apply only to capabilities admitted for a demonstrated need. Required rows span that sequence and do not all gate the first deployment. Each row has a separate initial disposition:

- **Required**: required for the first consumer; implement only the stated narrow scope.
- **Supplier-owned**: integrate the independent implementation only when the consumer needs it; count its maintenance cost.
- **Consumer-gated**: retain design evidence; implementation requires a demonstrated consumer.
- **Excluded**: no Canic implementation planned under the current design.
- **Historical**: evidence about old deployments, with no new product commitment.

A future architectural home does not move a Consumer-gated row into the backlog. M0 confirms the narrow M1 subset; the stateful consumer confirms the additional M2/M3 requirements before their implementation. Supplier ownership does not itself schedule an integration.

| Existing surface or responsibility | Disposition and destination | Initial product disposition | Acceptance witness |
| --- | --- | --- | --- |
| Facade, start/build macros, endpoint macros | Canic owns actor/endpoint declarations; Platform owns generic IC entrypoint macros; Wire owns codecs, sections 4, 10, 12 | Required — thin explicit composition | Generated exports match declarations; one Platform execution path; no separate Wire endpoint owner |
| Runtime core and universal layers | Replace with owned domain modules, sections 2–4 | Required — small runtime only | Small leaf dependency and Wasm graph |
| Contracts, identifiers and DTOs | Retain bounded semantics; reduce redundant representations, sections 3, 12 | Required — protocols actually spoken | Codec bounds and identity tests |
| App config, roles, capabilities and environment | Replace redundant controls with explicit selections and derived facts, section 5 | Required — minimal input model | Invalid combinations fail before effects |
| Specs, groups and deployments | Reusable app definition instantiated by fleets, sections 3, 5, 8 | Required — app blueprint and explicit instances; generic groups remain consumer-gated | Two fleets share definitions without sharing identities, journals or budgets |
| Coordinator and Registry | Conditional global authority with bounded delegation, sections 4, 6, 8 | Consumer-gated — global constraints | Demonstrated global constraint and membership/reservation concurrency |
| Root control plane | One surviving Root/management owner controls the generated starter's app, sections 4, 7–9 | Required — selected starter topology; general executor behavior still needs qualification | Installed management journey and custody/artifact availability during operator absence |
| Wasm Store and versioned artifacts | Separate dynamic per-fleet Store; immutable releases, selected-version references and retention, sections 4, 12 | Required — selected managed-fleet architecture; generic template engines remain consumer-gated | Resumable publication, coexisting versions, pins, cache reconstruction, maintenance and measured cold/warm deployment costs |
| Network enrollment and subnet catalog | Retain exact trust and typed evidence, sections 5–6 | Required — selected network and subnet | Wrong network and stale evidence refusal |
| Fleet generation and Ensure | One compiler and plan/apply owner, sections 5, 7 | Required — whole selected fleet; current CLI qualifies Root with one or two direct children | Repeated convergence has no effects; interrupted apply retains exact targets |
| Infrastructure bootstrap and supplied IDs | Explicit retained operation, section 7 | Required — bootstrap; supplied-ID breadth consumer-gated | Lost reply and controller mismatch |
| Capacity import, reset and recycling | Rebuild around asset/allocation identity, sections 8, 13 | Consumer-gated — reusable-capacity need | Reuse with late callbacks and exact custody |
| Component lifecycle and application initialization | One provisioning/readiness operation, sections 7, 10 | Required — first stateful consumer | Installed but unready stays unpublished |
| Singleton slots and child trees | Explicit per-parent cardinality over shared allocation, sections 8–10 | Required — fixed child path now; general singleton admission qualifies with its consumer | Concurrent requests cannot create a second live slot; upgrade retains identity |
| Scaling workers | Bounded interchangeable workers over shared allocation, sections 8–10 | Required — optional building block; general pools consumer-gated | Capacity race, idle quiescence, drain/readiness and bounded rollout; no implied data replication |
| Sharding | Composable shared-capacity assignment; application owns data movement, section 8 | Required — optional building block qualified with a selected consumer; general policies consumer-gated | Commit-time capacity, stable assignments across upgrade, old release cannot alter replacement |
| Keyed instances / placement indexes | One dedicated allocation per scoped key, section 8 | Required — optional building block qualified with a selected consumer | Same-key concurrency, exact pending recovery, wrong-parent refusal and binding/data preservation across upgrade |
| Service publication and Group Directory | Cascading generation-bound projections and scoped local lookup, sections 3, 8, 10 | Required — tree discovery; global directory consumer-gated | Dynamic children, repeated roles, exact replay and stale/conflicting projection refusal |
| Fleet ingress whitelist/admission | Optional deployment policy with selected consistency semantics, section 6 | Consumer-gated — extra Fleet admission | Application access works without it; selected revocation promise is qualified |
| Caller/topology access checks and RPC replay | Retain explicit authority and effect identity, sections 6–7, 12 | Required — selected commands and effects | Wrong caller, stale generation and duplicate |
| Optional IC Relay communication | Independent supplier plus explicit Canic adapter alongside direct calls, sections 11–12 | Supplier-owned — design admitted; narrow multi-canister pilot before adoption | Managed lifecycle/recovery proof and measured direct-call comparison; no automatic fallback |
| Inspect-message and payload limits | One generated endpoint catalog, section 6 | Required — endpoint enforcement and bounds | Bypass still denied at endpoint |
| Root proofs, attestation and issuer enrollment | Retain narrow infrastructure identity; delegate generic Auth, section 11 | Consumer-gated — enrollment requirement | Enrollment/revocation/renewal journey |
| Application login/session/receiver/logout | IC Auth composition, section 11 | Supplier-owned — IC Auth; full journey M5 if selected | Browser, issuer, receiver and device logout |
| Root/Coordinator and child funding | Unified grant mechanics with distinct purposes, section 9 | Required — direct-cycle funding path | Reservation, accepted credit and no duplicate |
| ICP refill and conversion | Removed from product scope, section 9 | Excluded — operators supply cycles manually | No conversion or alternate-account fallback; cycle-provider receipts remain required |
| Funding policy rotation and kill switches | New authority cannot reset old exposure, sections 6, 9 | Required — exposure survives authority changes | Revoke new work; settle original uncertainty |
| Stable memory and allocation manifest | IC Storage ownership, section 10 | Supplier-owned — IC Storage; narrow integration required | No unused-role allocation; schema reopen |
| Lifecycle restoration, readiness and timers | One composition root plus durable demand, section 10 | Required — first consumer and persistence rehearsal | Restore, trap, stale callback and watchdog |
| Entropy and random streams | Independent supplier, explicit fresh/buffered contract, section 10 | Supplier-owned — only selected entropy needs | No fallback or cross-lifecycle reply reuse |
| Metrics, logs, timers and provider observations | Bounded shared providers, section 14 | Supplier-owned — minimum operational evidence | Label/retention limits and truthful cost |
| Public observability and compact diagnostics | Public/protected split and typed errors, section 14 | Required — recovery status and safe errors | No sensitive authority leakage |
| Demo fleet visualization | Small host-served read-only view of actual topology and operation evidence, section 14 | Required — first multi-canister increment, no later than M2 | Runnable team exercise; truthful stale/unknown states; autonomous workers appear when supported |
| Host Observatory, costs and reports | Host-only aggregation, section 14 | Consumer-gated — broader aggregation | Partial results never become complete |
| Blob service and embedded/dedicated adapters | Independent engine and explicit composition, section 11 | Supplier-owned — Blob integration when needed | Real provider/application integration |
| IcyDB integration | Database owns semantics; explicit host composition, section 11 | Supplier-owned — integrate only for a demonstrated database need; profile demo starts with IC Storage | Database readiness and lifecycle recovery |
| Backup/checkpoint/authority snapshots | Future production adapter to IC Backup, section 13 | Consumer-gated — consistent application restoration | Multi-participant consistent restore |
| ICRC standards | Explicit external adapter only, section 11 | Consumer-gated — named standard requirement | Required external schema and guard tests |
| Build tools, generated packages and caches | Delegate mechanics; bind exact artifacts, section 12 | Supplier-owned — build mechanics; artifact binding required | Cold/warm, mutation and concurrent publication |
| Evidence envelopes and policy gates | Passive inspection plus explicit approval, sections 5, 12 | Required — canonical approval and evidence | A report cannot authorize mutation |
| Frontend environment handoff | Verified read-only projection, section 14 | Consumer-gated — explicitly deferred by maintainer; no current frontend work | Wrong network/protocol and unready targets if later admitted |
| Local replica and persistent local Fleet | Same semantics, explicit provider capabilities, section 14 | Required — narrow installed fixture provider | Start preserves; reset is explicit |
| Medic, info, inspect, diagnostic, state and list | Consolidated observation/operation vocabulary, sections 5, 14 | Required — status/reconcile; broader commands consumer-gated | Errors identify actual next owner/action |
| Scaffold, local workspace status and toolchain installation | Small Root-controlled app template and explicit preparation, sections 1, 5, 12, 14 | Required — generated Rust starter and persistent local loop; broader templates remain gated | Generated consumer builds and deploys; restart preserves state; checks never install tools implicitly |
| Token balance and transfer wrappers | Delegate generic token tools; retain only Fleet-bound financial operations, section 9 | Supplier-owned — no generic Canic wallet | No duplicate generic wallet; exact signer/account boundary |
| Fleet/subtree state cascades, suspend and resume | Generation-bound scoped lifecycle transitions, section 10 | Required — project-subtree operation journey | Sibling isolation, descendant acknowledgement, stale-command refusal and original-effect recovery |
| Test support, probes and downstream fixtures | Public consumer journeys in qualification workspace, section 15 | Required — public first-consumer fixtures | No private-core reconstruction |
| CI, releases, packaging, docs and scripts | Toko mechanics plus product acceptance, sections 15–16 | Required — selected graph and current documentation | Reproducible selected graph and complete docs |
| Release, retirement and destruction | Separate reuse, persistent release and deletion, section 13 | Consumer-gated — reuse/destruction; early persistent release required | Custody, application and conservation gates |
| Universal core, hidden lifecycle ownership and structural finalization | Remove from starting architecture, sections 2, 10, 12 | Excluded | Consumer requires no omnibus core or executable rebinding mode |
| Old metadata repair, cancelled migration schemas and obsolete release modes | Preserve rationale in source corpus only, sections 13, 18, 20 | Historical | No repair or migration obligation inferred from historical coverage |

## 18 Deferred ideas and paused slices

All twelve topics are accounted for. Their old release positions are not carried forward. Historical implementation suggestions are inputs, not mandatory designs.

| Source topic | Canic decision | Gate or reason |
| --- | --- | --- |
| Bounded multi-Fleet estates | Retain indexed local estates; defer cross-Fleet disposition | Q0 physical capsule/credit/deletion proof before any production transfer; ordinary multi-subnet operation does not need it |
| Cross-subnet transport groundwork | Adopt resource-bounded single attempts and explicit route evidence; choose response-wait policy per effect | Actual workload and financial recovery proof; bounded resources do not imply universal bounded-wait transport |
| Declarative authentication profiles | Adopt compiler-owned profile direction | Reuse IC Auth identity/session semantics; prove scopes and revocation |
| Demand-driven pool maintenance | Adopt durable-demand scheduling | Prove every rearm path and independent recovery liveness |
| Estate budget replenishment | Include as a bounded funding purpose | Real autonomous growth requirement, exact accounts and finite renewed grants |
| Immutable test checkout lease | Defer optimization | Measure remaining resolution cost; enforce real immutable source before shared reuse |
| Inspect-message admission | Retain implemented direction, rebuild thin catalog integration | Endpoint checks remain authoritative; stale-state and recovery-access tests |
| OpenChat-class support | Adopt strategic acceptance workload and incremental adoption goal | Shared tenancy, governance, persistent lifecycle and measured burden reduction |
| Operator funding conversion | Excluded; operators supply cycles manually | Maintainer decision 2026-11-14; no conversion adapter or fallback |
| Release-binding finalization | Select separate code and approval identities; exclude structural finalization initially | A new executable-binding requirement needs a fresh decision, not a second implemented deployment mode |
| Role-specific stable initialization | Adopt from first implementation | Physical allocation measurements and observation paths that do not allocate |
| Swappable Wire encoding | Preserve narrow boundary; defer second codec | Current Wire policy remains; explicit policy decision and full two-build proof needed |

| Maintained or paused line | Requirement carried forward | Historical mechanism not inherited |
| --- | --- | --- |
| 0.103 role-owned endpoint surface | Bounded role-specific commands/status | Candid naming and omnibus enum shape |
| 0.104 timer/lifecycle composition | One shared provider and synchronous restoration | Canic-owned timer mechanics |
| 0.105 local authorization | Framework-neutral local application guards | Superseded session/token internals |
| 0.106 platform qualification | Physical creation, costs, controller/placement and throughput evidence | An unrun live qualification reported as complete |
| 0.107 preflight/admission | Input-complete planning and typed catalog failures | Independent per-canister whitelist authority |
| 0.108 Root funding | Replay-safe accepted credit, bounded grant and exact rotation | A separate funding framework for every role |
| 0.109 Fleet ingress | Explicit policy ownership and truthful selected revocation semantics | Mandatory Fleet whitelist, unconditional fencing and historical cadence |
| 0.110 B1/B2 contraction and B3/B4 stops | Small runtime and measured generic/code/storage cost | More blanket codecs, wrappers or hardcoded old size targets |
| 0.110 FI1 bootstrap/import | Explicit prerequisites, custody, bounded reset and exact retry | Implicit infrastructure bootstrap during import |
| 0.110 RD1 repair-before-reset correction | Reset depends on current safety facts | Repairing obsolete metadata just to erase it |
| 0.110 FR1 paused release to capacity | First-class reusable-capacity outcome, section 13 | Deletion-heavy retirement and self-owned pools |
| 0.110 CS1 paused simplification | Single owners for bootstrap, parsing, formatting and persistence mechanics | Porting duplicated old flows before consolidating them |
| 0.110 OP1–OP4 | Component operations, frontend export, host Observatory and local Fleet | Separate journals and orchestration per CLI command |
| 0.110 RF2 and staging import follow-ups | Bounded observation and typed exhaustion; conversion receipts remain historical | Conversion implementation, balance-only proofs and replacement recovery budgets |
| 0.110 CR1 | Canonical infrastructure entrypoints | Application-maintained infrastructure source copies |
| 0.112 Blob/Wire and upstream ownership | Independent services and coherent protocol changes | Embedded service engines and stale Auth/Backup availability claims |
| Cancelled stateful-adoption proposal | Persistent production need reconsidered explicitly in section 13 | Its cancelled schemas, schedule or implied migration authority |
| Older archived designs and post-46 backlog | Historical rationale through the capability register | Automatic resurrection of superseded product modes |

## 19 Decisions required before implementation

The architecture is accepted as the basis for M0. The choices below still require their named evidence before implementation relies on them; architectural acceptance does not establish qualification.

| Decision | Recommended answer | Proof or consequence |
| --- | --- | --- |
| First hands-on workload | Team-operated Hello World locally and on mainnet, through the public product path | M0 resolves first-effect contracts; M1 includes deployment, invocation, inspection and a changed greeting; feedback shapes subsequent increments |
| First stateful consumer | Canic-owned profile demo with IC Storage; Toko integration excluded from current scope | Measure standalone baseline and bounded ownership invariant; M2 measures managed composition; second executable preservation by M3 |
| Production promise | Persistent applications are the target; the initial system is experimental, with preservation guarantees limited to explicitly qualified paths | Define lifecycle preservation requirements in M0; prove the M3 path; require M7 before general production claims |
| First persistent transition | Bounded maintenance-window release | Specify tolerated downtime, schema migration and uncertain-effect custody |
| Governance profile | Direct operator for M1; hierarchical governance over infrastructure owners as proposed in section 6 | M0 proves the operator path; prove governance feasibility before that integration; direct SNS registration of every descendant would require a different effect owner |
| Artifact binding | Separate executable identity from release approval; one activation evidence chain | Structural finalization excluded from initial product |
| Restore boundary | Application rollback with surviving authoritative control plane | Whole-Fleet/control-plane rollback needs a separately qualified external authority/evidence procedure |
| Physical infrastructure | Root controls a per-fleet Wasm Store and application canister; external operator controls Root | Qualify Store bootstrap, versioned custody and maintenance through the existing effect path; Coordinator remains conditional; current Root-held payloads require qualified migration |
| Fleet admission | Optional extra deployment policy | Application authentication works independently; select propagation semantics before adding machinery |
| Package and repository boundaries | One product repository/workspace with separate host/runtime graphs | Split further for demonstrated compilation, dependency or ownership needs |
| Application framework ownership | Application owns lifecycle and participant composition | Combined framework fixture before public composability claims |
| Endpoint ownership | Canic composes actor endpoints; Platform owns generic IC entrypoint macros; Wire owns codecs | Migrate existing consumers before retiring Wire Endpoint; preserve protocol, authorization, admission and reply semantics; optional Testkit Wire remains test-only |
| Funding autonomy | Manually supplied cycles with finite distribution grants, reactive funding plus coarse idle coverage | Budget exhaustion pauses; no ICP conversion or hidden unlimited authority |
| Protocol optionality | Wire initially; keep domain free of codec choices | Alternate owned Candid requires explicit policy decision |
| Application communication | Direct calls plus optional IC Relay adapter | Qualify a bounded multi-canister pilot and workload-specific gains before adoption; no mandatory Relay dependency |
| Capacity reuse versus deletion | Preserve controlled empty capacity by explicit choice | Retained custody and costs must be visible; deletion never a recovery fallback |

No capacity, cost or schedule estimate is invented here. Set measurable budgets before executing the relevant milestone, and record rejected alternatives beside the decision rather than building them all.

## 20 Source record and confidence

The research baseline is the predecessor checkout, then named `canic` and now hosted as `canic-legacy`, at HEAD `fa92d886ef9beee824de3ec900832722fef85bd2`, inspected on 2026-11-13 with substantial uncommitted changes. Its working-tree documents and source declarations were used; this is not a clean-commit implementation audit. [Source inventory](source-inventory.tsv) records file hashes for the research corpus so later readers can detect changed inputs. Inclusion records the corpus, not a claim of exhaustive line-by-line review.

Primary local sources, relative to that legacy checkout:

| Key | Sources and contribution |
| --- | --- |
| S1 | `README.md`, `CONFIG.md`, `Cargo.toml`, `AGENTS.md`, `docs/status/current.md`: product vocabulary, manifests, safeguards and current caveats |
| S2 | `crates/*/src/lib.rs`, Core `src/api/mod.rs`, CLI `src/cli/help/mod.rs`, CLI README: exported responsibility and command inventories |
| S3 | `docs/features/`: runtime, auth, scaling, orchestration, artifacts, backup, Blob and operator journeys |
| S4 | `docs/architecture/independent-package-adapters.md`, architecture and contract indexes: supplier ownership and historical layering |
| S5 | `docs/design/ideas/*/design.md` and ideas index: all twelve deferred concepts, including updated inspection status |
| S6 | `docs/design/0.106-*` through `0.110-*` and `0.112-*`: retained guarantees, paused slices and later amendments |
| S7 | Design index and named archived 0.103–0.105/cancelled adoption references: historical direction, not automatically current requirements |

Examples of conflicting source status matter to the rewrite: the ideas index classifies topics as deferred while the inspection note reports active implementation; the adapter architecture still calls application Auth unimplemented while the newer Auth guide describes installed browser acceptance; the 0.112 plan preserves restoration machinery that the later Backup guide says was removed. Numbered designs also retain superseded release positions. These are reasons to separate current code, qualification evidence and proposed direction—not reasons to assert that every old design is wrong.

The review incorporated on 2026-11-13 also checked the linked primary IC execution properties, management interface, CMC interface and SNS Root registration source. Those platform references are distinct from the frozen historical corpus; moving source documents do not prove a particular deployed canister version. M0 and each provider adapter must pin the interface/implementation evidence relevant to their qualification. The read-only Toko manifest observation identifies a consumer candidate only; it neither expands the historical inventory nor proves consumer approval or compatibility.

This document resolves ownership proposals, not those old release claims. Canic now has a direct-host CLI and narrow profile canister/manager implementations with the qualified paths recorded in M0; it has no general production management framework. No legacy test result establishes a new Canic guarantee. Each implemented responsibility must satisfy its own acceptance witnesses with the selected source, suppliers and installed fixtures.
