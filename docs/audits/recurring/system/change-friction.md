# Audit: Change Friction

`canic-core`

## Method Contract

- Audit ID: `CANIC-CHANGE-FRICTION-001`
- Method version: `2`
- Disposition: `retain`
- Owner: empirical edit blast radius and decision-axis friction
- Kind/profile: `trend` plus manual attribution
- Trace mode: `code_trace`
- Cost/runtime: medium; 30-60 minutes for the frozen sample and current-tree evidence
- Prerequisites: Git history, Bash, GNU awk/coreutils, the frozen v2 sample, and the canonical complexity and layering methods
- False-positive boundary: formatting, generated output, release sweeps, tests, and mechanical movement are classified explicitly; pressure alone is not a correctness defect
- Shared contract: [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)

## Purpose And Authority

This method measures how widely functional changes spread across the current
`canic-core` architecture. It owns historical feature-slice blast radius,
decision-axis amplification, feature locality, and their combined velocity
risk. It does not own correctness, dependency-direction validity, or general
complexity findings.

Authority is deliberately singular:

- `CANIC-LAYERING-001/v2` and `scripts/ci/run-layering-guards.sh` own current
  boundary violations. This method consumes their exact result and does not
  implement another import parser.
- `CANIC-COMPLEXITY-001/v2` and
  `docs/audits/scripts/measure-complexity-v2.sh` own current-tree file scope,
  reference-file counts, LOC, and hub evidence. This method does not recount
  those signals.
- This definition and `measure-change-friction-v2.sh` own the sample,
  per-slice classifications, formulas, percentiles, and final friction score.

Pressure may support an existing canonical product finding. It creates a new
product finding only when the traced product violates an active authority; a
high score by itself is a trend result, not a defect.

## Frozen Population

The exact five-row population is
`docs/audits/fixtures/change-friction-v2-sample.tsv`. Each row contains a full
commit, its exact first parent, slice type, stable label, and a complete list
of named flow axes. The runner requires all commits to be ancestors of the
audited full source commit.

The population is the first v2 baseline and must not be replaced to improve a
result. Future reports rerun the population for comparability. A new release
window requires a method-version change and a superseding fixture. A
`release_sweep` remains measurable but is reported separately from routine
`feature_slice` results.

Scope per row is the sorted set of Rust files changed between the exact parent
and commit under `crates/canic-core/src/**`. File additions, deletions, and
renames count by their Git path result. Other crates, documentation, generated
artifacts, and non-Rust files are outside this method.

Run the canonical measurement as:

```bash
bash docs/audits/scripts/measure-change-friction-v2.sh <full-source-commit>
```

Changing the script or fixture is a method-version change. A valid report
retains the fingerprints and normalized stdout digest and verifies identical
normalized output in two runs.

## Exhaustive Classification

Test classification takes precedence. A file is `test-support` when any path
component is `test` or `tests`, or the basename is `test.rs`, `tests.rs`, or
`test_support.rs`. It has class and layer `test`; it counts toward files,
subsystems, locality, and containment but not behavior-layer count.

Production files map by first path component:

| Subsystem | Path scope | Behavior layer |
| --- | --- | --- |
| access | `access/**` | endpoints |
| api | `api/**` | endpoints |
| bootstrap | `bootstrap/**` | workflow |
| cdk | `cdk/**` | ops |
| config | `config/**` | policy |
| dispatch | `dispatch/**` | endpoints |
| domain | `domain/policy/**` | policy |
| domain | other `domain/**` | model-storage |
| dto | `dto/**` | endpoints |
| format | `format/**` | endpoints |
| ids | `ids/**` | model-storage |
| infra | `infra/**` | ops |
| ingress | `ingress/**` | endpoints |
| lifecycle | `lifecycle/**` | workflow |
| memory | `memory/**` | model-storage |
| model | `model/**` | model-storage |
| ops | `ops/**` | ops |
| replay-policy | `replay_policy/**` | policy |
| role-contract | `role_contract/**` | policy |
| storage | `storage/**` | model-storage |
| view | `view/**` | model-storage |
| workflow | `workflow/**` | workflow |

Root files form subsystem `root` and map exactly as follows:

- endpoints: `control_plane_support.rs`, `protocol.rs`, `lib.rs`;
- ops: `log.rs`, `perf.rs`; and
- model-storage: `error.rs`, `memory_macros.rs`, `shared_support.rs`,
  `state_contract.rs`.

The complete denominator is 23 subsystems, including `root` and
`test-support`. An unknown first component or root file makes the run
`partial`, invalidates its score, and requires a method-version update. Files
are never classified by reviewer preference.

## Frozen Metrics

A flow axis is one fixture-named condition that changes reachable control
flow across behavior layers. Duplicate spellings within a row are forbidden.
Changing an axis list changes the fixture and method version.

For each slice:

- `files` is the total classified file count;
- `subsystems` is the number of distinct classified subsystems;
- `layers` is the number of distinct non-test behavior layers;
- `CAF = max(subsystems, layers) * flow_axes`;
- `density = files / subsystems`;
- `ELS = files_in_primary_subsystem / files`;
- `locality = files_in_primary_module / files`, where module means the
  directory containing the relative Rust path and root files use `root`;
- `containment = subsystems / 23`; and
- primary ties resolve lexicographically by subsystem or module name.

Aggregate file statistics include `feature_slice` rows only. Average is the
arithmetic mean. Median is the middle ordered value, or the arithmetic mean of
the two middle values. P95 is the nearest-rank value at `ceil(0.95 * n)`.
Release sweeps, if a future method version admits them, receive a separate
aggregate and never alter routine feature values.

## Current-Tree Inputs

### Enum shock

Use the non-test reference-file counts emitted by
`measure-complexity-v2.sh` for `Request`, `Response`, `CapabilityProof`,
`BuiltinPredicate`, and `RootCapability`. For each type, multiply its declared
variant count at the exact source commit by its reference-file count. The
maximum product is `enum_shock`. Variant counts come directly from the named
enum declaration; aliases, constructors, displays, serialization helpers, and
test-only variants do not add variants. The report records all five rows and
their definition paths.

### Boundary leakage

Run `scripts/ci/run-layering-guards.sh` against the exact audited product tree.
Its exit 1 is a product finding, not a method failure. `leakage_files` is the
number of unique stdout rows matching a repository-relative production Rust
path under `crates/canic-core/src/ops/**`; diagnostics and `path:line` search
matches do not count. Exit 2, missing fixtures, or unclassifiable output makes
this run `partial`. The canonical layering report owns the resulting defect.

### Gravity wells

Use the exact `CANIC-COMPLEXITY-001/v2` strict-hub evidence: a production file
with logical LOC greater than 600 and at least three of that method's fixed
domain categories. `strict_hubs` is the retained exact file count. This method
does not reinterpret domain membership or issue a second hub finding.

## Deterministic Risk Score

Score each disjoint input once:

| Bucket | Input | Score |
| --- | --- | --- |
| enum shock | `<=5`, `6-15`, `16-30`, `>30` | 2, 4, 6, 8 |
| sampled CAF | `<=4`, `5-8`, `9-16`, `17-24`, `25-32`, `>32` | 1, 3, 5, 7, 8, 10 |
| boundary leakage | `0`, `1-5`, `6-15`, `16-30`, `>30` files | 1, 4, 6, 8, 10 |
| gravity wells | `0`, `1`, `2-3`, `>=4` strict hubs | 1, 4, 6, 8 |
| edit blast | p95 files `<=3`, `4-5`, `6-10`, `11-20`, `>20` | 1, 3, 5, 7, 9 |

Apply weights enum shock 3, sampled CAF 2, boundary leakage 2, gravity wells
2, and edit blast 1:

```text
index = (3*enum + 2*caf + 2*leakage + 2*gravity + blast) / 10
```

Round half up to the nearest integer. This rounded 1-10 value is the only
Velocity Risk Index. No growth, hub, CAF, fan-in, capability, or reviewer
modifier may be added after it.

- `pass`: index 1-4 and complete evidence;
- `fail`: index 5-10 and complete evidence;
- `partial`: any required evidence, classification, or exact source identity
  is absent; and
- `blocked`: the source history or owning prerequisite cannot execute.

Trend failure records architectural pressure; it does not authorize broad
refactoring or imply a correctness failure.

## Required Evidence And Validation

Every report records the shared run identity and evidence manifest, method,
script, fixture, complexity-runner, and layering-guard fingerprints, both
normalized-output digests, all per-slice rows, all file classifications, the
five enum-shock rows, exact strict-hub set, canonical leakage result, five
bucket scores, weighted arithmetic, and focused tests for the highest-CAF
maintained flow.

Use `docs/audits/scripts/run-nonempty-cargo-test.sh` for each named Cargo
filter so a zero-test selection is a method error. Tests prove maintained
behavior only; they do not lower a pressure score.

## Finding And Comparison Rules

The first valid v2 run is a non-comparable baseline. Later runs provide both
immediate-parent causal comparison and original-v2 cumulative comparison.
No percentage delta is shown when either denominator is zero or either result
was produced by a different method fingerprint.

Canonical layering, complexity, and correctness owners receive overlapping
evidence. This method may create a change-friction product finding only for a
distinct active-authority violation. Remediation must be finding-backed and
must not introduce generic frameworks, aliases, compatibility paths, or
unrelated redesign.
