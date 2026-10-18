# Audit: Canonical Auth Boundary Invariant

## Method Contract

- Audit ID: `CANIC-AUTH-BOUNDARY-001`
- Method version: `2`
- Disposition: `revise`
- Owner: authenticated entrypoint convergence and endpoint verifier boundary
- Kind/profile: security `invariant`
- Trace mode: `code_trace`; focused rejection execution may use PocketIC
- Cost/runtime: medium; 30-60 minutes
- Prerequisites: Git, ripgrep, endpoint macro/access/auth code, generated
  endpoints, and bypass rejection fixtures
- False-positive boundary: private token-material/proof helpers are not
  endpoint authorization when every reachable endpoint still converges on the
  full boundary
- Shared contract: [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)

## Current Scope

Audit transport/topology guards, receiver-local caller authority, Root proof
provisioning and signed role attestations. Portable application session admission
is unfinished and must refuse. Root proof readiness and retained session storage
are not evidence of a working application login service.

Use [authentication architecture](../../../architecture/authentication.md) and
[access contracts](../../../contracts/ACCESS_ARCHITECTURE.md) for the maintained
boundaries. Follow the shared report and finding rules in
[AUDIT-HOWTO.md](../../AUDIT-HOWTO.md).

## Invariant and Trace

Every reachable guarded endpoint executes its declared admission checks before
handler effects. Trace exported wrappers through access evaluation to the
canonical attestation or caller-authority verifier. Root provisioning requires
its explicit controller/registered-caller authority; submitted proof material
cannot select the trusted Root or expand configured grants.

Inspect `crates/canic/src/macros/endpoints/role.rs`, the endpoint macro access
expansion, `crates/canic-core/src/access/` and the called auth ops. Confirm that
proof helpers are not alternate public admission paths. Supplied role attestations
bind the transport caller and receiving subnet; endpoints separately enforce the
required role. Root installation verifies configured key policy before storage.

## Focused Evidence

Select endpoint macro, public protocol, role-attestation and caller-authority
regressions for the paths changed. Record exact source and selected feature graph,
positive/refusal outcomes and any unexecuted installed case. Portable Auth's
unfinished integration stays an explicit completion gap in issue #491.

## Report

Include scope, source identity, method version/fingerprint, compared baseline,
trace paths, focused evidence, findings, residual risk and owner/action. Classify
results as PASS, FAIL, PARTIAL or BLOCKED with reasons. This revision follows the
current product surface; historical reports retain their original product and
method identities and are not relabeled or directly compared as current evidence.
