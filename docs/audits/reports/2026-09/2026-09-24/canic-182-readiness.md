# CANIC-182: grouped native-cycle readiness

Date: 2026-09-24. Canic-owned correction; Toko Miner remains read-only.

## Report and owner

Toko Miner's `docs/upstream/canic.md` records CANIC-182 against Canic/CLI
0.110.38 and ICP CLI 1.5.0. Its retained
`docs/upstream/artifacts/canic-182-readiness-2026-09-23.json` has SHA-256
`e2a85207aceebca1f1621691cfe69c883d5bec906b49698969873e8f84a1e38f`.
The status response contains `407_037_311_267_631` cycles, while readiness
reports `balance_unavailable`. The same direct integer parser remains in
published Canic 0.110.39.

The canonical correction belongs to
`canic-host::fleet_ensure::ops::readiness`. Exact Principal and controller-set
validation still precede the cycle conversion. The adapter accepts ASCII decimal
counts, either ungrouped or with three-digit underscore groups. It bounds input
to the maximum grouped `u128` representation and accumulates with checked
arithmetic, without allocating a normalized string. Bad grouping, signs,
fractions, units, whitespace, non-ASCII digits and overflow remain unavailable.
Other ICP parsers and funding effects are outside this change.

## Observable behavior

The existing readiness projection now populates the exact native balance and
its saturating floor shortfall. The captured count with a 500T floor gives
92,962,688,732,369 cycles of shortfall; a balance above the floor gives zero.
Human output and JSON already consume those fields. No schema change, new
funding action or new live observation is introduced.

The focused tests cover JSON decoding of the reported grouped value, plain
counts, zero, both representations of `u128::MAX`, malformed/overflowing values,
identity/controller rejection with a valid grouped balance, exact shortfall,
and unchanged retained-state drift rejection.

Qualification passes: five focused readiness tests (the existing opt-in external
inspection remains ignored), strict `canic-host` Clippy across all targets and
features, scoped formatting, layering and whitespace checks. The test command is
`cargo test --locked -p canic-host --lib fleet_ensure::ops::readiness`; lint uses
`cargo clippy --locked -p canic-host --all-targets --all-features -- -D warnings`.
Both run offline with `RUSTC_WRAPPER` empty. Logs and source/lock hashes are under
`.tmp/canic-182-20260924/`. The pre-existing lockfile edit is preserved and is not
part of this correction. No broad gate or live ICP call ran.

The bounded operator correction remains on the affected .110 line under the
post-publication correctness exception to the twelve-release cadence guideline.
Both changelog views accumulate in the open .40 draft; package versions remain
owned by the maintainer's release flow. Downstream adoption and live acceptance
remain separate from this local correction.
