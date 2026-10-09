#!/usr/bin/env bash
set -euo pipefail
root="$0"
[[ "$root" == /* ]] || root="$PWD/$root"
root="$(cd -P "${root%/*}/../.." && printf '%s/.' "$PWD")"
root="${root%/.}"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/file-digests.XXXXXX")"
finish() {
    local status=$?
    chmod u+r "$fixture/unreadable" 2>/dev/null || true
    if [[ "$status" == 0 ]]; then rm -rf "$fixture";
    else echo "Digest fixtures retained: $fixture" >&2; fi
}
trap finish EXIT
checker="$root/scripts/ci/verify-file-checksum.sh"
sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
sha512=ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f
cd "$fixture"
for file in 'file with spaces' '-leading-dash' 'back\slash' $'line\nbreak'; do printf abc > "$file"; done
for backend in gnu perl; do
    mkdir "$backend"
    if [[ "$backend" == gnu ]]; then
        if ! command -v sha256sum >/dev/null || ! command -v sha512sum >/dev/null; then
            echo 'GNU backend unavailable on this host; native shasum is tested instead'
            continue
        fi
        for tool in sha256sum sha512sum; do ln -s "$(command -v "$tool")" "$backend/$tool"; done
    else
        ln -s "$(command -v shasum)" "$backend/shasum"
    fi
    for algorithm in sha256 sha512; do
        expected="${!algorithm}"
        for file in 'file with spaces' '-leading-dash' 'back\slash' $'line\nbreak'; do
            actual="$(PATH="$fixture/$backend" "$BASH" "$checker" --print "$algorithm" "$file")"
            [[ "$actual" == "$expected" ]]
            PATH="$fixture/$backend" "$BASH" "$checker" "$algorithm" "$expected" "$file"
        done
    done
done
expect_failure() {
    local status=0
    "$@" > "$fixture/output" 2> "$fixture/error" || status=$?
    [[ "$status" != 0 && ! -s "$fixture/output" && -s "$fixture/error" ]]
}
expect_failure "$BASH" "$checker" --print sha256 absent
expect_failure "$BASH" "$checker" --print sha256 "$fixture"
expect_failure "$BASH" "$checker" --print md5 'file with spaces'
expect_failure "$BASH" "$checker" sha256 invalid 'file with spaces'
expect_failure "$BASH" "$checker" sha256 "${sha256/ba/aa}" 'file with spaces'
expect_failure "$BASH" "$checker"
printf abc > unreadable
chmod 000 unreadable
if [[ ! -r unreadable ]]; then expect_failure "$BASH" "$checker" --print sha256 unreadable; fi
mkdir empty
expect_failure env PATH="$fixture/empty" "$BASH" "$checker" --print sha256 'file with spaces'
mkdir fake
# Selected backend failures must never be hidden by a successful fallback.
printf '#!%s\nprintf "unexpected fallback" >&2\nexit 99\n' "$BASH" > fake/shasum
# shellcheck disable=SC2016 # Variables are expanded by the generated backend.
printf '#!%s\nprintf "%%s" "$DIGEST_OUTPUT"\nexit "$DIGEST_STATUS"\n' "$BASH" > fake/sha256sum
chmod +x fake/*
for output in '' abc "$sha256 extra" "$sha256  - extra" "$sha512  -" "$sha256  -"$'\n'"$sha256  -"; do
    expect_failure env PATH="$fixture/fake" DIGEST_OUTPUT="$output" DIGEST_STATUS=0 \
        "$BASH" "$checker" --print sha256 'file with spaces'
done
expect_failure env PATH="$fixture/fake" DIGEST_OUTPUT="$sha256  -" DIGEST_STATUS=9 \
    "$BASH" "$checker" --print sha256 'file with spaces'
[[ "$(cat "$fixture/error")" != *'unexpected fallback'* ]]
echo 'Portable digest vectors, filenames, backend failures and verifier round trips passed'
