# PocketIC redundancy review — 2026-09-18

The initial source/log review below identified two clear consolidation candidates
and substantial repeated fixture work. The maintainer subsequently authorized
working through the findings. The implementation disposition is recorded first;
the audit observations and timings below refer to the pre-cleanup .24 snapshot.

## Implementation disposition

- Removed the repeated invalid-state upgrade from the combined lifecycle case.
  Its remaining successful install and invalid-init/reinstall check is now named
  `invalid_reinstall_arguments_report_init_phase_error`. The dedicated
  `non_root_post_upgrade_failure_reports_phase_error` remains the single owner of
  invalid-state upgrade rejection. All six lifecycle cases pass.
- Removed `uncertain_mainnet_refill_reuses_the_exact_paid_request` and its governed
  registration. The unchanged four-asset survivor passes with exact request and
  replay counts. The ordinary single-asset successful-response case remains.
  No replacement shim, duplicate registration or aggregate-count guard was added.
- Retained the explicit warm-ups. Further reading of the accepted
  [B1 protocol](../../../working/0.106-fleet-estate-platform-qualification/b1-protocol.md)
  establishes one excluded warm-up per journey and network, including the local
  preflight. This resolves the audit's conditional suggestion: removing them is
  not a semantics-preserving consolidation of the maintained qualification.
  No protocol, paid-effect arithmetic or cohort behavior changed.
- Retained both mixed-topology wipes. The second starts after a completed
  reinstall, reseeds real Hub/Shard rows, selects the same desired build and proves
  another reset under a new operation while preserving cumulative conservation.
  Starting a small fixture from ordinary initial provisioning does not reproduce
  that state. A separate replacement needs its own two completed resets and setup;
  no net saving has been established. Shrinking topology between the two existing
  wipes would also lose the exact repeated-desired-state case. Do not remove this
  pass just because its measured cost is large.
- Retained the distinct funding journeys and public fixture boundaries. Existing
  immutable artifact caching already shares equal recipes. The current pooled
  Component Registry explicitly forbids starting the HTTP gateway on a pooled
  fixture, whereas the production-adapter journeys require a live gateway plus
  local plans/journals, Ledger state, lost-response markers and exact authority.
  Pooling only the canister state would not isolate those operations. A jointly
  restored host/runtime fixture needs its own design and qualification; it is not
  a safe mechanical replacement in this deduplication batch.

Focused PocketIC evidence:

| Selection | Result | Local log |
| --- | --- | --- |
| `make test-pocketic-case CASE=lifecycle_boundary` | All 6 cases pass; 142.23 s test execution including fixture builds, 277 s runner | `/tmp/canic-redundancy-lifecycle.log` |
| Exact internal `autonomous_refill_margin_survives_burn_and_replays_without_another_debit` case | Pass; 185.31 s including fixture builds, 351 s runner | `/tmp/canic-redundancy-refill.log` |

Additional focused checks pass:

- `governed_pocketic_inventory_preserves_baseline_order_and_journey_suffix`,
  selected exactly with `governed-pocketic-tests`; log:
  `/tmp/canic-redundancy-inventory.log`.
- Warning-denied Clippy for `canic-testing-internal --lib --tests` with
  `governed-pocketic-tests`, and `canic-tests --test lifecycle_boundary`; logs:
  `/tmp/canic-redundancy-internal-clippy.log` and
  `/tmp/canic-redundancy-lifecycle-clippy.log`.
- Rustfmt for the two changed test files and `git diff --check`.

Both runs rebuilt artifacts; these durations are not comparable to warmed
case timings in the audit below. No full release reduction is claimed. The
deduplication disposition is complete; broader fixture/build throughput work
remains separate in the open .25 speed batch. Packages remain .24, with no
commit, version bump, push or live deployment.

## Scope and execution evidence

Base is published `933a35b66403f6a94f99803252cc52c0aec32958` (.24), with the
existing .25 Root/Store query changes preserved. Inspected the workspace runner,
integration inventory and all four internal case registries. Detailed assertion
comparison concentrated on refill, funding, lifecycle, provisioning and reinstall;
runtime integration boundaries and host mint proofs were also compared. This is
not an assertion-by-assertion equivalence proof for every integration test.

The retained successful log is
`target/validation-runs/20260918T140257Z-60259.NaOVNO/0.log`. All 64 registered
internal cases have unique names and function registrations, and each appears
exactly once as a completed top-level case in that log. These are audit counts,
not a proposed hard-coded inventory guard. The ordinary lane excludes the
governed catalogue; its later exact harness calls it once. Host ignored proofs
and integration targets are separate selections. The instruction report remains
ignored, and external IcyDB composition is outside the ordinary release lane.

## Clear consolidation candidates

| Candidate | Existing coverage that survives consolidation | Disposition |
| --- | --- | --- |
| `lifecycle_boundary_traps_are_phase_correct`'s post-upgrade half and `non_root_post_upgrade_failure_reports_phase_error` | Both install the same authority fixture, upgrade it with the same Canic Wasm/arguments and retry policy, then assert the same `post_upgrade` phase failure | Keep one owner for this invalid-state upgrade. Preserve the combined test's successful initial install and invalid-init/reinstall assertion separately. |
| `uncertain_mainnet_refill_reuses_the_exact_paid_request` | Calls `assert_mainnet_refill(true, 1, 2)`. The four-asset `autonomous_refill_margin_survives_burn_and_replays_without_another_debit` calls the same helper with `(true, 4, 5)`, covers the same first uncertain response/retry, and checks exact amounts, balances, origins and effect-free replay | Remove the smaller uncertain-response case after the four-asset survivor passes. Keep the single-asset no-loss case for the ordinary successful response path. |

Source anchors:

- [Lifecycle cases](../../../../../crates/canic-tests/tests/lifecycle_boundary.rs):
  `invalid_reinstall_arguments_report_init_phase_error` and
  `non_root_post_upgrade_failure_reports_phase_error`.
- [Refill catalogue and helpers](../../../../../crates/canic-testing-internal/src/pic/fleet_registry/baseline.rs):
  `prepared_mainnet_root_automatically_refills_one_exact_pool_asset`,
  `autonomous_refill_margin_survives_burn_and_replays_without_another_debit`,
  `assert_mainnet_refill` and `assert_mainnet_refill_result`.

The smaller refill case took 4.43 seconds. Lifecycle individual timings were
not retained; the entire six-test target took 30.21 seconds, including distinct
participant trap and restoration checks. Do not count that entire target as a
saving. Even the 4.43 seconds is historical case cost, not a measured post-change
release reduction: shared setup/cache work may move to another case.

## Larger overlaps that need redesign

| Family | Observed cost | Why deletion alone loses coverage | Next safe direction |
| --- | --- | --- | --- |
| Single-asset funded estate, issued funding pause, four-Workload refill | 297.37 / 115.28 / 172.80 s | Same funding helper repeats fee rejection, lost transfer, creation receipt, conservation and replay. However, one provisions an initially unprovisioned estate, one resumes an already-issued creation, and one starts with four active Workloads and no Ready reserve. The single-asset paths also exercise generated desired state. | Factor common setup and consider exact isolated baselines before each distinct starting state. Keep one broad owner for shared transfer assertions and focused cases for the other triggers; preserve original intent and receipt checks in each recovery case. |
| Mixed-topology deliberate reinstall | 401.79 s inside the 665.23 s mixed journey; first wipe 194.43 s, second 154.34 s | The second wipe has a new operation identity, reseeded Hub/Shard rows and cumulative conservation. It proves a deliberate repeated reset actually wipes again while replay does not. | Investigate moving deliberate-repeat behavior to a smaller real managed fixture while retaining the full mixed-topology first wipe. The 154.34 s is an optimization envelope, not a promised saving; the replacement proof has a cost. |
| Generated retained-estate reinstall versus mixed-topology reinstall | 457.83 / 665.23 s total case time | Retained-estate coverage includes 19 Workloads plus five Ready assets, exact import completeness, source authority, successor review and conservation. Mixed topology covers selected-build reset, real Hub/Shard row wiping and interruption under changed workspace selection. | Share suitable artifact/setup mechanics, not assertions or accepted capacity. Keep both distinct contracts. |
| Composed direct-ingress versus published managed-App support | 50.00 / 120.16 s | Direct ingress checks framework/Canic parity, denied callers and application ownership. The public support case exercises the published fixture API, same-release restoration and standalone support. | A common exact artifact/baseline may help. Do not remove either API boundary merely because both activate a composed app. |

The 297.37-second estate case contains **246.32 seconds of initial artifact
preparation** and about 49.90 seconds under generation/review/journey work.
Deleting that case can move its cold compilation to the funding-pause case,
which used the same recipe in about 4.29 seconds. Case durations are not additive
estimates of removable work. The broader phase label also includes the funding
journey itself; it does not isolate pure generation cost.

The cohort preflights additionally execute width one as an explicit warm-up and
again in `[1, 8, 16, 32]`. Reset does this for empty and installed assets. Their
helpers repeat the same correctness assertions; no separate measured result is
emitted by those helpers. Treat this as a low-priority routing/consolidation
candidate: preserve any explicit qualification warm-up requirement in its audit
lane and the real width/capacity contract, rather than altering external-effect
accounting merely to shorten CI. The two whole cohort cases cost 3.84 and 18.46
seconds; only a portion is duplicated width-one work.

## Similar tests that should stay distinct

- Initial-child reserve failure propagates through the parent operation and
  Coordinator/host diagnostics. The later-child case allocates under an already
  active parent. Preserve both retained-claim recovery boundaries.
- Failed initial imports and four Failed reserve assets start at different
  inventory/provisioning states and use distinct repair branches. The latter
  explicitly proves no extra creation when the complete pool is already present.
- Prepared Root/Store bootstrap tests retain authorization and direct catalog
  reverification checks; application catalog tests recover lost chunk publication
  replies. Common bootstrap does not make these equivalent.
- Host mint interruption tests cover lost ICP transfer, notification and credit
  persistence. The combined monetary test adds a real native withdrawal, lost
  withdrawal reply and terminal conservation. The latter does not subsume the
  former's earlier interruption positions.
- Lifecycle rollback, timer reconstruction and corrupt stable-state rejection
  inspect different restore invariants. Direct ingress size checks and
  inter-canister adapter size checks exercise different execution paths.

## Proposed cleanup order

1. Consolidate the duplicate invalid-state upgrade and subsumed one-asset refill
   case. Update the registry and any direct selectors; run only the lifecycle
   target and surviving four-asset refill case.
2. Narrow or relocate duplicated width-one warm-ups after documenting which
   qualification lane owns warm-up, without changing capacity/accounting bounds.
3. Design smaller repeat-reset coverage and shared exact setup for funding
   scenarios. Qualify each changed real-canister boundary before removing its
   existing expensive owner. Measure survivor/setup time and the next ordinary
   maintainer-run release; do not promise the sum of deleted case durations.

This was the cleanup sequence proposed by the initial audit. The implementation
disposition above records the two removals and the reasons for retaining the
other cases. It does not claim the whole .25 speed batch is push-ready.
