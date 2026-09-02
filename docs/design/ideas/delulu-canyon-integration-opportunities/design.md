# Idea: Delulu Canyon Integration Opportunities

Date: 2026-09-02

## Status

- Classification: deferred, unnumbered supplemental idea. It is not a
  scheduled release, implementation authority or downstream integration
  commitment.
- Purpose: use Delulu Canyon as a concrete requirements probe for generic
  Canic capabilities that could help a persistent, multi-Canister Internet
  Computer game.
- Relationship: this document supplements the
  [language-neutral managed-guest feasibility idea](../language-neutral-managed-guest-feasibility/design.md),
  the broader [managed-guest exploration](../language-neutral-managed-guest-feasibility/exploration.md)
  and the [product frontend delivery idea](../product-frontend-delivery-handoff/design.md).
  It does not duplicate or promote them.
- External-source boundary: the assessment uses only Delulu Canyon's public
  repository and player-facing documents as reviewed on 2026-09-02. It is not
  a source-code, deployment, security or economic audit.
- Repository boundary: Delulu Canyon and every other external repository
  remain read-only. This idea grants no authority to modify or depend on them.
- Product boundary: every resulting Canic feature must remain application-
  neutral. No Delulu-specific role, token, world, endpoint or economic rule
  belongs in Canic.
- Financial boundary: Canic may improve deployment, authority, recovery and
  observability around a value-bearing application. It does not validate the
  peg, token value, market design or fitness of the game economy.

## Executive Answer

Delulu Canyon is a strong architectural fit for Canic's intended control-plane
problem, but not yet a direct integration target.

Canic could eventually own how the game's Canisters are built, identified,
placed, admitted, funded, observed and recovered. The game must continue to own
movement, combat, quests, player identity, world transfer semantics, social
policy, custody accounting and its economy.

Two current facts block a credible production adoption:

1. Delulu Canyon is presented as a Motoko application, while Canic's managed
   role declaration, Cargo qualification, build identity, lifecycle endpoints
   and runtime implementation are Rust-specific.
2. Delulu Canyon is live, persistent and value-bearing, while Canic's pre-1.0
   release boundary is reinstall-only and promises same-release recovery rather
   than cross-release application-state preservation.

The first fact is the exact subject of the existing managed-guest feasibility
idea. The second must remain an explicit adoption limit; this supplemental idea
must not weaken Canic's current pre-1.0 hard-cut policy.

## Public Application Shape

The public documents describe a distributed application with these relevant
properties:

- a browser client and realm that deploy independently and report separate
  versions;
- multiple persistent world Canisters, currently Earth, Water and Solar
  System;
- a Wayhouse that registers worlds and escrows one player while transferring
  them between worlds;
- a global Social service for chat, whispers, presence, parties, moderation
  and unread state;
- a Keeper that samples a market periodically and executes a six-hour economy
  epoch;
- ordinary ICRC ledgers plus separate custody, liquidity and governance
  services;
- application-level reconciliation for escrowed value, pending payouts and
  partially completed player operations;
- player-facing capacity selection that offers an available, quieter world;
  and
- production lessons involving a permanently stopped heartbeat, duplicate-
  request identity, stale response ordering, incomplete registration,
  interface drift, ingress cost and silent inter-Canister failures.

These are useful requirements because they exercise topology, authority,
durable operations, scheduled work, release identity and operational recovery
without asking Canic to understand game rules.

## Current Canic Fit

| Application pressure | Relevant Canic capability | Current limit |
| --- | --- | --- |
| Multiple worlds and shared services | Component Specs, Component Groups, bounded placement, Component children, Fleet Registry and runtime Directories | Canic can own identity and topology; it cannot partition world state, move a player or choose the game's admission policy. |
| Cross-world and cross-service calls | Fleet admission, exact role/parent bindings, endpoint guards, delegated subjects and typed calls | The managed runtime and supported bindings are Rust-specific; application authorization remains distinct from infrastructure authority. |
| Independent service releases | Qualified Wasm and Candid artifacts, release-set identities, Wasm Store publication, reviewed Fleet Ensure and passive evidence gates | Motoko/Mops artifacts are not yet qualified managed roles, and Canic does not currently bind a browser client's generated interface to a deployed release. |
| Long-running value operations | Canic's internal intent-before-effect journals, operation identities, exact replay and retained receipts | These semantics protect Canic-owned infrastructure workflows; there is no generic application-operation kit. |
| Keeper and heartbeat schedules | Shared `ic-timers` inventory, synchronous lifecycle composition and runtime observations | Ordinary application timers remain application-owned and fail-stop after a trap or instruction exhaustion. |
| World availability and cycles | Bounded Fleet placement, protected funding, exact balance observations and operator diagnostics | The Registry does not carry application readiness, live occupancy or a soft capacity projection. |
| Persistent recovery | Snapshot manifests, hashes, topology-aware selection and resumable same-release recovery | Mixed Rust/Motoko qualification is absent, and pre-1.0 recovery is not a cross-release state-preservation promise. |

Canic's current capability owners remain:

- [scaling and placement](../../../features/scaling-and-placement/README.md);
- [authentication](../../../features/authentication/README.md);
- [Fleet orchestration](../../../features/fleet-orchestration/README.md);
- [runtime and timer reliability](../../../features/runtime/README.md);
- [builds and evidence](../../../features/build-and-evidence/README.md); and
- [backup and restore](../../../features/backup-and-restore/README.md).

## Blocking Truths

### Managed Motoko Is Not Opaque Wasm Installation

Accepting a caller-provided `.wasm` path would let Canic hash and install some
bytes. It would not establish a managed Canister.

A managed Motoko Component needs the same independently observable lifecycle,
Directory, activation, authority, retry, readiness and child-request contract
as a Rust Component. The existing managed-guest feasibility idea owns the proof
for one compact language-neutral contract, an isolated Mops build and a real
Motoko lifecycle fixture.

This case study adds motivation, not an alternate shortcut or Motoko-only Root
protocol.

### Persistent State Cannot Be Hand-Waved

The public game describes player state, inventories, quests, custody claims,
parties and economy records that must survive ordinary product evolution.
Canic's current same-release interruption, retry, backup and restore guarantees
are useful but do not make a later Canic release state-compatible.

A live game could trial a fixed Canic release, but it should not depend on
moving pre-1.0 Canic releases until a separately approved post-1.0 continuity
contract exists. This idea does not schedule or design that contract.

### There Is No Cross-Canister Transaction

World transfer, a ledger payment, a database mutation and a caller-side state
change are separate messages. A completed callee mutation may survive a caller
trap or lost response. Canic must never advertise a helper as distributed
atomicity.

The reusable framework contribution is durable identity, exact replay,
observable status and explicit compensation boundaries. Delulu Canyon retains
ownership of what a valid journey, refund, payout or player recovery means.

## Candidate Generic Work

The priorities below are analytical only. They do not create a release order or
implementation authority.

### P0: Execute the Managed-Guest Feasibility Proof

The existing M0 design is the prerequisite for every direct Motoko benefit. It
should prove:

- one compact managed-guest Candid contract shared by Rust and Motoko;
- root-frozen bytes and digests without hashing a guest re-encoding;
- exact, isolated and source-non-mutating Mops build evidence;
- an explicit Motoko actor scaffold rather than hidden source rewriting;
- lifecycle prepare, synchronize, activate, status and application readiness;
- exact retry, conflicting operation rejection, restart and same-contract
  upgrade; and
- semantic DID conformance in PocketIC.

A Delulu-shaped fixture may model an ordinary world role and a non-custodial
shared service, but it must use generic names and test contracts. Canic must not
copy external game source or turn the feasibility proof into a live integration.

### P1: Language-Neutral Application Operation Receipts

Canic should investigate a small application-facing operation protocol with:

- a caller-owned bounded operation ID;
- a canonical semantic request digest;
- durable acceptance before the first external effect;
- explicit prepared, issued, terminal and recovery-required observations;
- exact replay returning the retained result;
- conflicting reuse failing before mutation;
- bounded terminal receipt retention and acknowledgement; and
- no controller, management-Canister, Fleet mutation or hidden retry authority.

Useful motivating journeys include partial account registration, world
handover, ledger-backed purchases, reward payout, user-crate refund and an
idempotent IcyDB service mutation. These examples do not make their application
semantics part of the framework.

### P1: Critical Application Job Health

Canic should investigate a bounded contract through which an application can
declare a recovery-critical job and report:

- stable job identity and owner;
- last accepted start and successful completion;
- current attempt generation and lease, if any;
- next expected deadline;
- consecutive failure or overdue state; and
- the exact application-owned recovery action available to an operator.

Fleet diagnostics could aggregate this status and identify an overdue world
heartbeat or economy epoch. The report must distinguish independently observed
Canic runtime facts from application-self-reported health. A generic watchdog
must not replay application work unless its durable operation contract proves
that takeover is safe.

### P2: Bounded Application Health Evidence

A managed role could expose a compact application-health envelope containing
named check identifiers, status, observation time, bounded numeric summaries
and an evidence digest. `canic medic fleet` could aggregate those envelopes
without interpreting them as infrastructure truth.

Examples include asset-versus-liability reconciliation, reserved-versus-
spendable balance, stalled handovers, insufficient oracle samples and scheduler
health. Canic must not define GOLD, tombstones, market thresholds or any other
game-specific invariant.

### P2: Soft Service Readiness and Capacity Projection

The Fleet Directory could eventually carry an authenticated, expiring
application projection such as:

- ready or maintenance state;
- application-defined capacity ceiling;
- current load;
- last heartbeat; and
- optional region or gameplay label.

This data may support a frontend that lists eligible worlds and recommends a
quieter one. It must remain soft routing evidence. A target Canister makes the
authoritative admission decision, stale data expires, and neither occupancy nor
readiness grants placement, controller or Fleet authority.

### P2: Frontend and Candid Compatibility Evidence

The existing frontend-delivery idea should consider independently deployed
browser clients and realms as a primary use case. A read-only generated manifest
could bind:

- exact deployed role Principals;
- complete and managed-subset DID hashes;
- generated frontend binding hashes;
- Fleet, release-set and network identity;
- supported client/realm compatibility constraints; and
- a digest over the complete delivery input.

CI should fail on stale generated bindings. Live diagnostics should report a
client/realm mismatch without treating frontend configuration as Fleet mutation
authority.

### P3: Motoko Parent to Rust IcyDB Service

After managed Motoko support is credible, the existing exploration's preferred
IcyDB path is applicable: provision a schema-specific Rust child service and
call it through typed Candid from the Motoko parent.

Conversation history, unread markers, party membership and public read models
are plausible motivating data shapes. This is not approval to move them. The
service must retain one source of truth for each mutation, require its exact
registered parent, use application operation receipts and keep IcyDB's
controller-only administration separate.

IcyDB remains Rust. Canic owns managed topology and the guest/root protocol;
IcyDB owns its service facade, schema-specific bindings and database semantics.

### P3: Reusable Ingress-Budget Primitives

Canic may investigate bounded per-caller or delegated-subject admission
primitives with stable accounting and metrics. They could reduce duplicated
rate-limit machinery for chat, presence, invitations and public actions.

The primitive must not pretend one policy suits every endpoint. Applications
own limits, forgiveness rules, membership exemptions and user-visible refusal
semantics. Fleet admission remains infrastructure caller authority rather than
a player-rate-limit mechanism.

## What Canic Should Not Own

Canic should not implement or standardize:

- an algorithmic peg, mint rule, reserve policy or price oracle;
- player, quest, combat, item, map, party, chat or moderation models;
- cross-world character schema or compensation policy;
- application-level selection of the quietest or most suitable world;
- token custody, escrow solvency or ledger reconciliation rules;
- a claim that multiple Canister updates are atomic;
- a generic unqualified shell builder or caller-provided managed Wasm;
- a Motoko rewrite of Canic infrastructure or IcyDB; or
- a Delulu-specific SDK, adapter, role or configuration field.

The framework owns reusable control-plane invariants. The application owns its
domain and every decision that can change a player's state or value.

## Safe Reference Adoption Sequence

If the relevant ideas are separately promoted and proven, a conservative
external adoption sequence would be:

1. Complete M0 entirely inside Canic with generic Rust and Motoko fixtures.
2. Prove a mixed Fleet with one non-custodial Motoko Component, exact lifecycle
   status, restart and same-contract upgrade.
3. Add application job-health and frontend-binding evidence without moving
   product state.
4. Pilot a non-custodial directory, presence or health projection rather than
   an economy or custody service.
5. Prove mixed-Fleet snapshot, restore and effect-free replay for the exact
   supported release boundary.
6. Trial one newly created world or service whose authoritative state has an
   independently reviewed recovery plan.
7. Consider social data or a schema-specific IcyDB service only after durable
   application receipts are proven.
8. Keep minting, ledgers, escrow and other value-critical services outside the
   initial adoption boundary.

This is a risk order, not a release-batch plan.

## Qualification Scenarios

A later promoted design should select only the scenarios belonging to its
bounded scope. The combined end-state evidence should eventually demonstrate:

1. A Motoko world installs, receives its exact Directory, activates and reports
   the same managed evidence as a Rust Component.
2. A lost lifecycle response replays without repeating an effect; a conflicting
   operation ID fails before mutation.
3. Three registered worlds expose distinct exact Principals and expiring soft
   readiness, while a stale or forged projection is rejected.
4. A critical application job becomes overdue after an interrupted run and is
   visible in Fleet diagnostics without Canic inventing successful recovery.
5. A partial application operation resumes under the same identity and cannot
   create two players, payouts or destination records.
6. A stale frontend binding or wrong-network Principal fails the delivery gate.
7. A schema-specific Rust IcyDB child accepts its exact registered Motoko parent,
   rejects another caller and returns a retained result after response loss.
8. Mixed-Fleet backup and same-release restore preserve exact role identity,
   artifact, snapshot and parent-before-child evidence.

No one implementation slice should attempt this entire matrix.

## Promotion Questions

Before any part of this idea becomes scheduled work, the maintainer must decide:

1. Is there a willing external integration owner, or is Delulu Canyon only a
   public requirements example?
2. Has the managed-guest M0 evidence justified a production language-neutral
   ABI and Mops builder?
3. Which one generic missing primitive has value beyond this game?
4. Can the slice be proven entirely inside Canic without mutating or depending
   on private downstream source?
5. Does the proposed surface preserve the distinction between application
   evidence and independently observed Fleet authority?
6. Is the external pilot non-custodial and recoverable at the supported release
   boundary?

An affirmative answer does not itself assign a minor line. Promotion remains a
separate human-owned planning decision.

## Public Sources

- [Delulu Canyon changelog](https://github.com/Smugandcomfy/delulu_canyon/blob/main/CHANGELOG.md)
- [Delulu Canyon architecture](https://github.com/Smugandcomfy/delulu_canyon/blob/main/docs/architecture.md)
- [Delulu Canyon Keeper](https://github.com/Smugandcomfy/delulu_canyon/blob/main/docs/the-keeper.md)
- [Delulu Canyon audit log](https://github.com/Smugandcomfy/delulu_canyon/blob/main/docs/audit-1.md)

These sources are mutable public descriptions. Any promoted design that relies
on an exact downstream behavior must record a commit-pinned source identity or
obtain an explicit integration contract from its owner.
