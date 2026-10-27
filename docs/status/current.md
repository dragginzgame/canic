# Current handoff — 2026-10-27

## Accepted scope and verdict

Continue the accepted 0.112 Blob/Wire hard cut using the
[closeout plan](../audits/working/0.112-push-readiness/plan.md) and its
[structured evidence](../audits/working/0.112-push-readiness/owned-candid-cut.json).
Owned `CandidType`, managed initialization and authority/hash contracts have been
cut to native Wire. The current local consumer qualification is complete; the
full requested cleanup is **not push-ready** because CT-06's external DTO mirrors
still need published Platform contracts and adoption, or an explicit scope decision.
Do not claim a zero-`CandidType` checkout. Required external IC/provider Candid
encoding and its generic call bounds remain necessary even after DTO adoption.

The candidate is dirty `main` at `cf2d3d1fba84a00a76cdbc541ae7e68fe36e9bb5`.
Package versions remain 0.111.0; root and detailed changelogs share the open
0.112.0 draft and describe the implemented hard cuts. No staging, commit, push,
version bump, release receipt, publication or deployment was performed. Siblings
remain read-only. Ordinary continuation authorizes neither broad validation nor
agent delegation. Preserve all existing dirty work.

Production portable Auth and live Backup are explicitly deferred. Their activation
and unavailable-path gates stay closed. Ten unstarted unrelated simplifications
remain follow-up work; they are not release prerequisites. The eight implemented
opportunities and their targeted evidence remain in the plan. Current installed
proof covers Component/subtree recycle and replacement, Root deletion readiness,
and retained timer authority; do not repeat the earlier pending-installation claims.

## Latest CT-06 supply preparation

Platform 0.6.19 remains the newest published version and default-branch exports
still omit the required types. A [concrete upstream patch](../audits/working/0.112-push-readiness/platform-external-contracts.patch)
is prepared in an isolated archive from c75fdff: explicit management/history/ECDSA,
ICRC transfer/memo and namespaced ICRC-21 exports, with migration/inventory notes.
Locked native, Wasm and MSRV 1.91 compilation and targeted strict library lint pass.
The proposed patch adds no dependency and changes no package version or lock.
Runtime tests and full upstream Clippy were not run. Evidence and exact adoption
hazards are in the plan and ct06SupplyPreparation JSON record.

The patch does not yet supply CMC, Cycles Ledger creation, NNS or asset contracts.
Full management settings/mode types also require request-cost and exact-retry
qualification when adopted; local mirror removal alone proves no Wasm saving.
Permission to submit the isolated PR is pending because the original request
excluded commits/pushes. Merge, publication and Canic adoption remain separate.
Canic production source/locks and the sibling working checkout are unchanged.

## Exact selected graphs

| Graph | Wire | Blob / Contracts | Memory / Storage | Host / Build |
| --- | --- | --- | --- | --- |
| Root | 0.7.18 | 0.49.2 | 0.51.5 | 0.68.9 |
| Dedicated Blob consumer | 0.7.17 | 0.49.1 | 0.51.4 | 0.68.8 |
| Embedded Blob consumer | 0.7.17 | 0.49.1 | 0.51.4 | See recorded independent lock |

All applicable graphs select Platform 0.6.19, Auth 0.17.1, Metrics 0.16.3,
Timers 0.31.5 and serde_cbor 0.11.2 with standard features. Root Query is 0.74.3,
Backup 0.44.4 and Testkit 0.68.9. These are separate graphs, not one aligned build.
The embedded lock gained the published ICRC closure during the earlier owned cut;
its retained package versions/checksums did not change. That delta and all three
lock hashes are recorded in the structured evidence. No lock or dependency changed
during the final consumer qualification.

The extracted-package CLI and downstream App have their own captured locks.
Relevant selections match the root; their package archive, CLI and sealed artifact
hashes are recorded separately. The qualified packages are local 0.111.0 candidate
archives, not published 0.112 artifacts.

## Qualification completed

The prior owned-cut evidence includes focused native authority/config/auth,
registry/provisioning/cursor, initialization, management/history, macro/facade and
fixture checks; strict affected-library/application Clippy; canonical DID and
embedded-peer regeneration; dedicated/embedded installed Blob cases; all 11 ingress
cases; checkpoint freeze/restore/delayed messages; Root bootstrap with subtree
recycle/replacement; held Store grant/revocation reply recovery; fast/release exact
runtime binding and malformed/wrong-method refusal. Corrected fixture budgets
preserve production checkpoint budgets and request byte/depth caps.

Final continuation evidence is under `/tmp/canic-closeout-final-20261026/`:

- Generated Blob clients pass against both installed compositions using Node
  24.21.0 and TypeScript 6.0.3. The first attempt omitted the required tool environment;
  its setup failure and passing rerun are both retained. This is Node-driven client
  qualification against PocketIC transport, not a browser UI or live provider proof.
- `make test-packaged-downstream-cli` passes from eight extracted packages:
  four-artifact build, complete cache reuse, installed workload, pre-install failure,
  lost install/controller replies and terminal replay with zero new effects.
  Cold build 622.95s and reuse 34.97s used a debug CLI; they are not production
  performance measurements. The installed case passed in 740.17s.
- Frontend admission, operator Component recovery/export, retained timer authority
  and autonomous Root deletion-readiness targeted PocketIC cases pass. Exact
  commands and outcomes are recorded in the structured evidence.

Production behavior is unchanged. The final timer run exposed two fixture defects:
optional TTL encoding needed a 4 KiB allocation allowance, and rejection injection
had to use an endpoint label because checkpoint admission now records overflow.
Both native and installed regressions pass after the fixture corrections. Source
continuity was verified before them; their exact scope and unchanged locks/index
are recorded separately from the final report edits. No broad suite, live provider, paid operation,
macOS or TEE qualification is claimed. Prior failed attempts remain evidence, with
passing reruns identified separately. Do not repeat broad or equivalent matrices
without a new source change, failure or unresolved acceptance obligation.

The comparable dedicated fast Root is **8,597,817 code-body bytes**, down from
8,802,273 immediately before the owned cut: **204,456 bytes (2.32%) smaller**.
Its full artifact is 9,109,546 bytes, SHA-256
`2d93f41d95edbd5422ea5bd0690a59f49ea4445cdd02da2106307b95fb338c35`.
Both use Wire 0.7.17 and unchanged admission limits. Other fixture graphs are not
comparable baselines. No qualified 40% savings claim is established.

## Remaining ownership work

Platform 0.6.19 and read-only source
`c75fdff26751f771492e5c44f154b218610362f6` lack the required public exports for
management settings/install/chunk/ECDSA, complete replicated-history identities,
and ICRC transfer/memo/consent DTO families. The pinned underlying crates contain
those types. CMC, Cycles Ledger, NNS and asset contracts additionally need an exact
published supplier. CT-06 records concrete Canic paths and required exports.
Resolve this upstream, publish/adopt deliberately, remove matching mirrors and
qualify exact external encoding/authority consumers. No private-crate dependency,
local override, alias or serialization workaround was added.

The maintainer selected removing the unqualified Blob `confirm_credit` wrapper;
uncertain payments and replay obligations remain. Blob owns funding workflows;
Canic owns endpoint mounting/auth/caller context, lifecycle/memory composition,
installation and runtime identity. Canic's Wasm Store remains its artifact registry.

Storage is the separately accepted serde_cbor hard cut: every nested u128 uses
exactly 16 big-endian bytes, without predecessor fallback. Wire DTOs retain native
numbers; IC envelopes and canonical hash arrays keep their own formats. The
unmaintained advisory exception is approved for the recorded package/checksum.
Optional CBOR tag preservation stays disabled for IC envelope compatibility.
The isolated Postcard experiment is measured but **not adopted**; record-owned
limits and installed recovery acceptance remain prerequisites.

Keep endpoint authorization, exact identity/controller bindings, bounded spending,
original retry bytes, uncertain-effect reconciliation, same-release recovery and
cycle conservation. Source hard cuts never authorize disposal of live liabilities.
Current-package release metadata changes and publication remain human-owned.

## Governance and history

Shared Tooling 0.26.20 at `a7f85d45d4212d4c39225f58597580560a333550` was adopted
through its canonical exporter; do not locally edit exported snapshots. Snapshot,
governance and documentation checks passed in that adoption. Its evidence remains
under `/tmp/canic-shared-adoption-20261026.WU2jco/`.

Before each compile or source edit, check host-visible Canic build ownership;
sandbox process lists can hide other sessions using `target/`. Run only owning
focused checks. The plan retains detailed earlier evidence and deferred
opportunities. The pre-reconciliation handoff is preserved in the continuation's
scratch evidence; historical reports and ablation patches remain unchanged.
