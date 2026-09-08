# Canic Fleet Subnet Root

Canonical Fleet Subnet Root entrypoint. Canic owns its source, lifecycle and
endpoints; applications supply configuration through the host build pipeline.
The exact App configuration, selected capabilities and release-build identity
remain compiled artifact authority. Use `canic` to build the Fleet.
Application initialization belongs in application canisters; Root code and
lifecycle hooks are owned by Canic.

Runtime orchestration remains in `canic-control-plane`. Releases use the
pre-1.0 reinstall hard cut; same-release recovery remains supported.
