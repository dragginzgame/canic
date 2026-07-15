# Mandatory End-To-End Trace Protocol

## Method Contract

- Audit ID: `CANIC-MANDATORY-TRACE-001`
- Method version: `1`
- Disposition: `retain`
- Owner: 0.92 cross-cutting end-to-end trace completeness and evidence
- Kind/profile: cross-method `invariant` plus versioned manual trace protocol
- Trace mode: every trace is a `code_trace` and must cite focused
  `execution_trace` evidence for observable success, rejection, boundary, and
  recovery behavior where the traced operation supports those states
- Cost/runtime: high; one to three hours per trace depending on PocketIC cost
- Prerequisites: immutable product snapshot, current method catalog, Git,
  ripgrep, targeted test runners, and PocketIC for canister execution paths
- False-positive boundary: this protocol owns end-to-end trace completeness;
  it does not replace a retained method, rescore an owner-specific invariant,
  or treat a supporting historical report as current trace execution
- Shared contract: [AUDIT-HOWTO.md](AUDIT-HOWTO.md)

This is a cross-cutting protocol, not a twenty-third retained audit
definition. The 22 retained methods remain the canonical property owners. The
protocol gives the ten mandatory 0.92 traces one reproducible method identity,
common completion rule, and report shape.

## Required Trace Set

The protocol runs exactly these trace IDs from the accepted 0.92 design:

| Trace ID | Required path |
| --- | --- |
| `TRACE-DEPLOY-001` | configuration -> build -> release-set manifest -> install/upgrade -> deployment truth |
| `TRACE-AUTH-001` | root provisioning -> chain-key/delegation proof -> issuer readiness -> token verification |
| `TRACE-CAPABILITY-001` | endpoint guard -> capability/attestation -> replay -> workflow -> mutation |
| `TRACE-CYCLES-001` | cycles funding and ICP refill -> ledger/notify outcome -> retry identity -> CLI projection |
| `TRACE-INTENT-001` | local and receipt-backed admission -> accounting -> settlement -> replay -> upgrade recovery |
| `TRACE-CONTROL-001` | control-plane and Wasm-store publication -> lookup -> reconciliation -> conflicting release rejection |
| `TRACE-TOPOLOGY-001` | scaling, sharding, topology, pool, parent, and subnet binding |
| `TRACE-BLOB-001` | blob request -> billing/cashier -> authorization -> persistence -> recovery |
| `TRACE-BACKUP-001` | plan -> journal -> snapshot -> checksum -> manifest -> restore preview/execute/recovery |
| `TRACE-LIFECYCLE-001` | restore -> timer scheduling -> user hooks -> stable state and metrics projection |

Adding, removing, merging, or changing a required path changes this method's
version and fingerprint.

## Frozen Sampling And Inspection Rules

For every trace:

1. start from each public/operator entrypoint that can initiate the named
   operation; list excluded entrypoints with evidence that they cannot reach
   the operation;
2. follow reachable calls directly in current source through admission,
   planning, execution, persistence, recovery, diagnostics, and public
   projection as applicable;
3. name the canonical owner of every decision, transition, record conversion,
   external call, retry/replay identity, and projection;
4. inspect every early return and error conversion on the sampled path; a
   typed cause that becomes an untyped error is recorded at that exact edge;
5. identify the authoritative persisted record/store and the upgrade,
   interrupted-operation, restore, or explicit no-persistence boundary;
6. inspect focused tests by exact test name or exact nonempty filter. A zero-test
   successful Cargo invocation is invalid evidence;
7. require executable positive, rejection, boundary, and recovery/regression
   evidence when those states are part of the operation. If safe execution is
   unavailable, record `partial` or `blocked`; and
8. preserve owner-specific findings under their existing canonical IDs. A
   trace may add a finding only for a newly observed owner/invariant defect.

Source search is navigation evidence, not completion evidence. A trace is not
complete until its report maps current entrypoints and transitions to exact
files/functions and records the corresponding focused execution evidence.

## Execution And Safety

- Product code and the immutable product snapshot remain read-only.
- Network is disabled unless the trace explicitly requires and authorizes a
  disposable non-production target; production/mainnet mutation is forbidden.
- Canister execution uses PocketIC or another named disposable environment.
- Build, state, and `CARGO_TARGET_DIR` outputs live outside the source tree.
- Pre-run and post-run `git status --porcelain` and the canonical product-tree
  hash are recorded.
- A tracked product-source mutation invalidates the run.
- Retained evidence must not contain credentials, tokens, private material,
  environment secrets, sensitive principals, or unnecessary private paths.

## Required Report Shape

One dated primary report may contain all ten traces, provided every trace has
an independent template and verdict:

```text
trace_id:
trace_method_id:
trace_method_version:
trace_method_fingerprint:
mode:
entrypoint:
caller_and_auth_context:
canonical_owner_per_transition:
persistent_state_touched:
external_calls:
retry_and_replay_identity:
rejection_paths:
partial_failure_paths:
upgrade_or_restore_boundary:
diagnostic_projection:
public_projection:
tests_or_evidence:
unreviewed_boundaries:
verdict:
findings:
```

The primary report also records the complete immutable run identity from
`AUDIT-HOWTO.md`, command/cwd/exit evidence, reviewer identity, exact source
sample, and pre/post product-tree hashes.

## Trace And Aggregate Verdicts

Per-trace `verdict` uses the shared `run_result` enum:

- `pass`: the full required path and its applicable failure/recovery states
  are traced and current evidence finds no violated invariant;
- `fail`: the full required path is traced and current evidence confirms one
  or more owner-specific defects;
- `partial`: one or more required transitions, projections, or applicable
  executable evidence classes are incomplete;
- `blocked`: a prerequisite or safety boundary prevents the trace; or
- `not_applicable`: allowed only when committed configuration and source
  reachability prove the conditional operation is absent from the product.

A confirmed product defect produces `fail`, not `partial`, when the trace is
otherwise complete. Missing evidence never becomes `pass` or `fail`.

The aggregate run is:

- `blocked` if any trace is blocked;
- otherwise `partial` if any trace is partial;
- otherwise `fail` if any trace fails; or
- otherwise `pass`.

All ten trace results must be valid before the 0.92 mandatory-trace gate is
complete. An aggregate `fail` permits finding-backed product work; an
aggregate `partial` or `blocked` does not.

## Comparability And Method Defects

The comparison baseline is the exact `v0.92.0` product snapshot. A method
defect follows the post-freeze correction protocol in `AUDIT-HOWTO.md`: mark
affected trace results invalid, increment and fingerprint this protocol, and
rerun the corrected method against the original product baseline before using
it for closeout.

For later product slices, each affected trace is compared both with its
immediate parent and with the original `v0.92.0` trace.

## Follow-Up Ownership

- Trace completeness or protocol defects belong to
  `CANIC-MANDATORY-TRACE-001`.
- Product, evidence, governance, operational, and documentation findings
  remain with the canonical retained-method owner named in `METHODS.md`.
- Any P0/P1 finding based on manual trace judgment requires maintainer review
  plus a second reviewer or an explicit single-review waiver before closeout.
