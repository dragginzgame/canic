# CLI Command Audit

Date: 2026-09-07
Scope: `canic-cli` command declarations, dispatch, rendered help, source/runtime
ownership, backup/restore entrypoints and active operator documentation.

## Verdict

The current top-level families have distinct maintained responsibilities. The
confirmed redundant leaf was `restore apply`, a dry-run renderer and optional
journal writer overlapping `restore prepare` and `restore run --dry-run`.
It has been hard-cut, including its parser, dispatch, output helpers and tests.
No alias replaces it.

`restore prepare` owns artifact validation and creation or adoption of pristine
recovery documents. `restore run` owns execution, preview and interruption
recovery. Removing the alternate journal writer preserves this ownership and
avoids a second preparation path that could replace a journal. The underlying
backup library's operation/journal types remain necessary to the maintained
runner; their names are not obsolete CLI commands.

## Corrections

- Backup and restore now share one backup-reference resolver. Previously,
  restore sorted readable manifests by directory name while `backup list`
  used creation time and included incomplete/invalid layouts. A displayed row
  could consequently select a different backup for restoration.
- Empty, zero and overflowing numeric backup references fail with a typed
  reference error instead of selecting the first row.
- Auth renewal and blob-storage subcommand help now succeeds without required
  operands. Medic Fleet help renders its own command, including inherited
  options. Diagnostic and inspection usage include their complete invocation.
- The recursive help integration test now checks successful exit, stdout-only
  output and matching command usage before checking ordering and example
  limits. Previously a mismatched page silently stopped traversal.
- `restore status` forwards the selected ICP executable and environment to
  its next-command preview, consistently with `restore run`.
- Active CLI documentation describes the maintained restore flow and current
  Fleet authority without a catalogue of removed deployment commands.

## Retained Command Ownership

| Family | Maintained purpose |
| --- | --- |
| `admission` | Plan, apply and inspect Coordinator-owned Fleet ingress policy. |
| `app` | Manage source Apps, configuration and role declarations/attachments. |
| `auth` | Inspect an issuer's Root-managed delegation renewal state. |
| `backup` | Capture, inspect, verify and prune backup layouts; inspect capture progress. |
| `blob-storage` | Inspect billing readiness, fund it and synchronize gateways for the current subsystem. |
| `build` | Produce App and infrastructure artifacts with build provenance. |
| `cycles` | Inspect balances/funding and perform explicit transfers, topups and conversions. |
| `diagnostic` | Explain a structured diagnostic code offline. |
| `evidence` | Compare and gate stable evidence envelopes. |
| `fleet` | Generate desired state and converge it through Fleet Ensure. |
| `info` | Query terminal Fleet inventory, environment, endpoints, metrics and subnet information. |
| `inspect` | Obtain protected runtime inspection for a Principal or Fleet role. |
| `medic` | Aggregate actionable workspace or Fleet readiness diagnostics. |
| `network` | Enroll canonical network trust identities. |
| `replica` | Start, inspect and stop the local replica. |
| `restore` | Plan, prepare, preview/execute/recover, and inspect snapshot restoration. |
| `scaffold` | Create a source canister package and declare it. |
| `state` | Inspect declared source state metadata and its manifest. |
| `status` | Show quick local workspace status. |
| `token` | Resolve Fleet targets for token balance and transfer operations. |
| `toolchain` | Install release tools using authoritative checksums. |

Similar names do not imply duplicated ownership: `app role declare` registers
an existing package while `scaffold canister` creates one; `cycles mint`
converts operator-held ICP while `cycles convert` uses Root-held ICP;
`backup manifest validate` checks the portable manifest while `backup verify`
checks the complete layout and bytes. Local `status`, focused runtime
`inspect` and aggregate `medic` answer different operator questions.

Blob-storage extraction and canonical infrastructure packaging remain separate
future slices. Removing their working commands ahead of those changes would
strand current functionality. Historical changelogs and archived audits are
historical evidence, not accepted parser surfaces.

## Validation And Limits

Targeted CLI regression and help checks are recorded after completion below.
This is a command-surface and source-ownership audit, not fresh qualification
of every paid/live operation. No deployment, broad validation, publication or
version change is part of this slice. The unrelated test-throughput batch
retains its own qualification and readiness decision.
