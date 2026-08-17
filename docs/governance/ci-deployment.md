# CI and Deployment Governance

This document is the authoritative workflow policy for commands, git,
versioning, releases, and deployment-adjacent automation.

## Commands

- Format: `cargo fmt --all`
- Check: `make check`
- Lint: `make clippy`
- Test: `make test`
- Build: `make build`
- Repository invariants: `make check-invariants`
- Shell automation lint: `make shellcheck`
- Complete local validation: `make validate`
- Release-cadence advisory: `make release-cadence`

Primitive targets perform only the operation they name. They do not configure
Git hooks, format before checking, or invoke unrelated invariant, feature,
lint, build, or test targets. `make validate` is the explicit composition
boundary for the complete local workflow.

The repository owns one `pre-commit` hook, configured by `make install-dev` or
`make install-hooks`. It runs only `make fmt`. It never stages files or runs
tests, Clippy, builds, validation, versioning, commits, or pushes. A partially
staged file rejects before formatting because formatting the working copy
cannot prove the staged snapshot. If formatting changes tracked working-tree
content, the hook rejects so the maintainer can review and stage the result.
Pre-existing unrelated unstaged changes do not reject when formatting leaves
them byte-for-byte unchanged. `make fmt-check` remains in validation and CI so
hook bypass does not weaken the release boundary.

`make test` executes every top-level integration test recorded in the guarded
workspace test inventory. New integration targets must declare their release
lane, execution class and suite before the gate accepts them. Ordinary tests
retain libtest's default parallelism; PocketIC suites remain explicitly
single-threaded and ordered until a measured narrower concurrency policy is
proven stable. `make test-wasm` is the fast lane and runs only its classified
release-surface integrations, never the PocketIC suites.
CI may run the ordinary and PocketIC lanes in separate jobs; it must not
parallelize the PocketIC suites themselves without replacing this measured
policy. Cheap source/governance preflight and security jobs gate every compile
and test lane so a deterministic repository-policy failure does not leave an
expensive PocketIC job running.

## Development Slices and Validation Tiers

A code slice is a small, focused implementation unit chosen for reviewability
and safety. It is not a release patch by default.

Release grouping, continuation and handoff readiness are governed by
[delivery cadence governance](delivery-cadence.md). A minor has no minimum
release count and should normally publish no more than 12 releases; an
implementation slice is not automatically a release.

Default development cadence:

- Choose batch boundaries by complete outcomes rather than elapsed time.
- Keep individual code slices focused by concern, module, or invariant.
- Combine compatible implementation, direct evidence, propagation and cleanup
  slices into the current planned release batch and open patch draft.
- Keep routine compile, lint, fixture and documentation fallout in that batch;
  do not turn it into another patch release.
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
Before patch validation and version mutation, `make patch` prints the
read-only `make release-cadence` advisory. The advisory reports when the next
release would exceed the soft 12-release minor-line guideline but never blocks
or expands the maintainer's release authority.
The Make version targets require a clean source tree, run the same explicit
`make validate` workflow and recheck tracked cleanliness before changing
package versions. They do
not mutate source formatting; the pre-commit hook handles routine formatting,
while validation's `make fmt-check` catches bypassed hooks. Any failed target
leaves the version unchanged. The underlying bump script rejects direct
invocation without the private validation marker supplied by those targets.
The root `Cargo.toml` is the sole live workspace package-version authority;
status and planning documents must not duplicate a version whose release-only
commit they cannot update. Current and committed version queries must use the
shared pinned `cargo-get` reader; release scripts must not maintain parallel
manifest parsers. After staging, `make release-commit` runs the fast
post-bump `make release-candidate` guard before committing or tagging. That
guard verifies locked offline Cargo metadata, uniform workspace package
versions and the installed-CLI default without repeating the already completed
full source validation.

The test target allocates one private repository-owned
`.tmp/test-runtime.<suffix>` directory. It clears only that scratch on success,
ordinary failure or handled interrupt. Before removing it, cleanup forcibly
stops only a detached PocketIC server whose exact `--port-file` is a direct
child of that invocation's scratch; this avoids the upstream server's late
socket-teardown panic without touching another invocation's server. Cleanup
never sweeps a shared path or another concurrent invocation's scratch. Canic
scripts must clean their own temporary files; explicit cleanup must not sweep
unrelated repository scratch or global `/tmp` content.
Before its final atomic network update, `make release-push` verifies the exact
release commit/tag pair from committed `HEAD`. It does not format, compile,
test, validate, or clean. Local
staged, unstaged and untracked changes neither block the push nor join it; they
remain local. The release version is read from `HEAD`'s committed `Cargo.toml`,
so a later local manifest edit cannot redirect tag selection. Test scratch has
already been removed by the test invocation that owned it. Release push
explicitly sends both the current branch ref and the exact workspace-version
tag ref in one atomic push, so the tag is still sent
when the branch commit is already present remotely. No fallible local cleanup
step runs after a successful push, and atomic push prevents a branch-only or
tag-only remote update. A transport interruption can still make the remote
outcome uncertain and must be resolved by inspecting the remote refs before
retrying.
GitHub Actions intentionally does not run a separate tag-only workflow. The
new `main` release commit owns one CI result containing preflight, security,
MSRV, Rust checks, ordinary tests, serial PocketIC tests and the conditional
release-profile workspace build. A green tag must never coexist with a red CI
result for the same source merely because the tag ran a weaker job graph.
For one-shot releases, humans may run `make release-patch`,
`make release-minor`, or `make release-major`, which perform those steps in
order.
Minor and major release bumps require interactive command-line confirmation
before running `make validate`.

Tags are immutable.

The dependency-risk inventory also runs on a weekly read-only schedule so a
new advisory is visible even when the repository receives no source push.

## Environment Selection

- `ICP_ENVIRONMENT` selects the target ICP CLI environment.
- If unset, it defaults to `local`.
- Canic automation should target environments declared in `icp.yaml`.
- Use `ICP_ENVIRONMENT` for Make/script defaults and `canic --environment <name>`
  for one-off CLI commands.
- Do not use DFX-era network variables as the Canic automation selector.

## Automation Language Boundary

Do not add Python code, `.py` scripts, Python build helpers, Python test
helpers, or Python CI glue to this repository.

Prefer Rust for durable tooling. Use shell only when a small wrapper is
sufficient.
