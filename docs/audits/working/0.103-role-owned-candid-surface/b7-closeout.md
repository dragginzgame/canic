# 0.103 B7 Hard-Cut Closeout

Date: 2026-08-17

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
| managed Component | 3,385,328 | `58ed80317199757e5e184cf97d852b610924fe18ab4100586fd1629c5cb67671` | 883,993 | `442a486e78803fa9f13e24fbb2879da74f46e1cadddcb84a7ba11d8029cb9138` |
| Fleet Subnet Root | 8,424,200 | `b4353e34166c54daba2fd00e3a05507493f74bcf0841193ca157bd30da61bb82` | 2,182,503 | `581d60430a663ffd0e60f232a593f35941c269b54c7f283128614044ec7a7b10` |
| Fleet Coordinator | 4,070,628 | `3b6ea1a42065108014c92179c803b0c90319e644144f9be766e1542353544049` | 1,013,747 | `5e419cc39238edc0b92f8588d2ba9e77f8bcf417ec7f4b40ccaba461f4f828d7` |
| Wasm Store | 3,329,993 | `2af177de15b7f90e315d1d8f0c3c53322a0b42e9215577b4b44720271088f54a` | 879,694 | `506c8a0cebfa31b688fe191baf5a3a8eebf50e50bba0ba700e1cf6df79f372d1` |

No same-source isolated pre-cut Wasm pair was retained for this batch, so these
numbers establish current artifact identity and size only. They do not support
a causal Wasm-size saving claim.

## Hard-Cut Result

- The legacy `shared`, `nonroot`, `cycles` and `topology` endpoint-emitter
  modules are deleted rather than retained as aliases.
- `start_local!` emits one local `canic_status` and selected ICRC standards;
  its canonical runtime-probe service has seven status selectors and no Canic
  command because standalone-local mode owns no Fleet mutation authority.
- Runtime protocol constants contain only command, status and the two Store
  byte lanes. Replay policy classifies those methods and their exact role
  command manifests.
- Active generated services, callers, fixtures, CLI/host presentation and
  application-facing documentation contain no supported old Canic method.
  The immutable normalized B1 register and historical audits retain old names
  only as non-authoritative conversion evidence. Raw pre-cut DID snapshots are
  absent from the current worktree and the capture tool materializes them only
  in temporary scratch while reproducing the register and manifest hashes.
- Blob-service methods and application methods are separately owned protocols;
  they were not folded into 0.103 or renamed by this hard cut.

The broad workspace, release matrix and broad PocketIC suites were deliberately
not run. Focused role builds, facade/source tests, command/status tests and one
Store bootstrap/reverification PocketIC journey own this closeout; the
maintainer's release flow owns the complete validation gate.
