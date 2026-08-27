# V1 Readiness Checklist

Use this checklist before asking the human maintainer to run the complete
validation and release workflow.

## Workspace And App

- The selected workspace and App configuration are explicit.
- Every deployed role is declared or attached through the maintained App
  surface.
- Release Wasm and optional binary init arguments are deterministic and present.
- Exact provenance is retained where the release process requires it.
- Network trust was enrolled from a reviewed DER root key and fingerprint.

## Desired Fleet

- `schema_version = 1` and the document's Fleet/environment match the command.
- The treasury is an exact controlled present canister.
- Every existing controlled canister has its exact Principal.
- Each missing canister has exact subnet, controllers, initial cycles and Wasm.
- Initial funding is safely above minimum balance plus expected execution burn.
- Ledger fee, creation fee, observation burn, update burn and stall bounds are
  current exact decimal values.
- A replacement or deletion with material cycles has an exact idempotent drain
  method, Candid file and treasury destination.
- No omitted or unknown controlled canister can retain cycles outside the
  reviewed estate.

## Plan Review

Run:

```bash
canic fleet ensure <fleet> --desired <path>
```

- Planning issued no Fleet mutation.
- Every canister disposition is expected.
- Total observed cycles cover the complete controlled estate.
- Retained and scheduled-transfer totals are credible.
- Creation funding, Ledger fees and management fees are separate.
- Maximum observation/update burn is bounded and credible.
- Maximum new funding and maximum operator debit are acceptable.
- The expected post-operation conservation equation balances.
- The exact `plan_sha256` is retained for review.

## Apply And Recovery

Run only the reviewed digest:

```bash
canic fleet ensure <fleet> --desired <path> --apply <plan_sha256>
```

- Apply rejected any changed desired bytes, artifact, authority or unsafe live
  balance.
- Intent was durable before every creation, funding, transfer, installation,
  controller update, start, stop and deletion.
- An ambiguous response was reconciled before retrying.
- Operator debit did not exceed the reviewed maximum.
- No canister was stopped or deleted above the material residual threshold.
- Terminal measured conservation balances exactly within the reviewed burn
  ceiling.
- An immediate second plan and apply contain zero mutation actions.

## Unsupported State

- No historical install/deploy/recovery path is being invoked.
- No old plan, bundle, journal, compatibility flag or version-pair rule is
  treated as current authority.
- If current identities were discarded, all cycle-bearing old canisters are
  still explicitly represented for safe drain or reuse.
- Concurrent applies use the same operator-state root; independent state roots
  are not a distributed lock.

## Validation Boundary

Automated coding work runs only targeted checks. The human deployment/release
workflow owns the full workspace tests, broad PocketIC matrix, validation,
versioning, tagging and publication.
