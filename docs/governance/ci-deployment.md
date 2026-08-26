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

`make validate` has three sequential barriers. The first runs every independent
formatting, repository-invariant, dependency, secret, shell and feature check,
then reports their complete failure set. Workspace checking and Clippy start
only when that barrier passes. The complete test graph starts only when both
compile/lint targets pass, so a deterministic compiler or warning failure never
leaves PocketIC running. Targets within each barrier remain sequential so
independent Cargo processes do not contend for the same build graph. Complete
failed-target logs are retained under
`target/validation-failures/`; the terminal summary repeats bounded failure
detail and the exact failed target list.

Make-based work shares the repository `target/`. When `sccache` is available
and no explicit `RUSTC_WRAPPER` is set, Make selects it and disables Rust
incremental compilation so compiler results remain cacheable. Without a
wrapper, Make leaves Cargo's profile defaults intact: local dev/test work may
remain incremental while `release` and `fast` artifacts stay non-incremental.
Explicit `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL` and `RUSTC_WRAPPER` values
remain authoritative. Canic artifact builds keep incremental compilation
disabled for deterministic Wasm output and independently discover `sccache`
for `canic build` and `canic install` when no wrapper was supplied.

CI uses the same runner for its preflight, security and Rust-check jobs. Tool
installation and version verification remain immediate prerequisites, after
which each job reports every independent policy, security or compile-check
failure in one run. Every expensive compile and test job still requires both
cheap gate jobs to pass.

The repository owns one `pre-commit` hook, configured by `make install-dev` or
`make install-hooks`. It runs only `make fmt`; it does not run tests, Clippy,
builds, validation, versioning, commits, or pushes. A partially staged file
rejects before formatting because formatting the working copy cannot prove the
staged snapshot. After successful formatting, the hook refreshes the index only
for files that were already staged and tracked files that were clean before the
formatter changed them. It never stages pre-existing unstaged edits and rejects
if formatting changes such a file. Therefore `git add .` followed by
`git commit` commits the formatted snapshot without a second staging pass, while
unrelated unstaged content remains byte-for-byte unchanged. `make fmt-check`
remains in validation and CI so hook bypass does not weaken the release
boundary.

`make test` executes every top-level integration test recorded in the guarded
workspace test inventory. New integration targets must declare their release
lane, execution class and suite before the gate accepts them. Ordinary tests
retain libtest's default parallelism; PocketIC suites remain explicitly
single-threaded and ordered until a measured narrower concurrency policy is
proven stable. After every serial suite the runner reports the shared server's
current resident memory, resident high-water mark and thread count from the
release-supported Linux process boundary. `make test-wasm` is the fast lane and
runs only its classified release-surface integrations, never the PocketIC
suites.
Cargo continues across independently selected test binaries inside each cost
tier, and the workspace runner records every failed suite before returning one
nonzero result. A failed ordinary tier is a hard barrier in the combined local
runner: it reports all ordinary failures and skips the serial PocketIC tier.
Plan-only inventory resolution still enumerates both tiers, and the explicit
PocketIC-only mode remains independently runnable. In CI, one ignored governed
`canic-testing-internal` harness calls every internal PocketIC case in explicit
order inside one Rust process. Fleet deployment restore and autonomous Root
removal are the first two cases; the harness reports each result immediately,
catches failures through the suite boundary and retains the process-local Fleet
baseline and artifact owners. The restore proof uses that baseline, while the
destructive Root-removal case uses an exclusive fresh instance because canister
deletion is outside the snapshot-reset contract. The matching pure internal
cases run in the ordinary tier before PocketIC. This keeps stateful deployment
recovery locally attributable while avoiding three cold process-local Fleet
baselines. The PocketIC lane clears transient heavy Wasm targets once before
its integration-suite group and once at
invocation cleanup; it retains Cargo freshness between the ordered suites.
CI may run the ordinary and PocketIC lanes in separate jobs; it must not
parallelize the PocketIC suites themselves without replacing this measured
policy. Cheap source/governance preflight and security jobs gate every compile
and test lane so a deterministic repository-policy failure does not leave an
expensive PocketIC job running.

The governed PocketIC runner resolves one repository-pinned server binary,
verifies its exact checksum even when `POCKET_IC_BIN` was supplied by the
caller, then starts one shared server in the invocation-owned private scratch
immediately before the serial PocketIC lane. The runner admits a numeric port
within 30 seconds, retains bounded stdout/stderr for startup failure and gives
the process a two-hour idle and hard lifetime. It retains the exact child PID,
stops and waits for it on every handled exit, and leaves invocation-scratch
cleanup as a crash-safety fallback bound to the numeric direct-child port path.
A failed suite prints bounded tails from both server streams next to its own
retained log. `ic-testkit` 0.8.9 owns the corrected bounded managed-server
primitive for one Rust process; Canic keeps a runner-owned server because the
serial lane still crosses the internal harness and several integration-test
processes. Repository fixtures use
testkit connect mode with their own 30-second instance-construction deadline.
Direct PocketIC test commands outside the governed runner must supply
`CANIC_POCKET_IC_SERVER_URL`; they fail immediately when it is absent rather
than spawning an implicit or unobservable child process.

## Development Slices and Validation Tiers

A code slice is a small, focused implementation unit chosen for reviewability
and safety. It is not a release patch by default.

Release grouping, continuation and handoff readiness are governed by
[delivery cadence governance](delivery-cadence.md). A minor has no minimum
release count and planned design cadence should normally publish no more than
12 releases; necessary post-publication correctness, security, recovery and
operator-regression fixes may exceed that guideline. An implementation slice
is not automatically a release.

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

- Automated coding work runs only the smallest targeted format, test, lint, or
  compile commands that exercise the touched code and relevant invariant.
- Human/CI batch validation may add wider package checks when cross-cutting
  behavior warrants them.
- The human-owned deployment/version/release flow owns workspace-wide,
  release-matrix, broad PocketIC and complete `make validate` gates.

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
ordinary status and planning prose must not act as a parallel package-version
source. Current and committed version queries must use the shared pinned
`cargo-get` reader; release scripts must not maintain parallel manifest
parsers. The governed bump is the one exception: after validating one exact
clean source commit, it seals the matching detailed changelog draft with the
release date, converts the exact open predecessor/target statement into a
sealed release-lineage statement, and writes that source commit into one
machine-checked current-status release marker. The release commit may then
differ from the validated source only in the enumerated version, lock,
installer, changelog and status surfaces. The cheap current-document semantics
gate still rejects volatile
"latest published" and manual release-truth prose elsewhere. After staging,
`make release-commit` runs the fast
post-bump `make release-candidate` guard before committing or tagging. That
guard verifies the sealed changelog and source marker, rejects every
non-release change after the validated source, and checks locked offline Cargo
metadata, uniform workspace package versions and the installed-CLI default
without repeating the already completed full source validation.

When an accepted release batch declares an exact downstream pre-publication
qualification gate, freeze one clean source commit before running that gate.
Build the candidate executable from that commit and run the declared no-effect
preflight before version mutation or publication. A source change invalidates
the downstream result and requires a new candidate commit; release-only
version surfaces do not change the qualified source. Run focused checks while
editing, the one declared production-boundary journey before review, and the
complete `make validate` gate once through the normal human version target
after the source candidate is frozen. Publication may proceed only when the
downstream preflight and normal release gate both identify that unchanged
source candidate.

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
explicitly disables implicit followed-tag publication and sends both the
current branch ref and the exact workspace-version tag ref in one atomic push,
so the tag is still sent
when the branch commit is already present remotely. No fallible local cleanup
step runs after a successful push, and atomic push prevents a branch-only or
tag-only remote update. A transport interruption can still make the remote
outcome uncertain and must be resolved by inspecting the remote refs before
retrying.

The historical-tag deletion helper removes remote refs before local refs and
verifies both requested boundaries. Deleted annotated tags remain present in
other clones until those clones remove them. A later `git push --tags` or
`git push --follow-tags` from such a clone republishes them and must not be
used; the exact release push is the maintained tag-publication path.
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

Publishing first re-runs the release-candidate guard, then verifies that every
crate in the governed publish order exposes the same workspace version before
declaring the package set available. A successful subset or library/CLI split
is never reported as a complete Canic release.

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
