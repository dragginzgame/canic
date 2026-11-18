# Getting started

Edit the greeting in `starter-app/src/endpoints/mod.rs`. From this directory, build the
application and inspect its Wasm:

```sh
canic app build --canister app
canic artifact inspect artifacts/app.wasm
```

Keep callable methods under `starter-app/src/endpoints/` and local entities under
`starter-app/src/schema/`. Put handwritten install/upgrade hooks in `src/lifecycle.rs`
within the crate, or a declarative database role in `src/setup.rs`. Each crate can
override `endpoints`, `schema` or `lifecycle` in `[package.metadata.canic.layout]`.

This stateless app reports `Ready` immediately. If you add persistent state,
update the readiness handler to report when the app can serve requests.

Commit the generated `Cargo.lock` after the first build. The workspace uses
the matching local Canic checkout selected at initialization.

When you're ready to deploy, follow the [managed fleet guide](https://github.com/dragginzgame/canic/blob/main/docs/demos/root.md).
It covers fleet creation, funding, approvals and upgrades. Keep the fleet
config and its journal together once you start planning deployments.
