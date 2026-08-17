# Canic Design Idea: Standalone Cycle-Burn Waveform

Date: 2026-08-16

## Status

- Status: unnumbered experiment whose first Subnet-scoped executor was
  terminally aborted before waveform execution after the intended canvas was
  clarified as the global Dashboard homepage.
- Repository: Canic.
- Runtime owner: `apps/saltz/burner`.
- Purpose: draw one complete mountain waveform in the global public
  cycle-burn-rate series by burning a fixed, bounded schedule of one canister's
  own cycles. The rejected installation remains terminal; the replacement
  global controller is local and inert until its remaining gates close.
- Fleet relationship: none. The canister is not a Root, Coordinator, Store or
  managed Component and has no Canic runtime dependency.
- Effect authority: install and status inspection are permitted. Deployment is
  inert. A cycles mint, canister top-up, `Arm` and `AuthorizeWaveform` are
  separate external effects and require an exact recorded envelope before
  execution.
- Qualification: direct mainnet burn visibility and repeated-input accumulation
  passed on 2026-08-16. The complete decay kernel and global-background model
  remain unqualified. The Subnet-scoped schedule is rejected for artistic
  execution and cannot be re-armed through its terminal installation.

This document remains under `docs/design/ideas/` because the experiment is not
a Canic product release line.

## Decision

Build one deliberately narrow standalone canister that can do exactly this:

```text
controller arms one embedded plan at one future chart boundary
    -> one retained ic-timers registration executes absolute deadlines
    -> 35 pre-roll messages burn without waveform authority
    -> one separately funded command authorizes the immutable waveform remainder
    -> each authorized message burns its one precompiled amount and commits one receipt
    -> completion, abort or the first fault stops permanently
```

The schedule is compiled from the checked-in numeric waveform at build time.
No endpoint accepts a burn amount, cadence, waveform, forecast or financial
ceiling. Installing the Wasm only creates a `Prepared` canister and cannot
start a timer or intentionally burn waveform cycles.

The application-owned surface is exactly two methods:

```text
burner_command(BurnerCommand) -> Result<BurnerSummary, BurnerError>
burner_status(BurnerStatusRequest) -> Result<BurnerStatusResponse, BurnerError>
```

Capabilities add command/status variants, not methods. There is no Canic-owned
method in this canister.

## Evidence That Changed The Plan

The [mainnet calibration report](../../../audits/working/saltz-b0b-calibration/mainnet-calibration.md)
freezes the exact B0 evidence.

The B0b canister burned one exact `4 Tcycle` pulse on canister
`w47na-gaaaa-aaaad-qmclq-cai`, placed on public 13-node
`verified_application` Subnet
`5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae`.
The pulse was clearly visible in the Subnet series, but its attributable peak
was only approximately `0.883 Bcycles/second` and remained elevated through
the observed tail. A naive independent 100-second-bucket model was rejected.

The later B0c plateau executed eighteen exact host-driven `200 Bcycle` steps.
The public Subnet series moved from approximately `0.312` to
`1.303 Bcycles/second`, an observed increase of approximately
`0.990 Bcycles/second`. This proved that repeated bounded input accumulates
into a clean signal. It did not measure the complete decay kernel.

The rejected Subnet controller used a provisional rectangular `4,531`-second
response inferred from the B0b scale. The B0d constant-input rise fits a
`4,200.842`-second gain denominator with `R² = 0.999475`. Its observed trailing
edge removes the first pulse between its 3,600- and 3,700-second 100-second
samples, showing that gain normalization, visible support and observation
phase are three different facts. The public API's caller-selected `step` still means returned
sample spacing; it does not establish independent burn attribution.

## Numeric Waveform Authority

The checked-in authority is:

```text
docs/design/ideas/saltz/saltz_24h_waveform_floor_100B_860.csv
SHA-256: 11fd75eb8fd0fed4f075d324051cc880db50619837bfe6c889fe9d654647d911
```

It contains 860 contiguous rational buckets covering exactly 24 hours and
normalized heights spanning `0..=1,000,000` parts per million. The repository
contains no source photograph, raster derivative or image-decoding pipeline.
The numeric geometry is sufficient to build the experiment but does not by
itself establish external source provenance or permission.

The executable compiler resamples the normalized geometry at control-step
midpoints using integer linear interpolation. It then applies the fixed target
mapping and causal inverse below. Floating-point simulator output is analysis
only and can never become a burn amount.

## Rejected Subnet Executable Plan

The terminal immutable plan was Subnet-scoped:

| Field | Exact value |
| --- | ---: |
| Assumed unrelated Subnet background | `625,000,000 cycles/second` |
| Controlled-signal scale | `10` |
| Visible target floor | `4,375,000,000 cycles/second` |
| Visible target relief | `15,000,000,000 cycles/second` |
| Control cadence | `100 seconds` |
| Chart cadence/alignment | `600 seconds` |
| Provisional kernel width | `4,531 seconds` |
| Per-step rate ceiling | `200,000,000,000 cycles/second` |
| Pre-roll steps | `45` |
| Waveform steps | `864` |
| Total steps | `909` |
| Pre-roll burn | `57,669,300,000,000 cycles` |
| Waveform burn | `900,383,644,723,000 cycles` |
| Total intentional burn | `958,052,944,723,000 cycles` |
| Immutable total ceiling | `1,300,000,000,000,000 cycles` |
| Initial funding steps/window | `42` / `70 minutes` |
| Initial intentional allocation | `53,824,680,000,000 cycles` |
| Plan digest | `e5977055cf691d29353c6649bd464a821475efd66432ff56ea93d76de419ff8d` |

The integer compiler:

1. verifies the CSV digest, row order and exact 24-hour duration;
2. resamples to 864 base midpoint targets without floating point;
3. subtracts the fixed background with a zero floor and scales every resulting
   controlled amount by exactly 10;
4. seeds 45 prior pre-roll rates for the 46-tap kernel including the current
   waveform pulse;
5. solves each next non-negative rate against 45 full 100-second overlaps and
   one final 31-second overlap in the 4,531-second window;
6. rounds a required positive rate upward to whole cycles/second;
7. rejects a per-step rate above the immutable ceiling;
8. integrates each rate into one exact `u128` pulse amount;
9. rejects a total above the immutable ceiling; and
10. digests every input constant, the initial funding step count, retained
    reserve, execution allowance and all 909 amounts.

The exact 10× scaling preserves normalized shape and timing. The existing
floating-point forward model predicts a 144-point correlation of
approximately `0.954`, mean absolute error of approximately
`52.7 Mcycles/second` and maximum error of approximately
`486.0 Mcycles/second` under the same provisional rectangular response. Those
figures are model evidence, not an observed public result.

## Candidate Global Plan (Tail Gate In Progress)

The replacement source hard-cuts the rejected scale layer into one direct
global-homepage contract. It remains inert until the complete B0d trailing
edge accepts or revises the measured transfer contract:

| Field | Exact local candidate |
| --- | ---: |
| Conservative unrelated global background credit | `30,000,000,000 cycles/second` |
| Visible target floor | `100,000,000,000 cycles/second` |
| Visible target relief | `50,000,000,000 cycles/second` |
| Control cadence | `100 seconds` |
| Homepage chart cadence | `600 seconds` |
| Measured gain denominator | `4,201 seconds` |
| Measured visible support | `3,600 seconds` |
| Control-grid phase lead | `100 seconds` |
| Per-step rate ceiling | `500,000,000,000 cycles/second` |
| Peak compiled control rate | `297,654,853,334 cycles/second` |
| Pre-roll steps | `35` |
| Waveform steps | `864` |
| Total steps | `899` |
| Pre-roll burn | `409,320,934,169,000 cycles` |
| Waveform burn | `9,072,189,520,950,000 cycles` |
| Total intentional burn | `9,481,510,455,119,000 cycles` |
| Immutable total ceiling | `10,000,000,000,000,000 cycles` |
| Candidate digest | `dc1cc6ba53470e0f4abf8045224c8a9bb92516b86e458e9238d4428def3e13d9` |

The replacement inverse uses 36 equal 100-second support taps, each normalized
by the measured 4,201-second gain denominator. Thirty-five prior inputs seed
the first waveform solve; the current input is the thirty-sixth tap. A pulse
began appearing approximately 10 seconds after execution and was fully
represented by approximately 60 seconds; the conservative 100-second phase
lead keeps execution on the control lattice and affects neither gain nor support.

There is no `control_signal_scale` field or intermediate base-rate family.
The compiler works directly in global cycles-per-second units. The complete
pre-roll is also the initial funding window, so a future Arm would require
exactly:

```text
409,320,934,169,000  complete 35-pulse pre-roll
  1,000,000,000,000  minimum retained balance
    100,000,000,000  execution allowance
-------------------
410,420,934,169,000  minimum balance at Arm
```

The terminal canister already exceeds that balance, but only a reinstall can
replace its terminal state and old plan. Reinstall, Arm, continuation and any
additional funding remain separately authorized mainnet effects.

## Economic Boundary

The replacement's initial funding window is its complete 35-pulse pre-roll,
covering controlled input from 60 minutes before the first labelled chart
point through 200 seconds before it. Arming requires the exact
`410,420,934,169,000-cycle` balance shown above. The existing terminal asset
exceeds that amount, so qualification requires no new ICP conversion.

`Arm` cannot authorize the 864 drawing pulses. `AuthorizeWaveform` is a second
variant of the same command endpoint and accepts only the exact embedded
digest. It requires the current balance to cover every still-pending pre-roll
pulse, the first drawing pulse and the retained reserve. It grants operator
consent to the immutable schedule; it is not a claim that all 24 drawing hours
are already collateralized. If authorization is absent at the first waveform
deadline, the run fails terminally before burn with
`WaveformNotAuthorized`. Surplus balance alone cannot cross the boundary.

The canister cannot mint, request or move funding. A controller may deposit
cycles through a separate operator action while the run is active. Additional
balance cannot increase the burn because all 899 amounts and the total are
embedded. Without a later top-up, the first pulse whose amount would cross the
retained reserve fails terminally; nothing catches up or resumes.

Arm funding includes both the retained reserve and execution allowance. Every
intentional-burn precondition preserves the retained reserve; the immutable
plan total prevents explicit burn from exceeding its separately authorized
allocation, while ordinary message execution may consume the allowance. A
stricter per-pulse `reserve + allowance` check was rejected by the full local
run because it safely stopped at receipt 898 after ordinary execution consumed
part of the allowance. Every pulse instead requires its exact embedded amount
plus the retained reserve. Targeted PocketIC evidence now funds the complete
pre-roll, proves an unfunded first waveform pulse fails before burn, crosses
that boundary under partial funding, stops before the first unaffordable pulse,
and separately completes all 899 exact receipts under full funding while
retaining the reserve and an unexhausted portion of the allowance.

The mainnet financial authorization must bind discrete ICP e8s and the maximum
cycles they can mint. B0c showed why: an exact `3 Tcycle` request deposited
`3,000,000,008,750` cycles because the conversion operates on discrete ICP
e8s. The written envelope must include that maximum overage rather than only a
nominal cycle target.

## Timing Contract

`Arm` accepts only:

- the exact 32-byte plan digest exposed by `Summary`; and
- a `chart_start_at_ns` aligned to an exact 600-second Unix epoch boundary.

The first pre-roll deadline is exactly 3,600 seconds before the labelled chart
start. The first waveform burn is exactly 100 seconds before chart start so
the complete measured attribution transition precedes the requested label. At arm
time the first deadline must still be at least 60 seconds in the future. Chart
start may be no more than seven days plus pre-roll and phase lead in the
future.

Every successor uses:

```text
schedule_start_at_ns + step_index * 100 seconds
```

It never schedules relative to callback completion, catches up missed work or
combines a missed amount with a later amount. A callback more than 60 seconds
late transitions immediately to terminal `Failed` without burning that or any
later step. The bias is deliberate: preserving the spending ceiling and
stopping a corrupted drawing matters more than finishing.

## Run State

The per-installation state machine is:

```text
Prepared -> Armed -> Running -> Completed
    |          |         |
    +----------+---------+-> Aborted
               +---------+-> Failed
```

- `Prepared`: no waveform timer is armed and no run evidence exists.
- `Armed`: the first pre-roll deadline is registered; no pulse has executed.
- `Running`: at least one exact pulse and receipt committed.
- `Completed`: all 899 amounts committed and their sum equals the plan total.
- `Aborted`: controller cancellation won and no later callback may burn.
- `Failed`: lateness, balance shortfall or an internal invariant stopped the
  registration.

`Abort` is idempotent once terminal. There is no pause, resume, retry,
replacement plan or second run in the same installation. A reinstall creates
a fresh inert installation, but reinstall is a separate controller effect and
never follows from canister code.

`waveform_authorized` is a monotonic per-run fact, not another phase. It starts
false at `Arm`, can become true only through the exact-digest
`AuthorizeWaveform` command after the pre-roll-to-first-waveform minimum is
funded, and can never return to false. Without it, `Running` is limited to
pre-roll receipts. With it, the controller may still abort at any time and a
later balance shortfall stops before burn with `InsufficientBalance`.

## Burn And Receipt Atomicity

Each timer callback is one non-awaiting message:

```text
validate phase/index/deadline/total/balance
    -> cycles_burn(exact embedded amount)
    -> require returned amount == requested amount
    -> append same-message receipt
    -> choose the next absolute deadline or Stop
```

No validation, inter-canister call or `await` occurs after entering the
burn/receipt section. If `cycles_burn` returns a different amount, the callback
commits the requested and actual values in one receipt, transitions to terminal
`Failed(PartialBurn)` and returns an invariant failure with `Stop`. It does not
convert the partial effect into a fresh burn request.

Each receipt contains:

- step index and pre-roll/waveform classification;
- expected and actual execution timestamps;
- requested and actual burn amounts; and
- same-message balances before and after the explicit burn.

Summary reports the immutable plan, current phase, exact balance, step and
receipt counts, total burned and terminal reason. Receipts are returned in
pages of at most 50.

## Candid Surface

```rust
enum BurnerCommand {
    Arm {
        authorization_digest: Vec<u8>,
        chart_start_at_ns: u64,
    },
    AuthorizeWaveform {
        authorization_digest: Vec<u8>,
    },
    Abort,
}

enum BurnerStatusRequest {
    Summary,
    Receipts { start: u32, limit: u16 },
}
```

Errors are composed into three top-level families:

```text
AccessDenied
Conflict { phase }
Rejected { reason }
```

Typed rejection reasons cover authorization, funding, start-window, receipt
page and timer failures. Internal workflow phases, caller kinds, failures and
features never become endpoints.

Both methods are controller-only. Status authorization is checked before any
run evidence is read. There is no anonymous operational surface.

## Timer Ownership

`ic-timers` owns registration, provider wake-ups, absolute deadlines,
cancellation, callback arbitration, counters and inventory. The burner owns
only its fixed amounts, run phase, progress and receipts.

One retained `OnceRegistration` named
`standalone-burner/execution/waveform` is registered synchronously during
`init`. It remains inactive in `Prepared`, is scheduled by `Arm`, returns one
native `TimerRunResult` per callback and stops on every terminal transition.
No application timer facade or shadow scheduling state exists.

## Upgrade And Persistence Boundary

The experiment is reinstall-only. Both `pre_upgrade` and `post_upgrade` trap,
so a normal upgrade cannot interrupt, reconstruct or mutate an armed run.
There is no stable-state schema, migration, mixed-version operation or recovery
promise.

This is intentional. A 24-hour drawing is more sensitive to timing than it is
valuable to upgrade continuity. Same-installation timer interruption is
handled by the timer provider; cross-Wasm continuation is forbidden.

## Validation

Required targeted evidence is:

1. waveform CSV digest, exact duration and point count;
2. deterministic integer schedule, digest and total;
3. no amount exceeds the rate ceiling and the total stays below the immutable
   maximum;
4. strict two-method extracted Candid;
5. controller-only status and commands;
6. underfunded `Arm` rejects without changing `Prepared`;
7. exhaustive pure checks cover all 899 amounts, exact summed burn and every
   derived deadline;
8. PocketIC commits exact consecutive timer pulses, crosses the
   pre-roll-to-waveform boundary under partial funding, stops before an
   unaffordable pulse, completes all 899 pulses under full synthetic funding
   and proves Abort prevents every later pulse;
9. release Wasm build and structural validation; and
10. inert mainnet install reports `Prepared`, zero receipts and zero
    intentional burn before funding.

No anti-resurrection test or compatibility surface is required. Current
behavior tests cover only the maintained standalone protocol.

## Mainnet Operational Boundary

The selected existing mainnet asset is:

| Field | Value |
| --- | --- |
| Identity principal | `5czt6-ctczu-3d74z-xwdcb-lq3vj-sbsei-g2tyx-x5jlz-lmbkz-2xosq-rqe` |
| Canister | `w47na-gaaaa-aaaad-qmclq-cai` |
| Subnet | `5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae` |
| Network | IC mainnet |
| Deployment environment | `waveform-burner-ic` |
| Installed Wasm SHA-256/module hash | `2388f3f4e38274999682da7a3525d6fbc41724c073c61d16b7c9b253ebecbfc9` |
| Phase after install | `Prepared` |
| Receipts/intentional burn after install | `0` / `0 cycles` |
| Controller status balance before 10× top-up | `5,980,046,999,847 cycles` |

The staged-trial Wasm was reinstalled on that asset on 2026-08-16. The release
artifact hash equals the controller-reported module hash. Controller-only
`Summary` returned the exact compiled plan, `Prepared`, no schedule, zero
receipts and zero intentional burn before the external trial effects.

The funding-and-arm record freezes:

- exact installed module hash and plan digest;
- balance immediately before mint/top-up;
- maximum ICP e8s spent and maximum cycles produced;
- exact top-up amount and post-top-up balance;
- aligned chart start and derived pre-roll start;
- public Subnet observation URLs and poll cadence;
- controller identity and canister/Subnet binding; and
- manual abort procedure.

The identity held `225.53140595 ICP` before conversion. The operator requested
exactly `4.8 Tcycles`; discrete conversion deposited
`4,800,000,007,071 cycles` and left `222.59132922 ICP`, an exact spend of
`2.94007673 ICP`. An exact `5.3 Tcycle` top-up produced a controller-observed
canister balance of `6,583,111,657,251 cycles` before Arm.

That first installation's callback executed at
`1,786,916,700,387,529,216 ns`, only `387,529,216 ns` after its exact deadline.
Its same-message receipt records requested and burned amounts both equal to
`128,154,000,000 cycles`; controller status then reported `Running`, one
receipt and no terminal reason. This proves live timer/burn/receipt execution,
not yet the public observation response.

Review then found that this first installation protected the retained reserve
but did not make the separate execution allowance ineligible for intentional
burn. The controller aborted after exactly two receipts and
`256,308,000,000 cycles`, before relying on the staged-window claim. Corrected
Wasm `280b3f8fa0ccb0d1c98fbb3ac37b7e3bc926e325ac9618ac51a8269e47565c77`
binds both balances into its digest and burn precondition. A corrective exact
`300 Bcycle` mint cost `0.18431215 ICP`, and an exact `300 Bcycle` top-up
produced `6,623,823,016,957 cycles` before the second Arm. Digest
`0c00db0e4dd0174d1bb8f2a2ae80129c17ed03d73ba1bda5913868dfc6f2435f`
scheduled the restarted first deadline at `2026-08-16T23:55:00+02:00`, the
one-hour evidence decision at `00:55`, continuation deadline before `01:05`,
and chart start at `01:10`.

The controller aborted that corrected 1× attempt after five exact receipts and
`640,770,000,000 cycles`: the early public series moved in the expected
direction, but remained too close to ordinary noise for a confident full-run
decision. The maintainer then authorized an exact 10× controlled-signal test.
Wasm `2388f3f4e38274999682da7a3525d6fbc41724c073c61d16b7c9b253ebecbfc9`
and digest
`e5977055cf691d29353c6649bd464a821475efd66432ff56ea93d76de419ff8d`
bind that scale, `53,824,680,000,000` initial intentional cycles, the reserve
and allowance. Converting exactly `49.1 Tcycles` cost `30.15060661 ICP`; the
exact top-up produced `55,080,036,485,836 cycles` before Arm. The first 10×
deadline is `2026-08-17T00:15:00+02:00`, the one-hour evidence decision is
`01:15`, and chart start is `01:30`.

The staged continuation gate passed early after 16 exact timer receipts and
`20,504,640,000,000 cycles` of intentional burn. Controller status reported
`Running`, no terminal reason and `34,575,030,604,169 cycles`. At the public
Dashboard's exact one-day `600`-second cadence, the owning Subnet reported:

| Timestamp | Local time | Cycle burn rate |
| ---: | --- | ---: |
| `1786917600` | `00:00` | `0.389455606 Bcycles/second` |
| `1786918200` | `00:10` | `0.507886778 Bcycles/second` |
| `1786918800` | `00:20` | `1.496329056 Bcycles/second` |
| `1786919400` | `00:30` | `3.377697751 Bcycles/second` |
| `1786920000` | `00:40` | `5.157094502 Bcycles/second` |

The three controlled intervals rose by approximately `0.988`, `1.881` and
`1.779 Bcycles/second`. The last two agree with the predicted approximately
`1.7..=1.9 Bcycles/second` rise for six 10× pulses per observation interval.
This proves a receipt-synchronous, proportional public response across
multiple Dashboard-cadence bins; it still does not prove the complete
24-hour artistic fidelity or the provisional kernel.

The identity held `5,192.25651045 ICP` at continuation. Minting the exact
requested shortfall `904,073,274,118,831 cycles` deposited
`904,073,274,124,352 cycles` after discrete conversion and cost exactly
`556.42132824 ICP`. The canister received exactly
`904,073,274,118,831 cycles`; no schedule, amount or ceiling changed. After
receipt 17, controller status reported `937,366,738,334,627 cycles` against
`936,266,764,723,000` remaining intentional cycles, leaving
`1,099,973,611,627 cycles` for the immutable reserve and execution allowance.
The identity retained `4,635.83518221 ICP`, and the cycles ledger retained
`99,400,054,212 cycles`.

At `2026-08-17T00:57+02:00`, operator observation clarified that the intended
canvas was the global Dashboard homepage. Its displayed `0.0459 Tcycles/second`
was approximately `45.9 Bcycles/second`, not the owning Subnet's approximately
`8 Bcycles/second` controlled reading. The global series remained dominated by
roughly `30..=60 Bcycles/second` unrelated traffic, so the Subnet-qualified
controller no longer met the stated objective. Under the maintainer's standing
stop-on-loss-of-confidence rule, `Abort` committed before chart start. Terminal
status reported 26 exact receipts, `33,320,040,000,000 cycles` burned,
`925,797,679,907,302 cycles` remaining and `ControllerAbort`. No waveform step
executed. A future global controller requires separate evidence and a new inert
reinstall; this terminal run cannot resume.

The operator may call `Abort` at any point after arming. Abort prevents future
pulses but cannot restore cycles already burned. Pre-roll alone costs
`57,669,300,000,000` cycles, so even a rejected visual attempt has a
real non-refundable cost.

## Non-Goals

This experiment does not:

- add deliberate burn to Canic;
- expose a generic burn amount;
- deploy or manage other canisters;
- use a Fleet Root, Coordinator or Wasm Store;
- fetch the Dashboard from the canister;
- forecast or compensate for live background changes;
- catch up missed work;
- pause or resume;
- support upgrades or state migration;
- guarantee recognizability against arbitrary IC traffic;
- claim that the provisional response kernel is a platform contract; or
- make the public preview part of the destructive canister.

## Design Principle

The canister can spend only the exact schedule visible before arming. It may
stop early for many reasons, but no retry, lateness, forecast error, extra
funding or caller input can make it burn more to compensate.
