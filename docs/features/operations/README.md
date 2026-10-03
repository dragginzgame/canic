# Operations And Diagnostics

The `canic` command-line program runs on a developer's or operator's computer.
It creates local workspace files, builds canisters, connects to trusted networks,
shows diagnostic information, and prepares reviewed deployment changes.

Commands provide readable output for people and stable JSON for scripts. Both
forms follow the same safety boundaries: inspecting or planning a change does
not silently grant permission to apply it.

```text
inspect current state
        |
        v
prepare a plan  (no paid Fleet effect)
        |
        v
review exact actions, spending bounds, and digest
        |
        v
apply that digest -> verify result -> retry safely if interrupted
```

## What It Provides

See [ICP integration](icp-integration.md) for selected builds, effective config
checks, management visibility and read-only frontend sync verification.

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
Retained seeds must also declare `management_creation_fee_cycles` explicitly
for future pool growth; generation never defaults this fee to zero.

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
