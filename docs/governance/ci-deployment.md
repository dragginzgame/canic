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
- Combine multiple compatible slices into one coherent batch or open patch
  draft when that makes review and publishing less noisy.
- Maintain the changelog by default when a meaningful code or behavior batch
  is complete. Reuse an existing untagged patch draft; otherwise prepare the
  next patch draft according to the [changelog policy](changelog.md).
- A changelog draft version is documentation planning, not a package-version
  bump. Release version files remain owned by the human release flow.

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

Release-line-specific validation matrices may further classify existing checks
for a bounded release line. Use
[docs/operations/release-validation-matrix.md](../operations/release-validation-matrix.md)
as the current matrix for slice close-out, implementation close-out, RC
promotion, and final release/tag validation. The matrix interprets this
governance policy for the active release line; it does not override the git,
versioning, or release boundaries in this document.

The sole supported host and Rust target authority is the
[supported host and target matrix](supported-platforms.md). Installer branches
outside a declared and validated cell do not create support claims.

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
The Make version targets complete their required test and feature gates before
changing package versions; any failed gate leaves the version unchanged. The
underlying bump script rejects direct invocation without the private gate
marker supplied by those targets.
Before its final atomic network update, `make release-push` verifies the exact
clean release commit/tag pair and clears Cargo build artifacts. No fallible
local cleanup step runs after a successful push, and atomic push prevents a
branch-only or tag-only remote update. A transport interruption can still make
the remote outcome uncertain and must be resolved by inspecting the remote
refs before retrying.
For one-shot releases, humans may run `make release-patch`,
`make release-minor`, or `make release-major`, which perform those steps in
order.
Minor and major release bumps require interactive command-line confirmation
before running their release gates.

Tags are immutable.

## Network Selection

- `ICP_ENVIRONMENT` selects the target ICP CLI environment.
- If unset, it defaults to `local`.
- Canic automation should target environments declared in `icp.yaml`.
- Use `ICP_ENVIRONMENT` for Make/script defaults and `canic --network <name>`
  for one-off CLI commands.
- Do not use DFX-era network variables as the Canic automation selector.

## Automation Language Boundary

Do not add Python code, `.py` scripts, Python build helpers, Python test
helpers, or Python CI glue to this repository.

Prefer Rust for durable tooling. Use shell only when a small wrapper is
sufficient.
