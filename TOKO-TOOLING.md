# Toko Tooling Engineering Baseline
<!-- Detailed references are pinned online; do not vendor the reference documentation. -->

Toko Tooling owns the common engineering rules for `dragginzgame` repositories.
Read this core, the local `AGENTS.md` and the current handoff when present. Read
linked specialist sections only when the task touches their subject, before the
affected action. Do not recursively read every linked guide at startup or reread
unchanged instructions during the same task. Local overlays should follow the
same approach: mandatory local constraints first, task-specific references after.

Detailed links below identify an immutable reviewed policy revision. Selecting
this file does not install their documents, audit procedures or task runners.
Read those references online at their explicit reviewed revision, never moving
`main` or a sibling checkout. Do not copy their documents into consumers. The
full upstream crate retains them for producer use. If a required reference is
unavailable, report that specific blocker before the affected action; do not
silently substitute a different revision or skip the rule.

## Adoption and local authority

Installing tooling and adopting this policy are separate decisions. Before adding
the adoption statement to a consumer's `AGENTS.md`, classify existing rules:

| Category | What the consumer does |
| --- | --- |
| Binding common rules | Apply authorization boundaries, source integrity, evidence requirements, dependency ownership and explicit prohibitions. These are not optional defaults. |
| Consumer-owned choices | Retain architecture, product limits, local commands, release inputs and validation gates within the common contract. No exception is needed for a choice the baseline delegates. |
| Explicit defaults | Keep a different choice only where the owning rule explicitly allows it; a prohibition is not a default. |
| Conflicts requiring an exception | Record the exact rule, affected repository/workflow, reason and maintainer approval in the local overlay before claiming adoption. Existing practice alone is not approval. |

For example, an existing Python workflow needs a scoped exception to the Python
tooling prohibition. A consumer-specific test command is ordinarily a local
choice; it must still supply the required validation evidence. Installing the
crate or vendoring this file grants no exception and does not activate audits.

- Apply the repository's reviewed snapshot or revision-bound baseline, never a
  moving sibling checkout. Keep snapshot-owned files intact; repair at Toko
  Tooling and adopt a reviewed committed revision through the exporter.
- Consumers own architecture, product limits, local commands and release inputs.
  Toko Tooling owns common engineering practice and fleet audits. Local exceptions to
  common rules need maintainer approval with scope and reason. Preserve explicit
  exclusions and approved product restrictions.
- During adoption, remove redundant or conflicting overlay wording; retain only
  local choices and documented exceptions. Do not turn a general prohibition
  into a new permission prompt when the common rule already authorizes the task.
  See [adoption details](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#adoption-and-local-authority).

## Documentation ownership boundary

Every repository must organise its documentation by topic and make ownership
clear. Keep product-owned documentation under `docs/`; use meaningful directories
as the collection grows rather than accumulating unrelated files at its root.
Use topics such as architecture, design and operations as needed; do not create
empty directories or require a `docs/<product-name>/` wrapper.

- Keep root `TOKO-TOOLING.md` as the small installed baseline and `AGENTS.md`
  as the local overlay. Read shared guides, detailed rules, audit procedures and
  task catalogs through immutable upstream links. Do not vendor, paraphrase into
  parallel policies, or create `docs/toko-tooling/` for those references.
- Install only the required executable entrypoints and their declared runtime
  inputs. A Markdown prompt actually consumed by a selected executable is a
  runtime input, not a reason to install the reference library. Root licence
  and contribution files retain their separate canonical-file contract below.
- The snapshot receipt identifies existing managed copies. During a scoped
  refresh, replace consumer-owned links to them with reviewed commit-pinned URLs
  and supply the complete retained entrypoint list: baseline, needed executables
  and operational inputs. Preview the exporter removing unchanged doc copies
  that are no longer required. Preserve native files and resolve modified
  managed files explicitly; do not hand-edit receipts or bulk-delete directories.
- Organisation of native docs can proceed independently. Retiring existing
  shared copies uses the current exporter, without a new directory migration or
  relocation feature. The upstream repository owns the reference documentation.

This supersedes the 0.39.3–0.39.4 managed-document namespace plan and older linked
instructions to vendor governance guides. It does not trigger unsolicited sibling
refreshes or change the offline verification of installed executable bytes.

## Installed tooling ownership

From 0.42, keep installed executable tooling and its operational inputs under
`vendor/toko-tooling/`, preserving their upstream relative paths inside that
directory. Only `TOKO-TOOLING.md`, `LICENSE.md` and `CONTRIBUTING.md` are installed
at root; `.toko-tooling.snapshot` remains the root ownership receipt. Do not put
consumer edits in the vendor tree. Product scripts, Makefiles, CI workflows,
release policies and `AGENTS.md` remain consumer-owned outside it.

Select only used executable entrypoints. Release and publication are explicit
selections, not companions of the default Make interface. Reference documentation
stays online, and producer regression suites stay upstream; qualify actual local
caller wiring instead of installing generic test scripts. Shared composite actions
may live inside the vendor tree and be called by consumer-owned workflows.

Migrate with the exporter and a complete retained entrypoint list. It must prove
old bytes against their source before retiring them and refuse differing existing
destinations without ownership proof, even when Git considers them clean. Review
preview, update local paths and hook configuration, then qualify the installed
callers. Do not manually move managed files or edit receipts. This replaces the
previous same-path installation rule without relocating reference documentation
or authorizing unsolicited sibling changes.

## Scope and authorization

The [contribution rules](https://github.com/dragginzgame/toko-tooling/blob/e873b124c1b23fa8e25718c6931b596e908c640f/rules/contributions.md) own detailed action authority;
local overlays must not duplicate it or add routine permission gates.

- Prefer direct edits in the normal authorized checkout when a PR is optional.
  Use branches there when needed; coordinate or wait when another session owns
  the inputs. Separate checkouts are explicit exceptions, not the default for
  isolation, PRs or deployment.
  Ordinary fixes include focused checks and pending notes, not staging file
  contents, commits or delivery. They do authorize `git add -N -- <explicit-paths>`
  for newly created, task-owned files when an applicable validation check requires
  Git to know those paths (for example, a new qualification lockfile). This is
  intent-to-add bookkeeping, not content staging: inspect the index, preserve
  existing staged work, explain the reason and proceed without asking again.
  Toko Tooling’s local preparation helper aligns the pending changelog entry
  with the package version in `Cargo.toml` and its lockfile.
- A PR request includes its topic branch, scoped commits, branch push and PR.
  Release/ship/deploy requests include their necessary delivery steps and direct
  integration-branch pushes. An unqualified “deploy”, “ship” or “release” applies
  only to the current repository. All three include both GitHub delivery and
  crates.io publication for its affected publishable groups, without another
  publication confirmation. Sibling delivery requires an explicit request;
  do not scan siblings or ask whether to include them by default. Announce the
  selected plan, then
  proceed under [delegated delivery](https://github.com/dragginzgame/toko-tooling/blob/e873b124c1b23fa8e25718c6931b596e908c640f/rules/contributions.md#delegated-delivery).
- Named scope, no-push/read-only instructions and exclusions take precedence.
  `toko` and `toko-miner` require explicit inclusion; DFINITY contributions remain
  excluded. A dependency blocker alone does not expand delivery scope.
- Preserve unrelated dirty work and reread files before editing. Sibling inspection
  is read-only; source contributions need scoped authority. Coordinate active
  sessions before changing their inputs or preparing an authorized fleet release.
- Do not create new directories directly under `~/projects`; only an explicit
  user exception for that exact directory permits it. Ordinary fix, PR and deploy
  authority does not. Additional checkouts require approval of their location.
  Never leave the only copy of lasting work in `/tmp`, `target/`, caches or a
  disposable patch.
  Follow the [checkout contract](https://github.com/dragginzgame/toko-tooling/blob/e873b124c1b23fa8e25718c6931b596e908c640f/rules/contributions.md#durable-work-and-checkout-location).
- Carry authorization forward without repeated approval. Inspection and readiness
  questions alone do not authorize repair or delivery. Ask only for a genuinely
  missing decision; do not create unsolicited PRs or sibling-release waiting issues.
- Runtime approval restrictions and repository protections remain independent.
  If rejected, report the exact action/reason, preserve work and continue independent
  tasks. Do not switch execution routes to evade the rejection.

## Ownership and simplification

Prefer deleting, reusing or narrowing an existing mechanism before adding one.
Give each policy and invariant one owner; consumers project its decisions.
Keep independent checks at real trust, corruption and recovery boundaries.
Trace callers before combining flows. Change generated code at its generator.
When a starter or style guide supplies a complete Rust example, use the same
source for the example people copy and the checks that exercise it. Short
illustrative snippets can stay inline.
New modes and abstractions need a demonstrated requirement and simpler-alternative
check. See [ownership details](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#ownership-and-simplification)
when changing component boundaries.

## Complete, reviewable changes

Finish the accepted coherent outcome, including directly required callers,
fixtures, docs and cleanup. Split independent outcomes, not compiler fallout or
each proof of the same change. Stop at new independent scope or a release boundary.
Maintain meaningful [changelog notes](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/changelogs.md) once per completed batch;
governance-only edits need no note unless they change a maintained surface or one
is requested. Exclude all legal and licensing content from changelogs, including
adoption notices and notes about this policy or its cleanup. This exclusion overrides
general changelog coverage and fixed-issue-link requirements.
Report outcome, relevant evidence and actionable blockers.
Detailed line metrics and symbol inventories are optional under the
[reporting rules](https://github.com/dragginzgame/toko-tooling/blob/626da7cd9626cc2e152594862a2d82ea82ef51da/rules/change-reporting.md), not a routine completion step.
See [contract-change details](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#complete-reviewable-changes)
when changing interfaces, persistence or installation/recovery behavior.

### Post-push review

After each successful push, review the pushed changes and their directly affected
callers before starting the next increment. Review the delivered commits, not
unrelated working-tree edits, to catch architectural drift, accumulated leftovers
("sediment") and technical debt early. Check for:

- Disagreement between design, implementation, CLI/configuration semantics and
  documentation.
- Duplicate policy, competing owners or parallel workflows.
- Obsolete paths, aliases, compatibility scaffolding, unused dependencies and
  temporary mechanisms left behind.
- New abstractions or infrastructure without a demonstrated consumer need.
- Gaps between claimed behavior and actual qualification evidence.

Prefer deletion, reuse or simplification while preserving safety, persistence
and recovery guarantees. Respect consumer-specific architecture decisions;
distinguish deliberate compatibility obligations and documented limitations from
accidental debt before recommending removal.

Keep the review focused on the pushed scope. It does not automatically trigger
a full audit, repeated test suites, unrelated refactoring, sibling changes or
another push. Reuse applicable evidence under the validation policy below;
repairs and delivery retain their existing authorization boundaries. This review
does not replace required gates or cancel remaining steps of an authorized
release, such as registry publication.

Report concrete findings with code locations and the smallest corrective action.
If nothing actionable is found, state the reviewed scope and that result without
claiming the entire repository is debt-free. Record deferred actionable findings
in the existing owning issue or relevant design notes. Use the normal delivery
response and existing records; do not create a parallel tracker, mandatory report
file or separate review framework.

## Validation and evidence

- Cargo may retrieve missing locked dependencies during normal work and releases.
  Offline mode requires explicit authorization for the invocation or job. Preserve
  the caller’s Cargo settings and report Cargo’s error if a selected package is
  unavailable. Complete dependency preparation before sealing reusable evidence.
  See the [Cargo network policy](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/cargo-dependencies.md#cargo-network-policy).

- Ordinary coding and cleanup end after the relevant focused checks pass. Do not
  turn that handoff into release preparation, fleet adoption or qualification of
  every supported host. Those are separately requested workflows; missing broad
  release evidence alone is not unfinished development work.
- **Finish a coherent edit batch, then run its smallest meaningful checks once.**
  During development, run checks when they answer a specific implementation or
  debugging question. Do not test after every edit or rerun merely because it is
  time to hand off. After a failure or further edit, rerun only affected checks.
- Select checks by changed behavior and directly affected callers, including
  relevant rejection/recovery cases. Reuse passing evidence while the relevant
  source, dependencies, configuration and tools remain unchanged. An unrelated
  documentation edit or new Git commit identity alone does not invalidate code
  checks; source-identity-sensitive checks retain their own contract.
- Name the specific package, test, script or contract being checked. A target
  called `clippy`, `check-wasm` or `test` is not focused merely because it is a
  single command. Local command guides must label aggregate targets and provide
  narrower selections. Do not stack check/build/test commands that prove the same
  compilation, or add feature/MSRV/native matrices without a changed obligation.
  Distinct lint, runtime, platform and product proofs still apply when relevant.
- Inspect nested work before calling a fixture focused. Select its affected
  cases rather than an aggregate that also runs unrelated installer, export or
  release suites. If no narrow entrypoint exists, provide one when practical or
  report that limitation; do not silently broaden an ordinary task into the full
  suite. Name the changed behavior each selected check proves. Keep unchanged
  generic regression matrices with their producer.
- Repository tooling uses one exact root Cargo dependency pin inherited as a
  dev-dependency by every real workspace package. Use ordinary
  `[dev-dependencies]` unless a supported check for that package reports the
  unused tooling marker. Only then use the exact
  `target.'cfg(any())'.dev-dependencies` form for this
  source-bundle marker. It preserves Cargo selection without compiling an unused
  crate into binary tests. Other target selectors, dummy imports and lint
  suppressions are not substitutes. `cargo update -v` reports newer
  releases; after changing the pin and Cargo.lock, run `make toko-tooling-update`.
  Offline admission checks every member, Cargo.lock and installed bytes together.
  See the
  [tooling selection rule](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/cargo-dependencies.md#toko-tooling-during-dependency-updates).
- When adding or widening a release check, record the failure it detects, the
  changes that require it, its existing callers and its observed elapsed cost in
  the owning release plan or review. Use existing timing evidence. If the cost
  is unknown, record it on the next ordinary delivery.
- Give repeated validation one execution owner: selected group scope must reach
  the leaf commands, and aggregate gates must not repeat full lint/setup through
  nested targets. Reuse unchanged release-stage evidence only through the
  qualified contract; see [validation mechanics](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#validation-and-evidence).
- Run Gitleaks once per repository CI run, outside host/package/release-group
  matrices. Do not repeat it during deployment or publication; follow the
  [repository secret-scanning contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#repository-secret-scanning).
- Reserve local full repository/workspace suites and broad gates for an explicitly
  authorized release or explicit request for those checks. Implementation,
  continuation, PR delivery and readiness wording do not authorize them. Focused
  passing evidence suffices for ordinary handoff; an unrun full suite alone is
  not a blocker. Preserve approved local restrictions on focused execution.
- Close ordinary issues after implementation and focused validation; pushing,
  publishing and deployment are separate. State the local/delivery status and
  preserve genuinely issue-specific acceptance under the
  [issue closure policy](https://github.com/dragginzgame/toko-tooling/blob/626da7cd9626cc2e152594862a2d82ea82ef51da/rules/agent-maintenance.md#issue-review-and-repair).
- Hosted integration/native CI runs asynchronously for development, PR delivery,
  separately authorized merges, releases, deployment and registry publication.
  Routine patches proceed after required local checks and publication admission
  pass; pending, queued or absent hosted results alone do not block delivery.
  Report CI separately without polling or holding the turn open for its matrix.
  Wait only for an enforced branch/release protection or a maintainer-selected,
  named qualification requirement scoped to the release or changed contract.
  State that requirement and its source before waiting. A first release, major
  or significant minor may justify such qualification; the version number alone
  does not select it, and it must not become a recurring patch-release gate.
  Known relevant failures still block until resolved or explicitly dispositioned;
  asynchronous CI is not permission to ignore a discovered defect. Preserve local
  release gates, archive/source admission and actual protections. A local target
  named `ci` does not require a successful GitHub Actions run. Do not replace a
  hosted wait with another local copy of its full matrix or invent approval gates.
- Toko Tooling owns its general regression matrix. Consumers check snapshot integrity,
  actual caller wiring and their product/native contracts. Do not replay generic
  producer fixtures by default; retain one only for a named missing consumer proof.
  Use [consumer qualification selection](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/docs/adoption/consuming-snapshots.md#consumer-qualification-selection)
  before changing existing rosters.
- Documentation-only changes need link, consistency and diff checks, not code
  suites. Inspection-only work does not activate tests. Test maintained behavior,
  typed failures and real boundaries, not removed names or private layout.
- Check for active builds before compilation or edits affecting their inputs;
  preserve unrelated artifacts. Bind evidence to actual inputs and commands;
  distinguish review, substitutes, local execution and live observations. Pending,
  cancelled or failed checks are not passes. Report limitations without inventing
  qualification. Read [validation mechanics](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#validation-and-evidence)
  for release ordering, Cargo preparation and external-effect recovery.

### CI setup failure diagnostics

Failed external-tool and setup steps on ephemeral CI runners must leave useful
diagnostics and retrievable evidence before the runner disappears. An exit code,
stderr byte count or runner-local path alone is insufficient.

- Surface a bounded, sanitized excerpt of the underlying failure, with the tool,
  failed phase, original status and exact evidence location. State truncation or
  missing diagnostics explicitly; do not infer a network cause from an exit code.
- Arrange failure-path collection even when setup fails before tests start. Select
  only relevant logs, failure metadata and attempt identifiers; include explicitly
  selected hidden attempt directories. Do not upload whole homes, caches, stores
  or workspaces merely to capture a small diagnostic. Do not follow arbitrary
  symlinks outside the selected evidence boundary.
- Exclude credentials, private keys, tokens, sensitive payloads and environment
  dumps from both printed and uploaded evidence. Redact known sensitive fields
  before publication; CI log masking does not sanitize artifact contents. If raw
  output cannot be safely retained, preserve a sanitized diagnostic and say what
  was omitted. Set explicit size limits and a finite artifact retention period.
- Preserve the original failing step's status through pipes, log printing,
  collection and cleanup. Report collection/upload failures separately; neither
  successful diagnostics nor `continue-on-error` may turn setup failure into a
  successful job. Link the retained artifact in the CI result when available and
  explicitly report when upload failed or required evidence was missing.
- When changing setup or collection wiring, exercise a focused failure path
  before relying on it: an early setup failure with a hidden attempt log must
  produce a useful bounded diagnostic, retain only the intended sanitized files,
  and leave the job failed. Check missing evidence and collector failure handling.
  An offline fixture can prove local collection; claim hosted upload/retrieval
  only when observed. Reuse unchanged evidence rather than adding a recurring
  failure rehearsal to every deployment.

Consumers own their paths, retention values and workflow wiring. Reuse existing
collectors where they fit; this requirement does not mandate a new helper,
framework, committed report or duplicate artifact upload. Better diagnostics do
not by themselves fix the underlying setup failure or authorize blind retries.

## DFINITY dependencies: own the solution

Do not submit DFINITY PRs or depend on future DFINITY work. Use available qualified
dependencies or own the solution under the [maintenance contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#dfinity-dependencies-own-the-solution).

## Independently owned implementations

- Code brought in-house must be independently authored, with documented rights
  to distribute it under our selected licence. Do not copy, adapt, translate or
  vendor third-party implementations into owned source. This applies throughout
  our first-party package graph, including tests, tools, generated source and
  embedded assets; moving code to another owned crate does not evade the rule.
- In particular, do not import or fork unmaintained DFINITY code as a way to
  remove a dependency. Taking over maintenance does not transfer copyright or
  remove upstream licence conditions. Implement the required behavior independently
  or retain a qualified external dependency.
- For a clean-room replacement, derive a behavior specification from published
  interfaces, protocol facts and lawful black-box observations. Keep upstream
  implementation text, patches and expressive tests out of the implementation
  context, including agent prompts. Record specification sources, author/context
  separation and independently written qualification tests. Keep ownership
  agreements and legal assessments in company records outside the repository.
  An agent that has read the upstream implementation must not label its rewrite
  clean-room; use a separate unexposed implementer and reviewed specification.
- Separately licensed external dependencies, including transitive dependencies,
  remain allowed under the dependency rules. Their distribution obligations still
  apply; this is not a claim that the complete binary has only our licence.
  Do not rewrite cryptography merely to remove third-party code or notices.
- Existing imports require an explicit, scoped migration plan or maintainer
  exception before claiming compliance. Preserve applicable licences, notices and
  provenance while that material remains. Do not strip credits, rebrand a fork,
  or claim that a rewrite automatically clears ownership or patent questions.
  This policy does not authorize sibling edits or immediate fleet rewrites.

This section supersedes older linked guidance permitting new maintained imports.

## Canonical licence and contribution policy

- Use only the Toko Studios Network Licence when granting a public licence for
  first-party code. Repositories granting that licence keep one root `LICENSE.md`
  and one root `CONTRIBUTING.md`, byte-identical to
  the canonical files in its reviewed Toko Tooling release. Adopt updates from
  that master deliberately; do not customise the copies or fetch moving `main`
  during builds. Local engineering instructions belong in `AGENTS.md`.
- Confidential proprietary repositories do not adopt a public licence merely
  to use this tooling. Omit first-party licence files and Cargo licence metadata;
  set every private Cargo package to `publish = false` and npm packages to
  `private: true`. Retain the canonical contribution policy. An unpublished
  helper in a licensed repository still inherits that repository's licence;
  publication flags alone do not determine licence coverage or confidentiality.
- Do not retain nested licence files, alternate licences, `licenses/` trees or
  duplicate contribution policies in tracked source, including vendored tooling
  snapshots. Package metadata points to the root licence; a standalone package
  archive may include the canonical licence where its packaging requires it,
  without creating a second maintained source copy.
- Rust workspaces covered by the repository's Toko licence declare `license-file`
  in `[workspace.package]`; every covered member, including unpublished helpers
  and qualification packages, uses
  `[package] license-file.workspace = true`. The product workspace selects
  `"LICENSE.md"`; an independent `qualification/Cargo.toml` selects
  `"../LICENSE.md"`, pointing to the same repository-root file. Resolve the path
  from the owning workspace, never by repeating member-relative paths or adding
  nested licence copies. Use `license-file`, not a separate `license` expression.
- Apart from these canonical files, required notices and factual metadata or
  links to them, do not retain legal material in repository files. This includes
  legal audits, opinions, exposure assessments, drafting history, superseded
  drafts, signed agreements and legal commentary in docs, source comments,
  handoffs or changelogs. Keep necessary company legal records outside the
  repository. Technical audits and factual source-origin records remain allowed.
- Separately licensed external dependencies remain allowed. Preserve their
  required notices and earlier grants; this policy does not relicense them or
  authorise deleting required records or rewriting Git history. If adoption
  would discard an obligation or required notice, stop that migration for review
  rather than silently stripping it or declaring compliance.

This section owns the repository policy and supersedes conflicting older linked
guidance, including permission for new licence-adoption changelog notices.

## Current dependencies and upstream independence

**One rule for every repository:** qualify the selected product and required
qualification graphs, not a fleet-wide dependency generation. An older native
test supplier may coexist with newer product dependencies when their actual
type, protocol and executor boundaries remain valid. A failed optional upgrade
probe blocks that upgrade, not release of an otherwise qualified existing
selection. Preserve real incompatibility refusals and required integration tests.

An unqualified request to adopt or update a named supplier (for example,
"adopt new ic-platform crates") covers that supplier's uses throughout the
current repository: the product workspace, every maintained qualification
workspace and native/dev/build callers. No separate qualification command or
permission is needed. Inventory the relevant manifests and locks, update the
affected declarations and callers, remove superseded dependencies and temporary
migration adapters, and run focused checks for the changed boundaries. Do not
expand this into unrelated dependency upgrades, sibling edits or deployment.

Separate workspaces need not select identical versions when their actual
integration permits independent selections. Report every retained older selection
with its concrete reason and remaining work; updating only the product workspace
while an in-scope qualification migration remains is **partial adoption**.
Explicitly maintained frozen comparison baselines, such as an old-engine
measurement matrix, stay frozen unless the user includes them. Identify that
exception in the result; an ordinary qualification lockfile or stale supplier pin
is not a frozen comparison baseline. Explicit user scope takes precedence.

- Freshness review is separate from pin consistency and passing tests. During
  dependency maintenance, review every direct supplier in the selected scope,
  including native tools, dev/build dependencies and independent qualification
  workspaces. Before freezing a release candidate, review first-party suppliers
  used by that release's product and required qualification graphs. Compare
  manifest requirements and actual locked selections with current stable,
  non-yanked registry releases, including versions outside the declared range.
  Report unavailable registry evidence as incomplete, never current.
- Give each newer supplier a disposition in the existing change or release
  notes: adopt within authorized scope, explicitly defer an optional refresh to
  a named maintenance boundary, or link an approved compatibility hold with its
  blocker and revisit condition. Revisit that disposition at the next applicable
  review; a fixed qualification supplier, passing old tests or a previous deferral
  is not a permanent exemption. Do not claim freshness from an older manifest
  minimum when the lock already selects the current version, or from an
  unpublished sibling checkout. No new report file is required.
- When adding or updating a direct dependency, select its latest stable,
  non-yanked release, including a newer major version where applicable. Verify
  registry availability at selection time and qualify the affected APIs, targets
  and features. An existing version requirement is not a reason to stop at the
  latest release within an old range. Keep compatible requirements and reviewed
  lockfiles; this rule does not require exact pins or prereleases.
- Select general-purpose dependencies independently of DFINITY's transitive
  selections. Do not downgrade or hold back our `num-bigint`, `sha2` or similar
  dependencies merely to match an upstream version or eliminate duplicates.
  Two versions are acceptable when their types need not interoperate and their
  actual size, build and runtime costs are reasonable for the consuming product.
  A whole-lockfile duplicate count does not establish deployed code size.
- Keep genuinely required shared type identities, runtime singletons, protocol
  bytes and security guarantees intact. If duplication may materially affect a
  product budget, measure the selected target/feature graph and resulting
  artifact; for Wasm, use complete uncompressed and code-section sizes. Prefer
  removing unused edges and narrowing features over holding back independent
  dependencies. Optional upstream edges can still constrain resolution; prove
  independence rather than assuming a disabled feature removes the constraint.
- Retaining an older direct selection is exceptional. Record the package and
  versions, concrete blocker (for example a regression, approved compiler/target
  incompatibility, shared public type or measured budget breach), affected scope,
  maintainer approval and condition for revisiting it in the local overlay or
  its linked policy. Upstream age, deduplication alone or unexamined migration
  effort is not sufficient. An upstream-only older transitive selection needs
  no freshness exception for our independent direct dependency; security
  findings still require remediation.

`thiserror` follows the same independent selection rule: use compatible version
requirements and let Cargo select the latest compatible release. PocketIC may
constrain its own graph; do not propagate that constraint into independent
workspaces or add unused dependencies to impose it. Align versions only for a
demonstrated integration requirement. This supersedes the 0.44.7 fleet-wide hold.

An unchanged release selection is not a new dependency adoption. The freshness
review above may defer an optional upgrade to the next dependency-maintenance
batch without a new approval round; this is not a permanent hold or permission to
ignore a security or correctness blocker. For a claimed dependency blocker, name
the selected package, target/features, conflicting identities and introducing
paths, and distinguish failed upgrade experiments from the active release graph.
Compare native qualification and deployed graphs separately; duplicate names in
a whole lockfile alone do not establish a conflict. Do not force a transitive
package outside its supplier's supported range merely to remove a duplicate.
- Reduce DFINITY coupling incrementally at the existing capability owner as
  affected work permits. Preserve reviewed cryptographic implementations,
  external behavior and provenance; do not bypass the Platform boundary or
  rewrite cryptography to obtain a newer dependency.
- Apply this selection rule during authorized dependency maintenance. It does
  not trigger background fleet updates, unrelated upgrades or republishing on
  every upstream release. Routine validation and deployment use their reviewed
  locked inputs; batch optional refreshes at requested maintenance boundaries.

<a id="repository-files-no-symlinks"></a>

## Repository files: self-contained source and packages

Keep source and distributions self-contained; prefer regular files. Read the
[file and symlink contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#repository-files-no-symlinks)
for justified internal links, packaging and filesystem security boundaries.

## CLI applications

When a release adds or changes a runnable CLI workflow, include a short
`### ▶️ Try it` section directly in the root changelog's release summary, after
any breaking-change notice. Show copyable setup/install and invocation commands,
their working directory, and the expected result. Keep one-time setup distinct
from normal use and identify credentials, network or paid/mutating effects.
Wrap long Bash/sh example commands across lines with `\` continuations and
indent continued arguments. Break between arguments, keeping each identifier,
URL and quoted value intact. The backslash must be the final character on its
line, with no trailing whitespace or comment, so the example remains copyable.
Keep short commands on one line.
Do not hide this quick start in detailed notes or substitute a link to them;
independently versioned groups also surface it in the root release index with
the group/version identified. Use actual supported commands, not a mandatory
`make m0-setup` convention. Omit it when no runnable workflow is affected;
do not rewrite historical entries or add a release gate for presentation.

Use Clap for as much Rust CLI behavior as its supported APIs can own, including
typed values, validation, argument relationships, defaults, help and diagnostics.
Check its built-ins and extension points before writing custom CLI mechanics.
Read the [CLI contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#cli-applications) when adding,
changing or reviewing commands.

## Metrics and logging

**`ic-metrics` is only for operational instrumentation of the Canic application
tree, reported through the shared endpoint owned by Canic's metrics adapter.**
It is not a general math, statistics or validation library; do not depend on it
or re-export its helpers for those purposes. Use `ic-logging` for logging.
Read the
[observability contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#metrics-and-logging)
when adding, changing or reviewing metrics and logging.

## IC application encoding

Reusable libraries own domain operations, data contracts and retained-data codecs.
Adapters or applications own public canister endpoint registration, caller guards,
framework lifecycle composition and deployment policy. Canic owns those bindings
for Canic-managed applications; independent services retain their own host adapter.
Wire encoding, bounded types or schema generation alone do not make a dependency
an endpoint-ownership violation. Keep native domain use independent of mounting
an endpoint; do not move domain engines into Canic to remove a codec dependency.

Hashing choices follow the [shared algorithm/purpose inventory](https://github.com/dragginzgame/toko-tooling/blob/626da7cd9626cc2e152594862a2d82ea82ef51da/rules/hashing.md).
Review new algorithms and direct hashing packages explicitly; pure hashing may
use approved direct dependencies without a Platform facade. Preserve existing
hash bytes and domains when changing dependency ownership.

Owned application types must not derive or implement `CandidType`. Use `ic-wire`
for owned RPC; keep unavoidable Candid in dedicated external-protocol boundary
types, not our domain, state or RPC models. CBOR belongs to Platform's typed
external-protocol APIs, including browser and test fixtures. Replace owned CBOR
records and hash inputs with Wire through an explicit format/identity migration.
Read the [encoding contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#ic-application-encoding)
and [RPC budgets](https://github.com/dragginzgame/toko-tooling/blob/626da7cd9626cc2e152594862a2d82ea82ef51da/rules/rpc-budgets.md) for codecs, bindings or protocol changes.

## Wasm optimization

Toko Tooling owns optimization selection and evidence rules; IC Tooling's
`ic-build` owns reusable build/optimizer mechanics, recommended recipes and their
qualification evidence. Consumers select qualified recipes against their product
budgets. Wasm size comparisons report both the complete uncompressed module and
its code-section size. Gzip/compressed size is not a Wasm size metric and must not
appear in those summaries unless transport/compression measurement is explicitly
requested. Read the [Wasm optimization contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#wasm-optimization)
before comparing Wasm sizes or changing deployment profiles, optimizer settings
or performance matrices.

## Gzip encoding

IC Tooling's `ic-host-artifacts::artifact::encode_gzip` owns gzip encoding.
Production code, tests, examples and generators reuse it; consumers retain
compression levels, budgets and artifact admission. Read the
[gzip contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#gzip-encoding) before changing
encoders or compressed-byte identities.

## Pre-1.0 contracts

Keep one current repository-owned contract, without compatibility shims or parallel
versions. Breaking public changes require a minor release; retained data and
external obligations remain real. Read [the hard-cut contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#pre-10-contracts)
before interface or storage changes.

## Release commands

Optimise deployment latency from an authorised release request to the required
GitHub refs being pushed and all selected crates being available in the registry.
Measure preparation, validation, packaging and delivery separately; hosted CI
duration is a separate metric unless delivery actually waits for it. Use existing
attempt evidence, not an extra benchmark gate. Reuse unchanged validation across
release and publication only through a qualified evidence contract, and reuse
admitted prepared tools instead of rebuilding them. Keep fresh source, final
archive and registry admission, required product/native checks and recovery
boundaries. Missing or changed evidence still requires execution or refusal;
speed alone never authorises skipping a gate.

### Consistent validation and delivery flows

Consumers must distinguish ordinary development, changed-tooling qualification,
repository CI, selected release validation, publication admission and failed-phase
recovery. Document their actual entrypoints in existing local guidance. Each
required check has a named guarantee, owner, input scope and execution trigger;
do not create a new report, policy engine or approval gate to record that mapping.

Use the shared Make entrypoints and release adapter protocol, not repository-local
commands with conflicting meanings. Setup installs; tool checks inspect; release
verification validates the selected product; publication admits and uploads; resume
recovers the selected attempt. Product leaf commands may differ. Keep group and
validation declarations in the existing release catalogs, not a second workflow
configuration. Shared mechanics own sequencing, timing, metadata transactions,
proof admission and delivery recovery; adapters supply product decisions and
checks. A missing reusable capability belongs upstream before another local engine
is introduced. Qualify these boundaries with affected consumer fixtures, not a new
mandatory release-time conformance suite.

- Product tests and required native/installed guarantees remain consumer-owned.
  Generic tooling regressions remain upstream. Consumer orchestration fixtures run
  for affected wiring changes or selected CI coverage, not unconditionally for
  every product release or upload merely because they exist.
- Selected release groups must reach leaf package/feature/target commands and
  affected reverse consumers. Independent versions alone do not prove independent
  validation. Common work in a multi-group batch needs one qualified execution
  arrangement or admitted reuse, not an unconditional full gate per group.
- Publication performs fresh final-source, archive and destination admission.
  Do not make full CI the default publication prerequisite or routinely stack
  `publish-check` and `publish` with identical expensive gates. A necessary
  repeated product check must identify changed inputs, a distinct guarantee or
  an explicit limitation of the available evidence admission.
- Use shared execution, metadata, evidence and recovery mechanics with thin local
  policy. Prefer removing unnecessary work before adding reuse machinery. A
  compiler-cache hit is not test evidence; missing or changed proof still requires
  execution or refusal through the qualified contract.
- During authorised delivery reconstruction, old stage counts, scripts and local
  gate wording are research inputs, not compatibility requirements. Replace them
  coherently, retaining product coverage and safety guarantees; retire superseded
  callers and guidance. Preserve historical attempts with their original recovery
  semantics. This is not permission to change an active frozen attempt.

Qualify actual dispatch, input invalidation and failure recovery, then compare
end-to-end and phase timings under stated tool/cache conditions. Separate real
delivery from simulated effects; fewer stage names or an isolated reuse probe do
not demonstrate faster releases. Until replacement is qualified, retain the
existing gate and disclose its fallback cost. This transition rule is not a
permanent exemption from simplifying a demonstrated repeated-work problem.

### Release execution and recovery

Plan deployment as one selected validation pass followed by fresh artifact and
delivery admission. Inspect the actual nested commands first; do not stack full
gates under release, version preparation and publication. Keep optional dependency,
compiler, formatting-policy and tooling upgrades out of an active release unless
requested or needed to resolve its blocker. Qualify changed release-tool wiring
with focused fixtures before expensive product tests, using existing evidence
when unchanged. Use retained stage timings to disclose expensive work and the
expected scope before starting or restarting; unknown cost stays explicit.

After failure, identify the failed phase and invalidated inputs before retrying.
A collector, metadata or upload failure does not by itself invalidate completed
runtime checks. Preserve their original evidence and use qualified stage admission
or post-processing recovery where available; never rewrite their origin or infer
passes from log text. Do not silently restart a full gate as a generic repair.
If recovery is unsupported, explain the concrete missing admission and rerun scope
before proceeding under the existing authority. A rule alone does not implement
recovery or waive a required check. Consumers must qualify such recovery when
improving their release adapters, including evidence-publication failure after
successful execution. Keep failed/missing/changed stages explicit and freshly
check source, final archives and remote/registry state before delivery or retry.

`make release-patch`, `make release-minor` and `make release-major` share the
[release contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#release-commands). Read it and the
local adapter before authorized release work. Normal releases start fresh;
specific-attempt recovery uses `release-resume`. Preserve complete qualification
of the selected release. Independent groups may adopt an explicitly qualified
[group validation plan](https://github.com/dragginzgame/toko-tooling/blob/e873b124c1b23fa8e25718c6931b596e908c640f/docs/releases/release-group-validation.md); until adoption, keep
the existing full gate. Repository-wide CI coverage remains required; authorised
reconstruction may replace its orchestration under the flow rules above.

## Changelogs and artifact preservation

Keep decisions and compact results in Git, not raw experiment archives. Routine
checks need no new evidence report. Experimental evidence defaults to one current
summary and at most two small data files per topic, 100 KiB combined, with a 5 MiB
repository budget; raw output belongs under `target/evidence/`. See the
[retention contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#changelogs-and-artifact-preservation)
for exceptions, historical cleanup and protected release/recovery records.

Use `### 🎯 Qualification` for results such as cursor behavior across an upgrade
or the workload budget at which an operation completes. Use `### 🧪 Testing`
for changes to test tooling or coverage.
Use IcyDB-style emoji category sections in the owning release changelog; read the
[changelog rules](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/changelogs.md) when updating them. Every issue fixed must
have a descriptive link with its number and title in that changelog before handoff, except legal and licensing
changes, which must not be recorded there: root `CHANGELOG.md` for a
workspace without independent release groups, or `releases/<group>/CHANGELOG.md`
for the owning group. A grouped workspace's root remains a navigation index;
it does not need duplicate issue links. Preserve selected
deliverables and required provenance/recovery evidence. Regenerable tool-build
scratch and obsolete cache generations need bounded retention; they are not
permanent evidence. Consumer release artifacts remain protected from implicit
cleanup. See
[artifact preservation](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#changelogs-and-artifact-preservation).
Keep plain `cargo clean` able to remove the owning workspace's entire `target/`
directory. Put workspace build/test scratch and generated fixtures beneath
`target/`; use `.tmp/` or other outside-target scratch only for a documented
technical need. Artifact retention must not narrow explicit cleanup.
Keep `.git` free of build trees and bulk retired-checkout archives. Git-side
tool state needs explicit size/retention limits under the artifact contract;
preserving evidence must not silently preserve every compiler output.

## IC stable-memory governance

Production stable stores use `ic-storage`-issued checked allocation handles. Read the
[memory contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#ic-stable-memory-governance) for
storage allocation, admission, backing access or persistence integrations.

## Rust workspaces and portable tooling

These rules apply to handwritten and generated Rust.

- Start with module comments and inner attributes, then child module declarations
  (including `mod tests;`), then imports. Leave one blank line before the import
  block and one before the following item. Prefer one nested `use` per crate
  for imports with the same scope, visibility and attributes. Preserve imported names.
- Separate item-level macro invocations and functions with a blank line, including
  adjacent methods, tests and test helpers. Keep comments and attributes attached
  to their item. Give each named argument its own line in multi-argument endpoint
  mounts and attributes. Document each mount’s role, each guard’s access policy
  and each endpoint’s caller-visible purpose.
- Compile shared logic on all supported targets. Use a target guard only for an
  operation that requires a target-exclusive API or a different implementation.
  Prefer an existing portable API. For example, isolate IC-specific clock or
  entropy acquisition and keep the callers unconditional.
  The same rule applies to `#[cfg(any(test, target_arch = "wasm32"))]`.
  A helper having only test and canister callers is insufficient reason to
  hide it from ordinary host builds.
- Use ordinary module discovery. The prohibition on `#[path]` includes
  conditional path overrides. Use either `foo.rs` or `foo/mod.rs` for a module.
  When adding a `foo/` directory for children, tests or fixtures, move `foo.rs`
  to `foo/mod.rs` and preserve the public module path. Apply this recursively.
  Keep executable entrypoints free of embedded Rust source injection and source
  patches, including in tests.
- Keep test bodies and test-only helpers in `tests.rs`, `*_tests.rs` or
  `tests/`. Move their references with them, retain the test module’s existing
  conditional attributes and declare fixture-only package roles explicitly.

For examples, test layout and endpoint documentation, see the [Rust hygiene guide](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/docs/principles/rust-code-hygiene.md).

Prefer `#[expect(..., reason = "...")]` over `#[allow(...)]` for Rust lint
suppressions. Repositories must review every `allow` for conversion and retain
one only with a concrete reason why an expectation cannot work. Follow the
[lint suppression rule](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/docs/principles/rust-code-hygiene.md#lint-suppressions).

### Common Rust lint policy

On adoption, reconcile compiler pins and lint configuration in every product,
qualification and other maintained workspace; installing the snapshot alone
does not perform or qualify this migration. Every member inherits its owning
workspace's lint table with `[lints] workspace = true`. A single-package repository
uses `[lints.*]` directly. Cargo cannot combine inherited and local lint tables:
document necessary package exceptions and keep their common checks equivalent.

Use this common minimum in each workspace root:

```toml
[workspace.lints.cargo]
unused_dependencies = "deny"
unused_workspace_dependencies = "deny"
unused_workspace_package_fields = "deny"
missing_lints_inheritance = "deny"
unknown_lints = "deny"

[workspace.lints.rust]
dead_code = "deny"
unused_must_use = "deny"
unfulfilled_lint_expectations = "deny"
unsafe_code = "deny"
unsafe_op_in_unsafe_fn = "deny"
missing_docs = "warn"

[workspace.lints.rustdoc]
broken_intra_doc_links = "deny"
invalid_markdown_table = "deny"

[workspace.lints.clippy]
all = { level = "warn", priority = -2 }
correctness = { level = "deny", priority = -1 }
redundant_clone = "warn"
needless_pass_by_value = "warn"
wildcard_imports = "warn"
missing_panics_doc = "warn"
must_use_candidate = "warn"
too_many_lines = "allow"
```

Use `unsafe_code = "forbid"` for packages requiring no unsafe constructs. Keep
`deny` with narrow reviewed expectations for FFI and unsafe export attributes;
do not weaken existing safe-package prohibitions. Missing-docs debt may retain
an explicit no-growth baseline and scoped exceptions during adoption; do not
invent boilerplate to claim the API is documented. Run the required Rustdoc
checks as well as Clippy: configuring a tool's lints does not execute that tool.

Run Cargo's dependency lints through the existing selected package/target checks;
they do not cover development dependencies or prove every optional/target-specific
use. Retain complementary checks for those gaps. Remove unused declarations and
workspace metadata rather than adding dummy imports or blanket lint allowances.
The missing-inheritance lint can detect an absent member lint table but an explicit
empty table still needs our inheritance review. Cargo, Rust, Clippy and Rustdoc
remain distinct lint namespaces; do not substitute one for another.

For Rust 1.100, qualify warning enforcement on fresh and warm
outputs, consume Cargo artifact records rather than internal build-directory
layouts, and keep dependency cooldowns opt-in. Standard-library simplifications
must preserve resource ownership and show a concrete reduction in complexity
or measured cost; compiler adoption does not require broad API rewrites.

Do not enable the entire `nursery` or `restriction` group. Select justified
individual lints. Existing `pedantic` policy may remain after reviewing its
exceptions; it is not required by the common minimum. Extra checks must protect
a named contract rather than recreate a broad experimental group. Enforce
`future_not_send` only on APIs promising multithreaded-executor compatibility,
not IC-local futures. Keep `too_many_lines` outside blocking gates; an optional
review invocation may enable it. `warn` remains blocking under `-D warnings`.

Review broad ownership/import allowances for relocation to narrow expectations;
keep justified naming and documentation-style choices. Removing a lint also
requires removing its now-unfulfilled expectations across supported targets and
features. Keep strict warning gates for the selected checks. Do not split coherent
operations, add parameter wrappers, widen visibility, discard results, add locks
or change ownership merely to satisfy a lint. Preserve behavior and fix the actual
problem; explain a justified exception where the suggestion would make it worse.

These requirements supersede older linked guidance delegating the entire lint
selection to consumers. Domain-specific stricter checks and reviewed exceptions
remain consumer-owned. Adoption must inspect actual member inheritance, local
attributes and command-line overrides and qualify the affected callers; a green
snapshot check alone is not compiler or lint-policy adoption.

### Product and qualification workspaces

Use one repository-root product workspace, with product packages under `crates/`
and Canic fleet components in `apps/<fleet-name>/`. Meaningful grouping such as
`crates/<domain>/<package-name>/` is supported, including Toko's grouped packages;
packages need not be immediate children. Grouping does not create another
workspace, dependency catalog or release group. Declare actual member paths
explicitly and discover packages through Cargo metadata, not a `crates/*` scan.
Every maintained Cargo package's containing directory must exactly match its
`[package].name`, including hyphens and underscores. This applies under `crates/`,
`qualification/` and `apps/<fleet-name>/`, including private helpers and fixture
packages. Intermediate domain/fleet directories and non-package directories such
as `src/`, `tests/`, `benches/`, `examples/`, `fixtures/`, `docs/` and `scripts/`
keep descriptive names; they need no package prefix. Root `Cargo.toml` and
`qualification/Cargo.toml` are virtual workspace manifests, not extra packages.
Additional workspace roots require a scoped maintainer exception; grouping alone
does not justify them. Generated scratch manifests, vendored upstream sources and
frozen historical evidence are not maintained package locations.
During adoption, rename mismatched package directories rather than package
identities, and update member paths, path dependencies, build/test commands,
deployment references and docs together. Preserve selected dependencies, coverage
and deployed identities. This supersedes older permission for fleet component
directories to use role names different from their Cargo package names; it does
not authorize sibling edits or deployment merely through policy adoption.

Ordinary unit tests, package integration tests and examples stay beside their
owning product code. Separate packages used only for qualification, benchmarks,
fuzzing, test fixtures or test support live under `qualification/`, with
`qualification/Cargo.toml` and its own `Cargo.lock`. Use one package tier:
`qualification/<package-name>/Cargo.toml`. Do not mix direct packages with nested
package trees, add domain or `crates/` wrappers, put a package in the qualification
root, or create an empty qualification workspace. Product grouping under `crates/`
does not extend to qualification. This is the standard whenever separate
qualification packages exist; it does not require first demonstrating a
dependency conflict.

Classify canisters by purpose: standalone product canisters belong under
`crates/`, product fleet components under `apps/`, and separate test, audit or
benchmark canisters under `qualification/`. Do not introduce a parallel top-level
`canisters/` tree. A test fleet's configuration may reference qualification
packages without moving them into the product workspace. Small non-package
fixtures remain beside their owning tests.

Each workspace owns its catalog, lockfile, profiles and member metadata. The
product root excludes `qualification`; qualification consumes product packages,
never the reverse. Reusable shipped testing tools are products, not automatically
qualification packages. Classify by actual responsibility, not publication flags
or names. Keep core tests independent of downstream frameworks.

Release groups contain only product-workspace packages. Qualification packages
are unpublished, belong to no product release group and do not inherit its
version. A group's validation plan explicitly selects the qualification checks
it needs. Required checks remain required even though their packages are outside
the group; no duplicate group or release ledger is created for a test harness.
Changes to product references and qualification locks must be admitted and
updated atomically with affected release metadata, preserving unrelated supplier
selections and private versions. Toko Tooling owns reusable multi-lock mechanics;
consumers own the validation mapping and semantic acceptance.

Apply this standard across repositories through scoped adoption work, preserving
package identities, coverage and release history. Previous general permission to
keep qualification packages in product groups or a nested `crates/` tree does not
exempt a migration. Policy adoption does not itself move sibling files or publish
packages. Verify multi-lock support in the selected release helpers during adoption.

Keep related product crates in that development workspace, with coordinated
versions within explicit release groups where contracts need independent
releases. Independent product groups all use explicit package versions and
`releases/<group>/CHANGELOG.md`; the root has no shared version and its changelog
is an index. Require explicit group selection for releases and publication.
Use `make deps-list` to view every local package's version and release group,
including the separate qualification workspace. Keep root `Cargo.toml` dependency
declarations for actual member usage, not display-only package entries. Remove an
unused alias from its group's dependency roster when removing its declaration;
retain the package in the group's member roster. Private product dependencies
may use path-only entries; qualification owns its own workspace catalog.
Keep unused-dependency linting enabled. When
`[workspace.dependencies]` contains multiple roles, group the used entries
under `# Production`, `# Production and tests`, and `# Tests and tooling`,
in that order. Omit empty groups and sort within each. Production includes
normal and build dependencies across supported targets and features. Use the
mixed heading for entries also declared for tests or tooling. Keep group ownership
in the existing JSON policy under the
[workspace versioning rules](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/rust-workspaces.md#coordinated-versions-and-workspace-boundaries).
Dependency adoption must distinguish required compatibility/security changes from
optional refreshes and check the destination graph before planning downstream
releases. A newer external patch alone does not require republishing consumers.
Defer optional dependency and snapshot refreshes by default, batching them at a
natural release boundary when requested. Adopt Toko Tooling for a needed fix,
capability or explicit refresh; an intact older snapshot alone is not work to do.
Required API, security or compatibility migrations qualify the affected consumer
boundary, rather than automatically updating the whole fleet.
Follow the [adoption planning contract](https://github.com/dragginzgame/toko-tooling/blob/e873b124c1b23fa8e25718c6931b596e908c640f/docs/validation/cargo-graph-checks.md#dependency-adoption-planning).
Use root-owned dependency selections, reviewed toko tooling and explicit setup.
Rust 1.100.0 is the approved minimum and pinned development/release compiler;
this supersedes older compiler versions in the revision-bound references below.
Follow the [compiler support policy](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/cargo-dependencies.md#minimum-supported-rust-version-msrv)
when adopting it and consolidating duplicate compiler checks. Preserve product
host requirements. Select canonical identities
from `ic-platform-types`, IC0/execution from `ic-platform-runtime`, and crypto or
external IC adapters from `ic-platform`, following the
[Platform ownership rules](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/cargo-dependencies.md#platform-package-owners).
External IC dependencies remain behind that reviewed boundary; Types alone may
add `ic_principal` for explicit registry conversions. Platform's unpublished
`qualification/bls-probe` comparison oracle may use unaliased crates.io
`ic-verify-bls-signature` at `^0.6.0`, default features disabled and only `alloc`,
as a nonoptional dev dependency under `cfg(not(target_family = "wasm"))`.
This exception requires the `bls-wrapper-probe` identity and Platform repository
metadata; it grants no normal/build, production or Wasm dependency permission.
Self-authentication does
not enable those conversions. npm SDK Core uses
`@tokostudios/ic-platform`. Keep reusable cores independently upgradable;
adapters own their matching integration stacks and must not impose consumer
framework dependencies on core development. Follow the
[core and integration graph boundary](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/rust-workspaces.md#core-and-integration-graph-boundary).
Do not add Python tooling. Read the applicable
[workspace/tooling contracts](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#rust-workspaces-and-portable-tooling)
for manifests, dependencies, hooks, installers or shared snapshots, and the
[Rust hygiene guide](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/docs/principles/rust-code-hygiene.md) for Rust implementation.

## Host support

All packages support their applicable workflows on macOS. Read the [host contract](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#host-support)
when changing platform behavior or CI. Routine CI cancels superseded runs while
preserving required hosts on the retained run.

## Feedback and handoff

Every issue reference must name its owning repository and issue number, even
for the current repository: `Canic #14`, `WasmStore #1`, `BlobStorage #3`.
The display name may omit the `IC` prefix, but must identify the repository
containing the issue, not the package or subject being discussed. For example,
a Platform migration tracked in Canic is `Canic #14 — Platform migration`, not
`Platform #14`. Use that repository-qualified label in links and link directly
to the issue when available. Never use a bare `#14` or rely on a heading to
supply its repository. Apply this in conversation, progress updates, final
reports, handoffs, documentation, changelogs and newly written issue/PR text.
Use `owner/repository#number` when a shortened name would be ambiguous or GitHub
closing syntax requires it. Verify an uncertain owner rather than guessing.

Shared dependency defects and advisories have one remediation issue at the
supplying owner. Do not create a waiting issue in every consumer; separate
consumer issues need distinct actionable work. Apply the
[dependency finding rule](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#shared-dependency-findings).

Use owning GitHub issues for follow-up; no parallel local trackers. CI/issue
maintenance and scheduled work run only when activated. Read [feedback details](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#feedback-and-handoff)
when reporting findings across repositories or maintaining issues.
Use the [shared audit methods](https://github.com/dragginzgame/toko-tooling/blob/626da7cd9626cc2e152594862a2d82ea82ef51da/audits/README.md) for selected code/structural
audits; adoption activates neither an audit schedule nor repair authority.

### Repository handoff files

Keep the current handoff a replacement summary of accepted scope, active blockers,
next actions and latest relevant evidence. Aim for 50–100 lines; trim above 150
on the next update while preserving unresolved safety/recovery facts. Link
canonical evidence instead of copying logs, test inventories or final reports.
Read [handoff retention details](https://github.com/dragginzgame/toko-tooling/blob/a716b501274678a8ebf163ef576bbb549e974acd/rules/engineering-details.md#repository-handoff-files)
when trimming historical content. Do not create a handoff file just for ceremony.
