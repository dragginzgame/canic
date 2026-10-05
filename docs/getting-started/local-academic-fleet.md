# Named Local Fleet

This runbook uses a named ICP CLI environment such as `academic`. The same
rules apply to any externally managed local target.

## Target Hygiene

Keep Canic's selector explicit and do not leak unrelated network variables:

```bash
env -u ICP_NETWORK icp --version
canic --environment academic replica status
```

Enroll the exact root key before Fleet observation. Obtain its expected SHA-256
fingerprint from the trusted replica operator through an authenticated or
independent trusted channel. Copy that value below; compare the local digest
with it. Deriving the expected value from the same untrusted key download does
not authenticate the replica. Canic rejects a mismatch before enrollment.

```bash
network_root_fingerprint='<64-lowercase-hex-from-the-replica-operator>'
sha256sum ./academic-root-key.der
canic network enroll academic \
  --root-key ./academic-root-key.der \
  --fingerprint "$network_root_fingerprint"
```

## Build

```bash
canic --environment academic build <app> --profile fast
```

Use an explicit release profile for production artifacts. Build environment
and Fleet environment remain distinct identities; the desired Fleet plan binds
the exact bytes it resolves.

## Ensure

Create `fleets/<fleet>.toml` with `environment = "academic"`, exact current
canister Principals, subnet placement, controllers, artifacts and cycle bounds.
See [Fleet ensure](../features/operations/fleet-ensure.md).

```bash
canic --environment academic fleet ensure <fleet> \
  --desired fleets/<fleet>.toml

canic --environment academic fleet ensure <fleet> \
  --desired fleets/<fleet>.toml \
  --apply <plan_sha256>
```

The selected global environment must equal the desired document. Planning has
no paid Fleet mutation. Apply requires the exact reviewed digest and current
authority. Rerun after interruption; do not reconstruct journals or use a raw
management-canister command to bypass a cycle-safety blocker.

## Sourced Helpers

Do not put `set -e` in a helper sourced into an interactive shell. Use
functions that return status normally:

```bash
canic_academic_plan() {
  env -u ICP_NETWORK canic --environment academic \
    fleet ensure "$1" --desired "fleets/$1.toml"
}
```

Executable scripts may still use strict shell options.

## Local Persistence

Whether the named environment persists canisters is an ICP launcher property,
not a Canic promise. If its state is discarded, do not assume old canisters
are gone on another endpoint. Any reachable old canister with recoverable
cycles must be represented explicitly in current desired state for reuse or
safe drain.

## Continue From Here

- [Create your first managed application](minimal-managed-fleet.md)
- [Operate a Fleet](../operations/README.md)
- [Browse all documentation](../README.md)
- [Back to the main README](../../README.md)
