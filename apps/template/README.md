# Fleet app template

Edit [src/endpoints/mod.rs](src/endpoints/mod.rs). It is also the actor that `canic app init --source` copies into a new project. The stateless greeting is public. The app reports ready immediately because it has no state to open.

From this directory, build just the application with the installed CLI and Rust Wasm target:

```sh
canic app build --canister app
canic artifact inspect artifacts/app.wasm
```

[canic.toml](canic.toml) declares the application and its supplied Root and Wasm Store. Fleet creation, network selection and funding happen later. To start an independent project, run this from a matching Canic checkout:

```sh
canic app init hello --source hello
cd hello
canic app build --canister app
```

As the app grows, keep callable declarations under `src/endpoints/` and move domain operations into ordinary Rust functions. Select a pre-decode guard for protected endpoints and check resource permissions in the handler. If the app gains persistent state, create it on install, reopen it on upgrade and report its actual readiness. The [counter](../demo/src/actor.rs) demonstrates that next step.

Generated crates opt into source layout checks with `[package.metadata.canic.layout]`
in their Cargo manifest. Canic checks selected packages during `canic app build`
and `canic artifact build`. Put callable mounts under `src/endpoints/`, IC DB
entities under `src/schema/`, and install/upgrade hooks in `src/lifecycle.rs`.
Create those files when the app needs them. To change a location for one crate,
set `endpoints`, `schema` or `lifecycle` to its source path in that table.
Existing crates can adopt the check by adding the table.
