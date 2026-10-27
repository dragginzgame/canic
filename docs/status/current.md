# Current handoff — 2026-10-27

## Latest release-gate diagnosis

The validation adapter expected the old order and omitted dependency admission
and fixture qualification. Its focused shell regression now passes after fixing
the expected sequence. Dependency-risk classification regressions and scoped
ShellCheck also pass. The audit gate now prints advisory/package/patch details
instead of exiting silently on cargo-audit status 1.

The maintainer accepted RUSTSEC-2026-0387 for the exact crypto-bigint 0.5.5
checksum recorded in the dependency-risk inventory. It now remains visible as
a warning and does not block release. Exact-match acceptance and changed-identity
rejection tests pass; replay of the captured audit passes. The captured
audit lists >=0.7.6 as patched; elliptic-curve 0.13.8 requires 0.5, through the
ECDSA/k256/p256/ic-agent graph. No dependency was changed; other vulnerabilities remain blocking.
Evidence: /tmp/tmp.AT1Up3yD87/audit.json, /tmp/canic-current-risk-diagnostic.log
and /tmp/canic-crypto-bigint-parents.log. No full release validation was rerun.

## Accepted scope and candidate

Continue the accepted 0.112 Blob/Wire hard cut using the
[closeout plan](../audits/working/0.112-push-readiness/plan.md) and its
[earlier structured qualification](../audits/working/0.112-push-readiness/owned-candid-cut.json).
The current base is main at `b9482ec52` with local supplier-adoption and lint fixes.
Own package versions remain 0.111.0; the open changelog target is 0.112.0.
Do not conflate the old dirty `cf2d3d1fb` evidence with this selected graph.

The maintainer accepts required external Candid. Owned endpoints, initialization
and authority/hash contracts use Wire. Removing redundant external DTO mirrors
remains intended; a zero-Candid checkout is not an acceptance requirement.
Production portable Auth, live Backup and ten unrelated unstarted simplifications
remain deferred with their unavailable paths closed. Preserve the eight completed
simplifications and their acceptance recorded in the plan.

## Current supplier adoption — workspace Clippy passed

The maintainer requested all latest IC releases and then explicitly instructed:
"dont bother testing just adopt and ill try and release". The subsequent explicit request to run Cargo Clippy and fix all findings
authorized the workspace lint gate. The subsequent reported allocation-peer
freshness failure was repaired through the owning refresh and verification commands.
The latest
published dependencies are adopted in the root and both independent Blob
consumer catalogs and locks, preserving existing features and unrelated work:

- Platform/macros 0.7.0; Blob Storage/Contracts 0.51.0; Auth/protocol types 0.18.0.
- Tooling/Host/Testkit 0.69.0; Query 0.75.0; Backup 0.45.0.
- Memory/Storage 0.51.10; Timers 0.31.10; Wire 0.7.18; Metrics 0.16.3.
- Shared Tooling 0.26.22 at 5fc904a935d82ad31f0fba14434b71ccb8c10345.
  Browser Platform remains the latest 0.1.1 with SDK Core 6.1.1 unchanged.

All three locks now select a single Platform 0.7.0 identity. The earlier Blob,
Backup and Query publication/type-identity blockers are superseded by the matching
published releases. No compatibility conversions, local overrides or fallback
codecs were added. Registry evidence and lock-update logs: /tmp/canic-deps-latest/.
Structured selection: latestSupplierAdoption20261027.

The final Auth/Blob/Platform graph passes `cargo clippy --locked --workspace
--all-targets --all-features -- -D warnings` through the governed workspace
runner. Log: `/tmp/canic-clippy-complete.log`. Fixes include grouped method IDs,
import/visibility and lint-expectation cleanup, and an explicit metrics rejection
result type for canisters with no compiled metrics families. No tests were run. Earlier passing checks qualify only their
recorded graphs. The allocation peer and its provenance have now been regenerated
for this selection. Both `refresh_embedded_root` and `verify_embedded_root` passed
via `cargo run --locked --offline -p canic-testing-internal --example <name>`.
All three Cargo locks stayed unchanged. Logs: `/tmp/canic-peer-refresh-current.log`
and `/tmp/canic-peer-verify-current.log`. This repairs the reported fixture gate;
no full validation or installed tests were rerun. Open
0.112 changelog notes reflect final selections; own package versions stay 0.111.0.
No staging, commit, push, release, deployment or paid operation was performed.
Remaining external DTO migration and the earlier issue-batch acceptance gaps are
recorded below and in the closeout plan; this adoption is not full release approval.

## Newly added issue batch

The maintainer requested working through the 25 newly added issues after #662.
Twenty-four are implemented locally; #678 is superseded by the owned Wire cut.
The issue inventory and invariants are recorded in the closeout plan. This batch
consolidates authority construction, pool transitions, bounded cycle reservations,
auth configuration, metrics, transport observations and Host/CLI mechanics.
No codec, protocol, error variant or storage layout is deliberately changed.

Targeted qualification has passed 940 native tests across 29 owning filters,
separate Root/Store Wasm compilation and strict library/test Clippy for the four
affected production crates. Four opt-in process/measurement tests remain ignored.
Four stale test assumptions were corrected: the native-Wire receipt golden,
query authority-observation count, release-reuse Wasm/schema fixture and a Medic
prose assertion. The receipt mismatch was reproduced with the original ordering
implementation before updating its golden. Allocation-peer regeneration and verification passed on that graph. The installed
provisioning case then failed its Cargo-input stability guard before installed
execution: the root lock changed during the command. New selections include
Tooling/Testkit 0.68.12, Memory/Storage 0.51.8, Timers 0.31.8 and Backup 0.44.8.
No dependency update was issued in this issue batch. Preserve the changed lock;
its owner is unconfirmed and clarification was requested. The independent locks
and changed Rust source hashes stayed unchanged. Retirement qualification has
not run. Prior passing checks do not qualify the changed root graph. Evidence:
issueConsolidation20261027; logs and both root-lock snapshots:
/tmp/canic-issues-663-689/ and /tmp/canic-deps-round2/root-before.lock.

No GitHub issue comment or closure, staging, commit, push or release is performed.
The existing external DTO migration remains the separate full-batch blocker.

## Supplier adoption qualified before the concurrent root-lock change

The latest maintainer-requested refresh selects Tooling/Host/Testkit 0.68.11,
Query 0.74.5, Backup 0.44.7, Memory/Storage 0.51.7 and Platform/Macros 0.6.21.
Timers 0.31.7, Wire 0.7.18, Blob Storage/Contracts 0.49.3 and Auth 0.17.1 stay
selected. Unpublished Timers 0.31.8, Persistence 0.51.8 and Blob 0.50.0 are
excluded. Both independent Blob consumers align their catalogs and locks.
The captured root lock already selected the new Rust versions and is preserved.
Existing feature selections remain unchanged. Shared Tooling stays 0.26.21.

The browser consumer now selects published Platform 0.1.1, preserving SDK Core
6.1.1 and pinned Node/npm/TypeScript. Locked installation and its SDK-boundary
check pass. Native checks pass: ten Query subnet-catalog, twenty-six Backup
restore-runner and sixteen memory cases. The exact managed_browser_compositions
PocketIC case passes for both dedicated and embedded consumers, including generated
clients and its malformed-funding path. It does not repeat the full funding-error
matrix. Allocation-peer verification found changed bytes; owning regeneration and
final verification pass. All three Cargo locks and the npm lock stayed unchanged.
Dependency boundaries, scoped doc links and diff checks pass. Structured evidence:
icFamilyRefreshSecond20261027. Logs and before-lock snapshots: /tmp/canic-deps-round2/.

The preceding icFamilyRefresh20261027 and supplierAdoption20261027 evidence
qualifies its recorded earlier graphs. Do not present it as fresh proof of the
latest selections. No DTO mirror is removed solely by changing dependency versions.

## Remaining DTO ownership work

The owning supply issues are [Platform #61](https://github.com/dragginzgame/ic-platform/issues/61)
(CMC/ICP transfer and receipts), [#62](https://github.com/dragginzgame/ic-platform/issues/62)
(Cycles creation/withdrawal), [#63](https://github.com/dragginzgame/ic-platform/issues/63)
(NNS lookup) and [#64](https://github.com/dragginzgame/ic-platform/issues/64)
(asset retrieval). Their published APIs still need actual Canic consumer
migration and qualification; do not close them on version selection alone.

The [prepared export patch](../audits/working/0.112-push-readiness/platform-external-contracts.patch)
adds management/history/ECDSA and ICRC transfer/consent identities from existing
upstream crates. Its isolated native/Wasm/MSRV/strict-library-lint evidence remains
in the plan as historical preparation evidence. Published Platform 0.6.21 now
supplies these exports, including the distinct ICRC-21 namespace. The supply gap
is resolved; actual Canic consumer migration and acceptance remain outstanding.

Capacity-import history already reuses available Platform ChangeDetails instead
of a local runtime enum. Three focused history tests and Root control-plane Wasm
compilation pass; exact origin, sender-version and controller checks remain.
Adopt the larger mirrors from their matching published public types with owning
consumer tests. Do not substitute differently shaped ledger contracts or introduce
serialization bridges.

## Retained qualification and safety

[Issue #662](https://github.com/dragginzgame/canic/issues/662) is implemented locally:
Core InternalError and Host IcpCommandError derive their mechanical formatting/
source implementations using existing thiserror. Five focused native Linux tests
pass for messages, exact source identity, projections and retained evidence.
Strict Core/Host library/test Clippy with all features and Core Wasm compilation
pass. A one-line Host status-fixture default-type spelling fixed an existing lint
found during that check. Logs: `/tmp/canic-662-{tests,clippy,clippy-rerun,wasm}.log`.
Formatting/diff checks pass; no full gate, macOS or installed test was run for this
presentation-only change. Custom conversions and other error types are untouched.

Earlier owned-cut qualification covers native/macro/ingress bounds, managed
initialization, both Blob compositions, generated clients, checkpoint restoration,
Root bootstrap/recycle/replacement, held replies, package consumers, timer custody
and Root deletion readiness. Those results remain attached to their recorded
source and dependency graphs; they are not fresh proof for this adoption.

The prior comparable dedicated fast Root measured 8,597,817 code-body bytes and
9,109,546 total bytes, 2.32% less code than its immediate pre-cut baseline. That
measurement predates this supplier graph. No new Wasm-size improvement is claimed.

The latest reported Clippy failures were corrected in four test/example files:
import placement, three unnecessary borrows and a redundant closure. Strict
Core/Contracts library/test/conformance-example Clippy passed before this adoption.
No production behavior changed in that lint correction; no broad gate was rerun.

Keep endpoint authorization, exact identity/controller binding, bounded spending,
original retry bytes, uncertain-effect reconciliation, same-release recovery and
cycle conservation. The unqualified Blob confirm_credit wrapper remains removed;
uncertain payments and replay obligations remain. Blob owns its service/funding
engine; Canic owns mounting, auth/caller context, lifecycle/memory composition and
installation identity. Canic's Wasm Store is its artifact registry.

Storage retains the accepted serde_cbor 0.11.2 hard cut: nested u128 values use
exactly 16 big-endian bytes; Wire keeps native numbers. No fallback, CBOR tag
feature or Postcard experiment was adopted. The advisory exception remains scoped.

No staging, commits, pushes, own-version changes, release receipts, deployments or
paid/provider effects are authorized by ordinary continuation. Preserve existing
work and sibling checkouts. Only focused validation is authorized; inspect host-
visible target ownership before source edits or builds. Full extraction readiness
still depends on the remaining external DTO adoption and its owning acceptance.
