# Scaling And Placement

<p align="center">
  <img src="../../../assets/1400x600/canic-scaling.jpg" alt="The Canic mechanic watering a growing tree of connected canisters" width="700" />
</p>

**Scaling** means adding canisters when an application needs more capacity.
**Placement** decides which part of the IC network may run them. Canic lets an
operator define both ahead of time, including hard limits on how far an
application may grow.

A **Component Spec** is a reusable blueprint for one kind of application
canister and any child canisters it may create. Each deployed **Component** has
its own identity, data, location, and limits.

## What It Provides

- reusable Component Specs and Groups of related Specs
- explicit Authority, Replica, PoolMember, and Ordinary deployment purposes
- bounded initial placement and same-release monotonic scale-out
- per-root density, aggregate placement, instance, descendant, and byte limits
- dynamic root-owned child trees with exact parent bindings
- sharding pools that divide data and scaling pools that add equivalent workers
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

- [Component Specs](../../../CONFIG.md#component-specs)
- [Component Groups](../../../CONFIG.md#component-groups)
- [Component Group deployments](../../../CONFIG.md#component-group-deployments)
- [Fleet services](../../../CONFIG.md#fleet-services)
- [Composable Component deployment design](../../design/archive/0.101-fleet-authoritative-service-provisioning-and-publication/0.101-design.md)
- [Current implementation status](../../design/0.110-fleet-runtime-contraction/status.md)
- [Fleet ensure bounds](../operations/fleet-ensure.md)
- [Academic Fleet walkthrough](../../getting-started/local-academic-fleet.md)
