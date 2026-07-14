# Recurring Audits

Reusable definitions live under one domain:

- `docs/audits/recurring/system/`
- `docs/audits/recurring/invariants/`

Select methods and canonical owners through
[METHODS.md](../METHODS.md). Run and report them through
[AUDIT-HOWTO.md](../AUDIT-HOWTO.md).

## Definition Requirements

Every definition names:

- stable audit ID and version;
- canonical owner and trigger;
- kind/output profile;
- mode, cost, runtime, scope, exclusions, and prerequisites;
- method-specific decision and false-positive rules; and
- the shared execution, evidence, state, retention, redaction, and comparison
  contract it follows.

## Output Profiles

- Invariant methods prove the exact positive, rejection, and boundary cases.
- Trend methods freeze metrics and compare only method-compatible baselines.
- Measured methods record fixtures, environments, uncertainty, and bounded raw
  evidence.
- Manual methods record exact samples, reviewer identity, unreviewed
  boundaries, and disagreement handling.

Generic hotspot, fan-in, or risk-score sections are required only when the
selected method says they influence its decision. A large uniform report shape
is not evidence of coverage.

## General Architecture Bundle

Start broad architecture review with:

- [system/layer-violations.md](system/layer-violations.md)
- [system/module-structure.md](system/module-structure.md)
- [system/dependency-hygiene.md](system/dependency-hygiene.md)
- [system/capability-surface.md](system/capability-surface.md)
- [system/dry-consolidation.md](system/dry-consolidation.md)

Add the build, release, auth, lifecycle, measured, trend, or publish method only
when its trigger or the accepted line coverage map requires it.
