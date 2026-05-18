# ic-memory

`ic-memory` is the extraction target for durable allocation-governance
infrastructure on the Internet Computer.

The core invariant is:

```text
stable_key -> allocation_slot forever
```

This crate is intentionally generic. It owns stable-key parsing, allocation-slot
descriptors, declaration snapshot sealing, ledger data shapes, policy traits,
substrate traits, and validated allocation sessions. Framework-specific rules,
such as Canic namespaces or memory ID ranges, belong in adapter crates.
