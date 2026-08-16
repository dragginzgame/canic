# Delivery Cadence Governance

This document defines how Canic groups development work into reviewable and
publishable release batches. Command, Git and version mutation rules remain
owned by [CI and deployment governance](ci-deployment.md). Changelog shape
remains owned by [changelog governance](changelog.md).

## Purpose

Canic needs both small implementation steps and meaningful releases. Treating
every focused test, compile repair or documentation correction as a new patch
release creates excessive validation, publishing and downstream update work.
Accumulating an entire minor in one worktree is equally difficult to review.

The release cadence is therefore outcome-based rather than time-based.

## Terms

- An **implementation slice** is the smallest coherent code or documentation
  step that can receive focused validation. It has no version identity.
- A **release batch** groups compatible slices into one substantive,
  end-to-end outcome suitable for review and publication.
- An **open patch draft** is the changelog description of the current release
  batch. It remains open until the maintainer chooses to publish and tag it.
- A **published patch** is an immutable versioned release created by the
  human-owned release flow.

A slice, commit, agent turn, changelog edit and published patch are not
interchangeable concepts.

## Minor-Line Planning Target

Before implementation begins, the design or status tracker should group one
minor-version line into roughly **6–10 substantive release batches**. Multiple
design documents assigned to the same minor share that range.

Six to ten is a planning target, not a quota. A smaller line may need fewer;
an unusually broad safety or architecture line may need more. When the plan
falls outside the range, the tracker must explain why the dependency and
review boundaries are genuinely different. It must not manufacture
micro-releases or unrelated mega-batches merely to hit a number.

After ten published releases in one minor, the tracker must explicitly
reassess whether remaining work belongs in:

- the current open batch;
- a small number of consolidated closeout batches; or
- a separately authorized later minor.

The advisory release-cadence tool surfaces this threshold but does not block a
maintainer's release decision.

## Release-Batch Contract

Each planned release batch names:

- one bounded outcome and its canonical owner;
- the directly affected delivery layers or operator surfaces;
- positive verification and the nearest invalid path;
- interruption and exact-retry evidence when external effects are involved;
- direct diagnostics, fixtures, generated surfaces and active documentation;
  and
- required cleanup or hard-cut fallout caused by the outcome.

Those items belong in the same batch. They do not become independent patch
releases merely because they require separate implementation slices or agent
turns.

Routine compiler fallout, Clippy fixes, fixture updates, status corrections
and changelog consolidation stay in the current batch. They justify a separate
release only when they expose an independent maintained behavior, security
fix, authority correction or operational emergency.

A release batch must remain reviewable. If work reveals another independent
authority, state machine or user-facing outcome, update the tracker and place
that work in another batch rather than hiding it inside the current one.

## Continuation and Handoff

Generic continuation such as `continue`, `keep going` or `next` continues the
current planned release batch while any of its direct acceptance criteria
remain incomplete. It does not imply that the previous implementation slice
should be published first.

An agent should report a batch as ready to push only when:

1. its complete bounded outcome is implemented;
2. its direct positive, negative and recovery evidence passes;
3. its required propagation and cleanup are complete;
4. status and the open changelog draft describe the whole batch; and
5. only maintainer-owned broad release validation remains.

If those conditions are not met, the handoff should say what remains in the
open batch instead of recommending another patch release.

The maintainer may still request an early emergency or checkpoint release.
That explicit decision overrides the normal batching target without changing
the scope of later work.

## Design and Status Trackers

Implementation slices may be finer-grained than release batches. Every active
design/status tracker must nevertheless include a release-batch plan that maps
its slices and completion evidence into the minor-line target.

### Design Directory Shape

A top-level numbered design directory is part of the maintained release
lineage: the current/recent baseline or an accepted, scheduled future line.
An unscheduled future concept never belongs there. Each numbered directory
normally contains exactly:

- the versioned normative design; and
- `status.md`, the compact implementation and release-batch tracker.

Add another top-level file only when it is a genuinely independent normative
authority or durable closeout artifact that would make either primary document
less clear. The status tracker must name that file's purpose. A source family,
implementation slice, test run or conversation turn is not a reason to create
another design file.

Large source inventories, bounded supporting investigations and temporary
traceability ledgers that remain needed for an active batch belong under
`docs/audits/working/<line>-<topic>/`, linked from the compact design or
status. They are working evidence, not extensions of the design. Consolidate
or delete them when the owning batch no longer needs their exact source
mappings; do not retain one permanent Markdown file per slice or source
family.

A working or retained supporting-evidence topic may contain at most eight
files by default. A larger pre-existing bundle requires a named exception in
the document-semantics guard and may shrink but must not grow. Raising a bound
requires explicit maintainer approval and an explanation in the owning status;
creating one file per slice, source module or agent turn is never sufficient
justification.

When a line closes or moves to `docs/design/archive/`, relocate closeout
reports, inventories, review notes, amendments and other supporting release
records to `docs/audits/release-lines/supporting/<line>-<topic>/`. Preserve
their content and repair references, but keep the archived design directory at
the same compact design/status boundary. Historical collection directories
that contain several independent design/status pairs may retain a single index.

`docs/design/ideas/` contains unnumbered deferred concepts, one descriptive
topic directory per idea. A topic normally contains only `design.md`; it may
also retain one compact `status.md` or `exploration.md` when that file preserves
material prior review context. Idea text confers no schedule or implementation
authority. Promotion requires a concrete need and owner, accepted release
position, complete batch plan, explicit maintainer approval and a deliberate
index/guard update.

The historical post-46 and optional-ideas collections remain bounded by their
approved directory and file ceilings. Adding another collection or topic
requires explicit maintainer approval and a deliberate guard update; a new
implementation slice must use its owning numbered design/status instead.

Use stable batch labels such as `B1`, `B2` and `B3` while planning. Do not
invent patch numbers. A patch number belongs to the maintainer-owned release
decision, except where an already-open draft is being extended.

Each tracker row records:

| Field | Required content |
| --- | --- |
| Outcome | One end-to-end capability, correction or qualification boundary |
| Owner | Canonical subsystem or authority |
| Included evidence | Direct success, rejection, retry and propagation work |
| Validation | Focused checks needed before handoff |
| Status | Pending, active, ready or published |

## Existing Over-Target Lines

Historical releases are immutable and are not renumbered or collapsed. An
active minor that already exceeds the target adopts this policy prospectively:
keep the current patch draft open for its complete outcome, consolidate the
remaining tracker into a small number of honest batches, and avoid further
one-proof or one-fallout releases.

## Tooling

Run:

```text
make release-cadence
```

The command reports the current minor line, its published release count, the
normal planning range and the ordinal of the next release. `make patch` runs
the same advisory before validation and version mutation. The advisory never
changes files, tags or release authority.
