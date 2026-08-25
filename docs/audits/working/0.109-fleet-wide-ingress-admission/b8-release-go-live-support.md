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
| Published recovery correction | annotated `v0.109.2`, commit `b48181ba2b4b0d9f679386ddb62179ba498b8ee2`; remote tag object `94aa4d190d142acda51eac35a19234fa0509b8bd` |
| Published artifact-reuse correction | annotated `v0.109.3`, commit `d861febb00bed2ca8145b705f82d836dc0ae6686`; downstream recovery crossed immutable retained-artifact reuse without Cargo or artifact mutation |
| Published provisioning-recovery correction | annotated `v0.109.4`, commit `2a5ef929b76bc29e62d19cbe1785690dc0d51fd2`; the downstream retained session crossed monotonic first-status reconciliation |
| Open pool-eligibility correction | 0.109.5 hard-cut changelog draft over published 0.109.4; exact Ready matching, undersized-Ready reinspection, first-excess planning, explicit recovery-pair and governed PocketIC balance-preservation tests pass; maintainer validation/version/publication not run |
| Canic effects | Repository source/documentation and local build artifacts plus maintainer-owned validation/version/publication through 0.109.3; no downstream effect from this repository |
| Downstream evidence | Separately owned exact-0.109.3 graph/CLI adoption, exact-source zero-debit plan and authorized recovery; retained-artifact reuse, Registry activation and Root preparation advanced before `CANIC-033`; App/frontend effects did not start |
| Excluded | Any new live staging, resume, paid effect or other external mutation from this repository |

Adversarial post-0.109.1 inspection reopened the in-repository correction:
0.109.1 contained the intended successor predicate, but the real restart order
could reject revision 4 before reaching it. Published 0.109.2 fixes
that path and the bounded fresh-pool wait. The subsequent 0.109.1 install also
proved that recovery incorrectly rechecked the original maximum debit before
replaying durable creation journals. The same candidate now owns that
`CANIC-029` correction. Exact downstream qualification and recovery-plan review
pass. The authorized resume then exposed `CANIC-031` before any IC update: the
host accepted the retained 0.109.1 builder but next routed that source through
ordinary 0.109.2 role-version validation. Published 0.109.3 makes finalized
reuse read-only and distinct from fresh builds, and the exact downstream
recovery crossed that gate. It then exposed `CANIC-033`: Coordinator-owned
zero-delay advancement raced the host's requirement that its first correlated
status remain `Planned`. The open correction retains any exact monotonic first
status and explicitly admits 0.109.4 as the next host successor for the same
retained 0.109.1 session. Published 0.109.4 crossed that boundary. Subsequent
authorized funding and import work exposed `CANIC-035`: the accepted 5T App is
blocked behind a full pool containing 2T and 4.5T Ready rows. Open 0.109.5 owns
the hard-cut plan, acceptance, typed-diagnostic and Ready-row refresh
correction. Its focused pure/host and governed PocketIC evidence now passes.
B8 therefore remains open for maintainer validation/publication, then the
separately authorized Root upgrade, asset refresh, exact-session resume and
deployed-state/admission evidence.

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

### Published 0.109.2 recovery correction

The maintainer-owned release flow published `v0.109.2` from clean commit
`b48181ba2b4b0d9f679386ddb62179ba498b8ee2`. Toko Miner independently resolved
all four Canic crates plus its exact support and CLI pins to 0.109.2. Its
complete `make ci` passed 52 Rust tests, both PocketIC journeys, 121 frontend
tests, Candid parity, Wasm checks and the production build.

The read-only current-workspace plan correctly classified paid-effect recovery,
fenced all three creations, required zero additional cycles and named Store-
bootstrap verification as the next phase. It then rejected the later workspace
source decision. A disposable exact `v0.2.0` checkout with physically confined
recovery evidence reconstructed the original digest and produced a blocker-free
warning-only plan. No resume, deployment, Ledger, frontend or other external
effect occurred.

### Published 0.109.3 artifact-reuse correction

The maintainer-owned release flow published `v0.109.3` from clean commit
`d861febb00bed2ca8145b705f82d836dc0ae6686`. Toko Miner independently adopted
the complete graph and CLI, reproduced its immutable `v0.2.0` source decision,
three fenced creations and zero remaining debit, and authorized the exact
resume. The host reused finalized 0.109.1 artifacts in 0.13 seconds without
Cargo or artifact mutation, closing `CANIC-031`'s reachability defect.

The live recovery advanced Registry and Root preparation before stopping at
Coordinator Component provisioning. The host retained
`preparation_in_flight`, while the Coordinator's zero-delay timer had already
advanced the exact operation beyond `Planned`. No App, Fleet catalog, frontend
or fixture publication occurred.

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
still rejected before the validator was reachable. Published 0.109.2 moves
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

### Published 0.109.2 candidate evidence

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

cargo test --locked -p canic-core --test timer_inventory_guard -- --nocapture
# 16 passed after the release-gate inventory correction

cargo clippy --locked -p canic-host -p canic-control-plane \
  -p canic-testing-internal --lib --tests -- -D warnings
# passed
```

`CANIC-029` targeted checks on the current working candidate:

```text
cargo check --locked -p canic-host -p canic-cli
# passed

cargo test --locked -p canic-host fleet_install_plan::tests -- --nocapture
# 16 passed

cargo test --locked -p canic-host install_root::fleet_install_recovery::tests -- --nocapture
# 2 passed

cargo test --locked -p canic-host install_root::coordinator_install_journal::tests -- --nocapture
# 4 passed

cargo test --locked -p canic-host install_root::fleet_subnet_root_install_journal::tests -- --nocapture
# 9 passed

cargo test --locked -p canic-host install_root::fleet_install_session::tests -- --nocapture
# 4 passed

cargo test --locked -p canic-host install_recompiles_the_exact_plan_digest_and_rechecks_live_funding -- --nocapture
# 1 passed

cargo test --locked -p canic-host retry_tests -- --nocapture
# 3 passed

cargo test --locked -p canic-host install_truth --lib -- --nocapture
# 38 passed

cargo test --locked -p canic-core --test timer_inventory_guard -- --nocapture
# 16 passed

cargo test --locked -p canic-cli deploy::plan::tests -- --nocapture
# 15 passed

cargo test --locked -p canic-cli deploy::tests::plan -- --nocapture
# 25 passed

cargo clippy --locked -p canic-host -p canic-cli --all-targets -- -D warnings
# passed
```

These were focused candidate checks. The later maintainer-owned complete gate
and immutable publication supersede their release-readiness limitation.

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

The first complete maintainer-owned release-gate run reached the ordinary test
barrier and found two source-inventory omissions: background/watchdog pool
maintenance no longer matched the canonical fresh-config-read expression, and
the new host sleep was absent from the governed timed-wait inventory. No
runtime or PocketIC case failed; the ordinary barrier correctly skipped the
serial PocketIC suites. The pool owner now retains its no-argument
authoritative minimum path while batch maintenance keeps exact demand, the
wait inventory is explicit, and the affected 16-test guard plus 17 pool tests
pass. A complete maintainer rerun remains required and is not claimed here.

The broader Coordinator filter initially found a test-only terminal admission
fixture with an arbitrary nonzero Root receipt hash. The fixture now derives
the exact operation/policy/catalog-bound receipt and the complete 44-test
filter passes. The governed fast tier then caught its stale 25-case inventory
expectation before PocketIC; that release-candidate inventory was 30. Warning-denied
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
The later Canic path audit reopened the deployment correction. The failed
credential observation remains historical 0.109.1 evidence; it was superseded
by the separately authorized fresh-install attempt described below.

## `CANIC-029`: retained-session recovery

The downstream 0.109.1 attempt durably created, funded and installed the exact
Coordinator, Root and Wasm Store. Its Coordinator creation journal is verified;
the Root journal is retained at sequence 15 in `store_bootstrapped`; and the
exact protected Store-bootstrap status query now succeeds and matches the
journal. The App was not created, the frontend was not changed, the default
identity was restored to anonymous and the operator balance is 14,788 cycles.
Those are downstream observations, not effects performed by this audit.

Published 0.109.1 detects that effects started but still requires the original
310,000,300,000-cycle maximum before journal replay. The correction introduces
one read-only recovery inspector over the exact immutable install session,
retained plan, artifact manifest and Coordinator/Root journals. It computes
the remaining debit with checked arithmetic: a creation intent beyond
`Planned` fences that amount and its exact Cycles Ledger creation fee forever.
An in-flight creation is likewise fenced against a duplicate debit but remains
an explicitly uncertain observation-only outcome. Exact replay is still
required; journal state is not inferred from a live canister alone.

Both `canic deploy plan` and `canic install` recompile the retained decision
authority and canonical digest. The original maximum remains in the plan;
only the live-balance admission check uses the journal-derived remainder. A
recovery report names the operation/session identity, exact retained 0.109.1
release build and builder, original digest, total and fenced creation counts,
uncertain outcomes, next replay phase, original maximum and remaining debit.
Once validated, that recovery section survives a later identity or decision
blocker, so a blocked report cannot regress to `no_effects_started: true`.
The only cross-patch allowance is 0.109.2 host recovery of an exact validated
0.109.1 session using its retained 0.109.1 artifacts. It does not upgrade or
migrate a canister and does not establish a general compatibility path.

The Store-bootstrap update remains outside the observation retry. Only a typed
`STATE_UNAVAILABLE` query is retried, at most five exact attempts with a one-
second wait; other failures return immediately. Later process recovery retains
the same Root-journalled operation identity. A disposable read-only
test against the reported downstream recovery directory reproduced all three
operator creations as fenced, zero remaining operator debit and
`fleet_subnet_root:pae4o-...:store_bootstrap_verification` as the next phase.
The test was removed after the proof; it performed no repository or network
mutation.

## `CANIC-031`: finalized-artifact recovery reachability

The separately authorized exact-source resume accepted the supported
0.109.1-to-0.109.2 retained builder pair and then failed in `BuildInputs` with
`role_contract_canic_version_mismatch` for both configured roles. The ordinary
snapshot resolver validated the predecessor consumer graph as though it were a
fresh 0.109.2 build before the later finalized-artifact branch could run. The
failure preceded every IC update; the confined and authoritative recovery
trees remained byte-identical and the downstream identity returned to
anonymous.

The open correction gives install snapshot resolution three explicit sources:
deployment plan, fresh workspace build and finalized release. Only the fresh
workspace branch invokes the current role build-spec resolver. Finalized
recovery instead validates the current exact App topology and declared package
names against the retained application union and infrastructure manifest,
requires every infrastructure entry to name the retained builder, reconciles
the Root release-set with the application union and checks its finalized digest
through a regular no-follow read. Before and after deployment-truth
preparation, it re-materializes every raw/gzip application and infrastructure
artifact through the existing immutable validators. That path neither invokes
Cargo nor rebuilds or rewrites an artifact or manifest.

The retained-session regression contains a 0.109.1 consumer lock graph,
finalized 0.109.1 release record, exact application/infrastructure/root
manifests, artifact bytes and interrupted Fleet session. Under the current
host it crosses finalized manifest admission into exact session-journal replay
with the complete fixture tree unchanged. The same proof rejects changed
package identity, topology, artifact bytes and root-manifest digest. The exact
pair test admitted only same-version recovery plus the named
0.109.1-to-0.109.2/0.109.3 rescues and rejected other predecessors or
successors. All 124 then-current focused `install_root` tests, the five
`release_build` tests and warning-denied `canic-host` library/test Clippy pass.
The complete maintainer gate published this correction in `v0.109.3`, and the
exact downstream session crossed finalized-artifact reuse without Cargo or
artifact mutation. Final verification remains coupled only to the separately
open Fleet completion evidence.

## `CANIC-033`: monotonic first provisioning status

The authorized 0.109.3 resume completed immutable artifact reuse, Registry
activation and Root Component-registry preparation. The host durably retained
its Component-provisioning preparation intent and submitted the exact
Coordinator command. That command intentionally schedules Coordinator-owned
advancement with a zero-delay timer before returning its operation receipt.
The host's first protected status query therefore observed the same exact
operation after it had advanced beyond `Planned`, but
`record_component_provisioning_prepared` rejected every other phase.

The correction keeps the existing host journal as the sole recovery owner. It
validates the same operation, plan hash, Fleet Registry, configuration,
operation kind, plan cardinality and bounded counts, then retains every exact
nonterminal Coordinator phase as the source of the next durable advance
request. A first `RuntimesActivated` observation must pass the existing
complete terminal-evidence validator and moves directly to the terminal host
phase, so restart proceeds to catalog publication without another provisioning
request. Mismatched identity/cardinality and incomplete terminal evidence still
reject before journal replacement.

The explicit retained-install successor list now admits 0.109.2 through
0.109.5 hosts for the one 0.109.1 predecessor. Same-version replay remains
unchanged; 0.109.6, other predecessors and reversed pairs reject. This does not
create a general migration or mixed-version runtime lane.

Focused tests recover the durable `preparation_in_flight` journal after a lost
prepare response, reconcile each nonterminal phase, bind the next request to
that exact observation, accept only complete terminal evidence and reject
substituted authority/cardinality. All 126 install-root tests and warning-
denied host library/test Clippy pass.

```text
cargo test --locked -p canic-host fleet_component_provisioning --lib -- --nocapture
# 10 passed

cargo test --locked -p canic-host install_root::fleet_install_recovery::tests --lib -- --nocapture
# 2 passed

cargo test --locked -p canic-host install_root:: --lib -- --nocapture
# 126 passed

cargo clippy --locked -p canic-host --lib --tests -- -D warnings
# passed
```

Published 0.109.4 crossed this first-status boundary in the retained session.
No fresh downstream staging attempt is appropriate. The later pool blocker is
owned by the open correction below; only after its immutable publication and
separately authorized recovery may Fleet/App, frontend and fixture publication
proceed.

## `CANIC-035`: exact pool eligibility and retained Ready refresh

The retained Root accepted one App requiring 5T, then became full with Ready
assets recorded at 2T and 4.5T. Count-only acceptance considered the pool
sufficient, while the later exact claim correctly found no eligible asset. An
external top-up could not repair the durable row because published 0.109.4
short-circuited import of an already-Ready principal and rejected retry-reset
for that state.

The hard-cut correction keeps the Root pool journal as the sole physical-asset
owner and the accepted Root batch as the sole provisioning owner:

- no-effect planning requires each Root's configured pool-asset amount to
  cover the largest `initial_cycles` requirement among its initial placements;
- Root acceptance greedily matches distinct sorted Ready balances to the exact
  sorted batch demands, so heterogeneous assets are not treated as fungible;
- a full ineligible pool returns typed `CAPACITY_INSUFFICIENT` after protected
  observer authorization, allowing the existing Coordinator retry record and
  host report to preserve the cause;
- the existing controller-only import/reset command derives the larger of the
  configured floor and every remaining unclaimed demand. An undersized Ready
  import is fenced as `PendingReset`; because the prior Ready record already
  proves it empty and Root-controlled, retry rechecks its exact controller,
  absent module and live balance without repeating uninstall effects. Only an
  adequate observation returns it to `Ready`; and
- exact retry while queued is idempotent, while replay after an adequate Ready
  result returns without another reset or claim.

No stable record or memory allocation changed. The maintainer explicitly chose
the pre-1.0 hard-cut direction: recovery speed and cycle preservation take
precedence over a generalized compatibility lane. The retained Root must be
upgraded, not reinstalled, so its existing pool rows, accepted batch and
operation IDs survive. The repair does not transfer or discard pooled cycles.
The downstream top-up must cover the exact freshly observed deficit plus its
transfer fee rather than assuming a rounded 500B amount; it and every
deployment command remain separately authorized downstream effects.

Focused evidence currently passes:

```text
cargo test --locked -p canic-control-plane ops::canister_pool::tests --lib -- --nocapture
# 10 passed

cargo test --locked -p canic-control-plane workflow::component_provisioning::tests --lib -- --nocapture
# 5 passed

cargo test --locked -p canic-control-plane workflow::canister_pool::tests --lib -- --nocapture
# 4 passed

cargo test --locked -p canic-host initial_group_placements_are_explicit_complete_and_durable --lib -- --nocapture
# 1 passed

cargo test --locked -p canic-host cross_patch_rescue_is_exactly_1091_to_explicit_successors --lib -- --nocapture
# 1 passed; admits 0.109.5 and rejects 0.109.6

bash scripts/ci/run-with-test-scratch.sh bash scripts/ci/run-workspace-tests.sh \
  targeted-pocketic \
  pic::fleet_registry::baseline::tests::topped_up_import_refreshes_the_ready_row_without_losing_cycles
# 1 passed in 112s through the governed PocketIC server;
# high-water 396,352 kB, 19 threads
```

The PocketIC journey uses the real Root management path: a never-verified
underfunded import first completes the full reset and stays failed, the exact
remaining deficit is added, the same principal becomes Ready at 5T, and a
second protected import leaves both its recorded and live balance unchanged.
It raises the unique governed serial inventory from 30 to 31 cases.
The active-batch threshold is separately covered by the exact demand resolver
and remaining-demand unit proof; the stable pool and provisioning record shapes
have no 0.109.1-to-candidate diff. Maintainer validation, versioning and
publication remain unrun, and the downstream live upgrade/top-up/resume remains
a separately authorized effect rather than release qualification performed by
this repository.

The remaining feedback is routed without widening B8 or adding product
capability to B9:

| Feedback | Disposition |
| --- | --- |
| `CANIC-026` supported managed-App qualification harness | Scheduled as a separate bounded B10 support batch after B9, so the pure simplification batch does not acquire another product capability. It must replace downstream private payload/lifecycle plumbing without creating runtime authority. |
| `CANIC-028` named-environment artifact advice | Non-blocking operator defect assigned to B9 cleanup: preserve exact selected-root observation, distinguish pre-install assumptions and remove the impossible `canic build` remediation when that command cannot populate the observed root. |
| Retained-decision source drift | Non-blocking operator diagnostic defect assigned to B9 beside `CANIC-028`: identify workspace-source drift, name the exact retained source identity/revision and omit unrelated identity/funding remediation after those checks pass. |
| `CANIC-030` retained funded Saltz asset | The open repository-estate cleanup now archives the exact calibration and terminal disposition record instead of deleting the only retained evidence for the live external canister and its approximately `2.590 Pcycles`; it authorizes no external effect. |
| `CANIC-006` state-preserving release transition | Already owned by scheduled 0.111's exact stop-the-world predecessor-to-successor transition. It remains blocked on the accepted 0.109 and 0.110 gates. |
| `CANIC-005` application retirement acknowledgement | Already owned by scheduled 0.110 B2. It remains blocked on 0.109 closeout and explicit 0.110 promotion. |
| `CANIC-015` saved plan consumption | Accepted as unscheduled deployment ergonomics; it is not an admission-authority or current planning-correctness blocker and receives no implementation authority here. |
| Long-running multi-Subnet local showcase | Useful future qualification work, but not part of the bounded 0.109 admission release. |
| Generic Fleet observatory | Already owned by scheduled 0.112 and remains blocked on its predecessors. |

## Result

Published 0.109.4 closes monotonic first-status recovery and the downstream
session crossed that gate. The open 0.109.5 `CANIC-035` hard cut now owns exact
pool policy, acceptance, diagnostics and retained Ready-row reinspection. Its
focused pure/host and governed PocketIC checks pass; maintainer validation,
versioning and publication remain unrun. B8 remains open for those gates, the
separately authorized Root upgrade/asset refresh/exact-
source resume and deployed-state/admission proof. B9 remains blocked until that
evidence is accepted; B10 and 0.110 remain blocked behind B9.
