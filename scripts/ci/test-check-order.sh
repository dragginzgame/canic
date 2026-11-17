#!/usr/bin/env bash
# Exercise the real Make dependency graph with bounded recording recipes, not Cargo.
set -euo pipefail
cd "$(dirname "$0")/../.."
mkdir -p target/check-order
export CHECK_ORDER_FIXTURE
CHECK_ORDER_FIXTURE=$(mktemp -d "$PWD/target/check-order/run.XXXXXX")

# Keep the selected stages in the Makefile. Refuse unsupported declarations before
# invoking Make, so a changed roster cannot accidentally launch a real build here.
perl - "$CHECK_ORDER_FIXTURE" <<'PERL'
use strict;
use warnings;
my ($dir) = @ARGV;
open my $make, '<', 'Makefile' or die $!;
my %deps;
while (<$make>) {
    next unless /^(?:(lint):|(?:CI_TEST_TARGETS)\s*:=)\s*(.*?)\s*$/;
    my ($target, $list) = ($1 // 'tests', $2);
    die "duplicate aggregate declaration" if exists $deps{$target};
    my @names = split /\s+/, $list;
    die "unsupported aggregate prerequisite" if grep { !/^[a-z][a-z0-9-]*$/ } @names;
    $deps{$target} = \@names;
}
die "missing aggregate stages" unless $deps{tests} && $deps{lint};
my %stages = map { $_ => 1 } (@{$deps{tests}}, @{$deps{lint}}, 'lint', 'format-tools-check');
open my $out, '>', "$dir/recipes.mk" or die $!;
for my $stage (sort keys %stages) {
    print $out "$stage:\n\t\@bash \"\$(CHECK_ORDER_FIXTURE)/stage.sh\" \"\$\@\"\n";
}
open my $tests, '>', "$dir/tests" or die $!;
print $tests "$_\n" for @{$deps{tests}};
PERL

cat > "$CHECK_ORDER_FIXTURE/stage.sh" <<'SH'
set -eu
mkdir "$CHECK_ORDER_FIXTURE/active" || exit 41
trap 'rmdir "$CHECK_ORDER_FIXTURE/active"' EXIT
printf '%s\n' "$1" >> "$CHECK_ORDER_FIXTURE/events"
sleep 0.05
if [[ "$1" == "${CHECK_ORDER_FAIL:-}" ]]; then exit 42; fi
SH

run_make() {
    make --no-print-directory -j8 -f Makefile -f "$CHECK_ORDER_FIXTURE/recipes.mk" \
        "MAKE=make --no-print-directory -f Makefile -f \"$CHECK_ORDER_FIXTURE/recipes.mk\"" "$@" ci
}
for mode in fail-fast keep-going; do
    options=()
    if [[ "$mode" == keep-going ]]; then options=(-k); fi
    status=0
    CHECK_ORDER_FAIL=effect-clippy run_make "${options[@]}" \
        > "$CHECK_ORDER_FIXTURE/$mode.log" 2>&1 || status=$?
    [[ "$status" != 0 ]]
    perl - "$CHECK_ORDER_FIXTURE" <<'PERL'
use strict;
use warnings;
my ($dir) = @ARGV;
open my $events, '<', "$dir/events" or die $!;
my %seen = map { chomp; $_ => 1 } <$events>;
die "lint failure was not reached" unless $seen{'effect-clippy'};
open my $tests, '<', "$dir/tests" or die $!;
while (<$tests>) { chomp; die "test stage $_ ran after failed lint" if $seen{$_}; }
PERL
    mv "$CHECK_ORDER_FIXTURE/events" "$CHECK_ORDER_FIXTURE/$mode.events"
done
run_make > "$CHECK_ORDER_FIXTURE/success.log" 2>&1
perl - "$CHECK_ORDER_FIXTURE" <<'PERL'
use strict;
use warnings;
my ($dir) = @ARGV;
open my $events, '<', "$dir/events" or die $!;
my %seen;
my $index = 0;
while (<$events>) {
    chomp;
    die "repeated stage $_" if exists $seen{$_};
    $seen{$_} = $index++;
}
die "lint completion missing" unless exists $seen{lint};
open my $tests, '<', "$dir/tests" or die $!;
while (<$tests>) {
    chomp;
    die "missing test stage $_" unless exists $seen{$_};
    die "test $_ preceded lint completion" unless $seen{$_} > $seen{lint};
}
PERL
printf 'Parallel Make: lint failure stops tests; successful stages run once without overlap.\n'
