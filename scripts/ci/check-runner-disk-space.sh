#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
usage: check-runner-disk-space.sh --path PATH --min-free-mib N [--label NAME] [--diagnostic-path PATH]...

Read available capacity on the filesystem containing an existing path.
N is a canonical non-negative decimal integer of at most 18 digits.
Optional diagnostic paths get recursive du -sk totals; failures only warn.
Exit 0: enough space; 1: insufficient space; 2: invalid/unavailable observation.
No installation, cleanup or implicit diagnostic paths.
USAGE
}

fail() { printf 'disk check: %s\n' "$*" >&2; exit 2; }
single_line() { [[ "$1" != *$'\n'* && "$1" != *$'\r'* && "$1" != *$'\t'* ]]; }
operand() {
    # Avoid option interpretation without relying on a BSD/GNU -- extension.
    case "$1" in /*) printf '%s\n' "$1" ;; *) printf './%s\n' "$1" ;; esac
}

selected_path=''
minimum_mib=''
label='runner disk'
diagnostics=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --path|--min-free-mib|--label|--diagnostic-path)
            [[ $# -ge 2 && -n "$2" ]] || fail "missing value for $1"
            single_line "$2" || fail "$1 requires a single-line value without tabs"
            case "$1" in
                --path) selected_path="$2" ;;
                --min-free-mib) minimum_mib="$2" ;;
                --label) label="$2" ;;
                --diagnostic-path) diagnostics+=("$2") ;;
            esac
            shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) fail "unknown argument: $1" ;;
    esac
done
[[ -n "$selected_path" && -n "$minimum_mib" ]] || fail '--path and --min-free-mib are required'
[[ "$minimum_mib" =~ ^(0|[1-9][0-9]{0,17})$ ]] || fail '--min-free-mib must be a canonical non-negative integer of at most 18 digits'
[[ -e "$selected_path" ]] || fail "path does not exist: $selected_path"

# -P selects one record per filesystem; -k fixes units regardless of host/env.
# Check df's exit status separately: plausible partial output is not evidence.
if ! observation="$(LC_ALL=C df -Pk "$(operand "$selected_path")")"; then
    fail "cannot read capacity for $selected_path"
fi
if ! available_kib="$(printf '%s\n' "$observation" | LC_ALL=C awk '
    NR == 1 { header = $1 == "Filesystem" && $2 == "1024-blocks" &&
        $3 == "Used" && $4 == "Available" }
    NR == 2 {
        valid = NF >= 6 && $2 ~ /^[0-9]+$/ && $3 ~ /^[0-9]+$/ &&
            $4 ~ /^-?(0|[1-9][0-9]*)$/ && $5 ~ /^[0-9]+%$/
        digits = $4; sub(/^-/, "", digits)
        valid = valid && length(digits) <= 18
        available = $4
    }
    END { if (NR != 2 || !header || !valid) exit 1; print available }
')"; then
    printf '%s\n' "$observation" >&2
    fail "malformed df -Pk observation for $selected_path"
fi

# Bound decimal input above and divide rather than multiplying the threshold:
# neither arithmetic overflow nor a rounded MiB value can turn a failure green.
available_mib=$((available_kib / 1024))
if [[ "$available_kib" -lt 0 ]]; then
    available_mib=$((- ((-available_kib + 1023) / 1024)))
fi
printf '%s: %s MiB available; %s MiB required; path: %s\n' \
    "$label" "$available_mib" "$minimum_mib" "$selected_path"

# Diagnostics are explicit and best effort. They never redefine gate status.
if [[ ${#diagnostics[@]} -gt 0 ]]; then
    printf 'Disk usage (KiB, selected paths):\n'
    for diagnostic in "${diagnostics[@]}"; do
        if ! LC_ALL=C du -sk "$(operand "$diagnostic")"; then
            printf 'warning: disk usage unavailable for %s\n' "$diagnostic" >&2
        fi
    done
fi
if [[ "$available_mib" -lt "$minimum_mib" ]]; then
    printf 'disk check: insufficient space for %s\n' "$label" >&2
    exit 1
fi
