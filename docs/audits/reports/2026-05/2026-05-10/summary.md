# Audit Summary - 2026-05-10

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `module-structure.md` | Recurring system | facade/core/control-plane/memory/testkit/operator crates, fleets, test/audit/sandbox canisters | `d6ea5e3b` | dirty | complete |
| `dependency-hygiene.md` | Recurring system | workspace manifests, published/support crates, operator crates, fleets, test/audit/sandbox canisters | `d6ea5e3b` | dirty | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `module-structure.md` | 4 / 10 | No high or critical structural violation was confirmed. Risk is mostly hub containment and the expanded 0.33 operator-crate package surface rather than dependency direction or cycles. |
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
  structural hotspot: `publication/mod.rs = 1509` lines and
  `publication/fleet.rs = 704` lines.
- `canic-core` provisioning and IC management remain known refactor pressure:
  `workflow/ic/provision.rs = 697` lines and `infra/ic/mgmt.rs = 612` lines.
- `canic` hidden macro/build support has grown with the metrics-profile work:
  `macros/endpoints.rs = 656` lines and `build_support.rs = 507` lines.
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

1. Control-plane maintainers: split publication workflow by phase or
   responsibility when publication behavior changes next.
2. Core/runtime maintainers: keep IC management/provisioning decomposition
   tracked in `docs/design/0.33-icp-cli/refactor-addendum.md`.
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
