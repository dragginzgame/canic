#!/usr/bin/env perl
use strict;
use warnings;
use File::Temp qw(tempdir);
use File::Path qw(make_path remove_tree);
use FindBin;
use JSON::PP;
use Test::More;

# All Git commands are substituted. No real commits, tag changes or pushes.
my $fixture = tempdir('tag-maintenance-test.XXXXXX', TMPDIR => 1, CLEANUP => 0);
my $passed;
END {
    if ($passed) { remove_tree($fixture); }
    else { warn "Tag maintenance fixtures retained: $fixture\n"; }
}
my $helper = "$FindBin::Bin/../dev/delete-github-tags-up-to.pl";
my $json = JSON::PP->new->canonical;
sub write_file {
    my ($path, $text) = @_;
    open my $file, '>', $path or die $!;
    print {$file} $text or die $!;
    close $file or die $!;
}
sub read_file {
    open my $file, '<', $_[0] or die $!;
    return do { local $/; <$file> } // '';
}
mkdir "$fixture/bin" or die $!;
my $stub = <<'STUB';
use strict;
use warnings;
use JSON::PP;
my $json = JSON::PP->new->canonical;
my $root = $ENV{TAG_FIXTURE_ROOT};
@ARGV >= 3 && shift(@ARGV) eq '-C' && shift(@ARGV) eq $root or die 'wrong repository';
open my $events, '>>', "$root/events" or die $!;
print {$events} $json->encode(\@ARGV), "\n" or die $!;
close $events or die $!;
open my $file, '<', "$root/git.json" or die $!;
my $state = $json->decode(do { local $/; <$file> });
close $file;
sub save {
    open my $file, '>', "$root/git.json" or die $!;
    print {$file} $json->encode($state) or die $!;
    close $file or die $!;
}
my $mode = $ENV{TAG_FIXTURE_MODE} // '';
my $command = shift @ARGV;
if ($command eq 'rev-parse') {
    if ($ARGV[0] eq '--show-toplevel') { print "$root\n"; }
    elsif ($ARGV[0] eq '--git-common-dir') { print "$root/.git\n"; }
    else { die 'unexpected rev-parse'; }
} elsif ($command eq 'remote') {
    ++$state->{destination_reads}; save();
    print $mode eq 'destination-race' && $state->{destination_reads} > 1
        ? "https://example.invalid/changed.git\n" : "https://example.invalid/selected.git\n";
    print "https://example.invalid/extra.git\n" if $mode eq 'multiple-destinations';
    exit 8 if $mode eq 'destination-failure';
} elsif ($command eq 'for-each-ref' || $command eq 'ls-remote') {
    my $scope = $command eq 'ls-remote' ? 'remote' : 'local';
    if ($scope eq 'remote') {
        $ARGV[-1] eq 'https://example.invalid/selected.git' or die 'wrong destination';
        ++$state->{observations};
        if ($mode eq 'remote-race' && $state->{observations} == 2) {
            $state->{remote}{'v0.1.0'} = 'b' x 40;
        }
        save();
    }
    for my $tag (sort keys %{$state->{$scope}}) {
        print "$state->{$scope}{$tag}\trefs/tags/$tag", ($scope eq 'local' ? "\t" : ''), "\n";
    }
    exit 7 if $mode eq "$scope-inventory-failure";
    print "broken inventory\n" if $mode eq 'malformed-inventory';
} elsif ($command eq 'push') {
    ++$state->{pushes};
    save();
    grep($_ eq '--atomic', @ARGV) or die 'missing atomic';
    grep($_ eq '--no-follow-tags', @ARGV) or die 'missing no-follow-tags';
    my %leases = map { /\A--force-with-lease=refs\/tags\/(.+):([a-f0-9]+)\z/ ? ($1 => $2) : () } @ARGV;
    my @tags = map { /\A:refs\/tags\/(.+)\z/ ? $1 : () } @ARGV;
    @tags > 0 && @tags <= 50 && @tags == keys(%leases) or die 'wrong batch';
    grep($_ eq 'https://example.invalid/selected.git', @ARGV) or die 'wrong push destination';
    if ($mode eq 'lease-race') { $state->{remote}{$tags[0]} = 'c' x 40; save(); }
    for (@tags) { ($state->{remote}{$_} // '') eq $leases{$_} or exit 11; }
    if ($mode eq 'reject' || ($mode eq 'second-batch-failure' && $state->{pushes} == 2)) {
        print STDERR "remote rejection evidence\n"; exit 12;
    }
    if ($mode ne 'false-success') { delete @{$state->{remote}}{@tags}; }
    save();
    exit 13 if $mode eq 'lost-reply';
} elsif ($command eq 'update-ref') {
    @ARGV == 1 && $ARGV[0] eq '--stdin' or die 'wrong local operation';
    my $input = do { local $/; <STDIN> };
    $input =~ /\Astart\n/ && $input =~ /prepare\ncommit\n\z/ or die 'missing transaction';
    my %expected = $input =~ /option no-deref\ndelete refs\/tags\/(\S+) ([a-f0-9]+)\n/g;
    keys(%expected) or die 'empty local deletion';
    if ($mode eq 'local-race') { $state->{local}{(sort keys %expected)[0]} = 'd' x 40; save(); }
    for (keys %expected) { ($state->{local}{$_} // '') eq $expected{$_} or exit 14; }
    delete @{$state->{local}}{keys %expected};
    save();
} else { die "unexpected Git operation: $command"; }
STUB
write_file("$fixture/bin/git", "#!$^X\n$stub");
chmod 0755, "$fixture/bin/git" or die $!;
local $ENV{PATH} = "$fixture/bin:$ENV{PATH}";
my $repo;
sub setup {
    my ($name, @tags) = @_;
    $repo = "$fixture/$name";
    make_path("$repo/.git");
    my %tags = map { $_ => 'a' x 40 } @tags;
    write_file("$repo/git.json", $json->encode({ local => {%tags}, remote => {%tags} }));
    $ENV{TAG_FIXTURE_ROOT} = $repo;
}
sub state { $json->decode(read_file("$repo/git.json")) }
sub invoke {
    my ($mode, @args) = @_;
    my $pid = fork(); defined $pid or die $!;
    if (!$pid) {
        $ENV{TAG_FIXTURE_MODE} = $mode;
        open STDOUT, '>', "$repo/output" or die $!;
        open STDERR, '>', "$repo/error" or die $!;
        exec $^X, $helper, '--repo', $repo, @args;
        die "exec: $!";
    }
    waitpid($pid, 0);
    return $?;
}
my @delete = ('--cutoff', '0.1', '--remote', 'origin', '--delete-local', '--delete-remote', '--yes');
setup('selection', qw(v0.0.9 v0.1.0 v0.1.9 v0.2.0 v0.10.0 v0.1.0-rc.1 v00.1.0 other));
is(invoke('', '--cutoff', '0.1.0'), 0, 'exact dry run succeeds');
like(read_file("$repo/output"), qr/local tags selected: 2\n/, 'exact cutoff includes lower stable tags');
ok(!-e "$repo/.git/tag-maintenance", 'dry run creates no maintenance state');
is(invoke('', '--cutoff', 'v0.1', '--remote', 'origin'), 0, 'minor remote inventory succeeds');
like(read_file("$repo/output"), qr/remote origin tags selected: 3\n/, 'minor cutoff includes all patches');
unlike(read_file("$repo/events"), qr/"push"|"update-ref"/, 'dry runs have no effects');
for my $cutoff ('', '1', '01.2', '1.02.0', '1.2.3-rc', '1.2.3+meta') {
    isnt(invoke('', '--cutoff', $cutoff), 0, 'invalid or ambiguous cutoff rejected');
}
isnt(invoke('', '--cutoff', '0.1', '--delete-local'), 0, 'confirmation required');
isnt(invoke('', '--cutoff', '0.1', '--delete-remote', '--yes'), 0, 'explicit remote required');
for my $mode (qw(local-inventory-failure remote-inventory-failure malformed-inventory multiple-destinations destination-failure)) {
    setup($mode, 'v0.1.0');
    isnt(invoke($mode, @delete), 0, "$mode rejects before effects");
    unlike(read_file("$repo/events"), qr/"push"|"update-ref"/, 'no mutation after failed observation');
}
setup('empty');
is(invoke('', @delete), 0, 'empty selection succeeds');
unlike(read_file("$repo/events"), qr/"push"|"update-ref"/, 'empty selection dispatches no deletion');
setup('success', qw(v0.0.9 v0.1.0 v0.2.0));
is(invoke('', @delete), 0, 'both scopes delete selected tags');
is_deeply([sort keys %{state()->{local}}], ['v0.2.0'], 'higher local tag preserved');
is_deeply([sort keys %{state()->{remote}}], ['v0.2.0'], 'higher remote tag preserved');
ok(!-e "$repo/.git/tag-maintenance/pending.json", 'completed intent retired');
my @receipts = glob "$repo/.git/tag-maintenance/attempt.*/intent.json";
is(scalar @receipts, 1, 'completed identity retained');
for my $mode (qw(reject false-success remote-race lease-race destination-race)) {
    setup($mode, 'v0.1.0');
    isnt(invoke($mode, @delete), 0, "$mode stops deletion");
    ok(exists state()->{local}{'v0.1.0'}, 'local recovery copy retained');
    ok(-f "$repo/.git/tag-maintenance/pending.json", 'unfinished intent retained');
    ok(!-e "$repo/.git/tag-maintenance/lock", 'owned lock released after failure');
    if ($mode eq 'reject') {
        my @logs = glob "$repo/.git/tag-maintenance/attempt.*/run.*";
        like(read_file($logs[0]), qr/remote rejection evidence/, 'raw Git error retained');
    }
}
setup('local-race', 'v0.1.0');
isnt(invoke('local-race', '--cutoff', '0.1', '--delete-local', '--yes'), 0, 'changed local ref rejected atomically');
is(state()->{local}{'v0.1.0'}, 'd' x 40, 'concurrent local identity preserved');
setup('lost-reply', 'v0.1.0');
isnt(invoke('lost-reply', @delete), 0, 'lost reply stops without replay');
ok(exists state()->{local}{'v0.1.0'}, 'lost reply preserves local copy');
my $retry_state = state();
$retry_state->{remote}{'v0.1.1'} = 'b' x 40;
$retry_state->{local}{'v0.1.1'} = 'b' x 40;
write_file("$repo/git.json", $json->encode($retry_state));
is(invoke('', @delete), 0, 'retry reconciles missing remote refs');
is(state()->{pushes}, 1, 'uncertain push is not repeated');
ok(exists state()->{local}{'v0.1.1'} && exists state()->{remote}{'v0.1.1'}, 'retry never expands saved selection');
setup('changed-retry', 'v0.1.0');
isnt(invoke('reject', @delete), 0, 'save rejected operation');
isnt(invoke('', '--cutoff', '0.2', '--remote', 'origin', '--delete-local', '--delete-remote', '--yes'), 0, 'retry cutoff change rejected');
$retry_state = state();
$retry_state->{remote}{'v0.1.0'} = 'b' x 40;
write_file("$repo/git.json", $json->encode($retry_state));
isnt(invoke('', @delete), 0, 'retry refuses changed remote identity');
is(state()->{pushes}, 1, 'conflicting retry has no second push');
setup('batches', map { "v0.1.$_" } 0 .. 100);
isnt(invoke('second-batch-failure', @delete), 0, 'later batch failure stops');
is(scalar(keys %{state()->{local}}), 101, 'all local copies retained after partial remote deletion');
is(scalar(keys %{state()->{remote}}), 51, 'only completed batch disappeared');
is(invoke('', @delete), 0, 'retry reconciles partial batch completion');
is(state()->{pushes}, 4, 'retry pushes only two outstanding batches');
# File::Temp may generate underscores. Bind retries to that same safe basename,
# without admitting path components outside the retained attempt directory.
setup('underscore-attempt', 'v0.1.0');
isnt(invoke('reject', @delete), 0, 'save interrupted operation for underscore retry');
my $pending_path = "$repo/.git/tag-maintenance/pending.json";
my $saved = $json->decode(read_file($pending_path));
rename "$repo/.git/tag-maintenance/$saved->{attempt}", "$repo/.git/tag-maintenance/attempt.a_bC12" or die $!;
$saved->{attempt} = 'attempt.a_bC12';
write_file($pending_path, $json->encode($saved));
is(invoke('', @delete), 0, 'retry accepts retained File::Temp underscore name');
is(state()->{pushes}, 2, 'retry dispatches only the outstanding deletion');

my $unsafe_case = 0;
for my $name ('../outside', 'attempt.safe/child', 'attempt.../outside') {
    setup('unsafe-attempt-' . ++$unsafe_case, 'v0.1.0');
    isnt(invoke('reject', @delete), 0, 'save interrupted operation before unsafe-name check');
    $pending_path = "$repo/.git/tag-maintenance/pending.json";
    $saved = $json->decode(read_file($pending_path));
    $saved->{attempt} = $name;
    write_file($pending_path, $json->encode($saved));
    isnt(invoke('', @delete), 0, 'saved evidence path traversal rejected');
    is(state()->{pushes}, 1, 'unsafe evidence path never causes another push');
}

setup('remote-only', 'v0.1.0');
is(invoke('', '--cutoff', '0.1', '--remote', 'origin', '--delete-remote', '--yes'), 0, 'remote-only deletion succeeds');
ok(exists state()->{local}{'v0.1.0'}, 'remote-only leaves local tags intact');
setup('invalid-plan', 'v0.1.0');
isnt(invoke('reject', @delete), 0, 'unfinished plan created');
write_file("$repo/.git/tag-maintenance/pending.json", '{broken');
isnt(invoke('', @delete), 0, 'corrupt saved intent rejected');
is(state()->{pushes}, 1, 'corrupt plan causes no push');
setup('lock', 'v0.1.0');
make_path("$repo/.git/tag-maintenance/lock");
write_file("$repo/.git/tag-maintenance/lock/owner", 'foreign owner');
isnt(invoke('', @delete), 0, 'concurrent maintenance rejected');
is(read_file("$repo/.git/tag-maintenance/lock/owner"), 'foreign owner', 'foreign lock preserved');
$passed = Test::More->builder->is_passing;
done_testing;
