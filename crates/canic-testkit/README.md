# canic-testkit

Public PocketIC-oriented test utilities for projects that use Canic.

This crate is intentionally limited to generic host-side test infrastructure,
such as the PocketIC wrapper, generic call helpers, ready polling, standalone
non-root Canic canister fixtures, PocketIC `install_code` retry helpers, and
cached baseline primitives. Canic's own root-topology and auth-fixture harness
code belongs in the unpublished `canic-testing-internal` workspace crate
instead of expanding this public surface.
