# ICP 1.5 integration

Canic requires ICP CLI `>=1.5.0, <2.0.0`; maintainer installation pins 1.5.0.

## Environment and selective builds

ICP script builds use `ICP_CLI_ENVIRONMENT`, which records the selection after
`icp build -e <environment>` is resolved. The inherited `ICP_ENVIRONMENT` remains
only the CLI default. A direct `build_artifact` invocation can supply
`--environment <name>`; a conflicting ICP-selected environment rejects.
The compiler still receives the normalized Canic network class in
`ICP_ENVIRONMENT`. Do not replace that runtime compilation input with the
arbitrary name of an ICP environment.

Config inspection reads bounded YAML structurally. It checks inline canister,
network and environment identities and each App environment's effective required
roles. Omitted `canisters` selects all declared canisters; `canisters: []` selects
none. Omitted `network` uses `local`; explicit environment declarations override
the implicit `local`/`ic` defaults. ICP owns recipe/build/sync interpretation.
External manifest paths and dependency graphs reject with explicit diagnostics;
Canic does not fetch or execute them during inspection.

`canic app check <app>` exits unsuccessfully when the effective config is incomplete.
`icp build -e <environment>` follows ICP's selected membership. `canic build`
continues to assemble the complete configured App and infrastructure artifacts
needed by Fleet operations. ICP selection is not authority to shrink that closure.

## Management inspection

```sh
canic --environment ic inspect management <canister-principal> --json
```

This reports the management status projection, including log, snapshot and status
visibility and cumulative query counters when available. `AllowedViewers` and
`Public` are observation settings, not controller grants. Partial public status
keeps unavailable cycles/settings/status fields absent rather than fabricating
zero balances or control authority. The command never changes visibility settings.
ICP obtains management status through an update, which can incur network charges.
Role-specific `inspect canister` and Observatory queries retain their existing purpose.

Successful version qualification is shared by one `IcpCli` context and its clones
for typed calls. A new context or changed working directory checks again; failures
are not cached. Standalone command runners retain their checks. No network status,
controller set or cycle balance is cached by this optimization.

See [frontend handoff](frontend-handoff.md) for read-only post-sync verification.
