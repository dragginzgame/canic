#!/usr/bin/env bash
# Shared companions: scripts/ci/test-tool-evidence.sh
set -euo pipefail
ROOT="$0"
[[ "$ROOT" == /* ]] || ROOT="$PWD/$ROOT"
ROOT="$(cd -P "${ROOT%/*}/../.." && printf '%s/.' "$PWD")"
ROOT="${ROOT%/.}"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/host-tools-test.XXXXXX")"
fixture_complete=false
finish() {
    local status=$?
    # Bash 3.2 may report zero after nounset before assertions finish.
    [[ "$fixture_complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$fixture"
    else
        echo "Host tool fixtures retained: $fixture" >&2
        echo "Host tool fixture stopped with status $status" >&2
        if [[ -f "$fixture/install.log" ]]; then tail -n 80 "$fixture/install.log" >&2; fi
    fi
    exit "$status"
}
trap finish EXIT
mkdir "$fixture/bin" "$fixture/assets" "$fixture/consumer"
export HOST_TOOLS_FIXTURE="$fixture"
cat > "$fixture/bin/uname" <<'SCRIPT'
#!/usr/bin/env bash
case "$1" in -s) echo "${TEST_OS:-Linux}" ;; -m) echo "${TEST_ARCH:-x86_64}" ;; esac
SCRIPT
cat > "$fixture/bin/curl" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
echo download >> "$HOST_TOOLS_FIXTURE/downloads"
if [[ "${TEST_INTERRUPT:-0}" == 1 ]]; then kill -TERM "$PPID"; exit 143; fi
while [[ $# -gt 0 ]]; do
    case "$1" in -o) output="$2"; shift 2 ;; https://*) url="$1"; shift ;; *) shift ;; esac
done
cp "$HOST_TOOLS_FIXTURE/assets/${url##*/}" "$output"
SCRIPT
chmod +x "$fixture/bin/"*
export PATH="$fixture/bin:$PATH"
pins="$fixture/pins.env"
printf 'export SHARED_TOOLING_JQ_VERSION=1.8.2\nexport SHARED_TOOLING_YQ_VERSION=4.47.2\n' > "$pins"
for tool in jq yq; do
    case "$tool" in
        jq) report=jq-1.8.2 ;;
        yq) report='yq (https://github.com/mikefarah/yq/) version v4.47.2' ;;
    esac
    # shellcheck disable=SC2016 # Read fixture location at execution time.
    printf '#!/usr/bin/env bash\necho executed >> "$HOST_TOOLS_FIXTURE/executions"\necho "%s"\n[[ "${TEST_VERSION_WARNING:-0}" != 1 ]] || echo "version warning" >&2\nexit "${TEST_VERSION_STATUS:-0}"\n' "$report" > "$fixture/$tool"
    digest="$(shasum -a 256 "$fixture/$tool")"
    for host in LINUX_AMD64 LINUX_ARM64 DARWIN_AMD64 DARWIN_ARM64; do
        case "$host" in
            LINUX_AMD64) jq_asset=jq-linux-amd64; yq_asset=yq_linux_amd64 ;;
            LINUX_ARM64) jq_asset=jq-linux-arm64; yq_asset=yq_linux_arm64 ;;
            DARWIN_AMD64) jq_asset=jq-macos-amd64; yq_asset=yq_darwin_amd64 ;;
            DARWIN_ARM64) jq_asset=jq-macos-arm64; yq_asset=yq_darwin_arm64 ;;
        esac
        if [[ "$tool" == jq ]]; then key=JQ; asset="$jq_asset"; else key=YQ; asset="$yq_asset"; fi
        cp "$fixture/$tool" "$fixture/assets/$asset"
        printf 'export SHARED_TOOLING_%s_SHA256_%s=%s\n' "$key" "$host" "${digest%% *}" >> "$pins"
    done
done
printf 'export SHARED_TOOLING_RIPGREP_VERSION=15.2.0\n' >> "$pins"
cat > "$fixture/rg" <<'SCRIPT'
#!/usr/bin/env bash
echo executed >> "$HOST_TOOLS_FIXTURE/executions"
case "$1" in
    --version) echo "ripgrep ${TEST_RG_VERSION:-15.2.0} (rev abc123)" ;;
    --pcre2-version) [[ "${TEST_PCRE2:-yes}" == yes ]] || exit 1; echo 'PCRE2 available' ;;
    *) exit 2 ;;
esac
SCRIPT
for host in LINUX_AMD64 LINUX_ARM64 DARWIN_AMD64 DARWIN_ARM64; do
    case "$host" in
        LINUX_AMD64) target=x86_64-unknown-linux-musl ;;
        LINUX_ARM64) target=aarch64-unknown-linux-musl ;;
        DARWIN_AMD64) target=x86_64-apple-darwin ;;
        DARWIN_ARM64) target=aarch64-apple-darwin ;;
    esac
    directory="ripgrep-15.2.0-$target"
    mkdir "$fixture/$directory"
    cp "$fixture/rg" "$fixture/$directory/rg"
    tar -czf "$fixture/assets/$directory.tar.gz" -C "$fixture" "$directory/rg"
    digest="$(shasum -a 256 "$fixture/assets/$directory.tar.gz")"
    printf 'export SHARED_TOOLING_RIPGREP_SHA256_%s=%s\n' "$host" "${digest%% *}" >> "$pins"
done

cat > "$fixture/cloc" <<'SCRIPT'
#!/usr/bin/env bash
echo executed >> "$HOST_TOOLS_FIXTURE/executions"
echo "${TEST_CLOC_VERSION:-2.10}"
exit "${TEST_CLOC_STATUS:-0}"
SCRIPT
cp "$fixture/cloc" "$fixture/assets/cloc-2.10.pl"
digest="$(shasum -a 256 "$fixture/cloc")"
printf 'export SHARED_TOOLING_CLOC_VERSION=2.10\nexport SHARED_TOOLING_CLOC_SHA256=%s\n' "${digest%% *}" >> "$pins"

consumer="$fixture/consumer"
install() {
    printf 'install %s for %s\n' "$*" "$consumer" >> "$fixture/install.log"
    bash "$ROOT/scripts/dev/install-host-tools.sh" --consumer "$consumer" --versions "$pins" "$@" >> "$fixture/install.log" 2>&1
}
refuse() { if "$@" > "$fixture/refusal.log" 2>&1; then echo 'host-tool refusal failed' >&2; exit 1; fi; }
refuse install --check
[[ ! -e "$consumer/.tools" && ! -e "$fixture/downloads" ]] || exit 1
grep -F "tool=jq expected=1.8.2 path=$consumer/.tools/host/bin/jq reason=missing-bundle" "$fixture/install.log"
grep -F 'make install-host-tools' "$fixture/install.log"
for host in Linux:x86_64 Linux:arm64 Darwin:x86_64 Darwin:arm64; do
    export TEST_OS="${host%:*}" TEST_ARCH="${host#*:}"
    consumer="$fixture/$host"; mkdir "$consumer"
    install > /dev/null 2>&1
    for tool in jq yq rg cloc; do [[ -x "$consumer/.tools/host/bin/$tool" ]] || exit 1; done
    before="$(wc -l < "$fixture/downloads")"
    install --check > /dev/null 2>&1
    install > /dev/null 2>&1
    [[ "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
done
original="$(readlink "$consumer/.tools/host")"
# Harmless stderr does not change the established stdout version comparison.
TEST_VERSION_WARNING=1 install --check
for tool in jq yq rg cloc; do
    case "$tool" in jq) expected=1.8.2 ;; yq) expected=4.47.2 ;; rg) expected=15.2.0 ;; cloc) expected=2.10 ;; esac
    mv "$consumer/.tools/host/bin/$tool" "$fixture/removed-tool"
    : > "$fixture/executions"
    refuse install --check
    tail -2 "$fixture/install.log" > "$fixture/diagnostic"
    grep -F "tool=$tool expected=$expected path=$consumer/.tools/$original/bin/$tool reason=missing-or-invalid-executable" "$fixture/diagnostic"
    grep -F 'make install-host-tools' "$fixture/diagnostic"
    [[ ! -s "$fixture/executions" && "$(readlink "$consumer/.tools/host")" == "$original" ]] || exit 1
    mv "$fixture/removed-tool" "$consumer/.tools/host/bin/$tool"
done
TEST_VERSION_STATUS=9 refuse install --check
tail -2 "$fixture/install.log" | grep -F 'tool=jq expected=1.8.2'
tail -2 "$fixture/install.log" | grep -F 'version-probe-failed(exit=9)'
echo changed >> "$consumer/.tools/host/bin/yq"
: > "$fixture/executions"
refuse install --check
[[ ! -s "$fixture/executions" ]] || exit 1
tail -2 "$fixture/install.log" | grep -F 'tool=yq expected=4.47.2'
tail -2 "$fixture/install.log" | grep -F 'reason=authentication-failed actual=not-probed'
TEST_INTERRUPT=1 refuse install
[[ "$(readlink "$consumer/.tools/host")" == "$original" && ! -e "$consumer/.tools/.host-tools.lock" ]] || exit 1
echo corrupt >> "$fixture/assets/yq_darwin_arm64"
refuse install
[[ ! -s "$fixture/executions" && "$(readlink "$consumer/.tools/host")" == "$original" ]] || exit 1
cp "$fixture/yq" "$fixture/assets/yq_darwin_arm64"
install > /dev/null 2>&1
[[ "$(readlink "$consumer/.tools/host")" != "$original" && -d "$consumer/.tools/$original" ]] || exit 1
# Authentic bytes with a wrong reported version cannot become active.
original="$(readlink "$consumer/.tools/host")"
cp "$pins" "$fixture/saved.env"
sed 's/jq-1.8.2/jq-1.8.20/' "$fixture/jq" > "$fixture/assets/jq-macos-arm64"
digest="$(shasum -a 256 "$fixture/assets/jq-macos-arm64")"
printf 'export SHARED_TOOLING_JQ_SHA256_DARWIN_ARM64=%s\n' "${digest%% *}" >> "$pins"
refuse install
[[ "$(readlink "$consumer/.tools/host")" == "$original" ]] || exit 1
cp "$fixture/saved.env" "$pins"
before="$(wc -l < "$fixture/downloads")"
TEST_OS=FreeBSD refuse install
[[ "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
mkdir "$consumer/.tools/.host-tools.lock"
echo changed >> "$consumer/.tools/host/bin/yq"
refuse install
[[ -d "$consumer/.tools/.host-tools.lock" ]] || exit 1

# Authenticate ripgrep before executing it, retaining failed candidates.
cp "$fixture/jq" "$fixture/assets/jq-macos-arm64"
consumer="$fixture/rg-corruption"; mkdir "$consumer"
install > /dev/null 2>&1
original="$(readlink "$consumer/.tools/host")"
echo corrupt >> "$consumer/.tools/host/bin/rg"
: > "$fixture/executions"
refuse install --check
[[ ! -s "$fixture/executions" ]] || exit 1
cp "$fixture/assets/ripgrep-15.2.0-aarch64-apple-darwin.tar.gz" "$fixture/authentic-ripgrep.tar.gz"
echo corrupt >> "$fixture/assets/ripgrep-15.2.0-aarch64-apple-darwin.tar.gz"
refuse install
[[ ! -s "$fixture/executions" && "$(readlink "$consumer/.tools/host")" == "$original" ]] || exit 1
# Restore the exact authenticated bytes. Repacking can change gzip/tar headers
# on native hosts even when the executable payload is unchanged.
cp "$fixture/authentic-ripgrep.tar.gz" "$fixture/assets/ripgrep-15.2.0-aarch64-apple-darwin.tar.gz"
: > "$fixture/executions"
TEST_PCRE2=no refuse install
tail -3 "$fixture/install.log" | grep -F 'tool=rg expected=15.2.0'
tail -3 "$fixture/install.log" | grep -F 'PCRE2-probe-failed'
[[ -s "$fixture/executions" ]] || exit 1
: > "$fixture/executions"
TEST_RG_VERSION=15.2.00 refuse install
[[ -s "$fixture/executions" ]] || exit 1
[[ "$(readlink "$consumer/.tools/host")" == "$original" ]] || exit 1
install > /dev/null 2>&1
[[ "$(readlink "$consumer/.tools/host")" != "$original" ]] || exit 1

# Incomplete prior sets are replaced explicitly, never repaired by a check.
for host in Linux:x86_64 Linux:arm64 Darwin:x86_64 Darwin:arm64; do
    export TEST_OS="${host%:*}" TEST_ARCH="${host#*:}"
    consumer="$fixture/cloc-$host"; mkdir "$consumer"
    # Model an incomplete retained installation without a second install mode.
    install
    original="$(readlink "$consumer/.tools/host")"
    rm "$consumer/.tools/host/bin/cloc"
    refuse install --check
    [[ "$(readlink "$consumer/.tools/host")" == "$original" ]] || exit 1
    install
    [[ "$(readlink "$consumer/.tools/host")" != "$original" && -d "$consumer/.tools/$original" ]] || exit 1
    before="$(wc -l < "$fixture/downloads")"
    install --check
    install
    [[ "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
    (cd "$fixture"; CDPATH="$fixture" bash "$ROOT/scripts/dev/install-host-tools.sh" \
        --consumer "${consumer#"$fixture/"}" --versions "$pins" --check) > /dev/null 2>&1
    bash "$ROOT/scripts/ci/test-tool-evidence.sh" "$consumer" host "$pins"
    # A malformed active link must not authenticate its newline-trimmed sibling.
    original="$(readlink "$consumer/.tools/host")"
    for target in "$original"$'\n' "$original"$'\n\n'; do
        rm "$consumer/.tools/host"
        ln -s "$target" "$consumer/.tools/host"
        [[ ! -e "$consumer/.tools/host/bin" ]] || exit 1
        : > "$fixture/executions"
        refuse install --check
        refuse install
        [[ ! -s "$fixture/executions" && "$(wc -l < "$fixture/downloads")" == "$before" &&
           ! -e "$consumer/.tools/.host-tools.lock" ]] || exit 1
        perl -e 'my $s=readlink($ARGV[0]); exit(defined($s) && $s eq $ARGV[1] ? 0 : 1)' \
            "$consumer/.tools/host" "$target"
    done
    rm "$consumer/.tools/host"
    ln -s "$original" "$consumer/.tools/host"
    install --check
done
original="$(readlink "$consumer/.tools/host")"
TEST_CLOC_STATUS=9 refuse install --check
TEST_CLOC_VERSION=2.100 refuse install
tail -3 "$fixture/install.log" | grep -F 'tool=cloc expected=2.10'
tail -3 "$fixture/install.log" | grep -F 'reason=version-mismatch actual=2.100'
[[ "$(readlink "$consumer/.tools/host")" == "$original" ]] || exit 1
echo corrupt >> "$consumer/.tools/host/bin/cloc"
: > "$fixture/executions"
refuse install --check
[[ ! -s "$fixture/executions" ]] || exit 1
echo corrupt >> "$fixture/assets/cloc-2.10.pl"
refuse install
[[ ! -s "$fixture/executions" && "$(readlink "$consumer/.tools/host")" == "$original" ]] || exit 1
cp "$fixture/cloc" "$fixture/assets/cloc-2.10.pl"
install
# The complete roster is required even when no consumer command uses each tool.
for key in RIPGREP_VERSION CLOC_VERSION; do
    cp "$pins" "$fixture/complete-pins"
    printf 'unset SHARED_TOOLING_%s\n' "$key" >> "$pins"
    before="$(wc -l < "$fixture/downloads")"
    : > "$fixture/executions"
    refuse install --check
    refuse install
    [[ ! -s "$fixture/executions" && "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
    cp "$fixture/complete-pins" "$pins"
done
echo 'Complete host tool installation, offline checks and retained failure tests passed'
fixture_complete=true
