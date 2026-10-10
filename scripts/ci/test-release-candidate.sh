#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-candidate.XXXXXX")"
finish() {
    local status=$?
    [[ "$status" == 0 ]] || cat "$fixture/output.log" >&2
    rm -rf "$fixture"
    exit "$status"
}
trap finish EXIT
mkdir -p "$fixture/base/member/src" "$fixture/base/scripts/dev" "$fixture/base/docs/changelog" "$fixture/bin"
cat >"$fixture/base/Cargo.toml" <<'TOML'
[workspace]
members = ["member"]
resolver = "3"
[workspace.package]
version = "1.2.3"
[profile.release]
opt-level = 3
TOML
cat >"$fixture/base/member/Cargo.toml" <<'TOML'
[package]
name = "fixture-member"
version.workspace = true
edition = "2024"
[features]
default = []
extra = []
TOML
printf 'pub fn fixture() {}\n' >"$fixture/base/member/src/lib.rs"
printf 'CANIC_CLI_VERSION="%s"\n' '${CANIC_CLI_VERSION:-1.2.3}' >"$fixture/base/scripts/dev/install_dev.sh"
printf '## [1.2.4]\n' >"$fixture/base/docs/changelog/1.2.md"
cargo generate-lockfile --offline --manifest-path "$fixture/base/Cargo.toml" >"$fixture/output.log" 2>&1
for consumer in consumer embedded-consumer; do
    consumer_root="$fixture/base/integrations/blob-service/$consumer"
    mkdir -p "$consumer_root/src"
    cat >"$consumer_root/Cargo.toml" <<TOML
[workspace]
[package]
name = "fixture-$consumer"
version = "0.1.0"
edition = "2024"
[dependencies]
fixture-member = { path = "../../../member" }
TOML
    printf 'pub fn consumer() {}\n' >"$consumer_root/src/lib.rs"
    cargo generate-lockfile --offline --manifest-path "$consumer_root/Cargo.toml" >>"$fixture/output.log" 2>&1
done
cp -R "$fixture/base" "$fixture/candidate"
mkdir -p "$fixture/candidate/scripts/ci" "$fixture/candidate/scripts/release"
cp "$ROOT/scripts/release/rewrite-owned-lock.sh" "$fixture/candidate/scripts/release/"
cp "$ROOT/scripts/release/adapter.sh" "$fixture/candidate/scripts/release/"
for script in check-release-candidate check-release-surface-content read-workspace-version read-cargo-workspace-version require-jq; do
    cp "$ROOT/scripts/ci/$script.sh" "$fixture/candidate/scripts/ci/"
done
cp "$ROOT/scripts/ci/rewrite-local-lock-versions.pl" "$fixture/candidate/scripts/ci/"
cat >"$fixture/bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" != -C ]] || shift 2
case "$*" in
    'log -1 --format=%s HEAD') echo source ;;
    'rev-parse HEAD') echo 1111111111111111111111111111111111111111 ;;
    'diff --name-only 1111111111111111111111111111111111111111 --')
        printf '%s\n' Cargo.toml Cargo.lock scripts/dev/install_dev.sh docs/changelog/1.2.md \
            integrations/blob-service/consumer/Cargo.lock integrations/blob-service/embedded-consumer/Cargo.lock
        [[ -z "${EXTRA_CHANGE:-}" ]] || echo "$EXTRA_CHANGE" ;;
    'cat-file -e 1111111111111111111111111111111111111111:'*)
        test -f "$BASE/${3#*:}" ;;
    'ls-files --others --exclude-standard') ;;
    'archive 1111111111111111111111111111111111111111') tar -cf - -C "$BASE" . ;;
    *) echo "unexpected Git read: $*" >&2; exit 99 ;;
esac
SH
chmod +x "$fixture/bin/git"
export PATH="$fixture/bin:$PATH" BASE="$fixture/base"
cd "$fixture/candidate"
cargo set-version --workspace --offline 1.2.4 >>"$fixture/output.log" 2>&1
cargo update --workspace --offline >>"$fixture/output.log" 2>&1
for consumer in consumer embedded-consumer; do
    cargo update --offline --manifest-path "integrations/blob-service/$consumer/Cargo.toml" >>"$fixture/output.log" 2>&1
done
printf 'CANIC_CLI_VERSION="%s"\n' '${CANIC_CLI_VERSION:-1.2.4}' >scripts/dev/install_dev.sh
printf '## [1.2.4] - 2026-10-01\n' >docs/changelog/1.2.md
printf '{"schema":1,"version":"1.2.4","source":"1111111111111111111111111111111111111111","date":"2026-10-01","gate":"complete"}\n' >release-validation.json
cp Cargo.toml "$fixture/accepted-manifest"
cp Cargo.lock "$fixture/accepted-lock"

run_case() {
    local expected="$1" status=0
    bash scripts/ci/check-release-candidate.sh >>"$fixture/output.log" 2>&1 || status=$?
    [[ "$status" == "$expected" ]]
}
run_case 0
cp integrations/blob-service/consumer/Cargo.lock "$fixture/accepted-consumer-lock"
cp "$BASE/integrations/blob-service/consumer/Cargo.lock" integrations/blob-service/consumer/Cargo.lock
run_case 1
cp "$fixture/accepted-consumer-lock" integrations/blob-service/consumer/Cargo.lock
printf '\n[[package]]\nname = "unreviewed"\nversion = "1.0.0"\n' >>integrations/blob-service/consumer/Cargo.lock
run_case 1
cp "$fixture/accepted-consumer-lock" integrations/blob-service/consumer/Cargo.lock
printf '\n[workspace.metadata]\nchanged = true\n' >>Cargo.toml
run_case 1
cp "$fixture/accepted-manifest" Cargo.toml
sed 's/opt-level = 3/opt-level = 0/' "$fixture/accepted-manifest" >Cargo.toml
run_case 1
cp "$fixture/accepted-manifest" Cargo.toml
printf '\n[dependencies]\nserde = "1"\n' >>member/Cargo.toml
run_case 1
cp "$BASE/member/Cargo.toml" member/Cargo.toml
sed 's/default = \[\]/default = ["extra"]/' "$BASE/member/Cargo.toml" >member/Cargo.toml
run_case 1
cp "$BASE/member/Cargo.toml" member/Cargo.toml
printf '\n[[package]]\nname = "unreviewed"\nversion = "1.0.0"\n' >>Cargo.lock
run_case 1
cp "$fixture/accepted-lock" Cargo.lock
printf '\necho changed-behavior\n' >>scripts/dev/install_dev.sh
run_case 1
printf 'CANIC_CLI_VERSION="%s"\n' '${CANIC_CLI_VERSION:-1.2.4}' >scripts/dev/install_dev.sh
EXTRA_CHANGE=new/Cargo.toml run_case 1
EXTRA_CHANGE=member/src/lib.rs run_case 1
printf '\nReworded explanatory text is not executable release authority.\n' >>docs/changelog/1.2.md
run_case 0
printf '## [1.2.4]\n' >docs/changelog/1.2.md
run_case 1
printf '## [1.2.4] - 2026-10-01\n' >docs/changelog/1.2.md
sed 's/1111111111111111111111111111111111111111/2222222222222222222222222222222222222222/' release-validation.json >"$fixture/receipt"
cp "$fixture/receipt" release-validation.json
run_case 1
echo 'release content fixtures passed (real Cargo, fake Git reads; no commits)'
