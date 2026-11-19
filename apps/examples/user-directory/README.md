# User directory

The shared `schema` crate builds both databases. Its model binds `Route` to the
hub's `Routes` store and `User` to the shard's `Accounts` store. Each actor passes
its store type to `mount_db_role!`. Keep the declared store key when upgrading a
populated role.

A user signs in to the hub and calls `register`. The hub asks Root to place
the account, records its route, and asks that shard to insert the private
user ID. A retry returns to the same reservation. The account key comes from
the verified session’s issuer and subject. The application chooses the user ID.

| Canister | What it owns |
| --- | --- |
| [User hub](hub/src/lib.rs) | The account directory. `my_shard` returns the caller’s confirmed route. |
| [User shard](shard/src/lib.rs) | Private account rows. Only its installed parent can provision a row. `my_id` requires a session on this shard. |
| Root | Placement, creation and separate Auth grants for the hub and each shard. |
| Wasm Store | The published shard executable used when the pool grows. |

The [local fleet](canic.toml) admits one account per shard and two shards.
The third distinct account receives `PoolFull`. The hub is independently
controlled. Root controls the shards, while the hub is their application parent.

From this directory:

```sh
canic app canister list
canic app build
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
