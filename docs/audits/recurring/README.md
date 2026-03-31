# Recurring Audits

Canonical layout for new recurring definitions:
- `docs/audits/recurring/<domain>/<focus>.md`

Legacy layout compatibility:
- Existing flat files under `docs/audits/recurring/*.md` are historical baseline definitions.
- Keep them append-only and do not delete or overwrite history.
- New definitions should use the domain-scoped layout.

## Required Report Structure

All recurring audit reports must preserve:

- Report Preamble
- Method tag/version
- Comparability status
- Verification Readout

All recurring audit templates must require:

- `## Structural Hotspots` with concrete file/module evidence
- `## Hub Module Pressure` with normalized pressure scoring
- `## Risk Score` using the normalized 0-10 scale
- `## Amplification Drivers` for change/friction audits that analyze feature slices/commits
- `## Early Warning Signals` for predictive decay detection (enum shock radius, cross-layer struct spread, hub growth, capability surface growth)
- `## Dependency Fan-In Pressure` for module/type fan-in hub detection

Structural hotspot and hub-pressure sections must be grounded in repository scans
and include command evidence (for example `rg '^use '`, cross-layer import scans,
`git log --name-only -n 20`, and symbol discovery scans).

## Recommended Starter Bundle

For general architecture-health audit rounds, start with:

- [system/layer-violations.md](system/layer-violations.md)
- [system/capability-surface.md](system/capability-surface.md)
- [system/complexity-accretion.md](system/complexity-accretion.md)
- [system/wasm-footprint.md](system/wasm-footprint.md)

See also:

- [system/README.md](system/README.md)
- [system/instruction-footprint.md](system/instruction-footprint.md)
