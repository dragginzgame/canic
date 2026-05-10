# Audit Summary - 2026-05-10

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `module-structure.md` | Recurring system | facade/core/control-plane/memory/testkit/operator crates, fleets, test/audit/sandbox canisters | `7e0ec893` plus current 0.33.5 cleanup worktree | dirty | complete |
| `dependency-hygiene.md` | Recurring system | workspace manifests, published/support crates, operator crates, fleets, test/audit/sandbox canisters | `d6ea5e3b` | dirty | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `module-structure.md` | 3 / 10 | No high or critical structural violation was confirmed. The stale core provisioning/IC-management, facade macro/build, and control-plane release publication hotspots were split; remaining risk is mostly control-plane fleet/lifecycle and host phase-file containment. |
| `dependency-hygiene.md` | 2 / 10 | No high or critical dependency hygiene violation was confirmed. The host and CLI package graphs were narrowed off the canister facade, leaving mostly intentional facade/support-package pressure. |

## Method / Comparability Notes

- `module-structure.md` uses `module-structure-v2`.
- The run is marked non-comparable with the April baseline because the scope now
  includes `canic-host`, `canic-cli`, `canic-backup`, and `fleets/**` as active
  0.33 package/operator surfaces.
- `dependency-hygiene.md` uses `dependency-hygiene-current`.
- The dependency-hygiene run is marked non-comparable with the April baseline
  because the 0.33 package graph added published operator crates and replaced
  older installer/proc-macro package names in the audited surface.

## Key Findings by Severity

### Medium

- `canic-control-plane` publication workflow remains the largest current
  structural hotspot by phase, but release publication is now split. Remaining
  publication pressure is `publication/fleet.rs = 704` and
  `publication/lifecycle.rs = 540`.
- `canic-host` install/release support files are the next operator hotspot:
  `install_root/mod.rs = 793`, `release_set/mod.rs = 741`, and `icp.rs = 600`.
- `canic-host` now exposes seven public host/operator modules and should remain
  in future module-structure scope.
- `access/auth/identity.rs` intentionally resolves delegated sessions at the
  endpoint auth boundary, but the state cleanup/metrics side effects should not
  spread into general policy modules.
- `canic-cli`, `canic-host`, and `canic-backup` are now published package
  surfaces. Their dependency direction is clean, but CLI/host/backup ownership
  boundaries need continued discipline.

### Low

- `canic-core` root containment stayed stable: support roots remain
  `#[doc(hidden)]`, and internal implementation roots remain `pub(crate)`.
- `canic-core` IC management and provisioning were split from monolithic files
  into normal directory modules. The largest focused files are now
  `infra/ic/mgmt/types.rs = 296`, `infra/ic/mgmt/lifecycle.rs = 186`,
  `workflow/ic/provision/allocation.rs = 193`, and
  `workflow/ic/provision/install.rs = 138`.
- `canic` hidden macro/build support has already been split into focused
  modules, leaving dispatch roots of `6` and `10` lines.
- `canic-control-plane` release publication was split from
  `publication/release.rs = 845` into focused modules, with the largest now
  `release/managed.rs = 275`, `release/chunks.rs = 189`, and
  `release/catalog.rs = 132`.
- `canic-testkit::pic` root pressure improved from the April report:
  `349 -> 285` lines.
- No crate-level or subsystem-level cycle was confirmed.
- No runtime import of `canic-testing-internal` or `canic-tests` was confirmed
  outside test/internal harness scope.
- No published crate runtime dependency on `canic-testing-internal` or
  `canic-tests` was found.
- `canic-host` now depends on `canic-core` plus host data/formatting crates,
  without linking the `canic` facade.
- `canic-cli` now depends on `canic-core`, `canic-host`, and `canic-backup`,
  without linking the `canic` facade.
- `canic-backup` stays independent of `canic`, `canic-host`, and `canic-cli`.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `module-structure.md` | 8 | 0 | 0 | Definition/baseline review, root surface scan, manifest/metadata scan, hub-size scan, cross-layer import scan, test/fleet/audit seam scan, and package build check passed. |
| `dependency-hygiene.md` | 8 | 0 | 0 | Definition/baseline review, metadata scan, direct manifest inspection, internal seam grep, feature scan, focused `cargo tree` checks, and operator package check passed. |

## Follow-up Actions

1. Control-plane maintainers: keep release publication behavior in the focused
   `publication/release/*` modules, and split `publication/fleet.rs` or
   `publication/lifecycle.rs` before adding more phase branches there.
2. Core/runtime maintainers: keep new IC management and provisioning behavior
   in the focused `infra/ic/mgmt/*` and `workflow/ic/provision/*` modules.
3. Facade/build maintainers: keep metrics/config build helpers behind hidden
   `__build`.
4. Operator maintainers: preserve CLI/host/backup ownership boundaries as the
   ICP CLI flow continues.
5. Auth maintainers: keep delegated-session cleanup side effects isolated to
   endpoint access-boundary code.
6. Host maintainers: keep host features on `canic-core`/data dependencies
   unless a future facade dependency is deliberately justified.
7. Package maintainers: keep all fleets and test/audit/sandbox canisters
   explicitly unpublished.
