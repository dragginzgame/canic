# Surface cleanup and subsystem usefulness — 2026-10-01

The proven cleanup is implemented: 100 net Rust lines removed, 381 fewer lines
in active design/operating documents, and obsolete planning retained separately
as history. The usefulness trace found a stronger large candidate: standalone
Root retirement cannot serve normally provisioned grouped Roots. Fixture-data
delivery is another substantial optional product family. Neither is established
as wholly dead code; their callers, persistence and recovery obligations matter.

## 1. Run metadata

- Method: `CANIC-MODULE-SURFACE-001`, `MSH-2.3`; Tier 2; reviewer: Codex `/root`,
  single review. Manifest: `ST-1`, `AT-1`, `DC-1`, `pre-1.0-hard-cut`,
  `raw-wasm-primary`, `HP-1`, `read-only-first`.
- Result: `partial`; validity: `valid` for named samples; non-comparable with
  the [preceding audit](module-surface-hardening.md), whose scope/version differ.
- Implementation requested for S1–S4/S7 and the proven portion of S5; larger
  subsystem review is `code_trace`. No minor closeout or release verdict.
- Commit, committed tree/product tree, lock/method/script hashes, tools, timing
  and reviewed source bytes are in [the manifest](surface-cleanup-and-subsystem-usefulness.json).
  Concurrent deployment work makes this a dirty-worktree sample, not a complete
  immutable product qualification.
- Scope: sampled Host/CLI Fleet, facade tests, Core/Control Plane retirement and
  fixture delivery, backup runner, current guides and active 0.110 design.
  Generated boundaries and tests sampled; siblings, live inventories, incident
  artifacts, optimized Wasm and concurrent deployment changes excluded.

## 2. Step status

PASS means the named sample was classified, not that every subsystem was reviewed.

| Step | Status | Evidence / limitation |
| --- | --- | --- |
| 0 — metadata | PASS | Manifest; source hashes bind dirty bytes |
| 1 — retention inventory | PASS | Section 4; sampled callers |
| 2 — dead/stale signals | PASS | Section 5; not inferred from mention counts |
| 3 — authority drift | PASS | Section 6; no live authority inspection |
| 4 — retained complexity | PASS | Sections 5/8; owner footprints include shared code/tests |
| 5 — generated boundary | PASS | Section 7; compiler probe, no expanded Wasm measurement |
| 6 — tests/features | PASS | Sections 4/7; no full test-registry review |
| 7 — removal plan | PASS | Section 8; larger product decisions deferred |
| 8 — runtime shape | PASS | Section 9; no size/speed claim |
| 9 — risk score | PASS | Section 10; qualitative sample only |

## 3. Evidence log

Commands run from the Canic root; paths and hashes are retained in the manifest.

| Evidence | Commands / inspection | Result |
| --- | --- | --- |
| Cleanup delta | `git diff --numstat -- <changed source/design paths>` | 100 net Rust lines; 381 active Markdown lines |
| Root caller screen | `rg -n 'RemoveRoot\|root_retirement\|RootRemoval' crates/canic-host/src crates/canic-cli/src` | No current operator callers found |
| Root eligibility | Inspect `ops/fleet_coordinator/root_lifecycle.rs:66–109` and draining reservation admission | Journals, placements and services fence removal; journal phase is not filtered |
| Root consumers | Coordinator/Root command dispatch, timer workflows, baseline PocketIC cases | Reachable standalone feature, distinct from clean reinstall |
| Fixture activation | Inspect Host metadata selection/action append; Core startup/readiness; Store role contract | Explicit fixture input triggers work; runtime/protocol owners remain |
| Application selection | `rg -n 'fixture\s*=' apps --glob Cargo.toml` | No checked-in application manifest declaration; external consumers unreviewed |
| Backup stop/recovery | Inspect CLI executor and `runner::accept_preflight_if_needed` | Fresh preflight always fails; accepted journals can bypass it |
| Footprints | Sum `splitlines()` of Rust files in manifest path lists | Root 3,988; fixture 5,016; backup 6,729; includes tests/shared owners |
| Focused qualification | Exact Cargo commands in section 11 | Native tests and scoped lint only |

## 4. Reachable surface and retention inventory

| Surface / owner | Why the caller still matters | Disposition |
| --- | --- | --- |
| Clean reinstall, activation/import recovery — Host/CLI | Current reset route and unfinished paid-effect reconciliation | RETAIN WITH OWNER |
| Initial membership seal and authority fences — Core/Control Plane | Activation publication, controller observation and restore ownership | RETAIN WITH OWNER |
| Standalone Root retirement — Control Plane | Raw authenticated commands, autonomous draining, conservation/readiness tests; absent from normal grouped operator route | DEFER WITH TRIGGER |
| Fixture-data delivery — Core/Control Plane/Host | Configured application import, exact content/grants/receipts and readiness | DEFER WITH TRIGGER |
| Backup runner/plans — Backup/CLI | Injected executors, accepted-journal recovery, same-release restore; fresh CLI adapter is unavailable | RETAIN WITH OWNER |
| Sharding — Core/application | Demo/test hubs explicitly select the feature and expose assign/resolve operations | RETAIN WITH OWNER |
| Optional local Fleet harness — Host | Feature-selected public example and documented disposable PocketIC use | RETAIN WITH OWNER |
| Public Host convenience wrappers — Host | External/generated consumer proof remains incomplete (prior S6) | DEFER WITH TRIGGER |

## 5. Dead/stale candidates and implemented cuts

Prior findings keep their canonical IDs and historical capture. This report records
their later disposition; it does not rewrite the original audit.

| Key | Outcome | Confidence / status |
| --- | --- | --- |
| S1 | Remove two orphaned seal error variants; correct three activation-preparation comments | High; fixed, qualified |
| S2 | Active design 1,649 → 1,280 lines; archive stopped/deferred/completed plans and replace outdated next-action/status instructions; guide loses 12 duplicate lines | High; fixed, links/semantics qualified |
| S3 | Remove README heading/table/wording gate and its unused reader; retain six structured package/feature tests | High; fixed, qualified |
| S4 | Remove four scaffold assertions naming removed legacy contracts; retain current config parsing/role assertions | High; fixed, qualified |
| S5 | Replace CDK export text formatting checks with compiler resolution; remove three explanatory-message checks | Partial; guard/routing/mode source checks retained pending replacement behavior proof |
| S6 | No cut to public Host constructors/wrappers | Deferred; external consumer authority still unproven |
| S7 | Reconcile method/catalog/report identity at MSH 2.3; retain superseded 2.2 fingerprint | High; fixed, catalog guard qualified |

New findings: first observed 2026-10-01; source audit `CANIC-MODULE-SURFACE-001`;
fix/validation commits and waivers N/A. No P0/P1 finding is asserted.

| Key / canonical ID | Class; severity; confidence | Surface class; deletion confidence | Disposition |
| --- | --- | --- | --- |
| U1 / `CANIC-0.110-ROOT-RETIREMENT-001` | evidence_gap; P3; high for grouped exclusion, medium for product usefulness | unclear; blocked on product/recovery ownership | DEFER WITH TRIGGER |
| U2 / `CANIC-0.110-FIXTURE-SCOPE-001` | evidence_gap; P3; medium | live-authority; low without consumer decision | DEFER WITH TRIGGER |
| U3 / `CANIC-0.110-BACKUP-PREFLIGHT-001` | evidence_gap; P3; confirmed fresh adapter gap | live-authority; low for the whole family | RETAIN WITH OWNER |

U1: `require_grouped_root_lifecycle_open` rejects any referenced Root. The journal
predicate checks planned placements without filtering completed operations.
Current placement/service records also fence removal. No current grouped retirement
or operator drain path was found. The
[fixture guide](../../../../features/build-and-evidence/fixture-artifacts.md)
explicitly restricts removal to standalone Roots. The standalone deletion-readiness
and conservation tests use a separately constructed Registry; the grouped journey
asserts rejection. The inspected five owner files total 3,988 lines, but include
live Store adoption and inventory work that must survive any cut.

U2: this is application data delivery, not disposable test scaffolding. Host reads
`package.metadata.canic.fixture`; absence returns without selecting a source.
Empty fixture manifests add no upload actions. Core still calls the importer from
startup/readiness, and the Store contract includes fixture storage/grants. No
assignment produces `NotRequired`. Thirty dedicated Rust files total 5,016 lines,
including tests; optimized retention, cost and downstream use were not measured.

U3: CLI `BackupIcpRunnerExecutor::preflight_receipts` always returns the typed
unavailable failure. Fresh execution therefore stops before effects. However,
`journal.preflight_accepted` bypasses that step, and the library accepts alternate
executors. Deleting the 6,729-line sampled plan/runner/create family would also
remove current recovery. The [availability guide](../../../../features/backup-and-restore/README.md)
already records the missing preflight and requires preserving recovery.

## 6. Runtime authority drift

The implemented cuts remove cold diagnostics and stale instructions; they do not
rename persisted preparation phases, remove initial membership seals, change
controller/cycle authority, or alter unfinished import recovery. Archived plans
are explicitly historical; the active handoff owns current delivery work.

U1's current safety fence is intentional evidence, not permission to bypass it.
Removing the feature requires tracing every unfinished paid draining operation.
U2 owns stable import progress, bound grants and readiness; U3 owns accepted
preflight journals. None may be replaced with inferred DTO/report authority.

## 7. Facade, generated boundaries and tests

The hidden CDK inventory is retained and checked by Rust import resolution rather
than source formatting. Current macro guard/routing/mode assertions remain.
Scaffold tests still parse maintained configuration. Structured Cargo feature
tests still enforce package behavior. No new legacy anti-resurrection test was
added. Root/Store commands and stable records remain generated-boundary consumers
until a whole-feature hard cut updates those contracts and their tests together.

## 8. Removal safety plan

| Candidate | Smallest next action / owner | Required proof / trigger |
| --- | --- | --- |
| S5 residual | Facade/macros: replace remaining source checks before deleting them | Executable guard/mode rejection and generated-consumer evidence |
| S6 | Host: review public wrapper consumers and pick one canonical entrypoint | External/example/generated caller proof and focused compile |
| U1 | Control Plane: retire the standalone product contract as one bounded hard-cut batch if it is no longer needed | Maintainer scope decision; classify unfinished paid records; preserve live Store adoption/inventory and any shared evacuation owner; update commands, resumers, records, projections, docs and tests together |
| U2 | Core/Control Plane/Host: decide whether any maintained application needs fixture import | Inspect selected consumers; either whole-feature hard cut or accepted build-selection design; measure raw Wasm before making size claims |
| U3 | Backup/CLI: keep recovery; narrow only unsupported fresh promises or implement the missing preflight in its separately accepted batch | Accepted-journal recovery proof; topology/caller/restore authority qualification |

## 9. Runtime shape and optimization risk

Implemented code changes are cold error declarations/comments and test scaffolding;
no hot-loop allocation, dispatch, encoding or stable-memory shape changes.
U1/U2 are hot-runtime, encode/decode and Wasm-sensitive families. Their deletion
plans must preserve terminal conservation, interruption recovery and exact content
authority. Raw Wasm and instruction comparisons are N/A for this trace; a source
line footprint does not predict artifact savings. U3 is a host paid-effect recovery
family, where losing journal interpretation is the principal risk.

## 10. Risk score

Sampled retained deletion pressure: **4/10** (moderate), classified judgment.
The three new findings comprise one unclear product surface and two live-authority
families needing usefulness boundaries; no new orphaned runtime family is proven.
There are zero confirmed new stale-compatibility/generated-fallback families.
Optimization risk applies to two proposed future cuts, not this implemented slice.

## 11. Verification readout

- `cargo test -p canic --test workspace_manifest --test reference_surface --test protocol_surface`:
  56 passed (47 protocol, 3 reference, 6 package/feature).
- `cargo test -p canic-cli --lib scaffold::tests -- --test-threads=1`: 23 passed.
- `cargo clippy -p canic-host -p canic-cli --lib -- -D warnings`: passed.
- `cargo clippy -p canic --test workspace_manifest --test reference_surface --test protocol_surface -- -D warnings`: passed.
- Scoped rustfmt, 39 changed design/guide/archive links and anchors,
  `check-current-document-semantics.sh`, `check-audit-method-catalog.sh` and
  `git diff --check`: passed. Document semantics retains two pre-existing advisory
  layout warnings. Test/lint logs are `target/review-validation/surface-cleanup-*`.
- No workspace gate, PocketIC journey, Wasm measurement, version/publication,
  deployment, incident effect or sibling mutation was run for this slice.
  Shared target ownership was checked; a waiting lint invocation was stopped
  when another validation acquired the target, then rerun after it was free.

## 12. Disposition summary

Five original findings are fixed (S1–S4/S7); S5 is partially implemented, S6
deferred. New subsystem dispositions: two `DEFER WITH TRIGGER`, one
`RETAIN WITH OWNER`. The cleanup slice is ready for review; the complete deployment
batch remains governed by its separate outstanding qualification. Open `.49`
changelog entries include this cleanup; package versions remain `.48`.

## 13. Follow-up

Prioritize the standalone Root retirement scope decision, then fixture-data
consumer need. These are concrete deletion investigations, not authority to
remove recovery from an existing installation. Keep backup recovery and finish
the already accepted deployment batch; no additional release is allocated.
