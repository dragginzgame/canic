# Build Artifacts

This is the canonical vocabulary for Canic build outputs and generated canister
interfaces after the 0.33 ICP CLI hard cut.

## Project State

- `.icp/` is the local ICP CLI project state root.
- `.canic/` is Canic operator state. Deployment-target install state lives under
  `.canic/<network>/deployments/<deployment>.json`.

## Canister Build Contract

Every Canic-managed canister package declares its runtime role in Cargo
metadata:

```toml
[package.metadata.canic]
fleet = "demo"
role = "app"
```

The package role is the single source of truth for the build and startup
macros. `canic::build!("<path-to-canic.toml>")` validates that the metadata role
exists in the named fleet config and emits the compile-time role/config
environment consumed by `canic::start!()`.

`role = "root"` emits the root build cfgs and selects the root lifecycle and
root endpoint bundle. Every other configured role selects the non-root
lifecycle and endpoint bundle. There is no separate public root startup macro.

Ordinary roles may be declared before topology placement so `cargo check` can
run during early development. `canic build <fleet> <role>` is stricter: the
role must be attached to topology before Canic writes deploy artifacts.

Build provenance is opt-in:

```text
canic build <fleet> <role> --provenance <path>
```

The provenance file is an `EvidenceEnvelopeV1` containing stable
`canic.build_provenance.v1` payload. It records source, Cargo, package
metadata, build-profile, and artifact hash evidence after a successful build.
It does not change deployment truth, install state, controllers, topology, or
artifact registry state.

Saved build provenance can be supplied back to passive evidence envelopes:

```text
canic fleet adoption report <fleet> --profile <profile> --format envelope-json --build-provenance <path>
canic deploy check <deployment> --format envelope-json --build-provenance <path>
```

Those commands only fingerprint the saved provenance envelope as input
evidence. They do not re-run builds or treat provenance as deployment truth.

## Canister Artifacts

- Direct Cargo canister builds emit raw wasm under
  `target/wasm32-unknown-unknown/<profile>/canister_<name>.wasm`.
- Canic release artifacts are gzip-compressed `.wasm.gz` files owned by the
  Canic build scripts.
- ICP CLI-visible canister artifacts live under
  `.icp/<environment>/canisters/<role>/`.

## Candid Extraction

`canic::finish!()` emits the `ic_cdk::export_candid!()` pointer only for debug
builds. Host builds use that debug-only pointer to run `candid-extractor` for
local development artifacts.

For `ICP_ENVIRONMENT=local`, Canic:

- builds a debug Wasm for Candid extraction;
- writes `.icp/local/canisters/<role>/<role>.did`;
- embeds public `candid:service` metadata into the local Wasm artifact for
  local `icp canister metadata` inspection.

For `ICP_ENVIRONMENT=ic`, Canic intentionally skips Candid extraction, removes
the generated `.did` sidecar path, and does not embed `candid:service`
metadata. This keeps production Wasm artifacts from carrying local interface
metadata bloat. Running `candid-extractor` directly against an `ic` release
Wasm is not the supported path; use the local/debug extraction path instead.

## Audit Usage

Recurring audits should use this vocabulary instead of restating tool-specific
paths inline. When an audit needs a concrete local environment path, use
`local` unless the report preamble names another environment.
