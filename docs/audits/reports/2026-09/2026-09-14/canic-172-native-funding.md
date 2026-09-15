# CANIC-172 native supplementary funding

Date: 2026-09-14. Working-tree qualification on Canic base `v0.110.16`,
`a875c6498721bd89ca98549e38389b8280910d68`, extending the open .17 draft.
Toko's read-only feedback ledger initially had SHA-256
`05a202c11f25e8863556d68c313372a822985f71049f0003cd2adbcbdf3b1433`.
The final refresh still ends at CANIC-175, with a new deployment-priority
handoff and SHA-256
`f5ddb5c3911ded515e00a8867e8e38f8814290bad1d61cb941bc75742e1fe859`.
Its funding/forecast-first priority agrees with this work. Build reuse and
inspection cost follow; its timing samples are not an additive whole-deployment
benchmark and its application watchdog burn remains unproven attribution.
No version, Git publication, deployment, live funding or sibling edit ran.

## Implemented host boundary

The existing `fleet_ensure::workflow::funding` owner now admits native reviews
alongside estate Ledger-account reviews. There is one current tagged pause
contract (`estate` or `native`), not a second recovery command or journal lane.
Record construction stays in the existing funding ops module. The current
journal/report shape changes in place; old nonempty funding-review shapes do
not acquire a fallback reader.

Review requires an already-issued `ProvisionComponents` action. Only an exact
Root selected by that action's batches or directory confirmation scope can
receive a review, once per Root/action pair. The original operation, plan,
issued/Applied effect prefix, starting operator/controlled balances and initial
Root Ledger balances remain fixed. Review uses the retained desired input.

The native observer reads management status and the exact Ledger's fee and
operator account directly. It requires the selected operator, configured Root
module, original resolved Principal, exact controllers and running status. It
does not depend on the runtime Funding/Inventory endpoints that were fenced
during the reported partial activation.

Amount = configured Root minimum minus observed native balance, plus the
configured observation/update margin, only when native balance is below the
minimum. Generation takes that minimum from the Root request threshold (10T
in Toko's retained plan). No new fixed operating reserve is introduced, and the
runtime 1T deployment guard is unchanged. This quote is not a startup-grant
forecast or proof that insufficient reserve explains every application failure.

A distinct review digest approves the exact amount, receiver, fee and fixed
withdrawal timestamp. Planning creates no effect. An unapproved quote can be
refreshed after its margin expires or removed when the Root is already funded;
its old digest then ceases to authorize the quote. Once intent exists, neither
amount nor timestamp changes. Lost withdrawal responses reuse that identity;
a retained receipt resumes through observations without another withdrawal.
Partial/unexplained operator debits and unreceipted operator credits reject.

Source debit must equal amount plus fee. Native credits extend total funding
and fee conservation bounds without entering the Root's Ledger-account funding
total. Destination balance may already have moved after a successful receipt;
whole-estate terminal accounting remains responsible for conserved transfers
and bounded execution debit. A low Root balance after the completed review
does not admit another credit under the same provisioning action.

## Focused qualification

- Eleven host funding cases pass: the existing estate approval/replay/drift
  boundaries plus native approval, public retained review, selected-Root scope,
  no repeated credit, source/fee/controller drift, unreceipted credits, exact
  terminal accounting, lost withdrawal response, lost post-receipt observation,
  and refresh restricted to unapproved quotes.
- One CLI case passes for native destination, amount, fee, explicit nullable
  effect and the separate approval digest. Existing estate IC fixtures compile
  against the tagged pause shape.
- Host, CLI and internal-testing all-target/all-feature Clippy passes with
  warnings denied. Scoped formatting, layering and whitespace checks pass.
- Logs: `/tmp/canic-172-native-funding-tests.log`,
  `/tmp/canic-172-native-funding-clippy.log` and
  `/tmp/canic-172-native-funding-guards.log`.

These are native host evidence tests, not real Ledger withdrawals. The earlier
[child reserve IC proof](canic-172-child-reserve.md) establishes same-claim
readiness/replay after disposable replenishment; it does not qualify this host
review, Ledger fee/receipt behavior or the complete composed recovery path.

## Composed native withdrawal qualification

The registered case
`pic::fleet_registry::baseline::tests::native_funding::issued_provisioning_recovers_native_withdrawal_and_receipt_observation`
now passes through the production IcpEnsurePlatform against PocketIC 16.0.0,
ICP CLI 1.5.0 and the existing Cycles Ledger stub. The stub executes a real
management `deposit_cycles`; it is not the deployed mainnet Ledger implementation.
The canonical sealed Coordinator, Root and Store artifacts retain their existing
runtime behavior. No additional test framework or production endpoint was added.

The fixture starts with its existing 80T Root and deliberately sets the host
minimum/creation policy to 100T before planning. It omits that forecast native
credit before issuing any operation effect, retaining the estate funding action
and its exact real balance observations. These synthetic amounts only create a
repeatable host shortfall; they are not operating recommendations or changes to
the runtime 1T guard. Every retained effect is actually issued through the
adapter after writing its intent. The test stops setup at a real issued
Coordinator provisioning action, preserving the original operation and balances.

Qualification proves:

- The old plan digest cannot approve the new native supplement.
- A real successful withdrawal with a lost transport reply retains Intent.
  Reconstructing the adapter retries the same withdrawal and recovers its
  duplicate receipt, with exactly one deposit and amount-plus-fee source debit.
- Losing the next account observation leaves the receipt durably Issued.
  A second adapter reconstruction observes completion without withdrawing again.
- Provisioning reaches one Workload and one Ready reserve with no Failed or
  PendingReset assets. Original starting balances, prior Applied effects and
  provisioning identity remain fixed. Native and estate funding account
  separately, and the terminal equation balances including both exact fee kinds.
- Terminal replay performs no additional withdrawal or creation, retains the
  exact journal and preserves pool identities/statuses and creation receipts.

The final case passed in 59.92 seconds (79-second targeted runner segment),
using the sealed artifact cache. These are validation durations, not deployment
benchmarks. `/tmp/canic-172-native-ic.log` retains the passing result. The first
attempt built the sealed artifacts and exposed a fixture assumption about the
generated Root name; the second reached both loss boundaries but correctly
rejected omitted observations from the fixture's preceding estate transfer.
Both setup defects are corrected without weakening journal verification.

This is a composed issued-operation/withdrawal/replay proof. It does not force
E163 or hold the same low-reserve child claim across that withdrawal. The earlier
child-reserve IC case remains a separate proof using disposable replenishment;
the two cases must not be presented as a single end-to-end incident reproduction.
Operator mint conversion and its receipts are also outside this case.

Final affected checks pass: internal-testing all-target/all-feature Clippy,
scoped formatting, layering and diff checks; the pure governed inventory-order
case; and the existing estate-funding pause IC regression (124.86 seconds).
Logs are `/tmp/canic-172-native-ic-clippy.log`,
`/tmp/canic-172-native-ic-inventory.log` and
`/tmp/canic-172-estate-pause-regression.log`. No broad suite ran. The final
read-only Toko ledger refresh retains the SHA-256 recorded above and ends at
CANIC-175. The subsequent [CANIC-174 correction](canic-174-funding-deadline.md)
qualifies child-grant deadline reconciliation and reviewed startup prepayment.
Native supplementary reviews now retain at least the same 1T deployment floor
as runtime admission, plus their configured margin. All seven focused funding
regressions pass in `/tmp/canic-native-floor-tests.log`.

The subsequent recovery review also respects the selected Root's bootstrap
request threshold plus one cycle. It retains a higher configured minimum and
adds the existing observation/update margin. A different Root's higher threshold
cannot inflate the quote; missing/duplicate selected authority and arithmetic
overflow reject before effects. This closes the gap between startup threshold
semantics and supplementary recovery without claiming live grant forecasting.
The generated-estate authority/replay case and all seven native recovery cases
pass in `/tmp/canic-native-threshold-planning.log` and
`/tmp/canic-native-threshold-recovery.log`; scoped host Clippy passes in
`/tmp/canic-native-threshold-clippy.log`.

## Same-child claim and native withdrawal — 2026-09-15

The registered case
`pic::fleet_registry::baseline::tests::native_funding::native_withdrawal_recovers_the_same_initial_child_claim`
passes the previously separate boundaries in one production-host journey.
The disposable local Fleet has one User Hub, one initial User Shard and one
Ready spare. Its existing audit Root alone exposes controller-guarded balance
instrumentation. The release binds the actual audit Candid/profile and derives
Root signing features from the same role requirements as the canonical builder.
No production artifact, reserve guard or recovery semantics changed.

Before the operation, the fixture reduces its Root to 20T and deliberately omits
one forecast native credit. After the parent registry commit it reduces the Root
to 800B. The protected Coordinator status must identify an E163 initial-child
operation, with a Reserved claim and no creation/install evidence. The host then
reviews the native shortfall and rejects approval through the original plan digest.

One separately approved withdrawal performs a real management deposit through
the existing Ledger stub. Lost withdrawal and post-receipt observation replies
survive two adapter reconstructions. The exact claimed child reaches readiness,
its retained failure clears, and terminal inventory contains two Workloads and
one Ready spare. Original starting balances, earlier effects and provisioning
identity remain fixed. Exact operator debit, funding, fees and bounded observed
burn balance in the terminal equation. Immediate replay preserves the journal,
pool identities, receipts and Ledger request counts without another deposit.
There is no PocketIC Root replenishment in this recovery path.

The case passed in 239.55s, including 152.75s of sealed-artifact preparation;
the targeted runner included native recompilation and took 423s. These are
qualification durations, not deployment benchmarks. Evidence:
`/tmp/canic-native-child-ic.log`. The build cache checks both pre/post inputs;
its snapshot now follows generated Root package preparation. Initial attempts
exposed fixture input ordering, omitted Shard capabilities and missing audit
Root signing features; all were corrected without weakening the guards.

This local case uses a zero Ledger fee. The existing synthetic-minimum case
separately qualifies nonzero fees and estate-account funding. The Ledger stub
is not the mainnet Ledger, audit burns are controlled fault injection, and the
small topology is not Toko's application estate. Operator ICP conversion and
live grant usage/reservations remain unqualified by this case.

Formatting, layering, document semantics and whitespace checks pass. Final scoped
Clippy encountered concurrent host work: the enlarged `IcpConfigError::Manifest`
propagates `result_large_err` across host callers and three existing fixture
return types, and the new uploaded-frontend check has a grouping lint. Both the
dependency-inclusive and `--no-deps` results are retained at
`/tmp/canic-native-child-clippy.log` and `/tmp/canic-native-child-owned-clippy.log`.
This checkpoint does not claim lint readiness; no suppression or unrelated
host edit was introduced to bypass the concurrent work. The pure governed
catalogue regression passes in `/tmp/canic-native-child-inventory.log`.

## Operator mint evidence boundary

Canic's pinned [ICP CLI 1.5.0 mint command](https://github.com/dfinity/icp-cli/blob/v1.5.0/crates/icp-cli/src/commands/cycles/mint.rs)
prints only deposited cycles and the new balance. Its
[mint operation](https://github.com/dfinity/icp-cli/blob/v1.5.0/crates/icp/src/operations/token/mint.rs)
receives the ICP transfer block internally, but discards transaction identifiers
from its returned `MintInfo`. The transfer has no fixed `created_at_time`, and
the CMC notification supplies no deposit memo. Replaying that command after a
lost response is therefore unsuitable as Canic's operation-bound retry owner.

The remaining integration needs retained intent before the ICP transfer, fixed
transfer identity, exact ICP and Cycles Ledger receipts and one verified credit
in the original journal's conservation equation. A printed balance delta is
insufficient. No mint implementation or live transaction was added during this
source review; the already-funded operator path remains the qualified surface.

## Remaining work

1. Integrate exact operator ICP mint credits/fees/receipts in the original
   operation. This slice requires existing operator cycles and deliberately
   rejects unexplained mint credits; it is not the full Toko manual-mint case.
2. Extend the qualified initial-grant/deployment-reserve forecast with live
   grant usage and outstanding reservations. The startup prepayment and
   CANIC-174 grant/deadline corrections now pass their focused evidence.

Earlier CANIC-171 interruption and CANIC-156 reserve-preview evidence also
remain. B1 stays behind Toko feedback. The complete accepted .17 batch remains
open and is not push-ready; an unrun broad suite is not the reason for that
assessment.
