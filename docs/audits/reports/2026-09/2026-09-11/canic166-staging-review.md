# CANIC-166 staging recovery: read-only review

The maintainer requested this assessment after confirming that staging need
not wait for CANIC-163. The existing explicit recovery review succeeds against
the observed staging estate. No apply, stop, start, install, funding, deletion
or publication command was issued.

The assessment used the current Canic diagnostic candidate and native ICP CLI
1.4.0. It copied the retained plan/journal/state, current desired input, App/ICP
configuration and the two immutable release artifact directories into a
private Canic-owned workspace. ICP identity selection and caches were isolated;
the original global identity remains `anonymous`. Temporary copied identity
material was removed after the review. No downstream file was written.

## Exact selected authority

| Field | Observed value |
| --- | --- |
| Environment / Fleet | `staging` / `toko-miner-staging-001` |
| Source operation | `aed7d8545e4722930c1b500fde05b87c4ba07e4e63088ebbd54593d7b2ab8895` |
| Source journal plan reference | `5714f60182f51fff270478361b9dafd96222c417cb084ccf67e2a5118b419407` |
| Selected target release build | `d35953a6157e746b9af5cf23e897a90faabbd8df25832ddcb37b57541da1f607` |
| Preparation review operation | `9bf8f663018d7749710f9dafb0fa3eaf8d664fc6c3d57ec593d9769b5ded3586` |
| Preparation review digest | `20e836ef8401e1509e7c8753edb9247c3c6a68d7c5d3906aeecd3c481ad2bb40` |
| Scope | `reinstall_preparation` |
| Applied effects / terminal | `0` / `false` |

The digest belongs to the isolated review, not to a newly staged review in
Toko's live workspace. Do not copy it into another operation's journal or
candidate marker. The [structured assessment](canic166-staging-review.json)
records the retained raw review path and its checksum.

## Scope assessment

The review contains exactly these ordered actions:

1. Stop Coordinator `jesds-iaaaa-aaaar-qcbgq-cai`.
2. Stop Root `2ydug-eaaaa-aaaab-qhfca-cai`.
3. Start that same unchanged Root.

Store has no action. There are no protocol actions, Root install bindings,
creation, transfer or deletion actions in this preparation review. The selected
target has the corrected replacement Root artifact, but that artifact is not
installed by this scope. Coordinator would remain stopped until the dependent
Full Ensure completes; this is an operational interruption if later applied.

Live admission checks accepted the source modules, exact controllers and subnet
placement, the inactive activation/issued provisioning evidence, and a complete
physical estate of three infrastructure canisters plus 24 pool assets. The
pool contains nine Workloads and 15 Ready assets with no pending creation.
Every pool asset remains controlled only by its Root. The Root Ledger account
is zero and matches its retained source balance.

This is the intended settlement preparation for the existing CANIC-157 owner.
Root reinstall and the later Full Ensure reset require subsequent reviews
after their prerequisites complete. This review neither describes their final
reset scope nor establishes their funding sufficiency or terminal conservation.

## Cycle assessment

| Quantity | Cycles |
| --- | ---: |
| Observed controlled native balance | 392,570,624,115,307 |
| Maximum preparation execution debit | 219,000,000,000,000 |
| Conservative post-preparation balance floor | 173,570,624,115,307 |
| New funding / operator debit / transfer / unavoidable fee allowances | 0 |
| Prior source observed debit | 114,852,237,593,758 |
| Retained source maximum execution debit | 375,000,000,000,000 |

The arithmetic is consistent: observed balance equals the preparation allowance
plus its conservative floor. The source debit remains within the source's
reviewed bound, and no Root Ledger debit is inferred or defaulted.

The 219T figure is a broad **allowance, not an expected cost**. The selected
desired input allows 1T per observation and 1T per update; the existing policy
budgets eight observation rounds across 27 canisters plus three updates:
`27 × 8 × 1T + 3 × 1T = 219T`. Those input bounds were retained unchanged.
The review does not attribute the prior 114.852T observed debit to particular
application work or establish the exact cost of future recovery. Its
`actual_conservation` remains null until an operation is actually completed.

## Result and remaining boundary

CANIC-166 has an admitted read-only recovery review for this observed source;
CANIC-163 is not a prerequisite. Both the original and isolated plan, journal,
state, desired input and copied App/ICP configuration retain their exact input
hashes. The original operation remains InProgress and untouched.

Any later apply must use its exact retained review and freshly revalidate source
authority, physical inventory, bounds and callback settlement through the
existing driver. New source activity or changed desired input may invalidate
this observation. This assessment does not authorize apply and does not close
staging recovery. The remaining operator implementation priority is CANIC-163;
CANIC-165 stays a separate provisioning design.
