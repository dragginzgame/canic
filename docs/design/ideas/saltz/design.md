# Canic Design Idea: Cycle-Burn Waveform Component

Date: 2026-08-16

## Status

* Status: unnumbered design idea.
* Repository: Canic.
* Purpose: experimental Canic-managed Component capable of deliberately burning a bounded, pre-authorized quantity of cycles according to a time-domain waveform.
* Initial demonstration target: reproduce the neon Swiss-mountain profile from the selected Saltz restaurant image over one complete ICP Dashboard `1D` Cycle Burn Rate window.
* This is not yet a numbered release design and authorizes no IC-mainnet effect.
* Burner implementation requires Canic's accepted shared `ic-timers` consumer
  hard cut. Any mainnet run must use Canic's accepted external-effect
  authorization contract for exact network, identity, asset, cycle,
  concurrency and terminal-state bounds.
* Qualification disposition: B0a and the B0b methodology are approved. B0b
  still requires a separate exact authorization before any live calibration
  effect. B1 implementation remains held until the promotion gates close.

The idea should remain under:

```text
docs/design/ideas/saltz/design.md
```

until the dashboard sampling contract and direct-burn observability have been qualified.

## Decision

Add one deliberately small Canic Component whose only unusual capability is:

```text
request an exact bounded amount of its own cycles
at an exact sequence of scheduled times
according to one immutable approved waveform
```

The returned actual burn must equal the requested amount. A smaller platform
result is recorded as a partial burn and fails the run closed; it is never
retried or caught up.

The Component is an execution engine, not an image processor, Dashboard scraper, autonomous fleet allocator or general-purpose scheduler.

The waveform is compiled by Canic host tooling before execution.

The Component:

1. receives one immutable run plan;
2. proves sufficient cycle funding and safety reserve;
3. arms one future start time;
4. burns the requested amount for each scheduled bucket;
5. records the actual amount burned;
6. never catches up missed burns into a later bucket;
7. can be aborted permanently;
8. completes after the final bucket; and
9. exposes bounded status and receipts.

The IC currently exposes a direct cycle-burning system primitive. The
[IC interface specification](https://legacy.internetcomputer.org/docs/references/ic-interface-spec)
defines `ic0.cycles_burn128`, while the current Rust CDK exposes the ergonomic
`ic_cdk::api::cycles_burn(amount: u128) -> u128`, returning the amount actually
burned.

## Why A Single Component

The target signal does not require a 100- or 1,000-Canister estate.

The intended topology is:

```text
Fleet Coordinator
    |
    v
Fleet Subnet Root
    |
    +-- Wasm Store
    |
    `-- Cycle Burn Waveform Component
```

Optionally a second identical Component may exist as an inactive replacement, but only one Component may own an active waveform run.

Multiple simultaneous burners would make reconciliation and timing harder without improving the waveform.

The direct burn primitive is deliberately preferred over manufacturing cycle consumption through compute allocation, storage, ingress traffic or pointless Wasm execution.

## Implementation Ownership

If promoted, the burner lives in a dedicated experimental Saltz application
or canister package, not in `canic-core`, the public `canic` facade or a
general-purpose operator command. Canic supplies normal lifecycle, authority,
protected diagnostics and shared-timer integration; Saltz owns the destructive
waveform domain.

The host compiler and run-authoring surface are Saltz-specific. Promotion must
not introduce a generic `canic burn` command or make deliberate cycle
destruction a standard managed-Component capability.

## Initial Saltz Waveform

The initial source is the selected public Saltz photograph.

The current reference artifact is:

```text
saltz_24h_waveform_floor_100B_860.csv
SHA-256: 8a9b886a493db55989a4f2c119d5bf99dea237302970953dd98b2ed28f5a0f97
```

It contains exactly 860 indexed buckets with contiguous offsets covering
`86_400_000_000_000` nanoseconds. Its normalized heights span
`0..=1_000_000` parts per million and its provisional visible target spans
`100_000_000_000..=116_666_666_667` cycles per second.

For feasibility orientation, integrating the CSV's provisional visible target
over 24 hours yields approximately `9_249.610124 Tcycles`. Constant illustrative
background scenarios produce:

| Assumed average background | Controlled burn over 24 hours |
| --- | ---: |
| `0 Bcycles/second` | `9_249.610124 Tcycles` |
| `30 Bcycles/second` | `6_657.610124 Tcycles` |
| `40 Bcycles/second` | `5_793.610124 Tcycles` |
| `50 Bcycles/second` | `4_929.610124 Tcycles` |

The zero-background row is the natural provisional funding upper bound before
reserve and execution allowance. These are reference calculations, not a burn
budget or authorization: the qualified request cadence, rounding policy,
measured background and hard run ceilings remain authoritative.

This CSV is immutable reference input, not an executable `RunPlan`. Its
approximately 100.465-second buckets and visible-rate mapping remain
provisional until the exact Dashboard observation contract and economic
feasibility gate are accepted. Promotion must additionally record the exact
source-image bytes, source URL, reuse permission and canonical extracted-point
encoding; public availability alone is not evidence of reuse permission.

The extracted neon line contains approximately:

```text
860 horizontal source samples
55 pixels of vertical range
```

The source geometry is preserved.

The profile compiler stores the original trace as normalized points:

```text
x: 0.0 .. 1.0
h: 0.0 .. 1.0
```

where:

```text
x = horizontal position through the photograph

h = (lowest_source_y - source_y)
    / source_vertical_range
```

No independent Y-axis stretching, smoothing or artistic reconstruction is permitted by the canonical Saltz profile.

The source profile is identified by a digest:

```rust
struct WaveformIdentity {
    source_sha256: [u8; 32],
    extracted_points_sha256: [u8; 32],
    reference_artifact_sha256: [u8; 32],
    compiler_version: String,
}
```

The exact dashboard cadence remains a qualification input.

The 860-point source waveform is therefore not necessarily executed as 860 burns.

Instead:

```text
source waveform
      |
      v
piecewise-linear master curve
      |
      v
resample to qualified Dashboard bucket cadence
      |
      v
RunPlan
```

No additional smoothing occurs during resampling.

## Dashboard Qualification Gate

Before promotion, Canic must freeze the normative observation contract and
establish the remaining facts experimentally.

### Q0. Exact observable target and economic feasibility

Freeze:

* the exact public chart URL and selected `1D` view;
* whether the target is the global or a Subnet-filtered series;
* the exact API host, path, response schema and units;
* the `start`, `end`, `step`, Subnet and other query parameters;
* the native metric resolution separately from API aggregation;
* the aggregation epoch and exact bucket-boundary function;
* burn-to-bucket attribution and the permitted execution phase;
* the frontend revision, query construction, downsampling and renderer; and
* the qualification timestamp and freshness policy.

The digested observation contract contains at least:

```rust
struct ObservationContract {
    api_contract_sha256: [u8; 32],
    frontend_contract_sha256: [u8; 32],

    aggregation_epoch_ns: u64,
    aggregation_bucket_width_ns: u64,
    execution_phase_offset_ns: u64,
    minimum_arm_lead_ns: u64,

    attribution: BurnAttribution,
}

enum BurnAttribution {
    SinglePulsePerBucket,
}
```

The aggregation width and minimum arm lead must be nonzero, and the execution
phase must be strictly less than the aggregation width. B0b selects a lead
long enough for authorization confirmation, funding reconciliation, initial
forecast submission, timer registration and an operator abort after reviewing
the final preview.

`SinglePulsePerBucket` may be admitted only if B0b proves that one
instantaneous `cycles_burn` operation at the qualified phase produces the
intended observed rate sample. If the metric requires distributed burn within
an aggregation interval, B0b stops B1; the executor is not silently expanded
into a different scheduling model.

The current public IC API documents
`GET /api/v3/metrics/cycle-burn-rate` with caller-selected `start`, `end`,
`step` and optional `subnet` parameters. Its current default `step` is 7,200
seconds. The current contract is published in the
[Dashboard OpenAPI](https://ic-api.internetcomputer.org/api/v3/openapi.json),
with the API families described by the
[Dashboard API reference](https://legacy.internetcomputer.org/docs/references/dashboard-apis/).
The promoted design must cite the exact current OpenAPI contract and
record separately what the experiment observed; neither the default nor the
current frontend behavior is an immutable platform guarantee.

Before Component implementation, use measured background and tail variation
to calculate:

```text
minimum recognizable waveform amplitude
maximum authorized 24-hour cycle exposure
expected execution and reserve allowance
```

The maintainer must predeclare an economic no-go ceiling. If the amplitude
needed for a recognizable waveform exceeds it, the idea stops before Burner
implementation.

### Q1. Direct burn visibility

Prove that a controlled `cycles_burn` operation is reflected in the public Cycle Burn Rate metric in the expected amount and time window.

The management surface separately exposes a `burned_cycles` consumption category, but that API is currently documented as experimental and must not become a runtime dependency of the Component.

The authoritative run evidence is:

```text
requested burn
actual cycles_burn() result
before/after Component balance
Dashboard observation
```

### Q2. Dashboard bucket cadence

Determine the native metric resolution, the effective interval returned for
the frozen API request and the independently effective rendered `1D` interval.

The public Dashboard surfaces more than one metrics API and permits a
caller-selected aggregation step. The promoted design must name one exact
endpoint and request rather than assume that every source pixel is
independently displayed.

### Q3. Metric publication lag

Measure:

* time from burn to metric visibility;
* aggregation boundary behavior;
* whether one burn contributes to one or several displayed samples;
* frontend downsampling;
* Dashboard Y-axis autoscaling; and
* variance between Metrics API data and the rendered line.

Until these are measured, the Saltz waveform is a design target rather than a claim of pixel-perfect reproduction.

## Fundamental Accuracy Limit

The public line contains:

```text
total visible burn
    =
unrelated IC network burn
    +
our controlled burn
```

The Component can add cycles.

It cannot subtract unrelated network activity.

Therefore:

```text
if background_rate < target_rate:
    burn target_rate - background_rate

if background_rate >= target_rate:
    burn 0
    record uncontrollable overshoot
```

No architecture can correct a positive external disturbance after the relevant Dashboard bucket has passed.

The authorized mainnet experiment must therefore choose sufficient vertical
headroom that normal network variation remains small relative to the waveform
amplitude without exceeding the predeclared economic ceiling.

## Waveform Plan

A prepared run contains an immutable target profile:

```rust
struct RunPlan {
    run_id: RunId,
    plan_payload_sha256: [u8; 32],
    payload: RunPlanPayload,
}

struct RunPlanPayload {
    waveform: WaveformIdentity,
    observation_contract_sha256: [u8; 32],

    start_time_ns: u64,
    run_duration_ns: u64,
    point_count: u32,

    target_floor_cycles_per_second: u128,
    target_amplitude_cycles_per_second: u128,

    assumed_background_cycles_per_second: u128,

    max_total_burn_cycles: u128,
    max_bucket_burn_cycles: u128,
    minimum_reserve_cycles: u128,
    execution_allowance_cycles: u128,
    reserve_horizon_ns: u64,

    max_lateness_ns: u64,
    max_consecutive_late_buckets: u32,
    max_forecast_age_ns: u64,
    max_forecast_points: u32,
}
```

The host compiler emits `RunPlanPayload`; it does not choose `run_id`.
`plan_payload_sha256` covers the canonical payload bytes and is the digest
shared by the host preview, Canister execution and terminal header. After
accepting the draft, the Component assigns `run_id` and wraps the unchanged
payload. A run authorization binds both the assigned `run_id` and the payload
digest.

Promotion must freeze compile-time maxima for point count, run duration,
serialized plan size and receipt count. `point_count` and `run_duration_ns`
must be nonzero. Every derived boundary, duration, execution phase and lateness
deadline must be checked, ordered and contained within the run.

### Exact rational time axis

One fixed integer bucket duration cannot partition every qualified run exactly.
The reference CSV, for example, divides one 24-hour interval across 860 points
using rational boundaries. `RunPlan` therefore stores the total duration and
point count as schedule authority.

For `0 <= i < point_count`:

```text
bucket_start(i)
    =
start_time_ns
    + floor(i * run_duration_ns / point_count)

bucket_end(i)
    =
start_time_ns
    + floor((i + 1) * run_duration_ns / point_count)

bucket_duration(i)
    =
bucket_end(i) - bucket_start(i)

execute_at(i)
    =
bucket_start(i) + observation_contract.execution_phase_offset_ns
```

All multiplication, division and addition use the shared overflow-safe
rational helper. This produces exactly `run_duration_ns` without accumulated
phase error and matches the boundary semantics embodied by the reference CSV.

`start_time_ns` denotes the first observation boundary, not necessarily the
first burn instant. The plan is admissible only when every derived bucket maps
exactly onto the qualified observation contract and:

```text
bucket_start(i) <= execute_at(i)

execute_at(i) + max_lateness_ns < bucket_end(i)
```

`arm_run` also requires:

```text
start_time_ns >= now_ns + observation_contract.minimum_arm_lead_ns

start_time_ns >= observation_contract.aggregation_epoch_ns

(start_time_ns - observation_contract.aggregation_epoch_ns)
    % observation_contract.aggregation_bucket_width_ns
    == 0
```

An arbitrary UTC start time is invalid even when it is sufficiently far in
the future. B0b must freeze the aggregation epoch, attribution model and phase
before any `RunPlan` can pass this check.

Each target point is represented as a bounded integer rather than floating point:

```rust
struct WavePoint {
    height_ppm: u32,
}
```

where:

```text
0       = lowest valley
1_000_000 = highest peak
```

The desired total network rate for bucket `i` is:

```text
target_rate[i]
    =
floor
    +
    amplitude * height_ppm[i] / 1_000_000
```

This calculation uses checked quotient/remainder arithmetic without an
overflowing intermediate product. The compiler and Component must share the
same rounding and remainder policy.

## Background Forecasting

The waveform itself is immutable after preparation.

Background forecasts may change.

Canic host tooling may observe the frozen Metrics API contract and submit
updated background forecasts for future, unexecuted buckets. Publication lag
means this is prediction, not correction of the bucket that produced the
observation.

The owning authority can influence the derived burn through these forecasts.
That authority is bounded by the immutable waveform and financial envelope,
and every accepted forecast carries exact observation provenance; the design
does not claim that provenance proves the forecast correct.

Instead:

```rust
struct BackgroundForecast {
    run_id: RunId,
    revision: u64,
    first_index: u32,
    generated_at_ns: u64,
    source_observed_from_ns: u64,
    source_observed_to_ns: u64,
    observation_contract_sha256: [u8; 32],
    source_response_sha256: [u8; 32],
    forecast_sha256: [u8; 32],
    rates_cycles_per_second: Vec<u128>,
}
```

`forecast_sha256` covers the canonical encoding of every other forecast field.
The Component recomputes it and rejects a mismatch before considering the
revision.

The Component derives the burn itself:

```text
desired_rate = waveform_target(index)

required_rate =
    max(0, desired_rate - background_forecast)

required_cycles =
    floor(
        (required_rate * bucket_duration(index) + carried_remainder)
        / 1_000_000_000
    )
```

The implementation must use an overflow-safe checked rational helper rather
than materializing an unchecked `u128` product. It carries the sub-cycle
remainder deterministically across buckets. The host budget preview and
Component use the identical helper and reconcile the final remainder.

The conversion state advances through every scheduled index, including a
missed or overshot bucket, so later requests remain identical to the preview.
Advancing the fractional remainder never transfers the missed bucket's
integral burn into a later bucket.

This keeps the artistic target immutable while allowing bounded adjustment
from a forecast of future background activity.

Background forecasts:

* apply only to future buckets;
* cannot rewrite an executed bucket;
* cannot increase the run's immutable total budget;
* cannot increase the per-bucket ceiling;
* must come from the owning Fleet authority;
* expire when older than the immutable observation-age ceiling;
* contain at most the promoted maximum number of points;
* cannot extend beyond the immutable run horizon;
* use monotonically increasing revisions; and
* are idempotent only when the same revision has the same digest.

Replacement semantics are exact:

```text
revision N+1 supersedes the active forecast for every still-unexecuted
index explicitly contained in N+1

an input containing an already-executed index is rejected

future indexes omitted from N+1 retain their prior source or fallback

same revision + same digest = idempotent success
same revision + different digest = conflict
older revision = stale rejection
```

If no fresh forecast controls an index, the plan uses its immutable fallback
background rate. The receipt freezes which authority controlled the executed
index, so later forecast replacement cannot obscure provenance.

A future qualification may choose to fail closed instead.

## Why The Burner Does Not Fetch The Dashboard Itself

The first implementation should not add HTTPS observation logic to the burner.

The Component should have one responsibility:

```text
execute an authorized cycle-burn waveform or fail closed exactly
```

Canic host tooling can already interact with external services without making the durable on-chain executor responsible for:

* HTTP consensus;
* JSON parsing;
* external availability;
* Dashboard API schema changes;
* retries;
* publication lag;
* frontend-specific behavior.

This separation also makes PocketIC testing straightforward.

The Component accepts bounded, provenance-carrying forecasts; it does not
decide what the public internet currently looks like or assert that a host
prediction is ground truth.

## Run State

The durable run state is:

```rust
enum RunState {
    Prepared,
    Armed,
    Running,
    Completed,
    Aborted,
    Failed,
}
```

### Prepared

The complete waveform and limits are stored.

No timer is active.

### Armed

The plan has passed funding and invariant checks.

The first execution time is in the future.

### Running

At least one bucket has executed.

Only future background forecasts and abort are accepted.

### Completed

Every scheduled bucket has either:

* burned successfully; or
* been explicitly recorded as skipped/overshot.

No further burn is possible for this `run_id`.

### Aborted

Operator termination is permanent.

No resume operation exists.

A new attempt requires a new `run_id`.

### Failed

An invariant required safe execution to stop.

Examples:

* insufficient reserve;
* arithmetic overflow;
* run-plan corruption;
* impossible index transition;
* partial actual burn;
* actual burn greater than the requested or authorized amount;
* excessive scheduling lateness.

Failure never automatically opens a replacement run.

### Run identity and replacement

Run identity is Canister-owned and monotonic:

```rust
struct RunId(u64);

struct RunIdentityState {
    next_run_id: u64,
}
```

The external `prepare_run` request does not supply `run_id`. After accepting a
new draft, the Component allocates the current `next_run_id` and advances the
high-water mark with checked arithmetic. Deleted historical evidence therefore
does not require an ever-growing set of used caller-selected identifiers.

Only one `Prepared`, `Armed` or `Running` run may exist. Repeating
`prepare_run` while the identical prepared payload digest remains current returns
the existing Canister-assigned ID; a conflicting plan is rejected. A bounded
`operator_reference` and external authorization digest may be retained for
reconciliation, but neither is uniqueness authority.

Every retained terminal receipt bundle has a compact interpretation header:

```rust
struct TerminalRunHeader {
    run_id: RunId,
    plan_payload_sha256: [u8; 32],
    waveform_digest: [u8; 32],
    started_at_ns: u64,
    terminal_at_ns: u64,
    terminal_state: RunState,
    totals: RunTotals,
}
```

Terminal headers and receipts share one promoted maximum retained-run count.
When that bound is full, preparing a replacement fails closed until an
explicitly authorized export and retention disposition removes an entire
header-and-receipt bundle. No evidence is automatically evicted. The monotonic
high-water mark remains after pruning and permanently prevents ID reuse.

## Scheduling

Use timer-driven execution rather than an external ingress call for every waveform point.

The Component is an `ic-timers` consumer. Canic's accepted shared-timer hard
cut is a prerequisite to Burner implementation. Saltz owns the fixed callback
identity and the business decision that the next bucket remains demanded;
`ic-timers` owns registration, deadline arbitration, cancellation, runtime
inventory and provider mechanics.

The Component holds only native `ic_timers::OnceRegistration` custody required
to cancel or replace its current registration. The callback returns native
`ic_timers::TimerRunResult` and uses `TimerDirective::ScheduleAt` for the next
absolute deadline. No Saltz-local timer directive, scheduler state, provider
generation or generic scheduling facade is allowed.

The active run does not support upgrade reconstruction. Module replacement is
prohibited while the timer owns an armed or running waveform.

The Component should use **one-shot scheduling**.

After executing bucket `i`:

```text
derive bucket_start(i + 1) from total duration and point count
add the qualified execution phase
register next one-shot timer
return
```

Do not implement the waveform using a blind recurring interval.

Every bucket boundary derives from:

```text
start_time_ns + floor(index * run_duration_ns / point_count)
```

rather than an integer-duration recurrence such as:

```text
now + nominal_bucket_duration
```

This prevents both execution delay and integer-division remainder from
accumulating into clock drift. The actual callback deadline is the derived
boundary plus the qualified execution phase.

## Late Execution

Exact image geometry is more important than spending the planned total.

Therefore missed burns are never caught up.

For bucket `i`:

```text
if now <= execute_at(index) + max_lateness:
    execute bucket

else:
    record MissedLate
    burn nothing
    move to next future bucket
```

Never:

```text
burn bucket i and bucket i+1 together
```

because doing so would change the public graph.

The immutable `max_consecutive_late_buckets` plan field determines when
consecutive lateness forces the entire run into `Failed`.

## Upgrade Prohibition During An Active Run

Timing fidelity is more valuable than live upgrade support for this bounded
experiment. Canic-owned module installation and upgrade workflows must reject
the operation while the Saltz run is `Armed` or `Running`, even when the
candidate Wasm hash is unchanged.

The lifecycle hooks enforce the same invariant as defense in depth:

```text
Prepared:
    module replacement may proceed only under ordinary Canic release policy

Armed | Running:
    pre_upgrade traps
    post_upgrade encountering persisted active-run state traps

Completed | Aborted | Failed:
    module replacement may proceed only under ordinary Canic release policy
```

The operator must abort the run before maintenance that replaces Wasm. There
is no same-Wasm active-run reconstruction path, no attempt to catch up buckets
after maintenance and no cross-release migration, adoption, mixed-version
operation or compatibility recovery.

## Exact Burn Operation

Immediately before a burn:

```text
1. derive target amount using checked arithmetic;
2. enforce bucket maximum;
3. enforce remaining total-run budget;
4. inspect liquid cycle balance;
5. preserve minimum safety reserve;
6. invoke cycles_burn(amount);
7. require the returned actual amount to equal the request;
8. record a partial result and fail closed if it is smaller;
9. update aggregate run accounting;
10. persist the receipt;
11. schedule the next bucket.
```

The current Rust CDK's `cycles_burn` returns the amount actually burned, and
the IC primitive burns at most the request and at most the liquid balance. The
receipt therefore records platform truth rather than merely the requested
amount. A partial result is never retried within the bucket or transferred to
a later bucket.

### Atomic burn-and-receipt invariant

Normative requirement:

```text
intentional burn and its receipt commit atomically within one
non-awaiting replicated message execution
```

The platform basis is the IC interface specification: `cycles_burn128`
reduces the current execution state's balance, while a message trap discards
the execution-state mutations from that execution. This specification
contract, not PocketIC behavior, is the authority.

All fallible validation and storage-capacity checks precede the burn. There is
no `await` after entering the burn-and-receipt commit section. PocketIC remains
regression evidence: fault injection immediately before and immediately after
`cycles_burn` must verify the expected platform semantics and prove that the
maintained implementation cannot leave an unreceipted intentional burn.
Ordinary message-execution cost is accounted separately.

## Cycle Reserve

The canister must never intentionally burn itself below its operational reserve.

Before `arm_run`:

```text
liquid_balance
    >=
max_total_burn_cycles
+ minimum_reserve_cycles
+ execution_allowance_cycles
```

`minimum_reserve_cycles` is a horizon-qualified plan input, not a timeless
safe balance. Its evidence records:

```text
reserve_horizon
measured idle burn over that horizon
recovery and abort allowance
terminal-disposition allowance
safety margin
```

`execution_allowance_cycles` separately covers the measured upper bound for
timer callbacks, stable writes, protected status and final reconciliation.
The same liquid-balance and residual-reserve check runs immediately before
every nonzero bucket.

The per-bucket condition is:

```text
liquid_balance
    >=
requested_bucket_burn
+ minimum_reserve_cycles
+ remaining_execution_allowance_cycles
```

If not:

```text
InsufficientRunFunding
```

No partial automatic start occurs.

The Component cannot autonomously acquire cycles.

Canic funding remains a separate authority.

## Run Receipts

Each bucket produces one bounded receipt:

```rust
struct BurnReceipt {
    run_id: RunId,
    index: u32,

    bucket_start_ns: u64,
    bucket_end_ns: u64,
    scheduled_at_ns: u64,
    executed_at_ns: Option<u64>,

    target_rate: u128,
    background_rate_used: u128,
    background_source: BackgroundSource,

    requested_burn_cycles: u128,
    actual_burned_cycles: u128,

    outcome: BucketOutcome,
}

enum BackgroundSource {
    PlanFallback,
    Forecast {
        revision: u64,
        forecast_sha256: [u8; 32],
    },
}
```

`scheduled_at_ns` is the phase-adjusted `execute_at(index)`, while the explicit
bucket boundaries preserve the observation window used for attribution.

The bucket result is:

```rust
enum BucketOutcome {
    Burned,
    PartialBurn,
    ZeroRequired,
    BackgroundOvershoot,
    MissedLate,
}
```

At approximately hundreds of points per day, complete receipt storage remains
bounded by the promoted immutable point and receipt maxima.

No unbounded event log is required.

## Aggregate Run Evidence

Maintain checked durable totals:

```rust
struct RunTotals {
    buckets_total: u32,
    buckets_completed: u32,
    buckets_burned: u32,
    buckets_partial: u32,
    buckets_zero_required: u32,
    buckets_overshot: u32,
    buckets_late: u32,

    cycles_requested: u128,
    cycles_burned: u128,
}
```

For every terminal run, receipts and totals must satisfy:

```text
buckets_completed
    = buckets_burned
    + buckets_partial
    + buckets_zero_required
    + buckets_overshot
    + buckets_late

cycles_burned <= cycles_requested <= max_total_burn_cycles
```

`Completed` requires `buckets_completed == buckets_total` and zero partial
burns. A partial receipt instead leaves the run terminal in `Failed`.

The final report can reconcile:

```text
initial balance
- actual waveform burn
- ordinary execution cost
= final balance
```

## Proposed Candid Surface

This is illustrative until promoted.

```text
prepare_run
arm_run
abort_run

submit_background_forecasts

run_status
run_receipts
```

No generic:

```text
burn_cycles(amount)
```

endpoint should exist.

That would turn a tightly bounded waveform executor into a general remote cycle-destruction primitive.

Every burn must derive from the approved immutable `RunPlan`.

## Authority

Only the owning Canic control-plane authority may:

* prepare a plan;
* arm a plan;
* submit background forecasts and their observation evidence; or
* abort the run.

Read access to detailed balance or funding information should remain restricted.

ICP security guidance recommends not publicly exposing canister cycle balances; detailed financial status should therefore follow Canic's existing protected diagnostics model rather than become an anonymous query.

The public-safe status may expose only:

```text
run state
current bucket
total buckets
scheduled completion
waveform identity
```

## Stable State

This idea intentionally does not choose stable-memory IDs yet.

Promotion must first consume Canic's current stable-allocation inventory.

The required durable authorities are only:

1. monotonic `next_run_id` high-water mark;
2. current `RunPlan` and its immutable identity;
3. immutable waveform points;
4. current run state and index;
5. bounded terminal headers and per-bucket receipt bundles;
6. aggregate totals; and
7. latest bounded background-forecast window and provenance.

No generic job engine, task queue or scheduler abstraction should be introduced.

## Host Profile Compiler

The host owns image-to-waveform preparation.

For the Saltz demonstration:

```text
selected image
    |
    v
extract neon centreline
    |
    v
860-point master trace
    |
    v
preserve native aspect ratio
    |
    v
resample to qualified Dashboard cadence
    |
    v
map y to floor + amplitude
    |
    v
produce authenticated, digested RunPlan
```

The compiler should also render a preview using the measured Dashboard plot geometry.

Before a real run, the operator sees:

```text
SOURCE IMAGE

EXPECTED DASHBOARD LINE

RUN COST / MAXIMUM CYCLE EXPOSURE
```

from the exact same `RunPlanPayload`.

The preview and executable payload must share `plan_payload_sha256`. The owning
Canic authority authenticates the submission. A digest is not described as a
signature; if offline signatures are later required, promotion must define
the signer, domain separator, signed bytes and verification contract
explicitly.

## No Hidden Image Logic In The Canister

The canister never knows what Saltz is.

It does not store PNG or JPEG data.

It stores only:

```text
waveform identity
normalized points
timing
burn bounds
```

The executor remains waveform-shaped rather than Saltz-image-shaped, but this
does not make it a public Canic capability. Any later controlled load/burn use
requires its own explicit promotion and authorization rather than inheriting a
generic destructive surface.

## Mainnet Run Authorization

Every real run requires a separate immutable authorization record containing:

1. Canic commit;
2. burner Wasm hash;
3. network;
4. physical Subnet;
5. root Principal;
6. burner Principal;
7. Canister-assigned run ID;
8. plan-payload digest;
9. waveform digest;
10. observation-contract digest and global/Subnet scope;
11. exact UTC start time;
12. expected duration;
13. bucket count;
14. maximum per-bucket burn;
15. maximum total burn and human-readable XDR estimate;
16. reserve horizon and minimum residual reserve;
17. execution allowance;
18. permitted background-forecast mode;
19. economic no-go ceiling;
20. public disclosure or coordination record;
21. abort conditions;
22. post-run reconciliation location; and
23. terminal asset disposition.

Approval of the Component itself never approves a particular burn run.

`prepare_run` is inert and returns the Canister-assigned ID plus payload
digest. The operator then freezes the authorization record around that exact
pair. `arm_run` verifies the pair and every authorization bound before
scheduling the first timer; preparation alone can never become authority to
burn.

The Cycle Burn Rate graph is shared public observability. The authorization
must decide how the experiment is disclosed so operators are not left to
interpret an intentionally shaped signal as unexplained network behavior.

## Abort Conditions

At minimum, automatically abort or fail closed when:

* remaining balance falls below reserve;
* total burned cycles would exceed authorization;
* one bucket would exceed its ceiling;
* schedule digest changes;
* too many buckets are late;
* background exceeds the drawable target for the immutable consecutive-bucket ceiling;
* timer reconstruction is inconsistent;
* checked arithmetic fails; or
* durable receipt state contradicts the next index.

Manual abort is immediate and permanent.

## Validation

### Pure model tests

Prove:

* exact source-point normalization;
* exact piecewise-linear resampling;
* no independent vertical stretching;
* checked rate-to-cycle arithmetic;
* exact nanosecond-to-second conversion and remainder reconciliation;
* rational boundaries cover the exact run duration for non-divisible point counts;
* every scheduled execution is phase-aligned and contained in its bucket;
* total budget equality;
* bucket ceiling enforcement;
* background subtraction;
* zero burn on overshoot;
* immutable waveform digest;
* no historical catch-up.

### PocketIC

Prove:

* prepare → arm → run → complete;
* arming before the minimum lead or off the aggregation phase is rejected;
* abort before start;
* abort while running;
* same-Wasm and different-Wasm upgrades are rejected while armed or running;
* late-bucket skip;
* stale background forecast handling;
* newer forecast replacement for exact future indexes;
* idempotent, conflicting, stale and oversized forecast handling;
* insufficient funding;
* exact and partial `cycles_burn` receipt accounting;
* traps immediately before and after burn cannot leave an unreceipted intentional burn; and
* the timer appears in shared inventory with Saltz ownership.

PocketIC validates Component semantics, not public Dashboard rendering.

### Disposable/mainnet qualification

Prove separately:

* direct burn appears in Cycle Burn Rate;
* exact aggregation window;
* latency to Metrics API;
* frontend sampling;
* autoscaling behavior;
* observed versus expected profile error.

No large artistic run begins before a very small calibration pulse has established this mapping.

## Fidelity Metric

The final experiment should define image accuracy numerically.

For every rendered Dashboard point:

```text
expected normalized height
observed normalized height
absolute error
```

Report:

```text
mean absolute error
maximum error
correlation
late/missing buckets
background-overshoot buckets
```

The Saltz run should only be described as successful if it passes a predeclared fidelity threshold.

## Suggested Implementation Sequence

### B0a — Observation Contract And Economic Feasibility

* freeze the exact API and rendered `1D` target;
* distinguish native, API and frontend aggregation;
* measure background and tail variation without an external effect;
* calculate the minimum recognizable amplitude and maximum exposure;
* record the economic no-go ceiling; and
* stop before implementation when the ceiling cannot be met.

### B0b — Separately Authorized Platform Calibration

* prove direct burn reaches the Dashboard metric;
* determine the 1D aggregation epoch, width and attribution;
* prove or reject single-pulse-per-bucket execution;
* freeze the permitted execution phase and minimum arm lead;
* determine publication latency;
* freeze Y-axis behavior;
* perform only tiny bounded burns.

### B1 — Burner Component

* require Canic's accepted shared `ic-timers` consumer hard cut;
* add bounded run model;
* add immutable waveform storage;
* implement timer execution;
* implement direct burn;
* implement receipts;
* prohibit module replacement while armed or running;
* add PocketIC tests.

### B2 — Saltz Compiler

* freeze selected image;
* freeze 860-point extracted master;
* resample to current Dashboard cadence;
* produce expected plot preview;
* produce cycle-budget calculation.

### B3 — Optional Forecast-Based Adjustment

* add bounded, provenance-carrying future background forecasts;
* preserve immutable waveform;
* validate stale/missing observation behavior;
* prove forecasting cannot exceed the approved cycle envelope.

### B4 — Explicitly Authorized Mainnet Experiment

* freeze complete run authorization;
* pre-fund exact maximum plus reserve;
* execute one 24-hour run;
* reconcile every bucket;
* compare actual Dashboard result to expected Saltz trace.

No B4 authorization is inherited from B0b, Component approval or available
balance. It follows Canic's accepted external-effect authorization contract
and includes the public-observability disclosure decision.

## Non-Goals

This idea does not:

* create hundreds of burner canisters;
* reserve large amounts of Subnet compute;
* generate cycle burn by executing useless instruction loops;
* introduce a generic cycle-burning endpoint;
* fetch or process image files on-chain;
* implement a generic Canic job scheduler;
* automatically create Fleets;
* automatically fund itself;
* infer permission from available cycle balance;
* replay missed burns;
* guarantee control over unrelated IC traffic;
* guarantee pixel-perfect Dashboard output before Dashboard cadence is qualified; or
* authorize any external effect merely because this design is implemented.

## Acceptance Criteria For Promotion

This idea is ready to become a numbered implementation design only when:

1. `cycles_burn` visibility in the public burn-rate metric is proven with a bounded calibration;
2. the `1D` graph's aggregation epoch, width, attribution and execution phase
   are measured;
3. publication lag and autoscaling are understood;
4. the selected Saltz source image and extracted waveform have immutable digests;
5. a preview generated from the executable plan matches the source geometry;
6. rational scheduling covers the exact run duration without drift;
7. all module replacement is prohibited while armed or running;
8. all burn arithmetic is checked and bounded;
9. a hard total-cycle authorization cannot be exceeded;
10. the Component preserves an operational cycle reserve;
11. background forecasting cannot mutate the waveform;
12. every executed bucket has a durable receipt; and
13. no mainnet effect is possible without a separately approved run authorization.

Promotion additionally requires:

14. the exact API/frontend observation contract and global/Subnet scope are
    digested;
15. the measured amplitude fits below the predeclared economic no-go ceiling;
16. partial burn is terminal, receipted and never caught up;
17. burn and receipt atomicity is proved under injected failure;
18. native `ic-timers` ownership is visible after install and active scheduling;
19. reserve and execution allowances are horizon-qualified;
20. run, forecast, receipt and retention bounds are frozen;
21. minimum arm lead and exact start-phase rejection are proved;
22. monotonic run identity and bounded terminal interpretation headers are
    proved;
23. forecast replacement and receipt provenance are unambiguous;
24. the active-run upgrade prohibition is proved for same and different Wasm;
25. no cross-release upgrade support is implied; and
26. the source image, permission and canonical waveform artifact are frozen.

## Design Principle

The Component should be almost boring.

Its entire trusted responsibility should reduce to:

```text
given:
    one immutable waveform,
    one immutable time axis,
    one immutable financial envelope,
    and bounded, provenance-carrying background forecasts,

request exactly the permitted amount for the current bucket,
fail closed if the platform burns less,
record exactly what happened,
and never compensate for mistakes by burning later.
```

Everything else belongs outside the canister.
