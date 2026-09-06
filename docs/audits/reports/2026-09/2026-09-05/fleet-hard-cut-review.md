# Fleet correction hard-cut review

The maintainer directed removal of custody and rework of retirement within Fleet on
2026-09-06. The separate custody package, host commands, pinned predecessor
owner and test harness are removed. Their passing fixtures do not qualify the
new retirement implementation.

## September 6 maintainer correction: explicit reinstall and conditional deletion

The maintainer clarified after the second Toko sanity review that reinstall is
the supported hard cut. It may preserve controlled canister identities and cycle
accounts while discarding application state. Complete Fleet evacuation is only
for deleting the Fleet. The earlier blanket retirement prerequisite was wrong
and is withdrawn. No temporary Wasm or predecessor endpoint reader belongs in
the reinstall path.

Generation now admits a changed Root module using management observations and
sealed replacement input. Ensure compiles an explicit Root reinstall prerequisite
through its existing effect journal. A subsequent reviewed full plan owns current
Fleet reconstruction and bounded protocol convergence. Policy/status errors no
longer infer resets. The complete
[generated changed-release journey](fleet-reinstall-journey.md) passes, including
lost-response recovery, working Fleet reconstruction, conservation and both
full-plan replay paths.

Fleet must settle existing work, transfer native cycles and canister-owned
Cycles Ledger balances to exact controlled destinations, verify receipts and
bounded residuals, then delete. Existing Root/Store native reclamation is the
starting point. Root and Coordinator Ledger evacuation now retain exact
receipts through lost replies and replay. Complete Coordinator native
evacuation/deletion and whole-Fleet terminal conservation remain optional
follow-up, outside this reinstall correction's release criteria.

The [IC management specification](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/#ic-method-delete_canister)
says raw deletion discards remaining cycles. The
[ICP CLI lifecycle guide](https://docs.internetcomputer.org/guides/canister-management/lifecycle/#delete-a-canister)
describes recovery before deletion. Local ICP CLI 1.4.0 uses a temporary shim
for liquid cycles. That is not proof of evacuating a canister-owned Ledger account.

Fleet Ensure owns current desired-state convergence. Retirement owns settling
work and returning cycles before deletion. Explicit reinstall uses generation
and Ensure. Neither operation introduces old schema readers, state migration,
release migration or a second deployment owner. Same-operation interruption
recovery, exact authority, intent before effects and conservation remain required.

## Disposition

| Area | Decision |
| --- | --- |
| Complete initial supply, funding validation and protected inventory | Keep: these correct ordinary deployment failures. |
| Bounded Ensure successor phases | Keep: one reviewed operation finishes within cumulative authority. |
| Exact receipts, same-operation recovery and conservation | Keep: required for paid and destructive effects. |
| Start-only successor release coupling | Removed: Start binds only the observed installed module and management authority. |
| Separate custody artifact, host modes and recovery-upgrade protocol | Removed completely. Fleet owns transfer before deletion. |
| Pinned predecessor protocol owner | Removed with custody; current Fleet gains no historical reader. |
| Explicit reinstall | Qualified through generation, reviewed Ensure execution, recovery, working Fleet reconstruction and replay. |
| Whole-Fleet evacuation | Conditional on deleting the Fleet. |

### Implemented hard cuts

- Removed implicit omission defaults from the current persisted Fleet plan,
  journal and state records. Nullable authority fields are explicit nulls;
  required lists, maps, funding bounds and scope are always present; unknown
  durable fields reject. User configuration defaults
  are a separate input contract and were not indiscriminately removed.
- Removed the observation-only fallback that substituted newer desired input
  into an older plan without a retained desired snapshot.
- Removed the applied-Create missing-balance repair for the 0.109.32 journal
  shape and its historical-shape tests. Current issued-Create lost-response
  recovery and exact terminal balance checks remain.
- Restored the narrow `RootStartPrerequisite` name: this scope permits Start
  of the exact observed module and cannot authorize infrastructure replacement.
- Removed the generator's unreachable policy-drift allowance and Root
  module-change rejection. Management observations bind explicit reinstall;
  old code receives no current protected queries before that reset.
- The Start prerequisite cannot authorize reinstall. Reuse of a controlled
  Principal during explicit reinstall is permitted and is not state migration.
- Aligned report JSON with the complete current plan key set, including its
  continuation authority, explicit nullable fields and scope.
- Removed the superseded Store-adoption plan validator, predecessor-module
  checks, special rejection retry and compatibility action ordering. Current
  reinstall has no old-plan continuation path.

## Evidence and remaining limits

The [focused Fleet tests](artifacts/feedback-activation/canic-start-cut-host-tests.log)
pass after the Start and policy cuts: 139 passed, two governed cases ignored.
The generated nineteen-plus-five journey passes in 1092.56s, including
conservation and both replays, before the final Start cut. The complete
[five-plus-five timing comparison](fleet-feedback-performance.md) qualifies
observation reuse for that local journey. These are checkpoint results and do
not qualify the new retirement changes.

Toko's retained journal and Wasms cannot prove present live retirement
eligibility. Read-only mainnet queries on September 6 failed while loading the
encrypted identity; no runtime observation or update was obtained. Applicability
remains unknown. Current source cannot add endpoints to those installed modules.
Any unsupported installed state must be reported explicitly; it does not justify
an unreviewed recovery mode. Downstream rehearsal remains downstream work.

The accepted correction batch is **ready for release approval**. Its complete
generated reinstall workflow and existing transfer corrections are qualified.
Whole-Fleet deletion is optional follow-up, not a blocker for this release.
The [readiness handoff](fleet-feedback-readiness.md) records the exact passing
checks, limits and Toko adoption work. Governed release approval, validation,
versioning and publication remain maintainer-owned.
