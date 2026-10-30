# Backup removal audit — 2026-10-29

## Verdict and scope

PASS for removal of the private Backup engine and old CLI. No active
`canic-backup` crate, dependency, command, compatibility wrapper, copied engine,
publication entry or package override remains. Two stale design references were
corrected. This is a repository removal audit, not qualification of live backup,
restore, fresh hosted runners or the whole release batch.

Requested after the maintainer explicitly approved the immediate hard cut and
then separately authorized a documentation-only retirement publication. Method:
`audits/code-hygiene.md`, using reviewed Shared Tooling 0.27.6 revision
`5a72359b5be19d5221334cf2c9b3204621d3f3f6` and local AGENTS.md. Source baseline:
`b9ed7f298bdaf8c9cceac142edf7e56b15d33208`, with the preserved uncommitted batch.
Current inspected input hashes: `target/backup-removal-audit/audit-inputs.sha256`.

## Evidence by boundary

| Boundary | Finding |
| --- | --- |
| Source and CLI | Removed crate and both CLI trees absent; no former engine type/runner consumers. Command registration, dispatch, global forwarding and orphaned helpers removed. |
| Cargo | Fresh locked/offline resolved metadata contains no removed package or dependency edge; upstream Backup/Agent 0.49.1 remain registry dependencies. No local override to a replacement engine. |
| CI and publication | No retired package in workflows, publication roster, metadata checks or downstream package staging/patches. Host checkpoint CI is retained. Stubbed publication/adoption and isolated package-output fixtures previously passed. |
| Documentation | Current feature/architecture/CLI/governance docs state live unavailability. Corrected an ideas document's owner and a closed 0.110 design's dangling persistence link, explicitly retaining historical context. |
| Runtime and adapters | Host checkpoint driver, Agent port, recovery adapter, Contracts, authenticated Root/participant checkpoint hooks, runtime fences and installed fixtures intentionally remain. These bind product authority/application facts to upstream accounting; they are not the removed generic engine. |
| Retained artifacts | Archived source, build output, release artifacts, prior backup data and journals remain evidence. They are not active workspace/publication inputs and were not deleted or converted into new authority. |

A full package-name search initially found 2,673 matches: 2,403 in audit history,
171 in changelogs, 40 in archived status, 57 in design records, and two in current
availability/status documentation. The non-archived design matches were the two
corrected references. Historical evidence was not bulk-rewritten. Supplemental
searches covered old engine type names and CLI invocations across crates, scripts,
workflows, Make/CI, applications, canisters and integration consumers.

Fresh checks: locked/offline full metadata, publish-manifest boundary and diff
whitespace pass. No broad suite ran. Prior hard-cut evidence includes 13 CLI tests,
help and ordering tests, strict lint and packaging/publication fixtures under
`target/backup-hard-cut/`. Root Cargo.toml/Cargo.lock hashes have since changed as
part of concurrent dependency work; those passes are not claimed for the new
complete graph. The current audit inspected that graph without altering it.

## Removal size

Baseline is HEAD above, limited to the three retired directories; this avoids
counting unrelated dirty work or presenting gross deletion as net whole-batch size.

| Deleted tree | Files | Physical lines |
| --- | ---: | ---: |
| crates/canic-backup | 105 | 22,095 |
| crates/canic-cli/src/backup | 66 | 5,007 |
| crates/canic-cli/src/restore | 10 | 3,340 |
| Total | 181 | 30,442 |

cloc 2.10 counts 179 Rust files: **25,457 code lines**, 1,932 comment lines and
2,931 blank lines (30,320 Rust physical lines). The remaining 122 physical lines
are the removed crate manifest/README. Counts include tests. Additional helper,
Cargo, CI and documentation cleanup is excluded from these conservative figures;
new retained Host integration is not subtracted. Raw counts and baseline extraction
are under `target/backup-removal-audit/`.

## Crates.io retirement publication

Published **canic-backup 0.112.1** via the explicitly approved one-off retirement
path outside the workspace. The source came from registry 0.112.0, verified against
its index checksum `62174660608a9429b8338127b87c0551dbfe3e446be59a104c8ca0baf032c42e`.
Rust source and dependency declarations are unchanged. Changes are the package
version, deprecation description/README, maintenance metadata and corresponding
root package lock identity. Cargo-generated manifest metadata is regenerated.

Crates.io does not display the old Cargo maintenance badge. The visible deprecation
is conveyed through the published description and README, as documented by the
[Cargo manifest reference](https://doc.rust-lang.org/cargo/reference/manifest.html#the-badges-section).
No existing versions were yanked. The normal workspace version and roster remain
unchanged; the retired crate was not reintroduced.

Initial dry-run refused the archive's generated reserved Cargo.toml.orig; removing
that generated input allowed Cargo to regenerate it. Verification then passed in
12.66s. Actual publication reverified in 1.50s and confirmed registry availability.
A fresh Cargo download independently confirmed the description, README and identical
Rust source. Published checksum:
`b839abc0f343e6680034000d5f10dbc4d74de450f5a777658dfd784b579cc8be`.
Logs, source comparison inputs and registry records are retained in
`target/backup-removal-audit/`; the isolated publication directory is
`/tmp/canic-backup-deprecation-01121/`.

## Remaining work

The hard cut and retirement notice are complete. Production live operator
integration and acceptance remain #513/#394/#550. Historical activation conflict,
fresh hosted/macOS evidence and other release blockers remain separate. No live
backup/restore, deployment, workspace release, commit or push occurred in this audit.
