# CANIC-188 issued-import recovery decision

## Current implementation boundary

The .48 Root implements only Reserve, Advance, Settle and Release. Its exact
reservation cannot be amended, and Release requires all source receipts and
terminal conservation. The stopped source therefore cannot progress once the
cumulative reservation is exhausted. Future Host budget generation alone cannot
change that issued authority.

AGENTS.md normally requires reinstall-only release transitions. The maintainer
authorized this scoped repair exception on September 29 after asking for the
fastest recovery. That authorizes in-repository implementation and qualification,
not live execution or changes to sibling repositories.
Reinstalling this Root would discard the active import owner. An ordinary release
upgrade outside this exact repair remains prohibited. New-import corrections and unchanged-seed
publication can be completed independently of this incident recovery decision.

## Accepted implementation exception; live execution not authorized

The one-off repair is implemented against the published .48 source for this exact import:

1. Retain the selected .48 build, exact approved review, Root/module identity,
   protected status, all source custody and retained payment/conservation evidence.
   Bind an immutable repair review to their hashes and a separately qualified
   repair artifact. No general previous-release decoder or adoption lane.
2. Before replacement, prove every destructive source effect is reconciled and
   that the Root is quiescent through an explicitly journaled stop and observed
   stopped state. Preserve all source controllers and native cycles. Any issued
   or unknown destructive effect blocks replacement until reconciled.
3. Build only the minimal repair against the frozen source/state layout. Its
   operation-specific repair must reject changed status, identities, controllers,
   phases, balances, source count or schema. The repair must not import an older
   application schema, replay payments or reset source progress.
4. Reconcile previously completed call allowances only with quiescence and exact
   retained evidence; preserve genuine uncertainty and the original debit/call
   ceilings. Resume the existing import with successful-call allowance settlement.
   Retain exact old/new artifact authority under the original operation; never
   overwrite the original review, journal or build manifest to hide the repair.
5. Qualify the actual public CLI path, including stop/install response loss,
   changed-authority rejection, interruption, conservation and terminal effect-free
   replay. Prove all 24 sources can finish before approving a live repair.

## Implemented route

The original Host review pins Root's module hash. The temporary repair therefore
finishes Root's source work, then restores the original .48 Root bytes **before
Settle and publication**. The original .48 CLI can then finish the unchanged
approved import. This avoids changing the original review, manifest, journal,
Root authority, call limit, debit limit or source progress.

The [incident tools](../../../scripts/dev/canic188/README.md) retain a minimal
patch against checksum-verified published .48 sources. No repair branch is added
to the maintained runtime. The patch settles successful call allowances and adds
one synchronous post-upgrade adjustment, admitted only by the exact frozen public
status hash and Root identity. It preserves the stable record layout. Any changed
record rejects atomically; a second application also rejects. Balances must remain
below the last observation, above the original floor and within the original 4T
observed-debit ceiling.

The status is bound to Root `2ydug-eaaaa-aaaab-qhfca-cai`, sequence 0 and plan
`7c02f983ab1bc9ed1206e7755ab6557c1ddf885b2bd8f01333eafc82554c8a0b`.
Its Candid SHA-256 is
`e5e4065bf8ddf9e694b47ede061f9e07641389ae2ae670dd0c24ee185300be33`.
The exact mixed-source path needs at most 242 further calls: with 94 consumed,
64 remain for bounded reconciliation within the original 400-call authority.
A changed-cost or changed-custody failure still fails closed; these are not new
spending permissions.

## Qualification and execution boundary

The frozen-record native test passes, including unchanged reservation/progress,
changed-record rejection, replay rejection and remaining-call arithmetic. The
repair Wasm builds against the published .48 dependency graph and retains the
original canister export inventory. The maintained Candid endpoint qualifier passes.

A dedicated PocketIC test seeds the retained public record into a separate .48
fixture. It then installs the actual repair artifact, recovers a discarded install
reply by exact module observation, verifies all public import evidence, rejects
reapplication without losing state, and restores the actual original Root artifact.
The same protected import survives restoration, and total observed Root debit stays
inside the original ceiling. This is a state-preservation proof, not a replay of
Toko's entire deployment or its historical private observations.

Separately, the real Root import journey and complete public CLI completed-estate
recovery/replay journey pass. Native tests exercise 24-source mainnet call counts,
conservative quotes and successful/unknown callback accounting. No broad gate ran.

Live execution and Toko repository writes remain unauthorized and unperformed.
Before live use, retain the exact candidate bytes and repair evidence, refresh the
protected status and module/controllers, and observe Root stopped. An issued or
unknown destructive effect, changed status hash, changed authority or insufficient
cycle margin blocks installation. Never retry an uncertain install blindly: inspect
its module and retained status first. Normal publication resumes only after the
original Root artifact is observed again. Keep the repair bundle until original
CLI completion and effect-free replay are retained.
