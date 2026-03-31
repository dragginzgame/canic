# Local Demo Workflow (`scripts/app/`)

These scripts support the reference canisters under `canisters/` and the local topology in `dfx.json`.

## Prerequisites

- `dfx` installed and on your `PATH`
- Wasm target + candid tooling:
  - `make install-canister-deps` (installs `wasm32-unknown-unknown` and `candid-extractor`)

## Start a Clean Local Replica

```bash
scripts/app/dfx_start.sh
```

This runs `dfx` (`dfx stop` then `dfx start --clean --system-canisters`).

Keep that replica running in a separate terminal. The install/build commands
below assume `dfx` is already running and fail if it is not.

## Install the Reference Topology

From the repo root:

```bash
make demo-install
```

This one command:
- creates the reference canisters in `dfx`
- builds the release artifacts
- reinstalls `root` in `Prime` mode
- stages the config-defined release set through the generic Canic root bootstrap helper for later publication into the live `wasm_store`
- waits for `root` to report `READY`

## Build Canisters

From the repo root:

```bash
dfx canister create --all
dfx build --all
```

`dfx.json` uses custom build commands which call `scripts/app/build.sh <canister>`. That script:
- builds the Rust canister crate (`-p canister_<name>`) for `wasm32-unknown-unknown`
- copies the resulting WASM into `.dfx/local/canisters/<name>/<name>.wasm`
- runs `candid-extractor` to produce `.dfx/local/canisters/<name>/<name>.did`

## Why `.wasm.gz` Exists

`dfx.json` sets `"gzip": true`, so dfx 0.30.2 also writes a gzipped artifact:
`.dfx/local/canisters/<name>/<name>.wasm.gz`.

The local bootstrap flow stages these gzipped artifacts through `root` into
root-local stable memory and then publishes ordinary roles into the live
`wasm_store`. Only the bootstrap `wasm_store` module itself is embedded into
`root.wasm`; ordinary roles are not.

## Generic Root Bootstrap Helper

Downstream Canic projects can reuse the same host-side release staging flow with:

```bash
CANIC_CONFIG_PATH=/path/to/canic.toml \
CANIC_STAGE_WASM_DIR=/path/to/.dfx/local/canisters \
bash /path/to/canic/scripts/canic/bootstrap_root_release_set.sh root
```

Useful environment variables:

- `CANIC_CONFIG_PATH`: path to the downstream project `canic.toml`
- `CANIC_STAGE_WASM_DIR`: path to the built `.wasm.gz` artifacts; defaults to `.dfx/$DFX_NETWORK/canisters`
- `CANIC_TEMPLATE_STAGE_VERSION`: override the staged template version if the project version is not discoverable from `Cargo.toml`; the helper stages each ordinary role as `embedded:<role>@<version>`
- `CANIC_PROJECT_ROOT`: override project-root discovery if the helper cannot infer it from `canic.toml` or `dfx.json`
