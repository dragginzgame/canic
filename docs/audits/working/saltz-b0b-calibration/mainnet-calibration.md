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
