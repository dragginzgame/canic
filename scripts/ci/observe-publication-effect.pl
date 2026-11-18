#!/usr/bin/env perl
use strict;
use warnings;
use JSON::PP;
my @programs = ($ENV{CANIC_TEST_EXECUTABLE}, "$ENV{IC_TESTKIT_SERVER_DIRECTORY}/pocket-ic");
my @files = ('Cargo.lock', 'qualification/Cargo.lock', 'scripts/dev/test-effects.sh',
    'scripts/ci/publication-effect-stage.sh', @ENV{qw(CANIC_EFFECT_PROBE_WASM CANIC_EFFECT_CANDIDATE_WASM)});
for my $program (@programs) {
    -x $program && -f $program or die "missing executable $program\n";
    push @files, $program;
    open my $libs, '-|', '/usr/bin/ldd', $program or die "ldd: $!";
    while (<$libs>) {
        die "unresolved library: $_" if /not found/;
        push @files, $1 if m{(/[^\n]+?)\s+\(0x[0-9a-f]+\)};
    }
    close $libs or die "ldd failed for $program\n";
}
my %unique = map { $_ => 1 } @files;
print JSON::PP->new->canonical->encode({schema=>1, files=>[sort keys %unique], configuration=>{}}), "\n";
