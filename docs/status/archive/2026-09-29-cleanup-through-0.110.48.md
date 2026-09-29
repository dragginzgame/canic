# Archived cleanup handoff through 0.110.48

## Published baseline and active batch

The maintainer published all seven packages at `0.110.47` and pushed the source.
Package versions remain `0.110.47`; `0.110.48` groups cleanup and reinstall
review corrections. The active
batch removes unused document helpers, obsolete API-removal tests, remaining
status-based release machinery and duplicate audit payloads. It also compacts
maintained planning documents while retaining historical evidence and links.

The accepted CANIC-185 follow-up now explains unavailable infrastructure funding
after an unpaid review has replaced the completed-Fleet record. Paired readiness
returns a typed reason with exact review identifiers and plain-output guidance.
It preserves retained authority and allows artifact compilation needed for a new
review; no replacement quote or payment approval is inferred. The
[acceptance note](../../audits/reports/2026-09/2026-09-29/toko-unpaid-review-readiness.md)
owns the diagnostic contract and qualification boundary. Toko acceptance remains
pending; its repository and live state were not changed.

CANIC-186's unpaid prior-review selection is corrected by exact-digest local
cancellation through the existing reinstall owner and shared archival/removal
primitives. Native recovery checks and the public-CLI PocketIC reset journey pass;
the [qualification note](../../audits/reports/2026-09/2026-09-29/toko-unpaid-review-cancellation.md)
records the evidence and downstream adoption boundary. Toko's real retained
review and estate remain unchanged; do not apply or fund its superseded .16 review.

Release validation now belongs to a generated `release-validation.json`, written
transactionally by the version bump and read from immutable tags for fast-lane
qualification. Status is an ordinary handoff: bumping, staging and publication
neither parse nor mutate it. Existing tags without the structured receipt require
the complete lane; no status-marker fallback or receipt backfill is introduced.

- [Maintained scope and batch dispositions](../../design/0.110-fleet-runtime-contraction/status.md).
- [Cleanup draft](../../changelog/0.110.md#011048---2026-09-29).
- [Published .47 behavior](../../changelog/0.110.md#011047---2026-09-29).

## Validation and release boundary

The maintainer's September 29 complete validation passed before .47 publication.
All seven publication/propagation records succeeded. Their logs remain in
`target/publication-runs/20260929T095623Z-25494.SaPMt4` and the corresponding
validation-run directories. Explicit post-publication cleanup cleared Cargo
outputs and 39 old audit build caches while preserving logs and deployment state.

The first cleanup pass is complete. Focused
checks passed: 136 configuration tests, 16 host binding/mutation tests, 18
release-flow tests, five release-index tests and one changelog-governance test.
Release-lane/integrity checks, document guards, shell lint, formatting, evidence
hashes and changed-document links also passed. The .48 changelog draft is ready;
package versions remain .47 until the governed release transaction.

The second cleanup pass removes the dormant module-source resolver, unused
installation/activation and Host wrappers, obsolete projection/wire-format tests,
and staged auth lint exemptions. Shared publication and same-release recovery
remain intact. Its 458 focused tests pass: 346 Core auth/identity/activation/metrics,
seven cost guards, 80 Host configuration/release/balance and 25 Control Plane
installation/publication tests. Core, Control Plane and Host library checks pass;
Core also compiles with local application authorization enabled. Scoped formatting,
document semantics and whitespace checks pass. This cleanup is ready for review;
the combined batch also includes the CANIC-186 qualification below.

The third cleanup pass removes 49 unconsumed Control Plane facade methods and
27 dead workflow entrypoints, plus leftover manifest/convenience helpers. Active
operation status, autonomous drivers and their authority checks remain with their
existing owners. CLI support now owns shared token/cycles recipient resolution;
no lower-layer dependency changes. Duplicate tests and the metrics Markdown-layout
guard are removed. This pass removes a net 873 Rust lines across 19 source files.
Its 340 focused tests pass: 28 CLI, 32 Control Plane, 180 Core, 80 Host and 20
lifecycle/cost/policy/DTO boundary checks. Warning-denied Clippy passes for all
four affected packages with all targets/features; the Root macro consumer also
compiles in isolation. Layering, document semantics, scoped formatting and
whitespace checks pass.

The final combined readiness/cancellation checks pass: 36 host tests, three CLI
tests and both recursive help tests. The host selection covers a compiled unpaid
review, unchanged readiness evidence, exact archival and interrupted cancellation;
three governed PocketIC cases remain ignored by that native filter. The selected
public-CLI PocketIC reset case separately passes cancellation, replacement-build
selection, issued-work rejection, lost-response recovery, conservation and
effect-free replay. Scoped formatting, whitespace checks and warning-denied lint
for all host/CLI/internal-test targets and features pass. The complete planned .48
batch is ready for maintainer review and push; its changelog draft is ready for
the governed release transaction, with package versions still .47.

The maintainer-directed .48 validation stopped at Clippy's function-length check
in `release_flow_guard`. Fixture creation is now separate from receipt assertions;
all 18 release-flow tests and warning-denied, all-feature Clippy for that test
target pass. Formatting and whitespace checks pass. This correction did not rerun
the broad gate or change release behavior; the existing changelog remains current.
Package versions remain .47. The maintainer owns commits and selects the release
command; the next release needs the complete lane to establish the structured
receipt for future fast releases.

The fourth cleanup pass implements all six duplicate-flow recommendations:
shared Host tool installation, raw/gzip qualification and bounded completed-estate
queries; shared CLI execution; ops-owned error and metric classification; and one
storage-owned registry coverage predicate. Tool-specific trust, method limits,
domain authority, pagination and recovery sequencing remain intact. Both registry
validation boundaries remain enforced. Isolated Root qualification also corrected
Coordinator-only fixture and Store-only import gates without lint suppressions.

Validation passes: 342 selected default-feature tests across CLI, Control Plane,
Core and Host; 185 all-feature Core tests; 57 isolated Root tests; nine Store-enabled
template tests; and 20 lifecycle/cost/policy/DTO boundary tests. The three ignored
Host contract inspections require explicitly supplied completed-source workspaces;
no live estate was inspected. Warning-denied Clippy covers all targets/features of
all four affected packages, with Control Plane rerun after its feature-gate fix.
Layering, document semantics, scoped formatting and whitespace checks pass. No
broad gate ran. The accepted cleanup batch is complete and ready for maintainer
review and push; the .48 changelog draft is ready for the governed release flow.
Package versions remain .47, and changes are uncommitted.

The final 0.110 closeout must include FI1 and subsequent corrections and receive
explicit human acceptance before 0.111 implementation. Downstream adoption,
live/performance acceptance and complete cycle attribution remain separate.

## History

- [Publication recovery and earlier .47 handoff](2026-09-29-publication.md).
- [Dated .110 implementation history](2026-09-29-fleet-runtime-contraction.md).
- [Earlier detailed session handoff](2026-09-29-precompact.md).
