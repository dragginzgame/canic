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

As the app grows, keep callable declarations under `src/endpoints/` and move domain operations into ordinary Rust functions. Select a pre-decode guard for protected endpoints and check resource permissions in the handler. If the app gains persistent state, create it on install, reopen it on upgrade and report its actual readiness. The [counter](../examples/counter/canic-demo/src/actor.rs) demonstrates that next step.

Generated crates opt into source layout checks with `[package.metadata.canic.layout]`
in their Cargo manifest. Canic checks selected packages during `canic app build`
and `canic artifact build`. Put callable mounts under `src/endpoints/`, IC DB
entities under `src/schema/`, and install/upgrade hooks in `src/lifecycle.rs`.
A stateful role using `canic::mount_db_role!` declares its generated lifecycle in
`src/setup.rs`. The `lifecycle` override also applies to this declaration. To change a location for one crate,
set `endpoints`, `schema` or `lifecycle` to its source path in that table.
Existing crates can adopt the check by adding the table.

### Adding an application endpoint

New apps already enable Canic's `method` feature. Put the function in `src/endpoints/`.
The function name is the public export and the Wire method ID defaults to the
hash of that name. The default frame budget is 512 bytes in each direction.

```rust
#[canic::query]
fn version() -> u32 {
    1
}
```

Use `#[canic::update]` for an update. For a protected method, enable
`auth-enrollment`, select the required scope in the attribute and take the
verified session first. [Scoped endpoints](../../docs/demos/auth.md#opt-in-volatile-receiver)

To migrate from the removed `mount_wire_method!` macro, keep the handler as an
ordinary typed function and put the query or update attribute above it. Keep any published `name`, `method`,
`limits`, `reply_limits`, `guard` and `scope` in the attribute. With a named
`contract`, the contract supplies both directional limits and the method ID.
Use an ordinary `async fn` for an asynchronous update. Bare handlers admit
public callers. To require a verified session, select its scope and take the
session as the first parameter.
