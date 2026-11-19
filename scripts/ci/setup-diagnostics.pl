#!/usr/bin/env perl
# Canic CI setup only. No raw output, caches, environment or payloads are uploaded.
use strict;
use warnings;
use Cwd qw(abs_path);
use FindBin qw($Bin);
use lib "$Bin/../../vendor/toko-tooling/scripts/ci";
use SetupDiagnostics qw(excerpt);
use Fcntl qw(O_RDONLY O_NOFOLLOW O_NONBLOCK S_ISREG);

my ($phase, @command) = @ARGV;
die "usage: setup-diagnostics.pl PHASE COMMAND...\n"
    unless defined($phase) && $phase =~ /\A[a-z0-9-]{1,48}\z/ && @command;
my $temp = abs_path($ENV{RUNNER_TEMP} // die "RUNNER_TEMP required\n") // die "RUNNER_TEMP must exist\n";
my $root = "$temp/canic-setup-tools";
# This ephemeral job owns its tool store; the existing download cache is unchanged.
$ENV{TOKO_TOOLING_STORE} = $root;
# Observe explicit installer reports before the shared excerpt drops old lines.
# Only paths are retained here. SetupDiagnostics owns all displayed output.
{
    package AttemptReports;
    sub TIEHANDLE {
        my ($class, $input) = @_;
        return bless { input => $input, pending => '', paths => [] }, $class;
    }
    sub READ {
        my ($self, undef, $length) = @_;
        my $n = sysread($self->{input}, $_[1], $length);
        return $n unless $n;
        for my $part (split /(?<=\n)/, $_[1]) {
            $self->{pending} .= $part if length($self->{pending}) <= 8192;
            next unless $part =~ /\n\z/;
            if ($self->{pending} =~ /\ACargo tool attempt retained: ([^\r\n]+)\n\z/
                && length($self->{pending}) <= 8192) {
                if (@{$self->{paths}} < 4) { push @{$self->{paths}}, "$1/install.log"; }
                else { $self->{omitted} = 1; }
            }
            $self->{pending} = '';
        }
        return $n;
    }
}
$| = 1;
print "Starting setup phase=$phase\n";
my $pid = open(my $reader, '-|');
die "cannot start setup command\n" unless defined $pid;
if (!$pid) {
    open STDERR, '>&', STDOUT or exit 125;
    exec { $command[0] } @command or exit 127;
}
my $reports = tie *REPORTS, 'AttemptReports', $reader;
my $tail = eval { excerpt(\*REPORTS) };
my $capture_ok = defined $tail;
$tail //= "[setup output collection failed]\n";
close $reader;
my $wait = $?;
my $status = $wait == -1 ? 1 : ($wait & 127) ? 128 + ($wait & 127) : $wait >> 8;
print "Setup phase=$phase status=$status\n";
print "| $_\n" for split /\n/, substr($tail, -8192);
print STDERR "Diagnostic capture failed; output may be incomplete\n" unless $capture_ok;
exit($capture_ok ? 0 : 1) unless $status;

my $truncated = $tail =~ /\A\[earlier output or oversized lines omitted/ ? 1 : 0;
my $tool = $command[0] =~ m{(?:^|/)(bash|perl|make)\z} ? $1 : 'setup command';
my $created;
my $output = "$temp/canic-setup-diagnostics/$phase";
my $ok = eval {
    die "artifact root is a symlink\n" if -l "$temp/canic-setup-diagnostics";
    mkdir("$temp/canic-setup-diagnostics", 0700) or die "cannot create artifact root\n"
        unless -d "$temp/canic-setup-diagnostics";
    mkdir($output, 0700) or die "cannot create fresh phase evidence\n";
    $created = 1;
    sub write_file {
        my ($name, $body) = @_;
        open my $file, '>', "$output/$name" or die "cannot write evidence\n";
        print {$file} $body or die "cannot write evidence\n";
        close $file or die "cannot close evidence\n";
    }
    write_file('summary.txt', "phase=$phase\nstatus=$status\ntool=" . $tool
        . "\ncapture_complete=" . ($capture_ok ? 1 : 0)
        . "\ncommand_tail_truncated=$truncated\nOnly sanitized excerpts retained; arguments/environment/caches/payloads omitted.\n");
    write_file('command.txt', $tail);
    my $index = 0;
    for my $path (@{$reports->{paths}}) {
        # Refuse traversal, every descendant symlink and nonregular files.
        my $relative = substr($path, length($root) + 1);
        die "attempt outside selected tool store\n" unless index($path, "$root/") == 0
            && $relative =~ /\A[A-Za-z0-9_.\/-]+\z/ && $relative !~ m{(?:^|/)\.{1,2}(?:/|$)};
        my $part = $root;
        die "tool store is a symlink\n" if -l $part;
        for my $component (split '/', $relative) {
            $part .= "/$component";
            die "attempt contains a symlink\n" if -l $part;
        }
        my @before = lstat($path);
        die "selected attempt log missing/not regular\n" unless @before && S_ISREG($before[2]);
        my $physical = abs_path($path) // die "cannot resolve attempt log\n";
        die "attempt log escaped tool store\n" unless index($physical, "$root/") == 0;
        sysopen(my $in, $path, O_RDONLY | O_NOFOLLOW | O_NONBLOCK) or die "selected attempt log unreadable\n";
        my @stat = stat($in);
        die "attempt log changed while opening\n" unless S_ISREG($stat[2])
            && $stat[0] == $before[0] && $stat[1] == $before[1];
        my $detail = excerpt($in, 16384);
        close $in or die "cannot close selected attempt log\n";
        write_file('attempt-' . ++$index . '.txt', $detail);
        print "Selected attempt log $index (sanitized excerpt):\n";
        print "| $_\n" for split /\n/, substr($detail, 0, 1024);
    }
    write_file('collection.txt', "selected_attempt_logs=$index\nMaximum 4 explicit install.log files, 16384 source bytes each.\n"
        . ($reports->{omitted} ? "Additional reported attempts omitted at the selection limit.\n" : '')
        . ($index ? '' : "No explicit attempt logs were reported; setup output retained instead.\n"));
    1;
};
if (!$ok) {
    my $error = $@;
    print STDERR "Setup diagnostic collection failed: $error";
    eval { write_file('collection.txt', "Collection incomplete: $error"); 1 } if $created;
}
else { print "Sanitized setup evidence: canic-setup-diagnostics/$phase\n"; }
exit $status;
