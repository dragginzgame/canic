# 0.92 Audit-System Inventory Method

Audit method ID: `CANIC-092-AUDIT-INVENTORY`

Method version: `1`

## Purpose

Produce a reproducible, read-only inventory of the audit authorities,
executable audit helpers, retained evidence, and coverage claims present at
the 0.92 release anchor. This method identifies conformance gaps and candidate
findings but does not assign final definition dispositions.

## Trigger And Owner

- Trigger: once at 0.92 Phase A, and again only if the inventory method itself
  is corrected before the method-freeze snapshot.
- Canonical owner: the accepted 0.92 design and its implementation tracker.
- Reviewer: the maintainer accepting the 0.92 line or a named delegate.

## Snapshot And Scope

The inventory is anchored to full commit
`5f7a89f9b966ebf2755d5630ddcba0cdf968ebb1` (`v0.91.6`). It reads:

- `AGENTS.md` and the governance files it makes normative;
- `docs/audits/{README.md,AUDIT-HOWTO.md,META-AUDIT.md}`;
- `docs/audits/recurring/`;
- `docs/audits/modular/`;
- `docs/audits/release-lines/` and its index;
- `docs/audits/reports/` structure, indexes, and compact retention counts;
- audit-labelled operational definitions in `docs/operations/`;
- executable helpers named by active definitions or audit-labelled CI guards;
  and
- active 0.92 design and tracker documents needed to interpret the inventory.

Historical reports are sampled for method identity, latest observed runs,
comparability, links, and retention structure. Their product conclusions are
not revalidated by this inventory.

Product source, manifests, tests, generated interfaces, and runtime behavior
are excluded. No product verdict or product-baseline identity is produced.

## Trace Mode And Safety

Mode: `code_trace`.

The method may read tracked files and Git objects and may run shell syntax
checks plus read-only document guards. It must not run product tests, build
Wasm, access a network, mutate authoritative environments, generate an audit
measurement report through a product test, or change existing audit methods.
Writing the dated primary inventory report, its day summary, and current 0.92
status documents is permitted output.

Record `git status --short` before and after the run. A change outside the
permitted output paths invalidates the run.

## Required Inventory

Record:

1. release anchor, commit/tree identities, audit-tree identity, dirty state,
   toolchain, lockfile hash, and relevant script hashes;
2. every reusable system, invariant, modular, operational, release-line, and
   CI-backed audit candidate found by path and index discovery;
3. purpose, trigger, claimed owner, method identity, implementation sink,
   latest observed evidence, and comparability status where determinable;
4. report and generated-artifact counts and size;
5. conformance against the accepted 0.92 definition contract;
6. overlap and competing-authority candidates;
7. explicit coverage or absence for the additional holistic topics in the
   design; and
8. evidence-backed findings, including exact paths and commands.

## Deterministic Evidence Commands

Use repository-relative paths and record versions for Git, Bash, ripgrep, GNU
findutils, GNU coreutils, Rust, and Cargo.

Required command classes:

- `git rev-parse` for full commit/tree identities;
- `git diff --name-status v0.91.6 -- <inventory paths>` to prove whether the
  current inventory inputs differ from the release anchor;
- `git status --short` before and after collection;
- `find`, `wc`, and `du` for bounded archive counts and sizes;
- `sha256sum` for the method, definitions, and executable helpers;
- `rg --files` plus index and heading scans for candidate discovery;
- an exact recurring-parent report-section conformance scan;
- targeted inspection of method tags, baseline selection, result states,
  retention, and operational guard literals; and
- `bash -n` for the six audit-related scripts identified at design time.

The three `check-*-audit.sh` document guards may be executed because they read
tracked documents only. Instruction and Wasm report generators must not be
executed during inventory because they write reports/build artifacts and run
product tests or builds. `run-layering-guards.sh` belongs to the later product
baseline and is not executed during Phase A.

## Conformance And Finding Rules

- A definition has an explicit method identity only when its active text names
  a stable audit ID and explicit version. A filename or the word `current` is
  insufficient.
- Latest observed evidence is not called comparable unless the definition and
  report identify the same versioned method and its included inputs can be
  reproduced.
- Missing owner, scope, exclusions, commands, negative proof, runtime/cost,
  result rules, retention, redaction, or follow-up ownership is recorded
  separately rather than inferred from report volume.
- A parent index or governance contract that disagrees with child definitions,
  scripts, or active operational documents produces a finding.
- A script that successfully enforces a stale literal contract is evidence of
  the conflict, not a passing audit-system conclusion.
- Finding severity measures impact; confidence measures evidence strength.

## Result Rules

- `pass`: the inventory is complete and finds no audit-system defect that must
  be corrected before product baselining.
- `fail`: the inventory completes and establishes at least one accepted
  audit-system defect that blocks method freeze or product baselining.
- `partial`: required inventory scope was sampled but not completed.
- `blocked`: authoritative inventory evidence could not be produced.
- `not_applicable`: not permitted for this required Phase A method.

## Outputs And Retention

The primary output is one dated Markdown report plus required day/month index
updates. Do not retain raw command transcripts when the report contains the
necessary counts, hashes, and exact command descriptions. Retain no secrets,
tokens, credentials, sensitive principals, environment dumps, or private
paths. Repository-relative paths are preferred in evidence.

The report must include the 0.92 run identity block, method fingerprint,
evidence manifest, inventory tables, conformance results, findings, explicit
unreviewed boundaries, verification readout, and next action.

## Positive And Invalid Proof

- Positive completeness proof: all candidate paths found through canonical
  indexes, audit-labelled operational files, and executable references are
  reconciled in the report.
- Known-invalid proof: an unversioned `current` tag must not satisfy method
  identity; a successful guard that pins a historical release verdict must not
  satisfy current authority; and a cross-day baseline must not satisfy a
  mandatory same-day baseline rule.
- Boundary proof: historical reports remain evidence inputs but cannot become
  current method authority merely through repetition or file count.
