# RF3 funding allocation bindings

The .25 working tree now retains complete Component identity from committed
allocation observations. The earlier projection kept only the Spec, release,
role and parent, discarding the Component/Fleet/placement identity needed to
qualify descendant joins. This follow-up continues CANIC-156/174; it is not a
new payment protocol or completed recovery quotation.

Generation checks the selected App/Fleet/network, Coordinator placement, Root
placement and Spec admission before interpreting Root-local usage. Shared pure
policy checks the selected release and Spec hash, the top-level Component role
and declared parent/child grant edge. A descendant cannot become Root-funded
merely because Root holds its pool asset.

Observed chains require matching complete Component bindings, advertised epoch,
release set and parent role. Duplicate identities, cycles and inconsistent
joins fail closed. Missing parents remain explicitly unknown. Iterative traversal
memoizes completed chains; it does not recurse or repeatedly traverse ancestry
for each descendant. No network calls, persisted authority or funding effects
are added. Existing unknown/pending ledger semantics remain in place.

## Qualification

- 25 targeted host funding policy/allocation/transport tests and two CLI preview
  tests pass: `/tmp/canic-rf3-bindings-tests.log`.
- The existing generated-estate planning/application/replay native fixture passes,
  including selected placement, advertised epoch and withdrawn Spec-admission
  rejection: `/tmp/canic-rf3-bindings-generation.log`.
- Scoped host/CLI library/test all-feature warning-denied Clippy passes:
  `/tmp/canic-rf3-bindings-clippy.log`. Grouped-comparison findings were resolved
  with named funding-edge identity and separate predicates, without suppression.
  Changed-source formatting and whitespace checks pass.

The selected base remains `v0.110.24` at
`933a35b66403f6a94f99803252cc52c0aec32958`; these checks include the preserved
dirty recovery/speed changes. No broad gate, version bump, commit, push,
deployment or sibling mutation ran. The upstream scan still ends with the
CANIC-166/172 retirement-evidence blocker already corrected locally; publication
and exact live Toko recovery remain downstream work.

## Remaining RF3 boundary

Seeded allocation consistency is not complete live placement coverage. Matching
advertised epochs is not an independent current-registry freshness proof. Live
descendant ledgers still require the existing protected update relay under a
reviewed observation budget. Complete inventory/registry qualification,
recursive demand and full recovery-reserve integration, transport and IC proofs
remain. Minimum native Root recovery must stay available before Root can afford
descendant telemetry. Complete RF3 is not ready to push; earlier qualified
recovery/speed fixes remain independently ready.
