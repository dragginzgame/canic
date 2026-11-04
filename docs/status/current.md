# Current handoff — 2026-11-04

## Delivered 0.115.0

The maintainer explicitly authorized the 0.114 → 0.115 boundary exception,
requested making 0.115 live, and then instructed skipping the full release gate.
Delivery is complete: all nine publishable crates are verified on crates.io,
and all four annotated release tags and their commits are pushed to `origin/main`.

| Group | Tag | Commit | Published crates |
| --- | --- | --- | --- |
| Framework | `framework-v0.115.0` | `c36990f59` | Contracts, Core, Control Plane, Macros, Canic |
| Host | `host-v0.115.0` | `3f7237449` | Host, CLI |
| IcyDB | `icydb-v0.115.0` | `818627bc2` | IcyDB adapter |
| Blob | `blob-service-v0.115.0` | `abfb30a51` | Blob service adapter |

All receipts truthfully record `maintainer-waived`. The one-time authority is
`ci/release-exception-0.115.json`; it applies only to 0.115.0 and does not qualify
future releases. No full suite, full Clippy gate or fresh hosted CI pass is claimed.
Exact source/version/lock transactions, package boundaries, annotated tags and
atomic pushes passed. Every crate passed Cargo's packaged compilation before
upload; every exact version was observed on crates.io. No application Fleet was
deployed: this delivery published Canic source, tags and packages.

Release logs: `target/release-0115/`. Package logs and timings:
`target/publication-runs/`. Preserve `.git/release-state/`, release artifacts,
`.tmp/release-0114-20261031/` and the candidate named by
`.tmp/release-01141-20261102/candidate-path`. Do not revive cancelled 0.114.1.
Check active target users before input edits or compilation; no alternate checkout
is authorized. The next minor still requires its own maintainer decision.

## Shared Tooling and dependency family

Shared Tooling 0.29.5 was fixed, committed and published at
`574dc765f711fbed23a663d74f9de285857e343d`, tag `v0.29.5`, then adopted through
its exporter; all 164 snapshot files verified. Release admission now compares
local lock records to all resolved local packages while independently requiring
exact workspace/group membership. The focused upstream release-group suite and
ShellCheck passed, including excluded-helper version preservation. Unrelated
Shared Tooling edits were restored; the named notes-preservation stash remains.

IC Auth's delivered conformance helper stays upstream-owned, dev-only, version
0.33.1, excluded from workspace membership and every Canic release group. Its
source/provenance lives in `integrations/auth-backend-conformance`; Core's actual
stable-storage conformance test remains connected and passed.

The delivered graph selects Timers 0.40.3, Metrics 0.29.1/Metrics Storage 0.68.0,
Runtime/guard 0.19.1, Storage 0.66.2, Logging 0.24.6, Platform 0.22.0,
Types 0.18.1, Wire 0.16.2, Blob/contracts 0.71.1, Backup/Agent 0.58.0,
Host Tools 0.82.0 and Testkit 0.85.1. Host Artifacts/FS/Process are 0.74.3;
Text is 0.74.2. Query retains its independent native Platform 0.19/Host Tools
0.79 boundary; some suppliers retain Platform 0.21 facades. Selected Wasm graphs
have one Timers, Storage, Runtime and execution guard. Rust/Cargo 1.99.1 admission
passed; edition remains 2024. No dependency identity bridges were introduced.

## Timer acceptance and retained evidence

Timers owns callback execution. Canic retains durable business demand, paid-effect
custody, lifecycle reconstruction, bounded retry authority and snapshot fences.
Three cleanup passes consolidated non-root bootstrap into one native continuation,
made Store GC report typed callback failure while Root owns retries, and fixed
Coordinator directory confirmation to wait on an unchanged provisioned receipt
without weakening changed-authority rejection. No further demonstrated scheduler
duplication or raw-provider bypass was found.

Before delivery, focused native regressions, ownership guards, owning lint,
Backup checkpoint tests, installed registration-budget recovery, bounded Store GC,
both private Blob compositions and all 14 timer-authority cases passed. See
`timer_sediment_cleanup_20261104` in the [adoption receipt](../audits/working/0.112-push-readiness/upstream-adoption.json)
and `target/timer-final-cleanup/`. Preserve the failed E132 canister logs in
`.tmp/test-runtime.P3ape4/`; the corrected rerun passed. Historical receipts retain
their exact graphs; the release waiver does not upgrade their scope.

Earlier issue evidence covers #348/#165/#158, #94, #46, #183, #123,
timer-specific #346, #730, #150, #89, #398, #176, #545, #590 and #375.
Actual downstream database composition and broader endpoint inventory remain
separate. Contracts' passive Types boundary and explicit schema/build-support
features are qualified in `schema_build_features_20261104` and
`target/feature-boundaries/`; publication does not assert an upstream Wire
optional-derive cut.

## Separate limitations

The [Auth progress table](../features/authentication/README.md#progress) remains
canonical. Controlled-local adapter, issuer, browser, capacity and recovery proofs
pass; production clock/restore bounds, key custody/activation, full UI and combined
state cost remain open. Publication is not full production Auth acceptance.
PRs #701/#703 were locally incorporated, not merged by this delivery.
Live Backup/Restore #513/#394/#550, Blob provider acceptance #622/#494,
Jobs #502 (unmerged #722), durable Relay #631/#636, metrics/pagination cost and
macOS/hosted qualification remain separate. ICRC #706 stays explicitly mounted.
The release closes no GitHub issues automatically and starts no new minor.
