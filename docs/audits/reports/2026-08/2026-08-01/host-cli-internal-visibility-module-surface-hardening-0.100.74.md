# MSH Compact Audit: Host and CLI Same-File Internals

## Verdict

- Risk score: 2 / 10.
- Tier: Tier 1.
- Patch mode: implementation-requested.
- Cleanup result: five one-caller helpers were inlined, 19 same-file functions
  and nine same-file constants were narrowed to private visibility, one
  orphaned query-provenance field and its enum were deleted, the installed-Fleet
  request/error contract lost ten unread or unreachable items, two text
  renderer branches orphaned by the `0.99.31` deploy-command removal were
  deleted, and 462 net Rust lines were removed.
- Runtime shape: unchanged. The Rust source API is intentionally hard-cut, but
  no allocation, dispatch, persistence, wire, command or operator-output
  behavior changed.

## Scope / Evidence

| Area | Evidence | Result |
| --- | --- | --- |
| release anchor | exact tag and clean-tree inspection before the concurrent feature began | `v0.100.73` at `3a4cd3b34c942a8d537adf3169aeafae1f41ac85` |
| in-scope roots | selected same-file helpers in `canic-cli` and `canic-host` | cold, warm and test-only internal surface |
| stale signals | dead/unused suppression, compatibility-vocabulary and installed-Fleet producer scans | no retained dead/unused suppression; installed-Fleet carried unread request fields and unproducible errors |
| consumers | declaration inventory plus repository-wide exact-name scans | 19 functions and nine constants had consumers only in their declaration file; five helpers had one caller and no invariant; subnet-query provenance had no reader; five installed-Fleet errors had no producer; two public renderers had no production caller |
| concurrent exclusion | repeated worktree and blame inspection | in-progress root-deletion work under core/control-plane/facade was excluded |

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| --- | --- | --- | --- |
| internal surface inventory | scanned internal function declarations under host, CLI and backup, then counted exact repository consumer files | same-file-only candidate set isolated | this report |
| suppression scan | searched Rust sources for dead-code and unused-import allowances/expectations | none in the inspected production crates | this report |
| consumer check | exact-name searches across all crate Rust sources | every narrowed function remained reachable only inside its declaration file | source diff |
| installed-Fleet producer check | inspected the complete resolver and exact variant references | only `NoInstalledFleet`, `FleetCatalog` and `CoordinatorAnchoredTopologyUnavailable` remain producible | source diff |
| renderer history check | exact-name scans plus `git log -S` across deployment-truth and deploy CLI history | authority-plan and execution-preflight text lost their final production consumers in `0.99.31` | source diff and repository history |
| validation | package check; CLI unit tests; focused host install and deployment-truth tests; strict no-dependency Clippy | pass | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| --- | --- | --- | --- | --- |
| 19 host/CLI functions exposed to a parent or crate while consumed only in their declaration file | `overexposed-internal` | high | `NARROW NOW` | the declaration module is the smallest current owner |
| nine host/CLI constants exposed beyond their only consumer file | `overexposed-internal` | high | `NARROW NOW` | output, finding and restore filename vocabulary remains unchanged under its declaration owner |
| restore-plan file forwarding helper | `duplicate-surface` | high | `INLINE NOW` | forwarded once to the canonical restore-plan writer without validation or error mapping |
| local-root Wasm path test helper | `live-test-support` with vocabulary-only wrapper | high | `INLINE NOW` | its sole test can call the canonical artifact-root owner directly |
| app-list row collector plus two blob-storage label helpers | `duplicate-surface` | high | `INLINE NOW` | each had one caller and only repeated an iterator or a direct boolean label |
| subnet Registry query source field and enum | `orphaned-helper` | high | `DELETE NOW` | the installed-deployment diagnostic owner was removed; remaining callers read only typed Registry entries |
| installed-Fleet `icp` and `detect_lost_local_root` request fields | `orphaned-input` | high | `DELETE NOW` | the complete resolver reads only Fleet and environment; every constructor merely populated the fields |
| five unproducible `InstalledFleetError` variants | `orphaned-error-surface` | high | `DELETE NOW` | the Coordinator-anchored resolver has no replica, ICP CLI, lost-local-root, Registry-parse or direct I/O branch |
| `ListCommandError::InstalledFleetHint` | `unreachable-error-surface` | high | `DELETE NOW` | its only constructor wrapped the deleted installed-Fleet ICP error |
| two Medic options parameters | `orphaned-input` | high | `DELETE NOW` | both existed only to forward the deleted installed-Fleet request fields |
| installed-Fleet resolver and four resolution types | `live-fail-closed-boundary` | high | `RETAIN WITH OWNER` | maintained CLI callers still use the typed boundary, which explicitly rejects the removed single-root topology until Coordinator-backed resolution replaces it |
| authority-plan text module | `removed-command-residue` | high | `DELETE NOW` | its only production caller was the deploy authority command removed in `0.99.31`; only the public re-export survived |
| execution-preflight text module and renderer-only test | `removed-command-residue` | high | `DELETE NOW` | its last production consumer was in the promotion text branch removed in `0.99.31`; the remaining test only preserved the orphan itself |
| concurrent root-deletion files | `live-authority` in progress | blocked for this slice | `BLOCKED` | another session owns the still-changing stable/control-plane/facade boundary |

## Hot / Wasm Risk

| Item | Hotness | Risk | Required Proof |
| --- | --- | --- | --- |
| visibility narrowing | cold / warm / test-only | none; symbol reachability only | package compile and focused tests |
| five helper inlines | cold / test-only | none; same direct calls and data flow | focused CLI and install-root tests |
| query-provenance deletion | cold operator lookup | none; transport choice and returned entries are unchanged | focused backup and host/CLI compile |
| installed-Fleet source-contract hard cut | cold host/CLI target resolution | low; pre-1.0 Rust callers must stop populating or matching dead items | host/CLI compile, focused resolver and command tests |
| orphaned renderer deletion | cold, unreachable presentation code | none at runtime; no command or production caller remained | history/consumer scan, package compile and focused deployment-truth tests |

## Disposition Ledger

| Disposition | Count |
| --- | ---: |
| `DELETE NOW` | 25 |
| `NARROW NOW` | 28 |
| `INLINE NOW` | 5 |
| `MOVE TO TEST` | 0 |
| `RETAIN WITH OWNER` | 5 |
| `MEASURE FIRST` | 0 |
| `BLOCKED` | 1 |

## Validation / Follow-up

- `cargo check -p canic-cli -p canic-host`: pass.
- `cargo test -p canic-host --lib installed_fleet::`: 2 passed.
- Focused host deployment-truth preflight and comparison filters: 17 passed
  after the renderer deletion.
- Focused CLI unit filters for apps, auth, backup-create, blob storage, list and
  Medic: 170 passed.
- `cargo clippy -p canic-cli -p canic-host --all-targets --no-deps -- -D warnings`: pass.
