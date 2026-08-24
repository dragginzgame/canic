# B0b Pulse And B0c Plateau Mainnet Cycle-Burn Calibration

Date: 2026-08-16

## Disposition

The bounded calibrations proved both that an exact direct cycle burn becomes
visible in the public Cycle Burn Rate metric for the owning Subnet and that
repeated bounded burns accumulate into a clean, controllable signal. They also
rejected the waveform design's original assumption that a caller-selected
`step=100` response represents independent approximately 100-second burn
buckets.

The `4 Tcycle` pulse produced an attributable peak increase of only
approximately `0.883 Bcycles/second` in the Subnet series and remained spread
across the observed tail. A naive 100-second attribution would have produced
`40 Bcycles/second`. The observed peak was therefore approximately `45.3`
times flatter than that model.

Consequently:

- direct-burn visibility passes;
- `SinglePulsePerBucket` at the provisional 860-point cadence is rejected;
- the host-driven `2 Bcycles/second` plateau input passes and produced an
  approximately `0.990 Bcycles/second` observed rise during the run;
- the metric's complete smoothing/decay kernel remains unqualified;
- the existing 24-hour cost and executor model is not executable authority;
- B1 Burner implementation and B4 artistic execution remain held; and
- no additional burn, mint, funding or retry is authorized by this evidence.

## Follow-On Plateau Authorization

On 2026-08-16 the maintainer explicitly requested an immediately stoppable
live test after reviewing the pulse result and smoothing-aware remediation.
That authority is narrowed to:

| Bound | Authorized value |
| --- | --- |
| Network and placement | The same IC-mainnet canister and frozen Subnet |
| Identity | The same exact controller principal |
| New cycles-ledger mint | At most `3 Tcycles` |
| Existing-canister top-up | At most `4 Tcycles` |
| Step | Exactly `200 Bcycles` |
| Step spacing | Nominally `100 seconds`, host driven |
| Step count | At most `18` |
| Intentional burn | At most `3.6 Tcycles` |
| Retained canister balance | At least `1 Tcycle` |
| Execution allowance | `100 Bcycles` |
| Autonomous work | None: no canister timer, retry or catch-up |
| Stop | Cease host calls and submit terminal `Abort` |

The follow-on tests whether a bounded sequence accumulates into the predicted
Subnet plateau. It does not execute any waveform point, authorize the
24-hour run or promote the Burner. Reinstallation of the disposable probe is
permitted because the completed pulse receipt is frozen in this report.

## Follow-On Plateau Result

The B0c run completed on 2026-08-16. The canister accepted exactly eighteen
sequential host calls, each requested and returned exactly `200 Bcycles`, for
an exact intentional total of `3.6 Tcycles`. It then entered terminal
`Completed` state with `next_step_index=18`; it has no timer, retry, catch-up
or further step authority.

| Field | Observed value |
| --- | --- |
| Planned start | `2026-08-16T19:53:06Z` |
| First execution | `2026-08-16T19:53:13.371637470Z` |
| Final execution | `2026-08-16T20:21:26.772760420Z` |
| Step schedule | `18 × 200 Bcycles`, every `100 seconds` |
| Intentional burn | Exactly `3_600_000_000_000 cycles` |
| First-step lateness | `7.371637470 seconds` |
| Later-step lateness | `0.414918186..=1.525997040 seconds` |
| Final same-message balance | `1_256_760_945_794 cycles` |
| Final replica balance | `1_296_755_485_373 cycles` |
| Terminal phase | `Completed` |
| Installed Wasm hash | `7075d86b4f9093cfe03d02d29f6bd8ef3389332729e7af7b4e56ba1be233f69b` |

The first attempted host wrapper failed before submitting step zero because it
could not open the local CLI lock file. It caused no canister call and no burn.
Step zero was then submitted directly within its 60-second lateness bound;
steps 1 through 17 were submitted by the corrected bounded host process. No
call was retried and every committed receipt has an exact
`balance_before_cycles - balance_after_burn_cycles = 200_000_000_000` burn.

The cycles-ledger command requested exactly `3 Tcycles`, but ICP-e8s
conversion deposited `3_000_000_008_750 cycles`, exceeding the written mint
cap by `8_750 cycles`. This is a procedural authorization variance even though
it is economically negligible and did not enlarge the exact top-up or burn
caps. Future plans must bind the discrete ICP-e8s input and its resulting
maximum deposit rather than expressing only a requested cycle amount.

The exact financial observations were:

| Field | Observed value |
| --- | ---: |
| ICP before mint | `83.54720719 ICP` |
| ICP after mint | `81.73435794 ICP` |
| Exact ICP spent | `1.81284925 ICP` |
| Cycles deposited by mint | `3_000_000_008_750` |
| Existing-canister top-up | `3_400_000_000_000` |
| Cycles-ledger balance after top-up and fee | `599_800_013_670` |

The current-time-bounded Subnet series moved from
`0.312413 Bcycles/second` at timestamp `1786909944`, 42 seconds before the
planned start, to `1.302812 Bcycles/second` at timestamp `1786911744`, about
57 seconds after the final execution. That is an observed rise of
approximately `0.990399 Bcycles/second`. The intermediate complete samples
formed a predominantly monotonic ramp rather than an isolated spike:

```text
0.312, 0.387, 0.405, 0.475, 0.572, 0.594, 0.675, 0.742, 0.782, 0.827,
0.893, 0.940, 0.983, 1.113, 1.109, 1.176, 1.219, 1.251, 1.303 Bcycles/s
```

One API poll requested an end timestamp later than wall-clock time and was
excluded completely. All result samples above came from polls whose end was no
later than their observation time. The plateau proves accumulated signal and
rough scale, not the complete kernel: post-stop decay and unrelated Subnet
background remain to be observed read-only.

## B0d 10× Staged-Executor Continuation Result

Two later 1× executor attempts proved timer/receipt mechanics but were
aborted before full funding because their public rise remained too close to
ordinary Subnet noise. The maintainer then authorized an exact 10× controlled
signal with a 42-pulse initial funding window. Release Wasm
`2388f3f4e38274999682da7a3525d6fbc41724c073c61d16b7c9b253ebecbfc9`
and authorization digest
`e5977055cf691d29353c6649bd464a821475efd66432ff56ea93d76de419ff8d`
freeze `909` amounts, `958,052,944,723,000` maximum intentional cycles and
the protected `1.1 Tcycle` reserve-plus-execution balance.

At the continuation checkpoint, controller status reported exactly 16
receipts, `20,504,640,000,000 cycles` burned, `Running`, no terminal reason and
`34,575,030,604,169 cycles` remaining. The official owning-Subnet API at the
Dashboard's exact one-day `600`-second cadence reported:

```text
timestamp    Bcycles/second
1786917600   0.389455606
1786918200   0.507886778
1786918800   1.496329056
1786919400   3.377697751
1786920000   5.157094502
```

The three controlled interval increases were approximately `0.988`, `1.881`
and `1.779 Bcycles/second`. The final two intervals each contain six exact 10×
pulses and agree with the predicted approximately `1.7..=1.9 Bcycles/second`
rise. This passes the staged proportional-response gate across multiple
Dashboard-cadence bins. It proves causal visibility and scale, not the full
24-hour trace or complete response kernel.

The identity then held `5,192.25651045 ICP`. The exact requested continuation
mint of `904,073,274,118,831 cycles` deposited `904,073,274,124,352 cycles`
after discrete conversion and spent exactly `556.42132824 ICP`. The canister
received exactly `904,073,274,118,831 cycles`. After receipt 17 it reported
`937,366,738,334,627 cycles`, while remaining intentional burn was
`936,266,764,723,000 cycles`; the resulting `1,099,973,611,627-cycle` margin
preserves the frozen reserve and execution allowance. The controller identity
retained `4,635.83518221 ICP`, and the cycles ledger retained
`99,400,054,212 cycles`.

At `2026-08-17T00:57+02:00`, the maintainer clarified that the target was the
global Dashboard homepage rather than the qualified owning-Subnet graph. The
homepage value of approximately `0.0459 Tcycles/second` represents
approximately `45.9 Bcycles/second` of global traffic. That scope invalidated
the Subnet-background controller as artistic execution authority. The standing
stop-on-loss-of-confidence rule caused immediate terminal `ControllerAbort`
before chart start. Final status recorded exactly 26 receipts,
`33,320,040,000,000 cycles` intentionally burned and
`925,797,679,907,302 cycles` remaining. No waveform step executed. This result
passes Subnet causal visibility but rejects the current plan for the global
homepage; any replacement requires a separately qualified global model,
reinstall and authorization.

## B0e Global-Homepage Qualification

B0e is read-only until a new exact run plan receives separate authorization.
It treats the Dashboard homepage's global Cycle Burn Rate graph as the canvas
and uses the terminal B0d input as system-identification evidence. No B0e
mint, top-up, install, Arm or intentional burn is authorized by this section.

The frozen seven-day observation requested the official global endpoint with:

```text
start=1786313400 end=1786918200 step=600 format=json
```

Its 1,009 exact ten-minute samples ended before the 10× controlled run. Rates
are expressed below in billions of cycles per second:

| Statistic | Global | Owning Subnet |
| --- | ---: | ---: |
| Minimum | `29.684` | `0.172` |
| 5th percentile | `33.102` | `0.260` |
| Median | `37.576` | `0.307` |
| Mean | `37.983` | `0.350` |
| Standard deviation | `3.540` | `0.148` |
| 95th percentile | `44.506` | `0.583` |
| 99th percentile | `48.371` | `1.083` |
| Maximum | `53.915` | `1.538` |
| Median absolute adjacent change | `0.971` | `0.011` |
| 95th-percentile absolute adjacent change | `4.255` | `0.117` |
| Maximum absolute adjacent change | `12.017` | `1.013` |

This rejects the Subnet-scale `4.375..=19.375 Bcycles/second` target for the
global canvas. The original `100..=150 Bcycles/second` target has a
`50 Bcycles/second` relief, approximately 11.8 times the global series' 95th-
percentile adjacent noise. It is the active bounded analysis candidate.

The 26 B0d pulses form a constant-input rise. Regressing the 27 official
100-second owning-Subnet samples from timestamp `1786918500` through
`1786921100` gives:

| Fit field | Observed value |
| --- | ---: |
| Increase per pulse | `0.305067390 Bcycles/second` |
| Linear fit | `R² = 0.999475` |
| Implied rectangular window | `4,200.842 seconds` |
| Individual increment range | `0.174537..=0.367527 Bcycles/second` |
| Median individual increment | `0.297814 Bcycles/second` |

The candidate window is therefore `4,201 seconds`, pending the complete
post-Abort trailing edge. The input stopped before the first pulse could age
out of that window; the expected decay begins around
`2026-08-17T01:25+02:00` and completes around `02:07`. Until that tail is
frozen, `4,201` is analysis rather than executable authority.

Under a conservative fixed `30 Bcycles/second` unrelated-background credit,
the provisional `4,201`-second controller reports:

| Candidate field | Provisional value |
| --- | ---: |
| Visible target | `100..=150 Bcycles/second` |
| Pre-roll | `420,915,781,440,966 cycles` |
| Waveform | `7,898,579,676,201,503 cycles` |
| Total intentional burn | `8,319,495,457,642,469 cycles` |
| Peak control rate | `412,851,809,635 cycles/second` |
| 144-point model correlation | `0.994841` |
| Model mean absolute error | `0.285 Bcycles/second` |
| Model maximum error | `6.827 Bcycles/second` |

Replaying that candidate over each historical 24-hour global day produced
target correlations of `0.923..=0.965`. Replaying all 144 possible ten-minute
start phases across six complete historical windows gave a best worst-window
correlation of `0.934` for a `02:10 UTC` (`04:10 CEST`) chart start; the
`11:30 UTC` (`13:30 CEST`) phase was nearly equal at `0.933`. This short dated
window does not make either time a permanent start rule, but it demonstrates
that ordinary homepage noise does not erase the proposed geometry.

At the last realized conversion of approximately `1.6248 Tcycles/ICP`, total
candidate burn is approximately `5,120 ICP`. Reusing the canister's roughly
`925.8 Tcycles` leaves approximately `7,394.8 Tcycles`, or `4,551 ICP`, still
to fund after reserve and execution allowance. That estimate is not mint or
top-up authority and remains sensitive to the later certified conversion.

## B0b Exact Authorization

| Bound | Authorized value |
| --- | --- |
| Network | IC mainnet |
| Identity principal | `5czt6-ctczu-3d74z-xwdcb-lq3vj-sbsei-g2tyx-x5jlz-lmbkz-2xosq-rqe` |
| Cycles-ledger mint | At most `7 Tcycles` |
| Canister creation/funding | At most `6 Tcycles` |
| Intentional burn | Exactly one `4 Tcycle` pulse |
| Retained canister balance | At least `1 Tcycle` immediately after the pulse |
| Placement | One public 13-node `verified_application` Subnet, frozen before burn |
| Retry/catch-up | Prohibited |

The identity's private key and recovery material are not evidence and were not
written to the repository or command transcript.

## Final Global Attempt And Preserved Balance

The later separately authorized global attempt used plan digest
`dc1cc6ba53470e0f4abf8045224c8a9bb92516b86e458e9238d4428def3e13d9`.
Pre-roll began at `2026-08-17T02:30:00+02:00` and the intended 24-hour chart
began at `03:30`. The controller executed exactly 535 receipts and burned
`5,859,496,546,135,400 cycles` before the maintainer ordered Abort.

The abort followed an unrelated global-metric spike beginning at
`2026-08-17T16:36:50+02:00`. Public per-Subnet evidence localized the spike to
Subnet
`brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae`,
which rose from approximately `2.4` to `215 Bcycles/second`; the waveform
canister's owning Subnet remained on its expected declining trace. The spike
therefore did not originate from an extra waveform receipt. Its approximately
`180 Bcycles/second` addition permanently distorted the public 24-hour canvas.

The terminal update and an independent controller query both reported
`Aborted` / `ControllerAbort`, 535 receipts and no authority for another pulse.
At final verification canister `w47na-gaaaa-aaaad-qmclq-cai` retained
`2,589,936,553,122,558 cycles`; the controller identity retained
`0.00010000 ICP`. The cycles are not convertible back to ICP. The canister must
not be reinstalled or deleted without an explicit disposition plan for this
approximately `2.590 Pcycle` asset.

## Frozen Placement And Artifact

| Field | Value |
| --- | --- |
| Canister | `w47na-gaaaa-aaaad-qmclq-cai` |
| Subnet | `5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae` |
| Subnet type | `verified_application` |
| Subnet nodes at qualification | `13`, all reported up |
| Controller | The exact authorized identity principal |
| Wasm module hash | `87b4722db2b2c64902d361b2d0f26a31038041dca9b8382731bfec4bc58191f1` |

Public topology provenance is available from the Dashboard API's
[canister record](https://ic-api.internetcomputer.org/api/v3/canisters/w47na-gaaaa-aaaad-qmclq-cai)
and
[Subnet record](https://ic-api.internetcomputer.org/api/v3/subnets/5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae).

## Financial And Receipt Evidence

The certified conversion rate used for the mint was
`xdr_permyriad_per_icp=16528`, or `1.6528 Tcycles/ICP`. The selected ICP
account moved from `87.77799739 ICP` to `83.54720719 ICP`, an exact difference
of `4.23079020 ICP`.

The cycles ledger received `7_000_000_004_920` cycles. After bounded creation
and funding it retained `999_900_004_920` cycles. The installed canister
reported `5_498_895_991_672` cycles before the calibration call.

The controller-only recoverable receipt is:

| Receipt field | Value |
| --- | ---: |
| Executed at | `2026-08-16T18:36:53.539370423Z` |
| Requested | `4_000_000_000_000` cycles |
| Returned by `cycles_burn` | `4_000_000_000_000` cycles |
| Same-message balance before | `5_458_887_732_521` cycles |
| Same-message balance after burn | `1_458_887_732_521` cycles |
| Caller | Exact authorized identity principal |

The difference between replica status and same-message balances is transient
execution accounting, not another intentional burn. A final read-only replica
status at `2026-08-16T18:50Z` reported `1_498_874_636_171` cycles, a running
canister, zero compute and memory allocation, and an estimated idle cost of
`899_671_630 cycles/day`. That one-shot installation had no timer or retry
path; its committed receipt made another `burn_once` reject. The separately
authorized B0c reinstall and its replacement hash are frozen above.

## Public Metric Observation

The observation used the current public
[`/api/v3/metrics/cycle-burn-rate`](https://ic-api.internetcomputer.org/api/v3/metrics/cycle-burn-rate)
contract. The relevant fixed requests were:

```text
pre-global: start=1786903500 end=1786905400 step=100 format=json
pre-subnet: start=1786903500 end=1786905400 step=100 format=json
            subnet=5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae
fine-global: start=1786905350 end=1786906060 step=10 format=json
fine-subnet: start=1786905350 end=1786906060 step=10 format=json
             subnet=5kdm2-62fc6-fwnja-hutkz-ycsnm-4z33i-woh43-4cenu-ev7mi-gii6t-4ae
```

The 20-point, 100-second pre-burn global window was
`34.429..=39.402 Bcycles/second`, with a `38.151 Bcycles/second` mean and a
maximum adjacent change of `2.773 Bcycles/second`. The exact Subnet over the
same window was `0.559..=0.721 Bcycles/second`, with a
`0.625 Bcycles/second` mean and a maximum adjacent change of
`0.101 Bcycles/second`.

In the fine Subnet series, the immediate pre-burn mean was
`0.713187 Bcycles/second`. The first elevated sample was timestamped
`1786905440`, approximately `26.5` seconds after the burn, and the peak sample
was timestamped `1786905470`, approximately `56.5` seconds after the burn. The
peak was `1.595957 Bcycles/second`, an increase of
`0.882770 Bcycles/second` over the immediate baseline and approximately `8.75`
times the pre-burn Subnet window's largest adjacent change.

The API first made the elevated series visible between observation polls at
approximately `47` and `68` seconds after the burn. Sample timestamps and API
publication time are therefore separate facts.

The Subnet value remained near `1.5 Bcycles/second` through the last frozen
sample at `1786906060`, more than ten minutes after execution. If the peak
increment were entirely attributable to a rectangular averaging window, the
equivalent window would be approximately `4_531` seconds, or `75.5` minutes.
That is an orientation calculation, not a measured platform contract: the
complete decay and the actual aggregation kernel were not observed.

The global series was too volatile for exact attribution. Its observed
post-burn peak exceeded its pre-burn mean by approximately
`3.005 Bcycles/second`, but unrelated global traffic was materially larger
than the Subnet signal. Exact-Subnet observation is the useful qualification
lane.

## Design Consequence

The API's caller-selected `step` controls returned sample spacing; it does not
establish independent burn-attribution buckets. A sequence of independently
calculated 100-second pulses would overlap through the platform's smoothed
rate signal and would not reproduce the 860-point preview.

The B0c plateau establishes that bounded repeated inputs overlap into a clean,
controllable rising signal. It does not recover the complete dated
smoothing/decay kernel. Before any full waveform proposal, later work must
observe the post-stop tail read-only and then either:

1. derive and qualify a bounded convolution-aware controller/deconvolution
   model; or
2. abandon this public Dashboard metric as the drawing surface.

Neither alternative nor any additional external effect is authorized by this
report. The exact financial envelope must also bind discrete ICP e8s and their
maximum minted-cycle result.

## Post-Calibration Asset Disposition

After the calibration closed, the maintainer separately authorized a
standalone full-waveform implementation and inert deployment. On 2026-08-16
the existing canister `w47na-gaaaa-aaaad-qmclq-cai` was reinstalled with
release Wasm/module hash
`46cd68c5231843e17c4694e95d32e4f4e1fe2d35ff7386208c8a15c56b89f5ce`.
That reinstall intentionally erased the completed probe runtime state; this
report retains its evidence.

Controller-only status after reinstall reported `Prepared`, no schedule, zero
receipts, zero intentional waveform burn and `1,290,997,070,850 cycles`. The
embedded plan digest is
`20bd60ecfdf993cc0b294128a2916385955114b4743506531164f6e41af0207c`.
No cycles mint, top-up or `Arm` was executed as part of the deployment. The
public Dashboard canister record remained stale at immediate verification and
is not authority for this later installation.

That inert installation was subsequently superseded by separately authorized
10× staged-trial Wasm
`2388f3f4e38274999682da7a3525d6fbc41724c073c61d16b7c9b253ebecbfc9`.
Its funding, authorization digest and timing evidence live in the active
standalone design/status record; they do not retroactively change this bounded
B0 calibration authority.
