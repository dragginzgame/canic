# 0.104 B4 Domain-Native Fixed-Owner Custody

Date: 2026-08-18

## Result

B4 was accepted by maintainer continuation on 2026-08-18. Root issuer renewal,
automatic cycle top-up and placement-receipt acknowledgement now declare,
schedule, reconcile and cancel their own native retained `Once` registrations.
The central timer workflow no longer selects or dispatches those jobs.

Each owner declares its registration lazily from current domain demand. A
profile without `AutomaticTopup` reserves no cycle-top-up row, and an empty
placement-receipt index reserves no acknowledgement row. Auth renewal derives
its deadline from enabled issuer configuration and current proof state. There
is no placeholder central registration for an absent capability or empty
domain.

The watchdog checks the cheap expired-attempt fence before reading domain
state, then asks the exact owner to confirm current demand and claim one
takeover. If demand disappeared, the owner abandons only the expired attempt.
Ordinary callbacks claim and finish their own attempt directly, including
cycle top-up's exact replay generation. Snapshot suspension, resumability and
active-attempt checks include the three domain-owned native claims without
copying provider state back into the central workflow.

The remaining central fixed-claim map contains only intent cleanup, log
retention, root Canister-pool maintenance, lifecycle deferrals and the one
recovery watchdog. Pool, lifecycle and snapshot-registry dissolution remains
B5 work.

## Provider Inventory And Historical Cost Observation

The representative runtime probe's ordinary inventory falls from seven B3
rows to five B4 rows:

```text
canic/async_job_recovery/watchdog
canic/intent_cleanup/run
canic/log_retention/run
companion-framework/inventory/visible
runtime-probe/application/timer-interval
```

The profile has neither `AutomaticTopup` nor pending placement receipts, so
`canic/cycles/topup` and `canic/placement/receipt_acknowledgement` are no
longer declared. The existing 24-hour authority journey continues to assert
zero idle callbacks for capability-pruned cycle top-up, intent cleanup and log
retention.

The seven-to-five table is a historical B3/B4 observation, not a universal
final inventory claim. The exact phase source states were not retained. The
final role-specific provider matrix in the B8 closeout evidence supersedes it.
The following instruction table is likewise historical and is not closeout
evidence:

| Observation | B3 | B4 | B4 minus B3 |
| --- | ---: | ---: | ---: |
| Scheduler instruction samples | 0 | 0 | 0 |
| Work instruction samples | 2 | 2 | 0 |
| Latest work instructions | 25,017 | 23,709 | -1,308 (-5.2284%) |
| Maximum work instructions | 25,162 | 23,898 | -1,264 (-5.0234%) |
| Total work instructions | 50,179 | 47,607 | -2,572 (-5.1257%) |

The watchdog values and signed B3 differences are historical for the same
reason. The closeout claim is limited to removal of the two unused provider
declarations; no causal B3-to-B4 performance improvement is claimed.

## Historical Fast-Profile Wasm Observation

This development-phase table is retained for provenance only. The B3 and B4
source states were not preserved, so neither the phase deltas nor the alleged
0.103 comparison are closeout or release acceptance evidence.

| Role | B4 raw bytes | Delta from B3 | Delta from 0.103.0 | Raw SHA-256 | B4 gzip bytes | Delta from B3 | Delta from 0.103.0 | Gzip SHA-256 |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| managed Component | 3,570,440 | +4,256 (+0.1193%) | +185,579 (+5.4826%) | `cc97b4322f9ba07f028c8ccb8ffd68abfb85d8cec51a3434c0c9efa8d1b7b277` | 944,358 | -210 (-0.0222%) | +61,091 (+6.9165%) | `0f867bf091c66983b68b714f813ae395b6deb162ee4779408b768e7218ba6610` |
| Fleet Subnet Root | 8,432,611 | +13,774 (+0.1636%) | +316,925 (+3.9051%) | `6cfe8e5ea2f05075785be933de05ca2bd0c61034f8ba3ed0f44309c8a0daeb3c` | 2,186,154 | +5,667 (+0.2599%) | +99,579 (+4.7724%) | `268ed435be9972834812b4af3588c25a2f43c4600c9aa00b48bdfeeb633c4178` |
| Fleet Coordinator | 4,063,721 | 0 | -7,414 (-0.1821%) | `00b409fae1302aea95d93a1d33f74f4bc5205775df60927a9a4c1162f631da93` | 1,010,918 | 0 | -2,990 (-0.2949%) | `9d0b5c83751628bc616aa3e2cde9a496b57061844b6da5dd6f2dec234e8138f6` |
| Wasm Store | 3,349,689 | 0 | +19,499 (+0.5855%) | `3a824d06fab11f15ffcaf19b12abe51800fe97b7e415d7f4be3058869f1a7071` | 885,178 | 0 | +5,545 (+0.6304%) | `591b5db2e999639749b6bcfd30eea09b0c1149d2d1bfbfe0d31c540fdbdf9d35` |
| **Four-role total** | **19,416,461** | **+18,030 (+0.0929%)** | **+514,589 (+2.7224%)** | — | **5,026,608** | **+5,457 (+0.1087%)** | **+163,225 (+3.3562%)** | — |

No B3-to-B4 or B4-to-0.103 size conclusion is used at closeout. The immutable
release-tree footprints in the working README supersede this table.

The current `runtime_probe` fixture is 3,659,738 raw bytes with SHA-256
`21be5d7bf67dbab0d5dbc21a2413b2a52592d1e4a2ddd2ae60837a698a7f87db`.
Deterministic gzip is 909,009 bytes with SHA-256
`3f71e0a8f1b6cc7a19877c3db353dc5fc0892b3420c30166a56a8a5e9883775f`.
That is 4,325 raw and 8,842 gzip bytes larger than B3, while remaining 13,623
raw and 1,606 gzip bytes smaller than the immediate pre-B2 fixture.

## Focused Validation

Passed on 2026-08-18:

- locked compilation for `canic-core`, `canic-control-plane`, `canic`,
  `runtime_probe` and `canic-tests`;
- locked warning-denied Clippy for the four production/fixture packages, all
  targets, plus the `timer_authority` integration target;
- all 1,119 `canic-core` library tests: 1,118 passed and one ignored;
- both maintained timer source-inventory tests;
- all four `timer_authority` PocketIC journeys on the B4 source, including the
  five-row lazy inventory and exact watchdog takeover;
- the real restored-Root snapshot/resume journey, proving that moved owner
  claims remain inside Canic's authority fence; and
- all four canonical fast product-role builds.

An intermediate rerun exposed `ic-testkit 0.8.6` pre-creating PocketIC 15's
`--port-file`, which made the pinned server exit successfully without binding.
`ic-testkit 0.8.7` fixes that ownership boundary by allocating a private
startup directory while leaving the server-owned port path absent. Canic now
locks 0.8.7, and all four journeys pass again in 6.89 seconds through one
runner-owned shared server with all three Wasm artifacts reused. The exact
cleanup helper also recognizes and terminates that shared server before its
private scratch is removed.

The complete workspace, release matrix and broad PocketIC suites were not run
for this historical phase. Later batches completed B1-B8 and `v0.104.0` was
published; current closeout authority is recorded in the design status.
