# Sharding And Placement

In Canic, we treat sharding as a **placement problem**: given an application key, which canister should own that partition of state?

The application chooses the partition key — for example, a user ID, tenant ID, or game world ID. Canic manages a named pool of shard canisters and records the mapping from each key to its assigned shard.

Placement works like this:

1. If the key already has an assignment to an eligible shard, reuse it.
2. For a new key, select among eligible shards with spare capacity using rendezvous hashing.
3. If none has room, request another shard through Canic's Root-controlled provisioning flow, subject to configured limits and admission checks.
4. If growth is blocked, return an error rather than silently exceeding those limits.

The assignment is persisted, so adding a shard does not automatically redistribute existing keys. New capacity serves new assignments. For example, if Alice's user ID is assigned to shard A, creating shard B does not move Alice's data or change her assignment.

Capacity here means **the number of assigned partition keys per shard**. It is not a measurement of memory usage or request load. Choosing useful keys and sensible limits matters: one very busy tenant can still create a hot shard.

There are also two distinct placement decisions. Fleet placement determines which subnet Root owns a deployed Component; the Component's sharding pool determines which child shard owns an application key. The sharding selector itself does not choose subnets.

Canic provides the assignment registry, bounded shard provisioning, and lookup machinery. The application still owns its data model, calls to the selected shard, and any coordination across shards. Automatic data splitting, data movement, and load rebalancing are outside this placement mechanism.

## Continue From Here

- [Use scaling and placement](../features/scaling-and-placement/README.md)
- [Plan and operate a Fleet](../operations/README.md)
- [Browse all documentation](../README.md)
- [Back to the main README](../../README.md)
