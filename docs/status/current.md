# Current handoff — 2026-10-27

## Accepted scope and release authority

Continue the accepted 0.112 Blob/Wire hard cut using the
[closeout plan](../audits/working/0.112-push-readiness/plan.md) and its
[graph-specific evidence](../audits/working/0.112-push-readiness/owned-candid-cut.json).
The maintainer explicitly authorized staging all changes, committing as
`0.112.0`, running `make release-minor`, and repairing/retrying until completion.
Own package versions remain 0.111.0; the open changelog target is 0.112.0.
No version preparation, tag or push has occurred. Do not begin another minor.
The release command delivers the branch and tag; it does not publish crates or
perform installation/deployment. If version preparation succeeds before a later
failure, use the exact `make release-resume VERSION=0.112.0` recovery path.

## Current source and next action

Committed base `627af697e` contains the extraction repairs, requested Auth/Timers
adoption and aligned test-canister auth profiles. The fourth release attempt
passed all pre-test gates but stopped before PocketIC on two intermittent Host
activation-source handoff conflicts. Full output:
/tmp/canic-release-0112-attempt4.log. Retained failure scratch: .tmp/test-runtime.xgNjD9.
Both failures occurred at adoption/tests.rs stage(), with
ActivationResetAdoptionConflict. They are not diagnosed as repaired.

The four owning cases pass under ordinary and all-feature package selections.
The exact release binary passes 30 repetitions of those cases and its entire
Host library (1262 passed, 20 ignored); the rebuilt Host library also passes.
The local test change separately asserts plan identity and exact copied source
evidence before staging, improving the next failure's diagnostics. Scoped strict
Host Clippy passes. Logs: /tmp/canic-release-repair-activation*.log.
Commit the diagnostic assertions and retry the authorized release; investigate
any recurrence rather than relaxing identity checks or adding automatic retries.

The corrected package-contract regression passes through the governed scratch
wrapper. The installed Root baseline/restoration case passes, preserving the same
instance, identities, controllers, module hashes and current time. Embedded-peer
verification passes without another refresh. Logs:
/tmp/canic-release-repair-profiles{,-root,-peer}.log.
The complete release gate remains required; targeted passes do not establish it.

## Repairs retained for complete qualification

Artifact/schema production locks the shared Cargo target across independent
workspaces. Generated infrastructure is prepared before fixture source capture.
Manually mounted fixture endpoints emit compiled Wire declarations. Both audit
Root callers use one App-qualified generated package. Protected readiness uses
recorded controllers; Store snapshot readiness uses its public role overview.
Installed assertions retain deferred terminal completion, Store timestamps,
sealed Root rejection and internal recovery during paused Fleet modes.

Focused shared-target exclusion, installed state cascade, retained child-claim
recovery and Root baseline restoration pass. The second release attempt exposed
additional installed failures now repaired but still awaiting complete rerun:
managed lifecycle/checkpoint capture, dedicated Blob schema production, raw
fixture metadata, funding observations, initialization and Store assertions.
Attempt-two evidence remains in /tmp/canic-release-0112-attempt2.log and
`target/test-runs/20261027T135608Z-1799846.YTSnQu/`.
Earlier repair details remain in the committed handoff at `cb6869cc3`; logs and
build artifacts are retained. Do not relabel earlier graph evidence as current.

## Selected dependencies

Root and both independent Blob consumers select Platform/macros 0.7.0,
Blob/Contracts 0.51.0, Auth/protocol types 0.19.0, Memory/Storage 0.51.10,
Timers 0.31.11, Wire 0.7.18 and Metrics 0.16.3 where applicable.
Root Host/Tooling/Testkit is 0.69.0, Query 0.75.0 and Backup 0.45.1.
Shared Tooling: 0.26.23 at 48ec9794892da6f8e2812653ec16c5e8ea23df47.
Browser Platform: 0.1.1; SDK Core: 6.1.1. No local dependency overrides.
Auth restore uses typed login states; its qualification backend uses explicit
read/admit/logout and protected credential admission. All 34 session-storage
regressions pass. Production portable Auth and live Backup remain deferred.

## Safety and remaining scope

Preserve authentication, explicit identities/controllers, bounded spending,
original retry bytes, uncertain-effect reconciliation, same-release recovery and
cycle conservation. The unqualified confirm_credit wrapper remains removed;
uncertain payments and replay obligations remain. Blob owns its service engine;
Canic owns mounting, caller context, lifecycle/memory composition and installation.
Owned RPC, initialization and hash inputs use Wire. Required external Candid is
accepted; available upstream types own those contracts. A zero-Candid checkout
is not required. External DTO migration acceptance remains graph-specific.
Storage retains serde_cbor 0.11.2 with nested u128 as 16 big-endian bytes.
The exact crypto-bigint 0.5.5 advisory acceptance remains visible in the risk
inventory; it does not waive other vulnerabilities or dependency risk checks.
Current gzip production and fixtures use IC Tooling's canonical encoder.
Do not edit snapshot-owned tooling locally or opportunistically adopt newer
sibling work while qualifying this source. Preserve existing artifacts and logs.
