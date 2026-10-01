# Complexity, hard cuts and unfinished features — 2026-10-01

The published source still contains three Host document-shape compatibility
families. Confirmed obsolete tests and one predecessor-only test struct are
removed, and three gaps in the hard-cut source guard are fixed. No maintained
Canic product contract above v1 was found. Fresh live backup creation remains
explicitly unimplemented.

Repository-wide screening is complete for the listed patterns and inventory;
manual reachability review covers the named candidates and owners below. This
is **not** a claim that every function in 585,653 Rust source lines has been
independently reviewed, that all tests are necessary, or that all remaining
compatibility has been removed. The residual Host cuts need paid-operation
recovery classification and are separate from the concurrent FR1 implementation.

## 1. Run metadata

- Primary method: `CANIC-MODULE-SURFACE-001`, `MSH-2.3`, Tier 2, manual
  `code_trace`; implementation explicitly requested. Reviewer: Codex `/root`,
  single review. No new P0/P1 finding or independent second-review claim.
- Manifest: `ST-1`, `AT-1`, `DC-1`, `pre-1.0-hard-cut`, `raw-wasm-primary`,
  `HP-1`, `read-only-first`. Identity, hashes, source inventory, retained
  mechanical output and finding records: [complexity-hard-cuts-and-feature-gaps.json](complexity-hard-cuts-and-feature-gaps.json).
- Release anchor: `v0.110.49`; source commit
  `75c7f0998fd4531f8a6b81d91b53b71daffca332`. The structured release receipt
  names a complete gate for its predecessor implementation commit
  `32da629d0214bf791541a9b3c1832dbef13ece29`. It is historical release evidence,
  not qualification of these edits.
- Baseline: `v0.110.48`, commit
  `8d37c74c9a4457b9e2bd47ee883f98fd2889d63b`. Both snapshots are read from Git,
  excluding current IcyDB edits and concurrent Fleet release additions.
- Compared report: [surface cleanup and subsystem usefulness](surface-cleanup-and-subsystem-usefulness.md),
  non-comparable because scope and measurement coverage differ. Core metrics
  compare identical v3 script runs at the two immutable source commits.
- Run result: `partial`; validity: `valid` for screening, mechanical counts and
  named traces. A full MSH verdict for every retained code unit and the complete
  `CANIC-COMPLEXITY-001/v3` entropy score are not claimed. That method's manual
  axis products, switch sets and two-reviewer P0/P1 attribution were not run.
- Scope: all tracked Rust in `crates/`, `canisters/`, `apps/`; product version,
  alias/default, compatibility, incomplete-code and stale-test screens;
  generated Candid, crate exports/features, relevant scripts, active guides and
  0.110 tracker. Host traces are pinned to the published source.
- Exclusions: sibling repositories, live canisters/accounts, retained incident
  artifacts, generated build output, optimized Wasm/instruction measurement,
  historical-document rewriting, concurrent FR1 changes and IcyDB qualification.

## 2. Step status

PASS records evidence for the frozen screening or named sample, not exhaustive
per-function reachability approval.

| MSH step | Status | Evidence / limit |
| --- | --- | --- |
| 0 — identity | PASS | Immutable source/baseline and manifest; dirty cleanup bytes recorded separately |
| 1 — inventory | PASS | All 1,950 tracked Rust files; export/feature owners sampled |
| 2 — stale signals | PASS | Product generations, aliases/defaults, predecessor tests and compatibility paths traced |
| 3 — authority | PASS | Host review hashes, restoration, paid journals and backup preflight; no live authority inspection |
| 4 — retained complexity | PASS | File/owner counts and large-state-machine attribution; no semantic entropy score |
| 5 — generated boundary | PASS | Current Candid inventories and facade/compiler checks retained; Wasm unmeasured |
| 6 — tests/features | PASS | Lexical test screen, exact-body candidates and named gaps; no full redundancy proof |
| 7 — removal plan | PASS | Five implemented test cuts, one detector correction, three residual Host shape families and bounded follow-ups |
| 8 — runtime shape | PASS | Cleanup is test-only; runtime-sensitive follow-ups classified |
| 9 — risk | PASS | Qualitative deletion-pressure score only |

## 3. Evidence log

| Evidence | Command / inspection | Result |
| --- | --- | --- |
| Baseline | `git log -2`, `git tag --list 'v0.110.*'`, `release-validation.json` | .49 tagged; original handoff/tracker still describes prepublication state |
| Whole-source inventory | `git archive <commit> crates canisters apps`; frozen count definition and normalized census retained in manifest | 1,950 current Rust files; baseline uses the same scope |
| Core mechanical comparison | `bash docs/audits/scripts/measure-complexity-v3.sh <full-commit>` at .48 and .49 | Both succeed; normalized stdout retained in manifest |
| Compatibility | `rg` across source for backward/legacy/obsolete/migration/fallback, Serde alias/default and version declarations | Three Host document-shape families traced; upstream/default/runtime fallbacks classified separately |
| Product hard cut | `bash scripts/ci/check-pre-1-0-hard-cut.sh`; inspect higher-number matches; compare baseline/current regex against each new fixture | Guard previously missed all three new fixtures; corrected guard rejects each and retains upstream/audit exceptions |
| Incomplete implementation | `rg` for `todo!`, `unimplemented!`, `TODO`, `FIXME`, `NotImplemented`, unavailable/unsupported and active tracker gaps | One explicit production preflight stub; other gaps require semantic/tracker review |
| Redundancy | Balanced-body normalized test discovery on immutable source, then named assertion review | No exact-body duplicate candidates; this does not exclude semantic redundancy |
| Consumers | Host plan hydration/compaction, review digest, bootstrap registration recovery, role macros, Root lifecycle, backup/restore runner | Retained owners and deletion obligations below |
| Targeted verification | Exact commands and outcomes in section 11 and manifest | No full workspace gate or live operation |

## 4. Inventory and retained complexity

Counts include tests, blank lines and comments. They measure source footprint,
not optimized artifact cost. `#[test]` counts are lexical; they are not a
registered, selected or executed test total.

| Owner | Rust files at .49 | Physical lines | Lines versus .48 |
| --- | ---: | ---: | ---: |
| Facade | 56 | 13,927 | -685 |
| Core | 664 | 163,769 | +1,295 |
| Macros | 12 | 3,039 | +377 |
| Control Plane | 179 | 104,825 | +474 |
| Host | 527 | 159,452 | -11,433 |
| CLI | 229 | 52,243 | +505 |
| Backup | 131 | 32,349 | +375 |
| Internal Testing | 61 | 35,290 | +2,001 |
| Integration Testing | 28 | 14,325 | +337 |
| Apps and canister fixtures | 63 | 6,434 | +215 |
| **Total** | **1,950** | **585,653** | **-6,539** |

The screen found 4,127 lexical `#[test]` attributes. Deleting tests from that
number alone would remove evidence without identifying its owner.

Canonical Core v3 counters deliberately include inline tests in production
files. They differ from the whole-repository census scope.

| Core v3 mechanical metric | .48 | .49 | Delta |
| --- | ---: | ---: | ---: |
| Files under `canic-core/src` | 650 | 653 | +3 |
| Logical lines | 132,020 | 133,316 | +1,296 |
| Non-test files | 596 | 598 | +2 |
| Non-test logical lines | 110,570 | 111,215 | +645 |
| Non-test files at least 600 logical lines | 33 | 33 | 0 |

The largest risk-bearing owners are current state machines:

| Owner / current file | Physical lines | Retained authority and change risk |
| --- | ---: | --- |
| `canic-control-plane/src/ops/component_registry/mod.rs` | 6,523 | Allocation, controllers, directory/runtime activation, draining and removal; one removal must preserve exact allocation identities across every route |
| `canic-control-plane/src/storage/stable/component_registry/mod.rs` | 6,049 | Persisted transition proofs, capacity and terminal receipts; record deletion can destroy same-operation recovery |
| `canic-control-plane/src/workflow/component_registry/mod.rs` | 6,259 | Top-level, peer, child and subtree orchestration; superficially similar paths carry different authenticated bindings |
| `canic-host/src/fleet_ensure/ops/platform.rs` | 10,030 | Platform adapters plus inline tests; management, Ledger, native funding and Root effects share the reviewed journal |
| `canic-host/src/fleet_ensure/workflow/mod.rs` | 5,954 | Bootstrap/import/reset, funding pauses, continuation and terminal replay; budget and journal changes have multi-phase fallout |
| `canic-testing-internal/src/pic/fleet_registry/baseline.rs` | 17,813 | Test catalogue/journeys and shared fixture state; not production runtime dead code |

The first three Component Registry owners total **18,831 physical lines**.
Their coupling is a real review cost: allocation/removal changes cross ops,
storage and orchestration. The pending allocation-scoped replay/issuer/funding
findings make this a correctness-linked hotspot, rather than a size-only
recommendation to split or delete it. Prioritize those exact bindings before a
generic rewrite. Host's two large adapter/workflow files total **15,984 lines**,
including tests, and own the compatibility residue in section 5.

Current request/proof/replay and parent/role branches are not alternative product
generations. Same-release policy generations, IC canister versions and key
versions may exceed one; resetting them would weaken current authority checks.

## 5. Dead/stale candidates and implemented cuts

New findings below have `first_observed_at = 2026-10-01`, source method
`CANIC-MODULE-SURFACE-001/MSH-2.3`, and no agent-created fix/validation commit.
All are P2 or P3; full structured records are in the manifest.

| Canonical finding / owner | Evidence | Disposition |
| --- | --- | --- |
| `CANIC-0.110-AUTH-STALE-TEST-001` / Core DTO tests | `PresenterlessDelegatedTokenClaims` exists only to encode and reject a predecessor token | Fixed: delete the struct and predecessor rejection; retain current claims/request identity assertions |
| `CANIC-0.110-REFILL-STALE-TEST-001` / Core refill tests | `retry_rejects_pre_hard_cut_nonself_operation` explicitly exercises the removed nonself refill route | Fixed: delete the test; current retry field matching and production self-target checks remain |
| `CANIC-0.110-FACADE-STALE-TEST-001` / Facade protocol tests | Removed facade path, old cycles record, AutomaticTopup types, old FleetKey/Store authority-directory forms | Fixed: remove historical absence assertions; retain current Candid types and exact service inventory |
| `CANIC-0.110-FACADE-REDUNDANT-TEST-001` / Facade tests | Store memory absence checked repeatedly despite exact method inventory; filtered set rechecks its excluded value; string literal tests its own words | Fixed: remove redundant checks and the separate memory-absence test |
| `CANIC-0.110-CLI-STALE-TEST-001` / CLI manifest tests | Help test freezes explanatory phrases and absence of removed `fleet backup` wording | Fixed: delete the test; command parsing and manifest validation tests remain |
| `CANIC-0.110-HARD-CUT-DETECTOR-001` / CI policy guard | The published pattern misses `ExampleContractV2`, unprefixed `SCHEMA_VERSION = 2`, and a `canic/example/v10` domain | Fixed: recognize versioned type declarations, zero-prefix schema constants and multi-digit wire generations; each new fixture independently changes from unmatched to matched |
| `CANIC-0.110-HOST-INLINE-PLAN-001` / Host plan content | `load_chunk_bytes` accepts inline bytes; `contains_inline_bytes`, `compact_inline_plan` and apply-time rewriting support the former durable representation | Deferred: confirmed stale-format lane, with unfinished-operation recovery obligation to classify before removal |
| `CANIC-0.110-HOST-IMPORT-SHAPE-001` / Host import model | `funding_credits` defaults and disappears when empty specifically to preserve an existing uncredited review digest | Deferred: compatibility-shaped hash authority; changing serialization changes the approved Root reservation hash |
| `CANIC-0.110-HOST-REGISTRATION-SHAPE-001` / Host bootstrap/journal model | `bootstrap_registration_recovery` and `registration_recovery_sha256` use omission defaults instead of explicit nullable current fields | Deferred: current document-shape inconsistency; identify unfinished issued journals before a strict-field hard cut |

The five-file cleanup removes **107 net Rust lines** after scoped formatting,
including one predecessor-only struct and three standalone tests. No production
runtime code or product schema is changed by that cleanup. The independent
shell guard correction adds three current-policy fixtures; these validate the
maintained v1 constraint rather than a removed legacy API or command.

No other production orphan was confirmed by declaration/consumer screening.
Single-mention generated schema types, feature-gated descriptors and public API
items are not deleted on lexical mention count. Host FR1 types that appeared
mid-audit are excluded by the maintainer's explicit separate-work instruction.

## 6. Runtime authority and compatibility classification

The Host residues are not a licence to read old completed-Fleet schemas. Completed
records remain opaque historical evidence; current inventory-driven reset is the
maintained completed-estate route. The residues above affect the representation
of a retained executable plan, review hash or journal, so their removal must not
invalidate genuinely unfinished paid effects.

No live unfinished-plan inventory was supplied or queried. This audit therefore
does not establish that any particular old-format plan can be discarded. Preserve
the exact CANIC-188 frozen `.48` bundle and restoration authority; it is a separate
explicit exception, not a general compatibility lane. No new migration, import,
alias, fallback or generation is proposed as a solution.

| Apparent residue | Classification / retention reason |
| --- | --- |
| ICP `alias = "canister_version"`, CLI version range and `/api/v2` routes | Upstream Host adapter/API contracts; outside the Canic-owned v1 generation rule |
| Human-authored TOML defaults | Intentional optional configuration inputs; not predecessor record decoding |
| Runtime funding fallback, default ingress limits, unavailable shard preservation | Current operating policies; deleting them changes current safety/availability |
| Funding/admission predecessor generations | Same-release policy rotation and exact replay; not cross-release schema generations |
| Retired diagnostic identities | Historical reason ledger, not executable schema compatibility; current generated retired list is empty |
| `*Data` wrappers used by state descriptors/tests | Canonical current state-contract names; feature-gated absence is not proof of dead storage |
| Delegation tombstone resurrection tests | Current credential revocation safety, retained; distinct from tests naming removed APIs |
| Hard-cut source guard and invalid-version tests | Current executable/schema policy checks, retained; no legacy command or obsolete schema fixture added |
| Lifecycle cross-release refusal | Maintained reinstall-only release binding; same-release restoration remains supported |
| Audit methods v2/v3/v6, Cargo lock schema 4, upstream dependency major versions | Explicit audit/upstream exceptions; neither Canic-owned protocol generations nor deletion candidates |
| `scripts/dev/*`, test Ledger/CMC stubs, retained incident artifacts | Maintainer helpers, deterministic external adapters and exact incident authority; retained |

## 7. Facade, generated boundaries and test coverage

Facade exports still use `__internal` and `__build` for generated endpoint,
lifecycle and build wiring. Ordinary public API, generated infrastructure and
current Candid types are distinct consumers. Broad core/control-plane re-exports
inside these hidden modules cannot be classified as dead from normal imports.

Current exact Store service inventory, current field/type checks, compiler-resolved
hidden CDK imports and typed facade signatures survive. Access guards, secret
redaction, controller/caller binding, unavailable observations, cycle conservation
and lost-response/effect-free replay checks survive. An assertion's negative
polarity is not itself evidence of an anti-resurrection test.

Source-spelling checks remain in `managed_endpoint_gate.rs` and portions of
`protocol_surface.rs`, including endpoint modes, guard placement and activation
hook dispatch. These are weaker than generated behavior proofs but currently
protect real maintained behavior. Replacing that coverage is a separate bounded
test-hardening task; deleting it wholesale would create an evidence gap. This
continues the prior report's S5 partial disposition.

The exact-body duplicate screen found no candidate of at least 80 normalized
characters. It is only a discovery aid: comments/string braces, feature variants,
table-driven cases and equivalent behavior with different bodies prevent an
exhaustive redundancy conclusion. Native policy and PocketIC evidence at different
boundaries are not interchangeable merely because their assertions resemble one
another. No aggregate test count becomes a release gate.

## 8. Removal plan

| Residual candidate | Smallest complete cut / required proof |
| --- | --- |
| Inline durable plans | Remove the inline reader branch, detection/compaction function, apply-time call and predecessor-format portions of tests together. Retain in-memory chunk bytes and current hash-only publication/hydration. Classify unfinished plans first; qualify interrupted current publication and exact replay. Do not add a legacy reader elsewhere. |
| Import review omission | Require an explicit `funding_credits` array and use the current v1 canonical review hash. Empty credit arrays remain valid. Demonstrate uncredited and credited current reviews, substitution rejection, Root reservation identity and lost-response replay without rehashing an issued paid import. |
| Bootstrap recovery omissions | Use the existing `required_option` deserializer and explicit null values in current journal/inspection documents. Update current shape tests and current generation together; qualify exact registration extension, exhausted allowance and interrupted publication. |
| Standalone Root retirement | Existing `CANIC-0.110-ROOT-RETIREMENT-001` / accepted FR1 owns the contract cut. Keep shared Store adoption, inventory, controller and cycle owners. Concurrent implementation is excluded from this audit. |
| Fixture-data delivery | Existing `CANIC-0.110-FIXTURE-SCOPE-001`: no checked-in application metadata selects a fixture, but importer/Store grants and readiness are reachable product behavior. Trace maintained external consumers before a whole-feature removal; no size saving claimed from source counts. |
| Generated source checks | Continue prior S5 with generated compile/behavior evidence for the maintained guards/modes; then remove text-spelling assertions and their readers. |

The Host cuts are recorded for their owner instead of being folded into concurrent
FR1 edits. The source audit gives their concrete removal scope; it does not invent
a new permission ceremony or authorize loss of unfinished-operation authority.

## 9. Runtime shape and optimization risk

The implemented Rust cleanup changes only tests; the CI correction strengthens
current-policy source admission. Neither adds allocation, dispatch,
generic monomorphization, serialization, state migration or Wasm success-path work.
Raw Wasm and instruction measurement are not required for this delta.

Inline-plan removal is Host encode/decode and interrupted-publication sensitive.
Review/journal omission cuts are Host persistence and exact-hash sensitive.
Root retirement/fixture cuts are canister runtime, controller, stable-state and
conservation sensitive; those owner batches need current artifact and recovery
proof. The large Component Registry files should retain their explicit transition
shape until a finding-backed change establishes a smaller safe owner boundary.

## 10. Unfinished features and risk

Deletion pressure is **4/10**, a classified MSH judgment for these samples: five
test-only cuts, one CI detector correction, three surviving Host shape families and the previously documented
retirement/fixture decisions. This is not the overall architecture's complexity
index or a comparative trend score.

| Feature / owner | Actual limitation at published .49 | Evidence / next bounded outcome |
| --- | --- | --- |
| Fresh backup create / CLI, Host, Backup | **Unimplemented adapter.** `preflight_receipts` always rejects; fresh execution cannot create a live backup. Dry-run planning and existing backup recovery remain. Selection also requires one Fleet Subnet Root. | `backup/create/executor/mod.rs`, availability guide; implement authoritative Coordinator/Component membership, controller/read authority and consistent quiescence in an accepted backup batch |
| Whole-Fleet release to reusable capacity / FR1 | **Absent from published baseline.** The accepted outcome retains infrastructure/child IDs and controlled cycles; current reset is not that complete capacity-release feature. | 0.110 design/status FR1; concurrent work is expected and explicitly excluded, so this audit gives no verdict on its progress |
| Grouped Root retirement / Control Plane | **Unsupported current scope.** Any referenced operation journal, placement or service fences standalone removal; completed journal references are not phase-filtered. No grouped operator retirement path was found. | `ops/fleet_coordinator/root_lifecycle.rs::grouped_root_lifecycle_references`; accepted FR1 owns contraction/replacement |
| Import after exhausted/older-unknown effects / R2 | **Accepted recovery gaps remain.** Current certified request retirement and bounded retries are implemented; the tracker still leaves exhausted budgets and older unknown outcomes open. | R2 tracker plus paid-attempt owners under `canister_pool/capacity_import`; do not describe bounded blocking as complete convergence |
| Funding/conservation / R3 and R8 | **Partially implemented accepted outcome.** Native grant/proof and replay corrections are present; complete native/reserved/ICP and external-account disposal accounting remains open. | Original R3/R8 findings; terminal conservation and exact destination/account custody proofs |
| Recycled allocation authority / R4 | **Partially implemented accepted outcome.** Routing/count/reset/removal fixes are present; caller replay, token issuer and funding isolation by allocation remain open. | Original `r2-recycled-principal-authority-1` through `-3`; qualify stale caller/proof/funding denial against the replacement allocation |
| Existing backup restore / R5 | **Authority/capture/upload gaps remain.** Restore planning copies manifest identities without binding the source release; upload reconciliation adopts a single new snapshot ID while ignoring its reported size/time. Existing checks do not prove a complete upload's exact content. | `restore/plan/mod.rs`, `restore/runner/execute.rs::reconcile_upload_operation`; complete-upload proof, source/target/release/controller authority and consistent capture remain accepted work |
| Background recovery / R6 | **Mechanism exists; full recovery outcome pending.** A durable attempt lease and role-native watchdog are implemented. Tracker still requires owner/trap recovery qualification; intent cleanup liveness is explicitly unqualified. Application timers/hooks deliberately remain fail-stop after a trap. | Core `workflow/runtime/async_job`, `workflow/runtime/timer`, native/PocketIC timer evidence and reliability-class guide; do not label all background recovery absent or all traps automatically recoverable |
| Operation-specific convergence / R7 | **Accepted outcome pending.** Shared mirror/policy changes can interact with exact operation ownership; the multi-root convergence acceptance remains open. | R7 tracker; operation-specific mirror acknowledgements, rotation/activation fences and multi-root retry proofs |
| Runtime cost attribution / diagnostics | **Measurement limitation.** Request/callback metrics and bounded comparisons exist; complete asynchronous instruction attribution and internal IC/remote-wait timing are unavailable, and instruction-audit root-proof/token stage checkpoints are missing. | Runtime/observability guides and `instruction_audit_support/report.rs`; not evidence that the functional APIs are unimplemented |
| Native macOS / Host, CLI | **Qualification gap.** CI includes both architectures; this Linux audit supplies no native macOS execution result. | Existing Qualification tracker; retain distinction between platform implementation and executed native evidence |
| CANIC-188 live convergence | **Operational work outstanding**, distinct from missing in-repository repair code. Exact repair is locally qualified; publication does not execute it. | Issued-import recovery decision; no live operation or downstream mutation authorized by this audit |

These rows consolidate existing accepted findings and documented limits. They do
not close the original 401-finding review or infer that all uncounted findings
still reproduce. Fresh backup preflight is the clearest demonstrably unavailable
user-facing feature. A passing .49 release gate does not erase these documented
product limits.

## 11. Verification readout

All **102 focused native tests pass**: 46 protocol-surface tests, three reference
surface tests, two current authentication DTO tests, 47 refill tests and four
CLI manifest tests. The exact commands and retained log hashes are in the
manifest. Warning-denied, all-feature Clippy passes for the two changed facade
integration targets and the Core/CLI library and test targets.

The corrected source guard, its fixture self-test, ShellCheck, shell syntax,
five-file Rust formatting and diff hygiene pass. Baseline-versus-corrected regex
results for each of the three new fixtures are retained in the manifest.

The shared target lock was checked before every Cargo invocation. An initial
attempt stopped at that lock before starting Cargo; no competing validation was
run. Checks used the authorized worktree, including the existing dependency
update and concurrent Host additions. They qualify this named cleanup; they are
not frozen-baseline execution or qualification of FR1. No full workspace gate,
PocketIC suite, publication or live operation ran for this audit.

## 12. Disposition

Five confirmed test-residue findings and one CI detector finding are fixed in
uncommitted edits. Three Host
document-shape families remain deferred with exact removal and recovery scopes.
Existing Root retirement/fixture/backup findings retain their canonical owners.
No maintained product generation above v1 was discovered and no replacement
compatibility lane was added. Production deletion, blanket semantic test
deduplication and full per-unit MSH review remain uncompleted claims.

This audit and test-only cleanup are separate from FR1. They do not make the
combined worktree push-ready, close 0.110, authorize 0.111, or mutate versions,
Git history, deployment authority or incident evidence.

## 13. Follow-up

Prioritize fresh backup authority/availability and the accepted recovery gaps as
product outcomes. For the requested hard cuts, finish the three named Host
shape removals after classifying unfinished issued records, then let FR1 own
retirement contraction. Review fixture consumer need and replace generated
source-spelling checks with current behavior evidence. Full per-unit reachability
and semantic test-redundancy review require additional owner-by-owner audit work;
this report supplies the inventory and named starting points without awarding
absence credit to unreviewed functions.
