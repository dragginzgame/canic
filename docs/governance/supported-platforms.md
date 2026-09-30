# Supported Host And Target Matrix

This document is the sole authority for Canic's release-supported host and
Rust target combinations. An installer branch or an upstream binary asset does
not create a support claim.

## Release-Supported Matrix

| Host environment | Native target | Canister target | Status | Evidence owner |
| --- | --- | --- | --- | --- |
| Ubuntu 24.04, x86_64 | `x86_64-unknown-linux-gnu` | `wasm32-unknown-unknown` | Release-supported | Native lanes and the explicit serial PocketIC/Wasm lane in `.github/workflows/ci.yml`; RC/final gates in `docs/operations/release-validation-matrix.md`. |
| macOS, Apple Silicon | `aarch64-apple-darwin` | `wasm32-unknown-unknown` | Release-supported; qualification outstanding | Host/CLI macOS CI coverage in `.github/workflows/ci.yml` or retained maintainer-run evidence on this target; coverage must be added. |
| macOS, Intel | `x86_64-apple-darwin` | `wasm32-unknown-unknown` | Release-supported; qualification outstanding | Host/CLI macOS CI coverage in `.github/workflows/ci.yml` or retained maintainer-run evidence on this target; coverage must be added. |

The supported cells cover the Canic CLI, host/build helpers, native checks and
tests, native release packages, and IC canister Wasm production. Existing CI
selects the fixed `ubuntu-24.04` runner image. The Rust toolchain versions,
downloaded tool versions, and archive digests are fixed by the workflow and
`tool-versions.env`.
The MSRV, ordinary-check and release-build lanes are native-target evidence.
Installing a Wasm target does not itself constitute Canister evidence; the
PocketIC lane owns CI's Wasm compilation and execution evidence.

## macOS Qualification

macOS support is a maintainer requirement. Missing CI coverage or a known macOS
failure is a support defect to correct, not grounds for classifying macOS as
unsupported. The support decision does not establish that existing releases
have passed macOS validation.

Qualification must name the macOS version, architecture and Rust toolchain and
cover both declared native targets. It must include locked Host/CLI compilation,
targeted native durability and backup/restore tests, tool installation, and a
representative canister Wasm build. File-mode handling must respect the host's
native types; path handling must accommodate normal macOS paths while preserving
symlink and authority protections. Linux results and cross-compilation alone do
not prove these native filesystem behaviors.

The current `durable_io` file-mode compilation failure and macOS filesystem
coverage identified in the code review remain outstanding qualification work.
The existing governed PocketIC runner and installer remain Linux x86_64 tooling;
their platform restriction does not exclude macOS Host/CLI support. Adding a
macOS PocketIC runner requires its own installer and execution evidence.

## Install-Capable But Not Release-Supported

The checksum-bound actionlint, Binaryen, ShellCheck, Gitleaks, ICP CLI, and
`ic-wasm` installers may contain branches outside the declared host/target
matrix. Those branches alone do not establish support. Their macOS branches
serve the supported macOS hosts and must be included in macOS qualification.

Other x86_64 Linux distributions may run the GNU/Linux tools, but only Ubuntu
24.04 is the declared Linux release host. Successful installation on another host
does not widen this matrix.

## Explicit Exclusions

- Windows is not release-supported and the repository installers reject it.
- PocketIC's repository installer supports Linux x86_64 only.
- Native targets outside the three declared matrix entries are not release
  targets.
- Canister targets other than `wasm32-unknown-unknown`, including WASI
  targets, are not supported.
- Mainnet is a deployment environment, not a build host or Rust target. This
  matrix does not authorize deployment or production mutation.

Adding a supported cell requires an explicit maintainer decision, a governance
update and a named CI or maintainer-owned qualification plan. Outstanding
qualification must remain explicit until evidence exists; declaring support or
adding an installer branch is not passing validation evidence.
