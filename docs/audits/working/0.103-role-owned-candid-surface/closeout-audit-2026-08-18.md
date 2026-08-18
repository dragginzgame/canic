# Canic 0.103 Closeout Audit

Date: 2026-08-18

## 1. Verdict

**HOLD 0.103 CLOSEOUT**

Publication boundary:

**NOT RELEASE-READY FOR ADDITIONAL REASONS**

Highest unresolved severity: **P0**. Two accepted security/correctness
boundaries are not implemented:

- several Root and Coordinator variants enter protected workflows, inspect
  durable state, or issue a Store query before establishing the variant's
  caller authority; and
- ordinary host/CLI calls do not require or verify the exact Candid/profile
  binding before transport.

There are additional P1 blockers in compile-time pruning and release identity.

## 2. Audited Source Identity

- Repository: `/home/adam/projects/canic`
- Branch: `main`
- Audited HEAD: `48043c836da53fdc319c673c53c0a707536a9867`
- Parent: `cc28ce837e323d07399e72f906736f2d38350d3a`
- Commit date: `2026-08-18T10:43:17+02:00`
- Subject: `0.103.0`
- State: clean; no staged, unstaged, or untracked files
- Tracked-diff SHA-256:
  `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- Staged-diff SHA-256: same empty hash
- Submodules: none
- Predecessor: `v0.102.2` ->
  `8cf4723cecd7579cbe3304b980c63b1bc3969d68`
- Existing lightweight `v0.103.0`:
  `721783675c2e7dc0981d7fa7639f654b84593df7`
- Package-version authority: root `Cargo.toml`, currently `0.102.2`
- Toolchain: `rustc 1.97.1`, `cargo 1.97.1`

The baseline was rebuilt in a disposable clean checkout of exact `v0.102.2`.
Candidate builds and generated comparisons used a disposable clone pinned to
`48043c8`.

Active authority:

- `docs/design/0.103-role-owned-candid-surface/0.103-design.md`
- `docs/design/0.103-role-owned-candid-surface/status.md`
- `docs/status/current.md`
- `docs/audits/working/0.103-role-owned-candid-surface/README.md`
- `docs/audits/working/0.103-role-owned-candid-surface/b3-profile-pruning.md`
- `docs/audits/working/0.103-role-owned-candid-surface/b6-surface-report.md`
- `docs/audits/working/0.103-role-owned-candid-surface/b7-closeout.md`
- `docs/changelog/0.103.md`
- `docs/design/0.104-ic-timers-consumer-hard-cut/status.md`

The B1 method TSVs and prior changelog entries were treated as historical
evidence, not live protocol authority.

## 3. Executive Summary

The generated surface substantially establishes:

```text
roles own methods
workflows own phases
```

The immutable baseline reproduced exactly:

| Owner | Canic | External | Application | Total |
| --- | ---: | ---: | ---: | ---: |
| Root | 117 | 1 | 6 | 124 |
| Managed auth | 23 | 1 | 10 | 34 |
| Coordinator | 24 | 0 | 0 | 24 |
| Store | 24 | 1 | 0 | 25 |
| **Total** | **188** | **3** | **16** | **207** |

The representative current surface is genuinely ten Canic-owned appearances:

```text
Root          2
Managed       2
Coordinator   2
Store         4
             --
             10
```

However, the candidate does not fully establish:

```text
callers own authority
capabilities own variants
```

Authorization is sometimes delayed until after protected work begins,
ordinary operator calls lack exact pre-call profile binding, and Store-only
protocol constants/replay material remain in non-Store Wasm.

## 4. Findings

### P0

#### P0-1 - Variant authorization occurs after protected workflow/state access

Evidence:

- The Root endpoint classifies only a subset of variants as controller commands
  at `crates/canic/src/macros/endpoints/root.rs:185`. `ProvisionChild`,
  `ProvisionComponents`, `ProvisionPeer`, `SynchronizeComponentDirectories`,
  and signer `PrepareRoleAttestation` then dispatch without an endpoint-level
  authority decision at lines 400, 422, and 574.
- Peer allocation reads Root/prepared state and queries Store before validating
  the requester at
  `crates/canic-control-plane/src/workflow/component_registry/mod.rs:671`.
- Child allocation follows the same ordering at
  `crates/canic-control-plane/src/workflow/component_registry/mod.rs:1244`.
- Coordinator acknowledgement reads current state and tests global joining
  state before validating the joining Root at
  `crates/canic-control-plane/src/ops/fleet_coordinator/mod.rs:351`.
- Coordinator Registry status reads the current registry before participant
  authorization at
  `crates/canic-control-plane/src/ops/fleet_coordinator/mod.rs:340`.

Observed behaviour: an untrusted caller can select protected variants and reach
state-dependent logic-and for Root provisioning, a Store query-before its exact
authority is established.

Accepted contract: variant authority must reject before workflow or
sensitive-state access; endpoints own authentication.

Practical impact: protected-state/error oracles and unauthorized resource
consumption are possible. No unauthorized durable mutation was demonstrated,
but the audit's explicit P0 boundary is already crossed.

Smallest correction: derive and enforce each variant's exact authority
immediately after bounded decode, then delegate. Lower layers may retain
defensive invariant checks but must not be the first authorization boundary.

Changes: runtime, endpoint tests, and authorization tests.

#### P0-2 - Exact profile binding is not selected before ordinary host/CLI calls

Evidence:

- CLI selection is only an optional filesystem path keyed by environment and
  role at `crates/canic-cli/src/support/candid.rs:5`.
- Missing root, missing role, and missing sidecar intentionally return `None` at
  `crates/canic-cli/src/support/candid.rs:31`.
- `RegistryEntry` carries no Candid hash or profile digest at
  `crates/canic-host/src/registry/mod.rs:11`.
- Inspect sends `canic_status` with this optional path at
  `crates/canic-cli/src/inspect/mod.rs:318`.
- List/readiness/version paths likewise proceed with optional bindings at
  `crates/canic-cli/src/list/live/mod.rs:145` and line 216.
- Overview parsing returns only `canic_version`; it discards the reported
  profile digest at `crates/canic-host/src/canic_metadata/mod.rs:37`.
- Generic host transport invokes typed calls without any target-profile
  evidence at `crates/canic-host/src/canister_protocol/mod.rs:81`.

Observed behaviour: build/release metadata contains profile hashes, but
ordinary host/CLI observation does not require them. A call can be attempted
with missing, stale, or merely role-named Candid.

Accepted contract: external immutable metadata selects the exact full binding
before the first call; missing or mismatched evidence must fail before
transport; Overview is verification-only.

Practical impact: wrong-profile calls and decoding can occur before mismatch
detection. This is one of the audit's explicit P0 examples.

Smallest correction: introduce one exact binding resolver fed by protected
release/Directory metadata. It must verify release, role, ordered capabilities,
Candid SHA-256, and profile digest before any call. Overview should compare the
returned digest only after that selection.

Changes: host, CLI, binding metadata, generated bindings/tests, and possibly
Directory projections.

### P1

#### P1-1 - Store-only protocol/replay material survives in non-Store Wasm

Evidence:

- Store lane constants are globally defined at
  `crates/canic-core/src/protocol.rs:5`.
- The global replay manifest includes both Store lanes for every consumer at
  `crates/canic-core/src/replay_policy/endpoint_manifest.rs:13`.
- Generic prepared-nonroot policy admits both Store lanes at
  `crates/canic-core/src/domain/policy/pure/fleet_activation.rs:28`.
- Artifact scans found both lane names in managed Component, Root, Coordinator,
  and Store Wasm. Only Store exports them.

Observed behaviour: generated Candid and handlers are pruned correctly, but
protocol constants, replay rows, and related policy reachability are not
manifest-exact.

Accepted contract: pruning removes unavailable variants, DTOs, referenced
Candid types, dispatcher branches, protocol constants, public handlers, and
endpoint-reachable workflows.

Practical impact: no extra method is exported, but negative profiles retain
protocol identity and replay/policy material that the release claims absent.

Smallest correction: split ordinary-role and Store replay/policy manifests,
and compile Store lane constants/policy only into the Store and exact callers
that genuinely need them.

Changes: runtime, Wasm, replay tests, pruning tests, generated evidence.

#### P1-2 - Existing tag and package authority cannot identify this candidate

Evidence:

- Root version is `0.102.2` at `Cargo.toml:59`.
- Existing `v0.103.0` resolves to `721783675...`, not audited HEAD
  `48043c836...`.
- Current status accurately records the conflict at
  `docs/status/current.md:17`.

Observed behaviour: current generated profile identities embed release identity
`0.102.2`; the existing lightweight `v0.103.0` names an earlier commit.

Accepted contract: one unambiguous release identity; current candidate must
not be published under a stale tag or package version.

Practical impact: publishing now would misidentify both source and generated
profile identities.

Smallest correction: after runtime closeout, the maintainer must reconcile or
remove the stray local tag, run the minor-version flow, rebuild/revalidate
profile and Wasm identities, commit, and create the release tag through the
maintained release flow.

Changes: package/release metadata and regenerated artifact evidence.

### P2

#### P2-1 - Active documentation still contains legacy methods and inconsistent counts

Evidence:

- Active operator documentation still instructs use of removed methods at
  `docs/operations/root-proof-provisioning.md:38`,
  `docs/contracts/AUTH_DELEGATED_SIGNATURES.md:270`, `docs/metrics.md:3`, and
  `docs/getting-started/local-academic-fleet.md:159`.
- The open changelog records the obsolete `45 / 80 / 2 / 61` disposition at
  `docs/changelog/0.103.md:175`, while accepted/current evidence is
  `49 / 78 / 2 / 59`.
- The accepted B1 table reports 19 Root command responses and 13 managed
  statuses at
  `docs/audits/working/0.103-role-owned-candid-surface/README.md:145`;
  generated Candid contains 20 and 12 respectively.

Observed behaviour: active status/B7 claim active documentation is migrated,
but several current documents retain old callable names, and quantitative
evidence disagrees with generated truth.

Accepted contract: active documentation and generated counts must agree with
the maintained surface.

Practical impact: maintainers and downstream callers can follow deleted
protocols; reviewers cannot rely on the quantitative report without
regenerating it.

Smallest correction: replace legacy examples with role selectors, update the
changelog disposition, and correct the two B1 table cells.

Changes: docs only.

### P3

No P3 findings.

## 5. Acceptance Matrix

| # | Criterion | Result | Evidence |
| ---: | --- | --- | --- |
| 1 | Complete B1 method/caller classification | PASS | B1 counts and reproduced hashes |
| 2 | Exactly one six-way disposition | PASS | Accepted B1 disposition |
| 3 | Role ceilings and Store lane admission | PASS | Generated DIDs and Store endpoints |
| 4 | Exact generated method/type counts | PASS | B6 matrix, independently rebuilt |
| 5 | Former phases private unless admitted | PASS | B4 reconciliation |
| 6 | Submit-once/autonomous journeys | PASS | Focused unit and PocketIC journeys |
| 7 | External-effect interruption/replay | PASS | Focused lost-response/replay tests |
| 8 | Authorization before workflow/state | **FAIL** | P0-1 |
| 9 | Variant-aware ingress/replay | PASS | Store ingress and focused tests |
| 10 | Exact-limit/first-excess | PASS | Focused ingress/chunk/capacity tests |
| 11 | Capability pruning/reserved names/timer boundary | **FAIL** | P1-1 |
| 12 | Bounded status variants | PASS | Generated role DIDs/status DTOs |
| 13 | Callers use role surfaces | PASS | Old-name caller scan |
| 14 | No old endpoint/help/fallback residue | **FAIL** | P2-1 |
| 15 | Thin `canic::start!` | PASS | Protocol/build-surface guards |
| 16 | Compact diagnostic composition | PASS | Compact public `Error` retained |
| 17 | Exact 0.104 consumer handoff | PASS | Capability manifest handoff |
| 18 | Changelog/status record hard cut | PASS | Core reduction recorded |
| 19 | Atomic commands avoid operation machinery | PASS | B1 sync/async accounting |
| 20 | External pre-call profile binding | **FAIL** | P0-2 |
| 21 | No B2/B3 superset protocol | PASS | Seven exact generated services |
| 22 | Exact response correlation | PASS | Closed response enums/dispatch |
| 23 | Complete generated quantitative report | **FAIL** | B1 count drift/incomplete reporting |
| 24 | No top-level Admin/Peer/Workflow catch-all | PASS | Request depth one |
| 25 | Runtime excludes funding management | PASS | Positive/negative Candid evidence |
| 26 | Atomic B4/B5 caller cut; B6 cleanup | PASS | Functional migration landed together |

## 6. Baseline and Final Surface Matrix

| Role/profile | Capabilities | Baseline Canic | Final Canic | Ext/App | Cmd req/resp | Status req/resp | Sync / async kinds | Lanes | Candid bytes/types | Candid SHA-256 | Profile digest | Wasm bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- | ---: |
| Runtime managed app | Runtime | - | 2 | 1/0 | 2/2 | 10/10 | 1 / 1 | 0 | 25,395/117 | `e3dda55a...1150` | `0acbf544...e590` | 3,385,328 |
| User hub | Runtime, Sharding, AutomaticTopup | - | 2 | 1/3 | 2/2 | 12/12 | 1 / 1 | 0 | 26,300/123 | `c9d46eb4...1523` | `6ab51a61...f811` | - |
| Managed issuer | Auth capabilities | 23 | 2 | 1/10 | 4/4 | 12/12 | 3 / 1 | 0 | 31,576/151 | `0d6ec4c5...5987` | `29e7ca43...c34` | - |
| Root non-signer | Root/control plane | 117* | 2 | 1/0 | 31/19 | 21/21 | 17 / 12 | 0 | 74,763/282 | `04210595...eccd` | `1bb068b7...8550` | - |
| Root signer | + RoleAttestationSigner | 117 | 2 | 1/3 | 32/20 | 22/22 | 18 / 12 | 0 | 76,205/291 | `0fa86d62...f892` | `1105c6d1...9fb` | 8,424,200 |
| Coordinator | FleetCoordinator | 24 | 2 | 0/0 | 9/8 | 7/7 | 7 / 2 | 0 | 29,233/97 | `84f147a8...f98f` | `206ea1a1...bfa` | 4,070,628 |
| Store | WasmStore | 24 | 4 | 1/0 | 10/8 | 7/7 | 7 / 2 | 2 | 11,957/69 | `fbf6250a...dad4` | `f1e01076...330b0` | 3,329,993 |

`*` The retained predecessor Root baseline is the signer/auth fixture, not a
distinct non-signer baseline.

The 188-to-ten claim means method appearances across four separate services,
not ten unique names. Current full-service total for signer Root + issuer +
Coordinator + Store is 26 after separately adding external and fixture-owned
methods.

## 7. Endpoint-Disposition Summary

- Baseline rows: 207
- Canic-owned rows: 188
- Role-command dispositions: 49
- Role-status dispositions: 78
- Store data lanes: 2
- Private/delete: 59
- External-standard: 3
- Application-owned: 16
- Unclassified: 0
- Forbidden seventh category: 0
- Current representative Canic method appearances: 10
- Representative old method appearances removed: 178

The largest merge is 21 Root operation-status queries into one role-local
`Operation` status selector. Fleet, pool, refill, provisioning,
synchronization, and removal methods are similarly merged into outcome
variants. Private phases have no shadow protocol variant.

## 8. Variant and Correlation Summary

| Role/profile | Commands | Command responses | Statuses | Atomic commands | Async variants / operation kinds |
| --- | ---: | ---: | ---: | ---: | ---: |
| Root signer | 32 | 20 | 22 | 18 | 14 / 12 |
| Root non-signer | 31 | 19 | 21 | 17 | 14 / 12 |
| Coordinator | 9 | 8 | 7 | 7 | 2 / 2 |
| Store | 10 | 8 | 7 | 7 | 3 / 2 |
| Managed issuer | 4 | 4 | 12 | 3 | 1 / 1 |
| Runtime/user hub | 2 | 2 | 10 or 12 | 1 | 1 / 1 |

- Maximum request-selector nesting: one
- Maximum response-selector nesting: two, only local operation detail
- Missing type-level correlations: none found
- Catch-all selector: none
- Runtime dispatch correlation: structurally consistent
- B1 documentation drift: Root responses `19 -> 20`; issuer statuses `13 -> 12`

## 9. Authorization Matrix

| Surface | Intended authority | Result |
| --- | --- | --- |
| Root controller variants | Controller before dispatch | PASS for listed controller variants |
| Root member/RPC variants | Exact member/capability caller | PASS for explicit predicates |
| Root child/peer/provisioning/sync variants | Exact parent, peer, Coordinator, or signer authority | **FAIL ordering** |
| Coordinator controller variants | Controller before dispatch | PASS |
| `AcknowledgeRootSnapshot` | Exact joining Root | **FAIL ordering** |
| Coordinator Registry status | Controller or exact participating Root | **FAIL ordering** |
| Store commands | Root, parent, or exact mutation predicate | PASS |
| Store protected statuses | Controller or Root as selected | PASS |
| Managed statuses | Public/protected by variant | Generated separation present |

`inspect_message` is bounded and variant-aware, and endpoint authorization is
repeated after inspection. Replay classification passed focused tests. The
failing issue is that not every selected variant establishes authority before
protected work.

## 10. Profile-Pruning Matrix

| Profile distinction | Variant | DTO/type graph | Public handler | Endpoint workflow | Wasm |
| --- | --- | --- | --- | --- | --- |
| Runtime-only / AutomaticTopup absent | Absent | Absent | Absent | Inert | Store constants still retained |
| AutomaticTopup enabled | Present | Present | Present | Present | Present |
| Sharding disabled/enabled | Correct | Correct | Correct | Correct | Correct |
| Root signer/non-signer | Correct 31/32 | Correct | Correct | Correct | Correct |
| Auth absent/present | Correct | Correct | Correct | Correct | Correct |
| Child-management absent/present | Correct | Correct | Correct | Correct | Correct in generated surface |
| Storage absent/Store | Correct | Correct | Correct | Correct handlers | **Protocol/replay constants leak** |

Capabilities do not add Candid methods. Candid variants/types and public
handlers prune correctly. The negative result is narrower but real: Store
method/replay identity is compiled into unrelated roles.

## 11. Profile-Binding Bootstrap

Implemented successfully:

- deterministic profile digest type;
- canonical ordered capability set;
- generated Candid SHA-256;
- build-artifact metadata;
- release-set metadata;
- Root Store/install/Registry persistence;
- Component Directory projection; and
- Overview includes a profile digest.

Not implemented:

- exact host binding selection before ordinary role calls;
- exact CLI binding selection before ordinary role calls;
- missing-evidence pre-call rejection;
- hash/profile comparison in ordinary observations;
- post-selection Overview verification; and
- proof that static inter-canister fragments match target metadata before
  calling.

No old-method probing or dual protocol was found. However, proceeding with
`None` or role-only Candid is itself contrary to the accepted bootstrap
contract.

## 12. Autonomous Operation Evidence

| Journey | Owner | Durable/replay evidence | Result |
| --- | --- | --- | --- |
| Component provisioning | Root/Coordinator domain records | acceptance replay, status, lost-response recovery | PASS |
| Component/subtree removal | Root component registry | exact operation identity and terminal receipts | PASS |
| Root removal | Coordinator + external controller | prepare/complete evidence and status polling | PASS |
| Registry synchronization | Root mirror + Coordinator acknowledgement | exact candidate/synchronization record | PASS, subject to auth ordering |
| Store Fleet activation | Store activation state | operation-owned status/replay | PASS |
| Store garbage collection | Store GC state | operation ID, bounded state, restart test | PASS |

Focused tests covered duplicate submission, conflicting operation identity,
interruption, uncertainty, status, and terminal replay. No universal operation
database was introduced.

Atomic synchronous commands use typed responses and do not create operation
records. The principal residual recovery risk is unauthorized entry into
workflows before validation, not demonstrated duplicate external effects.

## 13. B4 Correction Reconciliation

| Correction | Assessment |
| --- | --- |
| Pre-adoption Store staging | Independently required byte transport; exactly two lanes; PASS |
| `AcknowledgeRootSnapshot` | Required durable outcome, not polling phase; authorization ordering FAIL |
| `SynchronizeComponentDirectories` | Required high-level convergence evidence, not private phase; authorization ordering FAIL |
| `PrepareRootDeletionExecution` | Required pre-deletion authority evidence; PASS |
| `CompleteRootDeletion` | Required typed absence evidence; PASS |
| Participating Root Registry status | Legitimate protected observation; authorization ordering FAIL |

The four corrections add no new Candid method and do not recreate old phase
families.

## 14. Caller Hard-Cut Matrix

| Caller family | Current role variant | Profile-specific binding | Replay current | No fallback/old constant |
| --- | --- | --- | --- | --- |
| `canic-core` | Yes | Static fragments, target match not pre-proven | Yes | Yes |
| `canic-control-plane` | Yes | Static fragments, target match not pre-proven | Yes | Yes |
| Root callers | Yes | Metadata persisted | Yes | Yes |
| Coordinator callers | Yes | Metadata persisted | Yes | Yes |
| Store callers | Yes | Metadata persisted | Yes | Yes |
| Host | Yes | **No ordinary pre-call resolver** | Yes | Yes |
| CLI | Yes | **Optional role-path only** | N/A | Yes |
| Generated bindings | Current | Generated per profile | N/A | Yes |
| Fixtures/PocketIC | Current | Build-profile exact | Yes | Yes |
| Skynet/presentation | Current role status | Local generated type | N/A | Yes |
| Active documentation | **Legacy residue remains** | N/A | N/A | No |

The functional protocol/caller migration landed together in commit
`731cd2a2d`; no first functional migration was found only in the subsequent
cleanup commits.

## 15. Validation Performed

### Source and identity

Passed:

```text
git status --short
git branch --show-current
git rev-parse HEAD
git show -s --format=fuller HEAD
git rev-parse 'v0.102.2^{commit}'
git rev-parse 'v0.103.0^{commit}'
git cat-file -t v0.103.0
git diff --stat v0.102.2..HEAD
git diff --name-status v0.102.2..HEAD
git diff --binary HEAD | sha256sum
git diff --cached --binary | sha256sum
git ls-files --others --exclude-standard
git submodule status
cargo metadata --no-deps
rustc --version --verbose
cargo --version
```

Read-only `rg`, `git diff/show/log`, `sed`, `nl`, `awk`, `find`, `strings`, and
`sha256sum` inspections were used for every cited surface, old-method row,
stable schema, generated DID, and Wasm.

### Baseline

In a clean exact predecessor checkout:

```text
bash docs/audits/working/0.103-role-owned-candid-surface/capture-baseline.sh \
  /tmp/canic-0103-audit-v01022 \
  /tmp/canic-0.103-b1-v0.102.2
```

Result: pass; regenerated baseline and manifest hashes were byte-identical.

### Current Candid/profile builds

Canonical `canic-host` `build_artifact` fast-profile builds passed for:

```text
app
user_hub
issuer
root (non-signer)
root (signer)
wasm_store
```

Coordinator and Store checked-in DIDs parsed successfully. Store regeneration
was byte-identical.

### Focused tests

Passed:

```text
cargo test --locked -p canic --test protocol_surface
cargo test -p canic --test build_cfg_surface
cargo test -p canic --test managed_endpoint_gate
cargo test -p canic --test protocol_inventory_gate
cargo test -p canic-core --test timer_inventory_guard
cargo test -p canic-core --lib role_contract::
cargo test -p canic-core --lib replay_policy::tests::
```

Focused control-plane filters passed for:

```text
exact acceptance replay
component provisioning
directory publication
component and subtree removal
Coordinator lost-response journaling
Root Wasm Store roundtrip
Store GC roundtrip
Registry synchronization
capacity first-excess
staged payload bounds
```

Focused host filters passed for:

```text
profile Candid generation
profile-binding metadata validation
release-manifest malformed hashes
artifact provenance
```

Focused CLI Candid and Fleet-status tests passed, but those tests also
confirmed that missing bindings currently return `None`, supporting P0-2.

### Feature/build checks

Passed:

```text
cargo check -p canic-core
cargo check -p canic-control-plane
cargo check -p canic-host
cargo check -p canic-cli
cargo check -p canic-fleet-coordinator --target wasm32-unknown-unknown
```

This direct command failed intentionally:

```text
cargo check -p canic-wasm-store --target wasm32-unknown-unknown
```

The build script correctly requires the canonical Canic builder. The canonical
Store build passed.

`cargo test -p canic --test fleet_coordinator_surface` ran zero tests without
its feature and was not treated as evidence.

### PocketIC

Passed after running outside the socket-restricted sandbox:

```text
prepared_root_bootstraps_and_reverifies_its_exact_local_store
published_draining_root_autonomously_reaches_external_deletion_readiness
coordinator_commits_joining_roots_and_replays_original_receipts
```

The first sandbox attempt could not bind `127.0.0.1:0` and was interrupted; the
unrestricted rerun passed.

### Repository guards

Passed:

```text
bash scripts/ci/check-current-document-semantics.sh
bash scripts/ci/test-dependency-risk-inventory.sh
```

The first dependency-risk run failed because the sandbox blocked the RustSec
network fetch; the permitted network rerun passed.

### Artifact scan

Representative Wasm was scanned using the exact B1 old-method list:

```text
strings <artifact.wasm> | rg -F -f <old-method-list>
```

Only the two admitted Store lane names remained, but they were present in every
representative role, producing P1-1.

### Skipped broad validation

Not run:

- full workspace tests;
- workspace-wide Clippy;
- full release matrix;
- broad PocketIC suites; and
- package versioning/tagging/release commands.

These were deliberately skipped because the audit requested focused validation
first and `AGENTS.md` explicitly reserves broad gates for maintainer-owned
release flow. The confirmed P0/P1 source findings do not require a broad suite
to reproduce.

## 16. Measurement and Claim Assessment

| Claim | Result |
| --- | --- |
| Method reduction | PROVEN: 188 -> 10 |
| Variant pruning | PROVEN for generated Candid; incomplete for protocol/replay constants |
| Candid type reduction | Current types measured; predecessor type delta NOT PROVEN |
| Protocol reachability reduction | PARTIAL |
| Wasm reduction per role | NOT PROVEN |
| Data-section reduction per role | NOT PROVEN |
| Causal 0.103 size saving | NOT CLAIMED and NOT PROVEN |

Representative current artifacts:

| Role | Raw | Gzip | Data | Functions | Wasm exports | Candid methods |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Component | 3,385,328 | 883,993 | 247,836 | 13,860 | 9 | 3 |
| Root | 8,424,200 | 2,182,503 | 437,724 | 28,662 | 12 | 6 |
| Coordinator | 4,070,628 | 1,013,747 | 260,840 | 14,841 | 7 | 2 |
| Store | 3,329,993 | 879,694 | 242,016 | 13,933 | 11 | 5 |

B7 correctly says no isolated same-source pre-cut Wasm pair exists. Rebuilding
in a different workspace produced different Wasm identities because
build-root/config paths remain embedded, so the retained hashes are
current-environment artifact identities rather than location-independent
reproducibility evidence.

## 17. Documentation and Release-Truth Reconciliation

| Claim | Reconciliation |
| --- | --- |
| B1 accepted | Yes |
| B2-B7 complete | Active docs say yes; audit says no because P0/P1 remain |
| 0.103 unreleased | Yes |
| 188 Canic methods become ten | Yes |
| Root has 32 commands | Yes |
| Coordinator has nine | Yes |
| Store has ten commands, seven statuses, two lanes | Yes |
| External exact profile bootstrap | **No; active claim exceeds implementation** |
| Private timer pruning belongs to 0.104 | Yes |
| No protocol mutation remains authorized | Stated, but bounded blocker corrections are required |
| Existing tag needs reconciliation | Yes |
| Maintainer release flow is next | **No; corrections and focused re-audit come first** |

The active changelog and status correctly identify the stale tag/package
boundary. They do not correctly reflect the pre-call profile-binding
implementation, the remaining active old-method documentation, or all
generated counts.

## 18. 0.104 Handoff

The exact current scheduling inventory is frozen at
`crates/canic-core/tests/timer_inventory_guard.rs:92`, covering:

- `canic::start!` lifecycle deferrals;
- facade timer APIs/macros;
- Coordinator and Root recovery scheduling;
- component provisioning and registry recovery;
- fleet mirror and Root reconciliation;
- pool maintenance;
- runtime timer facade;
- initialization and upgrade scheduling;
- placement acknowledgement;
- issuer renewal;
- authority restoration;
- cycle top-up;
- local intent cleanup;
- runtime-log retention;
- root runtime work; and
- Store scheduling.

The accepted new autonomous-operation participants are listed at
`docs/audits/working/0.103-role-owned-candid-surface/capability-manifest.md:355`:

- twelve Root operation kinds;
- Coordinator component provisioning and Root removal;
- managed runtime configuration; and
- Store Fleet activation and garbage collection.

The fixed future owners are enumerated at
`docs/design/0.104-ic-timers-consumer-hard-cut/0.104-design.md:173`. B1 remains
pending and source mutation is not authorized at
`docs/design/0.104-ic-timers-consumer-hard-cut/status.md:24`.

The handoff is sufficient to avoid reopening the 0.103 Candid design, but 0.104
should not begin until the 0.103 P0/P1 corrections are closed and published.

## 19. Residual Risk

- Runtime risk: unauthorized variants can enter protected workflows before
  rejection.
- Security risk: state-dependent error oracles and pre-auth Store traffic.
- Recovery risk: no duplicate paid/controller effect was reproduced; existing
  operation recovery evidence is strong.
- Profile-binding risk: wrong or missing schema evidence does not stop ordinary
  calls before transport.
- Capability-pruning risk: Store-only protocol/replay material remains in
  unrelated Wasm.
- Release-identity risk: stale tag and `0.102.2` package/profile identity cannot
  publish this candidate truthfully.
- Documentation risk: active old methods and inconsistent quantitative
  evidence.
- Successor-handoff risk: low; the timer inventory is explicit, but 0.104 must
  wait for corrected 0.103 closeout.

## 20. Repository Mutation Statement

Repository changes made by the audit itself: none.

This file was added afterward at the maintainer's request to preserve the audit
result.
