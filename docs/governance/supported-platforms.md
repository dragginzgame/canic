# Supported Host And Target Matrix

This document is the sole authority for Canic's release-supported host and
Rust target combinations. An installer branch or an upstream binary asset does
not create a support claim.

## Release-Supported Matrix

| Host environment | Native target | Canister target | Status | Evidence owner |
| --- | --- | --- | --- | --- |
| Ubuntu 24.04, x86_64 | `x86_64-unknown-linux-gnu` | `wasm32-unknown-unknown` | Release-supported | Native lanes and the explicit serial PocketIC/Wasm lane in `.github/workflows/ci.yml`; RC/final gates in `docs/operations/release-validation-matrix.md`. |
| macOS, Apple Silicon | `aarch64-apple-darwin` | `wasm32-unknown-unknown` | Release-supported; qualification outstanding | `macos-host` on `macos-15` in `.github/workflows/ci.yml`; native results, tool installation and artifact qualification outstanding. |
| macOS, Intel | `x86_64-apple-darwin` | `wasm32-unknown-unknown` | Release-supported; qualification outstanding | `macos-host` on `macos-15-intel` in `.github/workflows/ci.yml`; native results, tool installation and artifact qualification outstanding. |

The supported cells cover the Canic CLI, host/build helpers, native checks and
tests, native release packages, and IC canister Wasm production. Existing CI
selects fixed `ubuntu-24.04`, `macos-15` and `macos-15-intel` runner images. The Rust toolchain versions,
downloaded tool versions, and archive digests are fixed by the workflow,
reviewed `ci/ic-tools.tsv`/`ci/tool-versions.env`, and Canic's
`tool-versions.env` executable and Cargo/lint identities.
The MSRV, ordinary-check and release-build lanes are native-target evidence.
Installing a Wasm target does not itself constitute Canister evidence; the
PocketIC lane owns CI's Wasm compilation and execution evidence.

## macOS Qualification

macOS support is a maintainer requirement. Missing CI coverage or a known macOS
failure is a support defect to correct, not grounds for classifying macOS as
unsupported. The support decision does not establish that existing releases
have passed macOS validation.

Maintainer Make and shell automation requires Bash 4.4 or newer, GNU coreutils,
GNU sed, jq and ripgrep. On macOS install them with
`brew install bash coreutils gnu-sed ripgrep`, then run `make install-tools` for
the reviewed repository JSON/YAML and IC toolsets. Put Bash's `bin`, coreutils'
`libexec/gnubin` and GNU sed's `libexec/gnubin` from `brew --prefix <formula>`
before system tools on `PATH`. This is a maintainer-runner prerequisite; installed
Canic CLI users do not need a replacement shell. Both macOS CI cells select these
tools and run the same success, failed-worker, interrupted-worker and owned-scratch
cleanup fixtures as Linux. Worker process groups use Bash job control and do not
require Linux's `setsid` utility.

Qualification must name the macOS version, architecture and Rust toolchain and
cover both declared native targets. It must include locked Host/CLI compilation,
targeted native durability and backup/restore tests, tool installation, and a
representative canister Wasm build. File-mode handling must respect the host's
native types; path handling must accommodate normal macOS paths while preserving
symlink and authority protections. Linux results and cross-compilation alone do
not prove these native filesystem behaviors.

Durable file creation uses the portable Rustix mode type. The `macos-host`
matrix installs and runs the pinned ICP CLI, ic-wasm, Binaryen and PocketIC tools,
qualifies the Rust Binaryen bundle installer and SDK executable resolution,
and verifies isolated hook preservation and safe setup. It
checks Host/CLI compilation, native durability, backup persistence and restore
recovery, and builds a representative application canister through the CLI. It
also checks the Control Plane's Wasm target. Its first successful
native results remain outstanding; the Wasm check does not qualify a linked
release artifact or its installation tools. Runner architectures follow
[GitHub's runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
The PocketIC installer selects pinned binaries for all three declared hosts.
The complete serial PocketIC lane is qualified on Linux; native macOS execution
of that lane still needs evidence. Linux-only process resource observations are
reported as unavailable on hosts without `/proc`.

## Install-Capable But Not Release-Supported

The checksum-bound actionlint, Binaryen, ShellCheck, ICP CLI, and
`ic-wasm` installers may contain branches outside the declared host/target
matrix. Those branches alone do not establish support. Their macOS branches
serve the supported macOS hosts and must be included in macOS qualification.

Other x86_64 Linux distributions may run the GNU/Linux tools, but only Ubuntu
24.04 is the declared Linux release host. Successful installation on another host
does not widen this matrix.

## Explicit Exclusions

- Windows is not release-supported and the repository installers reject it.
- PocketIC's repository installer supports the three declared native hosts;
  upstream Linux ARM assets do not extend this release matrix.
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

## Continue From Here

- [Install Canic](../../INSTALLING.md)
- [Review CI and deployment governance](ci-deployment.md)
- [Browse all documentation](../README.md)
- [Back to the main README](../../README.md)
