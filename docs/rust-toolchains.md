# Rust toolchain preparation

Snapshot `.github/actions/prepare-rust-toolchains/action.yml` to share explicit
Rust development/MSRV setup across native workflows. Rustup and `/bin/bash` must
already be available. The action installs the development toolchain with the
minimal profile, rustfmt and Clippy, then the minimum toolchain with the minimal
profile and the caller's required target. It does not change the default toolchain.

```yaml
- uses: ./.github/actions/prepare-rust-toolchains
  with:
    development: '1.99.0'
    minimum: '1.88.0'
    development-target: wasm32-unknown-unknown
    minimum-target: wasm32-unknown-unknown
    minimum-components: clippy,rustfmt
```

These versions illustrate the original Jobs caller, not fleet-wide pins. Each
consumer owns its versions and targets; omit `development-target` when that
workflow does not build Wasm with the development toolchain. Toolchain arguments
are passed separately, never evaluated as shell code. Missing required values,
leading options and whitespace are rejected before installation. Installation
failure stops immediately and preserves rustup's status and diagnostics.

The action provides setup only. Consumers retain validation gates, caches,
`rustc +VERSION --version --verbose` evidence, release/publication authority and
native host qualification. Optional `minimum-components` selects comma-separated
components when the minimum-toolchain lane also runs lint or formatting.

Local setup or single-toolchain CI lanes can call the same installer directly:

```bash
bash scripts/dev/prepare-rust-toolchain.sh 1.91.0 \
  --component clippy --component rustfmt --target wasm32-unknown-unknown
```

The standalone entrypoint admits the complete explicit argument list before
calling rustup. It accepts repeated component/target options, uses the minimal
profile and preserves rustup failures. Consumers retain their component policy
and version selection; checks must never call this setup entrypoint implicitly.

Adopt from a reviewed, published Shared Tooling revision using the
[snapshot workflow](consuming-snapshots.md), then replace the local action and
both ordinary/recovery callers together. The offline fixture executes the actual
YAML body against a substitute rustup, checking argument parity, failure
propagation and input admission. It does not qualify real downloads or macOS;
adopters must retain their native Linux/macOS checks.
