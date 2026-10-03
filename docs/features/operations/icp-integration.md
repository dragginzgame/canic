# ICP CLI integration

Canic requires ICP CLI `>=1.5.0, <2.0.0`; maintainer installation pins 1.6.0.

<img src="../../../assets/256x256/mechanic-note.png" align="left" width="96" alt="The Canic mechanic beside the ICP integration boundary" />

**Integration outcome:** Canic uses ICP's selected environment, build metadata,
management inspection, and YAML project boundary without introducing a second
form of Canic configuration or a parser inside canister Wasm.

<br clear="left" />

## At A Glance

| Concern | Section |
| --- | --- |
| Keep TOML and ICP YAML ownership separate | [Configuration Format Boundary](#configuration-format-boundary) |
| Select an ICP environment and role build | [Environment And Selective Builds](#environment-and-selective-builds) |
| Inspect management state safely | [Management Inspection](#management-inspection) |

## Configuration Format Boundary

Canic-owned human-authored configuration uses TOML, including `canic.toml` and
new configuration surfaces. Cargo manifests also use TOML. YAML configuration
handling is confined to host-side inspection of ICP's `icp.yaml`; ICP owns that
format and its interpretation. Canic does not offer interchangeable TOML and
YAML forms of its own configuration. The contributor rule lives in
[AGENTS.md](../../../AGENTS.md#configuration-formats).

The shared `toml` dependency handles Canic configuration and Cargo documents.
`canic-host` uses `serde-saphyr` with only deserialization enabled for ICP
inspection. Neither parser belongs in a canister's Wasm dependency graph.
Host build scripts parse configuration into generated typed runtime data; Root
also embeds compact TOML source for configuration reporting. That source is
text, not a runtime TOML parser. Host build-script and procedural-macro
dependencies must be distinguished from dependencies compiled into Wasm when
reviewing Cargo graphs.

This choice applies to configuration formats. JSON remains part of the Root
bootstrap release-manifest contract, CBOR serves stable storage, and Candid
serves canister interfaces. Their runtime uses are separate from configuration
parsing. Reuse the existing workspace format libraries for new Canic code;
upstream transitive parsers do not justify introducing another direct parser.

## Environment And Selective Builds

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

## Management Inspection

Host observation accepts additive ICP CLI JSON fields while validating the
fields it consumes. Unknown visibility variants still reject because their
observation semantics are undefined.

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

Short-lived canister, identity, balance and network-status request processes
default to `TOKIO_WORKER_THREADS=2` when that variable is unset. This avoids
creating a worker per available CPU for each small request on large hosts.
Explicit inherited or command-specific settings remain authoritative. The base
command contexts used to start replicas receive no new worker setting. Two is
a request-process default, not a Fleet capacity or canister execution limit.

Typed calls, Fleet initialization and Observatory transport share one private
temporary-argument writer. It completes the write and closes the file before
launching the child, retaining exclusive creation and Unix `0600` permissions.
These invocation files do not require a durable disk flush; callers remove them
after use and recreate them for another invocation. Fleet argument limits and
the separate durable intent/journal writes remain in force.

See [frontend handoff](frontend-handoff.md) for read-only post-sync verification.

## Continue From Here

- [Install the supported toolchain](../../../INSTALLING.md)
- [Read the Frontend Handoff guide](frontend-handoff.md)
- [Review supported platforms](../../governance/supported-platforms.md)
- [Browse Fleet Operations](README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)
