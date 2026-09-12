# CANIC-163: selected-build reinstall qualification

The Canic extension is implemented and qualified in the open 0.110.15 draft.
Package versions remain 0.110.14. The complete CANIC-160/163/164/166 batch is
ready for release review; publication and Toko wrapper adoption remain separate.

## Behavior

A fresh explicit reinstall selects the supplied desired build. Its reviewed
intent separately retains the completed source input, installed infrastructure
hashes and source Candid contracts, plus the selected target artifact digest.
Preparation seals source allocation; the full review captures every physical
pool asset and deliberately reinstalls the complete generated infrastructure.
The existing continuation then restores the selected application closure.

Source and target must retain Fleet, environment, operator, Cycles Ledger and
infrastructure topology. Live controller, principal, subnet and source module
checks remain exact. Terminal checks require the selected installed modules.
Both interrupted apply and an exact completed reinstall digest recover the
retained target despite another desired selection. A new explicit request cannot
replace an unfinished wipe. Source and selected artifact bytes remain required.

The runtime proof exposed a real lost-response distinction: the effect remains
`Intent` when the install response is lost; `Issued` follows a received response.
Reconciliation therefore admits the exact intended replacement Root in either
pending state, then verifies replicated management deployment history against
operator, mode, prior version and selected module. Other witnesses, modules,
controller sets and subnets remain rejected.

The durable reinstall review record changes through the maintained pre-1.0 hard
cut. There is no old-plan migration or new compatibility lane. Partial-activation
recovery keeps its existing separate source admission.

## Qualification

| Check | Result |
| --- | --- |
| Host source/target admission, artifact drift, install history and existing activation/adoption boundaries | 9 pass |
| Fleet CLI authority loading, parsing and rendering, including completed-digest loading without working TOML | 15 pass |
| Recursive command ordering/example bounds and bare-command help | 2 pass |
| Host, CLI and testing-internal library/binary/test Clippy, all features | Pass with warnings denied |
| Exact mixed-Fleet PocketIC journey | Pass, 770.29 seconds; 784-second runner |
| Source snapshot across PocketIC | 1,557 Rust/Cargo/toolchain files unchanged |
| Changed Rust formatting, document links/semantics, .15 draft, evidence checksums and secret scan | Pass |

The disposable estate has three infrastructure canisters and six pool assets.
Two independently sealed build identities share the fixture topology and data
definitions. The first wipe installs the different selected infrastructure and
application Wasms; the second deliberately wipes the identical selected build.
Both erase authored test user rows, restore installation fixture rows, retain
the physical estate and account for observed cycle consumption within reviewed
bounds. Infrastructure install counts are exactly three per deliberate wipe.

The first wipe interrupts before an install, after an install response is lost,
and before Root replacement. It advances Root's version through a non-deployment
inspection, rejects a conflicting new request, then loses the replacement Root
response and resumes while supplied desired input selects another release.
The completed-digest replay also supplies that other selection and performs no
new install. The second wipe receives a new operation identity and repeats
state, conservation and effect-free replay checks. Public allocation checks
across the resulting roles also pass.

Final assertions derive names from reviewed physical bindings and managed
application hashes from the selected release manifest's installed compressed
artifact identity. The fixture's raw build bytes have a different hash. This
keeps the evidence aligned with the production terminal inventory verifier.

The [machine record](canic163-qualification.json) binds the source snapshot,
[focused tests](canic163-evidence/native.log), [Clippy](canic163-evidence/clippy.log)
and [PocketIC log](canic163-evidence/pocketic.log). The two wipe journeys take
261.026 and 240.245 seconds; the combined selected-build phase takes 509.323
seconds. These are disposable fixture timings, not a Toko deployment benchmark.

No broad workspace suite, package/version change, Git publication, sibling
mutation or staging apply ran. Toko's wrapper must select the current build for
a fresh request and retain exact digest-based resume after interruption; its
application-specific startup qualification remains downstream. CANIC-165's
provisioning contract remains separate from this batch. The
[operator contract](../../../../features/operations/fleet-ensure.md#deliberate-selected-build-database-wipe)
and [batch design](../../../working/canic163-selected-build-reinstall/design.md)
record the maintained boundaries.
