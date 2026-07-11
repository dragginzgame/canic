# Destructive Reinstall Recovery

This runbook defines the supported hard-cut recovery boundary when stable state
cannot be upgraded and a canister must be reinstalled.

## Dependency Closure

A destructive reinstall removes every Canic-owned stable record in the target
canister. Reinstall the target together with every dependent canister whose
authority or placement state was owned by that target.

- Reinstalling a root invalidates its subnet registry and therefore requires a
  fresh deployment of the root-owned fleet.
- Reinstalling a hub or other placement manager invalidates its sharding,
  scaling, directory, assignment, and lifecycle records. Reinstall the
  placement-managed children as part of the same recovery closure.
- Reinstalling an auth issuer removes its active delegation proof. Once the
  issuer is registered in the restored topology, root renewal or issuer lazy
  repair provisions a fresh proof automatically.

Do not retain an old child and inject its principal through `subnet_index`.
That index contains service discovery entries, not placement ownership,
pool/slot/capacity records, controller evidence, or an adoption authorization.

## Preflight

Before mutation, run:

```bash
canic --network <environment> deploy check <deployment>
canic --network <environment> medic deployment <deployment>
```

For an existing verified deployment, `subnet_registry_role_missing` is a hard
failure: a configured bootstrap role is absent from an observed root registry.
If the registry itself cannot be queried, Canic reports an observation gap
instead of claiming corruption.

For delegated-auth issuers, also run:

```bash
canic --network <environment> auth renewal status <deployment> \
  --issuer <principal>
```

`issuer_unregistered` means topology must be restored before proof renewal can
succeed. It must not be repaired by a controller-only proof injection endpoint.

## Unsupported Recovery

Canic does not provide legacy env seeding, upgrade-argument migration,
registry self-healing, implicit child adoption, stable-layout fallback, or
manual delegation-proof provisioning. Those paths would conceal incompatible
or incomplete authoritative state.
