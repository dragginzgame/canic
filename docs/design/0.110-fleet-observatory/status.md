# Canic 0.110 Implementation Status

Date: 2026-08-18

## Status

- State: accepted and scheduled after the 0.109 stateful-adoption gate.
- Outcome: a generic, supported downstream observatory surface serving exact
  local identity and bounded Fleet overview from every installed Canister.
- Flagship consumer: external Prequel Wars, with its own data-only Galactic
  War Room profile and application views.
- Current baseline: no maintained generic package exists. The checked-in
  Skynet App/helper are removed by this planning cut; historical release/audit
  records remain truthful.
- Runtime impact: none from this planning cut.
- Implementation approval: none. B1 requires accepted 0.109 closeout and
  explicit maintainer promotion.
- Dependency posture: canonical Canic packages and infrastructure never depend
  on Prequel Wars, IcyDB or another downstream application crate.
- Frontend posture: browser environment/bindings and externally managed static
  assets remain a separate unnumbered delivery-handoff idea.

Design: [Generic Fleet observatory](0.110-design.md)

## Release-Batch Tracker

| Batch | Outcome | Included evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Supported generic contract and baseline | Package/API ownership, every-role fields, evidence/freshness, bounds, public allowlist and profile versioning | Source inventory, model/policy tests and explicit acceptance | Blocked on 0.109 and promotion |
| B2 | Generic view/renderer/HTTP package | Passive views/profile, escaping, routing, headers, parity and first excess | Native tests and package Clippy | Blocked on B1 |
| B3 | Canonical role adapters | Coordinator/root/Store/managed local identity and safe views | Role builds, Candid/HTTP and authority rejection | Blocked on B2 |
| B4 | Revisioned Fleet projection | Root summaries, aggregation, protected propagation, freshness and restart recovery | Authority/revision/replay PocketIC tests | Blocked on B3 |
| B5 | Operational views | Funding, estate, retirement and adoption state with exact counts and bounded detail | Boundary, truncation and scale-shaped fixtures | Blocked on B4 |
| B6 | Downstream profile support | Validated data-only profile, external-consumer fixture and accessibility/security contract | Downstream-style build, semantic and hostile-input tests | Blocked on B5 |
| B7 | Every-installed-role closeout | Multi-Subnet navigation, restart convergence, generated surfaces, docs and Skynet residue removal | Representative PocketIC journey and targeted repository gates | Blocked on B6 |

## Next Authorized Action

No 0.110 work is authorized. After 0.109 closes, request B1 promotion and
freeze the generic package boundary before any runtime, Candid or projection
mutation.
