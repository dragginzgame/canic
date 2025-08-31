# Repository Guidelines

## Project Structure & Module Organization
- Workspace: Rust workspace in `Cargo.toml`; shared version/lints.
- Crates: `crates/icu` (core library, benches), `crates/canisters/{root,example}` (IC canisters).
- CI/CD: GitHub Actions in `.github/workflows/` for CI and tagged releases.
- Scripts: Versioning, release, and checks in `scripts/app/`; env helpers in `scripts/env/`.
- Artifacts: Build output in `target/` (ignored).

## Build, Test, and Development Commands
- `make test`: Run all tests (`cargo test --workspace`).
- `make build`: Release build for all crates.
- `make check`: Type-check without building.
- `make clippy`: Lint; warnings are denied.
- `make fmt` / `make fmt-check`: Format or verify formatting.
- Versioning: `make version`, `make tags`, `make patch|minor|major`, `make release`.
- Utilities: `make security-check`, `make git-versions`, `make check-versioning`.

## Coding Style & Naming Conventions
- Language: Rust 2024 edition; keep code idiomatic and minimal.
- Formatting: `rustfmt` via `cargo fmt --all` before committing.
- Linting: `cargo clippy --workspace -- -D warnings`; follow workspace lints in `Cargo.toml`.
- Naming: snake_case for files/functions/modules; PascalCase for types/traits; constants in SCREAMING_SNAKE_CASE.
- Organization: Prefer small modules under `crates/icu/src/`; document public APIs.

## Testing Guidelines
- Framework: `cargo test`; Criterion benches in `crates/icu/benches/`.
- Placement: Co-locate unit tests with modules; use `tests/` for integration when needed.
- Naming: Snake_case test names (e.g., `handles_error_case`).
- Local run: Ensure `make test`, `make clippy`, and `make fmt-check` pass before PR.

## Commit & Pull Request Guidelines
- Commits: Short, imperative subject (e.g., "Add ledger helpers"); group related changes. Version bumps are handled by scripts.
- PRs: Clear description, link issues, list changes; call out breaking changes. Update `CHANGELOG.md` under `[Unreleased]` for user-facing changes.
- CI: PRs must pass tests, clippy, and formatting checks.

## Security & Configuration Tips
- Tags are immutable; never modify historical tagged code. Bump via `scripts/app/version.sh`.
- Prefer pinned git tags for consumers (see `INTEGRATION.md`).
- Verify integrity with `make security-check` and inspect tags with `make git-versions`.
