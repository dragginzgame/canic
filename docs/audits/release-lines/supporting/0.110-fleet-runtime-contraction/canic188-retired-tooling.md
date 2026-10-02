# CANIC-188 incident repair — retired tooling record

Retired on 2026-10-01 when the maintainer withdrew the repair exception. The
helpers and qualification example described below have been removed. Commands
and live sequences below record historical qualification only; current policy
requires hard cut plus reinstall. Retained incident bundles remain evidence.

This is the maintainer-authorized, operation-specific repair described in the
[recovery decision](canic188-issued-import-recovery.md).
The helpers prepare and qualify artifacts locally. They do not deploy, call the
live IC, or write to Toko Miner.

## Reproduction

`incident.toml` binds the Root, original artifact, operation, configuration and
retained public status. `published-source.sha256` covers every file in the
published Control Plane `.48` archive, whose SHA-256 is
`8436c8aa7e035f2d08c92998b1bafac9af246cd635f215e38bf090304a573078`.
The wrapper inputs are bound by `root-package.sha256`.

```bash
bash scripts/dev/canic188/prepare.sh \
  <published-canic-control-plane-0.110.48-directory> \
  <original-generated-canic-fleet-root-directory> \
  <original-canic.toml> <original-root.wasm>
bash scripts/dev/canic188/build.sh <prepared-directory>
bash scripts/dev/canic188/qualify.sh <read-only-toko-workspace> <prepared-directory>
```

Preparation checks the inputs before copying them into a fresh directory below
Canic's `target/`. The repair patch admits native credits during import,
settles successful-call allowances and adds one synchronous, exact-status-bound
post-upgrade adjustment. Earlier observed debit and uncertain reserves survive
credits; inter-call declines remain charged. It changes no
persisted type, public method, retained receipt, reservation limit or source progress.
The original `.48` dependency graph and configuration remain selected.

The separate fixture patch seeds the public incident record for PocketIC. Its
private Stopped observation is synthetic; no historical management effect is
claimed from that seed. Real PocketIC management installs exercise the repair,
discarded-reply reconciliation, replay rejection and original-artifact restoration.
Only `root.wasm` is the repair candidate. `fixture-root.wasm` is exclusively a
PocketIC fixture and must never enter a live repair bundle.

Qualification requires the pinned PocketIC server and builds only the local
example. The example accepts only an owned loopback server URL. It verifies the
maintained Candid endpoint inventory and checks original reservation/progress,
consumed call count and the observed Root debit ceiling through both replacements.
Native tests separately bind the exact record and 242-call remaining path.
Qualification covers the reported balance of 400,787,627,907,411 cycles and a
larger credit that remains above the original initial balance after installation.
Each case retains a structured `qualification-<balance>.json` record.

## Retained qualified candidate

The revised preparation/build/qualification sequence passed on October 1. Its
repair Wasm SHA-256 is
`48c76028a926df90cc2392024c78f1f6f8dbced74187705ed113c098268b7c38`.
The exact candidate, original Root, status, source hashes, patch and logs are retained
under Canic's `.canic/incident-repairs/canic188/<repair-sha256>/`, outside `target/`.
`bundle.sha256` verifies those retained files; `qualification.json` records the
checks and their limits. The live bundle excludes fixture Wasm. It has not been
deployed; fresh live preconditions below still apply.

The September 29 candidate `ed3084b6908a04b28effa21e00ec425aaf382d1423849fcbb4f3b12414d61f48`
remains retained unchanged as historical evidence. Its positive-balance rejection
is corrected by the October 1 candidate; it is not the selected repair.

Restoration qualifies state preservation. The immutable original `.48` Root and
CLI still reject a net terminal surplus; the reported balance remains below the
original starting balance. This candidate does not qualify arbitrary terminal
credits in those original owners. The maintained import accepts such surplus;
see the [qualification limits](canic188-issued-import-recovery.md#qualification-and-execution-boundary).

## Live execution boundary

Live execution and writes to Toko's operator workspace require separate authority.
Retain the qualified candidate, original Root, status, hashes, patch and logs in
a durable incident bundle before any live action. Build-directory cleanup must
not remove the only copy of an artifact needed to finish an issued repair.

The execution sequence is:

1. Retain the original approved import/journal and verify the selected mainnet
   identity, original Root module, current controllers and exact protected status
   hash. Verify the original Root floor and debit bound still admit repair costs.
2. Record stop intent, stop Root and observe it stopped. Any outstanding
   destructive source effect or changed retained status blocks replacement.
3. Record the exact repair artifact and upgrade intent, install in **upgrade**
   mode, observe its module hash, start Root and retain the adjusted status.
   Recover an uncertain install by observing its module and status; never blindly
   issue another install. The second application rejects atomically.
4. Use only the original operation's protected `Advance` commands for unfinished
   sources. Preserve source order and record each result. A rejection stops the
   sequence; do not change the operation, limits or journal. Existing Ready
   receipts are retained. The original 400-call bound leaves 64 calls beyond the
   remaining mainnet path, and the original 4T actual-debit ceiling still applies.
5. Once all 24 sources are Ready and the Root receipt is still absent, stop Root,
   retain restoration intent and restore the exact original `.48` Root artifact
   in upgrade mode. Observe its hash and start it. **Do this before Settle** so
   both replacements are included in terminal Root accounting.
6. Resume the original `.48` CLI's exact approved import. Its original artifact
   checks, publication and terminal conservation remain authoritative. Retain
   completion and immediate effect-free replay before closing the incident.

The current development CLI expects the new call quote and is not the continuation
client for the restored `.48` Root. Do not alter the original release manifest to
make the temporary repair look like its published artifact. The seed-byte fix
applies to new projections; it does not rewrite an already approved publication.
