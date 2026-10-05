# Changelog rules

These rules are part of the mandatory [engineering baseline](../DRAGGINZGAME.md)
for repositories with changelogs. Read them before editing release notes.
Consumer overlays own audience, presentation and package-specific views within
this contract; the [release contract](../docs/releases.md) owns release execution.

## Content and writing

- Root `CHANGELOG.md` is the concise release ledger. Lead with the outcome and
  practical effect for the repository's users, operators or developers. Use
  plain language and consolidate related changes into short bullets or summaries.
- Update the current draft by default when a meaningful behavior or maintained
  tooling batch is complete. Cover the coherent outcome, including relevant
  public APIs, commands, output fields, diagnostics and operational changes.
  Another compatible implementation slice extends that draft.
- Omit formatting-only churn, incidental internal renames, routine test runs
  and governance-only edits unless they change a maintained surface or the
  maintainer explicitly requests a release note. Meaningful coverage or test
  tooling changes may be noted; a `Testing` section describes those changes,
  rather than listing validation commands.
- Use only relevant sections, such as `Added`, `Changed`, `Fixed`, `Removed`
  and `Breaking`. Small entries may use a brief summary without subsections.
  Preserve the established heading style; emojis and descriptive release titles
  are consumer presentation choices.
- Keep implementation inventories, internal paths, test names, footprint tables
  and long examples out of root summaries unless needed to explain a public
  contract or required action. Move useful detail to the linked minor-line notes.
  Include a small representative example when it makes a newly supported
  command, query or configuration clearer; do not invent examples for cleanup.
- Describe supported behavior and measured effects accurately. Do not present
  planned work, a passing substitute test or an expected speed-up as shipped or
  qualified behavior. Routine validation results and readiness belong in the
  handoff or their existing evidence owner, rather than repeated release prose.

## Issue links and compatibility

- When a change fixes a GitHub issue, its changelog entry must include a clickable
  link to that issue, for example `[#123](https://github.com/OWNER/REPO/issues/123)`.
  Link every resolved issue relevant to the entry. When both root and detailed
  notes describe the fix, include the issue links in both views. Bare issue
  numbers or local tracking labels do not replace links; never invent issue URLs.
- Mark breaking public API or semantic changes explicitly in the root summary.
  State the affected contract and required consumer action, with fuller detail
  in the minor-line notes when needed. Include relevant API, CLI, response/error,
  wire/storage and execution-semantic changes; do not bury them under cleanup.
- State migration, regeneration, reinstall or reset implications accurately.
  A pre-1.0 hard cut does not waive retained-data or external-consumer obligations.
  Breaking public API or semantic changes require a minor release before 1.0.
  Report a conflicting selected patch target rather than silently renumbering it.

## Drafts and release history

- Keep the latest release or one current draft at the top. Do not use `Unreleased`
  or a separate release-notes queue. If the next version is undecided, use one
  undated `## [Draft]` without a patch number.
- An explicitly selected, unpublished version may label that same draft. Keep
  it current until its matching release tag exists or publication is reported;
  then preserve it and collect subsequent authorized work in a new undated Draft.
  Do not allocate a patch version for every slice, infer the next version or
  change manifests, lockfiles or release defaults while preparing notes.
- Resolve the version and date during explicitly authorized release preparation.
  Reconcile the complete candidate since the previous release, including work
  missing from the draft, and avoid duplicate entries for the same target.
  A changelog request alone does not authorize version changes, commits, tags,
  pushes or publication. Commits remain maintainer-owned.
- Changelog presentation, section position, draft labels and an undecided version
  must not block deployment. Final labeling and unambiguous release selection
  belong in release preparation. Report and repair missing notes when practical;
  explanatory prose is not an executable release or deployment guard.
- Preserve historical entries, dates, order and content. Do not silently rewrite
  published summaries to reflect today's implementation, delete history or
  reclassify imported undated releases as drafts. Historical notes are evidence,
  rather than authority for current supported contracts.
- Archive or correct historical notes only when explicitly authorized. Archiving
  retains all historical detail and leaves the version, date, concise summary and
  clickable detail link in the root ledger. Do not merge different minor lines
  into one detail file or discard content to shorten the root.
- Draft/date fields record note preparation; tags and registries establish their
  own publication state. Changelog text does not prove validation, publication,
  consumer adoption or deployment.

## Detailed notes when needed

- A small repository may keep its notes entirely in root `CHANGELOG.md`. Add
  detailed notes when release volume or explanation makes the root hard to scan,
  or retain them when the repository already uses that structure.
- Use one `docs/changelog/<major>.<minor>.md` file for all patches in a minor
  line, rather than one file per patch. Do not invent a version or minor line
  to name a detail file; keep an undecided draft in the root until its line is known.
- A root minor-line index uses one concise summary per patch, normally one short
  bullet. Product-facing ledgers may retain linked release titles and brief prose.
  Keep minor lines and their patch summaries newest first; detailed patch sections
  follow the same order. Preserve existing historical ordering when adopting
  these rules rather than reorganizing published notes without authorization.
- Every patch in a minor-line index has a matching detail section. Keep root and
  detailed notes aligned on scope, patch identity, issue links and compatibility.
  Where both views date a patch, their dates must agree. Small detail sections
  may remain short. Extend both views for the same draft.
- Root entries must link to their detail files with clickable Markdown links.
  Resolve relative targets from the containing file: a root ledger uses, for
  example, `[Detailed notes](docs/changelog/0.33.md)`.
- Detailed notes explain meaningful implementation changes, operator-facing
  command/output surfaces, changed invariants and coverage, and required consumer
  actions. Link to evidence and operational procedures at their existing owners
  instead of copying command logs, release receipts or issue status.

Before handoff, check that the current notes cover the completed batch, issue and
detail links resolve, both views agree when present, and historical content is
preserved. This review does not require running a broad validation or release gate.
