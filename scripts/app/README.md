# Local Demo Workflow (`scripts/app/`)

These scripts support the reference canisters under `crates/canisters/` and the local topology in `dfx.json`.

## Prerequisites

- `dfx` installed and on your `PATH`
- Wasm target + candid tooling:
  - `make install-canister-deps` (installs `wasm32-unknown-unknown` and `candid-extractor`)

## Start a Clean Local Replica

```bash
scripts/app/dfx_start.sh
```

This runs `dfx` (`dfx stop` then `dfx start --clean --system-canisters`).

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

The root reference canister (`crates/canisters/root`) embeds these gzipped WASMs via `include_bytes!` to simulate a “WASM bundle” used during local orchestration flows.
