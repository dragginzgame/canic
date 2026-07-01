# Blob Storage Billing Readiness

This runbook is for local operators and downstream devs validating a canister
that hosts Canic blob-storage billing endpoints.

It does not define product UI behavior, subscription billing, upload pricing,
or automatic funding. Product backends may orchestrate the same status, sync,
and fund endpoints programmatically, but product frontends should not become
responsible for provisioning blob-storage billing during normal upload flows.

## Target

Choose the blob-storage billing host canister as `<canister-or-role>`.

In a multi-canister fleet, this is the canister that:

- was built with blob-storage billing enabled;
- exposes `get_blob_storage_status`;
- exposes `_immutableObjectStorageUpdateGatewayPrincipals`;
- exposes `_immutableObjectStorageFundFromProjectCycles`;
- stores the blob-storage billing configuration;
- owns the Cashier payment account for the current V1 model.

If a product hub orchestrates instance billing, target the instance/backend
canister that actually hosts these endpoints unless the hub exposes a deliberate
diagnostic wrapper.

## Operator Flow

Inspect the billing host endpoint surface:

```text
canic info endpoints <deployment> <canister-or-role>
```

Run targeted medic diagnostics:

```text
canic medic deployment <deployment> --blob-storage <canister-or-role>
```

Default medic may also show a passive blob-storage hint when local Candid
sidecars advertise the billing endpoints:

```text
canic medic deployment <deployment>
```

That passive hint does not call blob-storage status, sync gateway principals, or
fund Cashier.

Check billing status:

```text
canic blob-storage status <deployment> <canister-or-role>
```

Use `--json` for automation. Parsed blob-storage command failures emit a
`blob_storage_error` object to stderr with a stable command error code.

Use `--check-ready` when a script should fail if uploads are not ready:

```text
canic blob-storage status <deployment> <canister-or-role> --check-ready
```

The check remains read-only. It renders normal status output, then exits `4`
when `ready_for_upload = false`. Warning status with `ready_for_upload = true`
does not fail the check. Failed checks print a compact stderr diagnostic with
the parsed readiness state and blocker/warning codes.

If gateway principals are missing, sync them explicitly:

```text
canic blob-storage sync-gateways <deployment> <canister-or-role>
```

Preview the sync call without executing it:

```text
canic blob-storage sync-gateways <deployment> <canister-or-role> --dry-run
```

If funding is needed, use the status `next` command or pass an explicit
unsigned base-10 cycle amount:

```text
canic blob-storage fund <deployment> <canister-or-role> --cycles <amount>
```

Preview the funding call without executing it:

```text
canic blob-storage fund <deployment> <canister-or-role> --cycles <amount> --dry-run
```

Re-check status:

```text
canic blob-storage status <deployment> <canister-or-role>
```

Expected ready state:

- `configured = true`;
- `ready_for_upload = true`;
- gateway principal count is nonzero;
- Cashier balance is available;
- no readiness blockers remain.

## Safety Rules

`status` is diagnostic. It must not sync gateway principals or fund Cashier.

`medic` is diagnostic. It must not sync gateway principals or fund Cashier.

`sync-gateways` is explicit mutation. Use `--dry-run` when you need to inspect
the canister call before executing it.

`fund` is explicit mutation. The CLI does not compute reserve policy itself; it
passes the requested cycle amount to the canister, and the canister endpoint
remains authoritative for reserve and `attached_cycles` behavior.

Production Cashier is not called by CI. Local and PocketIC validation should
use mock Cashier fixtures.

## After Upgrade

After upgrading the target canister, re-run:

```text
canic blob-storage status <deployment> <canister-or-role>
```

Confirm billing configuration, last gateway sync timestamp, gateway principal
count, funding status, and readiness blockers still match expectations.
