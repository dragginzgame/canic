# Audit: Security Boundary Ordering

## Method Contract

- Audit ID: `CANIC-AUTH-ORDERING-001`
- Method version: `2`
- Disposition: `revise`
- Owner: cross-stage auth, replay, capability, proof-install, and dispatch order
- Kind/profile: security `invariant` plus manual end-to-end ordering trace
- Trace mode: `code_trace`; named rejection paths may use PocketIC execution
- Cost/runtime: medium/high; 45-90 minutes excluding PocketIC fixtures
- Prerequisites: Git, ripgrep, current auth invariant definitions, and focused
  rejection tests where static order is insufficient
- False-positive boundary: this method owns sequencing only; individual trust,
  subject, audience, capability, and replay properties remain with their auth
  invariant owner and are not rescored here
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

Trace admission before replay reservation and handler effects. Infrastructure
calls bind the raw transport caller to exact receiver-local authority. Attestation
checks verify the signature and subject/audience/subnet/time bindings before
trusting the declared role. Caller-controlled proof fields cannot choose a trust
anchor. Refusal reaches neither business mutation nor paid effects.

For Root delegation renewal, trace configured policy and issuer admission before
batch preparation, intent recording before paid signing, response reconciliation
before installation, and proof verification before publishing active state. Stale
callbacks must not replace newer authority; retry-after and bounded spending
remain enforced. Inspect replay and cost guards at their owning effect boundary.

For retained session storage, trace complete validation before durable writes and
publication of heap indexes. Restoration must reject inconsistent account/session
bindings. Storage qualification does not authorize exposing unfinished portable
admission endpoints.

## Focused Evidence

Run the owning Root proof, replay-policy, access and session-storage regressions.
Use a named installed case where caller or management-call ordering cannot be
established natively. Report exact refusal/effect evidence and any recovery gap;
this method does not claim to audit the upstream cryptographic implementation.

## Report

Include scope, source identity, method version/fingerprint, compared baseline,
trace paths, focused evidence, findings, residual risk and owner/action. Classify
results as PASS, FAIL, PARTIAL or BLOCKED with reasons. This revision follows the
current product surface; historical reports retain their original product and
method identities and are not relabeled or directly compared as current evidence.
