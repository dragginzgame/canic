# Audit overlay: Canic CI and release integrity

## Method Contract

- Audit ID: `CANIC-RELEASE-INTEGRITY-001`
- Method version: `3`
- Disposition: `retain`
- Owner: Canic release authority, artifact custody and declared host/target proof
- Kind/profile: release `invariant` plus named manual review
- Trace mode: `code_trace`; effectful probes require separate execution authority
- Prerequisites: selected scope/source, current workflows and release/package inputs
- False-positive boundary: examples and inert fixtures are not usable credentials
- Shared contract: [common audit contract](../../../../audits/README.md)
- Shared method: [CI and release integrity](../../../../audits/ci-release-integrity.md)
- Snapshot revision: `4cb9bc907154225a3f7e4c33fd8d75fbeaa125c8`

Apply the shared method for trust traces, artifact-promotion proof, host evidence,
findings and verdicts. This overlay retains Canic's scope and product obligations;
it adds no release gate or authority to install, publish or deploy.

## Canic entrypoints and artifact custody

Trace the selected paths through [CI](../../../../.github/workflows/ci.yml),
[security](../../../../.github/workflows/security.yml), [Make](../../../../Makefile),
the [release adapter](../../../../scripts/release/adapter.sh) and
[workspace publication](../../../../scripts/ci/publish-workspace.sh).
[CI/deployment governance](../../../governance/ci-deployment.md) owns the actual
validation, package, release and effect boundaries; audit guidance cannot replace
those commands or their release receipts.

Include generated Fleet Root/Store/Coordinator and application/component Wasm,
selected interface metadata, native Host/CLI outputs and Cargo packages when the
chosen path promotes them. Trace raw, shrunk and compressed bytes to the artifact
actually admitted by the release index, Wasm Store, package upload or Fleet
installation. Keep source/lock/toolchain/features, checksums and destination bound
to that exact selection; a matching artifact name is insufficient.

Preserve credential/log custody, controller/network selection, explicit spending
and publication authority, intent-before-effect records, uncertain-result
reconciliation and retained bytes across same-release retries. Canic's pre-1.0
reinstall and cycle-conservation rules remain owned by [AGENTS.md](../../../../AGENTS.md).
Do not infer cross-release compatibility or permission to repeat a paid effect.

## Canic evidence and historical identity

Record the declared Linux/macOS architecture and Wasm/native target cells from
the selected workflows and product contracts, distinguishing local, hosted,
maintainer-only and unsupported paths. Missing required proof remains a gap;
this overlay changes no supported-host claim or product qualification gate.
Build reproducibility and measured instruction/Wasm evidence keep their existing
method owners.

Retain Canic's run/source/snapshot/method identities, selected proof commands,
unreviewed boundaries and report destination under
`docs/audits/reports/YYYY-MM/YYYY-MM-DD/`. Local manual-review, P0/P1 review,
retention and release-line closeout obligations remain in
[AUDIT-HOWTO.md](../../AUDIT-HOWTO.md) and [META-AUDIT.md](../../META-AUDIT.md).
Use shared authority/input and artifact/host proof instead of a second generic
checklist or report template.

The [exact v2 definition](https://github.com/dragginzgame/canic/blob/cf2d3d1fba84a00a76cdbc541ae7e68fe36e9bb5/docs/audits/recurring/system/release-integrity.md)
and its fingerprint remain historical; prior reports are unchanged. New runs
identify v3 and the recorded snapshot. Affected comparisons are
`N/A (method change)`. This delegation does not prove a defect in earlier results
or execute a new product audit.
