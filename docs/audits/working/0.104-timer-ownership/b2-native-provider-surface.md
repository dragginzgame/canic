# 0.104 B2 Native Provider Surface Hard Cut

Date: 2026-08-18

## Result

B2 was accepted by the maintainer on 2026-08-18. The public
Canic application timer facade is deleted without an alias. Application code
now owns native `ic-timers` registrations directly, and Canic callbacks use
the provider's result, completion and directive types rather than parallel
Canic vocabulary.

The hard cut removes:

- `canic::timer!` and `canic::timer_interval!`;
- the public `canic::api::timer` namespace and prelude exports;
- public `TimerHandle` plus application `set`, `set_interval` and `cancel`;
- `TimerClaimId`, transient claim keys and transient identity allocation; and
- Canic-local callback result and directive types and their one-for-one
  provider conversions.

Private lifecycle deferral and the remaining fixed Canic owners retain their
bounded native claims until B3-B5 move each claim to its final domain owner.
This is private propagation, not a replacement application facade.

The runtime probe now demonstrates direct application-owned native custody:
one detached once registration, one retained after-completion registration,
one cancelled native registration and bounded capacity rejection. Its
application row remains visible beside Canic and companion-framework rows in
the one shared provider inventory.

The design originally named a negative compile-fail test for the deleted
surface. Repository policy prohibits anti-resurrection tests for pre-1.0 hard
cuts, so B2 uses current-surface evidence instead: the direct native consumer
compiles, public facade references are absent, and the maintained source
inventory guard accepts only the current provider call sites.

## Historical Fast-Profile Wasm Observation

This development-phase table is retained for provenance only. The exact B2
source state was not preserved, and the alleged 0.103 baseline was not
reproduced by the independent closeout build. These values are not closeout
or release acceptance evidence.

| Role | Current raw bytes | Raw delta | Raw delta % | Raw SHA-256 | Current gzip bytes | Gzip delta | Gzip delta % | Gzip SHA-256 |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| managed Component | 3,569,959 | +185,098 | +5.4684% | `9126cc2db1767cba6ddac98ff6b40d40da86cbd3c1b72071a6ce8a3fda151c09` | 944,470 | +61,203 | +6.9292% | `915da1b25648bb5fccde4eb43b112530b73aad929620113abd358b186f32c047` |
| Fleet Subnet Root | 8,420,862 | +305,176 | +3.7603% | `968f34acd95d5f29d554d171483e427a0d44037dbeb520c509c3e0dd27f66d50` | 2,182,926 | +96,351 | +4.6177% | `5242ff376d64f03802231831c4c1ef6d5e6b8876fbd06e25082803d18ade473f` |
| Fleet Coordinator | 4,069,259 | -1,876 | -0.0461% | `c6f026d42b02147d9cacaf93bddb3b6162cfeeff3c36bcd9e83610a532c78bba` | 1,013,440 | -468 | -0.0462% | `07b91f4b392ef8b6c012dbe6a4b27d81c5f2f749c19b1b63dda136d307b2f947` |
| Wasm Store | 3,356,397 | +26,207 | +0.7870% | `1a25fffa500bba7ca1458e43365c519dd864f8195e621aec6469b53ea0282e39` | 889,486 | +9,853 | +1.1201% | `e38c6b9f2f749cd6977598563680c353d2efa27787f08f818da3e52ed61ac024` |
| **Four-role total** | **19,416,477** | **+514,605** | **+2.7225%** | — | **5,030,322** | **+166,939** | **+3.4326%** | — |

This table was originally interpreted as an intermediate regression. Because
the phase and baseline cannot be rebuilt, that interpretation is withdrawn;
the hard cut remains justified by ownership rather than size.

The direct-native `runtime_probe` fixture moved from 3,673,361 to 3,663,579
raw bytes, a reduction of 9,782 bytes or 0.2663%. Deterministic gzip moved
from 910,615 to 909,879 bytes, a reduction of 736 bytes or 0.0808%. Its current
raw SHA-256 is
`f4b2e94c1903e8348fbe487ae5464d727a4b8c11453c1f3acd2c928eef442979`;
its deterministic-gzip SHA-256 is
`1fc441595fe071187bdae49971990ea37b4a993d020c63df6d235cc9150b0879`.

## Provider Performance And Inventory

The managed runtime-probe journey retained seven provider inventory rows after
the one-shot and cancelled application registrations reached terminal state.
Exactly one row was scheduled: the direct application-owned
`runtime-probe/application/timer-interval`. Five idle Canic rows and one
companion-framework row were unregistered.

After two completed interval callbacks, the provider reported:

| Observation | Value |
| --- | ---: |
| Scheduler instruction samples | 0 |
| Work instruction samples | 2 |
| Latest work instructions | 25,145 |
| Maximum work instructions | 25,145 |
| Total work instructions | 50,248 |
| Maximum Wasm-memory growth | 0 pages |
| Maximum stable-memory growth | 0 pages |

The retained B1 baseline did not include numeric pre-cut instruction samples,
so these are current provider observations, not a causal instruction delta.
Warm harness wall time moved from 7.18 to 7.00 seconds while the locked
`ic-testkit` graph also changed; that 0.18-second observation is not attributed
to B2. No runtime performance improvement is claimed.

The four-test PocketIC journey also advances simulated time by 24 hours and
proves capability-pruned cycle top-up plus idle intent-cleanup and log-
retention owners execute zero callbacks. Direct application cancellation,
after-completion recurrence, provider capacity, invalid identity rejection,
watchdog exact-operation takeover and lifecycle reconstruction all pass.

## Focused Validation

Passed on 2026-08-18:

- locked warning-denied Clippy for `canic-core`, `canic-control-plane`,
  `canic` and `runtime_probe`, all targets;
- 1,122 `canic-core` library tests, with one ignored test;
- both timer source-inventory tests;
- both focused control-plane pool tests;
- eight facade/reference and managed-endpoint tests;
- warning-denied compilation and all four `timer_authority` PocketIC tests;
  the cold run completed in 63.62 seconds and the immediate warm run in 7.00
  seconds with all three Wasm artifacts reused;
- the exact single direct-native interval test after measurement
  instrumentation was removed; and
- all four canonical fast product-role builds.

The complete workspace, release matrix and broad PocketIC suites were not run.
They remain maintainer-owned release validation and do not block review of this
intermediate batch.
