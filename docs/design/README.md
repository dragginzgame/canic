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

## Scheduled Reserve-Fleet Critical Path

1. [0.103 role-owned Candid surface](0.103-role-owned-candid-surface/status.md)
   gives Root, Coordinator, Store and managed application canisters one
   bounded command/status control plane; capabilities add variants, never
   methods.
2. [0.104 `ic-timers` consumer hard cut](0.104-ic-timers-consumer-hard-cut/status.md)
   removes Canic-owned timer mechanics while preserving domain recovery.
3. [0.105 Fleet estate platform qualification](0.105-fleet-estate-platform-qualification/status.md)
   freezes local and separately authorized live-platform evidence.
4. [0.106 Coordinator-backed root funding](0.106-coordinator-backed-root-funding/status.md)
   closes replay-safe root operating funding without funding the estate
   Cycles Ledger budget implicitly.
5. [0.107 Fleet Subnet Canister estates](0.107-fleet-subnet-canister-estates/status.md)
   implements indexed reusable estates, bounded parallel work, transfer and
   the 10/100/1,000 proof.
6. [0.108 Skynet T2 Fleet observatory](0.108-skynet-fleet-observatory/status.md)
   turns the completed platform into an every-installed-Canister topology and
   Fleet-overview demonstration.

Deferred ideas do not gate this six-line path unless a later explicit
amendment moves one into a numbered design.

## Scheduled Successor

[0.109 framework-neutral local application authorization](0.109-framework-neutral-local-application-authorization/status.md)
is accepted and scheduled after 0.108. It is not another reserve-Fleet
critical-path dependency, and this scheduling cut authorizes no evidence or
runtime implementation. B1 requires accepted 0.108 closeout and explicit
maintainer promotion.

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
