# Canic

Canic builds Internet Computer applications and manages their fleets. An app declares its canisters, build recipes and parent relationships in `canic.toml`. Each fleet binds that app to a network and keeps its own deployment journal. For a managed fleet, Root controls installations and the versioned Wasm Store holds the approved releases.

## Install and try the local fleet

From this checkout, with Rustup and the repository's pinned Rust toolchain installed:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.100.0
make install-tools
make qualification-setup
make canic
canic --help
make demo
```

The demo starts a local Root, Store and persistent counter with synthetic cycles. At the `counter>` prompt, enter `increment`, then `read`. Enter `exit` to stop. Other examples use the same terminal. [Example fleets and commands](apps/examples/README.md) · [Counter walkthrough](docs/demos/demo.md)

## Initialize an app and fleet

From the Canic checkout, generate a new application in an unused directory and build it:

```sh
canic app init hello --source hello
cd hello
canic app build
```

An app can build with zero fleets. For an existing Rust workspace, use `canic app init NAME` and `canic app canister add` to declare its existing packages. [Application template](apps/template/README.md)

When you're ready to plan a mainnet fleet, run these commands from the generated directory with your ICP CLI identity. Replace `my-operator` with its name.

```sh
canic fleet init mainnet --network ic
canic fleet plan --identity my-operator
```

Review the network, controller, artifacts, cycle ceilings and deadline. The plan prints the exact apply command. Running it authorizes spending from the operator's funded cycles-ledger account. If an attempt is interrupted, inspect and resume the same operation. Preserve the fleet's journal and signed attempts. [Deployment, funding and recovery](docs/demos/root.md#deploy-the-generated-application) · [Existing standalone journals](docs/demos/standalone-journal.md)

## Inspect the fleet

From the project directory:

```sh
canic fleet show
canic fleet show --watch
```

The terminal shows the declared canister tree, observed controllers and retained operation evidence. The watcher prints a localhost URL. Use `--app` and `--fleet` when selection is ambiguous, or `--config` for a specific declaration. The repository's [hello-world config](canic.toml) and [historical profile config](qualification/fixtures/profile-mainnet.toml) remain available for observation.

## Command contexts

Use `canic --help` to list command groups and an action's `--help` for its arguments. For example, `canic fleet --help` lists fleet operations and `canic fleet auth --help` lists enrollment approvals. [Configuration and selection rules](docs/design/canic-system-design.md#5-configuration-and-operator-experience) · [Wire contract checks](docs/wire-ci.md)

## Check the mainnet wallet

```sh
canic wallet o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe
```

This reads public ICP and cycles-ledger balances for the Canic operator's default account. Add `--address-only` to derive its ICP receive address offline. Operators fund deployment cycles separately.

## Development

`make help` lists repository commands. Run the focused checks for the component you change. `make ci` runs the full local gate. `make publication-admission` runs release checks in a clean delivery checkout with a prepared simulator. Eligible product packages are listed in `CARGO_PUBLISH_PACKAGES` in the Makefile. Example and qualification packages stay private.

- [Qualification guide](qualification/README.md) for test commands and the guarantees each suite exercises.
- [System design](docs/design/canic-system-design.md) for ownership, scope and recovery contracts.
- [Repository instructions](AGENTS.md) and [Toko Tooling](TOKO-TOOLING.md) for contributor and release rules.
- [Licence](LICENSE.md), [changelog](CHANGELOG.md) and [legacy Canic](https://github.com/dragginzgame/canic-legacy).
