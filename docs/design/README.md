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
   creates absolute Wasm code-section and replica-validator function reserves
   through a zero-capability storage, codec, whole generated-surface,
   generic-instantiation, endpoint and recovery hard cut.
9. [0.111 bounded multi-Fleet estates](0.111-bounded-multi-fleet-estates/status.md)
   adds indexed Root-local estates, an ordinary reserve Fleet and one
   cycle-safe source-disposition/destination-credit operation without data,
   stable-memory or Principal preservation. B1 is held behind Q0 proof of the
   finalized source-executed cycle-disposition capsule.

The former stateful-retirement/release-adoption proposal is
[cancelled and archived](archive/0.111-rescinded-stateful-fleet-release-adoption/status.md).
It grants no compatibility exception or implementation authority.

The former runtime-heavy generic Fleet Observatory is now an
[unnumbered host-first idea](ideas/fleet-observatory/status.md).

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
