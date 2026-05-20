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

## Development Slices and Validation Tiers

A code slice is a small, focused implementation unit chosen for reviewability
and safety. It is not a release patch by default.

Default development cadence:

- Prefer roughly 20 minutes of coherent development work per batch when the
  task is open-ended.
- Keep individual code slices focused by concern, module, or invariant.
- Combine multiple compatible slices into one unreleased batch when that makes
  review and publishing less noisy.
- Do not assign patch versions during ordinary development. Version numbers are
  assigned during release preparation by the human-owned release flow.

Validation is tiered:

- Focused slice checks: run the smallest format, test, lint, or compile command
  that exercises the touched code and the relevant invariant.
- Broader batch checks: after a coherent batch or when touching cross-cutting
  behavior, add wider package or workspace checks as risk warrants.
- Full release checks: reserve full merge/release validation for release-ready
  or push-ready states, or when a maintainer explicitly asks for broad checks.

For documentation-only governance changes, use docs-appropriate validation such
as formatting, whitespace, link-shape review, and `git diff --check`. Do not run
code test suites unless code files changed or the maintainer asks for them.

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
- `make release-patch`
- `make minor`
- `make release-minor`
- `make major`
- `make release-major`

Release bumps are human-owned. The normal human release path is `make patch`,
`make minor`, or `make major`, followed by review of generated changes. Once
reviewed, humans finish the release with `make release-stage`,
`make release-commit`, and `make release-push`.
For one-shot releases, humans may run `make release-patch`,
`make release-minor`, or `make release-major`, which perform those steps in
order.

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
