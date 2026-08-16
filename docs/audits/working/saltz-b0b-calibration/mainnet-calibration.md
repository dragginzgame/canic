# B0b Mainnet Cycle-Burn Calibration

Date: 2026-08-16

## Disposition

The bounded calibration proved that an exact direct cycle burn becomes visible
in the public Cycle Burn Rate metric for the owning Subnet. It also rejected
the waveform design's original assumption that a caller-selected
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
- the metric's complete smoothing/decay kernel remains unqualified;
- the existing 24-hour cost and executor model is not executable authority;
- B1 Burner implementation and B4 artistic execution remain held; and
- no additional burn, mint, funding or retry is authorized by this evidence.

## Exact Authorization

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
`899_671_630 cycles/day`. The current installation has no timer or retry path;
its committed receipt makes another `burn_once` reject. A controller reinstall
would be a new destructive act and is not authorized by this report.

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

Any later proposal must first recover the complete dated smoothing/decay
kernel through read-only observation, then either:

1. derive and qualify a bounded convolution-aware controller/deconvolution
   model; or
2. abandon this public Dashboard metric as the drawing surface.

Neither alternative is authorized by this report. Increasing the burn before
that choice would spend more ICP without answering the failed cadence
assumption.
