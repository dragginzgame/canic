# Canic 0.106 Implementation Status

Date: 2026-08-20
Roadmap reconciled: 2026-08-30

## Status

- State: B1 was accepted on 2026-08-20; B2 execution is held. The immutable
  baseline, exact Q1 host blocker, complete Q2 normative provenance matrix,
  accepted Q3/Q4 protocol and complete Q6 repository inventory are captured
  in the
  [working evidence](../../audits/working/0.106-fleet-estate-platform-qualification/README.md).
- Runtime impact: none. No production configuration, stable state, Candid, CLI,
  timer or package version changed. Every package already locked by
  `v0.105.0` retains its exact predecessor version. The current release tree
  also carries the separately classified, test-only 0.108 B1 probe; it is an
  unpublished dependency leaf and does not enter 0.106 B2 or a shipped role.
- External effects: none authorized. B2 remains blocked until accepted B1
  freezes the protocol and an exact maintainer-approved run plan names network,
  identity and terminal asset disposition. The proposed count, concurrency,
  fee/refund, reserve, physical-asset and funded-exposure ceilings are frozen.
- Predecessor reconciliation: published `v0.105.0` is the immutable direct
  predecessor. Its pool storage, ops, workflow, lifecycle, async-recovery and
  allocation paths are unchanged from published `v0.104.2`, so the accepted
  0.104 timer/state boundary remains inherited while the B1 source baseline
  uses `v0.105.0`, never stale `v0.104.1` reachability evidence.
- Successors: 0.107 consumes the accepted B1 repository baseline for its
  deployment-readiness inventory, and 0.108 consumes the accepted B1 root-
  ownership/current-cost boundary; neither is gated on B2. Accepted B2
  evidence is mandatory for mutating 0.111 Fleet-estate work. The accepted Q6
  current-state complexity constraints are inputs to 0.110 contraction.

## Release-Batch Tracker

| Batch | Outcome | Owner | Included evidence | Validation | Status |
| --- | --- | --- | --- | --- | --- |
| B1 | Reproducible repository/local qualification boundary | host planning, test harness and current pool inventory | immutable baseline, Q2 provenance, accepted Q3/Q4 protocol, Q1/Q6 and local proofs, production-reachability inventory, proposed external envelope | focused host, control-plane and PocketIC checks; no external effects | Accepted 2026-08-20 |
| B2 | Dated platform qualification and accepted successor handoff | qualification harness and audit report | protocol-bound disposable measurements, separately approved minimal mainnet confirmation, qualified cost/balance recommendations and complete reconciliation | run-specific checks plus maintainer evidence review | Execution held: blocked on explicit external authorization |

The two-batch exception is intentional: 0.106 is an evidence-only predecessor
with one external-effect authorization boundary, not a maintained runtime
minor. Neither batch is a preassigned patch release.

## B1 Completion Contract

B1 must deliver together:

1. immutable source/tool baseline;
2. complete current pool state, stable-ID, ceiling, scan and timer inventory;
3. normative-versus-observed Cycles Ledger provenance matrix;
4. an accepted protocol freezing repetitions, sample sizes, warm-up,
   conditions, reset fixtures, latency/censoring rules and the Q4 horizon,
   safety-margin and recovery-reserve model;
5. strict empty-topology host plan and PocketIC activation proof, or an exact
   accepted blocker assigned to 0.111;
6. bounded non-production lane, replay and controller-observation harnesses;
7. local positive, first-excess, interruption and contradictory-evidence
   cases;
8. a classification of every added file/module plus proof no harness code is
   reachable from a shipped runtime and no production surface changed; and
9. reconciliation of the final timer/stable-state inventory against accepted
   0.104; and
10. a proposed B2 run plan with exact external-effect ceilings.

## B2 Completion Contract

B2 remains blocked until the maintainer separately approves its external run
plan. Completion then requires:

1. reconciled disposable 1/8/16/32 creation and reset cohorts;
2. exact contract-versus-observation Cycles Ledger duplicate, fee, `TooOld`,
   uncertainty and Subnet evidence;
3. separately bounded IC-mainnet confirmation;
4. horizon-qualified standby and workload-qualified claim/install balance
   recommendations;
5. controller and routing observations;
6. exact cost, duration, rate and unresolved-operation accounting;
7. terminal evidence for every created Principal and controller set; and
8. an accepted report with explicit freshness/revalidation rules.

If mainnet authorization is not granted, B2 and 0.106 are blocked, not failed,
and cannot be marked complete.

## Critical-Path Position

0.106 is application-safety/estate step 4:

1. 0.103 hard-cuts the Candid surface and internalizes orchestration phases;
2. 0.104 completes the timer-consumer and async-job recovery hard cut;
3. 0.105 adds framework-neutral local application authorization;
4. 0.106 qualifies the platform and current pool boundary;
5. 0.107 closes fresh-Fleet preflight and runtime-admission gaps;
6. 0.108 closes replay-safe Coordinator-backed root operating funding; and
7. 0.109 establishes Coordinator-owned Fleet-wide ingress admission with
   complete local enforcement projections; then
8. 0.110 creates absolute Wasm byte/function reserves through zero-capability
   runtime contraction; then
9. 0.111 adds bounded indexed estates and cycle-safe source disposition with
   exact destination-credit and bounded source-debit reconciliation, without
   preserving application data or an existing Principal.

All unrelated future concepts are unnumbered under `docs/design/ideas/`.
Published versions, historical changelogs, retained audit reports and archived
handoffs keep their historical identities.

## Next Authorized Action

Hold B2 until a separate maintainer authorization binds the accepted protocol
to an exact approved network, identity and terminal asset disposition before
any external effect. Its operation, asset, concurrency, fee/refund, reserve
and funded-exposure ceilings are already frozen. Protocol
`canic-0.106-q3q4-v1`, the exact Q1 `EmptyRootAdmissions` blocker and the four
Q6 constraints were accepted on 2026-08-20. The roadmap now assigns the Q1
reserve-Fleet blocker to 0.111 and sends the Q6 complexity constraints through
0.110 B1 reclassification. Only fresh marginal Wasm evidence may retain one in
B4; none is corrected by 0.106. The protocol's exact
predecessor-built fixture hash and initialized memory observations are now
frozen, and its 1/8/16/32 creation and empty/installed reset lanes, exact
uncertainty retry, first excess and controller/routing transitions pass
locally. The terminal dependency/source guard also passes. Q2 freezes the deployed
Cycles Ledger v1.0.6 interface/source authority and keeps every empirical cell
pending B2. Q6 now
freezes its current encoded shapes, generic receipt/cost ceilings and snapshot
owners, including four explicit 0.110 reclassification inputs. The current
empty-topology path fails at exact `EmptyRootAdmissions`. No repository-local
B1 work remains.
Because 0.107 consumes accepted B1 and is not gated on B2, its baseline and
contract batch is the next in-repository sequence. The renumbered 0.108 B1
proof remains historical accepted evidence. Parallel creation/reset and
10/100/1,000 qualification are unscheduled; bounded estate work remains behind
0.109 closeout, 0.110 closeout and explicit 0.111 promotion. Do not run a remote
or IC-mainnet experiment without its separate exact authorization.
