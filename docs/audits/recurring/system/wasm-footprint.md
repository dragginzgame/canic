# Audit: Wasm Footprint

## Method Contract

- Audit ID: `CANIC-WASM-001`
- Method version: `5`
- Disposition: `revise`
- Owner: canonical Canic-produced Wasm size and retained-size attribution
- Kind/profile: `measured` and `trend`
- Trace mode: `execution_trace` in an isolated local build environment
- Cost/runtime: high; normally 60-180 minutes
- Prerequisites: a clean disposable product worktree, Rust/Cargo with the Wasm
  target, pinned ICP CLI helpers, `ic-wasm`, Binaryen 132 `wasm-opt`, `twiggy`,
  isolated `.icp` state, and isolated clean Cargo targets
- False-positive boundary: method, roster, toolchain, profile, or execution-path
  drift makes results non-comparable; size pressure is not a correctness defect
  until attributed to an owned invariant
- Shared contract: [AUDIT-HOWTO.md](../../AUDIT-HOWTO.md)

## Purpose

Measure the Wasm artifacts Canic actually produces for installation, qualify
the governed release optimizer across every role, prove two-clean-build output
determinism, compare release and debug, and attribute retained-size pressure
without creating a competing build path.

This audit does not prove runtime correctness, authorize feature removal, or
replace `CANIC-BUILD-INTEGRITY-001` reproducibility evidence.

## Canonical Artifact Authority

The host `build_artifact` example is the single executable authority used by
this audit. It enters `build_workspace_canister_artifact`, applies Canic's
profile, shrink, metadata, Candid, required release optimization, provenance,
code-limit, and gzip rules, and copies the result to the ICP-visible artifact
path. Optimization is complete before any canonical gzip or artifact identity
is derived.

V5 retains v4's removal of the direct Cargo Wasm build and inferred
"pre-shrink" artifact. It consumes the release transform's governed
before/after metrics without retaining the pre-optimization module as a second
artifact. The runner must not recreate it with direct
`cargo build --target wasm32-unknown-unknown`, copy a target-directory Wasm,
or bypass the build guard.

V5 also binds each role build to its own captured log and parses the governed
optimizer metric record independently of the builder's path-confined staging
filename. The metric shape, not a disposable `candidate.wasm` path or the
later final artifact path, is the authority-bearing contract.

The authoritative measured classes are therefore:

- two independently built canonical release `.wasm`, `.wasm.gz`, and `.did`
  sets whose bytes must match exactly;
- governed pre/post optimization measurements for raw, deterministic gzip,
  code-section, data-section, and defined-function size/count;
- canonical debug `.wasm` and builder-produced `.wasm.gz`; and
- `ic-wasm`/`twiggy` analysis of the canonical release `.wasm`.

No alias, fallback, reconstructed pre-transform path, or duplicate gzip path is
part of v5.

## Fixed Scope

The configured part of the default and release-baseline roster is the attached
role set returned by:

```text
bash scripts/ci/list-config-canisters.sh \
  --config <product-root>/fleets/test/canic.toml --ci-order
```

At method admission the ordered roster is:

```text
app
test
user_hub
scale_hub
user_shard
scale_replica
root
fleet_coordinator
wasm_store
```

`fleet_coordinator` and `wasm_store` are appended as the two separately built
infrastructure roles after the configured role order. `root` is classified as
Fleet Subnet Root infrastructure; the two appended roles are Fleet Coordinator
and Wasm Store infrastructure. Every other role is an application Component.
There is no dedicated minimal role in this roster. Repeated retained hotspots
across at least three Components are the shared fan-in signal.

V5 retains v4's corrected nine-role roster and separately built Coordinator
and Store. V3 results lack optimizer and clean-build evidence, so they are
non-comparable and cannot baseline v5. V4 results use the same artifact
authority but are non-comparable because their optimizer-log parser cannot
consume the current path-confined staged artifact record.

V5 always captures three isolated build runs:

- two `release` runs, each using a fresh Cargo target and generated artifact
  tree, whose canonical Wasm, gzip, Candid, and optimization metrics must match;
- `release`, the shipping/install authority represented by the first matching
  run; and
- `debug`, a diagnostic comparison built through the same authority.

Fast or role-scoped investigations are development measurements, not a run of
this retained method.

## Execution Contract

Preferred command:

```text
WASM_AUDIT_PRODUCT_ROOT=/tmp/canic-wasm-audit-product \
  bash scripts/ci/wasm-audit-report.sh
```

The named product root must be a clean disposable linked Git worktree at the
snapshot being audited. The method checkout may be a separate maintainer
checkout so a corrected method can rerun an immutable historical product.

Optional control:

- `WASM_AUDIT_DATE=YYYY-MM-DD` pins the UTC report date.

There is no skip-build or cache-reuse mode. Every retained v4 run builds fresh
artifacts through the canonical builder with network access disabled. Build
output may create `.icp/` in the disposable product worktree and an external
temporary Cargo target. Any tracked product mutation or unexpected untracked
path fails the run.

The runner rejects `ICP_ENVIRONMENT=ic`, uses `local`, never contacts a replica,
never uses production credentials, and performs no deployment or destructive
authoritative operation.

## Immutable Identity And Comparability

Each run records the immutable fields required by `AUDIT-HOWTO.md`, plus:

- ordered roster key;
- two-clean-release/debug profile key;
- product execution-path key;
- exact external-tool key; and
- root-independent executable composite.

The executable composite hashes the content of the definition and named audit
scripts while labeling each digest with its repository-relative path. Absolute
checkout paths must not enter the composite.

A predecessor is compatible only when all of these match exactly:

- audit ID, version, and executable composite;
- roster and profile keys;
- execution-path key; and
- external-tool key.

The first valid v5 run records `N/A` deltas. Later runs compare causally to the
immediate compatible predecessor and retain the original v5 baseline identity
for cumulative release-line comparison. A missing or zero denominator is
`N/A`, never an invented percentage.

The execution-path key matters because `CANIC-092-BUILD-001` confirms that
absolute build paths currently enter root output. Raw byte size is still a
valid snapshot, but gzip or byte-for-byte comparisons across different paths
would be misleading.

## Required Measurements

For every role record:

- release Wasm and gzip bytes;
- exact before/after optimizer raw, gzip, code-section, data-section, and
  defined-function measurements;
- exact Wasm, gzip, and Candid identity across two isolated clean release builds;
- release-transform proof that the export inventory, derived IC Wasm features,
  and embedded public Candid metadata are unchanged;
- debug Wasm and gzip bytes;
- debug-minus-release byte and percentage delta;
- compatible-predecessor release byte and percentage delta, when available;
- release function count, data-section count/bytes, and exported-method count;
- largest shallow and retained `twiggy top` entries;
- bounded `twiggy dominators` evidence; and
- bounded `twiggy monos` evidence.

Retain one canonical size TSV, one optimizer TSV, one determinism TSV, one
aggregate summary, one detail file per role, one method identity JSON file,
and one evidence manifest. Raw Wasm, Cargo output,
and complete tool dumps are transient and must not enter the report archive.
All retained snippets redact repository, home, cache, credential, principal,
token, and private-material paths or values.

## Exact Result And Risk Rules

Required tools, builds, artifacts, and analyses are fail-closed. A missing
role, failed canonical build, missing release/debug artifact, missing or wrong
Binaryen, changed export/Candid/feature contract, nondeterministic clean build,
failed `ic-wasm`/`twiggy` command, source mutation, or unverifiable evidence hash makes
the run `blocked`; such a run cannot support a baseline.

For a complete run, add these disjoint inputs and cap at 10:

| Input | Score |
| --- | ---: |
| no compatible v4 predecessor | 2 |
| largest/smallest Component release ratio is 1.10-1.2499 | 1 |
| largest/smallest Component release ratio is at least 1.25 | 2 |
| root/max-Component release ratio is 2.0-2.9999 | 1 |
| root/max-Component release ratio is at least 3.0 | 2 |
| largest compatible release growth is 5.0-9.9999% | 1 |
| largest compatible release growth is at least 10.0% | 2 |
| largest retained item is 10.0-24.9999% of its release Wasm | 1 |
| largest retained item is at least 25.0% of its release Wasm | 2 |

The ratio alternatives in each pair are mutually exclusive. Negative or
missing predecessor growth adds zero.

- `pass`: complete evidence and risk 0-6;
- `fail`: complete evidence and risk 7-10;
- `partial`: not used for a missing required measurement; required gaps are
  fail-closed as `blocked`; and
- `not_applicable`: not valid for the fixed release roster.

A high first-baseline score is trend pressure, not automatically a product
finding. Create a finding only when evidence identifies a violated canonical
authority, unexplained comparable regression, or operational limit. Preserve
typed build/tool causes in blocked evidence.

## Required Report Sections

Every report contains:

1. verdict, validity, comparability, and exact score;
2. scope, immutable product/method/tool identity, timestamps, and safety state;
3. canonical artifact size matrix and compatible deltas;
4. governed optimizer before/after measurements and two-clean-build determinism;
5. release/debug comparison;
6. `ic-wasm` structure evidence;
7. `twiggy` shallow, retained, dominator, and monomorphization evidence;
8. Component spread, repeated fan-in signals, and separate infrastructure interpretation;
9. findings or an explicit no-new-finding statement;
10. checklist and command verification; and
11. retained artifact links and hashed evidence manifest.

Exact diagnostic text is asserted only where it is a documented
operator-facing contract. Internal failure evidence preserves the typed tool or
build cause instead of treating display strings as the authority.

## Method-Change Rule

V1 is preserved as invalid history and v2/v3 as valid superseded history. If
v4's artifact authority, fixed roster, build-run set, metric derivation,
comparison key or score changes, increment
the method version and apply the post-freeze method-defect protocol. Compare
only results produced by the corrected method.
