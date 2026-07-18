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
  repair provisions a fresh proof automatically. An application root that
  requires readiness before the first login may call
  `AuthApi::provision_chain_key_delegation_proof_for_issuer_root` after the
  install/reinstall completes.

Do not retain an old child and inject its principal through `subnet_index`.
That index contains service discovery entries, not placement ownership,
pool/slot/capacity records, controller evidence, or an adoption authorization.

## ic-memory 0.11 Format Boundary

Canic binaries built with `ic-memory 0.11.1` cannot open a protected
allocation ledger written by 0.10.x. The dependency deliberately recognizes
the old payload as unsupported and provides no legacy decoder or in-place
migration. Stable application keys and memory IDs are unchanged, but that does
not make the allocation-governance ledger compatible.

Treat adoption of 0.11.1 by an already deployed fleet as a destructive
reinstall. Reinstall the full dependency closure above and restore only through
application-owned export/import paths that are valid for the new installation.
Restoring the old stable-memory image also restores the incompatible ledger.

## Preflight

Before mutation, run:

```bash
canic --environment <environment> deploy check <deployment>
canic --environment <environment> medic deployment <deployment>
```

For an existing verified deployment, `subnet_registry_role_missing` is a hard
failure: a configured bootstrap role is absent from an observed root registry.
If the registry itself cannot be queried, Canic reports an observation gap
instead of claiming corruption.

For delegated-auth issuers, also run:

```bash
canic --environment <environment> auth renewal status <deployment> \
  --issuer <principal>
```

`issuer_unregistered` means topology must be restored before proof renewal can
succeed. It must not be repaired by injecting caller-supplied proof material;
the root readiness facade is valid only after registration and derives the
proof from Canic-owned policy and renewal state.

## Unsupported Recovery

Canic does not provide legacy env seeding, upgrade-argument migration,
registry self-healing, implicit child adoption, stable-layout fallback, or
delegation-proof injection. Those paths would conceal incompatible or
incomplete authoritative state.
