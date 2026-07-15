# Audit: Instruction Footprint

## Method Contract

- Audit ID: `CANIC-INSTRUCTION-001`
- Method version: `2`
- Disposition: `retain`
- Owner: local WebAssembly instruction measurement and critical-flow checkpoint coverage
- Kind/profile: `measured` plus observability invariant
- Trace mode: `execution_trace` in isolated PocketIC and `code_trace` for checkpoint ownership
- Cost/runtime: high; 30-90 minutes depending on artifact cache state
- Prerequisites: pinned PocketIC, the `canic-tests` root harness, the Canic-validated artifact builder, Rust/Cargo, Git, and GNU coreutils
- False-positive boundary: local instruction totals are pressure evidence, not correctness failures; install checkpoint groups are not endpoint totals; missing required rows or invalid build authority is a method failure
- Shared contract: [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)

## Purpose And Authority

This method measures local canister WebAssembly instructions for a fixed set
of maintained update and installation flows. It also proves whether important
multi-stage flows have stable `perf!` checkpoints. It does not measure remote
canister execution, message fees, payload/storage charges, management-call
fees, garbage collection, or total cycle billing.

Authority is singular:

- `scripts/ci/instruction-audit-report.sh` owns run identity, isolation,
  evidence manifests, and the executable composite fingerprint;
- `crates/canic-tests/tests/instruction_audit_support/scenarios.rs` owns the
  exact scenario roster;
- `execution.rs` owns fixture setup, measured calls, and perf deltas;
- `report.rs` owns checkpoint discovery, normalized output, comparison, and
  the deterministic score; and
- Canic's root test harness and `build_artifact` path own artifact creation and
  role validation. This audit may not build role Wasm directly with Cargo.

Changing any of those inputs changes the method fingerprint. Changing scope,
scenario meaning, counter semantics, checkpoint parsing, comparison, score,
or artifact authority requires a method-version increment.

## Fixed V2 Scenario Roster

Every scenario gets a fresh smallest applicable root-harness topology. Setup
and prerequisites happen before the measured call. No mutable PocketIC state
is shared between scenario rows.

| Scenario key | Origin | Required behavior |
| --- | --- | --- |
| `scale:request_cycles_from_parent:fresh` | update | child-to-parent structural capability round trip |
| `scale_hub:create_worker:empty-pool` | update | scaling observation, planning, creation, and registration |
| `user_hub:create_account:new-principal` | update | sharding assignment and shard allocation |
| `root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer` | update | explicit first delegation-proof provisioning |
| `issuer:canic_prepare_delegated_token:active-proof` | update | issuer token preparation from an active proof |
| `test:test_verify_delegated_token:valid-delegated-token` | update | verifier confirmation of a freshly issued delegated token |
| `root:canic_response_capability_v1:request-cycles-fresh` | update | fresh capability, policy, and execution path |
| `root:canic_response_capability_v1:request-cycles-replay` | update | identical second request returns the cached replay response |
| `root:canic_template_stage_manifest_admin:single-chunk` | update | stage one approved manifest |
| `root:canic_template_prepare_admin:single-chunk` | update | prepare one staged single-chunk release |
| `root:canic_template_publish_chunk_admin:single-chunk` | update | publish the prepared chunk |
| `root:bootstrap:init-checkpoints` | install | observe retained root-bootstrap checkpoints after a fresh install |

Query and composite-query instruction totals are outside v2. Adding either
requires an authoritative same-call measurement fixture and a new method
version. The runner must not substitute a post-query shared-metrics read.

## Measurement Semantics

The measured source is `performance_counter(1)`. Canonical rows record:

- `subject_kind` and `subject_label`;
- call count;
- total and average local instructions;
- exact scenario key and dimensions;
- caller/principal scope;
- `sample_origin` (`update` or `install`); and
- optional instruction-only cycle estimates, clearly separated from measured
  values.

Update rows are the matching persisted
`[perf, endpoint, update, endpoint_name]` row delta between immediately before
and immediately after the sampled call. Every required update row must have
`count > 0`. A zero exclusive endpoint total remains valid when the call's
work is attributed to nested/checkpoint scopes; the report retains that zero
and its checkpoint deltas instead of treating it as a missing call.

The installation row is the sum of non-zero retained root checkpoint rows
whose labels start with `bootstrap_`. It has count one, origin `install`, and
is a checkpoint-group observation rather than an endpoint total. At least one
matching checkpoint is required. Missing or zero required measurement aborts
the run without a primary result.

Instruction-only cycle estimates are optional decoration. They never replace
the measured local-instruction fields and do not affect the result or score.

## Checkpoint Coverage

The executable scanner visits current Rust files under `crates/`, recognizes
literal and namespaced single-line `perf!(` invocations, and ignores quoted
examples and line comments. Multiline invocation syntax is outside v2 and
requires a method-version change. Search commands in reports or reviews are
navigation aids, not alternative counters.

Coverage is evaluated for these seven exact flow classes and source owners:

| Flow class | Required source owner |
| --- | --- |
| root capability dispatch | `crates/canic-core/src/workflow/rpc/request/handler/mod.rs` |
| root proof provisioning | `crates/canic-core/src/workflow/runtime/auth/provisioning` |
| issuer delegated-token prepare and verification | `crates/canic-core/src/workflow/runtime/auth/prepare` |
| replay/cached response | `crates/canic-core/src/workflow/rpc/request/handler/replay.rs` |
| sharding assignment | `crates/canic-core/src/workflow/placement/sharding` |
| scaling/provisioning | `crates/canic-core/src/workflow/placement/scaling/mod.rs` |
| bootstrap/install/publication | `crates/canic-control-plane/src/workflow/bootstrap/root.rs` |

A flow class passes coverage when at least one scanned checkpoint starts with
its required source owner. A missing class is `partial` evidence, not zero
cost and not an inferred correctness defect. Measured checkpoint deltas are
retained separately from static coverage.

## Deterministic Risk Score

The score uses three disjoint inputs:

| Input | Score |
| --- | ---: |
| no comparable v2 predecessor | 2 |
| one missing critical-flow checkpoint class | 1 |
| two or more missing critical-flow checkpoint classes | 2 |
| highest average local instruction row exceeds 2,000,000 | 2 |

Otherwise each input contributes zero. Sum and cap at 10. No reviewer
modifier, estimate, absolute Wasm size, or correctness finding changes this
score.

Result rules:

- `blocked`: authoritative fixtures, PocketIC, or required prerequisites
  cannot execute;
- `partial`: the run completes but any critical checkpoint class or measured
  checkpoint evidence is absent;
- `fail`: complete evidence and risk score 7-10;
- `pass`: complete evidence and risk score 0-6; and
- `invalid`: the method identity is wrong, source mutates, a required call row
  is missing, the bootstrap checkpoint sum is zero, or output is presented
  after the runner failed.

The first valid v2 run is a non-comparable baseline; that fact adds risk but
does not by itself make the result partial or blocked.

## Required Evidence

Run:

```bash
bash scripts/ci/instruction-audit-report.sh
```

Retain the primary report and compact support artifacts:

- scenario manifest;
- canonical perf rows;
- measured checkpoint deltas;
- checkpoint coverage gaps;
- verification readout;
- method and environment identities; and
- evidence manifest with command, exit code, timestamps, tool versions, and
  SHA-256 artifact hashes.

The runner uses an isolated temporary `CARGO_TARGET_DIR`, forbids the `ic`
environment, records pre/post source status, and fails if anything outside its
report paths changes. Retained evidence must contain no credentials, tokens,
private material, or machine-specific repository root.

## Comparison And Findings

Compare only reports with the same method ID, version, fingerprint, scenario
key, and sample origin. The first valid v2 report has `N/A` deltas. Later runs
show causal comparison to their immediate compatible predecessor; release
closeout also compares cumulatively to the original v2 baseline. A missing or
zero denominator is `N/A`, never an invented percentage.

A high instruction count is trend evidence. Create a product finding only for
a distinct active-authority violation, an observed regression with a traced
cause, or a required observability gap. Deduplicate checkpoint gaps into the
canonical flow owner. Remediation must be finding-backed and may not introduce
aliases, compatibility layers, duplicate instrumentation, or unrelated
redesign.
