# Idea: Canonical Fleet Subnet Root Crate

Date: 2026-09-06

## Status

- Classification: future priority slice, deferred and unnumbered. Recording
  this idea does not schedule a release or authorize implementation.
- Priority: consider for the next suitable batch after the current release
  validation and release work finishes. Placement must be reconciled with the
  accepted roadmap before implementation starts.
- Need: give Fleet Subnet Root the same canonical Canic-owned entrypoint
  packaging as Fleet Coordinator and Wasm Store, removing the former
  application-owned Root customization model.
- Owners: `canic` and `canic-control-plane` for runtime composition;
  `canic-host` for build and artifact selection; `canic-cli` for affected
  operator surfaces.
- Repository scope: Canic only; downstream adoption remains separately owned.

## Problem

Fleet Coordinator and Wasm Store have canonical entrypoint crates,
`canic-fleet-coordinator` and `canic-wasm-store`. Root is still built through
an App-specific canister package. Its generated build sources include the
App configuration, and its Wasm depends on the selected capabilities.

This reflects the earlier ability for a user to define a custom Root, including
lifecycle hooks and endpoint extensions. The desired contract gives Canic sole
ownership of Root source and lifecycle. Most Root implementation already lives
in `canic-control-plane`, so the main change is build ownership, packaging and
qualification rather than a new Fleet control plane.

## Consistent Canister Crate Names

Use the shared `canic-fleet-` prefix for all three infrastructure canister
entrypoint crates:

| Canister role | Proposed crate | Change |
| --- | --- | --- |
| Fleet Coordinator | `canic-fleet-coordinator` | Keep existing name |
| Fleet Subnet Root | `canic-fleet-root` | Add canonical crate |
| Wasm Store | `canic-fleet-wasm-store` | Rename `canic-wasm-store` |

Keep the crate names concise by omitting `subnet`. Fleet Subnet Root remains
the architectural role name; the common package prefix identifies the Fleet
infrastructure family without changing role scope or protocol identifiers.
The Store rename is a pre-1.0 hard cut, with no compatibility crate or alias.

## Proposed Slice

Deliver one coherent batch with the following implementation steps and their
direct evidence and cleanup:

1. Add `canic-fleet-root` as the canonical, thin Root entrypoint crate and
   rename `canic-wasm-store` to `canic-fleet-wasm-store`, following the
   Coordinator packaging pattern. Keep runtime orchestration in its existing
   control-plane owner.
2. Make the host build pipeline select and build that crate automatically from
   the exact App configuration and required capabilities. Resolve the config
   input and feature selection consistently for workspace and packaged use.
3. Hard-cut user-supplied Root packages, Root lifecycle hooks and Root endpoint
   extensions from the supported surface. Remove the obsolete paths completely
   without aliases, wrappers or fallback builds. Ordinary application-canister
   customization remains outside this cut.
4. Propagate the canonical Root artifact and consistent crate names through
   Cargo dependencies, role contracts, artifact discovery, sealed release sets,
   package publication, fixtures, examples and active documentation. Replace
   the demo and test App Root entrypoints with the canonical build path.
5. Qualify fresh deployment, explicit reinstall and same-release interruption
   recovery using the canonical Root artifact, and include the complete outcome
   in the open changelog draft when implemented.

Root Wasm may still differ between Apps because configuration and capabilities
remain compiled inputs. A shared entrypoint crate does not make those artifacts
interchangeable. Exact role, configuration, capability and module-hash bindings
must remain authoritative throughout build, release and deployment.

## Completion Evidence

- Focused build and role-contract checks prove automatic canonical Root
  selection and correct configuration/capability inputs, including rejection
  of mismatched artifact authority.
- Artifact discovery and sealed release packaging include the exact Root Wasm
  alongside Coordinator, Store and application artifacts, using the proposed
  canonical package names consistently.
- A packaged consumer can build its Fleet without maintaining a Root crate.
- PocketIC exercises fresh deployment and explicit reinstall, with exact
  controller and observed cycle authority, bounded paid effects and terminal
  conservation.
- Lost-response and interruption cases resume the same journalled operation;
  successful completion immediately replays without another effect.
- Fixtures, examples and active docs use the maintained canonical surface.

Release transitions remain reinstall-only. This slice adds no cross-release
state migration, identity preservation, mixed-version operation or compatibility
recovery. Same-release recovery and existing Fleet authority remain required.

## Estimated Size And Planning Boundary

Provisional estimate: **2–4 focused engineering days** for the complete batch.
The largest uncertainties are host build/package resolution, fixture changes
and deployment/recovery qualification. Confirm the estimate with a bounded
inventory when the slice is promoted; it is not a delivery commitment.

Before promotion, assign an accepted release position and complete batch plan
under [delivery cadence governance](../../../governance/delivery-cadence.md).
This priority note does not amend the active 0.110 work or cross a minor
closeout gate.
