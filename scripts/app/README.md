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
- lets `root` bootstrap the internal `wasm_store` and publish the configured release set automatically
- waits for `root` to report `READY`

## Build Canisters

From the repo root:

```bash
dfx canister create --all
dfx build --all
```

`dfx.json` uses custom build commands which call `scripts/app/build.sh <canister>`. That script:
- builds the Rust canister crate for `wasm32-unknown-unknown`
- resolves `wasm_store` from the canonical `canic-wasm-store` package instead of a local `canisters/wasm_store` crate
- discovers the matching `canic-wasm-store` source automatically from the current `canic` checkout or published registry source, so downstreams do not need their own `wasm_store` crate
- copies the resulting WASM into `.dfx/local/canisters/<name>/<name>.wasm`
- runs `candid-extractor` to produce `.dfx/local/canisters/<name>/<name>.did`

## Why `.wasm.gz` Exists

`dfx.json` sets `"gzip": true`, so dfx 0.30.2 also writes a gzipped artifact:
`.dfx/local/canisters/<name>/<name>.wasm.gz`.

The normal local bootstrap path now embeds the ordinary release bundle into
`root.wasm` from the already-built child canister `.wasm.gz` artifacts under
`.dfx/$DFX_NETWORK/canisters`. After `dfx build --all`, reinstalling `root`
is enough; no separate release staging or bootstrap resume step is required.

There is no separate release-staging helper in the normal install path anymore.
