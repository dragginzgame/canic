#!/usr/bin/env bash
# Shared companions: scripts/ci/test-tool-evidence.sh
set -euo pipefail

ROOT="$0"
[[ "$ROOT" == /* ]] || ROOT="$PWD/$ROOT"
ROOT="$(cd -P "${ROOT%/*}/../.." && printf '%s/.' "$PWD")"
ROOT="${ROOT%/.}"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/ic-tools-test.XXXXXX")"
fixture_complete=false
finish() {
    local status=$?
    # Bash 3.2 may report zero after nounset before assertions finish.
    [[ "$fixture_complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$fixture";
    else echo "IC tool fixture retained: $fixture" >&2; fi
    exit "$status"
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
for tool in quill icp didc ic-wasm wasm-opt; do
    case "$tool" in
        quill) version=0.5.4; report="quill $version" ;;
        icp) version=1.6.0; report="icp $version" ;;
        didc) version=0.6.2; report="didc $version" ;;
        ic-wasm) version=0.11.1; report="ic-wasm $version" ;;
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
            wasm-opt)
                asset="binaryen-version_$version-$arch-$os.tar.gz"
                member="binaryen-version_$version"
                mkdir -p "$fixture/payload/$member/bin" "$fixture/payload/$member/lib"
                cp "$fixture/payload/$tool" "$fixture/payload/$member/bin/wasm-opt"
                echo library > "$fixture/payload/$member/lib/libbinaryen.dylib"
                echo library > "$fixture/payload/$member/lib/library with spaces.dylib"
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
    install --preflight > "$fixture/preflight.log" 2>&1
    [[ ! -s "$fixture/preflight.log" && ! -e "$consumer/.tools" ]] || exit 1
    install > "$fixture/install.log" 2>&1
    [[ -f "$consumer/.tools/ic/lib/libbinaryen.dylib" ]] || exit 1
    before="$(wc -l < "$fixture/downloads")"
    install --check > /dev/null 2>&1
    install > /dev/null 2>&1
    [[ "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
    (cd "$fixture"; CDPATH="$fixture" bash "$ROOT/scripts/dev/install-ic-tools.sh" \
        --consumer "${consumer#"$fixture/"}" --pins pins.tsv --check) > /dev/null 2>&1
    # Comments and record order do not change selection or rewrite provenance.
    original="$(readlink "$consumer/.tools/ic")"
    cp "$consumer/.tools/ic/files.sha256" "$fixture/original-receipt"
    for edit in comments reordered; do
        selected_pins="$fixture/$edit.tsv"
        if [[ "$edit" == comments ]]; then
            { printf '# consumer qualification notes\n'; cat "$pins"; } > "$selected_pins"
        else
            awk '{ rows[NR]=$0 } END { for (i=NR; i>0; i--) print rows[i] }' "$pins" > "$selected_pins"
        fi
        cp "$selected_pins" "$fixture/caller-pins"
        install --check > /dev/null 2>&1
        install > /dev/null 2>&1
        [[ "$(wc -l < "$fixture/downloads")" == "$before" &&
           "$(readlink "$consumer/.tools/ic")" == "$original" &&
           ! -e "$consumer/.tools/.ic-tools.lock" ]] || exit 1
        cmp "$consumer/.tools/ic/pins.tsv" "$pins"
        cmp "$consumer/.tools/ic/files.sha256" "$fixture/original-receipt"
        cmp "$selected_pins" "$fixture/caller-pins"
    done
    selected_pins="$pins"
    # Compaction retains distinct caller and installation provenance too.
    bash "$ROOT/scripts/ci/test-tool-evidence.sh" "$consumer" ic "$fixture/comments.tsv"
    # A malformed active link must not authenticate its newline-trimmed sibling.
    original="$(readlink "$consumer/.tools/ic")"
    for target in "$original"$'\n' "$original"$'\n\n'; do
        rm "$consumer/.tools/ic"
        ln -s "$target" "$consumer/.tools/ic"
        [[ ! -e "$consumer/.tools/ic/bin" ]] || exit 1
        : > "$fixture/executions"
        expect_failure install --check
        expect_failure install
        [[ ! -s "$fixture/executions" && "$(wc -l < "$fixture/downloads")" == "$before" &&
           ! -e "$consumer/.tools/.ic-tools.lock" ]] || exit 1
        perl -e 'my $s=readlink($ARGV[0]); exit(defined($s) && $s eq $ARGV[1] ? 0 : 1)' \
            "$consumer/.tools/ic" "$target"
    done
    rm "$consumer/.tools/ic"
    ln -s "$original" "$consumer/.tools/ic"
    install --check > /dev/null 2>&1
done
(
    cd "$fixture"
    bash "$ROOT/scripts/dev/install-ic-tools.sh" --consumer "$consumer" --pins pins.tsv --check > /dev/null 2>&1
)
# A text file's final record remains a selected tool without a final newline.
# Put this host's wasm-opt last so both installation and offline admission must
# consume it, rather than merely accepting the same complete matrix in awk.
for host in Linux:x86_64 Darwin:x86_64 Darwin:arm64; do
    export IC_TOOLS_TEST_OS="${host%:*}" IC_TOOLS_TEST_ARCH="${host#*:}"
    case "$host" in
        Linux:*) pin_host=linux-x86_64 ;;
        Darwin:x86_64) pin_host=darwin-x86_64 ;;
        Darwin:arm64) pin_host=darwin-arm64 ;;
    esac
    selected_pins="$fixture/no-final-newline-$pin_host.tsv"
    awk -F '\t' -v host="$pin_host" '
        $1 == "wasm-opt" && $3 == host { final=$0; next }
        { print }
        END { printf "%s", final }
    ' "$pins" > "$selected_pins"
    consumer="$fixture/no-final-newline $host"
    mkdir "$consumer"
    install > "$fixture/no-final-newline-$pin_host.log" 2>&1
    [[ -x "$consumer/.tools/ic/bin/wasm-opt" ]] || {
        echo "IC setup omitted the final selected tool for $pin_host" >&2; exit 1;
    }
    cmp "$selected_pins" "$consumer/.tools/ic/pins.tsv"
    install --check > /dev/null 2>&1
    # Even a receipt matching changed bytes must not bypass its version check.
    sed 's/wasm-opt version 132/wasm-opt version 133/' "$fixture/payload/wasm-opt" > "$consumer/.tools/ic/bin/wasm-opt"
    digest="$(shasum -a 256 "$consumer/.tools/ic/bin/wasm-opt")"
    awk -v hash="${digest%% *}" '$2 == "bin/wasm-opt" { printf "%s  %s\n", hash, $2; next } { print }' \
        "$consumer/.tools/ic/files.sha256" > "$fixture/changed-receipt"
    mv "$fixture/changed-receipt" "$consumer/.tools/ic/files.sha256"
    expect_failure install --check
    rg -F 'wasm-opt version mismatch' "$fixture/refusal.log" > /dev/null
done
export IC_TOOLS_TEST_OS=Linux IC_TOOLS_TEST_ARCH=x86_64
consumer="$fixture/consumer Linux:x86_64"
original="$(readlink "$consumer/.tools/ic")"

# Only equivalent, complete selections may reuse the bundle, before execution.
before="$(wc -l < "$fixture/downloads")"
selected_pins="$fixture/changed.tsv"
for change in version digest other-host host duplicate malformed extra-tool; do
    case "$change" in
        version) awk -F '\t' 'BEGIN { OFS="\t" } $1 == "quill" { $2="0.5.5" } { print }' "$pins" > "$selected_pins" ;;
        digest|other-host)
            changed_host=linux-x86_64
            [[ "$change" != other-host ]] || changed_host=darwin-arm64
            awk -F '\t' -v host="$changed_host" 'BEGIN { OFS="\t" } $1 == "quill" && $3 == host { $4=sprintf("%064d",0) } { print }' "$pins" > "$selected_pins" ;;
        host) sed 's/linux-x86_64/linux-arm64/' "$pins" > "$selected_pins" ;;
        duplicate) { cat "$pins"; head -n 1 "$pins"; } > "$selected_pins" ;;
        malformed) { cat "$pins"; printf 'not a pin record\n'; } > "$selected_pins" ;;
        extra-tool) { cat "$pins"; awk -F '\t' 'BEGIN { OFS="\t" } $1 == "quill" { $1="pocket-ic"; $2="16.1.0"; print }' "$pins"; } > "$selected_pins" ;;
    esac
    : > "$fixture/executions"
    expect_failure install --check
    case "$change" in host|duplicate|malformed|extra-tool) expect_failure install ;; esac
    [[ ! -s "$fixture/executions" && "$(wc -l < "$fixture/downloads")" == "$before" &&
       "$(readlink "$consumer/.tools/ic")" == "$original" ]] || exit 1
done
# Retained pins are independently admitted, not merely stripped of comments.
selected_pins="$pins"
head -n 1 "$pins" >> "$consumer/.tools/ic/pins.tsv"
: > "$fixture/executions"
expect_failure install --check
[[ ! -s "$fixture/executions" && "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
cp "$pins" "$consumer/.tools/ic/pins.tsv"

# Replacing an old six-tool bundle requires explicit setup. Offline checks and
# failed setup preserve its pins, receipts and payload; no old executable runs.
awk -F '\t' 'BEGIN { OFS="\t" } $1 == "quill" { $1="pocket-ic"; $2="16.1.0"; print }' "$pins" >> "$consumer/.tools/ic/pins.tsv"
# shellcheck disable=SC2016 # The retained executable reads its runtime fixture.
printf '#!/usr/bin/env bash\necho executed >> "$IC_TOOLS_TEST_FIXTURE/executions"\necho "pocket-ic-server 16.1.0"\n' > "$consumer/.tools/ic/bin/pocket-ic"
chmod +x "$consumer/.tools/ic/bin/pocket-ic"
digest="$(shasum -a 256 "$consumer/.tools/ic/bin/pocket-ic")"
printf '%s  bin/pocket-ic\n' "${digest%% *}" >> "$consumer/.tools/ic/files.sha256"
cp "$consumer/.tools/ic/pins.tsv" "$fixture/retained-pins"
cp "$consumer/.tools/ic/files.sha256" "$fixture/retained-receipt"
cp "$consumer/.tools/ic/bin/pocket-ic" "$fixture/retained-server"
: > "$fixture/executions"
expect_failure install --check
[[ ! -s "$fixture/executions" && "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
IC_TOOLS_TEST_INTERRUPT=1 expect_failure install
[[ "$(readlink "$consumer/.tools/ic")" == "$original" ]] || exit 1
install > /dev/null 2>&1
[[ "$(readlink "$consumer/.tools/ic")" != "$original" && ! -e "$consumer/.tools/ic/bin/pocket-ic" ]] || exit 1
cmp "$fixture/retained-pins" "$consumer/.tools/$original/pins.tsv"
cmp "$fixture/retained-receipt" "$consumer/.tools/$original/files.sha256"
cmp "$fixture/retained-server" "$consumer/.tools/$original/bin/pocket-ic"
install --check > /dev/null 2>&1
original="$(readlink "$consumer/.tools/ic")"

# Wrong digests never execute downloaded content or replace the active bundle.
selected_pins="$fixture/bad.tsv"
awk -F '\t' 'BEGIN { OFS="\t" } $1 == "quill" && $3 == "linux-x86_64" { $4=sprintf("%064d",0) } { print }' "$pins" > "$selected_pins"
: > "$fixture/executions"
expect_failure install
[[ ! -s "$fixture/executions" && "$(readlink "$consumer/.tools/ic")" == "$original" ]] || exit 1
[[ ! -e "$consumer/.tools/.ic-tools.lock" ]] || exit 1
rg -F 'Failed tool installation retained:' "$fixture/refusal.log" >/dev/null

# Failure after earlier tools succeeded also preserves the previous activation.
awk -F '\t' 'BEGIN { OFS="\t" } $1 == "wasm-opt" && $3 == "linux-x86_64" { $4=sprintf("%064d",0) } { print }' "$pins" > "$selected_pins"
expect_failure install
[[ "$(readlink "$consumer/.tools/ic")" == "$original" ]] || exit 1

# Offline checks reject changed binaries before executing any version command.
selected_pins="$pins"
printf '\nchanged\n' >> "$consumer/.tools/ic/bin/quill"
: > "$fixture/executions"
expect_failure install --check
[[ ! -s "$fixture/executions" ]] || exit 1
install > /dev/null 2>&1
[[ "$(readlink "$consumer/.tools/ic")" != "$original" && -d "$consumer/.tools/$original" ]] || exit 1

# Receipt traversal failure must not activate a candidate or lose its evidence.
original="$(readlink "$consumer/.tools/ic")"
# Change another host's record to require a new candidate with valid Linux assets.
awk -F '\t' 'BEGIN { OFS="\t" } $1 == "quill" && $3 == "darwin-arm64" { $4=sprintf("%064d",0) } { print }' "$pins" > "$fixture/receipt-pins.tsv"
selected_pins="$fixture/receipt-pins.tsv"
cat > "$fixture/bin/find" <<'SCRIPT'
#!/usr/bin/env bash
echo 'receipt traversal failed' >&2
exit 9
SCRIPT
chmod +x "$fixture/bin/find"
expect_failure install
[[ "$(readlink "$consumer/.tools/ic")" == "$original" ]] || exit 1
rg -F 'receipt traversal failed' "$fixture/refusal.log" >/dev/null
rg -F 'Failed tool installation retained:' "$fixture/refusal.log" >/dev/null
rm "$fixture/bin/find"

# Version prefix matches are not accepted, even with an authentic digest.
selected_pins="$fixture/prefix.tsv"
sed 's/quill 0.5.4/quill 0.5.40/' "$fixture/payload/quill" > "$fixture/assets/quill-linux-x86_64"
digest="$(shasum -a 256 "$fixture/assets/quill-linux-x86_64")"
awk -F '\t' -v hash="${digest%% *}" 'BEGIN { OFS="\t" } $1 == "quill" && $3 == "linux-x86_64" { $4=hash } { print }' "$pins" > "$selected_pins"
expect_failure install
rg -F 'version mismatch' "$fixture/refusal.log" >/dev/null

original="$(readlink "$consumer/.tools/ic")"
IC_TOOLS_TEST_INTERRUPT=1 expect_failure install
[[ "$(readlink "$consumer/.tools/ic")" == "$original" ]] || exit 1
[[ ! -e "$consumer/.tools/.ic-tools.lock" ]] || exit 1
rg -F 'Failed tool installation retained:' "$fixture/refusal.log" >/dev/null

# Invalid/incomplete matrices and unknown hosts fail without downloading.
before="$(wc -l < "$fixture/downloads")"
head -n 14 "$pins" > "$selected_pins"
expect_failure install
selected_pins="$pins"
IC_TOOLS_TEST_ARCH=aarch64 expect_failure install
[[ "$(wc -l < "$fixture/downloads")" == "$before" ]] || exit 1
consumer="$fixture/locked"
mkdir -p "$consumer/.tools/.ic-tools.lock"
expect_failure install
[[ -d "$consumer/.tools/.ic-tools.lock" ]] || exit 1
consumer="$fixture/symlink"
mkdir "$consumer"
ln -s "$fixture" "$consumer/.tools"
expect_failure install

echo 'IC tool installation, offline verification, retention and activation tests passed'
fixture_complete=true
