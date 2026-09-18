# CANIC-166 completed-source retirement

Date: 2026-09-17. Scope: host-only recovery admission; no live calls or sibling
mutations. Package remains 0.110.22; changes belong to the open 0.110.23 draft.

## Finding and source identity

Toko's explicit staging reinstall fails because `maximum_successor_actions` is
missing from an informational recovery forecast. The retained continuation
already contains that bound. Filling the forecast or recomputing the old plan
hash would incorrectly turn historical bytes into current executable authority.

Read-only feedback snapshot: sibling `docs/upstream/canic.md`, SHA-256
`ea41849e8b0cf2cb11854b4bb8246e8c8610725b04eb08b9ca511a1165aceb20`.
The exact source directory is
`.canic/fleet-ensure/staging/toko-miner-staging-001` in Toko Miner.

| Evidence | Identity |
| --- | --- |
| Operation | `a418ce5eaf8d6ff9e10d0b9dc031276eb10a78d02a9aa7b388eb82736b1ac638` |
| Journal's plan reference | `ce2504a5098f933af98a95a1361e02e53ed008a1a0552fa9e5a535d442e659ec` |
| Raw plan SHA-256 | `706763802d46979ade2d9083d5806845d72a26959064f967dd540a8204c770c8` |
| Raw journal SHA-256 | `99c87d1264b4adc13a92f9e803594d83fb1824ac051bdef1065bd408f65a6698` |
| Raw state SHA-256 | `a9ac8f974689a56fdc6d377c311fcea684fac10554c7909fcca8ddc36df5e985` |

The current receipt/phase decoders recognize 61 Applied actions: three native
funding/install actions and two immutable protocol phases with 24 and 34 actions.
The original plan reference remains an evidence label, not a verified current
plan digest. Raw bytes bind the historical plan; current canonical phase hashes
and exact action hashes bind the recognized completed evidence.

## Implementation

An ordinary decode failure can now identify a bounded completed-source review.
Explicit `--reinstall` inspects that evidence without executing or rewriting it.
Only completed native funding/install plus protocol phases qualify. Native
payments require the original receipt and recorded before/after balance proof;
paid retry counters use the same model invariant as ordinary journal validation.

Source-bound live inventory must match every retained principal, parent, role,
module, protocol binding and registry. Exact operator debit and ledger fees,
unchanged Root account balances and pool identities, and controlled-cycle burn
must reconcile within the original bounds. Creation histories, funding reviews,
transfers, unresolved effects and unexplained changes reject this bounded path.

A separate preparation record includes raw source hashes and measured source
conservation. Apply repeats source and live admission before adoption. The
existing handoff archives source plan/journal/state and every phase, validates
phase content references on interrupted recovery, and commits the replacement
pair before any effect can run. Completed handoff cannot roll back later journal
progress. Existing preparation, reset and full-Ensure owners execute new effects.

## Focused evidence

- Exact reported source: one-off read-only inspector qualification passed; helper
  removed afterward. Log: `/tmp/canic-terminal-exact-source-final.log`.
- Nineteen focused host regressions pass, including completed-source evidence,
  receipt/counter rejection, Root account and pool membership checks, module/
  parent/role drift, pre-apply conservation rejection, phase archives, every local
  handoff boundary, existing activation handoff and protected funding admission.
  Log: `/tmp/canic-terminal-final-regression.log`.
- CLI retained-input selection regression passes, including completed wipe replay
  selecting its reviewed input when the working TOML is absent.
  Log: `/tmp/canic-terminal-cli-regression.log`.
- Warning-denied host/CLI library and test Clippy passes. Final host inventory
  comparison also passes scoped Clippy: `/tmp/canic-terminal-final-clippy.log`.
- Focused source secret scan and whitespace checks pass. No broad validation ran.

The crash-corruption tests have valid uncorrupted replacement documents and a
healthy recovery control; corruption is the reason for rejection. Temporary
qualification helpers and absolute sibling dependencies are not retained in tests.

## Limits and release boundary

Retained receipt inspection is not an independent historical Ledger-block query.
The exact Toko source passed local inspection, not fresh live conservation or
staging admission. No new deployment, paid effect, reset, real interruption/retry
or terminal deployment replay was performed. Those downstream acceptance steps
remain open and must use exact separately reviewed authority. Existing controlled
Canic effects were not replaced with a new installer.

This host change does not alter canister runtime contracts and therefore does not
itself require rebuilding Toko's already-qualified game release. The selected
source/target artifacts and Candid contracts still must pass normal admission.
The local .23 dependency/diagnostics/recovery batch is ready for its maintainer
release flow; this is not a claim that CANIC-166 is closed in staging.

## Pre-push missing-field confirmation — 2026-09-18

Read-only reinspection confirms the reported staging plan, journal and state
still match the three raw hashes above. The forecast lacks
`maximum_successor_actions`, `fixture_publication_retry_attempts`,
`per_step_burn_cycles` and `startup_funding`; the actual continuation authority
retains `maximum_successor_actions = 73`. The completed journal still has 61
Applied effects and two phases. The earlier exact-source inspector evidence
therefore remains applicable to these unchanged source documents.

The maintained recovery fixture now reproduces that forecast field set rather
than an arbitrary opaque object. Seven focused host regressions pass against it,
including successful separate preparation review, receipt and conservation
rejection, source-byte preservation and interruption recovery at every local
handoff boundary. An additional negative case removes the actual continuation
bound: both source inspection and explicit reinstall reject with typed errors,
without effects or source-document mutation. No missing authority is defaulted.

Validation: `cargo test --locked -p canic-host --lib terminal_retirement_` and
warning-denied host library/test Clippy pass. Logs:
`/tmp/canic-terminal-prepush-regression.log` and
`/tmp/canic-terminal-prepush-clippy.log`. Formatting and whitespace checks pass.
The fix remains included in the open .23 release; no runtime change was needed
for this confirmation. Actual staging inventory/conservation review and deployment
acceptance remain outstanding. No sibling write, network effect or push occurred.
