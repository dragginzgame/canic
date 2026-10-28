# Current handoff — 2026-10-28

## Scope and source

This contribution binds one complete Root checkpoint preparation unit to published
IC Backup 0.48's original workflow, stage and attempt records. It progresses
[#513](https://github.com/dragginzgame/canic/issues/513),
[#550](https://github.com/dragginzgame/canic/issues/550) and
[Backup #29](https://github.com/dragginzgame/ic-backup/issues/29).

The isolated checkout is `/tmp/canic-backup-adoption.lzm8Ut`, topic branch
`feat/original-backup-checkpoint`, based on released Canic 0.112.0,
`5c236ab130c0d63892ef281b45bdc3fa353e012f`. The normal sibling checkout and its
unrelated work are untouched. Standing cross-repository authority covers this
scoped commit, topic push and PR; review, merge, own version changes, release and
publication remain separate. Own versions stay 0.112.0; compatible notes are open
under [0.112.1](../../CHANGELOG.md#01121).

The preceding released-source [handoff](https://github.com/dragginzgame/canic/blob/5c236ab130c0d63892ef281b45bdc3fa353e012f/docs/status/current.md)
retains earlier release evidence. Its pending-release wording is historical.

## Implemented boundary

`checkpoint` retains strict bounded complete unit intent, exact Wire BEGIN/ADVANCE
requests and fixed original allowances. It implements Backup's stage driver and
uses its new/retained coordinators without another journal or ready-operation
loop. Admission, original command custody and reply qualification are required
provider methods. Missing, changed, pending or returned-but-unqualified originals
never grant replacement spending. Returned reply bytes survive settlement errors.
Existing v1 checkpoint identity bytes now have one runtime-free contract owner.

Installed Root/two-participant qualification discards an actual advance reply,
refuses unsettled continuation, reserves one original observation, retains exact
STATUS evidence and resumes only the absent stage. Complete host replay makes no
provider calls. Separate fixture-owned management authority captures and loads
the same participant IDs with preserved funding policy, verifies frozen views,
then qualifies Root release and its effect-free replay. The fixture has an
explicit qualification network identity and exclusive backend custody; it does
not qualify live trust-anchor enrollment or a default production provider.

## Evidence and remaining work

Five owning native cases pass, including wrong original replies with retained
bytes, changed unit/context refusal and lost ingress spending preservation.
Strict affected Backup/internal-test Clippy passes. Both governed exact installed
cases pass: `pic::application_checkpoint::root::original::qualify_original_host_workflow`
and `pic::application_checkpoint::root::qualify_root_dispatch`; their registered
inventory preflight also passes. Dependency inheritance and external IC boundaries
pass. Another 22 focused contract/model/authority cases pass. Strict owner-library
Clippy passes for Contracts, Core and Control Plane. Final format, documentation
semantics and 141 local-link checks pass; the semantics check retains its three
existing advisory layout warnings. The bounded contribution is ready for PR review.

Logs: `/tmp/canic-checkpoint-native-final.log`,
`/tmp/canic-checkpoint-clippy-final3.log`,
`/tmp/canic-checkpoint-installed-3.log` and
`/tmp/canic-checkpoint-root-dispatch.log`,
`/tmp/canic-checkpoint-contract-tests.log` and
`/tmp/canic-checkpoint-owner-clippy.log`. Original host evidence survives under
`target/original-root-checkpoint--711663-2/`; the earlier failed release-helper
assertion retains `target/original-root-checkpoint--517972-2/`. Keep both original
journal sets and all raw request/reply files. The first two installed failures,
including the corrected no-NNS fixture setup, remain in their separate logs and
failed scratch directories. No full local suite or new native host matrix ran.

The bounded preparation slice is implemented. Full #513/#550/#29 remain open:
production Root-paid snapshot intent/reconciliation, complete artifact transfer,
restore/start, external obligations, post-load application acceptance and existing
terminal/source-reference provider composition are still required. Continue at
those existing owners; do not treat host Applied receipts or fixture management
calls as paid-effect authority. Live CLI creation retains `LiveCreateUnavailable`.
No merge, version bump, release, publication, live operation or artifact cleanup
was performed by this contribution.
