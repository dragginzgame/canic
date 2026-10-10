#!/usr/bin/env bash
# Shared companions: scripts/ci/read-cargo-workspace-version.sh scripts/ci/check-dependency-pins.sh
set -euo pipefail

ROOT="${BASH_SOURCE[0]}"
[[ "$ROOT" == /* ]] || ROOT="$PWD/$ROOT"
ROOT="$(cd -P "${ROOT%/*}/../.." && printf '%s/.' "$PWD")"
ROOT="${ROOT%/.}"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/cargo-metadata-test.XXXXXX")"
fixture_complete=false
finish() {
    local status=$?
    # Bash 3.2 may report zero after nounset before assertions finish.
    [[ "$fixture_complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$fixture"
    else echo "Cargo metadata fixtures retained: $fixture" >&2; fi
    exit "$status"
}
trap finish EXIT
reader="$ROOT/scripts/ci/read-cargo-workspace-version.sh"
manifest="$fixture/Cargo.toml"
for version in 0.1.7 1.0.0-rc.1+build.23; do
    printf "[workspace.package]\nversion = '%s' # inline comment\n" "$version" > "$manifest"
    actual="$(bash "$reader" "$manifest")"
    [[ "$actual" == "$version" ]] || exit 1
done
if bash "$reader" --stable "$manifest" > "$fixture/output" 2>&1; then exit 1; fi
for value in '"01.2.3"' '"1.2.3-01"' '"1.2.3+"' '"1.2.3\n"' '123' '{ workspace = true }'; do
    printf '[workspace.package]\nversion = %s\n' "$value" > "$manifest"
    if bash "$reader" "$manifest" > "$fixture/stdout" 2> "$fixture/stderr"; then exit 1; fi
    [[ ! -s "$fixture/stdout" ]] || exit 1
done
printf '[workspace.package]\nversion = "0.1.7"\nversion = "0.1.8"\n' > "$manifest"
if bash "$reader" "$manifest" > "$fixture/stdout" 2> "$fixture/stderr"; then exit 1; fi
[[ ! -s "$fixture/stdout" ]] || exit 1
# Even plausible partial output from a failed parser is never projected.
cat > "$fixture/failed-parser" <<'PARSER'
#!/usr/bin/env bash
printf '{"workspace":{"package":{"version":"0.1.7"}}}\n'
exit 9
PARSER
chmod +x "$fixture/failed-parser"
printf '[workspace.package]\nversion = "0.1.7"\n' > "$manifest"
if YQ="$fixture/failed-parser" bash "$reader" "$manifest" > "$fixture/stdout" 2> "$fixture/stderr"; then exit 1; fi
[[ ! -s "$fixture/stdout" ]] || exit 1

consumer="$fixture/consumer"
mkdir -p "$consumer/member/src" "$consumer/local/src" "$consumer/testing/src"
git init -q "$consumer"
cat > "$consumer/Cargo.toml" <<'TOML'
[workspace]
members = ["member", "local"]
resolver = "2"
[workspace.package]
version = "0.1.7"
[workspace.dependencies]
alias = { package = "local", path = "local" }
TOML
cat > "$consumer/member/Cargo.toml" <<'TOML'
[package]
name = "member"
version.workspace = true
edition = "2021"
[dependencies.alias]
workspace = true
optional = true
[dev-dependencies]
alias.workspace = true
[target.'cfg(unix)'.build-dependencies]
alias = { workspace = true, features = [] }
TOML
cat > "$consumer/local/Cargo.toml" <<'TOML'
[package]
name = "local"
version.workspace = true
edition = "2021"
TOML
# Independent graphs resolve against their own catalog, never the outer one.
cat > "$consumer/testing/Cargo.toml" <<'TOML'
[package]
name = "testing"
version = "0.2.0"
edition = "2021"
[workspace]
[workspace.dependencies]
alias = { package = "local", path = "../local" }
[dependencies]
alias.workspace = true
TOML
for package in member local testing; do printf '' > "$consumer/$package/src/lib.rs"; done
printf 'version = 3\n' > "$consumer/Cargo.lock"
cp "$consumer/Cargo.lock" "$consumer/testing/Cargo.lock"
git -C "$consumer" add -- .
check() { bash "$ROOT/scripts/ci/check-dependency-pins.sh" --consumer "$consumer" --cargo-inheritance > "$fixture/result" 2>&1; }
reject() { if check; then echo 'invalid inheritance accepted' >&2; exit 1; fi; }
check || { cat "$fixture/result" >&2; exit 1; }
cp "$consumer/member/Cargo.toml" "$fixture/member.toml"
# The audit's ordinary table form, plus target/build/dev declarations.
for section in dependencies dev-dependencies 'target."cfg(unix)".build-dependencies'; do
    cp "$fixture/member.toml" "$consumer/member/Cargo.toml"
    printf '\n[%s.other]\nversion = "1"\n' "$section" >> "$consumer/member/Cargo.toml"
    reject
done
cp "$fixture/member.toml" "$consumer/member/Cargo.toml"
for setting in 'version = "1"' 'path = "../local"' 'package = "local"' 'git = "https://example.invalid/lib"' 'registry = "other"'; do
    printf '\n[build-dependencies.alias]\nworkspace = true\n%s\n' "$setting" >> "$consumer/member/Cargo.toml"
    reject
    cp "$fixture/member.toml" "$consumer/member/Cargo.toml"
done
printf '\n[build-dependencies.missing]\nworkspace = true\n' >> "$consumer/member/Cargo.toml"
reject
cp "$fixture/member.toml" "$consumer/member/Cargo.toml"
sed 's/version.workspace = true/version = "0.1.7"/' "$fixture/member.toml" > "$consumer/member/Cargo.toml"
reject
cp "$fixture/member.toml" "$consumer/member/Cargo.toml"
# Catalog ownership is checked even for a root package's own dependencies.
printf '\n[dev-dependencies.other]\nversion = "1"\n' >> "$consumer/testing/Cargo.toml"
reject
git -C "$consumer" show :testing/Cargo.toml > "$consumer/testing/Cargo.toml"
check || { cat "$fixture/result" >&2; exit 1; }
git -C "$consumer" diff --exit-code
printf 'Cargo metadata tests passed\n'
fixture_complete=true
