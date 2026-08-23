# Canic 0.108 Implementation Status

Date: 2026-08-23

## Status

- State: 0.108.0 is the policy-only checkpoint. The first human closeout audit
  rejected the then-open 0.108.1 draft. Its source corrections were applied:
  both funding legs use
  bounded calls, the Root ICP replay journal fails closed at 4,096 lifetime
  identities, and the qualification matrix now uses PocketIC's production ICP
  Ledger/CMC plus real single- and multi-Root journeys. The maintainer's first
  validation attempt then exposed stale CLI/hash/timer expectations and one
  pre-activation Root-admission defect; its follow-up exposed duplicated-match
  Clippy failures. Those candidate defects were corrected. On 2026-08-22 the
  maintainer then accepted CANIC-019 as a distinct B9/M8 amendment: exhausted
  finite authority may be renewed only through an explicit, digest-bound,
  replay-safe same-release policy-generation rotation that retains prior usage
  and application state. B9 implementation, generated surfaces and focused
  interruption/PocketIC evidence are complete. Closeout hardening also retains
  exact complete plans for historical replay, recovers the Root through the
  protected-authority/Registry-mirror split, preflights retained fixed-window
  spend and proves the measured 25,315,095-byte fragmented history inside a
  32 MiB cell. The 2026-08-23 validation rerun then exposed one layering-gate
  defect: the model read rotation DTOs directly. One ops-owned conversion now
  supplies a DTO-free named model input without moving or duplicating the
  invariant decision; `make check-invariants`, focused consumers and
  warning-denied changed-package Clippy pass. The following maintainer test
  run completed all serial PocketIC suites but exposed four ordinary-test
  consistency failures: the replay-policy manifests omitted the five new
  rotation commands, and host tests/order evidence still referred to a removed
  controller-only admission helper. The subsequent correction classifies those
  commands by their durable operation IDs, exercises identity admission through
  the live operator-funding observer and stops combined validation before
  PocketIC whenever the ordinary tier fails. Fake ICP fixtures are now
  atomically published from a closed staged file, removing the parallel-test
  `ETXTBSY` race exposed during focused reproduction. The focused manifest,
  observer, install-order, runner-contract and warning-denied changed-package
  checks pass. The qualification-runtime follow-up keeps the complete internal
  PocketIC matrix in one ordered process, persistently caches its canonical
  Coordinator artifact, moves pure internal checks before the PocketIC barrier,
  reuses only reset-complete native baselines and emits Fleet phase plus shared-
  server resource timings. Two exact governed runs passed all 22 internal cases:
  the cache-populating pass took 293 seconds and the cross-process reuse pass
  took 208 seconds, with a 2,229,804 kB server high-water mark and 97 threads.
  The changed six-test native-agent target also passed, with a 2,519,036 kB
  high-water mark and 162 threads. Serial capacity remains one because these
  local measurements do not yet prove parallel stability. The final source at
  `075560dc1ff87d872dc40d22fa7b3e48f3113260` then passed the complete
  `make validate` gate, including all 22 governed internal PocketIC cases, and
  was tagged `v0.108.1` before final closeout. The fresh closeout audit found
  no runtime P0 or P1 defect but initially rejected release-line closeout
  because active evidence still described an unpublished draft and one older
  policy paragraph contradicted CANIC-019. The forward documentation correction
  resolves both findings, and the complete gate passes on that corrected
  source. On 2026-08-23 the maintainer explicitly accepted the corrected 0.108
  closeout, authorized the 0.108.2 release and directed continuation into 0.109
  after publication. No 0.108 product work remains.
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
- Toko operator feedback also exposed that Fleet input incorrectly required
  self-declared account, balance and observation metadata. The corrected hard
  cut retains only the top-level operator Principal, derives the active
  identity's Cycles Ledger account and live balance through ICP CLI, excludes
  volatile observations from the plan digest and rechecks sufficiency before
  install effects. The follow-up hard-cuts fresh-Fleet creation to cycles,
  includes one exact Cycles Ledger fee for every operator-created
  infrastructure Canister, and adds a node-scaled funding-profile scaffold
  that resolves selected Subnet IDs through validated Registry evidence,
  retains an offline explicit-count fallback and emits formula plus exact-TOML
  output. ICP conversion authority is deferred to an unnumbered idea.
- Toko fresh-install feedback exposed an observer-coupling defect outside the
  funding protocol: an autonomous Root could reach `Provisioned` before the
  Coordinator observed its intermediate counters and remain permanently
  unreconciled. The 0.108.1 implementation accepts the exact compiled terminal
  receipt directly, permits its completion time to predate the passive query intent,
  normalizes progress raced by the post-acceptance query, and retains bounded
  scheduled-retry diagnostics. Focused direct-terminal, canonical-acceptance,
  restart/replay, forged-receipt and stepwise regressions pass.
- Predecessors: completed 0.103 and 0.104, accepted 0.106 B1 baseline and
  completed 0.107 fresh-Fleet preflight/runtime admission, plus the
  current root cycle/external-call ownership inventory and proposed cost
  envelope are available. The 0.107 closeout and complete M0 evidence are
  accepted. The held 0.106 B2 external work does not gate this line.
- Successors: 0.109 Fleet-wide ingress admission remains blocked until this
  line is complete; 0.110 estate implementation also depends on completed
  0.109.
- Surface posture: 0.108.0 remains policy-only. Tagged 0.108.1 adds Coordinator
  `RequestRootFunding` and `SetRootFunding` plus Root `AcceptFunding` and
  `RefillCycles` commands, generated Candid contracts, timer-owned request/
  fallback initiation, protected Coordinator/Root `Funding` status and
  `canic cycles funding`. Tagged 0.108.1 also adds protected Coordinator
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
| B3 | Coordinator grant authority | Registry-bound decisions, treasury windows, reserve, intents, receipts and attached-cycles call | Policy, authority, replay, stable-capacity, role-contract and generated-surface tests | Included in tagged 0.108.1; runtime audit pass |
| B4 | Root acceptance and request journal | Exact acceptance, zero-accept replay and monotonic current/last-result replacement | Root state/restart tests, accepted M0 response-loss platform proof and representative generated Root artifact | Included in tagged 0.108.1; runtime audit pass |
| B5 | Sparse-policy correction and Root timer integration | Single, preview multi and professional multi topology profiles, finite non-renewing caps, explicit Fiduciary placement, low-balance request ownership, nonterminal resumption and unchanged descendant funding | Host/config/hash, timer/policy/restart, placement-warning, generated-Candid and Root/Coordinator artifact checks | Included in tagged 0.108.1; runtime audit pass |
| B6 | Manual and automatic ICP refill | Protected policy, cumulative budget, floor, terminal fallback and mutual exclusion | Ledger/CMC replay suites plus real reserve-denial fallback journey | Included in tagged 0.108.1; runtime audit pass |
| B7 | Operator and lifecycle completion | Direct top-up resolution, status, metrics, Medic, runbooks, draining fences and generated surfaces | CLI/host/lifecycle/snapshot checks | Included in tagged 0.108.1; runtime audit pass |
| B8 | Qualification and closeout readiness | Real value-transfer and fallback journeys, measured guidance, cycle-only creation admission, fee-complete operator debit, Registry/offline profile scaffold, sediment audit and closeout handoff | Complete `make validate`, ICP CLI 1.3 balance fixtures and PocketIC evidence | Corrected closeout accepted 2026-08-23; 0.108.2 publication pending |
| B9 | Explicit policy-generation rotation | No-effect installed-Fleet plan, exact digest/predecessor apply, Coordinator-owned durable fence and Root prepare/activate receipts, retained cumulative usage, complete historical replay checkpoints and unchanged application state | Policy/hash, controller/stale/concurrent/payload-drift rejection, mixed authority/mirror recovery, 32 MiB fragmented stable-capacity proof, interruption/restart, generated-Candid/CLI and focused PocketIC exhausted-to-successor journey | Included in tagged 0.108.1; runtime audit pass; protocol wording corrected forward |

Nine batches match the amended design's M0-M8 dependency boundaries. They are not
preassigned patch releases.

## Next Authorized Action

Complete the maintainer-owned 0.108.2 validation/version/publication workflow,
preserving the existing `v0.108.1` tag. After 0.108.2 publication, begin the
explicitly authorized 0.109 B1 batch. The 0.108.0 policy checkpoint and held
0.106 B2 work remain independent.
