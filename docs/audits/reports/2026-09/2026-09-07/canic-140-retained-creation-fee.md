# CANIC-140: Retained Estate Creation Fee

Date: 2026-09-07
Scope: current estate seed, desired-state generation and existing growth funding.

## Correction

Retained seeds previously rejected management creation fee authority, and
generation substituted zero. The existing funding planner then omitted that fee
from future autonomous creation even though Root execution charged it.

`management_creation_fee_cycles` is now required in every seed. Both fresh and
retained inputs use compact cycle units; generation copies the explicit value
into the current desired contract. Missing fields fail decoding, and malformed
amounts fail seed validation before live observation. No fallback remains.
Retained identities still represent already-paid assets. The fee contributes
only to the forecast for new creation; it does not retrospectively charge the
retained estate.

The existing desired digest, reviewed plan, funding domain and conservation
records retain the value. There is no runtime/Candid change, alternative funding
owner or new recovery mode. The CLI reads the retained seed through `--seed`;
`--management-creation-fee-cycles` continues to initialize a fresh seed. Help and
the operator guide explain both inputs.

## Focused Evidence

- All 20 host generator regressions pass (1.32s test execution). The maintained
  retained-estate generation/recovery/replay case now carries `500B`.
- Its new growth-plan assertion starts from two retained assets (one Workload
  and one Ready), forecasts one additional asset for the two-Ready floor, and
  verifies 5T readiness + 1T execution margin + 500B management fee = 6.5T.
  The separate 0.1B Ledger fee produces exactly 6.5001T creation funding.
  Planning performs zero mutations. This is host generation/planning evidence
  using controlled observations, not a real IC execution claim.
- Seed validation accepts explicit `0B`, `500B` and `1T`; missing, empty,
  unsuffixed and negative fee authority rejects. Existing fresh-seed authority,
  exact topology, controller, interruption and replay checks remain passing.
- Warning-denied Clippy passes for every target and feature of `canic-host`,
  `canic-cli` and `canic-testing-internal` (1m10s).

- The existing `funded_estate_recovers_transfer_and_autonomous_creation_responses`
  PocketIC case now runs the production generator against the prepared estate
  before planning or applying growth. It passes in 118.47s (127s runner including
  compilation). One retained pool asset becomes one Workload plus one newly
  created Ready asset. The generated `500B` management and `0.1B` Ledger fees
  feed the exact funding plan, real control-plane Wasms and Ledger stub.
  The same journey rejects a changed Ledger fee, recovers lost funding and
  creation replies, proves fee/conservation accounting and performs effect-free
  terminal replay. No new long test or application-specific cardinality is added.
- The generator validates real retained controllers, installed policy and pool
  observations. Its IC-built artifacts and network identity remain unchanged;
  a fixture wrapper routes all canister/cycle RPCs to PocketIC. Synthetic,
  validated Registry catalog input binds those actual PocketIC Principals to
  their subnet. This is not live NNS/mainnet evidence. Test-only `ic-query` and
  TOML dependencies reuse already-pinned workspace versions.
- Formatting checks pass for the four changed Rust files; `git diff --check`
  passes. Retained local logs are `/tmp/canic-140-generator.log`,
  `/tmp/canic-140-clippy.log` and `/tmp/canic-140-paid-growth.log`.
  The combined journey log is `/tmp/canic-140-combined-growth.log`; final fixture
  all-target/all-feature warning-denied Clippy passes (2.86s) and is retained in
  `/tmp/canic-140-combined-clippy.log`. Subsequent edits only rename a fixture
  helper and remove an unnecessary path clone; the final code compiles and
  formats cleanly without rerunning the unchanged paid-growth behavior.
- Both CLI help regressions pass (0.24s test execution), including recursive
  command help. Log: `/tmp/canic-140-cli-help.log`.

## Adoption And Limits

The maintainer requested application-neutral tests. The generic host budget
proof and combined small PocketIC journey preserve that scope. The previously
separate generation/runtime qualification gap is closed; Toko Miner's complete
application-cardinality and mainnet acceptance remain downstream work.

After publication, Toko Miner must add its exact current fee to the retained
seed, regenerate through the CLI, and review the corrected plan before funding.
For its reported policy, 1.9T readiness + 1T execution margin + 500B management
fee is 3.4T per new asset, plus the separate Ledger fee. Its completed Root
prerequisite and controlled balances must remain accounted for through the
normal authority checks. Do not hand-edit generated desired state or add an
unreviewed Ledger credit.

Published CLI/library adoption, the complete downstream growth journey and
terminal mainnet/application verification remain downstream qualification.
The source correction is ready for inclusion in the open 0.110.9 release batch;
the changelog is updated. This focused result does not independently qualify
the other changes in that batch or close the downstream issue's adoption gate.
The sibling repository was read-only. No broad validation, version transaction,
publication or live deployment was performed for this correction.
