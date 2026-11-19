# Greeting

One stateless application canister exposes a greeting, readiness and metrics.
Its local fleet declaration starts unbound. From this directory with the Canic
CLI installed:

```sh
canic app canister list
canic app build
canic artifact inspect artifacts/greeting.wasm
```

From the repository root, run `make journey-test` with the simulator prepared.
The installed CLI journey calls the greeting, upgrades the same canister and
checks its new reply. [Application endpoints](canic-hello/src/actor.rs)
