#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/ic-tools-test.XXXXXX")"
finish() {
    local status=$?
    if [[ "$status" == 0 ]]; then rm -rf "$fixture";
    else echo "IC tool fixture retained: $fixture" >&2; fi
}
trap finish EXIT
mkdir -p "$fixture/bin" "$fixture/assets" "$fixture/payload"
export IC_TOOLS_TEST_FIXTURE="$fixture"
cat > "$fixture/bin/uname" <<'SCRIPT'
#!/usr/bin/env bash
case "$1" in
    -s) echo "${IC_TOOLS_TEST_OS:-Linux}" ;;
    -m) echo "${IC_TOOLS_TEST_ARCH:-x86_64}" ;;
    *) exit 2 ;;
esac
SCRIPT
cat > "$fixture/bin/curl" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
while [[ $# -gt 0 ]]; do
    case "$1" in
        -o) output="$2"; shift 2 ;;
        --proto|--proto-redir|--connect-timeout|--max-time) shift 2 ;;
        -*) shift ;;
        *) url="$1"; shift ;;
    esac
done
echo "$url" >> "$IC_TOOLS_TEST_FIXTURE/downloads"
if [[ "${IC_TOOLS_TEST_INTERRUPT:-0}" == 1 ]]; then kill -TERM "$PPID"; exit 143; fi
cp "$IC_TOOLS_TEST_FIXTURE/assets/${url##*/}" "$output"
SCRIPT
chmod +x "$fixture/bin/"*
export PATH="$fixture/bin:$PATH"

pins="$fixture/pins.tsv"
: > "$pins"
for tool in quill icp didc ic-wasm pocket-ic wasm-opt; do
    case "$tool" in
        quill) version=0.5.4; report="quill $version" ;;
        icp) version=1.6.0; report="icp $version" ;;
        didc) version=0.6.2; report="didc $version" ;;
        ic-wasm) version=0.11.1; report="ic-wasm $version" ;;
        pocket-ic) version=16.0.0; report="pocket-ic-server $version" ;;
        wasm-opt) version=132; report="wasm-opt version $version (version_$version)" ;;
    esac
    # shellcheck disable=SC2016 # The generated executable reads its runtime fixture.
    printf '#!/usr/bin/env bash\necho executed >> "$IC_TOOLS_TEST_FIXTURE/executions"\necho "%s"\n' "$report" > "$fixture/payload/$tool"
    chmod +x "$fixture/payload/$tool"
    for host in linux-x86_64 darwin-x86_64 darwin-arm64; do
        case "$host" in
            linux-x86_64) target=x86_64-unknown-linux-gnu; os=linux; arch=x86_64 ;;
            darwin-x86_64) target=x86_64-apple-darwin; os=macos; arch=x86_64 ;;
            darwin-arm64) target=aarch64-apple-darwin; os=macos; arch=arm64 ;;
        esac
        case "$tool" in
            quill)
                asset="quill-$os-$arch"
                cp "$fixture/payload/$tool" "$fixture/assets/$asset" ;;
            pocket-ic)
                asset="pocket-ic-$arch-${host%-*}.gz"
                gzip -c "$fixture/payload/$tool" > "$fixture/assets/$asset" ;;
            wasm-opt)
                asset="binaryen-version_$version-$arch-$os.tar.gz"
                member="binaryen-version_$version"
                mkdir -p "$fixture/payload/$member/bin" "$fixture/payload/$member/lib"
                cp "$fixture/payload/$tool" "$fixture/payload/$member/bin/wasm-opt"
                echo library > "$fixture/payload/$member/lib/libbinaryen.dylib"
                tar -czf "$fixture/assets/$asset" -C "$fixture/payload" "$member" ;;
            *)
                package="$tool"; [[ "$tool" != icp ]] || package=icp-cli
                member="$package-$target"
                asset="$member.tar.xz"
                mkdir -p "$fixture/payload/$member"
                cp "$fixture/payload/$tool" "$fixture/payload/$member/$tool"
                tar -cJf "$fixture/assets/$asset" -C "$fixture/payload" "$member" ;;
        esac
        digest="$(shasum -a 256 "$fixture/assets/$asset")"
        printf '%s\t%s\t%s\t%s\n' "$tool" "$version" "$host" "${digest%% *}" >> "$pins"
    done
done

install() { bash "$ROOT/scripts/dev/install-ic-tools.sh" --consumer "$consumer" --pins "$selected_pins" "$@"; }
expect_failure() {
    if "$@" > "$fixture/refusal.log" 2>&1; then echo 'IC tool rejection failed' >&2; exit 1; fi
}
selected_pins="$pins"
for host in Linux:x86_64 Darwin:x86_64 Darwin:arm64; do
    export IC_TOOLS_TEST_OS="${host%:*}" IC_TOOLS_TEST_ARCH="${host#*:}"
    consumer="$fixture/consumer $host"
    mkdir "$consumer"
    install > "$fixture/install.log" 2>&1
    [[ -f "$consumer/.tools/ic/lib/libbinaryen.dylib" ]]
    before="$(wc -l < "$fixture/downloads")"
    install --check > /dev/null 2>&1
    install > /dev/null 2>&1
    [[ "$(wc -l < "$fixture/downloads")" == "$before" ]]
done
(
    cd "$fixture"
    bash "$ROOT/scripts/dev/install-ic-tools.sh" --consumer "$consumer" --pins pins.tsv --check > /dev/null 2>&1
)
export IC_TOOLS_TEST_OS=Linux IC_TOOLS_TEST_ARCH=x86_64
consumer="$fixture/consumer Linux:x86_64"
original="$(readlink "$consumer/.tools/ic")"

# Wrong digests never execute downloaded content or replace the active bundle.
selected_pins="$fixture/bad.tsv"
awk -F '\t' 'BEGIN { OFS="\t" } $1 == "quill" && $3 == "linux-x86_64" { $4=sprintf("%064d",0) } { print }' "$pins" > "$selected_pins"
: > "$fixture/executions"
expect_failure install
[[ ! -s "$fixture/executions" && "$(readlink "$consumer/.tools/ic")" == "$original" ]]
[[ ! -e "$consumer/.tools/.ic-tools.lock" ]]
rg -F 'Failed tool installation retained:' "$fixture/refusal.log" >/dev/null

# Failure after earlier tools succeeded also preserves the previous activation.
awk -F '\t' 'BEGIN { OFS="\t" } $1 == "pocket-ic" && $3 == "linux-x86_64" { $4=sprintf("%064d",0) } { print }' "$pins" > "$selected_pins"
expect_failure install
[[ "$(readlink "$consumer/.tools/ic")" == "$original" ]]

# Offline checks reject changed binaries before executing any version command.
selected_pins="$pins"
printf '\nchanged\n' >> "$consumer/.tools/ic/bin/quill"
: > "$fixture/executions"
expect_failure install --check
[[ ! -s "$fixture/executions" ]]
install > /dev/null 2>&1
[[ "$(readlink "$consumer/.tools/ic")" != "$original" && -d "$consumer/.tools/$original" ]]

# Version prefix matches are not accepted, even with an authentic digest.
selected_pins="$fixture/prefix.tsv"
sed 's/quill 0.5.4/quill 0.5.40/' "$fixture/payload/quill" > "$fixture/assets/quill-linux-x86_64"
digest="$(shasum -a 256 "$fixture/assets/quill-linux-x86_64")"
awk -F '\t' -v hash="${digest%% *}" 'BEGIN { OFS="\t" } $1 == "quill" && $3 == "linux-x86_64" { $4=hash } { print }' "$pins" > "$selected_pins"
expect_failure install
rg -F 'version mismatch' "$fixture/refusal.log" >/dev/null

original="$(readlink "$consumer/.tools/ic")"
IC_TOOLS_TEST_INTERRUPT=1 expect_failure install
[[ "$(readlink "$consumer/.tools/ic")" == "$original" ]]
[[ ! -e "$consumer/.tools/.ic-tools.lock" ]]
rg -F 'Failed tool installation retained:' "$fixture/refusal.log" >/dev/null

# Invalid/incomplete matrices and unknown hosts fail without downloading.
before="$(wc -l < "$fixture/downloads")"
head -n 17 "$pins" > "$selected_pins"
expect_failure install
selected_pins="$pins"
IC_TOOLS_TEST_ARCH=aarch64 expect_failure install
[[ "$(wc -l < "$fixture/downloads")" == "$before" ]]
consumer="$fixture/locked"
mkdir -p "$consumer/.tools/.ic-tools.lock"
expect_failure install
[[ -d "$consumer/.tools/.ic-tools.lock" ]]
consumer="$fixture/symlink"
mkdir "$consumer"
ln -s "$fixture" "$consumer/.tools"
expect_failure install

echo 'IC tool installation, offline verification, retention and activation tests passed'
