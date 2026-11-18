# Fleet app template

Edit [src/lib.rs](src/lib.rs). It is also the actor that `canic app init --source` copies into a new project. The stateless greeting is public. The app reports ready immediately because it has no state to open.

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

As the app grows, keep endpoint declarations in the actor and move domain operations into ordinary Rust functions. Select a pre-decode guard for protected endpoints and check resource permissions in the handler. If the app gains persistent state, create it on install, reopen it on upgrade and report its actual readiness. The [counter](../demo/src/actor.rs) demonstrates that next step.
