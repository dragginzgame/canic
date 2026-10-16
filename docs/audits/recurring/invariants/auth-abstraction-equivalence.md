# Audit: Auth Abstraction Equivalence Invariant

## Method Contract

- Audit ID: `CANIC-AUTH-EQUIVALENCE-001`
- Method version: `3`
- Disposition: `revise`
- Owner: equivalence between supported auth abstractions and generated paths
- Kind/profile: security `invariant`
- Trace mode: `code_trace`; focused execution when generated/manual paths
  cannot be proven statically
- Cost/runtime: medium; 30-60 minutes
- Prerequisites: Git, ripgrep, macro expansion/source, facade/runtime paths,
  and equivalent positive/rejection fixtures
- False-positive boundary: deliberately different credential types need equal
  required properties, not byte-identical implementation
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

Generated endpoint guards preserve the same caller, role, audience, subnet,
validity and epoch checks as the canonical attestation and caller-authority
implementation. Explicit infrastructure authority remains separate from any
represented application account.

Trace the endpoint parser, validator and access expansion through
`access::expr` to `access::auth::attestation` and caller-authority evaluation.
Check bounded decoded proof arguments, short-circuit composition, default Fleet
guards, denial mapping and handler exclusion. Custom guards stay synchronous.
Review generated Root and Blob endpoints against their declared access rules;
a macro wrapper must not broaden controller or service authority.

## Focused Evidence

Run the affected macro endpoint tests and facade endpoint/protocol tests. For
attestation changes, select the owning Core attestation tests and, when needed,
the exact installed rejection case. Compare equivalent valid and refused inputs,
including wrong caller, Root, audience, subnet, role, expired proof and stale
epoch. Do not substitute a syntax-only check for cryptographic proof acceptance.

## Report

Include scope, source identity, method version/fingerprint, compared baseline,
trace paths, focused evidence, findings, residual risk and owner/action. Classify
results as PASS, FAIL, PARTIAL or BLOCKED with reasons. This revision follows the
current product surface; historical reports retain their original product and
method identities and are not relabeled or directly compared as current evidence.
