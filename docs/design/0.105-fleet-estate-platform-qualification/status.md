# Canic 0.105 Implementation Status

Date: 2026-08-16

## Status

- State: B1 evidence work is approved; B2 execution is held. No qualification
  harness or evidence exists yet.
- Runtime impact: none. No production configuration, stable state, Candid, CLI,
  timer or package version changed.
- External effects: none authorized. B2 remains blocked until accepted B1
  freezes the protocol and an exact maintainer-approved run plan names network,
  identity, count, concurrency, cycle ceiling and terminal asset disposition.
- Predecessor reconciliation: repository-only B1 may continue while 0.103 and
  0.104 are pending, but B2 and the final current baseline require accepted
  0.103 Candid/operation ownership plus accepted 0.104 timer, recovery and
  stable-state ownership.
- Successors: 0.106 consumes the accepted B1 root-ownership/current-cost
  boundary and is not gated on B2. Accepted B2 evidence is mandatory for
  mutating 0.107 Fleet-estate work.

## Release-Batch Tracker

| Batch | Outcome | Owner | Included evidence | Validation | Status |
| --- | --- | --- | --- | --- | --- |
| B1 | Reproducible repository/local qualification boundary | host planning, test harness and current pool inventory | immutable baseline, Q2 provenance, frozen Q3/Q4 protocol, Q1/Q6 and local proofs, production-reachability inventory, exact external plan | focused host, control-plane and PocketIC checks; no external effects | Approved to begin |
| B2 | Dated platform qualification and accepted 0.107 handoff | qualification harness and audit report | protocol-bound disposable measurements, separately approved minimal mainnet confirmation, qualified cost/balance recommendations and complete reconciliation | run-specific checks plus maintainer evidence review | Execution held: blocked on accepted B1 and explicit external authorization |

The two-batch exception is intentional: 0.105 is an evidence-only predecessor
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
   accepted blocker assigned to 0.107;
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

If mainnet authorization is not granted, B2 and 0.105 are blocked, not failed,
and cannot be marked complete.

## Critical-Path Position

0.105 is reserve-Fleet critical-path step 3:

1. 0.103 hard-cuts the Candid surface and internalizes orchestration phases;
2. 0.104 completes the timer-consumer and async-job recovery hard cut;
3. 0.105 qualifies the platform and current pool boundary;
4. 0.106 closes replay-safe Coordinator-backed root operating funding; and
5. 0.107 implements reusable Fleet Subnet Canister estates and proves the
   10/100/1,000 progression; then
6. 0.108 serves the T2 Fleet observatory from every installed Canister.

All unrelated future concepts are unnumbered under `docs/design/ideas/`.
Published versions, historical changelogs, retained audit reports and archived
handoffs keep their historical identities.

## Next Authorized Action

Begin B1 only: freeze the baseline, provenance, measurement protocol, reset
fixtures and standby horizon; inventory current repository state and source
reachability; then build the bounded local harness and empty-topology proof.
Do not run a remote or IC-mainnet experiment or begin 0.107 production work.
