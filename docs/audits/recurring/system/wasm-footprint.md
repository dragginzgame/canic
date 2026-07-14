# Audit: Wasm Footprint

Method: `wasm-footprint-v2`

## Purpose

Track wasm footprint drift over time and identify size drivers in Canic canister
artifacts.

This is a build-artifact audit.

It is NOT:

- a correctness audit
- a feature-design audit
- a runtime performance audit

The job of this audit is to measure shipped wasm output, explain where the
bytes live, and identify the largest retained-size drivers with artifact tools
such as `ic-wasm` and `twiggy`.

This audit is not permission to delete intended behavior to make the numbers
look better.

## Why This Audit Is Canic-Specific

Canic is not a single-canister product.

The workspace ships a family of fleet and fixture canisters with two structural
properties that change how a wasm audit must work:

1. ICP CLI plus Canic build scripts own the supported canister build path.
2. `root` remains a special control-plane outlier because it embeds the
   bootstrap `wasm_store.wasm.gz` artifact and carries the thin-root install
   boundary, so it must still be evaluated separately from normal leaf
   canisters.

An audit copied directly from another repo will miss both of these facts and
will produce misleading comparisons.

## Risk Model / Invariant

This is a drift audit, not a correctness invariant audit.

Risk model:

- growing shared runtime cost silently taxes every canister artifact
- `root` bundle growth can hide child-canister regressions and can become a
  deployment/install bottleneck on its own
- large retained-size hotspots are expensive to optimize unless attribution is
  specific and repeatable

Optimization constraint:

- reduce wasm without removing intended runtime capabilities or operator-facing signal
- do not count feature removal as a normal wasm optimization win
- any feature-removal proposal requires a separate explicit design decision outside this recurring audit
- when a shipped wire type derives `CandidType`, prefer `//` comments over `///` because Rust doc attributes are retained in Candid runtime metadata and can silently grow every canister artifact

## Run This Audit After

- release hardening windows
- major auth/runtime/macro feature lines
- dependency bumps affecting `candid`, crypto, or IC/CDK crates
- changes to `icp.yaml`, the host `build_artifact` builder, or workspace
  release profile
- any PR explicitly claiming wasm-size reduction

## Report Preamble (Required)

Every report generated from this audit must include:

- Scope
- Definition path
- Compared baseline report path
- Code snapshot identifier
- Method tag/version
- Comparability status
- Auditor
- Run timestamp (UTC)
- Branch
- Worktree
- Profile
- Target canisters in scope

## Scope

Measure and report:

- raw built wasm size (`built .wasm`) from direct Cargo canister builds
- deterministic gzip of the raw built wasm (`built .wasm.gz`) as secondary context
- canonical shrunk wasm size (`shrunk .wasm`) from the supported ICP CLI/Canic build flow
- deterministic gzip of the shrunk wasm (`shrunk .wasm.gz`) as secondary context
- raw debug/dev wasm size (`wasm-debug built .wasm`) for comparison against the audit profile
- optional debug/dev deterministic gzip (`wasm-debug built .wasm.gz`) when captured by the runner
- shrink deltas between built and shrunk artifacts
- current shrunk-wasm bytes per canister in the audited scope
- debug-vs-audit deltas between `wasm-debug` and the audited profile
- `ic-wasm info` structure snapshots for built and shrunk artifacts
- `twiggy` breakdowns (`top`, retained `top`, `dominators`, `monos`) for hotspot attribution

### Default Canister Scope

Default scope is the attached role set returned by
`scripts/ci/list-config-canisters.sh --config fleets/test/canic.toml --ci-order`.
As of this audit definition, that is:

- `app`
- `user_hub`
- `user_shard`
- `scale_hub`
- `scale_replica`
- `root`

The audit runner must resolve each role through `[roles.<role>].package` and
then read the actual Cargo package name from that package manifest. It must not
guess `canister_<role>` or infer package identity from role names.

### Default Profile

- profile: `release`

The recurring audit must still compare the audited profile against `wasm-debug`
artifacts for the same canisters.

Reason:

- `release` remains the shipping/install authority
- `fast` is the middle shrunk local/test/demo lane and is worth auditing when
  local/operator cost is the question rather than shipping cost
- `wasm-debug` is the fastest way to see whether a regression is coming from
  optimization-sensitive codegen/linking or from real surface-area growth
- large debug-vs-release gaps are diagnostic signals and must be tracked, not ignored

Profile mapping:

- `release` -> Cargo `--release`
- `fast` -> Cargo `--profile fast`
- `wasm-debug` -> Cargo debug build

## Canic Artifact Model (Mandatory)

Use these artifact classes consistently:

### Shared Baseline Rule

If an explicit attached minimal/baseline role exists in the audited scope, use
it as the shared Canic runtime floor. If no such role is attached, the audit
must say so and use repeated retained hotspots across leaf canisters as the
shared fan-in signal.

The audit must not print `minimal = N/A` as if that were a valid baseline.
Missing baseline evidence is a report condition, not a size signal.

### Built Artifact

The built artifact is the direct Cargo output before post-processing:

- `target/wasm32-unknown-unknown/<profile>/<cargo-package-name>.wasm`

This is the primary baseline for "what the Rust build emitted before canister
post-processing".

### Canonical Shrunk Artifact

The canonical shrunk artifact is the ICP CLI-visible Canic build output:

- `.icp/local/canisters/<name>/<name>.wasm`

This is the primary baseline for "what Canic would actually ship/install via
its normal canister build flow".

### Deterministic Gzip Artifact

For both built and shrunk artifacts, record deterministic gzip output using:

- `gzip -n`

These `.wasm.gz` values are secondary continuity metrics.
They do NOT decide optimization success on their own.

### Root Bundle Rule

`root` must always be called out separately because it embeds the bootstrap
`wasm_store.wasm.gz` artifact and carries the control-plane/install boundary, so
it is still not comparable one-to-one with leaf canisters.

Required:

- identify `root` as `bundle-canister`
- compare `root` against its own prior baselines first
- avoid using `root` alone to judge shared-runtime regressions in leaf canisters
- use an explicit attached baseline role as the shared-runtime floor when one
  exists, and `root` as the bundle ceiling

## Decision Rule

- raw non-gzipped wasm is the optimization authority
- use built `.wasm` and shrunk `.wasm` as the primary pass/fail and drift metrics
- compare the audited profile against `wasm-debug` on the same day as a
  secondary diagnostic, not as the release decision metric
- record deterministic gzip artifacts for transport continuity only
- run `twiggy` on a name-preserving analysis artifact when possible so hotspot
  attribution remains readable

## Required Checklist

For each run, explicitly mark `PASS` / `PARTIAL` / `FAIL` with concrete evidence.

1. Wasm artifacts were built or loaded from cache for each target canister/profile in scope.
2. Artifact sizes were recorded in a machine-readable artifact.
3. `twiggy top` output was analyzed for offender ranking and summarized.
4. `twiggy dominators` output was analyzed for retained-size ownership and summarized.
5. `twiggy monos` output was analyzed for generic bloat signal and summarized.
6. Baseline path was selected according to Canic daily baseline discipline.
7. `wasm-debug` artifacts were captured or the run explicitly marked them `BLOCKED`.
8. Debug-vs-audit size deltas were recorded when comparable debug artifacts exist.
9. Current per-canister size snapshots were recorded in the top-level report and machine-readable artifact.
10. Size deltas versus baseline were recorded when comparable baseline artifacts exist.
11. Verification readout includes command outcomes with `PASS` / `FAIL` / `BLOCKED`.
12. New `CandidType` wire types were checked for `///` / `//!` doc-comment regressions when shared data-section growth is under review.

## Execution Contract

Preferred command:

- `bash scripts/ci/wasm-audit-report.sh`

Optional controls:

- `WASM_AUDIT_DATE=YYYY-MM-DD` to pin the report day path
- `WASM_AUDIT_SKIP_BUILD=1` to reuse cached artifacts under `artifacts/wasm-size/`
- `WASM_CANISTER_NAME=<name>` to scope to a single canister
- `WASM_PROFILE=release|fast|wasm-debug`

Recurring-run rule:

- a normal dated audit run must audit `release`
- the same dated run must also capture `wasm-debug` built artifacts for profile
  comparison; `scripts/ci/wasm-audit-report.sh` reports this as `Method V2`
- a report that lacks `wasm-debug` comparison must call that out explicitly as `PARTIAL` or `BLOCKED`

Optional scope note:

- If `WASM_CANISTER_NAME=root`, the audit runner may still build child bundles
  required to compile `root`.

## Output Contract

Canic follows the repository-wide audit history rules in
`docs/audits/AUDIT-HOWTO.md`.

That means:

- first run of day uses `docs/audits/reports/YYYY-MM/YYYY-MM-DD/wasm-footprint.md`
- same-day reruns use numbered variants such as `wasm-footprint-2.md`
- per-run artifacts live under a matching per-run artifact directory

Per-run artifact directory:

- `docs/audits/reports/YYYY-MM/YYYY-MM-DD/artifacts/<scope-stem>/`

Transient reusable build cache:

- `artifacts/wasm-size/<profile>/`

Required artifacts for each run:

- aggregated size report JSON (`size-report.json`)
- compact baseline metrics (`size-metrics.tsv`)
- aggregated size summary markdown (`size-summary.md`)
- debug/profile comparison markdown or table artifact when `wasm-debug` is available
- per-canister detailed markdown (`<canister>.md`)

Raw `ic-wasm info` and `twiggy` output is transient analysis input. Extract its
structure and hotspot evidence into the aggregate and per-canister reports; do
not retain parallel text and CSV copies in the report archive.

## Structural Hotspots (Required)

Every report generated from this audit must include:

- concrete artifact outliers by canister
- at least one retained-size hotspot table grounded in `twiggy`
- explicit note when `root` growth is dominated by embedded child bundles
- explicit note when feature canisters remain close to an attached baseline
  role, or when no dedicated baseline role is attached
- explicit comparison between `wasm-debug` and the audited profile, or an
  explicit `BLOCKED` note explaining why that comparison is absent

## Risk Score (Required)

Use the normalized `0-10` scale.

Suggested interpretation for this audit:

- `0-2`: stable, no meaningful drift
- `3-4`: minor drift, mostly attributable and low-risk
- `5-6`: moderate drift, shared baseline or bundle pressure rising
- `7-8`: high drift, large unowned hotspots or root/install risk emerging
- `9-10`: severe drift, artifact growth is blocking release posture

Default multi-canister scope is normal for Canic and must not raise the risk
score by itself. Raise risk for observed drift signals such as leaf spread,
same-day growth, missing required evidence, or root bundle growth that crosses
the configured outlier threshold.

## Early Warning Signals (Required)

Reports must watch for:

- an attached baseline role approaching the same size class as more
  feature-heavy canisters, or missing baseline evidence when no baseline role is
  attached
- `root` growing faster than the sum of child bundle changes
- shrink delta collapsing unexpectedly, which can mean dead code is becoming live
- `twiggy monos` showing repeated generic expansion in shared crates
- `ic-wasm info` function-count growth without corresponding feature growth

## Dependency Fan-In Pressure (Required)

This audit must call out shared crates or subsystems when retained-size evidence
suggests they tax most or all canisters, for example:

- `canic-core`
- `candid` and DTO glue
- auth / crypto support
- logging / metrics runtime
- lifecycle / macro runtime

## Verification Readout (Required)

Every report must include command outcomes with:

- `PASS`
- `FAIL`
- `BLOCKED`

`BLOCKED` must include a concrete reason, such as:

- missing `icp`
- missing `ic-wasm`
- missing `twiggy`
- missing cached artifacts for `WASM_AUDIT_SKIP_BUILD=1`

## Method Notes

Method anchors for this audit:

1. built and shrunk raw wasm bytes are the primary trend metrics
2. deterministic gzip bytes are secondary transport context
3. `twiggy` attribution should prefer a name-preserving analysis artifact
4. `root` is always interpreted as a bundle canister, not as a normal leaf peer

If any of these rules change, bump the method tag and mark affected deltas as
`N/A (method change)`.
