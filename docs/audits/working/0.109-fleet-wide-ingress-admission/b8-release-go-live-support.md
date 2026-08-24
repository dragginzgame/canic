# Canic 0.109 B8 Release And Go-Live Support

Date: 2026-08-24

## Revision and boundary

| Item | Value |
| --- | --- |
| Published predecessor | annotated `v0.108.2`, commit `dafc455339df92acb304072d3ec2b98c4069747d` |
| Validated source | `15508c770a10d30dccd65840e24dcf52b58e59d4` |
| Published base | annotated `v0.109.0`, commit `3cae3d2c95af087365d8b3fb096a505b6be9b418` |
| `CANIC-027` source | `f36326015d3b9fe3061d9545acebc46206870bdf` |
| Published correction | annotated `v0.109.1`, commit `44e90e6dd4fd9293f7f013cf58f3242c188620d2`; clean `main` and `origin/main` agree |
| Reopened correction | open 0.109.2 draft; post-publication path audit found the 0.109.1 successor validator unreachable from the earlier join/synchronization gates and the host polling loop able to exhaust its bound before scheduled retry |
| Canic effects | Repository source/documentation, local build artifacts and local PocketIC through the 0.109.2 candidate; maintainer-owned validation/version/publication only through 0.109.1 |
| Downstream evidence | Separately owned 0.109.1 dependency/CLI adoption and local qualification; no deploy, commit, push, canister or Ledger effect |
| Excluded | Live staging, paid effects and every audit-time external mutation from this repository |

Adversarial post-publication inspection reopened the in-repository correction:
0.109.1 contained the intended successor predicate, but the real restart order
could reject revision 4 before reaching it. The current 0.109.2 candidate fixes
that path and the bounded fresh-pool wait. B8 therefore still requires the
maintainer-owned complete release flow for 0.109.2 and a trusted downstream
zero-blocker plan against that exact package pair.

## Published package integrity

The crates.io registry reports both maintained public packages at exact version
`0.109.0`, non-yanked and with Rust `1.91.0` metadata:

| Package | Published at | Registry/archive SHA-256 |
| --- | --- | --- |
| `canic` | `2026-08-24T11:36:24.728439Z` | `2ae8d4b82c67034573f0e4fe73d289b32963651dbf7b4f0ca58dd53f5f7c7a72` |
| `canic-cli` | `2026-08-24T11:39:08.339814Z` | `60ad19f10771391fad0a68c764973d09979404f18189e2e8aba5548ebb3625cc` |

`cargo info --registry crates-io` resolved both exact public versions. The
downloaded archive hashes match the registry checksums. Their normalized
packaged manifests retain exact `0.109.0` Canic-family dependencies: `canic`
binds `canic-control-plane`, `canic-core` and `canic-macros`; `canic-cli` binds
`canic-backup`, `canic-core` and `canic-host`. This closes package presence,
version equality and internal-family pinning without relying on the workspace
path dependencies.

An isolated public-registry installation also completed successfully:

```text
cargo install --locked canic-cli --version 0.109.0 --root <isolated-temp-root>
# Installed package `canic-cli v0.109.0` (executable `canic`)

<isolated-temp-root>/bin/canic --version
# canic 0.109.0
```

The install compiled the registry-resolved `canic-core`, `canic-backup`,
`canic-control-plane`, `canic-host` and `canic-cli` 0.109.0 packages. It did
not use this workspace's path dependencies or mutate an installed operator
toolchain.

### Published 0.109.1 correction

The maintainer-owned release flow published `v0.109.1` from clean commit
`44e90e6dd4fd9293f7f013cf58f3242c188620d2`. Toko Miner independently resolved
the complete 0.109.1 Canic graph, pinned its qualification dependency and
developer CLI to 0.109.1, verified the immutable remote tag and confirmed the
installed CLI version. This supersedes the 0.109.0 package checkpoint for the
remaining B8 plan proof.

## `CANIC-109-GOLIVE-001`: fresh pool capacity

Root Component-batch acceptance no longer rejects a fresh installation solely
because the Ready pool is empty. After validating the exact Coordinator,
Registry mirror, compiled batch, Root Store and artifacts, the workflow runs
one bounded pass of the existing Root-owned Canister-pool maintenance journal.
It then revalidates the same Registry acceptance authority before retaining the
batch. If capacity is still not Ready, the existing Coordinator retry resumes
the same durable operation and the next pass advances the same Root journal.

This creates no host-side pool seeder, timer, creation journal or alternate
Ledger owner. IC-profile creation continues through the production
Cycles-Ledger operation identity, cost guard, pending creation record and
reset-to-Ready lifecycle. The Root's existing asynchronous lease prevents a
timer, manual command and Coordinator-driven pass from overlapping.

The host still owns a finite observation bound, but an unchanged durable status
now causes a one-second wait before the next query. That matches the
Coordinator's scheduled retry cadence instead of spending the complete bound
in a tight query loop. Changed status is recorded immediately, and persistent
non-progress still terminates within the existing plan-scaled count.

The 0.109.2 production-shaped PocketIC proof begins with zero imported pool
assets and an exact five-Component initial batch. The Cycles Ledger stub returns
five exact pre-created canisters under the Root; the Root issues five distinct
Ledger requests, adopts and resets each canister, accepts the batch and
provisions all five Components. Acceptance drives the sole pool journal toward
the exact batch demand, even when that demand exceeds the background
maintenance minimum, and rejects a demand above the immutable standby maximum.
While the Root-owned
provisioning journal is still advancing, protected Coordinator status exposes
the exact Root, `RootProvisioning` stage, registered `STATE_CONFLICT` code and
nonzero failure timestamp. Terminal provisioning clears the pending failure
projection.

A separate local-profile journey uses the same high-level Coordinator
`ProvisionComponents` command and Root journals with Root-owned Ready capacity.
It publishes the initial Fleet-service successor at Registry revision 4 and
reaches `RuntimesActivated` with one terminal Root receipt. This separates the
IC Cycles-Ledger proof from the local runtime activation proof instead of
faking NNS-dependent IC activation in a minimal PocketIC topology.

## `CANIC-109-GOLIVE-002`: monotonic Registry successor recovery

The 0.109.1 implementation performed this check only after its earlier join and
Root-synchronization recovery gates, so a legitimate revision-4 successor was
still rejected before the validator was reachable. The 0.109.2 candidate moves
that decision into one shared recovery owner used by join, synchronization and
activation.

Host restart accepts exact all-Joining or all-Active replay without any
successor exception. When live Registry state differs, each gate recompiles the
exact fresh Component-provisioning plan from the immutable install plan,
configuration and retained all-Active predecessor. It queries the exact
installed Coordinator's protected operation status and accepts only a live
Registry that:

- is canonical under the protected topology and its own manifest/version;
- advances the predecessor by exactly one revision;
- preserves the exact Fleet authority, admission policy, Component Specs and
  Root entries;
- is the first nonempty service publication from an empty-service predecessor;
- is bound to the deterministic install operation ID and compiled plan hash;
- names the exact predecessor and live published Registry versions; and
- retains a fresh-install phase at or after `ServiceTopologyPublished`.

Focused tests retain exact Joining and all-Active success plus exact-successor
success at the earliest recovery boundary, and reject missing Coordinator
evidence, pre-publication evidence, a substituted plan hash and a later
Registry revision. A host install-order guard requires all three pre-
provisioning gates to call the shared authority. The existing activation
journal tests continue to reject changed source or response authority and
recover one exact atomic activation result.

## `CANIC-109-GOLIVE-003`: typed retry evidence

The Coordinator provisioning record now retains at most one typed Root retry
failure: exact Root Principal, closed retry stage, registered compact
diagnostic code and failure timestamp. The scheduled workflow records that
value before rescheduling a failed Root step. Status projects it only when it
matches the current durable in-flight Root/stage and follows that intent's
start time. Later progress cannot expose a stale failure as current, and the
terminal compact scale-out receipt contains none.

Stable validation binds every retained failure to a Root in the exact plan,
rejects zero diagnostics and regressed timestamps, and preserves the record
across same-release restart. The canonical Coordinator Candid contains the
closed four-stage enum, bounded failure record and optional protected status
field. No log text becomes a protocol or replay authority.

## Focused validation

Published 0.109.0/0.109.1 ordinary checks:

```text
cargo test --locked -p canic-control-plane component_provisioning --lib -- --nocapture
# 19 passed

cargo test --locked -p canic-control-plane fleet_coordinator --lib -- --nocapture
# 44 passed; 15,069-byte maximum Root command and 5,565,526-byte Registry history

cargo test --locked -p canic-control-plane fleet_admission --lib -- --nocapture
# 11 passed; 2,055,610-byte maximum Coordinator admission record

cargo test --locked -p canic-control-plane root_admission --lib -- --nocapture
# 6 passed; 9,247-byte maximum target command and 7,807,793-byte Root journal

cargo test --locked -p canic-host fleet_component_provisioning --lib -- --nocapture
# 7 passed

cargo test --locked -p canic-host fleet_registry_activation --lib -- --nocapture
# 3 passed

cargo test --locked -p canic --test protocol_surface fleet_coordinator -- --nocapture
# 4 passed

cargo test --locked -p canic-testing-internal \
  pic::governed_suite::governed_fast_internal_suite --lib -- \
  --ignored --exact --nocapture
# 1 runner passed; all five ordinary internal checks passed and the serial
# PocketIC inventory was fixed at 30 unique ordered cases

cargo clippy --locked -p canic-core -p canic-control-plane -p canic-host \
  -p canic -p canic-testing-internal --all-targets -- -D warnings
# passed
```

Published 0.109.0 targeted PocketIC checks:

```text
bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::fleet_registry::baseline::tests::fresh_component_acceptance_drives_the_root_owned_pool_before_effects
# 1 passed in 63s including rebuilt artifacts; 406,356 kB high-water, 19 threads

bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::fleet_registry::baseline::tests::fresh_component_provisioning_reaches_runtime_active_with_root_owned_capacity
# 1 passed in 14s with cached artifacts; 409,588 kB high-water, 19 threads
```

### Open 0.109.2 candidate

```text
cargo check --locked -p canic-host -p canic-control-plane
# passed

cargo test --locked -p canic-host fleet_registry_activation --lib -- --nocapture
# 5 passed

cargo test --locked -p canic-host fleet_component_provisioning_install --lib -- --nocapture
# 2 passed

cargo test --locked -p canic-control-plane canister_pool --lib -- --nocapture
# 17 passed

cargo test --locked -p canic-control-plane component_provisioning --lib -- --nocapture
# 19 passed

cargo test --locked -p canic-host install_truth --lib -- --nocapture
# 38 passed after the three-gate recovery guard was added

cargo clippy --locked -p canic-host -p canic-control-plane \
  -p canic-testing-internal --lib --tests -- -D warnings
# passed
```

```text
bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::fleet_registry::baseline::tests::fresh_five_component_acceptance_seeds_the_root_owned_pool_before_effects
# 1 passed in 151s including cold artifact builds; 413,040 kB high-water,
# 19 threads

bash scripts/ci/run-with-test-scratch.sh \
  bash scripts/ci/run-workspace-tests.sh targeted-pocketic \
  pic::fleet_registry::baseline::tests::fresh_component_provisioning_reaches_runtime_active_with_root_owned_capacity
# 1 passed in 20s with mostly cached artifacts; 402,060 kB high-water,
# 19 threads
```

The complete maintainer-owned release gate has not been run on the 0.109.2
candidate and is not claimed here.

The broader Coordinator filter initially found a test-only terminal admission
fixture with an arbitrary nonzero Root receipt hash. The fixture now derives
the exact operation/policy/catalog-bound receipt and the complete 44-test
filter passes. The governed fast tier then caught its stale 25-case inventory
expectation before PocketIC; the exact current inventory is 30. Warning-denied
Clippy found and closed current-0.109 cfg, pattern, future-size and explicit
long-proof annotations without weakening runtime authority.

The maintainer-owned release flow subsequently completed the complete gate at
source commit `15508c770a10d30dccd65840e24dcf52b58e59d4` and published the
uniform 0.109.0 workspace as annotated tag
`v0.109.0` at `3cae3d2c95af087365d8b3fb096a505b6be9b418`.

## Read-only downstream checkpoint

The post-publication read-only Toko Miner check found its workspace at commit
`e61c15b54afd04744611724408dcceeae65dab7d` with extensive unrelated dirty
work. Its workspace still requests Canic `0.108`, its lockfile resolves
`0.108.0`, and its App configuration still contains `[app.whitelist]` without
the 0.109 admission-participant declaration. No downstream source, dependency,
generated artifact, identity or IC state was changed, and no downstream test or
deployment effect was run. This proves publication is complete but downstream
adoption evidence was absent at that checkpoint.

## Qualified-adoption feedback

The downstream maintainer subsequently reported that exact-0.109.1 `make ci`
passed 52 Rust tests, every Wasm and Candid check, both managed/standalone
PocketIC journeys and 121 frontend tests. The exact managed App and standalone-
local Wasms prove admitted, denied, anonymous, fenced and same-release upgrade
behavior, including retained caller-owned IcyDB state and reconstructed
authority and timers.

`CANIC-027` identified the Canic-owned cause: workspace source identity asks Git
for cached plus accepted untracked paths, so an index-tracked file deleted only
from the worktree remained in the candidate list and failed the subsequent file
read. The forward correction treats that exact absent worktree entry as absent
source while retaining current bytes for tracked modifications and nonignored
untracked files. Invalid relative paths, symlinks, non-files and I/O failures
other than `NotFound` remain fail-closed. Focused authority tests cover exact
deleted-state replay, restoration, tracked modification, accepted untracked
source and ignored untracked source; the complete owning planning filter and
warning-denied host Clippy pass.

The 0.109.1 planner crossed Toko Miner's current dirty-worktree source stage and
reached protected ICP identity observation. The run then stopped because its
process did not receive the operator-owned absolute
`CANIC_ICP_IDENTITY_PASSWORD_FILE`. It therefore produced no canonical 0.109.1
plan digest, fee/balance review or complete `not_executed` effect list. The
wrapper restored the anonymous identity and caused no canister, Fleet, Ledger
or deployment effect. `CANIC-027` is adopted but conservatively not verified.
The later Canic path audit reopened the deployment correction, so the next
accepted downstream plan must use the exact published 0.109.2 pair; the failed
credential observation remains historical 0.109.1 evidence.

The remaining feedback is routed without widening B8 or adding product
capability to B9:

| Feedback | Disposition |
| --- | --- |
| `CANIC-026` supported managed-App qualification harness | Scheduled as a separate bounded B10 support batch after B9, so the pure simplification batch does not acquire another product capability. It must replace downstream private payload/lifecycle plumbing without creating runtime authority. |
| `CANIC-028` named-environment artifact advice | Non-blocking operator defect assigned to B9 cleanup: preserve exact selected-root observation, distinguish pre-install assumptions and remove the impossible `canic build` remediation when that command cannot populate the observed root. |
| `CANIC-006` state-preserving release transition | Already owned by scheduled 0.111's exact stop-the-world predecessor-to-successor transition. It remains blocked on the accepted 0.109 and 0.110 gates. |
| `CANIC-005` application retirement acknowledgement | Already owned by scheduled 0.110 B2. It remains blocked on 0.109 closeout and explicit 0.110 promotion. |
| `CANIC-015` saved plan consumption | Accepted as unscheduled deployment ergonomics; it is not an admission-authority or current planning-correctness blocker and receives no implementation authority here. |
| Long-running multi-Subnet local showcase | Useful future qualification work, but not part of the bounded 0.109 admission release. |
| Generic Fleet observatory | Already owned by scheduled 0.112 and remains blocked on its predecessors. |

## Result

The 0.109.2 in-repository correction and its targeted evidence are complete.
B8 remains open for the maintainer-owned complete validation/version/
publication flow and the exact zero-blocker 0.109.2 no-effect plan from the
trusted downstream operator shell. B9 remains blocked until that evidence is
accepted; B10 and 0.110 remain blocked behind B9.
