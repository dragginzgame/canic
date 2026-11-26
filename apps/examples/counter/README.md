# Persistent counter

Root deploys a counter from a separate versioned Wasm Store. The counter owns
its stored value and its fence. The checked-in mainnet declaration has no
bound identities. The `local` fleet is disposable. The
interactive counter session creates its own local fleet.

The [canister](src/lib.rs) is a Wasm library. The IC calls its [install, upgrade and update entrypoints](src/endpoints.rs) directly. The [command codec](client/main.rs) runs on
your computer when you use `canic fleet call` to read or increment the counter.

From this directory with the Canic CLI installed:

```sh
canic app canister list
canic fleet canister list --fleet local
canic app build
canic artifact inspect artifacts/app.wasm
```

From the repository root, run `make demo` with the simulator and tools prepared.
After deployment, enter these commands at the `counter>` prompt in the same terminal.

```text
increment
read
exit
```

Both calls return the same stored value. A compatible upgrade preserves it.
[Local session and upgrade instructions](../../../docs/demos/demo.md)

## Snapshot readers

By default, only Root can read the managed counter’s native snapshots. To grant
additional readers access to all of its retained snapshots, add this table to
`canic.toml` and put their principal IDs in the list.

```toml
[apps.counter.fleets.local.snapshots]
roles = ["app"]
download_principals = []
```

With the local network and installed canister IDs bound, preview the target and
reader list. Then apply the reviewed list as a Root controller. Applying an empty
list revokes the additional readers.

```sh
approval=$(canic --json fleet canister plan-snapshot-readers counter --fleet local)
echo "$approval"
canic fleet call --fleet local --canister root --identity operator configure-snapshot-readers "$approval"
```
