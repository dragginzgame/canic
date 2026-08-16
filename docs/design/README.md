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

1. [0.103 `ic-timers` consumer hard cut](0.103-ic-timers-consumer-hard-cut/status.md)
   removes Canic-owned timer mechanics while preserving domain recovery.
2. [0.104 Fleet estate platform qualification](0.104-fleet-estate-platform-qualification/status.md)
   freezes local and separately authorized live-platform evidence.
3. [0.105 Coordinator-backed root funding](0.105-coordinator-backed-root-funding/status.md)
   closes replay-safe root operating funding without funding the estate
   Cycles Ledger budget implicitly.
4. [0.106 Fleet Subnet Canister estates](0.106-fleet-subnet-canister-estates/status.md)
   implements indexed reusable estates, bounded parallel work, transfer and
   the 10/100/1,000 proof.
5. [0.107 Skynet T2 Fleet observatory](0.107-skynet-fleet-observatory/status.md)
   turns the completed platform into an every-installed-Canister topology and
   Fleet-overview demonstration.

Deferred ideas do not gate this five-line path unless a later explicit
amendment moves one into a numbered design.

## Release-Batch Plan Template

The whole minor line normally contains roughly 6–10 substantive release
batches, shared across every design document assigned to that minor.
Implementation slices may be smaller, but they must map into these batches;
they are not automatically patch releases.

Copy and complete this table in the design or its status tracker:

| Batch | Bounded outcome and owner | Included direct evidence and fallout | Focused validation | Surface impact | Status |
| --- | --- | --- | --- | --- | --- |
| B1 |  |  |  |  | Pending |
| B2 |  |  |  |  | Pending |
| B3 |  |  |  |  | Pending |
| B4 |  |  |  |  | Pending |
| B5 |  |  |  |  | Pending |
| B6 |  |  |  |  | Pending |

Add or remove rows to match the real dependency boundaries. If the line is
planned outside the normal range, explain why immediately below the table.
Use stable batch labels during design; the maintainer assigns version numbers
when a release is actually prepared.

Each batch should include its direct implementation, positive and adversarial
tests, interruption/retry evidence where applicable, documentation, generated
or fixture propagation and required cleanup. Do not create separate batches
for ordinary compile fallout or changelog maintenance.
