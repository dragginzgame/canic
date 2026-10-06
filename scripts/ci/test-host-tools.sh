#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/host-tools-test.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
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
    printf '#!/usr/bin/env bash\necho executed >> "$HOST_TOOLS_FIXTURE/executions"\necho "%s"\n' "$report" > "$fixture/$tool"
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
consumer="$fixture/consumer"
install() { bash "$ROOT/scripts/dev/install-host-tools.sh" --consumer "$consumer" --versions "$pins" "$@"; }
refuse() { if "$@" > "$fixture/refusal.log" 2>&1; then echo 'host-tool refusal failed' >&2; exit 1; fi; }
refuse install --check
[[ ! -e "$consumer/.tools" && ! -e "$fixture/downloads" ]]
for host in Linux:x86_64 Linux:arm64 Darwin:x86_64 Darwin:arm64; do
    export TEST_OS="${host%:*}" TEST_ARCH="${host#*:}"
    consumer="$fixture/$host"; mkdir "$consumer"
    install > /dev/null 2>&1
    before="$(wc -l < "$fixture/downloads")"
    install --check > /dev/null 2>&1
    install > /dev/null 2>&1
    [[ "$(wc -l < "$fixture/downloads")" == "$before" ]]
done
original="$(readlink "$consumer/.tools/host")"
echo changed >> "$consumer/.tools/host/bin/yq"
: > "$fixture/executions"
refuse install --check
[[ ! -s "$fixture/executions" ]]
TEST_INTERRUPT=1 refuse install
[[ "$(readlink "$consumer/.tools/host")" == "$original" && ! -e "$consumer/.tools/.host-tools.lock" ]]
echo corrupt >> "$fixture/assets/yq_darwin_arm64"
refuse install
[[ ! -s "$fixture/executions" && "$(readlink "$consumer/.tools/host")" == "$original" ]]
cp "$fixture/yq" "$fixture/assets/yq_darwin_arm64"
install > /dev/null 2>&1
[[ "$(readlink "$consumer/.tools/host")" != "$original" && -d "$consumer/.tools/$original" ]]
# Authentic bytes with a wrong reported version cannot become active.
original="$(readlink "$consumer/.tools/host")"
cp "$pins" "$fixture/saved.env"
sed 's/jq-1.8.2/jq-1.8.20/' "$fixture/jq" > "$fixture/assets/jq-macos-arm64"
digest="$(shasum -a 256 "$fixture/assets/jq-macos-arm64")"
printf 'export SHARED_TOOLING_JQ_SHA256_DARWIN_ARM64=%s\n' "${digest%% *}" >> "$pins"
refuse install
[[ "$(readlink "$consumer/.tools/host")" == "$original" ]]
cp "$fixture/saved.env" "$pins"
before="$(wc -l < "$fixture/downloads")"
TEST_OS=FreeBSD refuse install
[[ "$(wc -l < "$fixture/downloads")" == "$before" ]]
mkdir "$consumer/.tools/.host-tools.lock"
echo changed >> "$consumer/.tools/host/bin/yq"
refuse install
[[ -d "$consumer/.tools/.host-tools.lock" ]]
echo 'Host tool installation, offline checks and retained failure tests passed'
