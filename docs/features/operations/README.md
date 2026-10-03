# Operations And Diagnostics

Canic's operator tools run on your computer. They build canisters, inspect IC
networks, prepare deployment plans, apply approved changes, collect diagnostics,
and help recover interrupted work.

<img src="../../../assets/256x256/mechanic-point-right.png" align="left" width="110" alt="The Canic mechanic pointing toward the operations guides" />

Inspecting or planning never silently grants permission to change a Fleet.
Canic first shows the exact actions and spending bounds, then requires the
operator to approve that plan's digest.

<br clear="left" />

## Choose A Task

| If you want to… | Start here |
| --- | --- |
| Understand the deployment workflow | [Fleet Ensure overview](fleet-ensure.md) |
| Generate or write a deployment document | [Fleet desired state](fleet-ensure-desired-state.md) |
| Review and apply an ordinary Fleet change | [Plan and apply](fleet-ensure-plan-and-apply.md) |
| Create a Fleet from supplied canisters or add capacity | [Bootstrap and capacity](fleet-ensure-bootstrap-and-capacity.md) |
| Replace a pre-1.0 installation | [Clean reinstall](fleet-ensure-clean-reinstall.md) |
| Resume interrupted work or protect cycles during recovery | [Recovery and cycle safety](fleet-ensure-recovery-and-cycle-safety.md) |
| Drive Fleet Ensure from automation | [Automation results and receipts](fleet-ensure-automation.md) |
| Run a persistent local test network | [Local development Fleet](local-development-fleet.md) |
| Diagnose funding or ICP conversion | [Fleet funding](../../operations/fleet-funding.md) |
| Back up or restore a Fleet | [Backup and restore](../backup-and-restore/README.md) |
| Hand verified bindings to a frontend | [Frontend handoff](frontend-handoff.md) |

## The Reviewed Workflow

| Step | What Canic does | Effect boundary |
| --- | --- | --- |
| Inspect | Reads current state and authority | No paid Fleet effect |
| Plan | Calculates exact actions, funding bounds, and a digest | No paid Fleet effect |
| Review | Presents the plan for an operator decision | No approval is implied |
| Apply | Performs only the work bound to the approved digest | Requires `--apply <plan_sha256>` |
| Verify | Observes the result and prepares a successor plan | A converged Fleet has no mutation actions |

If an external call has an uncertain result, Canic records and reconciles that
operation before retrying. It does not assume that a lost reply means the work
failed.

## What The Operator Tools Provide

- App creation, role scaffolding, attachment, and configuration inspection
- canonical network enrollment and local replica lifecycle
- current desired-state generation from explicit release and estate authority
- reviewed Fleet planning, application, resumption, and verification
- exact canister dispositions and cycle-conservation bounds
- build and deployment evidence for later diagnosis
- backup, recovery, and passive policy checks
- readable terminal output plus stable JSON for scripts

Useful orientation commands are:

```bash
canic help
canic fleet generate staging --app-config canic.toml --release-build <sha256>
canic fleet ensure staging
```

## The Host And Canister Boundary

The CLI and host libraries may use workspace files, operator credentials, and
the installed `icp` binary. Deployed canisters never receive those files or
identity keys. They receive only approved requests through their defined
interfaces.

See [ICP integration](icp-integration.md) for selected builds, effective
configuration checks, management visibility, and read-only frontend sync
verification. App, Fleet, and workspace are different concepts: an App is the
reusable model, a Fleet is one deployment, and a workspace is the local source
and operator-state root.

## Continue From Here

- [Open the Fleet Ensure overview](fleet-ensure.md)
- [Read the CLI command reference](../../../crates/canic-cli/README.md)
- [Browse recovery and release operations](../../operations/README.md)
- [Configure an App](../../../CONFIG.md)
- [Choose the Canic features you need](../README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)
