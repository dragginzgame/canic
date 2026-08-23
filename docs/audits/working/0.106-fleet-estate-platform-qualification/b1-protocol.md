# 0.106 B1 Cycles Ledger Provenance And Protocol

Date: 2026-08-20
State: Q2 provenance and Q3/Q4 protocol accepted for B1 on 2026-08-20; B2 observations remain open

## Authority Boundary

This record separates immutable published contract/source facts from later
empirical observations. It authorizes no Cycles Ledger call, Canister creation,
cycle transfer or remote experiment.

The official IC repository at commit
[`771df0e86ac79a9f5e5f996e516aefb45fa05594`](https://github.com/dfinity/ic/commit/771df0e86ac79a9f5e5f996e516aefb45fa05594)
maps the mainnet Cycles Ledger to `cycles-ledger-v1.0.6` and Wasm SHA-256
`ed99402535bb4f58e4ab469acc40c903f2fdeea409be16623d5c6a9131cbf120`.
The frozen mapping file SHA-256 is
`d23cc1441b85eb8d4be0154754cccda40037d18e3af08b826ac0b1a662c77673`.

The lightweight
[`cycles-ledger-v1.0.6`](https://github.com/dfinity/cycles-ledger/tree/29d98de5131918649a4c1cdd47fc176dea8770ef)
tag resolves to release commit
`29d98de5131918649a4c1cdd47fc176dea8770ef`, dated 2025-09-19. The exact
sources used here are:

| Source | Frozen identity |
| --- | --- |
| Published interface specification | [`INTERFACE_SPECIFICATION.md`](https://github.com/dfinity/cycles-ledger/blob/29d98de5131918649a4c1cdd47fc176dea8770ef/INTERFACE_SPECIFICATION.md), SHA-256 `553c35fa4f2e79650a3779df13a3035e9bca24f0222af23b4d5f6dcbb969a82d` |
| Published Candid | [`cycles-ledger.did`](https://github.com/dfinity/cycles-ledger/blob/29d98de5131918649a4c1cdd47fc176dea8770ef/cycles-ledger/cycles-ledger.did), SHA-256 `179462f9038c632c71fb9527385d25b130b3c4e91b8f8213691506bcc438b43d` |
| Fee and time-window constants | [`config.rs`](https://github.com/dfinity/cycles-ledger/blob/29d98de5131918649a4c1cdd47fc176dea8770ef/cycles-ledger/src/config.rs) |
| Transaction hash, duplicate lookup and creation execution | [`storage.rs`](https://github.com/dfinity/cycles-ledger/blob/29d98de5131918649a4c1cdd47fc176dea8770ef/cycles-ledger/src/storage.rs) |
| Canic adapter | `crates/canic-core/src/infra/ic/cycles_ledger.rs`, SHA-256 `9d91c534fe80c35dde1d7a0bec847c33ec28dc197907f46136fa590bce26e102` at `v0.105.0` |
| Canic durable retry workflow | `crates/canic-control-plane/src/workflow/canister_pool/refill.rs`, SHA-256 `373c43433ff852240c0ae2f90a3291137ad96ead4d0ff30db7aa67301a08f399` at `v0.105.0` |

Canic resolves `icrc-ledger-types 0.2.0`, but that package does not own the
Cycles Ledger `create_canister` extension used here. Canic deliberately mirrors
the exact published Candid request/result in its IC adapter; Q2 authority is
therefore the deployed release/interface above, not the incidental ICRC type
dependency.

## Q2 Contract-Versus-Observation Matrix

`Pending B2` means no empirical claim has been made. A future observation must
name its network, Cycles Ledger module identity, time and exact request receipt.

| Fact | Contract or versioned implementation says | Classification | B2 observation |
| --- | --- | --- | --- |
| Method and account | `create_canister` deducts from the caller's default Cycles Ledger account unless `from_subaccount` is supplied. Canic supplies `None`. | Published interface plus exact Canic request | Pending B2 |
| Request shape | Request fields are `from_subaccount`, nanosecond `created_at_time`, `amount`, and optional CMC settings/subnet selection. Canic supplies one Root controller and exact `Subnet` selection. | Published Candid | Pending B2 |
| Ledger debit | The caller is debited `amount + 100,000,000` cycles. The Ledger passes `amount` cycles to the CMC creation call. | Published interface and v1.0.6 source | Pending B2 |
| Reset setup top-up | The v1.0.6 `withdraw` endpoint sends the requested `amount` to exact canister `to` and deducts a separate 100,000,000-cycle fee from the caller's account. | Published interface | Pending B2 |
| Top-up retry identity | `withdraw` accepts `amount`, `from_subaccount`, `to` and nanosecond `created_at_time`; repeating the same parameters withdraws at most once. The harness must retain and reuse all four fields after uncertainty. | Published interface | Pending B2 |
| Creation cost | The CMC/target Subnet charges Canister creation from the attached `amount`; the amount retained by the new Canister is therefore not the Ledger debit and is not frozen here as one network-independent constant. | Published interface plus platform cost contract; exact result remains observation-dependent | Pending B2 |
| Duplicate identity | v1.0.6 hashes one Burn transaction containing source account, optional spender, `amount`, `created_at_time` and a fixed create-Canister memo. `creation_args`, controller settings and subnet selection are not in that hash. | Versioned deployed implementation; narrower than a literal all-request-fields reading | Pending B2 |
| Canic retry identity | Canic retains and compares operation ID, Ledger Principal, placement Subnet, Root, amount and `created_at_time`, and reuses the complete request. A conflicting request cannot replace the occupied singleton lane. | Frozen Canic implementation | Local focused tests pass; platform observation pending |
| Time precision | `created_at_time` is nanoseconds. The transaction window is 24 hours and permitted drift is 60 seconds. | v1.0.6 source constants and nanosecond arithmetic | Pending B2 |
| Exact time edges | `TooOld` occurs only when `created_at_time + 24h + 60s < ledger_time`; equality remains admitted. A timestamp greater than `ledger_time + 60s` returns `CreatedInFuture`. | Versioned deployed implementation | Pending B2 |
| Duplicate before terminal creation | The burn block is emitted before the CMC call. A concurrent exact duplicate can therefore return `Duplicate { duplicate_of, canister_id: None }` while creation is still unresolved. | Versioned deployed implementation | Pending B2 |
| Successful duplicate | After CMC success, v1.0.6 updates the retained transaction-hash entry with the created Principal. An exact duplicate then returns the original block and `canister_id: Some(original)`. | Published Candid and versioned deployed implementation | Pending B2 |
| Response loss | Within the retained deduplication window, an exact retry can recover a successful result through `Duplicate` with `Some(canister_id)`. It must remain unresolved when the duplicate has no Principal. | Versioned deployed implementation; transport loss itself is an observation condition | Pending B2 |
| `TooOld` before any uncertain call | A locally known-unapplied Canic intent may roll to a new operation only after the old timestamp expires and exact-effect authority proves no call became uncertain. | Frozen Canic implementation | Local focused test passes; platform observation pending |
| `TooOld` after uncertainty | Canic converts an uncertain request that reaches `TooOld` into terminal `UnresolvedAfterLedgerWindow`; it never invents a new paid request. | Frozen Canic implementation | Local focused test passes; platform observation pending |
| Insufficient funds and future timestamp | Validation precedes the burn block; these outcomes do not establish a paid creation effect. | Versioned deployed implementation | Pending B2 |
| Failed creation and refund | After the burn, CMC failure may produce fee/refund block identities. Refund processing can consume additional Ledger fees, so the run must reconcile debit, creation, refund and approval-refund blocks separately. | Published error shape and versioned deployed implementation | Pending B2 |
| Temporarily unavailable or transport failure | The interface exposes temporary unavailability, while an inter-Canister transport failure may occur outside the returned variant. Neither alone proves whether the paid effect happened. | Published Candid plus transport boundary | Pending B2 |
| Exact Subnet | `SubnetSelection::Subnet` expresses one requested physical Subnet and v1.0.6 passes the CMC arguments through. Successful placement on that exact Subnet remains an observation to confirm independently. | Published interface and versioned deployed implementation | Pending B2 |
| Controller set | Canic explicitly supplies the owning Root as the sole controller and v1.0.6 passes normalized settings to the CMC. The resulting controller set remains an independently queried observation. | Published request contract and versioned deployed implementation | Pending B2 |

## Required B2 Accounting

Every creation lane must retain five separate quantities:

1. the requested CMC creation `amount`;
2. the 100,000,000-cycle Cycles Ledger fee for the initial burn;
3. any later refund-block or approval-refund-block fees;
4. cycles observed in the created Canister after creation; and
5. Root/harness execution cost and unresolved reserved exposure.

No report may infer one quantity by subtracting two unrelated balance samples.
Each paid request retains its exact operation ID, `created_at_time`, Ledger
block evidence and returned Principal or explicit unresolved disposition.

Every reset setup row separately retains the pre-top-up balance, exact
`withdraw` request, 100,000,000-cycle fee, returned block, post-top-up balance
and any uncertain disposition. A setup top-up is never hidden inside measured
reset burn. It uses one exact retry identity and no replacement withdrawal.

## Safety Consequences For The Harness

- A lane retry must be byte-equivalent across every Canic-owned request field,
  even though the deployed Ledger deduplication hash omits CMC settings and
  subnet selection.
- `Duplicate { canister_id: None }` is unresolved, not success and not proof of
  failure.
- `TooOld` after any uncertain result is terminal unresolved evidence. It never
  authorizes a new paid request.
- A successful result is incomplete until the requested Subnet, sole Root
  controller and balance accounting are independently observed.
- A transport error is not classified as a definite rejection.
- Every empirical row records the current Cycles Ledger module identity. A
  changed deployed tag/Wasm hash invalidates the protocol comparison until B1
  is reconciled.

## Local Q2 Boundary Evidence

The focused PocketIC Root fixture now exercises both terminal paths against
one exact test-only Cycles Ledger boundary:

- an ordinary exact request returns one Principal in one request and reaches
  one truthful `Ready` asset; and
- an uncertainty fixture first commits `Duplicate { canister_id: None }`, then
  accepts only the byte-equivalent retry and returns the same Principal as
  `Duplicate { canister_id: Some(...) }` on request two.

Both journeys verify the created row, exact Root controller, exact requested
Subnet and terminal `Ready` state. Focused host tests independently reject a
wrong Root, wrong Subnet, zero amount, absent timestamp and absent creation
arguments. These are deterministic Canic/harness proofs, not observations of
the deployed Cycles Ledger and do not populate any B2 cell.

The bounded creation-lane preflight also passes exact cohorts 1, 8, 16 and 32.
Every lane is submitted before collection; one execution-order-independent
lane returns an unresolved duplicate while all healthy lanes complete. The
pending lane then recovers through its exact request, the first request beyond
the configured cohort rejects, every returned Principal is unique, and every
asset observation retains the requested physical Subnet and sole Root
controller.

The B1 bounded reset preflight passed the same 1, 8, 16 and 32 cohorts
separately for the empty control and exact-hash predecessor workload. Each
asset was observed running on one selected application Subnet with the Root as
sole controller;
the separately accounted setup top-up then freezes exactly 5T cycles
immediately before admission. Every management reset is submitted before any
response is collected. Every terminal asset remains running, on the selected
Subnet, Root-only and uninstalled, and its retained balance does not exceed the
5T start. The B1 capture rejected any raw-Wasm size or SHA-256 drift. Current-
source harness runs retain the lane and terminal-accounting proof while
validating the exact bytes they installed. These are deterministic PocketIC
facts, not throughput or platform-cost observations.

The local controller harness observes the exact transition
`[source] -> [source, destination] -> [destination]` for an empty same-Subnet
asset. Missing or contradictory routing evidence rejects before the first
controller mutation and leaves `[source]` intact. These remain PocketIC harness
facts; B2 must observe the separately authorized platform transition.

## Q3 Candidate Measurement Protocol

The accepted protocol identity is `canic-0.106-q3q4-v1`. It became immutable
when B1 was accepted on 2026-08-20. The B1 capture froze the installed
fixture's initialized heap and stable-memory observations below. No value below
authorizes execution on a remote network.

### Networks, Cohorts And Samples

| Environment | Cohorts | Measured repetitions | Purpose |
| --- | --- | ---: | --- |
| PocketIC local preflight | 1, 8, 16 and 32 lanes | one per cohort and journey | Deterministic admission, lane independence, exact retry and terminal-accounting proof only; never reported as platform throughput. |
| Maintainer-approved disposable remote network | 1, 8, 16 and 32 lanes | three per cohort and journey | Primary throughput and latency sample. |
| Separately approved IC-mainnet confirmation | one lane only | three per journey | Contract and order-of-magnitude confirmation; no 8/16/32 mainnet cohort is authorized by this protocol. |

One repetition admits exactly the cohort width. The three measured journeys
are:

1. create through `Ready`, with both the Ledger-creation segment and complete
   create-to-Ready latency retained from the same operation;
2. reset of the empty control fixture; and
3. reset of the installed workload fixture.

Thus the disposable sample contains 3, 24, 48 and 96 operations per journey
for cohorts 1, 8, 16 and 32 respectively. There is one separately labelled,
excluded one-lane warm-up for each journey and network. Warm-up assets and
costs remain in the effect ledger but never enter a statistic.

### Starting State And Fixtures

- Every paid create request supplies exactly `5_000_000_000_000` cycles and
  retains the separately accounted 100,000,000-cycle Ledger fee.
- Before each disposable repetition, the funding account holds at least the
  cohort's complete requested amount plus all Ledger fees plus a separate
  `10_000_000_000_000`-cycle unresolved/execution reserve. The run manifest
  records the exact starting balance rather than normalizing it after the run.
- Every reset asset starts with exactly `5_000_000_000_000` cycles, the same
  physical Subnet and the owning Root as sole controller. Any top-up used to
  restore that starting condition is a separate effect-ledger row.
- The empty reset-control fixture is running, root-controlled and uninstalled,
  with no module hash. Its status, cycles and controller observation is frozen
  immediately before admission.
- The installed fixture is the repository's `payload_limit_probe`, built at
  `v0.105.0` source with the Fast PocketIC profile, release build identity
  `1111111111111111111111111111111111111111111111111111111111111111`,
  protocol-profile digest
  `0404040404040404040404040404040404040404040404040404040404040404`,
  build fingerprint
  `21e57f6c8e65640e23c467e00a3977d1670d5023dff1f941a2155edcec9c5c4e`
  and raw Wasm SHA-256
  `e96e05382a8accfa13ecf67b24f9cb771117bca712a9395bf7d8279c06484206`
  (3,010,225 bytes). It is installed with its ordinary local no-argument
  lifecycle and receives no workload call before reset.
- PocketIC 15.0.0 observes, after deferred lifecycle completion and without a
  snapshot, 208,937,103 total memory bytes: 1,376,256 Wasm-memory bytes,
  201,392,128 stable-memory bytes, 64 global-memory bytes, 3,010,225 Wasm-binary
  bytes, zero custom-section bytes, 414 canister-history bytes, 3,145,728
  Wasm-chunk-store bytes and zero snapshot bytes. The B1 capture froze each
  value and the raw module hash against exact `v0.105.0` source. Later-source
  test runs verify their own installed bytes and do not rewrite or compare
  themselves to this immutable predecessor observation. Reproduction of this
  identity therefore requires the exact tagged source; a mismatch there
  invalidates the fixture rather than silently selecting a replacement.

The installed probe is a bounded representative Canic lifecycle workload, not
a claim about every App. Any 0.110 funding recommendation derived from it must
add the Q4 workload headroom rule and name this exact limitation.

### Event And Censoring Rules

- Admission time is the monotonic harness timestamp immediately before the
  exact operation is durably admitted. Start time is the timestamp immediately
  before its first external call.
- Creation segment success is the first reconciled Ledger result containing
  the exact created Principal. Complete creation success is the first truthful
  `Ready` row for that Principal after controller, module and balance checks.
- Reset success is the first truthful `Ready` row with the Root as sole
  controller and no installed module.
- Typed rejection before an external call is terminal rejected. A known
  external failure is terminal failed. Transport uncertainty, duplicate
  without a Principal and interrupted scheduling are unresolved until exact
  reconciliation proves success or a terminal condition.
- Each operation has a 15-minute stop deadline from admission. A cohort has no
  later than a 20-minute collection deadline. An unresolved lane is retained
  as right-censored at its own deadline and in reserved-exposure accounting.
  The harness does not retry after the deadline or wait for one lane before
  polling another.
- The exact request is the only paid retry. A test-induced response loss may
  suppress delivery once; it must not alter the retained request or operation
  identity.

### Statistics

Every cohort and journey reports admitted, started, completed, rejected,
failed and right-censored counts; cohort wall time; terminal completions per
minute; lane saturation; maximum simultaneously in-flight lanes; and per-lane
cycles requested, fees, retained balance and unresolved reserve.

Terminal latencies are reported with median and empirical maximum. For an even
terminal sample, the median is the arithmetic mean of the two central sorted
values. A nearest-rank p95 (`ceil(0.95 * n)`) is reported only when at least 20
terminal observations exist, so it is eligible for the disposable 8/16/32
cohorts only when their accepted terminal sample still meets that threshold.
Rejected and failed operations remain terminal latency observations under
their own outcome classes. Right-censored operations are never inserted at the
deadline into terminal percentiles and are reported separately with their
observed censoring duration. No cross-network aggregate or causal percentage
is permitted.

## Q4 Candidate Horizon And Balance Model

The empty standby horizon `H` is seven complete 24-hour days. The harness
records one observation immediately after fixture admission, then at least
every six hours and at the exact seven-day boundary: 29 scheduled observations
including both boundaries. A late or missing observation is retained with its
actual timestamp; it is not interpolated.

The standby asset starts running, root-controlled, uninstalled, on the exact
qualified Subnet with `5_000_000_000_000` cycles. No application ingress,
timer, install, top-up or controller change is admitted during `H`. The run
records balance, module hash, controller set, Canister status, memory metrics
available from the platform and every harness management call. Net burn is
starting balance minus terminal balance, adjusted only by separately proven
cycle additions or withdrawals. Wall-clock extrapolation cannot replace the
seven-day observation.

Let:

- `B_H` be measured adjusted burn across `H`;
- `B_interval_max` be the largest adjusted burn in one observed six-hour
  interval;
- `B_reset_max` be the largest measured asset-balance loss across one complete
  installed-fixture reset and verification; and
- `B_claim_max` be the largest measured loss across claim, install, lifecycle
  completion and activation of the exact installed fixture.

The frozen safety margin is `max(ceil(B_H / 4), 4 * B_interval_max)`. The
recovery reserve is `2 * B_reset_max`, representing one complete attempt and
one exact recovery attempt. All arithmetic is checked and rounded upward to
the next 100,000,000 cycles.

The report may propose these dated 0.110 inputs:

~~~text
standby_minimum_cycles = B_H + safety_margin + recovery_reserve

claim_threshold_cycles =
    5_000_000_000_000 + B_claim_max + safety_margin + recovery_reserve

claim_top_up_cycles =
    claim_threshold_cycles - standby_minimum_cycles

overfunded_observation_cycles = 2 * claim_threshold_cycles
~~~

The subtraction is checked and rejects a protocol contradiction when the
standby value exceeds the claim threshold. The 5T term is the exact configured
initial balance of the representative `payload_limit_probe` role for this
protocol; it is not a universal App minimum. A later workload with a larger
configured initial balance substitutes that exact value and reruns the claim
measurement. The overfunded value is informational only: it creates no maximum
balance, rejection, sweep or funding authority.

## Proposed B2 External-Effect Envelope

This proposal freezes arithmetic and call counts without authorizing a
network. Each network reuses its creation assets for the empty reset and then
the installed-workload reset. If reuse is impossible, the run aborts; it does
not create replacement assets beyond the physical-asset ceiling.

| Bound | Disposable remote network | IC-mainnet confirmation |
| --- | ---: | ---: |
| Cohort concurrency | 32 maximum | 1 maximum |
| Operations per journey, including one excluded warm-up | 172 | 4 |
| Physical assets | 172 maximum | 4 maximum |
| Paid creation amount | 860,000,000,000,000 | 20,000,000,000,000 |
| Maximum two-journey setup top-up principal | 1,720,000,000,000,000 | 40,000,000,000,000 |
| Frozen ordinary fee/refund envelope | 86,000,000,000 | 2,000,000,000 |
| Separate unresolved/execution reserve | 10,000,000,000,000 | 10,000,000,000,000 |
| Maximum funded exposure | 2,590,086,000,000,000 | 70,002,000,000,000 |

The focused pure guard derives the 172/4 operation and physical-asset counts
plus both funded-exposure totals with checked `u128` arithmetic from the frozen
cohorts, repetitions, 5T principal, fee-row count and 10T reserve.

The ordinary fee/refund envelope admits three 100,000,000-cycle Ledger rows
per creation request (initial fee plus two bounded failure/refund rows) and one
100,000,000-cycle `withdraw` fee for each of the at-most two setup top-ups per
asset. The separate 10T reserve remains available for unresolved or execution
accounting; every actual fee, refund and reserve use is retained rather than
netted. A run rejects before its first effect unless its spendable account can
cover the exact funded exposure for that network.

The authorization record remains deliberately unbound:

| Required authority | Current value |
| --- | --- |
| Protocol identity | Accepted `canic-0.106-q3q4-v1` on 2026-08-20 |
| Disposable network identity and destruction owner | Not supplied; execution unauthorized |
| Disposable signing/funding account and Root controller Principal | Not supplied; execution unauthorized |
| Disposable terminal disposition | Reconcile all rows as Root-only and uninstalled, then destroy the named disposable network; exact owner not supplied |
| IC-mainnet signing/funding account and Root controller Principal | Not supplied; execution unauthorized |
| IC-mainnet terminal asset disposition | Not supplied; no preservation, controller transfer, cycle withdrawal or deletion is authorized |

Separate maintainer authorization must replace every `Not supplied` cell with
one exact value before B2 execution. The Q1/Q6 dispositions were accepted with
B1 and do not authorize external effects. A higher asset, operation,
concurrency or funded-exposure value is a new plan and requires new
authorization; an unused ceiling grants no later spending authority.

## B1 Acceptance And Remaining B2 Gate

The maintainer accepted repository-local B1 on 2026-08-20. Q2 normative
provenance is complete and no empirical Q2 row exists. Q3/Q4 protocol
`canic-0.106-q3q4-v1` is frozen. The exact Q1 `EmptyRootAdmissions` blocker and
the four Q6 constraints—unbounded failure text, the structurally narrow
handoff-receipt bound, unbounded terminal-receipt retention and the absent
canonical receipt-map snapshot payload—are accepted as 0.110-owned work, not
as 0.106 corrections.

Exact binding of the B2 network, signing/funding identities and terminal asset
disposition is a separate post-B1 execution gate. It may remain unbound without
invalidating accepted repository-local B1, but B2 remains unauthorized and
0.106 remains incomplete while it is unbound.
