# 0.104 B3 Domain Async-Job Recovery Hard Cut

Date: 2026-08-18

## Result

B3 was accepted by maintainer continuation on 2026-08-18. Core
stable-memory ID 60 now stores only serial business-attempt fences and the one
exact retry identity that is not already owned by another domain. It stores no
timer command, provider deadline, scheduling-owner flag, provider retry streak
or terminal provider condition.

The reinstall-only hard cut replaces the old key with the one current key
`canic.core.async_job_recovery.v1`; there is no old-key reader, migration,
fallback or parallel schema lane. The allocation ID remains 60.

## Durable Contract

The fixed record has four closed owners:

| Owner | Retained durable authority | Exact retry authority |
| --- | --- | --- |
| Root issuer renewal | Checked attempt generation and optional active lease | The auth domain's delegation batch and proof state; no generated shared operation ID |
| Root Canister-pool maintenance | Checked attempt generation and optional active lease | The pool's creation, reset and handoff journals; no generated shared operation ID |
| Automatic cycle top-up | Checked attempt generation, optional active lease and checked operation generation | One pending cycle-funding operation generation, reused after takeover or retryable failure |
| Placement-receipt acknowledgement | Checked attempt generation and optional active lease | The placement domain's durable receipt operation ID; no generated shared operation ID |

The exact measured worst-case CBOR encoding is 589 bytes. The old generic
record used a 2,048-byte stable bound, so the maintained bound falls by 1,459
bytes, or 71.2402%. This is an encoding ceiling reduction, not a claim that
stable memory pages shrink in an already installed canister.

Claiming an owner either coalesces behind a live lease or advances its checked
attempt generation after expiry. Only cycle top-up receives a generated
operation identity. Its expired takeover and retryable completion preserve the
same exact operation generation; success or invariant failure clears it, and a
later independent operation advances it. An exact attempt token is required to
finish, so a stale continuation cannot clear the takeover lease.

The recovery watchdog now inspects only expired business-attempt leases. It no
longer reads copied provider deadlines, activates a durable scheduling lane or
translates callback directives into generic recovery deadlines. Healthy
schedule and reconciliation requests go directly to the native registration.

## Historical Fast-Profile Wasm Observation

This development-phase table is retained for provenance only. The exact B2
and B3 source states were not preserved, so the phase deltas cannot be rebuilt
independently and are not closeout or release acceptance evidence.

| Role | B3 raw bytes | Delta from B2 | Delta from 0.103.0 | Raw SHA-256 | B3 gzip bytes | Delta from B2 | Delta from 0.103.0 | Gzip SHA-256 |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| managed Component | 3,566,184 | -3,775 (-0.1057%) | +181,323 (+5.3569%) | `ab9e19f3635e351ce534d0b0ca640bb8ed4e63605806f5ea713e092d6bc97a47` | 944,568 | +98 (+0.0104%) | +61,301 (+6.9403%) | `11da3fbbe3f37557391a4563846541e2ee7894c37fdb9834fac35d317abff7b5` |
| Fleet Subnet Root | 8,418,837 | -2,025 (-0.0240%) | +303,151 (+3.7354%) | `99f3ad8e05132b822bb0bd68a87a4bade09f99866c6a3dfa30ef2ff4250bf0e2` | 2,180,487 | -2,439 (-0.1117%) | +93,912 (+4.5008%) | `d135491c593996fb1590c9fa364240a30169e5bd6a8b0707d38b4db500e6a14f` |
| Fleet Coordinator | 4,063,721 | -5,538 (-0.1361%) | -7,414 (-0.1821%) | `00b409fae1302aea95d93a1d33f74f4bc5205775df60927a9a4c1162f631da93` | 1,010,918 | -2,522 (-0.2489%) | -2,990 (-0.2949%) | `9d0b5c83751628bc616aa3e2cde9a496b57061844b6da5dd6f2dec234e8138f6` |
| Wasm Store | 3,349,689 | -6,708 (-0.1999%) | +19,499 (+0.5855%) | `3a824d06fab11f15ffcaf19b12abe51800fe97b7e415d7f4be3058869f1a7071` | 885,178 | -4,308 (-0.4843%) | +5,545 (+0.6304%) | `591b5db2e999639749b6bcfd30eea09b0c1149d2d1bfbfe0d31c540fdbdf9d35` |
| **Four-role total** | **19,398,431** | **-18,046 (-0.0929%)** | **+496,559 (+2.6270%)** | — | **5,021,151** | **-9,171 (-0.1823%)** | **+157,768 (+3.2440%)** | — |

The original B2-to-B3 size interpretation is withdrawn. The immutable
release-tree footprints in the working README supersede this phase table.

The current `runtime_probe` fixture is 3,655,413 raw bytes with SHA-256
`bdcb8d6d61e482655f77de15d8fbea12bdd0d9c00e3c71cf8cb4f1a4629954d0`.
Deterministic gzip is 900,167 bytes with SHA-256
`092f1023d87bcc46d7f40ef13cbe32370e9287b913cd494d752b8ffee262c71a`.
That is 8,166 raw bytes and 9,712 gzip bytes smaller than B2, and 17,948 raw
bytes and 10,448 gzip bytes smaller than the immediate pre-B2 fixture.

## Historical Provider Performance And Recovery

The phase-to-phase cells below are historical observations because the B2 and
B3 source states were not retained. Only final-tree measurements are used for
closeout.

After two direct application interval callbacks, the provider reported:

| Observation | B2 | B3 | B3 minus B2 |
| --- | ---: | ---: | ---: |
| Scheduler instruction samples | 0 | 0 | 0 |
| Work instruction samples | 2 | 2 | 0 |
| Latest work instructions | 25,145 | 25,017 | -128 (-0.5090%) |
| Maximum work instructions | 25,145 | 25,162 | +17 (+0.0676%) |
| Total work instructions | 50,248 | 50,179 | -69 (-0.1373%) |
| Maximum Wasm-memory growth | 0 pages | 0 pages | 0 pages |
| Maximum stable-memory growth | 0 pages | 0 pages | 0 pages |

These sub-percent differences are observational noise-scale results, not a
runtime-performance improvement claim. The B3 watchdog takeover produced one
scheduler sample at 21,503 instructions and one work sample at 51,476
instructions with zero Wasm- or stable-memory page growth. B2 retained no
numeric watchdog baseline, so that is a current cost observation only.

The four-test PocketIC run completed cold in 68.11 seconds and immediately
warm in 6.99 seconds. The dependency graph advanced to `ic-testkit 0.8.5`
during this work, so neither wall-clock result is treated as a causal B3
comparison. The final isolated interruption journey completed in 3.54 seconds
with all three Wasm artifacts reused.

## Focused Validation

Passed on 2026-08-18:

- locked compilation for `canic-core`, `canic-control-plane`, `canic`,
  `runtime_probe` and `canic-tests`;
- locked warning-denied Clippy for the four production/fixture packages, all
  targets, plus the `timer_authority` integration target;
- all 1,118 `canic-core` library tests: 1,117 passed and one ignored;
- the exact stable-bound, claim/coalescing, lease-takeover, cycle-only exact
  retry, stale-completion, abandon and overflow tests;
- all 19 role-contract, 11 state-contract, eight timer-workflow and two
  control-plane pool tests;
- both maintained timer source-inventory tests;
- all four `timer_authority` PocketIC journeys, followed by the exact
  interruption journey after measurement instrumentation was removed; and
- all four canonical fast product-role builds.

The complete workspace, release matrix and broad PocketIC suites were not run
for this historical phase. Later batches completed B1-B8 and `v0.104.0` was
published; current closeout authority is recorded in the design status.
