# CI Diagnostics

Canic provides one offline workspace validation command for ordinary CI:

```text
canic medic --ci
```

Workspace Medic includes the same declared-state audit produced by
`canic state audit`. Its concise output exposes that result as `state_audit`
alongside total check, warning and failure counts. Downstream CI does not need
to run both commands.

Use `canic state audit --ci` when the state-contract findings need to be
isolated from the rest of Medic. A passing workspace audit prints only its
status and counts. Warning and failure output includes only the actionable
finding rows; warning behavior and process exit codes are unchanged.

Neither workspace Medic nor state audit inspects live canisters. Run the
Fleet-scoped command after a local or remote Fleet has started when live drift
must be checked:

```text
canic medic fleet <fleet> --ci
```

Bare `canic medic` intentionally has workspace scope. Without an explicit
environment, Fleet-only checks are `not_evaluated`; their absence is not a
workspace warning.
