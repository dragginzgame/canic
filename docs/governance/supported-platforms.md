# Supported Host And Target Matrix

This document is the sole authority for Canic's release-supported host and
Rust target combinations. An installer branch or an upstream binary asset does
not create a support claim.

## Release-Supported Matrix

| Host environment | Native target | Canister target | Status | Evidence owner |
| --- | --- | --- | --- | --- |
| Ubuntu 24.04, x86_64 | `x86_64-unknown-linux-gnu` | `wasm32-unknown-unknown` | Release-supported | All four jobs in `.github/workflows/ci.yml`; RC/final gates in `docs/operations/release-validation-matrix.md`. |

The supported cell covers the Canic CLI, host/build helpers, workspace checks,
tests, native release packages, and IC canister Wasm production. CI selects the
fixed `ubuntu-24.04` runner image. The Rust toolchain versions, downloaded tool
versions, and archive digests are fixed by the workflow and
`tool-versions.env`.

## Install-Capable But Not Release-Supported

The checksum-bound actionlint, ShellCheck, Gitleaks, ICP CLI, and `ic-wasm`
installers also contain Linux AArch64 and macOS x86_64/AArch64 branches. Those
branches are retained for maintainer convenience, but they are not
release-supported cells because no current CI or required maintainer gate
exercises them.

Other x86_64 Linux distributions may run the GNU/Linux tools, but only Ubuntu
24.04 is the declared release host. Successful installation on another host
does not widen this matrix.

## Explicit Exclusions

- Windows is not release-supported and the repository installers reject it.
- PocketIC's repository installer supports Linux x86_64 only.
- Native targets other than `x86_64-unknown-linux-gnu` are not release
  targets.
- Canister targets other than `wasm32-unknown-unknown`, including WASI
  targets, are not supported.
- Mainnet is a deployment environment, not a build host or Rust target. This
  matrix does not authorize deployment or production mutation.

Adding a supported cell requires an explicit governance change plus named CI
or maintainer-owned validation evidence. Adding an installer branch alone is
insufficient.
