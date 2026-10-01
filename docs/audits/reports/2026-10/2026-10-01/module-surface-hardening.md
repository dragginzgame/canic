# Obsolete and redundant surface audit — 2026-10-01

The previous completed-Fleet cut removed the substantial obsolete workflow.
This follow-up found small confirmed code residue, conflicting active design
instructions, obsolete scaffold assertions, and tests that freeze document or
source formatting. It did **not** establish another large, safely deletable
runtime subsystem. Large retained recovery, registry and funding modules have
current callers and authority responsibilities.

The most useful next batch is active-document contraction plus the small proven
residue. Reducing source-string tests is a separate batch where replacement
behavior or compiler evidence must preserve their meaningful invariants.

## 1. Run metadata

- Audit: `CANIC-MODULE-SURFACE-001`; reviewer: Codex `/root`, single review.
- Scope: follow-up to the maintainer-requested completed-Fleet cleanup; repository
  source, active documentation, tests and supporting scripts. Read-only code trace;
  only this report and its evidence manifest are new artifacts.
- Tier: 2 because sampled surfaces include deployment, generated endpoints,
  recovery and persisted authority. This is not a minor closeout or release gate.
- Run result: `partial`; result validity: `valid` for the named samples. The
  repository-wide search is a candidate screen, not exhaustive semantic review.
- Baseline commit: `02af7277664c1664a58bfaa5e6fde997474c9802`.
- Committed tree: `59c6be1f502c442bee929daf019347310601600e`.
- Committed product tree: `6c764ee0bff95865c0a12da62e2c3d252a41dc7c7503b831236fae89aa37f73f`.
- Dirty worktree: true. Concurrent deployment/package work continued during the
  audit. Candidate-file hashes bind the reviewed bytes in
  [the evidence manifest](module-surface-hardening.json); the committed product
  hash does not describe those uncommitted changes.
- Declared toolchain: Rust `1.98.1`; tools observed: ripgrep `15.2.0`, Perl `5.38.2`.
  No compiler, Cargo, Make, PocketIC, Wasm build or paid operation was executed.
  Compile target, runtime feature selection, fixture seed and performance baseline
  are N/A for this code-trace run.
- Generated code and test surfaces: sampled. Sibling repositories, live state,
  retained incident bytes and historical audit contents: excluded from mutation.
- Comparability: `non-comparable`; no quantitative prior MSH baseline is asserted.
- Method header version: `2.2`. Its required embedded manifest and STEP 0 still
  say `MSH-2.1`; this conflict is recorded below, not silently normalized.
- Taxonomies: `ST-1`, `AT-1`, `DC-1`; compatibility: `pre-1.0-hard-cut`;
  Wasm rule: `raw-wasm-primary`; hot-path model: `HP-1`;
  proof policy: `read-only-first`. Method and script fingerprints are in the manifest.

## 2. Step status

PASS means the stated sampled code-trace evidence exists; it does not certify
unsampled code or authorize deletion.

| Step | Status | Evidence | Comparability limit |
| --- | --- | --- | --- |
| 0 — metadata | PASS | Commit/tree/lock/method hashes and dirty state | Concurrent work; pinned candidate bytes only |
| 1 — retention inventory | PASS | Sections 3–4, root and facade exports, current callers | Sampled owner families |
| 2 — dead/stale signals | PASS | Findings S1–S7 | Identifier counts are leads, not deletion proof |
| 3 — authority drift | PASS | Section 6, current reset/recovery ownership | No live authority inspection |
| 4 — dead-surface complexity | PASS | Seal residue and historical planning payload | No second large dead runtime lane established |
| 5 — generated boundary | PASS | Section 7, endpoint macros and their tests | Expansion/Wasm execution not repeated |
| 6 — diagnostics/tests/features | PASS | S1, S3–S6 | No whole test-registry deletion proposed |
| 7 — removal plan | PASS | Section 8 | Reviewable follow-up, no implementation |
| 8 — runtime shape | PASS | Section 9, classified retained/current boundaries | No size or speed claim |
| 9 — risk score | PASS | Section 10 | Qualitative pressure for this sample only |

## 3. Evidence log

Working directory for every command: `/home/adam/projects/canic`. Evidence is
source trace and the compact manifest; disposable exploratory output is not a
new evidence authority. These successful commands reproduce the main findings:

```sh
git status --short
git rev-parse HEAD HEAD^{tree}
sha256sum Cargo.lock docs/audits/modular/module-surface-hardening.md scripts/ci/audit-product-tree-hash.sh
bash scripts/ci/audit-product-tree-hash.sh HEAD
rg --version
perl -e 'print "$^V\n"'
rg -n 'AuthoritySealContract|AuthoritySealHeadroom' crates scripts
rg -n 'sealed|ReinstallPreparation' crates/canic-host/src/fleet_ensure/model/mod.rs
rg -n '^#{1,4} ' docs/design/0.110-fleet-runtime-contraction/0.110-design.md
sed -n '705,731p' docs/design/0.110-fleet-runtime-contraction/0.110-design.md
sed -n '1545,1556p' docs/design/0.110-fleet-runtime-contraction/0.110-design.md
sed -n '1609,1620p' docs/design/0.110-fleet-runtime-contraction/0.110-design.md
sed -n '663,722p' crates/canic/tests/workspace_manifest.rs
sed -n '133,169p' crates/canic-cli/src/scaffold/tests.rs
sed -n '1022,1056p' crates/canic/tests/protocol_surface.rs
sed -n '1275,1338p' crates/canic/tests/protocol_surface.rs
sed -n '1774,1795p' crates/canic/tests/protocol_surface.rs
sed -n '115,144p' crates/canic/tests/reference_surface.rs
rg -n '\bplan_release_build_for_profile\(' crates scripts --glob '*.rs' --glob '*.sh'
rg -n '\brecord_application_metrics\b' docs/features crates --glob '*.md' --glob '*.rs'
rg -n 'method_version|Method version' docs/audits/modular/module-surface-hardening.md
```

Searches also covered legacy/compatibility names, public functions and error
variants across Rust sources, CLI consumers of Host owners, Root/Store wire
fragments, authority snapshot fences, intent snapshots, fixture grants,
PocketIC registration and historical ablation tooling. Candidate-screen Perl
scripts counted identifier occurrences; manual caller/cfg inspection rejected
false positives. In particular, excluding directories named `fixture` hid real
production fixture-delivery callers. That heuristic was not used as deletion
evidence. Some exploratory reads used nonexistent guessed paths; corrected
directory-module paths supplied the findings below.

## 4. Reachable surface and retention inventory

| Owner/surface | Observed current consumer or invariant | Disposition |
| --- | --- | --- |
| Host completed-Fleet reset | CLI and completed-reset PocketIC journey call current clean reinstall | RETAIN WITH OWNER |
| Host activation preparation/reinstall | CLI `plan_reinstall`, source-bound activation recovery journey | RETAIN WITH OWNER |
| Host capacity-import journals and certified request retirement | Same-operation paid-effect reconciliation; CANIC-188 | RETAIN WITH OWNER |
| Core/Control Plane authority snapshot fences | Root/Coordinator endpoint macros, timer suspension/resumption and restore ownership | RETAIN WITH OWNER |
| Component Registry membership seals | Initial publication/activation and current Root inventory | RETAIN WITH OWNER |
| Core intent snapshot helpers | Explicit `cfg(test)`; corruption/reconstruction and accounting tests | RETAIN WITH OWNER |
| Control Plane fixture grants | Component installation and pool recycling call validation/issuance/revocation | RETAIN WITH OWNER |
| Host observatory/state/component owners | Current `observatory`, `state`, `medic`, `component` and topology consumers | RETAIN WITH OWNER |
| Public metric publication API | Documented application-owned local publication contract, even without an in-repository application caller | RETAIN WITH OWNER |
| Partial Root Candid enums | Different request/response subsets for observation, provisioning and inventory | REJECT CLEANUP |
| Historical ablation patches | Hash-bound historical experiment runner and reproduction tests | RETAIN WITH OWNER |
| `scripts/dev/*`, CANIC-188 bundle, prior handoff archive | Intentional helpers and exact retained evidence | RETAIN WITH OWNER |

The 17,813-line PocketIC Fleet baseline, 9,952-line Host platform adapter and
7,412-line Host Fleet test file are size hotspots, not proven obsolete chunks.
Splitting files would improve navigation but would not remove behavior or
reduce protocol complexity. This audit does not count that as deletion.

## 5. Findings and candidates

All findings are open, first recorded on 2026-10-01, against the baseline and
source fingerprints above. Source audit ID is `CANIC-MODULE-SURFACE-001`.
Duplicate attribution, fix commit, validation commit and waiver are N/A; no
P0/P1 finding or release-blocking verdict is issued by this sampled review.

| Key / canonical ID | Class; severity; confidence | Owner and location | Disposition |
| --- | --- | --- | --- |
| S1 / `CANIC-0.110-HOST-SEAL-001` | documentation_drift; P3; confirmed | Host Fleet errors/model comments | DELETE NOW |
| S2 / `CANIC-0.110-DOCS-ACTIVE-001` | documentation_drift; P2; confirmed | Active 0.110 design and duplicated delivery instructions | NARROW NOW |
| S3 / `CANIC-0.110-TESTS-DOCS-001` | governance_conflict; P2; confirmed | Facade workspace feature-document test | NARROW NOW |
| S4 / `CANIC-0.110-TESTS-SCAFFOLD-001` | governance_conflict; P3; high | CLI scaffold assertions naming removed contracts | DELETE NOW |
| S5 / `CANIC-0.110-TESTS-SOURCE-001` | evidence_gap; P2; high | Facade source-string security/macro checks | PATCH WITH PROOF |
| S6 / `CANIC-0.110-HOST-SURFACE-001` | evidence_gap; P3; medium | Public Host constructors/planning convenience wrappers | DEFER WITH TRIGGER |
| S7 / `CANIC-0.110-AUDIT-METHOD-001` | audit_method_defect; P3; confirmed | MSH method version metadata | NARROW NOW |

### S1 — orphaned seal errors and stale preparation descriptions

`fleet_ensure/ops/current_protocol/mod.rs:226` defines
`AuthoritySealContract`; `fleet_ensure/policy/mod.rs:120` defines
`AuthoritySealHeadroom`. Each identifier occurs only at its declaration across
the current code/scripts. Both variants have ordinary fields, no `#[from]`
conversion, no serialization derive, and no remaining command path constructs
them. The completed-Fleet sealing owner was removed by the previous cut.

Model comments at lines 1047 and 1250 still describe preparation as sealing
authorities. The retained activation preparation owner is current recovery,
not the removed Host seal command. Delete the two error variants and correct
these descriptions; do not rename persisted phase identifiers or remove
Component Registry membership sealing. Expected cut is approximately 15 Rust
lines plus comment corrections. Verification: caller scan, Host Fleet narrow
compile/lint; no new anti-resurrection test.

### S2 — active design combines current contracts with superseded instructions

`0.110-design.md` is 1,649 lines. At lines 1609–1620, **Next Authorized Action**
directs implementation toward `.43` after `.42`; `.48` is published and the
current batch is `.49`. The FI1 release table at line 1553 still says
**In progress for .43** and ready for its release gate. The Status block at
lines 712–729 says the line adds no runtime capability and has no runtime
impact, while the maintained design now includes bootstrap/import and operator
features. The top amendments explain precedence, but the lower instructions
remain easy to retrieve independently as present authority.

The stopped B3 specification, deferred B4/macro work, earlier validation-delivery
sequencing and completed CR1 history can be separated from current obligations.
Archive historical planning with dates and preserve links; retain the accepted
B1/B2 decisions, bounded .47 amendment, FI1/current clean reinstall, funding,
B5 budgets and human closeout contract. Delivery cadence, release status and
next actions should have one current owner each. An editorial review could
move roughly 200–400 lines out of the active design; this is an estimate,
not a deletion count or permission to discard historical evidence.

The 1,956-line Fleet operator guide also repeats reset/recovery guidance across
the opening clean-reinstall sequence and later wipe/unreadable-plan sections.
Consolidate operator instructions around links to the authoritative sequence,
while retaining detailed recovery constraints that affect a real decision.
Verification: local links, current-document semantics and manual contract
comparison. Do not introduce prose/heading release guards.

### S3 — README layout is a release-tested contract

`crates/canic/tests/workspace_manifest.rs:666` parses the exact heading
`## Feature Contract\n`, an exact Markdown row shape and literal `Yes`/`No`
prefixes to enforce documented feature/default inventories. A harmless heading,
table or default-label edit can fail this test without any changed Cargo
feature or executable behavior.

The root agent policy explicitly excludes heading inventories and explanatory
formatting from release-blocking guards. Keep the existing Cargo feature/default
semantics tests and README presence/link checks; remove this layout parser or
replace it with maintained structured evidence if such enforcement is needed.
Approximately 60 lines are involved. Verification: narrow workspace-manifest
tests and a document-format variation that leaves the structured contract intact.

### S4 — scaffold tests preserve removed surface names

`crates/canic-cli/src/scaffold/tests.rs:145–164` asserts absence of
`auto_create`, `topup_policy`, `[[canisters]]` and `CanisterRole::new`.
These are predecessors, not the maintained scaffold contract. The same test
already parses the generated config and checks current roles/Component Specs.

Delete the obsolete absence assertions and strengthen only meaningful current
model assertions where needed. Keep workspace independence, dry-run, role
attachment and creation behavior. This is a small cut, around four assertions;
it prevents tests from continually putting removed forms back into agent context.
Verification: targeted scaffold tests, including generated config parsing.

### S5 — source spelling substitutes for stronger endpoint/macro evidence

Samples in `crates/canic/tests/protocol_surface.rs`:

- Root capability routing at line 363 checks exact Rust expression strings.
- Application-session protection at line 430 reads endpoint source attributes.
- Blob billing checks at lines 1022–1056 include explanatory wording and exact
  return-type spelling; non-billing checks at lines 1276–1338 parse attributes.
- Missing-finish diagnostics at line 1775 check a marker declaration as text.

`reference_surface.rs:117` also checks an exact comma-separated CDK re-export
list, so import formatting can change the result. Existing macro compile probes,
AST endpoint checks, Candid checks and PocketIC tests cover related invariants,
but the audit did not establish replacement coverage for every assertion.

Reduce these tests only after mapping the exact invariant: compiler/type checks
for exported names, structured Candid for methods/modes/types, compiler failure
evidence for missing `finish!`, and real guarded calls or AST checks for auth.
Do not delete a guard just because another test has a related name. Roughly
150–250 source-test lines merit review; retained/replacement tests may offset
the cut. This is test-maintenance pressure, not measured runtime savings.

### S6 — public convenience surfaces have no demonstrated production owner

`PayloadSchemaRefV1::experimental` at Host `evidence_envelope/mod.rs:89` has
no in-repository consumer. `release_build/mod.rs:140` and `:145` expose
`plan_release_build` and `plan_release_build_for_profile`; their callers are
tests and the convenience chain, while production selects profile **and
network** through `plan_release_build_for_profile_and_network`.

These are public Host APIs, so absence of an internal caller alone is insufficient
proof of obsolescence. Trigger: confirm the maintained Host contract and any
generated/downstream consumers. Then delete the unneeded constructor or move
test defaults to test support without retaining a compatibility wrapper.
Do not remove the experimental stability enum/schema or change production build
network selection based on this evidence. Potential saving is tens of lines.

### S7 — audit method declares two current versions

`docs/audits/modular/module-surface-hardening.md:6` declares method `2.2`,
but lines 186 and 445 require `MSH-2.1` in the report. This can give identical
reviews different method identities and undermine comparisons. Reconcile the
current method metadata while preserving truthful historical report versions;
check the audit-method catalog. This run records both values and the exact
fingerprint and claims no cross-run quantitative comparison.

## 6. Runtime authority drift

Confirmed drift is descriptive: removed Host seal errors and active historical
instructions. No second obsolete executable authority lane was established.
Ordinary authority snapshot sealing is not completed-Fleet sealing: Root and
Coordinator macros still route it through Core and Control Plane lifecycle
owners. Certified handoff retirement still reconciles genuinely unfinished
ingress; completed records remain history. Neither should be removed.

Fresh backup unavailability does not retire same-release restore/recovery.
The exact CANIC-188 exception remains separately governed. This report neither
qualifies its live repair nor changes the concurrent deployment audit's closure.

## 7. Facade and generated boundary

Facade tests combine typed DTO/Candid checks with brittle source assertions.
Only the latter are cleanup candidates, with S5 replacement proof required.
Hidden CDK exports support generated macros, even when they lack an ordinary
Rust application caller. Public metric publication has documented application
consumers. Fixture modules also implement current product delivery, not merely
test scaffolding.

Root wire-fragment enums in Host observation/current-protocol/component transport
and CLI subnet inventory contain different subsets. Sharing one giant enum could
increase coupling and Candid reachability. Similar type names are not proof of
redundant ownership; no consolidation is recommended without exact wire analysis.

## 8. Removal safety plan

1. First slice: S1 and S4; remove confirmed residue and preserve current behavior.
2. Documentation slice: S2 and S7; contract-preserving archive/contraction and
   one unambiguous method version. Keep `current.md` compact and preserve history.
3. Test-governance slice: S3; retain feature semantics and document presence.
4. Separately map and replace S5 evidence before deleting source-string checks.
5. Defer S6 until public-owner/consumer evidence resolves it.

For implementation, preserve concurrent edits, check shared `target/` ownership
before any compile, run only affected package/behavior checks, extend the open
`.49` changelog for meaningful code/test changes, and leave everything uncommitted.
These slices do not close the complete deployment batch or authorize broad gates.

## 9. Runtime shape and optimization risk

S1 is cold Host diagnostic residue; S2/S3/S4/S5/S7 concern documentation or tests.
No production storage, timer, policy, funding or transport graph changes are
proposed for immediate removal. S6 is cold public build/evidence support pending
owner proof. There is no claimed Wasm-byte, function-count or instruction saving.
Current query/encode paths and paid-effect recovery remain retained; changing
them would require a new proof/measurement scope.

## 10. Risk score

Sampled cleanup pressure: **3/10 — moderate**, predominantly active-document
confusion and brittle tests. This is not a whole-repository architecture score.

| Bucket | Candidate groups | Highest observed risk |
| --- | ---: | --- |
| Stale compatibility | 1 | Removed scaffold names retained in assertions |
| Stale generated fallback | 0 confirmed | No obsolete generated fallback established |
| Orphaned helper/diagnostic | 1 confirmed | Two old seal errors and descriptions |
| Overexposed internal | 1 unresolved | Test-default/public helper ownership |
| Duplicate surface | 2 | Active design instructions and document/source test coupling |
| Unclear | 1 | Public consumer proof for S6 |
| Optimization-risk cleanup | 0 proposed | Current hot/runtime boundaries retained |

Buckets overlap; they are not a sum of independent savings or findings.

## 11. Verification readout

The JSON manifest parses, all 13 recorded source fingerprints still match, and
all seven finding identities are unique. `git diff --check` passes.
`bash scripts/ci/check-current-document-semantics.sh` passes with the two existing
advisory layout warnings for the 0.110 design directory and its retained incident
repair document. These are document/source checks, not runtime qualification.

Source/caller/cfg evidence supports the immediate queue. No code or tests were
changed, and no new compilation or runtime qualification is claimed. Earlier
cleanup qualification remains evidence for its recorded source state only.
Hashes in the manifest bind these sampled findings; changes to a candidate file
require rereading it before implementing the cut. Global search does not prove
all public APIs unused, all tests redundant, or the worktree push-ready.

## 12. Disposition summary

- DELETE NOW: S1 and S4 — small confirmed obsolete residue.
- NARROW NOW: S2, S3 and S7 — active instructions, prose gate and method metadata.
- PATCH WITH PROOF: S5 — preserve meaningful endpoint/compiler/auth evidence.
- DEFER WITH TRIGGER: S6 — public Host ownership/consumer confirmation.
- RETAIN/REJECT CLEANUP: current authority, paid recovery, snapshot fences,
  Component membership seals, historical evidence and distinct wire subsets.

## 13. Follow-up

Prioritize the active-document and proven-residue slices. They address the
maintainer's agent-confusion concern without reopening Fleet recovery design.
Expect a modest code cut and a larger reduction in active planning/document
surface. Another 16,000-line deletion is not supported by this audit's evidence.
