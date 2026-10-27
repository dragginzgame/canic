#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-stage-test.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/scripts/release" "$fixture/scripts/ci" "$fixture/bin"
cp "$ROOT/scripts/release/"{stage,adapter}.sh "$fixture/scripts/release/"
printf '#!/usr/bin/env bash\necho 1.2.3\n' > "$fixture/scripts/ci/read-workspace-version.sh"
cat > "$fixture/bin/git" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == ls-files ]]; then
    [[ "${ROSTER_FAIL:-0}" == 0 ]] || exit 31
    printf '%s\0' crates/example/Cargo.toml testing/example/Cargo.toml
elif [[ "$1" == --literal-pathspecs && "$2" == add && "$4" == --pathspec-file-nul ]]; then
    cp "${3#--pathspec-from-file=}" "$STAGED_ROSTER"
else
    exit 32
fi
SH
chmod +x "$fixture/bin/git"
export STAGED_ROSTER="$fixture/staged"
PATH="$fixture/bin:$PATH" bash "$fixture/scripts/release/stage.sh"
printf '%s\0' Cargo.toml Cargo.lock scripts/dev/install_dev.sh release-validation.json \
    CHANGELOG.md docs/changelog/1.2.md \
    integrations/blob-service/consumer/Cargo.lock \
    integrations/blob-service/embedded-consumer/Cargo.lock \
    crates/example/Cargo.toml testing/example/Cargo.toml > "$fixture/expected"
cmp "$fixture/expected" "$STAGED_ROSTER"
rm "$STAGED_ROSTER"
if PATH="$fixture/bin:$PATH" ROSTER_FAIL=1 bash "$fixture/scripts/release/stage.sh"; then
    echo 'incomplete roster was accepted' >&2; exit 1
fi
[[ ! -e "$STAGED_ROSTER" ]]
echo 'Alternate release staging uses the complete governed roster and fails before Git on roster errors'
