# Canic Design Authoring

Every new minor design must follow
[delivery cadence governance](../governance/delivery-cadence.md) and include a
release-batch plan before implementation begins.

## Directory Meaning

- A top-level numbered directory is part of the maintained release lineage:
  the current/recent baseline or an accepted, scheduled future line. It
  normally contains only its versioned design and `status.md` tracker; an
  unscheduled future concept never belongs here.
- [`ideas/`](ideas/README.md) contains unnumbered, deferred concepts. An idea
  has no release position or implementation authority even when it preserves
  an older reviewed draft.
- `archive/` contains completed or superseded historical designs whose
  released identities remain immutable.

Promotion from `ideas/` is a planning decision, not a rename performed during
an implementation slice. It requires an owner, concrete need, scheduled line,
release-batch plan and explicit maintainer acceptance.

## Scheduled Application-Safety And Estate Path

1. [0.103 role-owned Candid surface](0.103-role-owned-candid-surface/status.md)
   gives Root, Coordinator, Store and managed application canisters one
   bounded command/status control plane; capabilities add variants, never
   methods.
2. [0.104 timer ownership and synchronous lifecycle composition](0.104-ic-timers-consumer-hard-cut/status.md)
   removes Canic-owned timer mechanics, documents native adoption and lets one
   application restore Canic with another synchronous runtime.
3. [0.105 framework-neutral local application authorization](0.105-framework-neutral-local-application-authorization/status.md)
   establishes bounded caller/scoped local authority before presentation or
   estate work.
4. [0.106 Fleet estate platform qualification](0.106-fleet-estate-platform-qualification/status.md)
   freezes local and separately authorized live-platform evidence.
5. [0.107 Coordinator-backed root funding](0.107-coordinator-backed-root-funding/status.md)
   closes replay-safe root operating funding without funding the estate
   Cycles Ledger budget implicitly.
6. [0.108 Fleet Subnet Canister estates](0.108-fleet-subnet-canister-estates/status.md)
   adds application retirement evidence before recycling, indexed reusable
   estates, bounded parallel work, transfer and the 10/100/1,000 proof.
7. [0.109 stateful Fleet release adoption](0.109-stateful-fleet-release-adoption/status.md)
   qualifies one whole-Fleet stop-the-world predecessor-to-successor
   transition before stateful production claims.
8. [0.110 generic Fleet observatory](0.110-fleet-observatory/status.md)
   publishes supported downstream views/rendering with external Prequel Wars
   as the flagship consumer.

Deferred ideas do not gate this eight-line path unless a later explicit
amendment moves one into a numbered design.

## Release-Batch Plan Template

The whole minor line has no minimum release count and should normally publish
no more than 12 releases, shared across every design document assigned to that
minor. Implementation slices may be smaller, but they must map into coherent
batches; they are not automatically patch releases.

Copy and complete this table in the design or its status tracker:

| Batch | Bounded outcome and owner | Included direct evidence and fallout | Focused validation | Surface impact | Status |
| --- | --- | --- | --- | --- | --- |
| B1 |  |  |  |  | Pending |
| B2 |  |  |  |  | Pending |
| B3 |  |  |  |  | Pending |

Add or remove rows to match the real dependency boundaries. If the line is
expected to exceed 12 published releases, explain why immediately below the
table. Use stable batch labels during design; the maintainer assigns version
numbers when a release is actually prepared.

Each batch should include its direct implementation, positive and adversarial
tests, interruption/retry evidence where applicable, documentation, generated
or fixture propagation and required cleanup. Do not create separate batches
for ordinary compile fallout or changelog maintenance.
