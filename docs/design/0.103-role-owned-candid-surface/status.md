# Canic 0.103 Implementation Status

Date: 2026-08-18

## Status

- Architecture state: accepted and scheduled as reserve-Fleet critical-path
  step 1.
- B1 evidence state: accepted on 2026-08-17, including the bounded external
  profile-bootstrap, caller-cut, variant-accounting, request/response-
  correlation and cycle-capability clarifications. A bounded B4 correction was
  accepted later that day: 32 Root and nine Coordinator command variants retain
  four independently required authority/evidence outcomes without restoring
  phase methods.
- Implementation state: B2 through B7 are complete in the unreleased worktree.
  The maintainer explicitly authorized B4 mutation on 2026-08-17. Root and
  Coordinator now expose only their role-owned command/status methods;
  autonomous Component provisioning and Root removal, both atomic caller cuts,
  the pre-adoption Store staging seam, cross-cutting presentation, legacy-
  emitter deletion, ceremonial `metrics` Cargo feature removal and exact
  generated Candid all pass their focused checks.
- Closeout-audit state: the initial 2026-08-18 audit's authorization-order,
  external profile-binding, Store-pruning and active-document/count findings
  are corrected and pass focused re-audit. The only open finding is the
  maintainer-owned package/tag identity boundary; the audit preserves its
  initial evidence and records the current disposition in section 21.
- Outcome: one bounded command/status control plane per Canic role. Compiled
  capabilities add variants, command/status authority is variant-specific
  and workflows retain every private phase. Only genuinely asynchronous or
  durable commands acquire operation identities.
- Baseline evidence: the immutable [`v0.102.2` role-surface capture](../../audits/working/0.103-role-owned-candid-surface/README.md)
  freezes all 207 methods across representative Root and managed profiles plus
  canonical Coordinator and Store interfaces. It separates 188 Canic-owned,
  three external-standard and 16 fixture-owned methods.
- Runtime impact: the unreleased B2-B7 worktree adds the exact managed,
  Root, Coordinator and Store `canic_status` DTOs and dispatchers plus typed
  compile-time capability derivation. Config-derived `AutomaticTopup` prunes
  its variant, and canonical builds bind exact generated Candid to Overview and
  release-artifact metadata. Root Store verification, install/Registry state
  and Component Directory projections preserve the selected profile identity.
  Generic host transport requires the fully resolved immutable binding, and
  CLI observation/mutation paths fail before transport when release, role,
  capabilities, Candid hash or profile digest evidence is absent or conflicts.
  Existing operation owners resolve their exact durable IDs; B4 adds the Root
  and Coordinator operations with the commands that create those identities
  rather than through a universal operation store. Representative Runtime-only,
  Sharding/AutomaticTopup, Root signer/non-signer and Store builds prove exact
  positive/negative variants, referenced types and public handler reachability.
  Active presentation and local-only emission use the consolidated surface;
  the four representative role appearances fall from 188 Canic methods to ten.
  Private top-up timer/callback pruning remains the explicit 0.104 owner.
- Release boundary: reinstall only; no old method alias, fallback caller,
  dual protocol, migration or mixed-version release set is permitted.
- Implementation approval: B1 was explicitly accepted against the completed
  0.102 diagnostic boundary on 2026-08-17. B2 through B7 are complete. The
  implementation batch is ready for the maintainer-owned release flow; no
  additional protocol mutation is authorized by closeout.
- Successor: 0.104 timer ownership begins from the completed autonomous-
  operation surface after the 0.103 release boundary is published. No later
  production protocol may restore the removed phase methods.

Design:
[Role-owned Candid surface and autonomous operations hard cut](0.103-design.md)

## Quantitative Contract

- managed application canister: status plus an optional command, maximum two;
- Fleet Subnet Root: command plus status, maximum two;
- Fleet Coordinator: command plus status, maximum two;
- those roles: maximum three only with one proven composite-status exception;
- Wasm Store: starts with command/status; every additional method needs
  independent byte-transport/resource admission evidence, and six is an
  absolute emergency ceiling rather than spare capacity; and
- capabilities, callers, operations and phases add zero methods.

Externally mandated standards and application-owned methods are counted and
classified separately from the Canic-owned architectural ceilings.

B1 may propose smaller counts. Raising a ceiling requires an explicit design
amendment and cannot be hidden in implementation.

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Exact endpoint/caller inventory and role manifest | six-way disposition, command/status authority and correlation, external binding bootstrap, reserved-name rule, old-to-new variant map and method/variant/type counts | reproducible Candid/source/caller inventory and review | Accepted 2026-08-17: bounded clarifications incorporated into the complete register, manifest, operation ownership and 0.104 handoff |
| B2 | Manifest-exact role status surfaces and standards | only accepted Root/Coordinator/Store/managed status variants, standards, bounded observations and exact external profile identity | DTO/policy, Candid, profile-binding, status authorization and first-excess tests | Complete in worktree: all four role dispatchers, exact current operation owners, representative status Candid, artifact-to-Directory profile binding and bounded-observation evidence pass |
| B3 | Compile-time variant pruning | pruned variants, DTOs, public handlers and types; reserved-name failures; thin `start!` | build pass/fail, Candid absence, collision, protocol reachability and role guards | Complete in worktree: exact positive/negative profile builds, native reserved-name collision, incompatible-feature rejection and thin composition pass; [evidence](../../audits/working/0.103-role-owned-candid-surface/b3-profile-pruning.md) |
| B4 | Root and Coordinator autonomous intent plus atomic caller cut | outcome-named commands, async/durable receipts, typed atomic responses, variant auth/replay, self-advance and private phases; every Root/Coordinator caller, binding, constant, replay row and fixture | property, binding and PocketIC replay/interruption journeys | Complete in worktree: 32 Root and nine Coordinator commands, exact participant observation authority, private reconciliation, caller/binding/replay cleanup, pre-adoption Store staging and autonomous provisioning/removal PocketIC journeys pass |
| B5 | Managed and Store role surfaces plus atomic caller cut | optional managed command, inert pruning, Store control/byte lanes; every managed/Store caller, binding, constant, replay row and fixture | authority, binding, absence, payload and cross-canister PocketIC tests | Complete in worktree: managed profiles expose only cfg-selected command/status variants; Store exposes command/status plus two admitted byte lanes; exact callers, replay rows, generated Candid and bootstrap/reverification pass |
| B6 | Cross-cutting presentation and residue | host/CLI help and presentation, remaining application/docs propagation, representative reports and global residue | targeted host/CLI/build/PocketIC checks and residue scans | Complete in worktree: host/CLI fixtures, then-current application presentation, active documentation and seven representative generated profiles use the current role surface; the later Prequel Wars planning cut removes the obsolete Skynet App; [evidence](../../audits/working/0.103-role-owned-candid-surface/b6-surface-report.md) |
| B7 | Removal and measured closeout | old surface deletion, residue guards, count/size report and documentation | generated count, forbidden-method and residue checks | Complete in worktree: legacy emitters and the empty `metrics` Cargo feature are deleted, current-surface guards pass, Store lanes are absent from managed/Coordinator Wasm and 188 representative Canic methods become ten; current Wasm identity is recorded without a causal savings claim; [evidence](../../audits/working/0.103-role-owned-candid-surface/b7-closeout.md) |

B2 emits no temporary all-capabilities status superset. The completed B2/B3
sequence has representative exact variant/type/public-handler/protocol-
reachability evidence.

## B4 Contract Reconciliation

The authorized implementation proved that four accepted B1 deletions could
not be completed without either losing required evidence or weakening an
existing authority boundary. The maintainer accepted this bounded correction
on 2026-08-17; it does not reopen the role-surface architecture:

1. Initial Store bootstrap currently stages the admitted release-set bytes in
   the Root before the Root publishes them to its adopted Store. The accepted
   Root union deletes the three staging variants, while the accepted Store
   authority admits only the exact Root caller. A host/controller cannot move
   those bytes directly without a separately accepted pre-adoption Store
   authority, which belongs to the B5 Store cut.
2. Registry synchronization requires one durable Coordinator acknowledgement.
   A Root status query cannot commit it, and the accepted
   `SynchronizeRegistry`/`ActivateRegistry` DTOs carry no replacement mutation
   channel. Removing `AcknowledgeRootSnapshot` would make complete activation
   evidence impossible.
3. Scale-out requires exact evidence that affected existing Component
   Directories converged. The accepted Root provisioning status does not carry
   the affected/synchronized counts or receipt currently returned by
   `SynchronizeComponentDirectories`; polling it cannot prove the Coordinator's
   durable confirmation.
4. Physical Root deletion is executed by an external controller after the
   Root is stopped. The accepted Coordinator `RemoveRoot` request carries only
   the initial draining reservation, so it cannot bind pre-execution authority
   evidence or typed post-deletion absence evidence. The Coordinator is not the
   Root's management controller and cannot manufacture either observation.

The corrected contract retains `SynchronizeComponentDirectories` as the 32nd
Root outcome; retains `AcknowledgeRootSnapshot`,
`PrepareRootDeletionExecution` and `CompleteRootDeletion` as the seventh
through ninth Coordinator outcomes; authorizes an exact participating Root to
read the existing Coordinator `Registry` status; and moves pre-adoption Store
staging to the admitted B5 Store lanes. Coordinator removal polls Root operation
status for draining, removal and readiness rather than exposing those phases.

## B1 Completion Contract

B1 must deliver together:

1. an immutable source, toolchain and generated-Candid baseline;
2. every emitted method by canonical role and capability;
3. exact execution mode, visibility, authority, payload and replay policy;
4. every workflow owner and in-repository caller;
5. exactly one disposition per method: role command variant, role status
   variant, Store data plane, external standard, application-owned or
   private/delete;
6. one accepted old-to-new variant and DTO mapping;
7. the exact static capability declaration, invalid-combination rules,
   external profile-binding bootstrap, verification-only `Overview` and
   reserved-name collision rule;
8. a final role/capability manifest satisfying the quantitative ceilings and
   proving capabilities prune variants and protocol reachability without
   adding methods;
9. a method/variant/type report, exact request/response correlation and the
   pre-call binding selection authority for every host, CLI and inter-canister
   caller;
10. proof that atomic commands avoid operation machinery while asynchronous or
    durable commands receive exact operation ownership; and
11. the exact 0.104 timer-consumer handoff.

B4 and B5 must each change every maintained caller with the receiving protocol.
Complete cross-cutting presentation and residue closure belongs to B6, but no
caller may wait until B6 for its first migration.

## Critical-Path Position

1. 0.103 hard-cuts the Candid surface and internalizes orchestration phases.
2. 0.104 hard-cuts timer mechanics/domain recovery and adds synchronous
   lifecycle composition.
3. 0.105 adds framework-neutral local application authorization.
4. 0.106 qualifies platform behavior, costs, balances and bounded lanes.
5. 0.107 closes replay-safe Coordinator-backed root operating funding.
6. 0.108 implements reusable Fleet Subnet Canister estates and proves the
   10/100/1,000 progression.
7. 0.109 qualifies one exact stateful predecessor/successor transition.
8. 0.110 serves the generic Fleet observatory from every installed Canister.

Unnumbered ideas remain outside the path.

## Next Action

Run the maintainer-owned minor release flow, which advances the current
`0.102.2` package authority to the exact `0.103.0` target and owns the complete
validation gate. No local or remote `v0.103.*` tag remains. The open `0.103.0`
changelog records this completed batch; no agent-owned version, tag or push is
authorized. Begin 0.104 only after the 0.103 release boundary is published.
