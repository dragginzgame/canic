# Canic Contracts

Runtime-free wire unions, passive DTOs, identifiers, diagnostic codes, and byte
codecs shared by Canic endpoints, Core, Control Plane, Host, and CLI.

Capability-selected endpoint unions expand from the same declarations used by
ordinary consumers. This package does not own state, storage, lifecycle, timers,
authentication policy, or platform effects.

Use `dto::wire` for complete role contracts and `dto::wire::projection` for
purpose-specific bounded transports. Each projection states why it limits the
schema and is checked against canonical selector payloads. Nested reply unions
check every retained label; `candid::Reserved` skips explicitly unused receipt
data. Unknown operation variants remain a transport failure for that caller.

Production callers and framework fixtures reuse identical bounded shapes,
including replies that differ only in Rust boxing. The shared
`dto::role::OperationAcceptedResponse` carries asynchronous acceptance from Root,
managed and Store commands. Unique fixture projections retain their own bounds
and canonical compatibility checks.

Stable-memory codecs and registration remain with Core/Control Plane. Their
storage records wrap the shared values while retaining the exact fixed-width
cycle bytes and CBOR template keys. Runtime configuration compilation, policy,
authority checks and effects remain outside this package.

Core retains internal intent IDs and bounded stable resource keys. Control Plane
retains `installation::FleetCoordinatorInitArgs`, which includes Core's compiled
runtime configuration. Neither edge belongs to the closed wire vocabulary.

The `canic` facade exposes these types to applications. Direct users of the
former Core/Control Plane DTO and shared-ID paths must use this package. The
maintainer authorized this Rust API cut within 0.110 for
[#354](https://github.com/dragginzgame/canic/issues/354).
