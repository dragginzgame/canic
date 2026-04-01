# Local Demo Workflow (`scripts/app/`)

These scripts support the reference canisters under `canisters/` and the local topology in `dfx.json`.

## Prerequisites

- `dfx` installed and on your `PATH`
- Wasm target + candid tooling:
  - `make install-canister-deps` (installs `wasm32-unknown-unknown` and `candid-extractor`)

## Local Replica Contract

The install/build commands below assume the target `dfx` replica is already
running. They fail fast if it is not.

If you want a manual convenience helper for local work, use:

```bash
scripts/app/dfx_start.sh
```

That helper is optional and repo-local only. The install/test flows still do
not auto-start `dfx`.

## Install the Reference Topology

From the repo root:

```bash
make demo-install
```

This one command:
- creates the reference canisters in `dfx`
- builds the release artifacts
- emits a build-produced root release-set manifest from the configured ordinary `.wasm.gz` artifacts
- reinstalls `root` in `Prime` mode
- stages the configured ordinary release set into `root` through the published Rust helper in `canic-installer`
- resumes bootstrap so `root` can create the internal `wasm_store` and publish the staged release set
- waits for `root` to report `READY`

This is a manual local smoke flow, not part of `make test`.

## Build Canisters

From the repo root:

```bash
dfx canister create --all
dfx build --all
```

`dfx.json` uses custom build commands which call `scripts/app/build.sh <canister>`. That script:
- builds the Rust canister crate for `wasm32-unknown-unknown`
- keeps `wasm_store` out of downstream `dfx.json` and resolves it internally from the canonical `canic-wasm-store` package instead of a local `canisters/wasm_store` crate
- discovers the matching `canic-wasm-store` source automatically from the current `canic` checkout or published registry source, and if that canonical crate is not present it synthesizes a hidden wrapper directly from the resolved `canic` source, so downstreams do not need their own `wasm_store` crate or extra `wasm_store` build config
- copies the resulting WASM into `.dfx/local/canisters/<name>/<name>.wasm`
- runs `candid-extractor` to produce `.dfx/local/canisters/<name>/<name>.did`

## Why `.wasm.gz` Exists

`dfx.json` sets `"gzip": true`, so dfx 0.30.2 also writes a gzipped artifact:
`.dfx/local/canisters/<name>/<name>.wasm.gz`.

`root.wasm` stays thin again. Only the bootstrap `wasm_store.wasm.gz` is
embedded in `root`; the ordinary role `.wasm.gz` artifacts stay outside `root`
and are staged after `root` install from the build-produced
`.dfx/local/canisters/root/root.release-set.json` manifest by the Rust helpers
in `canic-installer`.

During normal custom builds, `scripts/app/build.sh` now opportunistically emits
that manifest as soon as the full root-subnet ordinary artifact set exists, so
downstreams do not need a local copy of the manifest-emission logic just to
keep `.dfx/local/canisters/root/root.release-set.json` in sync.
