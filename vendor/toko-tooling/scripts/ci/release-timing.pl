#!/usr/bin/env perl
# Shared companions: scripts/ci/ReleaseTiming.pm
use strict;
use warnings;
use FindBin qw($RealBin);
use lib $RealBin;
use ReleaseTiming;
use JSON::PP;
my $operation=shift(@ARGV)//'';
if ($operation eq 'begin' && (@ARGV==2 || @ARGV==3)) {
    my $span=ReleaseTiming::begin(@ARGV);
    print encode_json($span) if $span;
} elsif ($operation eq 'end' && @ARGV==2) {
    my ($token,$status)=@ARGV;
    $status =~ /\A[0-9]+\z/ or die "invalid timing status\n";
    ReleaseTiming::finish(decode_json($token),$status) if length $token;
} elsif ($operation eq 'report' && @ARGV==1) {
    print JSON::PP->new->canonical->pretty->encode(ReleaseTiming::report($ARGV[0]));
} elsif ($operation eq 'run' && @ARGV>=3) {
    my ($phase,$mode)=splice(@ARGV,0,2);
    my $span=ReleaseTiming::begin($phase,$mode);
    $ENV{RELEASE_TIMING_PARENT}=$span->{id} if $span;
    system @ARGV;
    my $status=$? == -1 ? 127 : ($? & 127) ? 128+($? & 127) : $? >> 8;
    eval {ReleaseTiming::finish($span,$status)};
    if ($@) {warn $@; $status ||= 1;}
    exit $status;
} else {die "usage: release-timing.pl begin PHASE MODE [ORIGIN] | end TOKEN STATUS | run PHASE MODE COMMAND... | report JOURNAL\n";}
