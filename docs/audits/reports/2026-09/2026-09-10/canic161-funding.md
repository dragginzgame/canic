# CANIC-161 Workload funding assessment — 2026-09-10

The maintainer identifies the excessive nested loop as Toko application code.
That report changes the attribution: no Canic loop defect is established.
This assessment documents Canic's funding boundary without changing a runtime
policy or modifying Toko Miner or IcyDB.

## Evidence

The downstream CANIC-161 ledger reports all three Game Shards out of cycles
with IC0207 in release
`6ff4c10e350e50d72b231739cc9266122e34f9b0bd20f7b39a87d558b05d7ceb`.
It reports local operator deposits of 10T per shard, followed by successful
enrolment and assigned-shard login. Those are downstream observations, not a
repeated experiment here. They do not measure consumption or establish the
original replenishment policy.

Read-only inspection of the current Toko `apps/toko_miner/canic.toml` finds
1.9T initial cycles for Game Shards and no `topup` policy on any Component or
child. Canic derives `AutomaticTopup` only for an exact configured role with
that policy; its non-Root runtime selects no automatic schedule when it is
absent. Initial funding therefore does not promise continued replenishment.

The retained incident release's Game Shard Wasm is 7,985,849 bytes and matches
its application artifact union SHA-256:
`ad69ce92acfe698e7b0be6f80af7eefd351a28896d0aef6ced0c986a0192f65d`.
Its only Wasm custom section is `icp:public candid:service`; this inspection
does not recover an embedded configuration or prove the installed role policy.
No live Fleet observations or updates were issued.

## Correction and limits

The maintained [funding runbook](../../../../operations/fleet-funding.md#workload-funding-and-application-failures)
now separates initial funding, opt-in Workload replenishment and protected Root
funding. It gives the existing `canic info cycles` route for exact Workload
observations and explains the manual deposit boundary. The Canic CLI's direct
top-up target remains Coordinator or an exact current Root; a cycles-ledger
transfer is not a canister deposit. Public errors and controller authorization
remain unchanged.

Application repair belongs in Toko. Additional funding does not repair a loop,
and policy enablement should follow measured normal consumption and reviewed
parent limits. This work does not invent another absolute instruction budget,
automatically enable replenishment, or assert that the current config caused
the historical incident. It also does not attribute the incident to metrics.

Six existing Canic native top-up tests pass: exact-role capability selection,
non-Root parent policy including absent-policy handling, Root policy/switch,
minimum spacing, deadline overflow and bounded deterministic retry. Command:
`RUSTC_WRAPPER= CARGO_NET_OFFLINE=true cargo test --locked -p canic-core --lib automatic_topup`.
The [structured receipt](canic161-funding.json) binds the inspected sources,
artifact, command and retained output. No production source was changed for
this assessment, so no PocketIC or broad suite was run.

CANIC-161 remains partially assessed. Exact installed policy, incident burn,
application-loop repair evidence and representative post-recovery behaviour
remain unqualified here. The documented manual-funding boundary is complete;
the downstream incident itself is not declared resolved. This documentation
extends the existing open 0.110.14 batch without a version or Git operation.
