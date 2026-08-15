# Canic 0.102 Runtime Ops Diagnostic Leaves

Date: 2026-08-15

## Status

This provisional B1 ledger covers configuration lookup, protected deployment
validation, runtime environment access and the request/response dispatcher at
immutable baseline `v0.101.53`. It allocates no numbers. Transparent wrapper
enums preserve their typed cause and receive no duplicate diagnostic identity.

## Configuration Lookups

`ConfigOpsError::Config` preserves the exact `ConfigError` leaf already mapped
in [configuration-leaves.md](configuration-leaves.md). The three direct lookup
failures remain distinct:

| Candidate label | Current producer | Public projection | Action and retry |
| --- | --- | --- | --- |
| `CONFIG_COMPONENT_SPEC_UNAVAILABLE` | `ComponentSpecNotFound` | `RUNTIME_CONFIGURATION_INVALID` | Correct the checked-in Spec reference, rebuild and reinstall |
| `CONFIG_ROLE_UNAVAILABLE` | `CanisterNotFound` | `RUNTIME_CONFIGURATION_INVALID` | Declare the role in the required Spec/package map, rebuild and reinstall |
| `CONFIG_ROLE_AMBIGUOUS` | `CanisterRoleAmbiguous` | `RUNTIME_CONFIGURATION_INVALID` | Make the lookup Spec-qualified or remove the ambiguous declaration |

Spec IDs, role names and package-map descriptions remain in typed
configuration/status data; none enters the compact error.

## Protected Deployment Validation

`ConfigOps::validate_protected_component_deployment` currently destroys a
typed `ProtectedComponentDeploymentError` by formatting it into
`InternalError::invalid_input`. B4 must preserve the cause. Its
`Configuration` variant is a transparent edge to the exact compiled
configuration candidates already mapped in
[configuration-leaves.md](configuration-leaves.md); it receives no wrapper
code.

The ten direct protected-context decisions are exact internal candidates:

| Exact identity | Current typed owner |
| --- | --- |
| `COMPONENT_DEPLOYMENT_BINDING_MISMATCH` | `ProtectedComponentDeploymentError::BindingMismatch` |
| `COMPONENT_DEPLOYMENT_CONFIGURATION_DIGEST_MISMATCH` | `ProtectedComponentDeploymentError::ConfigurationDigestMismatch` |
| `COMPONENT_DEPLOYMENT_UNKNOWN_DEPLOYMENT` | `ProtectedComponentDeploymentError::UnknownDeployment` |
| `COMPONENT_DEPLOYMENT_GROUP_MISMATCH` | `ProtectedComponentDeploymentError::ComponentGroupMismatch` |
| `COMPONENT_DEPLOYMENT_UNKNOWN_MEMBER` | `ProtectedComponentDeploymentError::UnknownMember` |
| `COMPONENT_DEPLOYMENT_COMPONENT_SPEC_MISMATCH` | `ProtectedComponentDeploymentError::ComponentSpecMismatch` |
| `COMPONENT_DEPLOYMENT_COMPONENT_SPEC_HASH_MISMATCH` | `ProtectedComponentDeploymentError::ComponentSpecHashMismatch` |
| `COMPONENT_DEPLOYMENT_PURPOSE_MISMATCH` | `ProtectedComponentDeploymentError::PurposeMismatch` |
| `COMPONENT_DEPLOYMENT_LABELS_MISMATCH` | `ProtectedComponentDeploymentError::LabelsMismatch` |
| `COMPONENT_DEPLOYMENT_LIMITS_MISMATCH` | `ProtectedComponentDeploymentError::LimitsMismatch` |

All ten project publicly to `COMPONENT_DEPLOYMENT_CONTEXT_INVALID`. Their
deployment IDs, member paths, labels and expected/actual values are protected
authority context. The caller action is identical: reject the retained context
and repair/reinstall from the exact compiled App authority. A public error must
never be used to reconstruct that authority.

## Runtime Environment

Seven `EnvOpsError` variants describe absence of one required environment
field: `CanisterRoleUnavailable`, `MissingFields`,
`FleetSubnetRootPidUnavailable`, `RootPidUnavailable`,
`SubnetPidUnavailable`, `ComponentSpecUnavailable` and
`ParentPidUnavailable`. They reuse `ENV_REQUIRED_FIELDS_MISSING`, already
mapped for `EnvPolicyError::MissingEnvFields`. The field-list `String` is
deleted from the diagnostic path. Both producers have the same environment
owner, reinstall action and no-unchanged-retry policy.

The remaining environment paths are:

| Candidate label | Current producer | Public projection | Action and retry |
| --- | --- | --- | --- |
| `ENV_MANAGED_BINDING_UNAVAILABLE` | direct prose in `EnvOps::managed_binding` | `ACCESS_DEPENDENCY_UNAVAILABLE` | Use a Registry-managed Component runtime and retry only after valid initialization |
| `ACCESS_ROOT_REQUIRED` | `EnvOpsError::NotRoot` | existing exact access code | Invoke the root-only operation on the exact configured root |
| `ACCESS_NONROOT_REQUIRED` | `EnvOpsError::IsRoot` | self | Invoke the non-root operation on a managed non-root runtime |
| `ENV_ROOT_AUTHORITY_CONFLICT` | `RootPidImmutable` | `RUNTIME_ENVIRONMENT_INVALID` | Reject the conflicting import; preserve the initialized root authority |
| `ENV_RESTORE_MEMORY_REGISTRY_UNAVAILABLE` | `MemoryRegistryNotInitialized` | `ACCESS_DEPENDENCY_UNAVAILABLE` | Complete memory bootstrap before restoring environment state |

`ENV_MANAGED_BINDING_UNAVAILABLE` remains exact internally because absence is
legitimate for infrastructure runtimes but invalid for a managed-runtime
operation. The public projection must not mislabel it as a foreign-caller
denial. `ACCESS_ROOT_REQUIRED` is a deliberate reuse of the same access
semantic already mapped in [access-leaves.md](access-leaves.md), not a second
root code.

## Request, RPC And Wrapper Ownership

The request dispatcher has one exact direct decision:

| Exact identity | Current typed owner | Public projection | Action and retry |
| --- | --- | --- | --- |
| `RPC_RESPONSE_VARIANT_INVALID` | `RequestOpsError::InvalidResponseType` | `RPC_RESPONSE_INVALID` | Fix the closed `Request`/`Response` dispatcher; unchanged retry cannot repair it |

The following wrappers allocate no code:

- `RequestOpsError::IcInfra` preserves its `IcInfraError` cause;
- `RpcOpsError::RequestOps` preserves the request/IC cause;
- `RpcOpsError::RemoteRejected(Error)` preserves the remote wire diagnostic
  exactly and must not translate it through a local class;
- `RuntimeOpsError` preserves environment, runtime-log and memory causes; and
- `StorageOpsError` preserves the selected storage-owner cause.

The remote error remains untrusted evidence about the remote operation. Its
code may guide the documented caller action, but it never proves local
authority, commitment or absence.

## Current Count

This pass contributes **18 new exact semantic candidates**:

- three configuration lookup leaves;
- ten protected deployment-context leaves;
- four new environment/access leaves; and
- one request-dispatch leaf.

It reuses `ENV_REQUIRED_FIELDS_MISSING`, `ACCESS_ROOT_REQUIRED`, the compiled
configuration leaves and all transparent dependency leaves. It introduces two
additional safe projections:

- `COMPONENT_DEPLOYMENT_CONTEXT_INVALID`; and
- `RPC_RESPONSE_INVALID`.

Neither wrappers nor each missing environment field inflate the count.

## Required Tests

- exhaustive mapping for all direct variants and transparent wrappers;
- protected deployment errors retain typed causes without formatted context;
- every protected deployment detail is absent from the public projection;
- all required-field accessors and import validation share the approved
  environment identity;
- root-required and non-root-required gates remain distinct;
- remote public errors round-trip without local reclassification; and
- response-variant mismatch has one compact identity and no response payload.
