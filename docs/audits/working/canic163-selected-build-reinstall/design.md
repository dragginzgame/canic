# CANIC-163: selected-build Fleet reinstall

Status: qualified Canic-only operator batch in the open 0.110.15 draft.
Package versions remain unchanged. CANIC-165 provisioning and live staging
recovery are separate work.

## Contract

A fresh explicit reinstall selects the supplied current build. The completed
source operation supplies the observed physical estate, installed authority and
source protocol contracts. These are distinct from the selected target Wasms.
Both are retained in the reviewed operation before effects. Ordinary resume and
apply use that retained target even if the workspace selects another build.
A new explicit request cannot replace unfinished intent.

Retain the existing preparation, full reset and continuation executor. Seal
allocation through the source protocol, enumerate the complete physical estate,
then deliberately reinstall all infrastructure and rebuild the application
closure using the selected target. Preserve exact controllers, subnets, physical
identities and cycle accounting for unchanged physical topology; logical pool
roles may change. No state migration or cross-schema compatibility is promised.
Partial-activation recovery retains its separate existing admission.

Root is also the replicated install-history witness. Before its own replacement,
observations must match the captured source Root. After that exact intended Root
reinstall, reconciliation must match the reviewed target Root and its retained
Candid; a third module, controller change or unrelated version advance cannot
prove the effect. Applied preceding installs retain their journal receipts.

## Batch and evidence

| Batch | Outcome and owner | Required evidence | Status |
| --- | --- | --- | --- |
| R1 | Host explicit reinstall separates source authority from selected target; CLI and runbook describe selection and resume | Native source/target authority rejection; exact target retention; changed and identical Wasm PocketIC wipes, rows and installation fixtures; before/after install interruption; conflicting new request; terminal replay; full estate and cycle accounting; affected Clippy | Ready |

Implementation includes durable source bindings, source observation dispatch,
replacement Root history reconciliation, terminal target verification and all
schema/test fallout. Targeted native and exact disposable-Fleet checks precede
readiness. No broad gate, publication, staging apply or downstream mutation is
part of this batch. R1 and the complete .15 draft are ready for release review;
[qualification and retained evidence](../../reports/2026-09/2026-09-11/canic163-selected-build-reinstall.md)
cover 26 focused native/CLI/help tests and the complete disposable-Fleet journey.
Toko wrapper adoption and its actual application fixtures require downstream
qualification after this Canic seam is released.

The first disposable changed-build run exposed an Intent/Issued distinction:
Intent is durable before the install call, while Issued is recorded after its
response. Lost-response reconciliation must therefore admit the exact intended
replacement Root in either pending state and still verify management deployment
history. Requiring Issued incorrectly rejects a successfully installed Root
whose response was lost. The nearest invalid cases remain another target,
module, controller set, subnet or unproved deployment history.
