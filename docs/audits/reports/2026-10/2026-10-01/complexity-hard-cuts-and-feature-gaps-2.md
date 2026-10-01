# Complexity hard-cut follow-up — 2026-10-01

The three deferred Host compatibility families are removed from the maintained
contract. The follow-up also removes fallback execution without retained reviewed
input, obsolete recovery guidance and the redundant old-scope assertion. Current
contracts stay at `v1`. Existing FR1 work and the frozen CANIC-188 bundle are
preserved; no retained operation file, live reservation or incident artifact was rewritten.

This closes the finding-backed cleanup requested in the
[published-baseline audit](complexity-hard-cuts-and-feature-gaps.md). That report
and its measurements remain historical evidence. Its exhaustive per-function
and semantic-test-redundancy exclusions still apply: repository-wide screening
and named manual traces do not prove universal absence of redundancy.

## Run identity and coverage

- Method: `CANIC-MODULE-SURFACE-001/MSH-2.3`, Tier 2, `code_trace`, single
  reviewer Codex `/root`; maintainer-requested implementation. Result: `pass`
  for the named follow-up scope; validity: `valid`. No minor closeout verdict.
- Source anchor: published `v0.110.49`, full commit
  `75c7f0998fd4531f8a6b81d91b53b71daffca332`. The cleanup is uncommitted;
  committed source identity alone does not describe its bytes.
- Immediate causal baseline: tracked source at the start of this follow-up,
  reconstructed from that commit plus the pre-cut worktree diff. It includes
  the stopped session's FR1 edits. Per-file before/after hashes and the isolated
  cleanup diff are retained in the
  [manifest](complexity-hard-cuts-and-feature-gaps-2.json).
- Scope: the deferred durable-plan/import/bootstrap families, their current
  consumers and tests, reviewed-input execution admission, active Fleet guide,
  old-scope test residue, and a repeated repository-wide compatibility/version
  screen over `crates`, `canisters`, `apps` and relevant scripts.
- Exclusions: live inventories/accounts/canisters, external repositories,
  incident execution, full FR1 qualification, IcyDB qualification, full entropy
  scoring, raw/optimized Wasm measurement and exhaustive semantic deduplication.
- Taxonomies: `ST-1`, `AT-1`, `DC-1`; policy `pre-1.0-hard-cut`; Wasm rule
  `raw-wasm-primary`; risk model `HP-1`; proof policy `read-only-first`.
- The original .48/.49 census remains the numerical baseline. No comparison of
  the mixed dirty worktree with that census is presented as a product trend.

| MSH step | Status for follow-up scope | Evidence / boundary |
| --- | --- | --- |
| 0 — identity | PASS | Full anchor plus causal before/after file hashes; dirty worktree explicitly bounded |
| 1 — inventory | PASS | Prior immutable census; named changed-file inventory and retained consumers |
| 2 — stale signals | PASS | Deferred families cut and broad lexical screen repeated; no universal absence credit |
| 3 — authority | PASS | Issued hashes, paid evidence and incident bytes are unchanged; no live authority claim |
| 4 — retained complexity | PASS | Scoped production/test delta; current Root, fixture and recovery owners classified |
| 5 — generated boundary | PASS | No canister export/layout change; current shared APIs compile through Host/CLI |
| 6 — tests/features | PASS | Exact current shape, public admission, interruption/replay and feature-gap retention |
| 7 — removal plan | PASS | Three deferred families and the newly confirmed residue are removed |
| 8 — runtime shape | PASS | Host decode/admission hard cuts; no canister instruction or Wasm size claim |
| 9 — risk | PASS | Exact-hash/recovery sensitivity named; no entropy score or broad release verdict |

## Cuts and current behavior

| Finding | Final maintained behavior / evidence |
| --- | --- |
| `CANIC-0.110-HOST-INLINE-PLAN-001` | Deleted inline chunk loading, `contains_inline_bytes`, `compact_inline_plan` and its apply-time call. Both Wasm and fixture publication hydrate only from exact SHA-256/size references. A durable request containing unreferenced bytes fails authority validation. In-memory chunk bytes and current write-before-publication retention remain. Tests create current plans directly and prove reopen, partial publication, prepared authority, missing/corrupt content and lost-response replay |
| `CANIC-0.110-HOST-IMPORT-SHAPE-001` | Removed empty-credit omission/defaulting from durable reviews and omission from review request projection. Uncredited reviews explicitly serialize `funding_credits: []`; credited reviews retain their original survey evidence. The maintained digest covers the explicit field. Current review, reservation, handoff, origin, publication and restart tests pass |
| `CANIC-0.110-HOST-REGISTRATION-SHAPE-001` | Journal `bootstrap_registration_recovery` and inspection `registration_recovery_sha256` serialize explicit `null` and use the current required-option decoder. Current-field omission tests, null round-trip, registration extension, spent allowances, journal reopen and terminal receipt publication pass |
| `CANIC-0.110-HOST-REVIEWED-INPUT-001` | Removed working-input fallback and its `RetainedDesiredUnavailable` error. Executable plans require retained reviewed input before effect driving. Missing input fails with typed `PlanIntegrity`; the new public-workflow proof verifies zero mutations and unchanged retained bytes. Exact in-progress input/replay still wins over newer working input. Nullable data remains useful only for normalized comparison projections |
| `CANIC-0.110-HOST-SCOPE-TEST-001` | Removed the redundant `scope != project` assertion and renamed its test around the current workspace scope. The current positive serialized scope assertion remains |
| `CANIC-0.110-HOST-COMPAT-GUIDE-001` | Removed inline compaction, omitted-credit digest preservation and pre-retention recovery instructions from the active Fleet guide. Current content references, explicit record fields and reviewed-input recovery are documented. Human command renderings remain current display output; structured `next_action` owns process execution |
| `CANIC-0.110-HARD-CUT-DETECTOR-001` | Extended the earlier correction to colon-separated wire domains, journal/plan/state/layout/data/snapshot/record/response/policy versions and application examples. Two additional current-policy fixtures reject a colon `v2` domain and a journal generation above one. Existing upstream/audit exceptions still pass |

The three prior deferred finding IDs keep their identity and now have
`finding_status = fixed` in this follow-up manifest. The earlier report is not
rewritten. The new input, scope and guide findings have canonical IDs above;
all are P2/P3, with no independent P0/P1 review claim.

The follow-up removes **55 net lines from production module files**, including
comments, and adds **61 net test lines** for exact current durable shape and
effect-free malformed-plan rejection. Combined with the first cleanup, this
audit removes **101 net Rust lines**. These are scoped source counts, not
artifact-size or instruction-cost claims. The added tests assert maintained
record/admission behavior, not rejection of removed command forms or predecessor
schemas. The compatibility compaction portions of two existing tests are gone.

## Authority, retained complexity and feature gaps

Changing serialization changes new review/journal hashes. Nothing here rehashes
an issued import, edits its reservation, resets spent attempts, adopts old state,
or adds a migration/compatibility generation. An issued operation's original hash
and paid evidence remain its authority. Local source cleanup authorizes no live
transition or disposal of that evidence. The exact CANIC-188 repair remains
frozen against `.48` and finishes through its original artifact/CLI sequence.

The repeated screen finds no maintained Canic-owned generation above `v1` and
no additional obsolete decoder among the traced candidates. Retained matches
are current operating authority: upstream ICP API/version contracts, optional
human-authored TOML, same-release policy rotation, credential tombstones,
monotonic operation sequencing and role-specific activation state. Root cascade/
credential records have current Root writers; rejecting those records inside a
Component runtime enforces current role separation. They are not legacy schema
support. Completed-operation records remain opaque history for inventory-driven
reset; unfinished effects still retain exact reconciliation authority.

Standalone Root retirement and fixture delivery retain the prior report's
current consumers. FR1 owns retirement contraction and is unfinished. Absence of
a checked-in fixture selector alone does not make the importer, Store grants or
readiness paths dead. Current generated-source guards retain real endpoint and
lifecycle invariants; they are not classified as redundant merely because they
inspect source text. Replacing those checks with generated execution evidence is
test-hardening work, not a prerequisite for the confirmed hard cuts here.

The [twelve feature/qualification limits](complexity-hard-cuts-and-feature-gaps.md#10-unfinished-features-and-risk)
remain classified. Fresh live backup creation still always fails its unimplemented
Coordinator preflight. Existing backup/restore machinery remains. No feature-gap
row is closed by deleting its tests or documentation. The stopped session has
implemented FR1 admission and an exact snapshot-deletion primitive; a whole-Fleet
release CLI, authenticated collection/quiescence, account settlement, full reset
execution and end-to-end interruption/replay qualification still remain. That is
progress beyond the published baseline, not a completed FR1 feature.

## Verification and disposition

All **241 focused native tests pass**: 165 Host and 76 CLI Fleet tests.
Warning-denied, all-feature Clippy passes for Host and CLI library/test targets.
The hard-cut source guard and fixtures, ShellCheck, shell syntax, scoped Rust
formatting, audit catalog, document semantics and diff hygiene pass. Document
semantics reports its two existing advisory layout warnings for the exact
CANIC-188 design exception. Commands, counts and retained log hashes are in the
manifest. One needless borrow in the new shape proof was corrected after the
first lint pass; the final warning-denied result passes.

The first cleanup's 102 focused tests are separate evidence, giving **343 native
test passes across both cleanup slices**. No test-count total is a release gate.
Shared-target ownership was checked before every Cargo command; no competing
validation or live effect ran.

Current production platform calls and canister stable-state layouts are not
changed by this follow-up. Native checks target Host document/authority admission
and its CLI consumers; a fresh PocketIC suite is not claimed. Existing FR1
snapshot PocketIC evidence remains owned by that separate batch. The two ignored
funding cases remain unexecuted in the selected native run.

The named cleanup is complete and ready for review. It does not complete FR1 or
make the combined worktree push-ready. The release target remains intentionally
unassigned under root Unreleased; no patch number, version, commit, staging,
publication, deployment or broad workspace gate was performed. Historical audit
method revisions and immutable release/incident evidence keep their truthful
versions and bytes.
