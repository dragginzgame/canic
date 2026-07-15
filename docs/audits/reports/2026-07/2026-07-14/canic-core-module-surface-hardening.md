# Canic Core Module Surface Hardening

## Verdict

- Run result: `fail`.
- Result validity: `valid`.
- Comparability: `non-comparable: first frozen MSH-2.0 run`.
- Audit tier: `Tier 2`.
- Patch mode: `read-only`.
- Dead-surface risk: 4 / 10.

The published `canic-core` surface is mostly justified by the facade,
generated endpoint/build wiring, the control-plane crate, and host/operator
consumers. No obsolete runtime fallback, duplicate execution path, public
stable record, or test-only production dependency was found.

Two bounded hard-cut cleanup areas remain:

1. the already indexed root proof batch request/outcome are internal
   orchestration state exposed through public Candid/Serde DTOs, and three
   outcome variants are never constructed by the production issuer-call path;
   and
2. `canic_core::error` is a hidden-but-public root path with no current
   cross-crate consumer. The real sibling contract already uses the narrow
   `control_plane_support::error` bridge.

The first area confirms `CANIC-092-LAYERING-004`. The second is new finding
`CANIC-092-SURFACE-001`. Product code remains unchanged pending Phase C
baseline review.

## Method Manifest

```text
method_version: MSH-2.0
surface_taxonomy: ST-1
authority_taxonomy: AT-1
deletion_confidence_model: DC-1
compatibility_policy: pre-1.0-hard-cut
wasm_signal_rule: raw-wasm-primary
hot_path_risk_model: HP-1
proof_policy: read-only-first
```

## Run Metadata

| Field | Value |
| --- | --- |
| `baseline_report` | `N/A`; first frozen MSH-2.0 run |
| `comparability_status` | `non-comparable` |
| `code_snapshot` | `v0.92.0` / `91736337fc1cfeb891f17d7d62affb5e671348e2` |
| `in_scope_roots` | `crates/canic-core/src`; direct facade/generated/sibling consumers under `crates/canic`, `canic-macros`, `canic-control-plane`, `canic-host`, `canic-cli`, tests, canisters, and fleets |
| `excluded_roots` | historical documents and changelogs except as non-authoritative provenance; generated target output; unrelated sibling repository code |
| `generated_code_inclusion` | sampled through macro expansion source and facade hidden-boundary consumers |
| `test_surface_inclusion` | sampled for public-surface pins and test-only visibility |
| `audit_tier` | `Tier 2` because `canic-core` owns facade, generated, stable-state, lifecycle, and Wasm-sensitive support |
| `patch_mode` | `read-only` |
| reviewer | Codex, single reviewer; no new P0/P1 manual finding requires second-review approval |

## Run Identity

```text
release_anchor: v0.92.0
source_commit_full: 91736337fc1cfeb891f17d7d62affb5e671348e2
source_tree_hash: fd31bb8289365a38f2bea7f8ebd6973908ee959f
product_tree_hash: c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e
clean_worktree: false; report/status-only Phase C changes, product tree unchanged
cargo_lock_hash: 6cd75f146077bbf3f254fda608f1265531d1065ce0cd9c1bb56d67118f3de5cc
rust_toolchain: rustc 1.97.0; cargo 1.97.0
target_triple: x86_64-unknown-linux-gnu
feature_set: canic-core all features for focused compile; package defaults for focused tests
audit_method_id: CANIC-MODULE-SURFACE-001
audit_method_version: 2.0
audit_method_fingerprint: 404a359b4448ea7288055f0444e3178ae972f4eb7e1a0814aa693ce67df59030
audit_script_hashes: N/A; manual code trace with exact commands retained below
external_tool_versions: Cargo/Rust 1.97.0; Git 2.43.0; ripgrep 15.1.0; GNU find/wc
fixture_or_seed: canic-core crate root, public/hidden exports, proof-install DTO lane,
  facade/macro/control-plane consumers, cfg/test and stale-signal scans
environment_class: offline local code trace; tracked source read-only
started_at: 2026-07-14T17:42:00Z
completed_at: 2026-07-14T18:05:20Z
```

## Evidence Manifest

```text
command: crate-root public/hidden inventory; repository-wide direct-consumer
  scans; stale/dead-signal scan; facade/macro/control-plane boundary inspection;
  proof-install DTO construction and match trace; targeted package check and tests
working_directory: Canic repository root
exit_code: 0 for retained commands and focused validation
stdout_path: not_retained; normalized results are recorded in this report
stderr_path: not_retained; no validation failure
baseline_identity: v0.92.0 / 91736337fc1cfeb891f17d7d62affb5e671348e2
method_identity: CANIC-MODULE-SURFACE-001/v2.0 / 404a359b...
tool_versions: Cargo/Rust 1.97.0; Git 2.43.0; ripgrep 15.1.0
timestamps: 2026-07-14T17:42:00Z to 2026-07-14T18:05:20Z
artifact_hashes: method 404a359b...; source tree fd31bb82...
retention_class: primary Markdown only
redactions_applied: repository-relative paths only
```

## Step Status

| Step | Status | Evidence artifact | Comparability impact |
| --- | --- | --- | --- |
| STEP 0 | PASS | run metadata and immutable identities above | first frozen run |
| STEP 1 | PASS | crate-root and repository consumer inventory | none |
| STEP 2 | PASS | dead/stale signal and proof-outcome construction scans | none |
| STEP 3 | PASS | DTO, storage, replay-policy, support-boundary traces | none |
| STEP 4 | PASS | candidate/variant complexity table | none |
| STEP 5 | PASS | facade, macro, generated, and sibling boundary review | none |
| STEP 6 | PASS | feature/cfg/test/diagnostics review | none |
| STEP 7 | PASS | read-only removal safety plan | none |
| STEP 8 | PASS | runtime-shape classifications | none |
| STEP 9 | PASS | ST-1 risk buckets and 4/10 score | none |

## Evidence Log

| Evidence | Command / inspection | Result | Artifact |
| --- | --- | --- | --- |
| public surface inventory | `sed -n '1,280p' crates/canic-core/src/lib.rs`; `rg -n '^pub mod |^pub\(crate\) mod |^mod |^#\[doc\(hidden\)\]' .../lib.rs` | 19 public root modules, 10 hidden markers including `__reexports`, and nine crate-private execution roots | this report |
| source breadth | `find crates/canic-core/src -type f -name '*.rs'`; `wc -l` | 546 Rust files and 95,579 physical lines including tests/comments | this report |
| stale signal scan | exact scans for dead/unused suppressions, legacy, compatibility, shim, deprecated, temporary, and fallback | no dead/unused suppression; only current auth/HTTP fallback wording and two non-surface words | this report |
| proof DTO trace | exact searches for `RootDelegationProofBatchInstallRequest` and `RootDelegationProofInstallOutcome` across core, facade tests, canisters, and fleets | request is internal batch carrier plus protocol pin; outcome is internal-only; three variants have no production constructor | this report |
| direct error consumer check | repository-wide scan for `canic_core::error` and `canic::__internal::core::error`, excluding docs/target | zero current code consumers | this report |
| current error bridge | inspection of `control_plane_support.rs` and all control-plane imports | sibling crate consistently consumes `control_plane_support::error` | this report |
| facade/generated boundary | inspection of `canic/src/lib.rs`, `canic/src/macros/**`, and `canic-macros/src/endpoint/**` | hidden core alias and access/dispatch/bootstrap/ingress paths have current generated consumers | this report |
| replay owner check | repository-wide `replay_policy::` trace and control-plane deployment inspection | core workflows own manifests/types; control plane uses canonical `CostClass`; retain public owner | this report |
| focused compile | `cargo check --locked -p canic-core --all-features` | PASS | terminal output not retained |
| focused behavior | `cargo test --locked -p canic-core --lib workflow::runtime::auth::provisioning::tests` | PASS, 3 tests | terminal output not retained |
| public protocol pin | `cargo test --locked -p canic --test protocol_surface root_delegation_proof_batch` | PASS, 2 tests; confirms internal request remains publicly serialized | terminal output not retained |

## Reachable Surface And Retention Inventory

`canic-core` documents 598 root-reachable items in the independently frozen
module-structure report. MSH classifies the roots by current owner rather than
assuming every `pub` item is a stable end-user API.

| Item | Kind | Path | Visibility | Feature/Cfg | Consumer evidence | Consumer should exist? | Authority reason | Surface class | Owner | Disposition | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| normal facade/core roots | modules | `api`, `cdk`, `dto`, `ids`, `log`, `memory`, `perf`, `protocol` | `pub` | mixed/current | facade, CLI/host, control-plane, canisters, tests | yes | low-level runtime and public facade contracts | live-authority | owning root modules | RETAIN WITH OWNER | low-medium |
| replay inventory | module | `replay_policy` | `pub` | all | core workflows plus control-plane `CostClass` | yes | canonical replay/cost manifest owner across runtime crates | live-authority | replay policy | RETAIN WITH OWNER | low |
| generated facade support | modules | `access`, `bootstrap`, `dispatch`, `ingress`, `__reexports` | hidden `pub` | build/target/test dependent | facade and proc-macro expansion source, ctor hook | yes | cross-crate generated wiring | live-generated-boundary | facade/macro boundary | RETAIN WITH OWNER | low |
| sibling support | modules | `control_plane_support`, `role_contract`, `shared_support`, `state_contract` | hidden `pub` | mixed | control-plane, host, CLI, build/test support | yes | named cross-crate support and static contract owners | live-authority | respective support contract | RETAIN WITH OWNER | low-medium |
| core error root path | module | `error` | hidden `pub` | all | no direct external path; internal code and hidden bridge only | no | type is live, root path has no authority reason | overexposed-internal | internal error model | NARROW NOW | low |
| active proof public contracts | DTOs | proof/ref/status/policy/renewal DTOs | `pub` | auth | current root/issuer endpoints and operator projections | yes | real Candid/operator contracts | live-authority | endpoint DTO boundary | RETAIN WITH OWNER | medium |
| batch install request/outcome | DTO/internal carrier | `dto/auth/renewal.rs:36-53` | `pub`, Candid, Serde | auth | workflow/ops plus public protocol pin; no endpoint owns batch request/outcome | no | internal orchestration state | overexposed-internal | internal auth plan/outcome model | NARROW NOW | medium |
| test support | module/helpers | `test`, cfg-test fixtures/reset helpers | `pub` only under `cfg(test)` or effective crate-private roots | test | canic-core unit tests | yes | test seams do not enter downstream dependency builds | live-test-support | core unit tests | RETAIN WITH OWNER | low |
| constants and macros | constants/macros | crate root, memory/log/perf macros | `pub` | mixed | facade, build, canister source, generated code | yes | stable low-level and macro-expansion contracts | live-authority / live-generated-boundary | core/facade | RETAIN WITH OWNER | low |

## Dead / Stale Candidate Table

| Candidate | File | Lines | Signal | Current consumers | Consumer should exist? | Authority reason | Surface class | Deletion confidence | Disposition | Risk if removed |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `RootDelegationProofBatchInstallRequest` public/serialized form | `dto/auth/renewal.rs` | 32-39 | internal carrier derives public wire traits and is pinned by facade integration test | auth ops/workflow; one protocol pin | internal carrier yes, public wire no | no endpoint or operator contract owns it | overexposed-internal | high | NARROW NOW | low if moved atomically to internal owner |
| `RootDelegationProofInstallOutcome` public/serialized form | `dto/auth/renewal.rs` | 42-53 | internal install outcome is re-exported from public auth DTO | provisioning and batch failure recorder | internal outcome yes, public DTO no | internal result classification | overexposed-internal | high | NARROW NOW | medium because persisted diagnostic text uses `Debug` formatting |
| `AlreadyInstalled`, `ProofMismatch`, `ExpiredOrSuperseded` | same enum | 48, 51-52 | no production issuer-call constructor; `AlreadyInstalled` only has a match arm | success match or none | no under current response mapping | historical/anticipated vocabulary only | orphaned-helper | high | DELETE NOW | low after focused behavior and diagnostics update |
| `canic_core::error` root path | `lib.rs:33-34`, `error.rs` | root plus 3 public types | hidden module remains publicly addressable but direct consumer scan is empty | internal imports; types re-exported through support bridge | internal model and bridge yes, root path no | none for direct root visibility | overexposed-internal | high | NARROW NOW | low; Rust hard cut only |

Historical changelog text that preserved the DTO wire shape or replay exports is
context, not current authority. Replay policy is retained because it has a
current cross-crate canonical-owner role. The proof batch request/outcome does
not: the real wire operations are the root proof-return endpoint and the
issuer-local `InstallActiveDelegationProofRequest` endpoint.

## Runtime Authority Drift

| Area | Runtime authority | Alternate authority found? | Evidence | Allowed role? | Finding | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| stable runtime state | internal model/storage/record plus ops conversion | no public record owner | module-structure surface map and root inspection | yes | none | low |
| root proof installation | workflow orchestration plus auth ops/state | yes, public DTO implies a boundary contract for an internal batch carrier/outcome | full construction/call/match trace | no | `CANIC-092-LAYERING-004` | medium |
| error ownership | internal error model; support facade for control-plane | duplicate public root reachability | zero direct external root consumers | no | `CANIC-092-SURFACE-001` | low-medium |
| replay/cost policy | `replay_policy` manifests/types | no competing owner | core workflow and control-plane `CostClass` imports | yes | retain | low |
| generated endpoint/build wiring | facade hidden roots and macro expansion | no obsolete fallback | direct macro source paths | yes | retain | low |
| public core architecture text | `AGENTS.md` fixed layer contract | README/lib docs still describe domain-before-ops flow | direct documentation inspection | no | confirms `CANIC-092-DOCS-001`; no duplicate | medium governance |

## Complexity Retained Only By Surface

| Module | Complexity signal | Retention justification | Dead-surface link | Public/hidden items | Current consumers | Shrink action | Disposition | Blast radius | Risk |
| --- | --- | --- | --- | ---: | --- | --- | --- | --- | --- |
| auth renewal DTO/provisioning | six public outcome variants and a public batch carrier | actual proof/status/policy DTOs remain valid | three variants and both public/internal classifications are unnecessary | 2 candidate types, 3 dead variants | one workflow, auth ops, protocol pin | internal named types; remove wire derives/re-export/pin; delete dead variants | NARROW/DELETE NOW | core auth plus one facade test | medium |
| root error module | 3 public types reachable through two paths | types remain necessary internally and through support bridge | direct root path has no consumer | 1 module / 3 types | internal plus bridge | make root module private; keep canonical bridge | NARROW NOW | core/control-plane compile surface | low |
| replay policy | 21 public declaration/re-export lines | current canonical replay/cost owner | none | 21 lines across manifest/types | core and control-plane | none | RETAIN WITH OWNER | none | low |

## Facade / Generated Boundary Review

| Surface | Boundary type | Generated consumer evidence | Could narrow? | Required replacement | Deletion confidence | Disposition | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `canic::__internal::core` | hidden facade alias | start/endpoint/timer macros and proc-macro expansion use it broadly | not safely within this slice | explicit generated-support tree plus expansion proof | low | RETAIN WITH OWNER | medium blast radius |
| core `access` / `dispatch` | hidden generated endpoint support | proc-macro access evaluation and dispatch paths | no current reason | N/A | low | RETAIN WITH OWNER | high if removed |
| core `bootstrap` / `ingress` | hidden build/lifecycle support | `build!`, `start!`, host, and facade constant consumers | no current reason | N/A | low | RETAIN WITH OWNER | high if removed |
| core `__reexports` | hidden macro support | external ctor path for memory test bootstrap | no current reason | alternative macro-owned ctor path | low | RETAIN WITH OWNER | medium |
| `control_plane_support` | hidden sibling support | direct control-plane imports | individual entries only after owner trace | per-owner public/private bridge | low | RETAIN WITH OWNER | medium |
| direct core `error` root | hidden ordinary module | none | yes | no replacement; support bridge already exists | high | NARROW NOW | low |

The whole-core hidden alias is broad, but it does not by itself create a second
runtime authority: macros need a stable cross-crate path and the underlying
core package is already published. A future macro-support redesign would be
new scope unless a concrete public-root cleanup cannot be completed without
it.

## Feature / Diagnostics / Test Surface

| Surface | Feature/Cfg | Production consumer? | Test/diagnostics consumer? | Visibility could narrow? | Action | Disposition | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| auth create/verify implementations | named auth features | yes when enabled | broad unit coverage | not mechanically without feature redesign | none | RETAIN WITH OWNER | medium |
| blob storage and sharding surfaces | named features | yes when enabled | unit/PocketIC coverage | no stale feature found | none | RETAIN WITH OWNER | medium |
| `perf` recorder and macro | all; reset under test | yes for checkpoints | instruction audit and unit reset | production recorder must remain public for facade macro | none | RETAIN WITH OWNER | low |
| core `test` and auth fixtures | `cfg(test)` | no downstream production build | core unit tests | keyword could narrow but creates no published surface | no material cleanup | RETAIN WITH OWNER | low |
| fallback-labelled auth metrics | all | yes | metric tests | no; these describe current identity semantics, not compatibility fallbacks | none | RETAIN WITH OWNER | low |

## Removal Safety Plan

| Candidate | Action | Disposition | Owner boundary | Hotness | Required proof | Focused validation | Raw Wasm relevant? | Follow-up trigger |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| batch install request | move to named internal model/ops carrier; remove public re-export and Candid/Serde pin | NARROW NOW | auth install plan | warm / wasm-sensitive | endpoint/DID inventory proves it is not a wire payload | core provisioning/batch tests; facade protocol-surface update; retained DID comparison | secondary, expected neutral/smaller | approved finding-backed slice after baseline review |
| install outcome | move to internal typed outcome and preserve typed diagnostics | NARROW NOW | auth install result model | warm / wasm-sensitive | no public endpoint/stable contract; typed failure projection retained | provisioning, batch retry/success, diagnostics tests | secondary, expected neutral/smaller | same slice |
| three dead variants | delete and remove unreachable arms/vocabulary | DELETE NOW | internal outcome model | warm / wasm-sensitive | constructor and match scan plus focused tests | chain-key batch and root provisioning tests | secondary, expected smaller | same slice |
| direct error root path | change root module visibility to private; retain support re-export | NARROW NOW | internal error model / control-plane support bridge | compile-time only | core, control-plane, facade compile and package surface comparison | targeted check/rustdoc/package proof | no runtime-shape change | approved core-surface hard-cut slice |

No compatibility alias, deprecated path, fallback, or anti-resurrection test
should accompany either hard cut. Release notes should name the removed Rust
paths. A generated Candid comparison should prove that no actual endpoint
interface changed when the internal batch DTO pin is removed.

## Runtime Shape / Optimization Risk

| Candidate | Hotness | Runtime shape today | Proposed shape | Risk signal | Required proof | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| proof batch carrier/outcome | warm / wasm-sensitive | owned internal vectors and enum; Candid/Serde implementations retained in Wasm | same carrier/result ownership with fewer variants and no unnecessary wire implementations | persisted failure currently formats `Debug`; avoid losing useful typed cause | focused auth tests and diagnostic assertion; raw Wasm only as secondary evidence | PATCH WITH PROOF |
| direct error root visibility | compile-time only | one public module path plus hidden support re-export | private root module plus unchanged support re-export | no allocation, dispatch, or data movement | targeted package checks | NARROW NOW |
| generated core alias | wasm-sensitive generated boundary | direct static paths, no allocation | no change proposed | replacement could grow re-export/generic surface | design and measurement before any redesign | RETAIN WITH OWNER |

## Risk Score

| Bucket | Count | Highest risk | Notes |
| --- | ---: | --- | --- |
| stale compatibility | 0 | none | historical preservation language is not counted where a current owner exists |
| stale generated fallback | 0 | none | sampled generated paths are live |
| orphaned helper | 3 | low-medium | three production-unconstructed install outcomes |
| overexposed internal | 3 | medium | batch request, install outcome, and direct error root path |
| duplicate surface | 0 | none | direct error path is classified by overexposure, not double-counted |
| unclear | 0 | none | all candidates have named owners and consumers |
| optimization-risk cleanup | 1 | medium | auth outcome cleanup must preserve useful failure diagnostics |

Risk 4/10 reflects a moderate, bounded cleanup queue. No obsolete authority or
fallback currently changes runtime behavior, so a high/critical score is not
supported.

## Verification Readout

- `cargo check --locked -p canic-core --all-features`: PASS.
- Core provisioning unit filter: PASS, 3/3.
- Facade root-delegation protocol filter: PASS, 2/2.
- Direct `canic_core::error` / hidden-facade error path consumer scan: zero.
- Public proof batch request remains deliberately Candid-roundtripped by the
  facade test despite having no endpoint ownership.
- Production issuer result construction is limited to `Installed`,
  `RejectedBySigner`, and `CallFailed`; `AlreadyInstalled` is only matched,
  while `ProofMismatch` and `ExpiredOrSuperseded` have no production use.
- No patch, interface regeneration, stable-state change, Wasm measurement, or
  broad workspace gate was performed.

## Disposition Summary

Counts below are classified decision rows, with the three dead enum variants
counted individually because each preserves vocabulary and match surface.

| Disposition | Count |
| --- | ---: |
| DELETE NOW | 3 |
| NARROW NOW | 3 |
| INLINE NOW | 0 |
| MOVE OWNER | 0 |
| MOVE TO TEST | 0 |
| RETAIN WITH OWNER | 7 grouped surface families |
| RETAIN HOT PATH | 0 |
| MEASURE FIRST | 0 |
| PATCH WITH PROOF | 1 grouped auth cleanup |
| REJECT CLEANUP | 0 |
| BLOCKED | 0 |

## Findings

### `CANIC-092-LAYERING-004` - Internal proof-install state is accidental public DTO surface

This run independently confirms the existing finding. It adds MSH evidence
that the batch request is not an endpoint payload, the outcome is internal-only,
and the facade protocol test actively pins the unnecessary serialized request
surface. The canonical finding remains owned by the auth install plan/outcome
model; no duplicate ID is created.

### `CANIC-092-SURFACE-001` - Internal error module has an unnecessary public root path

```text
finding_class: product_defect
severity: P2
confidence: confirmed
finding_status: open
owner: canic-core internal error model and control-plane support boundary
current_location: crates/canic-core/src/lib.rs:33-34;
  crates/canic-core/src/error.rs:22-208;
  crates/canic-core/src/control_plane_support.rs:1-3
intended_owner: private core error module with the existing hidden
  control_plane_support::error re-export for the sibling runtime crate
affected_surfaces: public Rust path canic_core::error and the transitive hidden
  canic::__internal::core::error path; no Candid, JSON, stable-state, CLI, or
  runtime behavior
source_audit_ids: CANIC-MODULE-SURFACE-001/v2.0
source_method_fingerprints: 404a359b4448ea7288055f0444e3178ae972f4eb7e1a0814aa693ce67df59030
baseline_commit: 91736337fc1cfeb891f17d7d62affb5e671348e2
first_observed_at: 2026-07-14T18:05:20Z
typed_cause_or_invariant: hidden documentation status does not make a Rust
  module private; every current cross-crate error consumer already uses the
  named control-plane support bridge, so direct root reachability has no
  canonical authority
risk: accidental public Rust surface invites unsupported direct dependencies
  and leaves two routes to the same internal types before 1.0
verification_status: repository-wide production/test source scan found no
  direct core/facade error-root consumer; internal and control-plane bridge
  consumers were traced; focused all-feature core compile passes
duplicate_of: none; DOCS-002 covers one broken link, while this finding owns
  actual Rust visibility
recommended_slice: make the root error module private, retain the one existing
  control-plane support re-export, update the separate broken rustdoc link,
  and record the hard cut without an alias or deprecated path
fix_commit: none
validation_commit: none
waiver: none
disposition: await complete Phase C baseline review and maintainer acceptance
```

## Follow-Up Actions

1. Deduplicate this report into the Phase C finding index, adding only
   `CANIC-092-SURFACE-001` and another source for `CANIC-092-LAYERING-004`.
2. Do not patch either candidate until the complete baseline is reviewed.
3. If approved, execute the DTO/model hard cut and the error-root visibility
   hard cut as separate, causal slices with focused parent comparisons.
4. Record Rust public-surface removals and confirm actual generated Candid is
   unchanged.

## Unreviewed Boundaries

- This audit classified the crate-root and material hub/public candidates; it
  did not attempt a line-by-line dead-code proof for all 546 Rust files.
- Macro consumers were inspected from source, not compiler-expanded output.
  Their live paths are already covered by focused macro and module-structure
  evidence; no generated path was recommended for deletion.
- Raw Wasm/instruction deltas were not measured because their separately
  frozen methods are blocked and this run made no patch.
- The full stable-state recovery and ten mandatory product traces remain
  outside this module-surface report and are not implied to pass.
