# Audit Summary - 2026-05-11

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `publish-surface.md` | Recurring system | publishable crate manifests, package-local READMEs, binary/example/bench surface, and package verification for the 0.34 crate set | `bfa521d4` plus current 0.34 backup CLI worktree | dirty | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `publish-surface.md` | 3 / 10 | All 11 publishable crates package and verify. Package roles are broadly clear. The run found README default-feature drift and stale audit-template crate names; both were fixed in the same follow-up. |

## Method / Comparability Notes

- `publish-surface.md` uses `publish-surface-current`.
- The run is partially comparable with the April baseline because the package
  audit method is unchanged, but the published crate set changed after the 0.33
  hard cut: `canic-cli`, `canic-host`, `canic-backup`, and `canic-macros` are
  now current surfaces.
- The report was run against a dirty worktree because active 0.34 backup CLI
  work is in progress.

## Key Findings by Severity

### Medium

- Found and fixed `crates/canic/README.md` under-documenting the facade default feature surface:
  `crates/canic/Cargo.toml` defaults `metrics`, `control-plane`, `sharding`,
  and `auth-crypto`, while the README's “Default surface” section names only
  `metrics`.

### Low

- Found and fixed `docs/audits/recurring/system/publish-surface.md` naming old
  crate surfaces such as `canic-installer` and `canic-dsl-macros`; the current
  package set uses `canic-cli`, `canic-host`, `canic-backup`, and
  `canic-macros`.
- `cargo package` verification for `canic` emits the intended packaged-build
  warning about missing local `CANIC_CONFIG_PATH`/fleet config while still
  completing successfully.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `publish-surface.md` | 6 | 0 | 0 | Definition/baseline review, manifest inspection, README inspection, `cargo package` verification for 11 crates, packaged artifact-size capture, and metadata scan passed. |

## Follow-up Actions

Completed in the same audit follow-up:

1. Updated `crates/canic/README.md` so “Default surface” lists all default
   features and explains that disabling default features opts out of the
   pre-1.0 standard runtime bundle.
2. Refreshed `docs/audits/recurring/system/publish-surface.md` so its
   canonical published crate map matches the current 0.34 crate set.
