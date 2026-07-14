# Canic Audits

This directory separates current audit policy and reusable definitions from
retained historical evidence. Start here instead of browsing dated report
artifacts directly.

## Start Here

- To run or record an audit, read [AUDIT-HOWTO.md](AUDIT-HOWTO.md).
- To review the architecture invariants every audit applies, read
  [META-AUDIT.md](META-AUDIT.md).
- To select an active method or find its canonical owner, read
  [METHODS.md](METHODS.md).
- For repeatable auth and system audits, use [recurring/](recurring/README.md).
- For module-surface review or an explicitly requested cleanup, use
  [modular/](modular/README.md).
- For numbered release-line closeouts, use
  [release-lines/](release-lines/README.md).
- To find historical runs, use the [report archive](reports/README.md).

The [0.83 technical-debt ledger](0.83-technical-debt/README.md) is a retained
historical exception. New numbered-line audit material belongs under
`release-lines/` or the dated report archive.

## Directory Ownership

| Path | Purpose | Change policy |
| --- | --- | --- |
| `AUDIT-HOWTO.md` | Audit execution and storage rules | Maintained governance |
| `META-AUDIT.md` | Cross-audit architecture contract | Maintained governance |
| `METHODS.md` | Active method, disposition, trigger, and ownership catalog | One canonical active catalog |
| `method-fingerprints-v1.md` | Content identities for the prepared/frozen v1 method set | Regenerate deliberately when a method version changes |
| `retired-methods.md` | Immutable identity and replacement for hard-cut methods | Append-only retirement entries |
| `recurring/` | Reusable invariant and system definitions | Update deliberately; reports do not belong here |
| `modular/` | Module-surface policy and cleanup runner | Reusable playbooks only |
| `release-lines/` | Numbered-line closeout and program-state audits | Append-only primary reports |
| `reports/` | Dated audit reports and necessary supporting evidence | Append-only primary reports; generated evidence follows retention policy |
| `0.83-technical-debt/` | Historical 0.83 debt ledger | Frozen except for link repair |

## Retention Boundary

Historical Markdown reports are not deleted or rewritten to change their
findings during cleanup. Reproducible or duplicate generated evidence may be
pruned under the rules in [AUDIT-HOWTO.md](AUDIT-HOWTO.md). Several early
report days predate the current summary and artifact-discipline rules; the
[report archive](reports/README.md) records those gaps without inventing
retrospective conclusions.

New audit runs must keep the Markdown report as the primary evidence. Raw
artifacts are retained only when they are necessary for reproducibility or a
future comparison, and must follow the bounded artifact rules in
[AUDIT-HOWTO.md](AUDIT-HOWTO.md).
