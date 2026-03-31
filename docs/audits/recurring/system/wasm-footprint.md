# Audit: Wasm Footprint

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

The workspace ships a family of reference canisters with two structural
properties that change how a wasm audit must work:

1. `dfx` is the canonical canister builder and shrink/gzip owner.
2. `root` is a bundle canister that embeds child `.wasm.gz` artifacts via
   `include_bytes!`, so it must be evaluated as a special outlier rather than
   as a normal peer to leaf canisters.

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

## Run This Audit After

- release hardening windows
- major auth/runtime/macro feature lines
- dependency bumps affecting `candid`, crypto, or IC/CDK crates
- changes to `dfx.json`, `scripts/app/build.sh`, or workspace release profile
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
- canonical shrunk wasm size (`shrunk .wasm`) from `dfx build`
- deterministic gzip of the shrunk wasm (`shrunk .wasm.gz`) as secondary context
- raw debug/dev wasm size (`wasm-debug built .wasm`) for comparison against the audit profile
- optional debug/dev deterministic gzip (`wasm-debug built .wasm.gz`) when captured by the runner
- shrink deltas between built and shrunk artifacts
- debug-vs-audit deltas between `wasm-debug` and the audited profile
- `ic-wasm info` structure snapshots for built and shrunk artifacts
- `twiggy` breakdowns (`top`, retained `top`, `dominators`, `monos`) for hotspot attribution

### Default Canister Scope

Default scope is the full reference canister set in `dfx.json`:

- `app`
- `minimal`
- `user_hub`
- `user_shard`
- `scale_hub`
- `scale`
- `test`
- `root`

### Default Profile

- profile: `wasm-release`

The recurring audit must still compare the audited profile against `wasm-debug`
artifacts for the same canisters.

Reason:

- `wasm-release` remains the shipping/install authority
- `wasm-debug` is the fastest way to see whether a regression is coming from
  optimization-sensitive codegen/linking or from real surface-area growth
- large debug-vs-release gaps are diagnostic signals and must be tracked, not ignored

Profile mapping:

- `wasm-release` -> Cargo `--release`
- `wasm-debug` -> Cargo debug build

## Canic Artifact Model (Mandatory)

Use these artifact classes consistently:

### Minimal Baseline Rule

`minimal` is the canonical minimal leaf-canister baseline for wasm audits.

This is a locked audit assumption:

- `minimal` exists to measure the shared Canic runtime floor
- `minimal` must remain on the standard non-root Canic runtime surface
- `minimal` must not accumulate role-specific helpers beyond that shared surface
- `minimal` must not accumulate provisioning helpers, RPC helpers, or other bespoke behavior
- if `minimal` changes meaning, the audit definition is wrong until it is explicitly revised

When comparing leaf canisters, interpret size above `minimal` as role-specific
addition on top of the shared runtime floor.

### Built Artifact

The built artifact is the direct Cargo output before `dfx` shrinking:

- `target/wasm32-unknown-unknown/<profile>/canister_<name>.wasm`

This is the primary baseline for "what the Rust build emitted before canister
post-processing".

### Canonical Shrunk Artifact

The canonical shrunk artifact is the `dfx build` output:

- `.dfx/local/canisters/<name>/<name>.wasm`

This is the primary baseline for "what Canic would actually ship/install via
its normal canister build flow".

### Deterministic Gzip Artifact

For both built and shrunk artifacts, record deterministic gzip output using:

- `gzip -n`

These `.wasm.gz` values are secondary continuity metrics.
They do NOT decide optimization success on their own.

### Root Bundle Rule

`root` must always be called out separately because it embeds child
`.wasm.gz` bundles and is therefore not comparable one-to-one with leaf
canisters.

Required:

- identify `root` as `bundle-canister`
- compare `root` against its own prior baselines first
- avoid using `root` alone to judge shared-runtime regressions in leaf canisters
- use `minimal` as the shared-runtime floor and `root` as the bundle ceiling

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
3. `twiggy top` output was captured for offender ranking.
4. `twiggy dominators` output was captured for retained-size ownership.
5. `twiggy monos` output was captured for generic bloat signal.
6. Baseline path was selected according to Canic daily baseline discipline.
7. `wasm-debug` artifacts were captured or the run explicitly marked them `BLOCKED`.
8. Debug-vs-audit size deltas were recorded when comparable debug artifacts exist.
9. Size deltas versus baseline were recorded when comparable baseline artifacts exist.
10. Verification readout includes command outcomes with `PASS` / `FAIL` / `BLOCKED`.

## Execution Contract

Preferred command:

- `bash scripts/ci/wasm-audit-report.sh`

Optional controls:

- `WASM_AUDIT_DATE=YYYY-MM-DD` to pin the report day path
- `WASM_AUDIT_SKIP_BUILD=1` to reuse cached artifacts under `artifacts/wasm-size/`
- `WASM_CANISTER_NAME=<name>` to scope to a single canister
- `WASM_PROFILE=wasm-release|wasm-debug`

Recurring-run rule:

- a normal dated audit run must audit `wasm-release`
- the same dated run must also capture `wasm-debug` built artifacts for profile comparison
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
- per-canister size report JSON (`<canister>.size-report.json`)
- aggregated size summary markdown (`size-summary.md`)
- debug/profile comparison markdown or table artifact when `wasm-debug` is available
- per-canister detailed markdown (`<canister>.md`)
- built `ic-wasm info` snapshot (`<canister>.built.ic-wasm-info.txt`)
- shrunk `ic-wasm info` snapshot (`<canister>.shrunk.ic-wasm-info.txt`)
- `twiggy top` text (`<canister>.twiggy-top.txt`)
- `twiggy top` CSV (`<canister>.twiggy-top.csv`)
- retained-size CSV (`<canister>.twiggy-retained.csv`)
- `twiggy dominators` text (`<canister>.twiggy-dominators.txt`)
- `twiggy monos` text (`<canister>.twiggy-monos.txt`)

## Structural Hotspots (Required)

Every report generated from this audit must include:

- concrete artifact outliers by canister
- at least one retained-size hotspot table grounded in `twiggy`
- explicit note when `root` growth is dominated by embedded child bundles
- explicit note when feature canisters remain close to `minimal`, because that
  signals shared-runtime baseline pressure
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

## Early Warning Signals (Required)

Reports must watch for:

- `minimal` approaching the same size class as more feature-heavy canisters
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

- missing `dfx`
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
