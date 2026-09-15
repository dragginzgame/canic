# ICP 1.5 integration implementation — 2026-09-15

## Outcome

The maintainer accepted the [integration audit](icp-1.5.0-integration.md) fixes
and improvements, then selected read-only frontend post-sync verification.
The accepted ICP work is implemented in the open 0.110.17 draft. Package versions
remain 0.110.16. Publication and the complete release batch retain their separate gates.

## Delivered behavior

- **Build selection:** the artifact helper uses `ICP_CLI_ENVIRONMENT`, accepts
  explicit standalone selection and rejects conflicting authority. Compiler input
  still uses the normalized Canic network class. Explicit environment declarations
  override implicit names correctly.
- **Config readiness:** bounded structural YAML parsing replaces line scanning.
  Omitted and empty membership differ; App checks reject excluded required roles.
  Unresolved external manifests/dependencies and ambiguous declarations fail with
  typed causes. Canic's complete build closure remains separate from ICP selection.
- **Current CLI contract:** the minimum supported ICP CLI is 1.5.0. Current process
  fixtures preserve public/partial status, management version fallback, balance,
  snapshot and identity behavior. Incomplete query statistics reject rather than
  becoming zero counters.
- **Inspection:** `inspect management` exposes visibility and cumulative query
  statistics. Allowed viewers never become controller authority. Partial public
  status retains unavailable fields.
- **Local process cost:** successful version qualification is shared within one
  `IcpCli` context and its clones for typed calls. Fresh contexts, changed working
  directories and failed probes recheck. No remote authority or balances are cached.
- **Frontend readback:** `frontend verify-uploaded` binds the retained digest,
  environment, asset Principal and network root trust. It verifies actual uploaded
  bytes and lengths through bounded `get`/`get_chunk` queries, including manifest
  bytes and the root-level II alternative-origins asset. Immediate repeats have no
  canister mutation effect. The existing uploader retains upload ownership.

Operator documentation: [ICP integration](../../../../features/operations/icp-integration.md)
and [frontend handoff](../../../../features/operations/frontend-handoff.md).

## Focused evidence

Evidence is retained under [artifacts/icp-1.5.0-implementation](artifacts/icp-1.5.0-implementation/).

| Check | Result |
| --- | --- |
| Changed host/CLI module regressions | 105 CLI and 80 host tests pass; two existing Node.js-dependent cases remain ignored |
| Updated Fleet process fixtures | Six selected native regressions pass, including retained-estate replay |
| Artifact helper environment precedence | Three tests pass |
| Recursive CLI help and example limits | Both integration tests pass |
| Host/CLI Clippy, all targets/features | Pass with warnings denied |
| Internal-testing Clippy, all targets/features, no dependency linting | Pass with warnings denied; resolves the recorded transient error-size blocker |
| Native ICP 1.5 selection/bundle probe | All six observations pass |

These are targeted checks, not complete workspace validation. The frontend
readback comparison/rejection/repeat evidence uses a query-reader boundary; an
actual asset-canister upload and HTTP/browser delivery were not exercised here.
Final cleanup after those checks changes only type documentation and handoff/evidence
files; targeted formatting and whitespace checks pass.

## Boundaries

The post-sync integration is a host script appended after the existing recipe's
sync steps. It is not a portable WASI plugin; ICP's experimental bundler rejects
script sync steps. Capacity review must still run before upload. Readback is not
an atomic snapshot across all files and does not establish HTTP certification,
CORS, caching or a browser Internet Identity ceremony.

Offline management signing, asset-upload ownership, bundle distribution and
Coordinator-backed backup remain outside this delivery. No broad gate, version
bump, Git publication, deployment or external-repository mutation was performed.
The complete open release batch is not push-ready while its independently tracked
funding/recovery and B1 work remain outstanding.
