# Role-contract manifest templates

These inputs are materialized into invocation-owned workspaces by
`FixtureWorkspace`. `Cargo.toml.fixture` becomes `Cargo.toml` during copying,
then dependency paths are rewritten for that isolated workspace.

They are test data, including a deliberately invalid duplicate-name dependency
case, rather than maintained build workspaces. They have no independent release
or lock selection. Keep the invalid input intact so Cargo's typed failure is
projected through the maintained role-contract boundary. Active integration and
artifact-producing workspaces retain their own tracked lockfiles.
