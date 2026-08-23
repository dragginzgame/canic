# Canic 0.108 Implementation Status

Date: 2026-08-22

## Status

- State: 0.108.0 is published. The first human closeout audit rejected the open
  0.108.1 draft. Its source corrections are now applied: both funding legs use
  bounded calls, the Root ICP replay journal fails closed at 4,096 lifetime
  identities, and the qualification matrix now uses PocketIC's production ICP
  Ledger/CMC plus real single- and multi-Root journeys. The maintainer's first
  validation attempt then exposed stale CLI/hash/timer expectations and one
  pre-activation Root-admission defect; its follow-up exposed duplicated-match
  Clippy failures. Those candidate defects are corrected, the focused
  regressions pass and the complete maintainer gate must be rerun against the
  final immutable candidate before a new human verdict. On 2026-08-22 the
  maintainer then accepted CANIC-019 as a distinct B9/M8 amendment: exhausted
  finite authority may be renewed only through an explicit, digest-bound,
  replay-safe same-release policy-generation rotation that retains prior usage
  and application state. B9 implementation, generated surfaces and focused
  interruption/PocketIC evidence are complete. Closeout hardening also retains
  exact complete plans for historical replay, recovers the Root through the
  protected-authority/Registry-mirror split, preflights retained fixed-window
  spend and proves the measured 25,315,095-byte fragmented history inside a
  32 MiB cell. The corrected open draft still requires the final maintainer
  validation gate and a fresh human closeout audit before release.
- Runtime impact: B3 adds exact registered-Root admission, the Coordinator
  funding kill switch, fixed Fleet/Root window accounting, durable
  current-plus-last replay and reserve-aware attached-cycle calls. B4 adds the
  Root-owned request journal and exact Coordinator-only accept-once/
  zero-accept-replay command. B5 adds the sole Root `cycles:topup` timer owner,
  resumes the exact durable operation before new work, stops on non-renewing
  cap exhaustion and resamples after every terminal result. B6 adds protected
  manual and terminal automatic ICP refill with one durable Ledger/CMC replay
  owner and cumulative plus non-renewing caps. B7 adds explicit installed-
  authority recovery, funding status/metrics, Medic and lifecycle/snapshot
  fences. B8 qualifies both real funding paths and reconciles active docs. B9
  adds explicit no-effect planning and controller-applied funding-policy
  generation rotation without changing application or descendant state.
- Maintainer amendment: on 2026-08-22 the funding design replaced the earlier
  illustrative values with an affordable single-Subnet 10T/30T profile and a
  professional multi-Subnet 250T/1000T profile, then added a distinct bounded
  `preview_multi_subnet` staging profile. Its recommended one-Root values are
  140T Coordinator creation, 80T reserve, 30T Root creation, 10T Wasm Store,
  10T/30T Root threshold/target, one 30T grant per 90-day window and a finite
  two-grant/60T lifetime cap with automatic ICP disabled: 180T total. It also
  hard-cuts the Fiduciary-backed `recommended` selector: each Fiduciary
  placement needs an exact protected-input acknowledgement and retained
  high-cost warning. B5 propagates that correction through immutable input,
  plans, policy hashes, Registry authority, stable accounting and generated
  Candid before enabling the Root timer.
- Predecessors: completed 0.103 and 0.104, accepted 0.106 B1 baseline and
  completed 0.107 fresh-Fleet preflight/runtime admission, plus the
  current root cycle/external-call ownership inventory and proposed cost
  envelope are available. The 0.107 closeout and complete M0 evidence are
  accepted. The held 0.106 B2 external work does not gate this line.
- Successor: 0.109 estate implementation remains blocked until this line is
  complete.
- Surface posture: 0.108.0 remains policy-only. The open draft adds Coordinator
  `RequestRootFunding` and `SetRootFunding` plus Root `AcceptFunding` and
  `RefillCycles` commands, generated Candid contracts, timer-owned request/
  fallback initiation, protected Coordinator/Root `Funding` status and
  `canic cycles funding`. The open draft also adds protected Coordinator
  `BeginFundingPolicyRotation`, `StageFundingPolicyRotationRoot` and
  `ApplyFundingPolicyRotation` variants plus Root prepare/activate lifecycle
  variants, while the CLI exposes `--plan-rotation` and `--apply-rotation`.
  Direct top-up and conversion resolve only exact authenticated installed
  Coordinator/current-Root authority.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | M0 recovery and admission proof | Minimal Coordinator/root atomicity proof, current/last-result model, offline break-glass authority and measured request/refill floors | Focused PocketIC value-transfer plus bounded pure/host proof | Accepted 2026-08-21 |
| B2 | Protected policy hard cut | Fleet-input schema-1 policy, validation, hashing, propagation and generic refill sediment removal | Host/config/hash, Candid-containment and final payload-bound tests | Complete; included in the 0.108.0 checkpoint |
| B3 | Coordinator grant authority | Registry-bound decisions, treasury windows, reserve, intents, receipts and attached-cycles call | Policy, authority, replay, stable-capacity, role-contract and generated-surface tests | Complete 2026-08-22; open 0.108.1 draft |
| B4 | Root acceptance and request journal | Exact acceptance, zero-accept replay and monotonic current/last-result replacement | Root state/restart tests, accepted M0 response-loss platform proof and representative generated Root artifact | Complete 2026-08-22; open 0.108.1 draft |
| B5 | Sparse-policy correction and Root timer integration | Single, preview multi and professional multi topology profiles, finite non-renewing caps, explicit Fiduciary placement, low-balance request ownership, nonterminal resumption and unchanged descendant funding | Host/config/hash, timer/policy/restart, placement-warning, generated-Candid and Root/Coordinator artifact checks | Complete 2026-08-22; open 0.108.1 draft |
| B6 | Manual and automatic ICP refill | Protected policy, cumulative budget, floor, terminal fallback and mutual exclusion | Ledger/CMC replay suites plus real reserve-denial fallback journey | Complete 2026-08-22; open 0.108.1 draft |
| B7 | Operator and lifecycle completion | Direct top-up resolution, status, metrics, Medic, runbooks, draining fences and generated surfaces | CLI/host/lifecycle/snapshot checks | Complete 2026-08-22; open 0.108.1 draft |
| B8 | Qualification and closeout readiness | Real value-transfer and fallback journeys, measured guidance, sediment audit and closeout handoff | Targeted repository gates and PocketIC evidence | Initial closeout rejected; candidate validation defects corrected; final maintainer gate and re-audit pending |
| B9 | Explicit policy-generation rotation | No-effect installed-Fleet plan, exact digest/predecessor apply, Coordinator-owned durable fence and Root prepare/activate receipts, retained cumulative usage, complete historical replay checkpoints and unchanged application state | Policy/hash, controller/stale/concurrent/payload-drift rejection, mixed authority/mirror recovery, 32 MiB fragmented stable-capacity proof, interruption/restart, generated-Candid/CLI and focused PocketIC exhausted-to-successor journey | Complete 2026-08-22; open 0.108.1 draft |

Nine batches match the amended design's M0-M8 dependency boundaries. They are not
preassigned patch releases.

## Next Authorized Action

Establish the immutable corrected candidate revision, rerun the complete
maintainer-owned validation gate and run a fresh human-owned 0.108 closeout
audit against the complete open 0.108.1 B3-B9/M2-M8 batch. Do not begin 0.109,
run remote qualification effects, version, tag or publish until that verdict
and the maintainer-owned release workflow authorize those actions. The
published 0.108.0 checkpoint and held 0.106 B2 work remain independent.
