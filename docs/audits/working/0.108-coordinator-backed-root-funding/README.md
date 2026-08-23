# Canic 0.108 Coordinator-Backed Root Funding Evidence

Date: 2026-08-22
State: M0 accepted 2026-08-21; M1 complete in published 0.108.0; B3-B9
closeout corrections and CANIC-019 applied to the open 0.108.1 working draft

## Authority And Scope

The historical first part of this record supports M0 and M1 of the accepted
[0.108 design](../../../design/0.108-coordinator-backed-root-funding/0.108-design.md).
M0 adds one unpublished test-only Canister and one serial PocketIC integration
target. M1 adds protected policy to the existing Fleet-input, plan, init,
root-authority and Registry contracts, but no runtime grant state machine,
timer, treasury ledger or public endpoint.

The closeout-correction section below covers B3-B9/M2-M8 in the open 0.108.1
draft. It does not rewrite M0/M1 history or claim that 0.108.1 is published.

The published checkpoint reconciliation starts from `main` at
`5523280c7c1b081d455c69fb551448c4cf9212f7`. The observed toolchain is
Rust/Cargo 1.97.1 with MSRV 1.91.0 and PocketIC 15.0.0. The exact test Wasm
build fingerprint after the final M1 payload-bound recomputation is
`f93c470cc9e016449c8dfb446703ee8401ef21fa8658ab17ad64b3a5514d1cc9`;
the 625,326-byte artifact has SHA-256
`692bc4facc00a9b886c08009864319ad2b59807887c44dcd4a0cc041d54479e9`.

## Selected Platform Primitives

The Coordinator probe uses
`ic_cdk::call::Call::bounded_wait(...).with_arg(...).with_cycles(...).await`.
The exact root reads `msg_cycles_available()` and calls
`msg_cycles_accept(exact_amount)` only after caller, operation, amount and
target bindings pass. Fresh acceptance and receipt persistence occur
synchronously without an `await`. Exact replay accepts zero and returns the
prior receipt, leaving the attached principal unaccepted for automatic return
to the Coordinator.

The Root also records `canister_cycle_balance()` immediately before and after
fresh acceptance. PocketIC observes an exact increase of the 1T attached
principal only after `msg_cycles_accept`, proving that the selected balance
API excludes incoming attached cycles before acceptance.

These are test-only proof choices for the workspace's pinned `ic-cdk 0.20.2`;
M0 does not add their surrounding state machine to production.

## Interruption And Authority Matrix

| Boundary | PocketIC action | Retained authority | Result |
| --- | --- | --- | --- |
| Intent | Stop and restart the Coordinator after a separate prepare message | Exact root, operation `[0x17; 32]`, 1T grant, call cost and reservation remain in the probe heap | Prepared intent remains byte-equivalent |
| Caller | A second Coordinator attaches the same request to the root | Root is initialized with the exact first Coordinator | Foreign caller is denied, accepts zero and creates no receipt |
| Call | Stop the root before the Coordinator dispatches | Coordinator retains the prepared exact intent | Call fails without a root receipt; restart admits the same request |
| Root transaction | Root accepts 1T, writes its receipt and deliberately traps before reply | The complete Root replicated message is atomic | Root retains neither the 1T balance increase nor receipt; Coordinator receives the attached principal back apart from call execution |
| Receipt | Root accepts and commits, then the Coordinator response callback traps | Root retains the exact receipt; Coordinator remains prepared | No new operation is minted after response loss |
| Replay | Coordinator dispatches the same root/method/arguments/amount | Root receipt binds operation, Coordinator, root and amount | Root accepts zero, returns the prior receipt and Coordinator commits once |

The journey uses no production `cfg(test)` branch. The complete fixture package
is test-only, unpublished, dependency-leaf guarded and absent from shipped role
configuration.

## Dated Headroom Observation

The exact proof grant is `1,000,000,000,000` cycles. The reported operating
observations below exclude the additional deliberate Root-trap call, whose
separate assertion proves its attached principal is refunded.

| Quantity | PocketIC 15.0.0 observation |
| --- | ---: |
| Final `canic_command` cost for the 16 KiB encoded-command bound | 42,118,809,000 |
| Grant plus final bounded call reservation | 1,042,118,809,000 |
| Coordinator execution beyond the accepted fresh grant | 12,341,408 |
| Coordinator execution during zero-accept replay | 12,461,694 |
| Root execution deducted during fresh acceptance | 5,217,306 |
| Root execution deducted during replay | 5,227,446 |

The replay Coordinator spent less than the attached 1T principal and the root
balance did not increase, proving automatic return of the unaccepted replay
principal. These are one deterministic PocketIC observation, not IC-mainnet
costs or universal production thresholds.

For the next protected-policy review, M0 retains the checked admission shape:

~~~text
exact grant
    + cost_call(exact method and encoded request)
    + 100,000,000 Coordinator execution allowance
~~~

and a separate `100,000,000` Root request/retry execution allowance. Each
allowance is the smallest 100M-rounded value above the corresponding maximum
attached-cycles observation. M1 recomputed `cost_call` for the existing public
`canic_command` method and a conservative 16 KiB encoded funding-command
bound. M2 must assert the final command DTO's maximum encoding fits that bound.
This does not replace the distinct automatic ICP-refill execution/recovery
floor recorded below.

## Root Admission Floors

The expanded probe executes the actual nested Root-to-Coordinator-to-same-Root
shape. The final bounded Root outbound `canic_command` request reserves
`42,118,809,000` cycles. Fresh Root request execution beyond the returned 1T
grant is `10,859,260` cycles; terminal retry execution is `5,659,729` cycles.
The smallest 100M-rounded value above the maximum call reservation plus
observed execution remains `42,200,000,000` cycles, which M1 freezes as the
minimum `request_threshold`.

The emergency probe follows fee, decimals and ICP/XDR-rate acquisition, then
deliberately loses the ledger-transfer response, recovers the exact duplicate
block, deliberately loses the CMC-notify response, replays the exact notify
and commits the terminal result. Its maximum exact call reservation is
`42,102,599,000` cycles and the complete failure/recovery journey consumes
`58,420,824` cycles. The corresponding 100M-rounded automatic-refill
execution/recovery floor is `42,200,000,000` cycles.

These values are dated fixture measurements, not universal IC-mainnet costs.
M1's final method/bound recomputation keeps both validation constants at
`42,200,000,000`; they may not be lowered without an equal or stronger
PocketIC result.

## M1 Protected Policy Hard Cut

Fleet-input schema 1 now requires `coordinator.root_funding` whenever roots
exist and requires `root_funding` for every root. Optional per-root
`icp_refill` and nested `automatic` policy are protected Fleet input only.
Unknown fields and any attempt to place the policy in `canic.toml` fail
closed. One shared model validator owns positive fields, reserve/request/
emergency floors, target ordering, per-root and Fleet grant feasibility, ICP
caps, reserved system Principals and explicit IC-override acknowledgement.

The host propagates exact typed policy through the canonical plan and digest,
Coordinator init/install journal, root install journal and activation
authority. Coordinator genesis and root activation independently validate
their protected copies before persistence. Registry publication, canonical
Registry hashing and mirror comparison include the complete root authority;
policy drift therefore blocks instead of being repaired from a request.

Canonical policy hashes use domain-separated, explicit field encodings. The
accepted fixtures are:

- Coordinator policy:
  `26a8f270734f672e87b68c6e6ca8b98df1001d7dd7cea985c979a6bdf4963618`;
- full root funding policy:
  `83eabbea6076f289519e734faf021f134ce7e4e028b16ddcff5af64f1dc6f40c`;
  and
- complete Registry manifest with the root policy:
  `88d950e6a112c8bea333bf69b8e40afb675024895654723a8def99945385fa95`.

Changing any decision-bearing Coordinator/root policy field changes its
policy hash and the complete plan/Registry identity. The generic
`CanisterConfig.icp_refill` model, renderer and application-config authority
are deleted without an alias. The existing manual refill workflow now reads
only validated root activation authority.

The canonical Coordinator Candid was regenerated deterministically. Its only
M1 expansion is the intended optional Coordinator init policy plus root
binding/Registry policy types. Runtime-funding variants occur only as nested
authority data; no new public method or command/status selector exists yet.

## 0.108.0 Checkpoint Boundary

The release checkpoint contains accepted M0 test evidence, the complete M1
protected-policy hard cut and the urgent fresh-Fleet provisioning corrections.
It does not contain M2 grant DTOs, grant-decision policy, treasury windows,
intents, receipts, a dedicated funding stable-memory allocation or Coordinator
grant operations. Those prematurely staged and unwired sources were removed
before the checkpoint was declared source-ready.

The protected Coordinator and per-root policies remain because they are the
complete M1 contract. No production grant, request, timer or funding endpoint
is reachable in 0.108.0.

## Bounded Receipt Decision

The design now rejects a generalized receipt collection. Each root retains
one current operation and one last exact terminal result. A checked monotonic
sequence is part of the operation identity: exact current/last replay returns
the retained result, stale or skipped sequences reject, and exact N+1 arrival
acknowledges N before replacement. At most one Coordinator call attempt for a
root may be outstanding. Under those invariants there is no delayed N call to
service after N+1 is admitted and no need for a replay horizon, purge timer or
separate cross-Canister acknowledgement.

The retained pure counterexample test passes fresh sequence 1, exact current
retry, conflicting operation-ID reuse, exact terminal replay, skipped sequence
rejection, exact successor replacement and stale predecessor rejection.

## Offline Break-Glass Authority

M0 fixes the offline authority chain to the canonical-network Fleet catalog
row plus the digest-bound Fleet install plan and canonical per-placement Root
install journals. The existing catalog test proves local resolution preserves
the exact Coordinator and refuses to reinterpret the Coordinator-anchored
Fleet as the removed single-root topology. The new journal test reloads the
canonical host file and derives the exact Coordinator and Root Principals from
its protected authority without invoking ICP, Coordinator or Root transport.
M6 must compose those validated sources; live calls may corroborate but can
never be required for target selection.

## Focused Validation

Newly executed on 2026-08-21 for M0:

- `cargo check --locked -p root_funding_probe --target wasm32-unknown-unknown`: pass.
- `cargo clippy --locked -p root_funding_probe --target wasm32-unknown-unknown -- -D warnings`: pass.
- `cargo clippy --locked -p canic-tests --test pic_root_funding_recovery -- -D warnings`: pass.
- Governed pinned-server run of `cargo test --locked -p canic-tests --test pic_root_funding_recovery -- --nocapture --test-threads=1`: pass, 4 tests covering both atomicity cases, pre-acceptance balance semantics, current/last sequence bounds, nested Root request/retry costs and the complete emergency failure/recovery floor.
- `cargo test --locked -p canic-host durable_root_journal_resolves_break_glass_authority_without_target_calls --lib`: pass, 1 test.
- `cargo test --locked -p canic-host coordinator_catalog_rejects_the_removed_single_root_topology_resolver --lib`: pass, 1 test.
- `cargo test --locked -p canic-host qualification_harness_packages_are_test_only_leaves --lib`: pass, 1 test.
- `cargo clippy --locked -p canic-host --lib -- -D warnings`: pass.
- `bash scripts/ci/check-workspace-test-inventory.sh`: pass, 39 targets with 9 serial PocketIC targets.
- `make current-document-semantics-gate`: pass.
- `cargo test --locked -p canic --test changelog_governance`: pass, 1 test.
- `git diff --check`: pass.

Newly executed on 2026-08-21 for the final M1 bound and protected policy:

- Governed pinned-server run of `cargo test --locked -p canic-tests --test pic_root_funding_recovery -- --nocapture --test-threads=1`: pass, 4 tests; exact 16 KiB `canic_command` reservation is `42,118,809,000` cycles and both frozen floors remain `42,200,000,000` cycles.
- `cargo test --locked -p canic-host fleet_install_input --lib`: pass, 23 tests covering exact policy propagation and strict invalid-policy rejection.
- Focused core policy/hash, activation, Registry and plan-digest tests: pass.
- Focused Coordinator genesis and Fleet-budget admission tests: pass.
- `cargo test --locked -p canic-cli --no-run`: pass.
- Canonical Coordinator Candid regeneration: pass; the checked-in surface contains only the intended protected data expansion and no funding endpoint.

Final 0.108.0 checkpoint reconciliation on 2026-08-21:

- Locked core/control-plane compilation and the Rust 1.91.0 core check pass.
- Warning-denied Clippy passes for core and both Root packages that had exposed
  the generated dispatch-frame regression. The large Root dispatch future is
  now immediately heap-boxed; its local expectation applies only to that boxed
  async block.
- Focused funding policy, canonical hash, memory ownership, role-contract,
  Coordinator, Fleet-input/plan, provisioning-identity, finalized-Candid,
  generated-protocol, host state-manifest and CLI planning tests pass.
- Formatting, changelog governance and whitespace checks pass. The complete
  workspace/release matrix and governed PocketIC journeys were not rerun during
  this agent-owned checkpoint repair; prior M0 PocketIC evidence is retained,
  and the downstream fresh Toko installation remains the explicit end-to-end
  confirmation after publication.

The first 2026-08-21 pinned server start was denied a sandbox loopback bind and
reached no product behavior. The approved local-only server and targeted test
above are the behavioral result.

## B3-B9 Closeout Correction

The first human-owned closeout audit rejected the open 0.108.1 draft. The
correction starts from published `v0.108.0` commit
`187dacd4f3c07b3077513bc9d9148fe7261fa4ff`. The current local candidate is
`d4e18003248b085d05dc431153da3efa998dc119` on `main`, followed by the dirty
working-tree corrections recorded below. The revision is two commits ahead of
`origin/main`; neither those commits nor the working-tree correction are a
published 0.108.1 release. The maintainer must establish the final immutable
candidate before re-audit. This record names the exact reproducible base and
focused tests without treating a local commit or passing subset as release
truth.

The audit corrections are:

- both value-bearing funding legs now use
  `ic_cdk::call::Call::bounded_wait`: Root to exact installed Coordinator for
  `RequestRootFunding`, then Coordinator to the authenticated current Root for
  `AcceptFunding` with the exact attached amount. Snapshot acknowledgement
  remains a separate non-value-bearing call;
- the Root-owned ICP journal retains at most 4,096 lifetime operation
  identities. Terminal records are not evicted, exact replacement/replay at
  capacity remains valid, and a new identity fails with `CAPACITY_LIMIT`;
- PocketIC 15's built-in production ICP Ledger and CMC now own value-transfer
  semantics. The local refill stub remains only for deterministic adapter and
  fault-classification tests; and
- this record, the design status, active handoff, changelog and runbook now
  distinguish published 0.108.0, the rejected draft, the corrected open draft
  and the still-required human re-audit.

### M0-M8 / B1-B9 Traceability

| Milestone / batch | Requirement | Implementation/evidence | Result |
| --- | --- | --- | --- |
| M0 / B1 | Attached-cycle atomicity, refund, response-loss replay and measured bounded-call floor | Test-only `root_funding_probe`; accepted PocketIC interruption matrix and 42.2B-cycle floors above | Accepted; assumptions retained because production now uses the same bounded-call and accept-zero primitives |
| M1 / B2 | Protected policy, canonical hashing/propagation and generic refill hard cut | Fleet-input/model/hash/plan/init/Registry validation and published 0.108.0 evidence above | Complete in 0.108.0 |
| M2 / B3 | Sole Coordinator grant authority, reserve/windows, durable current/last result | `ops/fleet_coordinator/root_funding.rs`, Coordinator stable ID 62, workflow authority/replay tests, one- and two-Root PocketIC journeys | Pass in corrected draft |
| M3 / B4 | Root-owned request journal and exact accept-once command | `ops/root_funding`, Root stable ID 63, restart/zero-accept tests and accepted M0 callback-loss proof | Pass in corrected draft |
| M4 / B5 | One Root timer, recovery-first ordering, finite non-renewing caps and unchanged descendant owner | `workflow/runtime/cycles`, timer/lifecycle guards, 91-day cap PocketIC journey and non-Root parent-funding unit proof | Pass in corrected draft |
| M5 / B6 | One manual/automatic Ledger/CMC replay owner, floor/reserve/caps and terminal fallback | `workflow/ic/icp_refill`, stable ID 39, 4,096-record bound, built-in Ledger/CMC replay, fallback and no-spend journeys | Pass in corrected draft |
| M6 / B7 | Exact installed-authority recovery, protected status/CLI/Medic and lifecycle/snapshot fences | Host resolver, role status/Candid, CLI/Medic and snapshot/lifecycle focused suites | Pass in corrected draft; final rerun listed below |
| M7 / B8 | Representative generated consumers, measured local qualification, sediment/docs and closeout truth | Generated Root/Coordinator builds, real PocketIC matrix, active-document reconciliation and targeted hygiene gates | Prior evidence passes; post-validation correction still requires the final maintainer gate and human re-audit |
| M8 / B9 | Explicit same-release funding-policy generation rotation with retained predecessor usage and application state | Exact plan/hash validation, Coordinator/Root durable fences and receipts, bounded stable checkpoints, CLI/Candid surfaces, interruption/replay unit proof and governed exhausted-to-successor PocketIC journey | Pass in corrected open draft; final maintainer gate and human re-audit remain required |

### Corrected PocketIC Matrix

The governed runner starts only the pinned local PocketIC server. No external
Ledger, CMC, canister, network or funding effect occurs.

| Design journey | Direct evidence | Result |
| --- | --- | --- |
| One active Root receives one exact grant | `real_coordinator_funds_one_active_root_exactly_once` | Pass |
| Two Roots use independent limits and one Fleet budget | `two_roots_use_independent_limits_and_one_coordinator_budget` | Pass |
| Reserve blocks a valid grant | `terminal_coordinator_reserve_denial_runs_one_real_icp_fallback` retains the exact no-grant reason before fallback | Pass |
| Response loss converges without a second transfer | Accepted M0 post-commit loss/replay plus production bounded-call wiring; built-in Ledger/CMC exact replay separately proves value idempotency | Pass; no mock-only value claim |
| One Root cannot receive a second automatic grant | `automatic_grant_cap_never_renews_after_the_ninety_day_window` plus pure cooldown/window policy cases | Pass |
| Window rollover cannot renew the lifetime cap | Same 91-day journey retains count/result/current state | Pass |
| Terminal no-grant runs one real ICP conversion | `terminal_coordinator_reserve_denial_runs_one_real_icp_fallback` uses PocketIC's production Ledger and CMC | Pass |
| Uncertain grant suppresses ICP fallback | `uncertain_grant_suppresses_icp_and_direct_topup_remains_available` | Pass |
| Insufficient ICP and rate denial spend nothing | `insufficient_real_icp_spends_nothing_and_creates_no_refill`; `real_rate_gate_denial_spends_no_icp_and_creates_no_refill` | Pass |
| Refilled Root preserves descendant funding ownership | The fallback journey holds one real registered Component stopped until refill completion, then sends its exact structural capability request; the Root deposits 5T once and exact replay deposits zero more | Pass |
| Stopped automatic path permits direct recovery | `uncertain_grant_suppresses_icp_and_direct_topup_remains_available` stops the Coordinator, retains the Root operation and applies an exact direct top-up without mutating the journal | Pass |

The separate
`production_ledger_and_cmc_exact_replay_never_duplicates_value` journey sends
one exact transfer to the CMC top-up subaccount, observes the production
Ledger's duplicate binding to the same block, notifies the CMC twice and proves
the second notification adds zero cycles. This is the positive value-transfer
evidence behind the replay claim; unit stubs are not substituted for it.

### Correction Validation

Executed locally on 2026-08-22 against the working draft:

- `cargo test --locked -p canic-control-plane root_funding --lib`: pass, 12
  focused Coordinator/Root authority, replay and state tests.
- `cargo test --locked -p canic-core icp_refill --lib`: pass, 81 focused
  policy, journal, replay and capacity tests.
- Governed targeted PocketIC runs through
  `bash scripts/ci/run-with-test-scratch.sh bash scripts/ci/run-workspace-tests.sh targeted-pocketic <test-name>`:
  `real_coordinator_funds_one_active_root_exactly_once`,
  `two_roots_use_independent_limits_and_one_coordinator_budget`,
  `automatic_grant_cap_never_renews_after_the_ninety_day_window`,
  `terminal_coordinator_reserve_denial_runs_one_real_icp_fallback`,
  `uncertain_grant_suppresses_icp_and_direct_topup_remains_available`,
  `production_ledger_and_cmc_exact_replay_never_duplicates_value`,
  `real_rate_gate_denial_spends_no_icp_and_creates_no_refill`, and
  `insufficient_real_icp_spends_nothing_and_creates_no_refill`: pass.
- `cargo test --locked -p canic --test protocol_surface`: pass, 40 public
  protocol, checked-in Coordinator Candid and role-ingress checks.
- `cargo test --locked -p canic-cli funding`: pass, 4 focused CLI tests;
  `cargo test --locked -p canic-cli --test subcommand_order`: pass, 1 recursive
  help-order and example-count check.
- `cargo test --locked -p canic-host funding --lib`: pass, 12 protected-input,
  plan/hash and generated role-state checks;
  `cargo test --locked -p canic-host fiduciary --lib`: pass, 1 exact
  placement-acknowledgement check; and
  `cargo test --locked -p canic-host recommended_coordinator_selector_is_a_hard_decode_cut --lib`:
  pass, 1 hard-cut check.
- `cargo test --locked -p canic-core role_contract --lib`: pass, 21 role and
  allocation-owner checks; `cargo test --locked -p canic-core memory::policy --lib`:
  pass, 6 memory-map policy checks.
- `cargo test --locked -p canic-core --test lifecycle_boundary_guard`: pass, 7;
  `cargo test --locked -p canic-core --test timer_inventory_guard`: pass, 16;
  and `cargo test --locked -p canic-control-plane state_contract --lib`: pass,
  4.
- `cargo clippy --locked -p canic-core -p canic-control-plane -p canic-testing-internal --lib --tests -- -D warnings`:
  pass; `cargo clippy --locked -p canic -p canic-cli -p canic-host --lib --tests -- -D warnings`:
  pass.
- `cargo fmt --all -- --check`, `git diff --check`,
  `make current-document-semantics-gate`,
  `cargo test --locked -p canic --test changelog_governance`,
  `bash -n scripts/ci/run-workspace-tests.sh`, and
  `bash scripts/ci/check-workspace-test-inventory.sh`: pass; the inventory
  remains 39 targets, 30 parallel and 9 serial PocketIC.

The complete maintainer-owned validation/release matrix was not run in this
focused correction pass and is not claimed here. The post-validation section
below supersedes this pass's initial readiness handoff; neither section claims
versioning, publication or remote qualification.

### Post-Validation Candidate Correction

The maintainer's first complete local validation attempt against the local
0.108.1 candidate found eight deterministic failures rather than a runtime or
remote effect:

- the CLI plan fixture still expected an obsolete 2,100 Tcycle maximum debit
  and 1,500 Tcycle Coordinator allocation;
- pre-activation Root join did not yet enforce one Root's exact profile,
  Fleet-window target capacity and Fleet lifetime-cycle capacity;
- five canonical plan, Registry, service-binding and draining fixtures retained
  hashes from the predecessor policy shape; and
- the deployment/restore proof still expected a funding-disabled Root to omit
  the canonical top-up timer declaration rather than declare it unregistered.

Those defects are corrected in local commit
`d4e18003248b085d05dc431153da3efa998dc119`. The follow-up maintainer validation
reached warning-denied Clippy and found a duplicated Single/preview profile
match arm; the focused rerun exposed the same masked lint in host validation.
The dirty working-tree correction merges both arms, appends the new profile
after the two existing enum variants so their serialized ordinals stay
unchanged, updates the resulting multi-profile draining hash and strengthens
the preview test to assert the complete 180 Tcycle staging envelope.

Focused post-validation reruns on 2026-08-22:

- `cargo clippy --locked -p canic-core -p canic-host --lib --tests -- -D warnings`:
  pass.
- `cargo test --locked -p canic-core fleet_funding --lib`: pass, 16 tests.
- Exact core plan, Registry manifest, initial-services, service-binding and
  Root-draining hash regressions: pass, one test each.
- Exact pre-activation Root-admission regression: pass.
- Exact host physical-topology/preview-profile regression: pass and asserts
  140T Coordinator creation, 80T reserve, 30T Root creation, 10T Store
  creation, 10T/30T request/target, 30-/90-day periods, 30T window, two/60T
  non-renewing cap and absent ICP policy.
- Exact CLI deployment-plan regression: pass.
- `cargo test --locked -p canic --test protocol_surface`: pass, 40 tests,
  including the checked-in Coordinator Candid profile variant.

The complete maintainer validation attempt began before the final dirty
corrections and is not evidence for this exact working tree. It must be rerun
by the maintainer after the final candidate is made immutable. No remote
qualification or value-transfer effect was performed by these corrections.

### CANIC-019 / B9 Policy-Generation Rotation Evidence

The accepted amendment adds an explicit same-release rotation rather than a
renewable timer or mutable reset. Planning is read-only and binds the exact
installed Fleet, predecessor Registry/generation/policy hashes and usage,
placement evidence, proposed successor policies, maximum new exposure, zero
operator debit and Coordinator-treasury source. Apply is controller-protected.
One Coordinator durable operation fences automatic work, prepares all affected
Roots, publishes one successor Registry generation and activates each Root
through exact idempotent receipts. Historical usage, operation sequences,
application Registry state and descendant-funding ownership remain monotonic.

Completed checkpoints are non-evicting and bounded to 4,096 total Root entries
across all rotations. Protected status reports checkpoint count, Root-entry
count and remaining capacity. CLI planning refuses a successor whose affected
Roots will not fit. Every checkpoint retains the complete plan so exact begin,
stage, apply and status replay remains available after later rotations and
payload drift rejects. Coordinator stable-capacity evidence covers a maximum
active rotation, one maximum-width completed checkpoint and the larger
25,315,095-byte worst case of 4,096 one-Root checkpoints inside the corrected
32 MiB cell. Root state retains only one current and one terminal rotation
record.

Focused validation executed locally on 2026-08-22 against `main` HEAD
`d4e18003248b085d05dc431153da3efa998dc119` plus the recorded dirty draft:

- `cargo test --locked -p canic-core rotation_plan_`: pass, 2 tests.
- `cargo test --locked -p canic-control-plane policy_rotation`: pass, 4 Root
  and Coordinator prepare/activate, replay and retained-usage tests.
- `cargo test --locked -p canic-control-plane coordinator_policy_rotation_converges_once_and_preserves_application_registry_state`:
  pass. It covers stale begin, exact begin/stage/apply replay, kill-switch and
  concurrent-operation fencing, every durable phase, Registry recovery from
  retained checkpoints, corrupt-checkpoint rejection, exact old-operation
  replay and payload-drift denial after two successive rotations, and writable
  post-rotation Root draining lifecycle state.
- `cargo test --locked -p canic-control-plane maximum_root_funding_ledger_fits_its_stable_cell_bound`:
  pass. The strengthened test first measured the prior 16 MiB bound as too
  small for the valid 25,315,095-byte maximally fragmented history; after the
  bound correction it proves the maximum live grant ledger, maximum active
  rotation, one 4,096-Root checkpoint and 4,096 one-Root checkpoints all fit
  32 MiB. `cargo test --locked -p canic-control-plane maximum_format_root_journal_fits_its_stable_bound`:
  pass.
- `cargo test --locked -p canic-cli funding_`: pass, 5 CLI plan/apply/status
  parsing and digest checks.
- `cargo test --locked -p canic --test protocol_surface coordinator_protected_funding_candid_types_are_explicit`,
  `cargo test --locked -p canic --test protocol_surface root_and_coordinator_funding_ingress_are_declared` and
  `cargo test --locked -p canic --test protocol_surface coordinator_command_surface_is_exact`:
  pass. The checked-in Coordinator Candid exposes the exact status and command
  additions; the command surface contains 14 maintained variants.
- `cargo clippy --locked -p canic-core -p canic-control-plane -p canic-host -p canic-cli -p canic -p canic-testing-internal --all-targets -- -D warnings`:
  pass after the retained-plan, mixed-state recovery and fragmented-capacity
  corrections for every package changed by B9.
- `CARGO_INCREMENTAL=0 bash scripts/ci/run-with-test-scratch.sh bash scripts/ci/run-workspace-tests.sh targeted-pocketic pic::fleet_registry::baseline::tests::explicit_policy_rotation_reopens_exhausted_automatic_funding_once`:
  pass, 1 test in 141.27 test seconds and 200 seconds for the governed suite.
  The governed local PocketIC journey exhausts
  generation one, advances 91 days without renewal, rotates to exact generation
  two, preserves registered application state, replays terminal begin/stage/
  apply without a second checkpoint, rejects drifted terminal begin/stage
  payloads, drains through the two existing descendant limits and observes
  only monotonic funding operation sequence two.
- Representative generated Root and Coordinator debug artifacts rebuild from
  the corrected source. The Coordinator refresh leaves the checked-in
  canonical DID exact, and `cargo test --locked -p canic --test
  protocol_surface fleet_coordinator` passes all four matching surface checks.

The first real-canister run exposed two implementation defects that unit
fixtures had not represented: a terminal fresh-Fleet provisioning receipt was
mistaken for active provisioning, and exact terminal Stage replay was rejected
while Begin/Apply replay succeeded. Closeout review then found four additional
local defects: Root activation consulted the split authority/mirror view before
its prepared recovery record; old completed commands were replayable only from
the most recent receipt and did not bind full payload; delayed exact Root
prepare replay could disarm the successor timer; and a one-checkpoint capacity
fixture did not represent maximally fragmented retained history. The final
implementation corrects those boundaries, preflights retained fixed-window
spend, stores each exact plan and raises the measured cell bound to 32 MiB. The
passing governed journey exercises the real-canister corrections. It used only
the pinned local PocketIC server; no external Ledger, CMC, canister, network or
funding effect occurred.

The 2026-08-23 maintainer validation rerun stopped at `check-invariants` because
`model/fleet_funding_policy` imported rotation DTOs in both production and test
code. The correction retains the economic validator in the model behind one
DTO-free named input; `ops/fleet_funding_policy` owns the sole boundary-plan
conversion and the original adversarial test follows that adapter. `make
check-invariants` passes all ten requested targets, both core rotation-plan
tests and all four Coordinator rotation tests pass, all five CLI funding tests
pass, and warning-denied Clippy passes for core, host and CLI. The complete
maintainer validation gate has not completed for the corrected candidate and
must be rerun; this focused correction does not claim it.

## M0 Disposition

The maintainer accepted complete M0 on 2026-08-21. It freezes the
selected CDK primitives, both separate transaction boundaries, nested request
and emergency execution-floor methods, fixed reservation-time windows,
monotonic current/last-result retention, the single Draining funding fence and
offline break-glass authority as inputs to production design.

M1 is complete: strict protected policy, validation, canonical hashing,
plan/init/root/Registry propagation and generic refill removal are present,
with no grant state machine or new public endpoint. The source is ready for the
maintainer-owned 0.108.0 release flow; M2 begins only after that checkpoint and
has no source in it. Neither M0 nor M1 authorizes a 0.106 B2 effect or remote
mutation.
