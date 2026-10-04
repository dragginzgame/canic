# Canic Design Authoring

Design documents describe accepted or proposed implementation work. They do
not replace current feature guides, operational procedures, or exact contracts.

<img src="../../assets/256x256/mechanic-think.png" align="left" width="110" alt="The Canic mechanic considering where a design belongs" />

Every new minor design must follow
[delivery cadence governance](../governance/delivery-cadence.md) and include a
release-batch plan before implementation begins.

Before writing, decide whether the work is an unscheduled idea, an accepted
release-line design, or historical material. That choice determines where the
document belongs and whether it grants implementation authority.

<br clear="left" />

## Choose A Destination

| State of the work | Location | Meaning |
| --- | --- | --- |
| Interesting but unscheduled | [`ideas/`](ideas/README.md) | No release position or implementation authority |
| Accepted and scheduled | Numbered top-level directory | Maintained release lineage with a design and status tracker |
| Completed or superseded | `archive/` | Immutable historical design record |
| Temporary evidence for an active batch | `working/<line>-<topic>/` | Bounded source material owned by that batch |

## Authoring Workflow

1. State the problem, owner, security boundary, and concrete completion
   condition.
2. Identify affected runtime, CLI, configuration, protocol, persistence, and
   operator surfaces.
3. Divide the work into coherent release batches using the template below.
4. Include positive, adversarial, interruption, recovery, propagation, and
   cleanup evidence where the behavior requires them.
5. Link the design from its status tracker and keep implementation progress in
   that tracker rather than rewriting the accepted design as a diary.

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

1. [0.103 role-owned Candid surface](archive/0.103-role-owned-candid-surface/status.md)
   gives Root, Coordinator, Store and managed application canisters one
   bounded command/status control plane; capabilities add variants, never
   methods.
2. [0.104 timer ownership and synchronous lifecycle composition](archive/0.104-ic-timers-consumer-hard-cut/status.md)
   removes Canic-owned timer mechanics, documents native adoption and lets one
   application restore Canic with another synchronous runtime.
3. [0.105 framework-neutral local application authorization](archive/0.105-framework-neutral-local-application-authorization/status.md)
   establishes bounded caller/scoped local authority before presentation or
   estate work.
4. [0.106 Fleet estate platform qualification](0.106-fleet-estate-platform-qualification/status.md)
   freezes local and separately authorized live-platform evidence.
5. [0.107 fresh-Fleet preflight and runtime admission](0.107-fresh-fleet-preflight-and-runtime-admission/status.md)
   makes planning target- and Fleet-input-complete, preserves structured NNS
   catalog failures and makes the application whitelist durably evolvable.
6. [0.108 Coordinator-backed root funding](0.108-coordinator-backed-root-funding/status.md)
   closes replay-safe root operating funding without funding the estate
   Cycles Ledger budget implicitly.
7. [0.109 Fleet-wide ingress admission](0.109-fleet-wide-ingress-admission/status.md)
   replaces independent per-canister whitelists with one Coordinator-owned
   policy, complete local enforcement projections and one synchronous managed
   composed-framework caller boundary. Its release/adoption-support batch must
   close before its binding
   [post-implementation complexity audit](../audits/release-lines/0.109-post-implementation-complexity-audit.md)
   enters remediation. That audit must then be superseded by an accepted
   passing immutable verdict before any 0.110 implementation or promotion.
8. [0.110 Fleet runtime contraction](0.110-fleet-runtime-contraction/status.md)
   retains accepted B1/B2 contraction and the published `.42` checkpoint.
   [Reviewed Fleet capacity import](0.110-fleet-runtime-contraction/0.110-design.md#fi1-reviewed-fleet-capacity-import--planned-011043)
   and supplied-infrastructure bootstrap shipped in `.43`; `.44` cut historical
   receipt schemas and consolidated qualification. Published corrections through `.47` cover
   operator feedback, funding, runtime size and CI throughput; see the
   [current handoff](../status/current.md) for evidence and remaining limits.
   Further B3 records/codecs stop and remaining B4 pruning is deferred.
   B5's `.42` evidence remains a qualified checkpoint; the
   [closeout audit](../audits/release-lines/0.110-closeout-audit.md) must cover
   the final FI1 scope and receive human acceptance before the next minor.
9. [Standalone blob service extraction](0.111-standalone-blob-service-extraction/status.md)
   was explicitly selected for implementation after 0.110.52. The independent
   service owns blob semantics; Canic owns an isolated application adapter and
   removes its embedded implementation. This reprioritization does not declare
   FR1 complete, accept minor closeout, or assign the extraction's release.


[Bounded multi-Fleet estates](ideas/bounded-multi-fleet-estates/design.md)
is deferred and unnumbered. Its unproved Q0 capsule and indexed-estate work
are not prerequisites for blob extraction. Neither roadmap promotion nor
finishing `.43` substitutes for requested and accepted human minor closeout.

Future Fleet setup and managed-service slices follow the accepted
[Coordinator setup prerequisite](0.110-fleet-runtime-contraction/0.110-design.md#infrastructure-prerequisites--accepted-2026-09-25):
the operator establishes Coordinator authority, initializes/registers Root and
Store, completes Root activation, then imports capacity on that Root's subnet.
Coordinator placement is independent of its Roots; pool import does not bootstrap
infrastructure implicitly. Explicit infrastructure bootstrap belongs to the same
`.43` batch, with Coordinator setup confirmed before dependent Root/Store effects.

The former stateful-retirement/release-adoption proposal is
[cancelled and archived](archive/0.111-rescinded-stateful-fleet-release-adoption/status.md).
It grants no compatibility exception or implementation authority.

The former runtime-heavy generic Fleet Observatory is now an
[promoted 0.110 host-first batch](0.110-fleet-runtime-contraction/0.110-design.md#op3-host-first-fleet-observatory-canic-002).

Deferred ideas do not gate this nine-line path unless a later explicit
amendment moves one into a numbered design.

## Release-Batch Plan Template

The whole minor line has no minimum release count. Planned design cadence
should normally publish no more than 12 releases, shared across every design
document assigned to that minor. Necessary post-publication correctness,
security, recovery and operator-regression follow-ups may exceed that
guideline. Implementation slices may be smaller, but they must map into
coherent batches; they are not automatically patch releases.

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

## Continue From Here

- [Read delivery cadence governance](../governance/delivery-cadence.md)
- [Review deferred design ideas](ideas/README.md)
- [Check the current implementation handoff](../status/current.md)
- [Find architecture documents](../architecture/README.md)
- [Find audit methods and evidence](../audits/README.md)
- [Browse all documentation](../README.md)
