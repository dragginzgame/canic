# User directory

The shared `schema` crate builds both databases. Its model binds `Route` to the
hub's `Routes` store and `User` to the shard's `Accounts` store. Each actor passes
its canister schema type to `mount_db!`. Keep the declared store key when upgrading a
populated role. IC DB routes each typed query to its declared store.
The separate receiver lifecycle retains the installed topology. For a shard,
`parent_storage_key` names the stable cell holding the parent principal selected
by the fleet’s approved install.

A user signs in to the hub and calls `register`. The hub selects a shard locally, asks Root for a new child when all its shards are full,
records the route, and asks the shard to insert the private
user ID. A retry returns to the same reservation. The account key comes from
the verified session’s issuer and subject. The application chooses the user ID.

| Canister | What it owns |
| --- | --- |
| [User hub](hub/src/lib.rs) | The account directory. `my_shard` returns the caller’s confirmed route. |
| [User shard](shard/src/lib.rs) | Private account rows. Only its installed parent can provision a row. `my_id` requires a session on this shard. |
| Root | Child creation, creation budgets and separate Auth grants for the hub and each shard. |
| Wasm Store | The published shard executable used when the pool grows. |

The [local fleet](canic.toml) admits one account per shard and two shards.
The third distinct account receives `PoolFull`. The hub is independently
controlled. Root controls the shards, while the hub is their application parent.

From this directory:

```sh
canic app canister list
canic app build
```

For a fresh fleet, use the hub client and the operator identity approved at installation.
Configure the number of keys per shard before the first registration. Inspect the shard
before draining it. Repeat drain after the application moves its last account so Root
receives the exact generation release.

```sh
canic fleet call --canister hub --identity operator configure-shard-routes shard 1
canic fleet call --canister hub --identity operator observe-shard "$SHARD"
canic fleet call --canister hub --identity operator drain-shard "$SHARD" "$SLOT" "$GENERATION"
```

From the repository root, run the installed example with synthetic cycles:

```sh
make qualification-setup
bash scripts/dev/test-store.sh user-directory
```

The exercise binds a disposable simulator, publishes the shard to Store, and
supplies Root with a time-limited pool approval. It signs local credentials
with a fixture key. After hub registration it separately enrolls each new
receiver and establishes a shard session before reading the private ID.
The checked-in 16-byte installation nonce and issuer are local fixture inputs.
