#!/usr/bin/env bash
# Shared companions: scripts/ci/release-timing.pl
# One active phase per shell; callers retain their existing EXIT/recovery owner.
_release_timing_helper="${BASH_SOURCE[0]%/*}/release-timing.pl"
release_timing_token=''
release_timing_begin() {
    [[ -n "${RELEASE_TIMING_JOURNAL:-}" ]] || return 0
    release_timing_token="$(perl "$_release_timing_helper" begin "$1" "${2:-executed}" "${3:-}")"
}
release_timing_end() {
    [[ -n "$release_timing_token" ]] || return 0
    perl "$_release_timing_helper" end "$release_timing_token" "$1" || return
    release_timing_token=''
}
