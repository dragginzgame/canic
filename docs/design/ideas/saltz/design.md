# Canic Design Idea: Standalone Cycle-Burn Waveform

Date: 2026-08-16

## Status

- Status: unnumbered experiment with a separately authorized standalone
  implementation and inert mainnet deployment.
- Repository: Canic.
- Runtime owner: `apps/saltz/burner`.
- Purpose: draw one complete mountain waveform in the public cycle-burn-rate
  series for one exact IC Subnet by burning a fixed, bounded schedule of the
  canister's own cycles.
- Fleet relationship: none. The canister is not a Root, Coordinator, Store or
  managed Component and has no Canic runtime dependency.
- Effect authority: install and status inspection are permitted. Deployment is
  inert. A cycles mint, canister top-up and `Arm` are separate external effects
  and require an exact recorded envelope before execution.
- Qualification: direct mainnet burn visibility and repeated-input accumulation
  passed on 2026-08-16. The complete decay kernel remains unqualified, so the
  compiled control schedule is a dated bounded proposal rather than a promise
  of Dashboard fidelity.

This document remains under `docs/design/ideas/` because the experiment is not
a Canic product release line.

## Decision

Build one deliberately narrow standalone canister that can do exactly this:

```text
controller arms one embedded plan at one future chart boundary
    -> one retained ic-timers registration executes absolute deadlines
    -> each message burns its one precompiled amount and commits one receipt
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

The current controller therefore uses a provisional rectangular
`4,531`-second response inferred from the B0b scale. That is an explicit model
assumption, not a platform guarantee. The public API's caller-selected `step`
still means returned sample spacing; it does not establish independent burn
attribution.

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

## Frozen Executable Plan

The current immutable plan is Subnet-scoped:

| Field | Exact value |
| --- | ---: |
| Assumed unrelated Subnet background | `625,000,000 cycles/second` |
| Visible target floor | `1,000,000,000 cycles/second` |
| Visible target relief | `1,500,000,000 cycles/second` |
| Control cadence | `100 seconds` |
| Chart cadence/alignment | `600 seconds` |
| Provisional kernel width | `4,531 seconds` |
| Per-step rate ceiling | `20,000,000,000 cycles/second` |
| Pre-roll steps | `45` |
| Waveform steps | `864` |
| Total steps | `909` |
| Pre-roll burn | `5,766,930,000,000 cycles` |
| Waveform burn | `90,038,364,472,300 cycles` |
| Total intentional burn | `95,805,294,472,300 cycles` |
| Immutable total ceiling | `130,000,000,000,000 cycles` |
| Initial funding steps/window | `42` / `70 minutes` |
| Initial intentional allocation | `5,382,468,000,000 cycles` |
| Plan digest | `491cd73eb597ca4586fd33516d0390160df0b51111fb388d96843b21552a86c9` |

The integer compiler:

1. verifies the CSV digest, row order and exact 24-hour duration;
2. resamples to 864 midpoint targets without floating point;
3. subtracts the fixed background with a zero floor;
4. seeds 45 prior pre-roll rates for the 46-tap kernel including the current
   waveform pulse;
5. solves each next non-negative rate against 45 full 100-second overlaps and
   one final 31-second overlap in the 4,531-second window;
6. rounds a required positive rate upward to whole cycles/second;
7. rejects a per-step rate above the immutable ceiling;
8. integrates each rate into one exact `u128` pulse amount;
9. rejects a total above the immutable ceiling; and
10. digests every input constant, the initial funding step count and all 909
    amounts.

The existing floating-point forward model predicts a 144-point correlation of
approximately `0.954`, mean absolute error of approximately
`52.7 Mcycles/second` and maximum error of approximately
`486.0 Mcycles/second` under the same provisional rectangular response. Those
figures are model evidence, not an observed public result.

## Economic Boundary

The authorized staged trial funds 42 exact pulses, covering 70 minutes. Arming
requires the canister balance to cover all of:

```text
 5,382,468,000,000  first 42 exact pulses
 1,000,000,000,000  minimum retained balance
   100,000,000,000  execution allowance
-------------------
 6,482,468,000,000  minimum balance at Arm
```

The canister cannot mint, request or move funding. A controller may deposit
cycles through a separate operator action while the run is active. Additional
balance cannot increase the burn because all 909 amounts and the total are
embedded. Without a later top-up, the first pulse whose amount would cross the
retained reserve fails terminally; nothing catches up or resumes.

The mainnet financial authorization must bind discrete ICP e8s and the maximum
cycles they can mint. B0c showed why: an exact `3 Tcycle` request deposited
`3,000,000,008,750` cycles because the conversion operates on discrete ICP
e8s. The written envelope must include that maximum overage rather than only a
nominal cycle target.

## Timing Contract

`Arm` accepts only:

- the exact 32-byte plan digest exposed by `Summary`; and
- a `chart_start_at_ns` aligned to an exact 600-second Unix epoch boundary.

The first pre-roll deadline is exactly 4,500 seconds before chart start. At
arm time that first deadline must still be at least 60 seconds in the future.
Chart start may be no more than seven days plus pre-roll in the future.

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
- `Completed`: all 909 amounts committed and their sum equals the plan total.
- `Aborted`: controller cancellation won and no later callback may burn.
- `Failed`: lateness, balance shortfall or an internal invariant stopped the
  registration.

`Abort` is idempotent once terminal. There is no pause, resume, retry,
replacement plan or second run in the same installation. A reinstall creates
a fresh inert installation, but reinstall is a separate controller effect and
never follows from canister code.

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
7. exhaustive pure checks cover all 909 amounts, exact summed burn and every
   derived deadline;
8. PocketIC commits exact consecutive timer pulses and abort prevents every
   later pulse;
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
| Installed Wasm SHA-256/module hash | `728edf4a7d652cc1ffa79e7dda5e96e4a91e42c67eaabb9cc7e2e240f325294b` |
| Phase after install | `Prepared` |
| Receipts/intentional burn after install | `0` / `0 cycles` |
| Controller status balance before trial top-up | `1,283,120,450,111 cycles` |

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
canister balance of `6,583,111,657,251 cycles` before Arm. The accepted Arm
bound digest
`491cd73eb597ca4586fd33516d0390160df0b51111fb388d96843b21552a86c9`,
scheduled the first pre-roll deadline at `2026-08-16T23:45:00+02:00`, the
one-hour evidence decision at `00:45`, the continuation deadline before
`00:55`, and chart start at `01:00`.

The operator may call `Abort` at any point after arming. Abort prevents future
pulses but cannot restore cycles already burned. Pre-roll alone costs
`5,766,930,000,000` cycles, so even a quickly rejected visual attempt has a
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
