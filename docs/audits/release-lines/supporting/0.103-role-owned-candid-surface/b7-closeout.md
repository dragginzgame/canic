# 0.103 B7 Hard-Cut Closeout

Date: 2026-08-18

This is unreleased worktree evidence. It records the current 0.103 hard cut but
does not claim a release tag or immutable commit that does not yet exist.

## Method Reduction

The immutable source is the accepted `v0.102.2` B1 manifest. Current counts
come from the post-B5 canonical services in `b6-surface-report.md`.

| Representative profile | Baseline Canic methods | Current Canic methods | Removed |
| --- | ---: | ---: | ---: |
| Fleet Subnet Root auth fixture | 117 | 2 | 115 |
| managed auth fixture | 23 | 2 | 21 |
| Fleet Coordinator | 24 | 2 | 22 |
| Wasm Store | 24 | 4 | 20 |
| **Total appearances** | **188** | **10** | **178** |

The current ordinary role budget is command plus status. Store additionally
retains only its independently admitted chunk read and publish lanes. External
standards and application-owned methods remain separately counted. The current
four-profile total is 26 methods including those separate owners, compared
with 207 in the B1 capture.

## Representative Fast-Profile Wasm

These artifacts were rebuilt after legacy emitter deletion through the
repository-owned canonical builder. Gzip hashes cover the exact published
builder output; raw hashes cover its decompressed Wasm.

| Role | Raw bytes | Raw SHA-256 | Gzip bytes | Gzip SHA-256 |
| --- | ---: | --- | ---: | --- |
| managed Component | 3,384,861 | `504458f66d1d8a8bd62e18a11daf641be50dda57c3292c55328f3f30e3efb361` | 883,267 | `0ccd0d33ecfbea9c57c87b83e3a37f7a022690cf9f4dd9e76125e15bc05de7e7` |
| Fleet Subnet Root | 8,115,686 | `7f56d4fc998f9a6c8d41ff5afb88355430248a83fefd5084ffa4b23936793829` | 2,086,575 | `d29af86ab4505b9e16806375b26770d802ba995366809d3539ae18c23bddbd9a` |
| Fleet Coordinator | 4,071,135 | `8d6dbfe377aaf90a8895609a181ddcde27b601cf656c5c0077b5d74a445e4ebc` | 1,013,908 | `73ac90b6db527fb1a8a736d4693d48750ded3c3fdb8e5718eed272a382bdf58b` |
| Wasm Store | 3,330,190 | `682ab5140b54603542dcabead4bbc80f8a2130f0ba8ffbe728175067f2b7f922` | 879,633 | `9bc2bec7d6bc5e2e91dd6790dd98b4b612e02865d0701bb32740a863f4c4a730` |

No same-source isolated pre-cut Wasm pair was retained for this batch, so these
numbers establish current artifact identity and size only. They do not support
a causal Wasm-size saving claim.

## Hard-Cut Result

- The legacy `shared`, `nonroot`, `cycles` and `topology` endpoint-emitter
  modules are deleted rather than retained as aliases.
- `start_local!` emits one local `canic_status` and selected ICRC standards;
  its canonical runtime-probe service has seven status selectors and no Canic
  command because standalone-local mode owns no Fleet mutation authority.
- Ordinary runtime protocol constants and replay policy contain only command
  and status. The two Store byte lanes are compiled only into Store and the
  Root code that calls them; Store-only preflight and replay policy are
  selected by the Store endpoint expansion.
- Active generated services, callers, fixtures, CLI/host presentation and
  application-facing documentation contain no supported old Canic method.
  The immutable normalized B1 register and historical audits retain old names
  only as non-authoritative conversion evidence. Raw pre-cut DID snapshots are
  absent from the current worktree and the capture tool materializes them only
  in temporary scratch while reproducing the register and manifest hashes.
- Blob-service methods and application methods are separately owned protocols;
  they were not folded into 0.103 or renamed by this hard cut.
- The empty `metrics` facade Cargo feature, its default selection and its typed
  role-contract catalog key are deleted. All 38 live Cargo manifests, the
  generated Wasm Store wrapper, packaged-downstream probe and synthetic
  role-contract fixtures now select only effective features. Metrics profiles
  remain derived from role configuration.

The post-correction artifact scan found neither Store lane name in the managed
Component nor Coordinator Wasm. Root retains both exact call names but exports
neither lane; Store retains and exports both. This is the intended caller/
owner boundary and closes the earlier superset-protocol finding.

The broad workspace, release matrix and broad PocketIC suites were deliberately
not run. Focused role builds, facade/source tests, command/status tests and one
Store bootstrap/reverification PocketIC journey own this closeout. The feature
cleanup additionally passes locked metadata resolution, the 17-test core
catalog suite, 31 host package-contract tests, seven generated Store-wrapper
tests, seven facade manifest/documentation tests and six CLI medic role-
contract tests. The maintainer's release flow owns the complete validation
gate.
