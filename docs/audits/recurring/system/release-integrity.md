# Audit: CI And Release Integrity

## Method Contract

- Audit ID: `CANIC-RELEASE-INTEGRITY-001`
- Method version: `1`
- Disposition: `retain`
- Owner: CI trust, secret exposure, artifact provenance/checksums, and
  supported host/target release matrix
- Kind/profile: release `invariant` plus named manual review
- Trace mode: `code_trace`; package/release probes execute only in disposable
  local or CI environments
- Cost/runtime: medium/high; 45-120 minutes excluding package builds
- Prerequisites: Git, ripgrep, actionlint, current workflow files, release
  scripts, package metadata, and an approved secret scanner for closeout
- False-positive boundary: example placeholders and documented test fixtures
  are classified separately from usable credentials or untrusted execution
- Shared contract: [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)

## Purpose

Prove that CI and release automation use least authority, immutable external
inputs, bounded secrets, attributable artifacts, and an explicit supported
host/target matrix.

This method audits the automation contract. It does not authorize release
version changes, commits, tags, pushes, publication, or deployment.

## Invariants

- Workflow permissions are absent or least-privilege for each job.
- Third-party actions are pinned to immutable commit identities; mutable tags
  are findings unless the action is repository-owned and locally resolved.
- Pull-request or untrusted inputs cannot execute with production/release
  credentials.
- Secret values, tokens, private keys, and sensitive environment material are
  neither committed nor retained in audit/build logs.
- Published artifacts have source/tool/feature provenance and cryptographic
  checksums before promotion.
- Supported host and target combinations are declared and exercised or
  explicitly excluded.
- Human-owned versioning, staging, tag, push, publish, and deployment
  boundaries remain enforced.

## Scope

- `.github/workflows/`, action configuration, and workflow permissions;
- release/package/install/build scripts and Make targets;
- Cargo publishing metadata, install URLs, release index, checksums, and
  provenance output;
- secret-handling inputs and log/report retention paths; and
- Rust host, Wasm target, external tools, and supported execution matrix.

Historical workflows and reports are evidence only, not active authority.

## Required Checks

```bash
actionlint
rg -n '^permissions:|^[[:space:]]+permissions:|uses:|pull_request_target|workflow_run|secrets\.|GITHUB_TOKEN' .github -g '*.yml' -g '*.yaml'
rg -n 'curl|wget|sha256|checksum|provenance|artifact|cargo publish|git tag|git push' Makefile scripts .github docs/governance -g '*.sh' -g '*.yml' -g '*.yaml' -g '*.md' -g 'Makefile'
rg -n 'BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9_]{20,}' . --hidden --glob '!.git/**' --glob '!target/**'
```

The local pattern scan is necessary but not sufficient. Closeout also records
an approved dedicated secret scanner, its version/rules, and its result. If no
approved scanner is available, this required method is `blocked`.

Review every external `uses:` value for immutable pinning and every job's
effective permission/secret boundary. Review downloaded tools for checksum or
equivalent integrity verification.

## Artifact And Matrix Proof

Record:

- source commit/tree, lockfile/toolchain/features, builder environment, and
  command for every promoted artifact class;
- raw/shrunk/compressed/package checksums and their publication boundary;
- current host OS/architecture assumptions;
- Wasm and native target support;
- installed external tool versions; and
- which matrix cell is local, CI, maintainer-only, or unsupported.

An undocumented or unvalidated claimed support cell is an evidence gap. An
artifact promoted without attributable source identity or checksum is a
failure.

## Fixtures And Boundaries

- Positive: pinned workflow/action and checksum-bound artifact path.
- Rejection: untrusted PR input cannot reach release secrets or publication.
- Boundary: maintainer-only release actions stay outside automated agent
  authority.
- Regression: mutable action tag, unchecked download, or retained credential
  pattern is detected and fails review.

## Required Report

Include run identity, workflow permission/action table, secret-scan manifest,
download/tool integrity table, artifact provenance/checksum map, host/target
matrix, human authority boundaries, findings, unreviewed boundaries, and
verdict.
