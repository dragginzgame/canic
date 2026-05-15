#!/usr/bin/env bash
set -euo pipefail

limit="${CHANGELOG_LINE_WIDTH:-88}"
root="${1:-CHANGELOG.md}"

if [ ! -f "$root" ]; then
  echo "missing changelog: $root" >&2
  exit 1
fi

minor="$(
  awk '
    /^## \[[0-9]+\.[0-9]+\.x\]/ {
      version = $2
      sub(/^\[/, "", version)
      sub(/\.x\]$/, "", version)
      print version
      exit
    }
  ' "$root"
)"

if [ -z "$minor" ]; then
  echo "could not find current minor-line header in $root" >&2
  exit 1
fi

detail="docs/changelog/${minor}.md"
failed=0

check_root_current_minor() {
  awk -v minor="$minor" '
    BEGIN {
      in_section = 0
      in_fence = 0
      heading = "^## \\[" minor "\\.x\\]"
    }
    $0 ~ heading {
      in_section = 1
    }
    in_section && /^## \[/ && $0 !~ heading {
      exit
    }
    in_section {
      if ($0 ~ /^```/) {
        in_fence = !in_fence
      }
      if (!in_fence && $0 ~ /^  [^[:space:]]/) {
        printf "%s:%d:root patch bullets must stay on one line:%s\n", FILENAME, FNR, $0
      }
    }
  ' "$root"
}

check_detail_file() {
  if [ ! -f "$detail" ]; then
    return
  fi

  awk -v limit="$limit" '
    /^```/ {
      in_fence = !in_fence
    }
    !in_fence && length($0) > limit {
      printf "%s:%d:%d:%s\n", FILENAME, FNR, length($0), $0
    }
  ' "$detail"
}

output="$(
  {
    check_root_current_minor
    check_detail_file
  }
)"

if [ -n "$output" ]; then
  printf "%s\n" "$output" >&2
  failed=1
fi

if [ "$failed" -ne 0 ]; then
  echo "changelog prose lines must be ${limit} columns or less" >&2
  exit 1
fi

echo "changelog OK: ${root} ${minor}.x single-line bullets; ${detail} <= ${limit}"
