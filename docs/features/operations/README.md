# Operations And Diagnostics

The `canic` binary is the operator surface for local workspace setup, builds,
network trust, evidence, diagnostics, and current desired-state Fleet
convergence. Human-readable output and stable JSON modes serve interactive and
automated workflows without merging their authority.

## What It Provides

- App creation, role scaffolding, attachment, and configuration inspection
- canonical network enrollment and local replica lifecycle
- current desired-state generation from release authority and either explicit
  live-verified estate identities or a durable no-effect fresh-estate seed
- one reviewed `canic fleet ensure` plan/apply workflow
- exact canister dispositions and cycle-conservation bounds
- deployment evidence and passive policy gates
- durable intent and exact replay for ambiguous effects

Useful orientation commands are:

```bash
canic help
canic fleet generate staging --app-config canic.toml --release-build <sha256>
canic fleet ensure staging
```

Add `--fresh --management-creation-fee-cycles <exact-fee>` to generation when
the selected environment has no retained estate seed or live Fleet canister.

## Boundary

The CLI may read and write workspace/operator state and invoke the installed
`icp` binary. Live canisters never gain filesystem or identity-key access.
Planning has no paid Fleet effect; mutations require the exact reviewed
`--apply <plan_sha256>` digest.

App, Fleet, and workspace are distinct terms and must not be conflated.

## Start Here

- [Installing Canic](../../../INSTALLING.md)
- [CLI guide](../../../crates/canic-cli/README.md)
- [Fleet ensure](fleet-ensure.md)
- [Operations index](../../operations/README.md)
- [Release validation matrix](../../operations/release-validation-matrix.md)
- [Recovery and retry runbooks](../../operations/recovery-retry-runbooks.md)
- [Supported platforms](../../governance/supported-platforms.md)
