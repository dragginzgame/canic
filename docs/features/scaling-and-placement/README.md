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

## Allocation and recovery

Routing follows the allocation that owns a canister ID. Directory synchronization
delivers that identity to the parent. Removing a child makes its index bindings
and shard assignments unavailable; reusing the same physical ID does not restore
those routes. Scaling counts likewise include only workers from current allocations.

An existing shard key remains owned until the application explicitly releases it
with `ShardingApi::release_partition_key`. Releasing an old allocation's key leaves
the replacement shard's count unchanged. Index recovery can release an unavailable
binding before the application explicitly binds or creates its replacement.
Ordinary resolution never silently relocates an existing assignment.

Late successful creation replies retain their completed allocation accounting even
if the child was removed before local registration. They cannot overwrite a newer
claim or recycle a replacement. Root also checks the expected allocation on every
new recycle request. An exact completed removal can replay after a later allocation
reuses that ID; it returns the retained completion without affecting the replacement.

## Start Here

- [Component configuration](../../../CONFIG.md#component-specs)
- [Composable Component deployment design](../../design/archive/0.101-fleet-authoritative-service-provisioning-and-publication/0.101-design.md)
- [Current implementation status](../../design/0.110-fleet-runtime-contraction/status.md)
- [Fleet ensure bounds](../operations/fleet-ensure.md)
- [Academic Fleet walkthrough](../../getting-started/local-academic-fleet.md)
