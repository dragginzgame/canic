# Canic Features

Canic is a toolbox: an application can use one feature without adopting all of
them. Start with the problem you want to solve, then follow that feature's guide.

Each guide begins with a plain-language overview, explains what the feature can
and cannot do, and links to its configuration and operating instructions.

The feature guides are introductions. [Configuration](../../CONFIG.md) defines
the settings you can write, contracts define exact program-to-program rules,
architecture explains the design, and operations pages provide procedures.

## Feature Guides

| Capability | Use it for | Guide |
| --- | --- | --- |
| Canister runtime | Give Rust canisters startup, storage, timer, call, and monitoring tools | [Runtime](runtime/README.md) |
| Authentication | Decide who may call an application and on whose behalf | [Authentication](authentication/README.md) |
| Fleet orchestration | Safely deploy and manage a group of canisters | [Fleet orchestration](fleet-orchestration/README.md) |
| Scaling and placement | Add canisters and control where they may run | [Scaling and placement](scaling-and-placement/README.md) |
| Builds and evidence | Build Wasm and record evidence about how it was produced | [Builds and evidence](build-and-evidence/README.md) |
| Backup and restore | Verify snapshots and recover a deployment within one release | [Backup and restore](backup-and-restore/README.md) |
| Blob storage | Store large application data, optionally with billing | [Blob storage](blob-storage/README.md) |
| Operations and diagnostics | Set up, inspect, troubleshoot, and operate Canic | [Operations and diagnostics](operations/README.md) |

For the exact delivery boundary of work in progress, see
[Current Status](../status/current.md).
