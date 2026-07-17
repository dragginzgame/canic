# Canic Changelog Governance

This document defines the authoritative rules for maintaining
`CHANGELOG.md`, minor-line changelog archives, and unreleased development
notes.

These rules are intended to be followed by automated agents.

---

# 1. Purpose

The root `CHANGELOG.md` file is the canonical release ledger for Canic.

It records high-level architectural and behavioral changes per release.

It must remain concise and structured.

It is maintained by default when a meaningful code or behavior batch is
complete. A versioned entry may remain an open draft and accumulate compatible
work until that version is tagged.

Detailed change breakdowns belong in:

`docs/changelog/<major>.<minor>.md`

For example: [docs/changelog/0.33.md](../changelog/0.33.md)

Every versioned patch draft and finalized patch entry must have both views:

- one concise patch bullet in the root `CHANGELOG.md`;
- one detailed patch section in the matching
  `docs/changelog/<major>.<minor>.md` file.

The root ledger is for scanning release history. The detailed file is where
agents record the implementation breakdown, tests, notes, CLI surfaces, JSON
shape changes, and operational nuance.

---

# 2. File Structure

## 2.1 Canonical Ledger

- Root: `CHANGELOG.md`
- Must contain:
  - Version headers
  - Date
  - High-level summary sections
  - Links to detailed notes
- Root minor-line summary entries must use exactly one concise bullet per patch version.
- Within each minor-line section, patch entries must be ordered chronologically newest first (`x.y.9` before `x.y.8` before `x.y.7`).
- Each root minor-line section must link to its detailed
  `docs/changelog/<major>.<minor>.md` file when that file exists.
- A completed meaningful code or behavior batch must update the root changelog
  by default. Small incomplete slices may wait until they form a coherent
  batch, and governance-only, formatting-only, or routine test-only work is
  omitted unless it changes a maintained surface.

## 2.2 Detailed Minor Notes

- Location: `docs/changelog/<major>.<minor>.md`
- Contains:
  - Deep architectural explanation
  - Internal module movements
  - Test matrix expansions
  - Execution-shape changes
  - Validation and invariant notes
  - Migration commentary
- This is the preferred place for code examples, LoC snapshots, and fenced blocks (` ``` `) that improve scanability.
- Detailed minor notes may be substantially more verbose than root changelog entries.
- Every patch listed in a root minor-line section must have a matching
  versioned section in the detailed minor notes. If the patch is intentionally
  small, the detailed section can be short, but it must still exist.
- When a patch adds or changes a CLI query, command result, JSON shape, or
  visible output column, record that operator-facing surface under that patch
  section in the detailed minor notes. Keep implementation cleanup separate from
  those CLI surface bullets so readers can quickly find new automation inputs
  and outputs.

All patch releases in the same minor line share one detailed notes file.
Example: `0.33.0`, `0.33.1`, and `0.33.2` all map to [docs/changelog/0.33.md](../changelog/0.33.md).

Within a detailed minor notes file, patch sections must also be ordered chronologically newest first.

The root changelog must link to the detailed file when present.

## 2.3 Unreleased Notes

Only the root `CHANGELOG.md` may contain a top-level `## Unreleased` section.
Detailed minor notes must start with versioned patch sections and must not
carry their own `Unreleased` sections.
This root-only rule is guarded by `cargo test -p canic --test changelog_governance`.

Keep root `Unreleased` as a short holding area for incomplete work, work whose
release target is intentionally undecided, or notes that a maintainer asks not
to place in the open patch draft yet. It is not the default destination for a
completed meaningful batch.

Rules:

- Do not create a patch-numbered section for every small code slice. Complete a
  coherent batch first.
- Group related incomplete slices into coherent bullets under `Unreleased`.
- Keep notes concise enough that they can be collapsed into a release entry
  without rewriting from scratch.
- Do not use `Unreleased` for formatting-only churn, transient debugging notes,
  or validation command inventories unless the validation surface itself
  changed.
- When a coherent batch completes, move its relevant `Unreleased` content into
  the open detailed patch section, delete or clear the consumed root bullets,
  and create or update the single concise root patch bullet.

Terminology:

- Code slice: a small focused change suitable for review and targeted
  validation. It does not imply a version.
- Unreleased batch: one or more related incomplete or deliberately unassigned
  slices collected before they enter an open patch draft.
- Open patch draft: the newest versioned changelog entry with no matching
  immutable `v<version>` tag. Compatible completed work is added to this entry
  until the tag exists.
- Published patch release: a versioned release prepared by the human-owned
  release flow.

---

# 3. Version Entry Rules (Root CHANGELOG.md)

Each version entry must follow:

## [<version>] – <YYYY-MM-DD> – <Short Title>

### Added
- High-level new capabilities

### Changed
- Architectural or behavioral changes

### Removed
- Removed APIs, contracts, or behaviors

Rules:

1. Keep the existing changelog structure and header format.
2. Smaller entries may omit the title segment and use:
   `## [<version>] - <YYYY-MM-DD>`.
3. Changelog subsections are optional; include only sections relevant to that release.
4. If an entry reaches 4 lines or more of changelog content, split it into subsection headers.
5. For small cleanup releases, prefer no subsection headers; use a short plain-language summary with concise bullets.
6. For structural cleanup/audit passes, use subsection headers and include an explicit `Audit` subsection with footprint stats.
7. If a section like `Changed` becomes large, split into topic-based subheaders (for example `Changed - Aggregate Execution`, `Changed - Structure`).
8. Do not include file paths.
9. Do not include test names.
10. Do not include internal refactor noise.
11. Do not exceed ~15 bullets total in the root entry.
12. If a section exceeds ~4 lines of explanation, move detail to `docs/changelog/<major>.<minor>.md`.
13. For a root minor-line entry (`<major>.<minor>.x`), use exactly one bullet per patch version listed in that minor line.
14. Each root minor-line patch bullet must be a high-level summary sentence, not an exhaustive implementation list.
15. If a patch bullet starts becoming a multi-clause internal inventory, shorten it and move detail to `docs/changelog/<major>.<minor>.md`.
16. Root minor-line patch bullets must be listed in descending patch order, with the newest patch first and the oldest patch last.
17. Create or update a versioned root entry when a coherent meaningful batch
    completes. The entry remains an open draft until its matching tag exists.
18. Never create a second patch draft merely because another compatible slice
    completed; extend the existing untagged draft instead.

## 3.1 Section Header Emoji Mapping

When section headers are used in `CHANGELOG.md` or `docs/changelog/*.md`,
emoji-prefixed headers are the default and must use this fixed mapping:

- `Added=➕`
- `Changed=🔧`
- `Fixed=🩹`
- `Removed=🗑️`
- `Breaking=⚠️`
- `Migration Notes=🧭`
- `Summary=📝`
- `Cleanup=🧹`
- `Audit=📊`
- `Testing=🧪`
- `Governance=🥾`
- `Documentation=📚`

Keep emoji usage consistent across releases.

## 3.2 Link Formatting

For root changelog references to detailed notes, links must be clickable Markdown links.

Use this source text in the root `CHANGELOG.md`:

```markdown
[docs/changelog/0.33.md](docs/changelog/0.33.md)
```

Do not use plain backticked path text for detailed-breakdown links.

---

# 4. Automation Rules for Agents

During ordinary development:

1. Keep code slices focused and reviewable.
2. Let small compatible slices accumulate until they form a coherent batch;
   do not create a patch entry for each mechanical edit.
3. When a meaningful code or behavior batch is complete, update its changelog
   by default without waiting for a separate maintainer request.
4. Treat the newest versioned entry without a matching immutable `v<version>`
   tag as the open patch draft. Update both its single root bullet and detailed
   section when more compatible work is added.
5. If no open draft exists, use the current workspace version when it is not
   tagged; otherwise open the next patch version in the active minor line.
   An explicitly requested minor, major, or exact target takes precedence.
6. Do not advance to another patch number while an open draft exists. The tag,
   not the number of completed slices or commits, closes the draft.
7. Use root `Unreleased` only for incomplete or deliberately unassigned work.
   Do not add `## Unreleased` to detailed minor notes.
8. Do not change Cargo versions, release-script defaults, install URLs, or lock
   file package versions while maintaining changelog drafts.
9. If the work is changelog-policy/governance-only, do not add or update
   release notes in either root or detailed changelog files unless explicitly
   requested as a release artifact.

When preparing a release:

1. Identify all changes since last version tag.
2. Group changes into:
   - Added
   - Changed
   - Removed
3. Extract only architectural or behavioral changes.
4. Ignore:
   - Formatting-only changes
   - Test-only changes (unless behaviorally significant)
   - Internal renames without surface impact
5. Confirm the open patch draft covers every relevant completed change and
   move any remaining in-scope `Unreleased` notes into it.
6. Update its concise root summary to describe the complete release batch.
7. Generate or update `docs/changelog/<major>.<minor>.md` with full detail.
8. Insert clickable Markdown link from root file to detailed file.
9. Confirm the target patch appears in both files before declaring the
   changelog ready.
10. Use the existing open draft version unless the maintainer explicitly
    changes the release target.
11. Do not create a new version header if the newest entry already exists for the target version.
12. If a change set is changelog-policy/governance-only, do not add or update release notes in `CHANGELOG.md` or `docs/changelog/<major>.<minor>.md`.
13. When updating an existing minor line, keep the patch bullet/section in chronological order. In normal patch releases this means adding the new patch at the beginning of the existing minor-line list.

Agents must never:

- Delete historical version entries.
- Rewrite previous release summaries.
- Reorder version history.
- Collapse multiple minor lines into one detailed file.
- Add release notes for changelog-policy/governance-only edits (for example updates to `docs/governance/changelog.md`, `AGENTS.md`, or changelog formatting policy), unless explicitly requested as a documented release artifact.
- Open a second patch draft while the newest versioned entry remains untagged.
- Treat a changelog draft version as authorization to change package or release
  version files.

---

# 5. Breaking Changes

If a change alters:

- Public API
- Response types
- Cursor format
- Execution semantics
- Error taxonomy
- Persistence format

The root entry must:

- Include a clear note under "Changed" or "Removed".
- Mention migration implications.
- Be explicitly marked as potentially breaking.

---

# 6. Archival Policy

Older detailed entries may be moved from root CHANGELOG.md
into docs/changelog/<major>.<minor>.md if the root file grows large.

When archiving:

- Leave version header in root.
- Replace detailed content with a summary.
- Insert link to detailed file.

Historical content must never be discarded.

---

# 7. SemVer Enforcement

- MAJOR: incompatible surface or behavioral changes.
- MINOR: additive capability.
- PATCH: internal fixes without surface change.

Agents must not bump version without checking semantic impact.
During ordinary development, a versioned changelog draft may target the
upcoming release even while `Cargo.toml` still has the previous published
version. This documentation target does not authorize a package-version bump.
If the requested semantic boundary conflicts with an existing open draft,
report the conflict instead of silently allocating another release.

---

# 8. Writing Style, Verbosity, and Jargon

Use plain, industry-friendly language.

Required writing style:

- Lead with outcome and user impact.
- Keep wording concise and junior-friendly.
- Avoid jargon unless the technical term materially improves clarity.
- Keep entries intentionally brief and non-technical by default.
- Include deep internal names only when required for migration or debugging.
- Prefer a small number of consolidated bullets over long fragmented lists.
- Explain why a change matters, not only what changed.

Bullet and detail rules:

- Prefer short bullets (1-2 sentences), with inline code formatting for API/type names when relevant.
- Bullets do not need to be single-line if additional sentence context is needed.
- In root minor-line summaries, prefer one short sentence per patch bullet; avoid long multi-clause bullets that enumerate every internal change.
- Avoid deep implementation detail (module paths, helper names, routing internals) unless required for migration/debugging.
- Root minor-line patch bullets should stay concise and readable. Detailed
  changelog prose may wrap naturally where it is clearest.
- In root `CHANGELOG.md`, avoid code examples/LoC dumps unless strictly necessary.
- Prefer placing code examples, LoC snapshots, and fenced blocks in `docs/changelog/<major>.<minor>.md`.
- Inline fenced examples are optional, not mandatory.
- In root `CHANGELOG.md`, include at most one inline fenced example per minor version (`0.x.x` line), and only when it materially improves clarity.
- In `docs/changelog/<major>.<minor>.md`, include at most one inline fenced example per patch entry (`## 0.x.y`), and only when it materially improves clarity.
- Use inline fenced examples only for meaningful code, config, or flow snapshots that explain behavior better than prose; if no good example exists, skip it.

Testing section rules:

- Do not add a `Testing` section for routine validation runs (`make check`, `make test`, `cargo test`).
- Add `Testing` only when the release adds or changes tests, coverage, or test tooling.
- For `Unreleased` notes, omit routine validation command lists. Validation
  commands belong in agent handoff/final responses, not in changelog notes,
  unless the validation tooling or coverage changed.

---

# 9. Release Flow

For each release:

1. Keep the open versioned changelog draft current as meaningful batches
   complete.
2. Before release, compare the draft with all changes since the previous tag
   and consume any remaining in-scope `Unreleased` notes.
3. Confirm the root bullet and detailed minor-line section agree.
4. The maintainer commits the completed implementation and changelog batch.
5. The maintainer runs the governed version target, which performs its gates
   before updating package and release version files.
6. The maintainer reviews, stages, commits, tags, and pushes through the
   governed release targets.

Order must be preserved.
The normal human patch flow is `make patch`, review, `make release-stage`,
`make release-commit`, and `make release-push`; `make release-patch` is the
one-shot equivalent from a clean release-ready commit.

---

# 10. Ownership

Changelog governance is architectural, not cosmetic.

It documents system evolution and must reflect real semantic shifts.

It is part of Canic's correctness discipline.
