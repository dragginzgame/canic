# Codebase Health Audit - 2026-07-12

## Scope

- Snapshot: published `v0.86.8` plus the current unreleased scaffold changes.
- Reviewed: production ownership and layering, repeated operator flows,
  filesystem mutation, project/path resolution, process-environment safety,
  dependency advisories, manifest hygiene, and the completed 0.86 structural
  boundaries.
- Excluded: full workspace tests, PocketIC, deployment, network fetches, and
  broad Wasm rebuilds under the repository's targeted-validation policy.

## Executive Summary

- Risk score: **4 / 10**.
- The three safety findings from the 2026-07-11 audit are closed: restore
  persistence is durable, build authority is command-local, and Canic-owned
  stable/wire CBOR uses a maintained implementation.
- The 0.86 production hubs are split, and the role-feature, state-allocation,
  descriptor, project-root discovery, and release-set authorities remain
  centralized.
- The current scaffold changes close the leading filesystem finding with one
  durable writer and one rollback path.
- The largest remaining duplicate production flow is operator error handling:
  commands repeatedly unpack the same host ICP and installed-deployment errors,
  while ICP CLI wording is classified in several consumers instead of its
  adapter.

## Findings

### Medium - Scaffold mutations were not failure-atomic

Evidence at the `v0.86.8` snapshot:

- Fleet creation wrote seven new files sequentially. A later write failure
  left the new fleet directory present, so retry was rejected as an existing
  target.
- Canister scaffolding wrote a new role crate, rewrote workspace membership,
  and then declared the fleet role. Failure after an earlier mutation left a
  partial scaffold.
- Workspace membership was rewritten with truncating `fs::write`, while the
  fleet configuration path already used the host's durable sibling-write
  primitive.

Current correction:

1. Scaffold rewrites use the existing host durable byte-replacement function.
2. Only the pre-existing workspace and fleet documents touched by the command
   are captured.
3. One private rollback function restores those documents and removes only the
   preflight-proven new scaffold directory.
4. A typed rollback error retains both the operation and cleanup failures.

The correction deliberately adds no journal, generalized transaction type,
recovery command, or compatibility path.

### Medium - ICP and installed-deployment failures are repeatedly translated

Evidence:

- Ten CLI functions exhaustively translate `InstalledDeploymentError` into
  command-local variants.
- Seven CLI functions exhaustively translate `IcpCommandError`; the host also
  performs a similar translation into `InstalledDeploymentError`.
- Most translations copy the same command and diagnostic strings, convert the
  same version failures, and choose a synthetic snapshot command label.
- ICP CLI wording is separately inspected by install, replica, live-list,
  metrics, and install-readiness consumers for already-installed,
  missing-project, local-network, missing-method, missing-canister-ID, and
  missing-Wasm cases.

Impact:

New host error variants require edits in many unrelated commands. Consumers can
also disagree about the meaning of the same ICP CLI diagnostic, recreating the
string drift that typed Canic-owned failures already removed.

Recommended hard cut:

1. Keep raw process output and sources in `canic-host::icp`.
2. Give that adapter one typed classifier for the exact external diagnostics
   Canic acts upon.
3. Carry host errors through command boundaries without reconstructing their
   fields in every command.
4. Keep command-specific exit codes, hints, and rendering local.
5. Delete the replaced translators and consumer-local ICP wording classifiers
   in the same slice.

Do not add a global error framework or classify arbitrary text outside the ICP
adapter.

### Medium - Test-only process environment mutation is not globally coordinated

Evidence:

- Release-set path tests and install-root tests contain 17 Rust 2024 `unsafe`
  blocks that mutate process environment variables.
- Those test areas use separate locks, so either lock covers only its own
  module while the environment remains process-wide.
- Production build authority no longer mutates the global environment.

Impact:

The unsafe surface is test-only, but parallel host tests can become flaky or
invalidate the safety assumptions around process-wide environment mutation.

Recommended hard cut:

- Preserve the existing public environment-reading path functions.
- Move their precedence decisions into small pure functions taking explicit
  override values and a start path.
- Test those pure inputs directly and delete the environment mutation helpers
  and locks.

Do not introduce a generalized project-context service. Project-root,
workspace-root, ICP-root, config-choice, and manifest discovery have distinct
contracts and already compose the same focused host owners.

### Low - Two canister manifests carried stale dependency-scan metadata

Evidence:

- Cargo-machete identified direct `candid` use despite it being listed as
  ignored in the test root and sharding-root stub manifests.
- Both manifests use Serde helper attributes through derives, which the static
  scanner cannot observe directly.

Current correction:

The two manifests now ignore only the macro-derived `serde` use and the
`ic-cdk` macro surface. Direct `candid` use is no longer suppressed.

## Duplicate-Flow Decisions

| Candidate | Decision | Reason |
| --- | --- | --- |
| Role features, capabilities, and memory allocations | KEEP CURRENT | The role-contract catalog already owns the mapping and descriptor owners join through typed keys. |
| Project, workspace, ICP, and config discovery | KEEP CURRENT | These are distinct precedence contracts composed from shared discovery helpers, not competing authorities. |
| Host and backup durable writers | KEEP SEPARATE | Each crate owns its persistence boundary; adding a shared filesystem crate would invert or widen ownership for little benefit. |
| CLI ICP/installed-deployment translators | CONSOLIDATE | They repeatedly reconstruct the same typed host failures across unrelated commands. |
| ICP diagnostic wording classifiers | CONSOLIDATE IN ICP ADAPTER | The external tool boundary should own its wording once; policy and commands should consume typed meaning. |
| List, metrics, and cycles thread joins | KEEP LOCAL | Only three small loops exist, and their keys, panic reports, and result semantics differ. A generic fan-out framework would obscure those differences. |
| Command-specific exit codes and hints | KEEP LOCAL | They are presentation policy for different commands, not duplicate transport logic. |

## Passing Boundaries

| Boundary | Result |
| --- | --- |
| Restore and backup recovery writes | PASS - durable sibling replacement is used. |
| Production process environment mutation | PASS - no production `set_var` or `remove_var` remains. |
| Canic-owned CBOR | PASS - direct `serde_cbor` ownership is gone; the remaining copy is transitive through IC crates. |
| 0.86 structural ownership | PASS - Medic, deploy-plan, and state-manifest responsibilities have focused owners. |
| Role/state mapping authority | PASS - no second feature, allocation, memory-ID, or descriptor mapping was found. |
| Dependency manifest scan | PASS after the two exact metadata corrections. |
| Security advisories | PASS with four unmaintained transitive warnings and no known vulnerability. |

## Recommended 0.87 Scope

1. Keep the implemented scaffold failure-atomicity correction as Slice A.
2. Make the host ICP adapter the one owner of external diagnostic
   classification and remove repeated command-side error reconstruction.
3. Remove test-only process environment mutation through explicit pure resolver
   inputs, without creating a project-context abstraction.

After those three bounded slices, close 0.87 and audit again. Do not turn the
line into another broad module-splitting program.

## Verification

- `cargo audit --no-fetch`: no known vulnerabilities; four unmaintained
  transitive warnings.
- `cargo machete`: passes after the two exact metadata corrections.
- Production unsafe/environment scan: no global environment mutation.
- Repeated-flow scan: ten installed-deployment translators, seven direct CLI
  ICP translators, and six consumer areas with ICP wording classification.
- Focused scaffold tests: 23 passed.
- Targeted `canic-cli` library Clippy: passed with warnings denied.
- Full test/PocketIC/Wasm matrix: not run under targeted-validation policy.
