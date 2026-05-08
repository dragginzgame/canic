# CI and Deployment Governance

This document is the authoritative workflow policy for commands, git,
versioning, releases, and deployment-adjacent automation.

## Commands

- Format: `cargo fmt --all`
- Check: `make check`
- Lint: `make clippy`
- Test: `make test`
- Build: `make build`
- Merge verification normally includes `make fmt-check`, `make clippy`, and
  `make test` unless the task or maintainer explicitly narrows scope.

## Git Boundary

Automated agents must never run:

- `git add`
- `git commit`
- `git push`

Agents may inspect state with read-only commands such as `git status`,
`git diff`, `git log`, and `git show`. Humans own staging, commits, pushes,
tags, and history.

Do not rewrite history or tags. Do not revert user changes unless explicitly
requested.

## Versioning and Release

Automated agents must never change release version numbers directly.

Do not run:

- `cargo set-version`
- `scripts/ci/sync-release-surface-version.sh`
- `scripts/ci/bump-version.sh`
- `make patch`
- `make minor`
- `make major`

Release bumps are human-owned. The normal human release path is `make patch`,
`make minor`, or `make major`, followed by review of generated changes, commit,
tag, and intentional push.

Tags are immutable.

## Network Selection

- `ICP_ENVIRONMENT` selects the target ICP CLI environment.
- If unset, it defaults to `local`.
- Canic automation should use `icp-cli`/`icp.yaml` environments rather than
  `icp` networks or `icp.yaml`.

## Automation Language Boundary

Do not add Python code, `.py` scripts, Python build helpers, Python test
helpers, or Python CI glue to this repository.

Prefer Rust for durable tooling. Use shell only when a small wrapper is
sufficient.
