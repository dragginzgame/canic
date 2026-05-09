# Canic Audit How-To

This document defines how to run and store architecture audits under `docs/audits/`.

## 0. Folder Structure (Canonical)

```text
docs/audits/
├─ AUDIT-HOWTO.md
├─ META-AUDIT.md
├─ recurring/
│  ├─ README.md
│  ├─ invariants/
│  │  ├─ README.md
│  │  └─ <focus>.md
│  └─ system/
│     ├─ README.md
│     └─ <focus>.md
└─ reports/
   └─ YYYY-MM/
      ├─ summary.md
      └─ YYYY-MM-DD/
         ├─ <scope>.md
         └─ summary.md
```

## 1. Audit Types

### Recurring audits
Recurring audits are stable, repeatable audit definitions that run on a schedule and enforce architectural contracts.

Location:
- `docs/audits/recurring/<domain>/<focus>.md`

All new recurring definitions should use the domain-scoped recurring layout.
Current domains include:

- `docs/audits/recurring/invariants/`
- `docs/audits/recurring/system/`

### Audit reports
Reports are historical outputs from audit runs.

Location:
- `docs/audits/reports/YYYY-MM/YYYY-MM-DD/<scope>.md`
- Reports must be grouped by month, then day directory.
- Each month directory must include `docs/audits/reports/YYYY-MM/summary.md`.

All reports must use the month/day layout.

## 2. Naming Conventions

Use these file patterns:
- Recurring definitions: `docs/audits/recurring/<domain>/<focus>.md`
- Reports (inside day directory): `<scope>.md`
- Same-day reruns for a scope: `<scope>-2.md`, `<scope>-3.md`, ...
- Required report directory: `docs/audits/reports/YYYY-MM/YYYY-MM-DD/`
- Required month summary: `docs/audits/reports/YYYY-MM/summary.md`

## 3. Audit Execution Discipline

For each audit run:
1. Use one audit definition per run.
2. Keep prompt scope fixed for the run.
3. Record findings with structured risk levels.
4. Save output as a new report file under `docs/audits/reports/YYYY-MM/YYYY-MM-DD/`.
5. Never overwrite prior run artifacts.

For crosscutting structure/velocity runs, include the required Hub Import Pressure metric:
- top imports for each hub module
- unique sibling subsystem import count
- cross-layer dependency count
- delta vs daily baseline report

### Daily baseline policy (mandatory)

For each `scope` on a given day:
- The first report file (`<scope>.md`) is the canonical daily baseline.
- Every same-day rerun (`<scope>-2.md`, `<scope>-3.md`, ...) must compare against `<scope>.md`.
- Do not chain comparisons across reruns (for example, `-3` must not compare against `-2`).
- Baseline resets on the next day.

Example:
- `docs/audits/reports/2026-03/2026-03-09/complexity-accretion.md` = baseline
- `docs/audits/reports/2026-03/2026-03-09/complexity-accretion-2.md` compares to baseline above
- `docs/audits/reports/2026-03/2026-03-09/complexity-accretion-3.md` compares to baseline above

### Required report preamble (every report)

Each report must include a short preamble block with:
- scope
- compared baseline report path:
  - first run of day: `N/A`
  - rerun: path to that day’s baseline file (`.../<scope>.md`)
- code snapshot identifier (for example `git rev-parse --short HEAD`, or `N/A`)
- method tag/version (for example `Method V3`)
- comparability status:
  - `comparable` (all tracked metrics use the same method), or
  - `non-comparable` (method changed, with one-line reason)

### Method-drift rule

If a metric formula, counting scope, or classification model changes:
1. bump the method tag in that report,
2. add a `Method Changes` section,
3. mark affected deltas as `N/A (method change)` instead of numeric deltas,
4. keep at least one unchanged anchor metric for continuity where possible.

### Verification readout discipline

Every report must include a `Verification Readout` section with command outcomes.

Allowed statuses:
- `PASS`
- `FAIL`
- `BLOCKED`

For `BLOCKED`, include a concrete reason.

### Actionability discipline

If any finding is `PARTIAL`/`FAIL`, or if overall risk index is `>= 6`, include explicit follow-up actions with:
- owner boundary
- action
- target report date/run

If no follow-up is required, state that explicitly.

## 4. Summary File Discipline

Each day report directory must include `summary.md`.
Each month report directory must include `summary.md`.

Day `summary.md` must contain:
- run contexts
- risk index summary
- method/comparability notes
- key findings by severity
- verification readout rollup
- follow-up action list (or explicit no-action statement)

Month `summary.md` must contain:
- index of included run days (`YYYY-MM-DD`)
- links to each day summary
- month-level status note (`complete` / `partial` / `blocked`)
- carry-forward follow-up list (or explicit no-action statement)

Day `summary.md` is append/update within that run day only; never rewrite prior day summaries.
Month `summary.md` is append/update within that month only; never rewrite prior month summaries.

## 5. History Preservation Rule

Audit history is append-only.

Required:
- No audit definition or report artifact may be deleted.
- Existing historical reports must remain accessible.
- If a relocation or rename collides with an existing filename, preserve the older artifact as `*_legacy.md`.

## 6. Required Governance Files

- `docs/audits/AUDIT-HOWTO.md`: operational process.
- `docs/audits/META-AUDIT.md`: architecture contract and dependency invariants.

## 7. Internal Linking Rule

Use normalized paths only:
- `docs/audits/recurring/...`
- `docs/audits/reports/...`

Do not reference deprecated locations in new reports.
