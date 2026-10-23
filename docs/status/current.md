# Current handoff

## Source and scope

This summary covers committed checkpoint
`cf2d3d1fba84a00a76cdbc541ae7e68fe36e9bb5`. Package metadata remains
0.111.0; root and detailed notes prepare 0.112.0. It does not establish release
readiness or describe later uncommitted work as delivered.

Continue the accepted 0.112 integration batch under [AGENTS.md](../../AGENTS.md).
The [design and status](../design/0.112-standalone-blob-service-extraction/status.md)
retain scope decisions and their original evidence. Historical selections there
are not the current dependency catalog; inspect Cargo.toml and all three locks.

At this checkpoint, the catalog selects Blob/contracts 0.41.1, Auth/protocol
0.14.3, Backup 0.38, Query 0.68, Host/Build/Testkit 0.64, Platform 0.6.7,
Memory/Storage 0.50.10, Metrics 0.15.5 and Timers 0.30.2. The reviewed Shared
snapshot is 0.26.6 at `4cb9bc9`. Later local adoption has separate inputs.

## Resume priorities

- [Owned Wire contracts and frontend artifacts (#613)](https://github.com/dragginzgame/canic/issues/613)
  require one coordinated endpoint/caller/schema/release-identity cut. Do not
  infer completion from upstream codec availability or native schema tests.
- [Mounted Blob coverage (#622)](https://github.com/dragginzgame/canic/issues/622)
  remains distinct from the qualified journey/metrics subset. The checkpoint
  adds payment-relationship responses and reply bounds; delayed callback races,
  populated pagination and paid/provider acceptance retain their own evidence.
  See the [owner/coverage audit](../audits/working/blob-0411-adoption.md) and
  [composition guide](../../integrations/blob-service/README.md).
- [Funding acceptance (#494)](https://github.com/dragginzgame/canic/issues/494)
  must distinguish provider observations, local journals and actual credit.
  Do not fabricate provider evidence or repeat uncertain paid effects.
- [Replica shutdown (#589)](https://github.com/dragginzgame/canic/issues/589)
  verifies the selected loopback gateway after ICP stop. Safe orphan custody,
  PID diagnostics and stale-descriptor cleanup remain separate; listener
  reachability or executable names never authorize killing a process.
- Live backup/capture and coordinated same-release restore remain with their
  [owning issue (#550)](https://github.com/dragginzgame/canic/issues/550).
  The unused injected capture runner is retired. Retained plans, participant
  models and dry runs do not establish a live backup or installed orchestration.

Other follow-up stays in the [owning issues](https://github.com/dragginzgame/canic/issues).
Trimming this handoff closes no issue and does not waive product acceptance.

## Latest evidence at this checkpoint

The mounted dedicated and embedded cases pass for relationship responses at
`/tmp/canic-blob-relationship/` and reply bounds at `/tmp/canic-blob-bounds/`.
Controlled-provider and earlier mounted acceptance remain at
`/tmp/canic-blob622-provider/` and `/tmp/canic-blob041/`.
These prove the named Linux/PocketIC cases, not exhaustive branches, production
provider behavior, native macOS or complete release readiness.

Coordinated Backup/Query/Host native adoption evidence is
`/tmp/canic-query068/`; Blob catalog/lock admission is `/tmp/canic-blob0411/`.
Replica shutdown's focused cases and lint are in `/tmp/canic-589/`; its earlier
sandbox socket-bind failure is retained separately. Health and log-rendering
proofs remain under `/tmp/canic-issue534/` and `/tmp/canic-issue554/`.
Use the original source/lock identities before reusing any of those results.

## Working-tree custody and validation

Read dirty files before continuing. The normal checkout may contain newer
uncommitted endpoint, Blob, dependency and handoff work beyond this checkpoint;
preserve it and reconcile with its owner before applying this summary.
Do not replace those newer facts with the committed dependency list above.

Use focused checks under [CI/deployment governance](../governance/ci-deployment.md).
Common scripts are reviewed snapshot payloads; adapters retain Canic policy.
The complete gate, native acceptance, releases, publication and live/paid effects
keep their separate authority. No full gate is implied by this documentation edit.

## Retained history

The complete prior 5,187-line handoff, including original failures, qualification
limits, recovery locations and older source identities, remains immutable at
[the checkpoint revision](https://github.com/dragginzgame/canic/blob/cf2d3d1fba84a00a76cdbc541ae7e68fe36e9bb5/docs/status/current.md).
Consult that evidence when resuming its specific obligation, not as a startup
journal. Existing uncommitted notes and build/evidence directories are untouched.
