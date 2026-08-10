# Scaling And Placement

Canic models reusable topology separately from concrete deployment. A
`ComponentSpec` describes one top-level role and its allowed descendant tree;
each deployed Component receives its own identity, root binding, state, and
effective limits.

## What It Provides

- reusable Component Specs and configuration-only Component Groups
- explicit Authority, Replica, PoolMember, and Ordinary deployment purposes
- bounded initial placement and same-release monotonic scale-out
- per-root density, aggregate placement, instance, descendant, and byte limits
- dynamic root-owned child trees with exact parent bindings
- sharding pools for stateful partitions and scaling pools for instances
- reduction-only limits for each concrete deployment member

Groups may include other groups, but compilation flattens them before planning.
There is no Group Canister, group controller, or group-local Wasm Store.

## Boundary

The Coordinator owns composition planning, placement orchestration, and
Fleet-wide service publication. Each Fleet Subnet Root owns concrete identity
allocation and lifecycle effects. Application Components may request admitted
children, but they do not acquire management-canister or root authority.

The active 0.101 line is still delivering this architecture. Check the current
status before treating every designed scaling surface as complete.

## Start Here

- [Component configuration](../../../CONFIG.md#component-specs)
- [Composable Component deployment design](../../design/0.101-fleet-authoritative-service-provisioning-and-publication/0.101-design.md)
- [0.101 implementation status](../../design/0.101-fleet-authoritative-service-provisioning-and-publication/status.md)
- [Fleet installation limits](../../architecture/fleet-install-input.md)
- [Academic Fleet walkthrough](../../getting-started/local-academic-fleet.md)
