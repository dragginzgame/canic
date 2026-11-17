#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
mkdir -p target/ci-setup-tests
fixture=$(mktemp -d "$PWD/target/ci-setup-tests/run.XXXXXX")
helper="$PWD/scripts/ci/setup-diagnostics.pl"
export SETUP_TEST_TOKEN='sentinel-private-token-81924'
run_case() {
    local name="$1" expected="$2" status=0
    shift 2
    mkdir -p "$fixture/$name"
    RUNNER_TEMP="$fixture/$name" perl "$helper" "$name" "$@" > "$fixture/$name.console" 2>&1 || status=$?
    [[ "$status" == "$expected" ]] || { echo "wrong original status for $name: $status"; exit 1; }
}

cat > "$fixture/failure.sh" <<'SH'
set -eu
attempt="$TOKO_TOOLING_STORE/rust-fixture/.attempt"
mkdir -p "$attempt"
cat > "$attempt/install.log" <<'LOG'
error: hidden compiler failure E0425
Authorization: Bearer hidden-authorization-value
password=hidden-password-value
-----BEGIN PRIVATE KEY-----
hidden-key-material
-----END PRIVATE KEY-----
failed https://user:secretpass@example.com/source?credential=hidden-query-value
LOG
printf 'private payload' > "$attempt/source.crate"
printf 'error: early installer refusal\n'
printf 'known value: %s\n' "$SETUP_TEST_TOKEN"
printf 'Cargo tool attempt retained: %s\n' "$attempt"
exit 41
SH
run_case early 41 bash "$fixture/failure.sh"
evidence="$fixture/early/canic-setup-diagnostics/early"
grep -q 'hidden compiler failure E0425' "$evidence/attempt-1.txt"
grep -q 'hidden compiler failure E0425' "$fixture/early.console"
grep -q 'status=41' "$evidence/summary.txt"
[[ $(find "$evidence" -type f | wc -l) == 4 ]]
if grep -Er 'sentinel-private|hidden-authorization|hidden-password|hidden-key|hidden-query|user:secretpass|private payload' "$evidence" "$fixture/early.console"; then
    echo 'sensitive fixture escaped sanitation'; exit 1
fi

# shellcheck disable=SC2016 # The child receives the wrapper's selected store.
run_case missing 29 bash -c 'echo "Cargo tool attempt retained: $TOKO_TOOLING_STORE/missing"; exit 29'
grep -q 'collection failed: selected attempt log missing' "$fixture/missing.console"
grep -q 'status=29' "$fixture/missing/canic-setup-diagnostics/missing/summary.txt"
run_case empty 19 bash -c 'exit 19'
grep -q 'no setup output available' "$fixture/empty/canic-setup-diagnostics/empty/command.txt"
grep -q 'No explicit attempt logs' "$fixture/empty/canic-setup-diagnostics/empty/collection.txt"

mkdir -p "$fixture/collector"
printf 'occupied' > "$fixture/collector/canic-setup-diagnostics"
run_case collector 43 bash -c 'echo "error: original failure"; exit 43'
grep -q 'original failure' "$fixture/collector.console"
grep -q 'collection failed' "$fixture/collector.console"
[[ $(cat "$fixture/collector/canic-setup-diagnostics") == occupied ]]

mkdir -p "$fixture/external"
printf 'foreign-secret' > "$fixture/external/install.log"
# shellcheck disable=SC2016 # Expansion belongs to the child command.
run_case symlink 47 bash -c 'mkdir -p "$TOKO_TOOLING_STORE"; ln -s "$1" "$TOKO_TOOLING_STORE/attempt"; echo "Cargo tool attempt retained: $TOKO_TOOLING_STORE/attempt"; exit 47' bash "$fixture/external"
grep -q 'attempt contains a symlink' "$fixture/symlink.console"
if grep -R 'foreign-secret' "$fixture/symlink/canic-setup-diagnostics"; then exit 1; fi

# shellcheck disable=SC2016 # $_ belongs to Perl, not the invoking shell.
run_case bounded 31 perl -e 'print "oversized-secret" x 10000, "\n"; print "error: bounded line $_\n" for 1..10000; exit 31'
grep -q 'command_tail_truncated=1' "$fixture/bounded/canic-setup-diagnostics/bounded/summary.txt"
[[ $(wc -c < "$fixture/bounded/canic-setup-diagnostics/bounded/command.txt") -le 65536 ]]
[[ $(wc -c < "$fixture/bounded.console") -lt 10000 ]]
if grep -R 'oversized-secret' "$fixture/bounded/canic-setup-diagnostics"; then exit 1; fi
run_case success 0 bash -c 'echo "setup complete"'
[[ ! -e "$fixture/success/canic-setup-diagnostics" ]]
run_case signal 143 bash -c 'kill -TERM $$'
grep -q 'status=143' "$fixture/signal/canic-setup-diagnostics/signal/summary.txt"
run_case pipeline 1 bash --noprofile --norc -e -o pipefail -c 'false | true; echo should-not-run'
if grep -q 'should-not-run' "$fixture/pipeline.console"; then exit 1; fi
# Exercise the workflow's actual reporting shell on upload failure and success.
perl -0777 -ne '
    /- name: Report setup evidence availability.*?        run: \|\n((?:          [^\n]*\n)+)/s or die "report step missing\n";
    my $script=$1; $script =~ s/^          //mg; print $script;
' .github/workflows/ci.yml > "$fixture/report.sh"
status=0
ARTIFACT_URL='' GITHUB_STEP_SUMMARY="$fixture/summary" bash -e "$fixture/report.sh" > "$fixture/upload.console" 2>&1 || status=$?
[[ "$status" == 1 ]]
grep -q 'upload failed or no setup evidence' "$fixture/upload.console"
ARTIFACT_URL='https://example.invalid/artifact/123' GITHUB_STEP_SUMMARY="$fixture/summary" bash -e "$fixture/report.sh"
grep -q 'https://example.invalid/artifact/123' "$fixture/summary"
echo "Setup diagnostics: failure, redaction, hidden logs, bounds, missing evidence, symlink refusal and original statuses passed."
