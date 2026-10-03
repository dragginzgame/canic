# Fleet Ensure

`canic fleet ensure <fleet>` brings one deployed Fleet into line with a reviewed
desired-state document. It observes the current IC estate, prepares a no-effect
plan, and changes canisters only after the operator approves that exact plan
digest.

<img src="../../../assets/256x256/mechanic-point-right.png" align="left" width="110" alt="The Canic mechanic pointing toward the Fleet Ensure paths" />

Most operators need the normal workflow: write or generate desired state, review
the plan, apply its digest, and run it again to verify that no mutations remain.
Use the task table below when starting from supplied infrastructure, replacing a
pre-1.0 release, recovering interrupted work, or integrating automation.

<br clear="left" />

## Choose Your Starting Point

| What you need to do | Follow this guide |
| --- | --- |
| Write or generate the deployment document | [Fleet desired state](fleet-ensure-desired-state.md) |
| Review and apply an ordinary Fleet change | [Plan and apply](fleet-ensure-plan-and-apply.md) |
| Replace any pre-1.0 installation | [Clean reinstall](fleet-ensure-clean-reinstall.md) |
| Start with supplied infrastructure | [Bootstrap and capacity](fleet-ensure-bootstrap-and-capacity.md#supplied-infrastructure-bootstrap) |
| Add supplied pool canisters | [Bootstrap and capacity](fleet-ensure-bootstrap-and-capacity.md#add-supplied-capacity-to-a-current-fleet) |
| Resume or diagnose interrupted work | [Recovery and cycle safety](fleet-ensure-recovery-and-cycle-safety.md) |
| Drive Ensure from a script or wrapper | [Fleet Ensure automation](fleet-ensure-automation.md) |

## The Normal Workflow

1. **Describe the result.** Select one current desired Fleet document containing
   the network, canisters, artifacts, controllers, funding, placement, and
   bounded spending authority.
2. **Plan without changing the Fleet.** Ensure observes the selected estate and
   reports the exact actions needed to reach that desired state.
3. **Review the plan.** Check the dispositions, transfers, fees, maximum
   debit/burn, conservation equation, and `plan_sha256`.
4. **Apply only that digest.** The digest binds the approved inputs and effects.
   Canic records intent before issuing external effects and reconciles uncertain
   outcomes before retrying.
5. **Verify convergence.** Run the immediate successor plan. A converged Fleet
   has zero mutation actions.

```bash
canic fleet ensure <fleet> --desired fleets/<fleet>.toml
canic fleet ensure <fleet> --desired fleets/<fleet>.toml --apply <plan_sha256>
```

Use `--identity <name>` on `fleet generate`, `fleet readiness`, and
`fleet ensure` when you need to select the ICP signing identity explicitly.
Pass the same identity while reviewing or resuming an operation.

## What Review And Apply Mean

| Phase | What it may do | Operator approval |
| --- | --- | --- |
| Observe | Read current state and authority | None beyond command execution |
| Plan | Produce actions, spending bounds, and a digest | No paid Fleet effect |
| Apply | Perform only the effects bound to the reviewed digest | Exact `--apply <plan_sha256>` |
| Resume | Reconcile and continue the same approved operation | Reuses the retained approval |
| Successor review | Observe the result and prepare any next phase | New approval if more effects are needed |

A successful command is not automatically proof that the complete Fleet has
converged. Human output and JSON automation results distinguish phase completion
from `fleet_completed: true`.

## Safety Boundaries

<img src="../../../assets/256x256/mechanic-attention.png" align="left" width="110" alt="The Canic mechanic raising a hand beside Fleet Ensure safety rules" />

- Planning never grants permission for paid Fleet effects.
- Apply authority is limited to the exact reviewed digest.
- Interrupted operations reconcile live results before another effect is
  attempted.
- Funding and destructive actions remain bounded by the reviewed plan.
- Every controlled canister with recoverable cycles must reach an exact
  controlled destination or an explicitly accounted terminal outcome.
- An immediate replay of a completed operation must be effect-free.
- Every pre-1.0 release transition is a hard cut and clean reinstall; predecessor
  application state is not migrated forward.

<br clear="left" />

The detailed recovery guide defines the exact
[cycle-conservation](fleet-ensure-recovery-and-cycle-safety.md#cycle-conservation),
[retirement](fleet-ensure-recovery-and-cycle-safety.md#retirement-boundary), and
[hard-cut](fleet-ensure-recovery-and-cycle-safety.md#hard-cut-boundary)
contracts.

## Focused Reference Guides

| Guide | What it contains |
| --- | --- |
| [Desired state](fleet-ensure-desired-state.md) | Generation, physical inventory, schema, funding, placement, and artifact selection |
| [Plan and apply](fleet-ensure-plan-and-apply.md) | Plan review, digest approval, application, resumption, and no-effect replay |
| [Clean reinstall](fleet-ensure-clean-reinstall.md) | Selected-build replacement, retained evidence, interruption recovery, and fresh convergence |
| [Bootstrap and capacity](fleet-ensure-bootstrap-and-capacity.md) | Supplied Coordinator, Root, Store, and pool-capacity admission |
| [Automation](fleet-ensure-automation.md) | JSON result contracts, next actions, timing receipts, and wrapper rules |
| [Recovery and cycle safety](fleet-ensure-recovery-and-cycle-safety.md) | Conservation, retirement, unreadable evidence, hard cuts, and exceptional recovery |

## Existing Reference Routes

These routes keep established links useful while the detailed material lives in
focused guides.

<a id="automation-results"></a>
- **Automation results:** [JSON and wrapper contract](fleet-ensure-automation.md#automation-results)
<a id="clean-reinstall-from-physical-inventory"></a>
<a id="clean-reinstall-of-a-completed-fleet"></a>
- **Clean reinstall from physical inventory:** [replacement procedure](fleet-ensure-clean-reinstall.md)
<a id="generate-current-desired-state"></a>
- **Generate current desired state:** [generation procedure](fleet-ensure-desired-state.md#generate-current-desired-state)
<a id="bootstrap-a-literally-empty-estate"></a>
- **Bootstrap a literally empty estate:** [fresh-estate generation](fleet-ensure-desired-state.md#bootstrap-a-literally-empty-estate)
<a id="operator-icp-conversion"></a>
- **Operator ICP conversion:** [conversion procedure](fleet-ensure-desired-state.md#operator-icp-conversion)
<a id="supplied-infrastructure-bootstrap"></a>
- **Supplied infrastructure bootstrap:** [bootstrap procedure](fleet-ensure-bootstrap-and-capacity.md#supplied-infrastructure-bootstrap)
<a id="add-supplied-capacity-to-a-current-fleet"></a>
- **Add supplied capacity to a current Fleet:** [capacity import](fleet-ensure-bootstrap-and-capacity.md#add-supplied-capacity-to-a-current-fleet)
<a id="desired-state"></a>
- **Desired state schema:** [complete desired-state reference](fleet-ensure-desired-state.md#desired-state)
<a id="plan-and-apply"></a>
- **Plan and apply:** [ordinary convergence workflow](fleet-ensure-plan-and-apply.md)
<a id="cycle-conservation"></a>
- **Cycle conservation:** [accounting contract](fleet-ensure-recovery-and-cycle-safety.md#cycle-conservation)
<a id="retirement-boundary"></a>
- **Retirement boundary:** [retirement contract](fleet-ensure-recovery-and-cycle-safety.md#retirement-boundary)
<a id="hard-cut-boundary"></a>
- **Hard-cut boundary:** [pre-1.0 release boundary](fleet-ensure-recovery-and-cycle-safety.md#hard-cut-boundary)
<a id="unreadable-retained-plan"></a>
- **Unreadable retained plan:** [evidence-preserving recovery](fleet-ensure-recovery-and-cycle-safety.md#unreadable-retained-plan)
<a id="deliberate-selected-build-database-wipe"></a>
- **Deliberate selected-build database wipe:** [wipe procedure](fleet-ensure-recovery-and-cycle-safety.md#deliberate-selected-build-database-wipe)
<a id="retained-growth-and-dependent-recovery-review"></a>
- **Retained growth and dependent recovery review:** [review procedure](fleet-ensure-recovery-and-cycle-safety.md#retained-growth-and-dependent-recovery-review)
<a id="completed-replay-after-operator-account-activity"></a>
- **Completed replay after operator account activity:** [replay behavior](fleet-ensure-recovery-and-cycle-safety.md#completed-replay-after-operator-account-activity)

## Continue From Here

- [Write or generate desired state](fleet-ensure-desired-state.md)
- [Review and apply a Fleet plan](fleet-ensure-plan-and-apply.md)
- [Browse Fleet operations](README.md)
- [Choose the Canic features you need](../README.md)
- [Browse all documentation](../../README.md)
