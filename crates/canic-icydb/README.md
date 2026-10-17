# Canic IcyDB adapter

The adapter owns native canister exports, caller/controller context, secure boot
entropy acquisition and lifecycle/watchdog scheduling. Consumers supply their
actual `icydb` facade binding to `start!`; emitted code is checked against that
binding. The adapter has no runtime IcyDB or Memory dependency, so it cannot create
a parallel allocator or duplicate the application's native database types.
Database semantics, accepted schema authority, retained state and precommit reply
admission remain with IcyDB. Native methods have no Candid envelope. The upstream
SDK management call used for entropy retains its own Candid protocol.
