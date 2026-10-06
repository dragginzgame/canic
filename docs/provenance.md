# Provenance

Shared Tooling is maintained collaboratively across Dragginz Game repositories.

- Canic supplied the initial portable tooling and CI mechanics.
- The first cross-consumer review on 2026-09-03 incorporated IcyDB feedback on
  no-build decisions, state-space control, canonical authority, carried
  decision artifacts, reviewable changes, focused validation, and the boundary
  between shared baselines and project policy.
- The 2026-10-05 changelog review compared the local changelog guidance and current
  ledgers across Canic, IcyDB, IC Backup, IC Blob Storage, IC Host Tools, IC Memory,
  IC Metrics, IC Query, IC Testkit, IC Timers and Toko Miner. Canic and IcyDB supplied
  the main conventions for concise root summaries, minor-line detail files,
  compatibility notes and historical preservation in `rules/changelogs.md`.
  Their reviewed policy files were `docs/governance/changelog.md` at Canic revision
  `481e94d0e93080168fb04546cebb3fc48887a66f` and IcyDB revision
  `8e74b6bac5d054eab9dd9de3435f258d85926040`; those policy files were clean, while
  IcyDB's current ledger included working-tree edits. Shared Tooling retains its
  own draft, release and authority contract; consumer-specific presentation and
  scoped exceptions were not promoted into universal rules. This source review
  does not establish adoption in any consumer.

## Shared audit methods

The 2026-10-06 consolidation extracted reusable review questions from these
consumer definitions. Their audit-definition files were clean when reviewed;
unrelated in-progress consumer changes and new reports were not treated as
published method evidence.

| Source revision | Reused material |
| --- | --- |
| [IcyDB `8631511`](https://github.com/dragginzgame/icydb/tree/8631511d703f2b9ecf63ac865b9c4614116cba57/docs/audits) | Audit scope/evidence contract, consolidated flow-convergence and complexity methods, targeted module surface and cleanup playbooks. |
| [Canic `e1a211a`](https://github.com/dragginzgame/canic/tree/e1a211a00f01568ccc99bedc494c62a7141444dd/docs/audits) | Audit identity/comparison discipline, module reachability and runtime-shape proof requirements. |
| [IC Memory `6e98b07`](https://github.com/dragginzgame/ic-memory/blob/6e98b07882ed43d180195f5f915eafaef05f25db/docs/audits/recurring/code-hygiene.md) | API, trust-state, panic, constructor and negative-test hygiene. |
| [IC Timers `98c4b29`](https://github.com/dragginzgame/ic-timers/blob/98c4b296d7461525c15a01e30adbe33b75bcfa38/docs/audits/code-hygiene.md) | Narrow hygiene scope, typed/runtime authority and documentation consistency questions. |
| [Toko Miner `061cfb6`](https://github.com/dragginzgame/toko-miner/blob/061cfb6e3702a7075ab3c118bfaf315d7d2b0053/docs/audits/crosscutting/code-hygiene.md) | Cross-language hygiene, generated boundaries and tracked-artifact discipline. |

The common methods deliberately use individual findings instead of composite
scores, preserve inspection versus repair authority, and keep broad gates
explicit. Consumer-specific architecture, release-line traces, metric rosters,
numeric thresholds and method-catalog machinery remain local. Module cleanup is
an implementation procedure, not an additional audit verdict. Definition
consolidation does not imply consumer snapshot adoption or new runtime proof.

## Local IC tools and verification helpers

The common installer combines existing Canic and IcyDB Binaryen/PocketIC setup
requirements with IC Timers' verification-before-execution checks and IC Metrics'
retained provisioning evidence. Exact default versions match the inspected
consumers; Quill is added from the official release. Platform digests and release
sources are recorded in [IC tools](ic-tools.md). This is setup orchestration;
artifact inspection and execution contracts owned by `ic-host-tools` remain there.

The local jq/yq pair extends the existing shared yq selection in
`ci/tool-versions.env`; jq 1.8.2 digests were read from official release-asset
metadata on 2026-10-06. [Local setup](local-setup.md) records upstream releases
and the distinction between pinned executables and system bootstrap packages.

The evidence-manifest helper and its fixtures came from IC Blob Storage's
uncommitted working tree after `ebd535e` on 2026-10-06; those bytes are not attributed
to that commit or treated as released consumer behavior. The nonempty Cargo
test helper adapts Canic's `docs/audits/scripts/run-nonempty-cargo-test.sh` at
`e1a211a00f01568ccc99bedc494c62a7141444dd`, adding caller-workspace selection,
logging-failure handling and failed-output retention. The exact release-tag
check extracts IC Timers' annotated-tag/selected-commit checks at
`98c4b296d7461525c15a01e30adbe33b75bcfa38`, retaining explicit identity inputs
instead of choosing consumer metadata or HEAD. Consumer migrations remain
separate from preparation of these shared owners.

This record acknowledges sources; it does not make any consumer's local
governance authoritative here. Git history remains the exact source history,
and current repository documents own the maintained shared contract.
