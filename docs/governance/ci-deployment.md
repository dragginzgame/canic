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
- Eligible non-runtime patch validation and bump: `make patch-fast`
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

## Command Authority

An unambiguous maintainer instruction in the current conversation authorizes
the exact Git, version, release, publication or deployment action it names.
Natural language is enough; automation must not require a magic phrase, a
second confirmation or a hand-executed command. The active repository,
version, Fleet and environment context may resolve the target when only one is
possible. Ask once when the target or external effect remains genuinely
ambiguous. Generic continuation, readiness and audit requests do not authorize
external effects.

Before an authorized network effect, retain the checks that prevent actual
damage: exact network and identity, exact reviewed plan digest, maximum debit,
cycle conservation and duplicate-effect protection. If those facts still
match the reviewed plan, do not add another ceremony gate. A changed digest,
environment, identity, debit bound or destructive disposition requires a new
plan or maintainer decision.

Release-blocking guards validate machine-relevant facts, not editorial prose.
Exact checks are appropriate for structured records, identifiers, versions,
digests, schemas, executable command ownership and required file/link
presence. They must not freeze explanatory sentences, line wrapping,
illustrative values, a full heading inventory or ordinary readiness narrative.
If a documentation fact must drive automation, represent it as a dedicated
machine-readable field. Documentation gates remain lightweight and must not
turn wording cleanup into a failed compile/test release cycle.

Make-based work shares the repository `target/`. When `sccache` is available
and no explicit `RUSTC_WRAPPER` is set, Make selects it and disables Rust
incremental compilation so compiler results remain cacheable. Without a
wrapper, Make leaves Cargo's profile defaults intact: local dev/test work may
remain incremental while `release` and `fast` artifacts stay non-incremental.
Explicit `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL` and `RUSTC_WRAPPER` values
remain authoritative. Canic artifact builds keep incremental compilation
disabled for deterministic Wasm output and independently discover `sccache`
for `canic build` when no wrapper was supplied.
Do not run a second Canic Cargo/check/test process against the same repository
`target/` during validation. Cargo will serialize parts of those graphs on its
build-directory lock while both processes still compete for CPU and memory;
changing source or `Cargo.lock` underneath the validating process can also turn
an otherwise quick immutability assertion into a late failure. Read-only plan
inspection remains safe while the owned validation finishes.

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
The ordinary integration inventory is resolved into one multi-package Cargo invocation so
its shared dependency graph is compiled once rather than once per owning
package. Unit/lib/bin coverage and the internal fast harness remain separate
where their target and fixture contracts differ. Timing output calls this
`libtest-parallel` to distinguish parallelism inside one Cargo invocation from
concurrent suite execution. When Make selects `sccache`, the runner reports
request/hit/miss deltas, retains the server through the complete two-hour test
envelope and uses a 40 GiB local cache; a reset is reported rather than
silently presenting zero requests as cache evidence.
Cargo continues across independently selected test binaries inside each cost
tier, and the workspace runner records every failed suite before returning one
nonzero result. A failed ordinary tier is a hard barrier in the combined local
runner: it reports all ordinary failures and skips the serial PocketIC tier.
Plan-only inventory resolution still enumerates both tiers, and the explicit
PocketIC-only mode remains independently runnable. In CI, one ignored governed
`canic-testing-internal` harness calls every internal PocketIC case in explicit
order inside one Rust process. Fleet deployment restore and autonomous Root
removal are the first two cases; the harness reports each result immediately,
prints the ten slowest cases, catches failures through the suite boundary and
retains the process-local Fleet
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
- The maintainer-directed deployment/version/release flow chooses whether the
  complete gate or the governed fast patch lane is appropriate.

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

Automated agents may stage, commit, tag and push when the maintainer explicitly
requests those actions in the current conversation. The instruction need not
use prescribed wording and may authorize the normal sequential release command
as one action. Without that instruction, agents remain read-only for Git
publication and may inspect state with commands such as `git status`,
`git diff`, `git log`, and `git show`.

Do not rewrite history or tags. Do not revert user changes unless explicitly
requested.

## Versioning and Release

Version mutation, tagging, publication and push require an explicit maintainer
instruction, but an automated agent may execute them once instructed. A direct
request such as “publish 0.109.15” is sufficient authority for the normal
version, stage, commit, tag, push and package-publication sequence in the named
repository. Do not ask the maintainer to repeat it or run intermediate commands
by hand. Do not infer the same authority from “continue,” “finish,” “ready to
push,” an audit request or a request to prepare a candidate.

The normal complete patch path is `make patch`, followed by
`make release-stage`, `make release-commit`, and `make release-push`; the
one-shot form is `make release-patch`. Minor and major releases use their
corresponding commands.
Before patch validation and version mutation, `make patch` prints the
read-only `make release-cadence` advisory. The advisory reports when the next
release would exceed the soft 12-release minor-line guideline but never blocks
or expands the maintainer's release authority.
The complete Make version targets require a clean source tree, run the same
explicit `make validate` workflow and recheck tracked cleanliness before
changing package versions. They do
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
release date and writes that source commit into one machine-checked
current-status release marker. Lineage prose is descriptive and is not a
versioning or publication authority. Immediately before changing version
files, the bump transaction fetches the current `origin` branch, requires it
to remain an ancestor of the validated local source, and requires the exact
planned release tag to be absent remotely. The release commit may then
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

### Fast non-runtime patch lane

`make patch-fast` is a governed alternative only when the current workspace
version has an exact immutable published tag and every attributable change
after that tag is confined to documentation/governance, the lockfile, or the
release tooling that owns this lane. Runtime, build, package, protocol,
generated, Candid, configuration and product-fixture changes reject before
version mutation. `make release-patch-fast` performs the same eligible gate and
then uses the normal stage, candidate, commit, tag and atomic push path.

The fast lane verifies the immutable tag's structured validation receipt,
requires that receipt or its fast-release chain to retain a complete validated
release ancestor, and checks ancestry, diff hygiene, current-document and
release-matrix semantics. It runs
the release integrity and release-flow checks when tooling changed. A lockfile
change additionally runs the dependency-risk gate, locked offline metadata and
a locked workspace all-targets check. It deliberately skips workspace tests
and PocketIC. The sealed status marker records `gate=fast`; it is not evidence
that `make validate` ran on that patch.

Use the fast lane for a compatible patch-only lock correction, documentation/governance
correction or release-tooling correction whose production source is unchanged.
Any ineligible path, missing receipt, non-ancestor tag or targeted failure
falls back to the complete path; there is no override flag.

When an accepted release batch declares an exact downstream pre-publication
qualification gate, freeze one clean source commit before running that gate.
Build the candidate executable from that commit and run the declared no-effect
preflight before version mutation or publication. A source change invalidates
the downstream result and requires a new candidate commit; release-only
version surfaces do not change the qualified source. Run focused checks while
editing, the one declared production-boundary journey before review, and the
complete `make validate` gate once through the normal maintainer-directed version target
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
release commit/tag pair from committed `HEAD`, refreshes the current `origin`
branch, requires fast-forward ancestry and rejects any conflicting remote tag.
An idempotent retry may observe the exact same annotated tag object. It does
not format, compile, test, validate, or clean. Local
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
For one-shot releases, the maintainer or an explicitly authorized agent may run
`make release-patch`, `make release-patch-fast`, `make release-minor`, or
`make release-major`, which perform those steps in order.
Minor and major release commands do not add an interactive confirmation after
the maintainer has already issued the explicit command.

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
