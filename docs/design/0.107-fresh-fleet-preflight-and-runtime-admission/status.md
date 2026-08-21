# Canic 0.107 Implementation Status

Date: 2026-08-21

## Status

- State: the design is accepted as application-safety/estate step 5, and B2-B7
  implementation is complete. Minor closeout remains pending the exact AC12
  documentation re-audit.
- Runtime impact: B2-B5 change only the host CLI/planning and pre-effect install
  boundary. B6 owns the bounded managed-role runtime-whitelist state and
  existing-method variants. No external effect was performed.
- Predecessor: accepted repository-local 0.106 B1 baseline. The separately
  authorized 0.106 B2 external evidence does not gate this line.
- Successors: 0.108 Coordinator-backed root funding retains its passing B1
  evidence but begins production work only after this line; 0.109 estates and
  later scheduled lines move one minor number without semantic change.
- Repository boundary: Toko remains read-only and supplies requirements plus
  final acceptance evidence only.
- Historical delivery estimate: seven release batches and approximately 10-15
  engineering days, excluding upstream release latency and separately approved
  live-IC work.
- Implementation approval: the maintainer accepted B1 on 2026-08-20, and
  B2-B7 are complete. The 2026-08-21 closeout audit passed AC1-AC11 and AC13
  and rejected only AC12's active-document residue; this documentation-only
  correction is ready for the exact re-audit.

Design: [Fresh-Fleet preflight and runtime admission](0.107-design.md)

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Exact baseline and contract | Toko traceability, current planner/install/whitelist/upstream inventories, bounds and exact surface contract | source/fixture inventory and explicit acceptance | Accepted 2026-08-20 |
| B2 | Target-correct planning | direct plan-leaf environment forwarding and mismatch rejection | CLI parse/forwarding/help tests | Complete |
| B3 | Fleet-input-complete pure preflight | shared compiler, no-effect ordering and fresh-Fleet blockers | host plan/install ordering and fixture tests | Complete |
| B4 | Complete evidence and digest binding | placement, counts, funding, balance, output and install-receipt parity | parity, insufficient-funds and receipt tests | Complete |
| B5 | Structured catalog inconsistency | Registry version/provenance/cache/subject/retry/effect propagation and upstream update if needed | typed collector/host/CLI tests | Complete |
| B6 | Durable runtime whitelist | seed/restore, bounds, add/remove/revision/digest/replay and config hard cut | core/facade/restoration tests | Complete |
| B7 | Operator proof and closeout | command/status UX, adversarial/recovery journeys, generic fixture, downstream read-only rerun and residue cleanup | targeted package and bounded PocketIC checks | Complete; AC12 documentation correction ready for closeout re-audit |

These are coherent outcome batches, not preassigned patch releases.

## B1 Result

The repository-local B1 capture is complete under
[`docs/audits/working/0.107-fresh-fleet-preflight-and-runtime-admission/`](../../audits/working/0.107-fresh-fleet-preflight-and-runtime-admission/README.md).
The maintainer accepted it on 2026-08-20 as production and stable-state
implementation authority for the sequenced B2-B7 batches.

- At B1 capture, the planner, installer, whitelist access path, stable-
  allocation registry and managed-role macros were byte-for-byte `v0.105.0`
  source. The retained hashes remain that accepted predecessor baseline.
- The maintained direct plan leaf now has one frozen grammar shared with
  install: required App, Fleet and Fleet input; common profile and optional
  finalized release-build identity; one forwarded global environment; and an
  optional install-only expected plan digest.
- One pure host compiler and domain-separated schema-1 plan digest cover all
  decision-bearing target, Fleet-input, catalog, placement, funding, balance,
  release-source and no-effect evidence before durable build preparation.
- Runtime admission is frozen for managed non-root roles beneath the existing
  role methods. Memory ID 61 owns schema 1, at most 256 sorted principals, a
  128-entry page and one retained operation under Root-or-controller
  administration. Test-only maximum encodings measure 8,417 stable bytes,
  4,072 status Candid bytes and 101 mutation Candid bytes.
- Read-only Toko commit
  `bf14a5d3d89be4335d3da2601e8a60128fde04df` contains 175 compiled
  whitelist principals, leaving 81 entries beneath the hard maximum. It has no
  current Canic integration or CANIC-011/012/013 identifiers, so B7 must use a
  newer read-only acceptance source or record that exact external blocker.
- Exact `ic-query 0.40.1` loses a Registry version already known after its
  initial fetch and does not retain failed cache-stage context or unknown
  retryability. B1 freezes the smallest additive typed upstream result; B5 may
  not claim full provenance through string parsing or a fork.

No production source, stable implementation, Candid or CLI was changed by B1.
No 0.106 B2 effect or sibling-repository mutation occurred.

## B2 Result

- The top-level environment is forwarded to the direct `deploy plan` leaf.
- A hidden/internal environment that disagrees with the selected top-level
  environment rejects before dispatch instead of silently winning.
- Planning resolves the selected ICP environment to one canonical network.
  Missing authority and contradictory environment profiles are blockers, and
  a contradiction cannot fall through to Fleet-catalog lookup.
- Direct plan help now identifies the top-level environment placement. The
  leaf still accepts no ICP executable because planning performs no ICP
  command or IC effect.
- Focused plan/forwarding tests and warning-denied `canic-cli` Clippy pass. No
  Canister runtime, stable state, Candid, external Canister or sibling
  repository changed.

## B3 Result

- Direct `deploy plan` now requires the same `--fleet-input` document as
  install, uses the common `--profile` spelling, accepts the optional exact
  finalized `--release-build` identity and hard-cuts its old `--config` and
  `--build-profile` options without aliases.
- One pure `canic-host` compiler validates the canonical App/Fleet identity,
  resolved Coordinator/root placement, admissions, Component Group
  assignments, limits and positive creation funding. Its named output carries
  the resolved build profile/release source and admits only
  `build_started = workspace_mutation_started = ic_mutation_started = false`.
- Both direct planning and fresh installation call that compiler. Installation
  resolves canonical target authority, Fleet input and an existing
  session/finalized or workspace release source before release-build
  allocation. The later immutable Fleet-plan compiler reuses the same
  preflight rather than retaining a second topology/funding validator.
- Direct mainnet planning is cache-only by default. The closeout correction
  adds explicit `deploy plan --refresh-catalog`, while installation uses the
  same live-capable acquisition path automatically. Both may issue public NNS
  Registry query calls and update only the private `.canic/ic-query` cache when
  it is missing or invalid. Both compile from stable validated snapshot
  authority; cache path, collection time and disposition remain separate
  report provenance. Missing cache authority remains a typed pre-effect
  blocker for direct planning without the flag, with a direct refresh remedy.
- Invalid Fleet schema and invalid Component admissions both reject in the
  Planning phase before `.canic/release-builds` exists. Focused pure-compiler,
  plan-report, install-ordering and no-allocation tests pass.
- B3 does not yet claim the complete counts, balance, maximum debit or canonical
  plan digest. Those decision-bearing facts and their receipt binding remain
  B4. No Canister runtime, stable state, Candid, external Canister or sibling
  repository changed.

## B4 Result

- One canonical schema-1 decision now contains exact target, Fleet input,
  workspace or finalized release authority, expected artifact set, catalog
  evidence, placement, per-category funding, derived Canister counts and
  bounded operator-balance evidence. Missing, stale, insufficient or changed
  authority rejects instead of becoming a warning.
- The domain-separated SHA-256 plan digest is compiled from the compact
  canonical decision payload. Workspace release authority hashes the selected
  source/build inputs while excluding plan delivery and generated output, so
  writing a report cannot change its own decision identity.
- Direct planning renders the exact digest, maximum operator debit, balance
  validity, counts, root placement/pool summary and funding categories in text
  and JSON. Install recompiles the decision before release-build allocation;
  an optional exact expected digest and changed balance or source evidence fail
  in Planning before an effect begins.
- Same-release install sessions retain the original workspace-versus-finalized
  decision source and digest. The persisted Fleet plan, deployment truth,
  completion receipt, known replica rejection receipt and resume comparison
  all bind that digest; conflicting retry or recovery authority fails closed.
- Focused validation passes 35 host Fleet-input/plan/session tests, three known
  rejection-receipt tests, the install parity/truth/resume cases, 15 CLI install
  tests, 23 direct-plan tests, the workspace-source exclusion case and
  warning-denied host/CLI Clippy.
- No Canister runtime, stable state, Candid, external Canister or sibling
  repository changed. B5 later completed live catalog failure propagation and
  complete typed inconsistency provenance.

## B5 Result

- The exact locked production graph advances from published crates.io
  `ic-query 0.41.2` to `0.42.0`. The former's checksum is
  `a9c7486d35030ca36b45636599da5f142b92bf51c548cf238d8567750376fded`
  and the latter locks at
  `311b60543bc5c09c961abe9612d2bf3e26e99ba8bcadb3c01d043056c544a318`.
  `0.41.2` reconstructs the complete pinned `canister_ranges_*` family before
  considering legacy routing and never falls back after a modern-family
  failure. Its detailed load result retains the request source and assurance,
  exact cache stage/disposition, pinned and returned Registry-value versions,
  exact failing endpoint/assurance, completed record reads, typed offending
  subject, stable code/category and `Unknown(typed reason)` retryability.
- Its portable API owns canonical Registry-key constants, typed subjects and
  ordinary uncertified-query evidence construction, so downstream fixtures do
  not hand-format Registry identities. Canic's Root-subnet fixture consumes
  those builders directly.
- Canic calls only the detailed cached/live APIs. One exhaustive typed
  projection carries all upstream-known fields—including routing-shard lower
  bounds and inline/chunked value encoding—into the deployment-plan JSON and
  text report and adds explicit false build, workspace-mutation and IC-
  mutation facts. Catalog failures are classified as Fleet-catalog evidence;
  no error-string parser, fork, inferred version, guessed retry decision or
  routing fallback was introduced.
- Focused host tests prove a real cache-absence journey and a pinned-version
  Registry-record projection with returned-value and completed-shard evidence.
  Focused CLI tests prove end-to-end cache-failure propagation plus JSON/text
  rendering of the versions, endpoint, assurance, subject, completed record,
  cache trigger and unknown reason without calling it transient. The Root-
  subnet fixture cannot escape into a live refresh. Warning-denied host/CLI
  Clippy passes.
- Direct `deploy plan` remains cache-only by default and exposes the live
  detailed path only through `--refresh-catalog`. The opt-in path may update
  the private catalog cache but cannot start a build, deployment-state
  mutation or IC update call, closing the fresh-checkout planning gap reported
  by the downstream acceptance rerun.
- `ic-query 0.42.0` hard-cuts the combined authority accessor. Canic consumes
  `snapshot_authority()` as canonical Registry-version/digest/assurance/source
  identity while reporting load path, collection time and disposition as
  transient acquisition provenance. Plan and install therefore preserve
  digest parity without a compatibility adapter.
- Install resolves the effective ICP identity after exact plan recompilation
  and before release-build allocation or Wasm preparation. Anonymous, unusable
  and Fleet-operator-mismatched identities reject at the Identity phase;
  encrypted non-interactive execution receives the
  `CANIC_ICP_IDENTITY_PASSWORD_FILE` remedy.

## B6 Result

- Memory ID 61 now owns one schema-1 stable record containing at most 256
  sorted unique principals, a revision, the canonical membership digest and
  one retained exact-operation result. The real maximum record remains exactly
  8,417 bytes; the 128-entry status and mutation request remain 4,072 and 101
  Candid bytes.
- Fresh managed non-root lifecycle synchronously sorts, deduplicates and seeds
  that record from validated App configuration. Same-release post-upgrade
  validates only the retained record and never reseeds, merges or repairs it.
  Root, Coordinator, Store and standalone-local specialized surfaces do not
  expose the variants.
- One pure policy owns zero-ID, revision, capacity and operation-conflict
  rejection; checked revision advance; idempotent add/remove; exact response-
  loss retry; canonical hashing; and corruption rejection including a retained
  request-hash mismatch. Rejected decisions cannot reach the atomic complete-
  record replacement.
- Managed `canic_command` and `canic_status` add only the frozen
  `RuntimeWhitelist` variants. They read the transport caller once and complete
  controller-first, exact-stable-Root authorization before facade state access.
  `caller::is_whitelisted()` now reads only this stable authority; compiled
  configuration is seed input and 0.105 application sessions remain separate.
- Focused unit, conversion, role-allocation, source-boundary and Candid checks
  pass. The managed fixture compiles, and one bounded PocketIC journey proves
  compiled seeding, independent controller and Root authorization, unrelated-
  caller denial, response-loss exact retry, conflict rejection, immediate
  removal, same-release restoration without reseeding, re-addition without a
  rebuild and zero application-session creation.

## B7 Independent Result

- Extracted managed Candid contains the bounded command/status variants beneath
  the existing two method identities. Current Root, Coordinator, Store and
  canonically rebuilt standalone-local artifacts contain no runtime-whitelist
  type or selector.
- The generic fixture and bounded PocketIC journey cover authorized Root and
  controller administration, unrelated-caller denial, immediate access
  removal, response-loss replay, conflicts, restoration and application-
  session separation. Focused warning-denied Clippy passes for the owning core,
  facade, fixture and test target.
- The original read-only Toko snapshot was clean `main` at
  `bf14a5d3d89be4335d3da2601e8a60128fde04df`, with no Canic integration or
  CANIC identifiers. Final read-only inspection of the separate dirty Toko
  Miner working tree at `4cd7aa8c18e6edde4a9d28a3b4d23709ff542d3e`
  records CANIC-012 and CANIC-013 as verified. It records exact external
  blockers for CANIC-009's first real anonymous/locked installation exercise
  and CANIC-011's first installed-Fleet mutation/removal/restoration exercise.
  That evidence satisfies acceptance criterion 13's blocker alternative; no
  Toko file or live IC state was changed by the audit.
- B7 is reconciled against the completed B5 result and its downstream
  cold-cache feedback: the explicit live catalog-acquisition mode is present,
  ordinary planning remains cache-only, install acquires automatically and
  effective identity admission precedes builds. The closeout audit confirmed
  that implementation evidence and rejected only superseded publication and
  schedule wording under AC12. This correction removes that residue; the minor
  remains unaccepted until the exact documentation re-audit passes.

## Feedback Traceability

| Feedback | Owning batches | Closeout proof |
| --- | --- | --- |
| CANIC-009 early install identity | B3-B4, B7 | effective ICP identity is usable, non-anonymous and equal to the Fleet-input operator before build preparation |
| CANIC-011 runtime whitelist evolution | B1, B6, B7 | authorized bounded mutation plus denial, retry and restoration journeys |
| CANIC-012 target/Fleet-input-complete plan | B1-B4, B7 | plan/install input and stable snapshot-authority digest parity, cache-only plan default, explicit plan acquisition and automatic install acquisition |
| CANIC-013 catalog inconsistency diagnostics | B1, B5, B7 | structured provenance and truthful retry/effect rendering |

## Next Authorized Action

Request and complete the exact AC12 documentation re-audit for 0.107. Do not
begin 0.108 production implementation until that re-audit accepts minor
closeout. Keep 0.106 B2's external effects held pending their separate exact
authorization.
