# Persistent counter

Root deploys a counter from a separate versioned Wasm Store. The counter owns
its stored value and its fence. The checked-in mainnet declaration has no
bound identities. The interactive session creates a separate local fleet.

The [canister](src/lib.rs) is a Wasm library. The IC calls its [install, upgrade and update entrypoints](src/endpoints.rs) directly. The [command codec](client/main.rs) runs on
your computer when you use `canic fleet call` to read or increment the counter.

From this directory with the Canic CLI installed:

```sh
canic app canister list
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
