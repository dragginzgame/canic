#!/usr/bin/env perl
use strict;
use warnings;
use Cwd qw(abs_path);
use File::Basename qw(dirname);
use File::Compare qw(compare);
use File::Copy qw(copy);
use Fcntl qw(:flock);

# Keep private consumer builds disposable without changing their source layout,
# independent lockfiles or the production Host's deployment-state locations.
my $root = abs_path(dirname(__FILE__) . '/../..');
my $control = "$root/target/integrations";
my $workspace = "$control/workspace";
my $mode = shift @ARGV // '';
die "usage: $0 --path | --prepare | --run PROGRAM [ARG ...]\n"
    unless ($mode eq '--path' || $mode eq '--prepare') && !@ARGV
        || $mode eq '--run' && @ARGV;
if ($mode eq '--path') {
    print "$workspace\n";
    exit 0;
}
open my $repository, '-|', 'git', '-C', $root, 'rev-parse', '--show-toplevel'
    or die "resolve source repository: $!\n";
chomp(my $repository_root = <$repository> // '');
close $repository or die "resolve source repository failed\n";
die "run the integration helper from the source repository, not its build copy\n"
    unless $repository_root eq $root;

sub checked_path {
    my ($base, $relative, $parents) = @_;
    my @parts = split m{/}, $relative, -1;
    die "invalid source path: $relative\n"
        if grep { $_ eq '' || $_ eq '.' || $_ eq '..' } @parts;
    my $path = $base;
    for my $index (0 .. $#parts) {
        $path .= "/$parts[$index]";
        die "symlink in integration workspace path: $path\n" if -l $path;
        if ($index < $#parts && $parents && !-e $path) {
            mkdir $path or die "mkdir $path: $!\n";
        }
    }
    return $path;
}

checked_path($root, 'target/integrations/prepare.lock', 1);
open my $lock, '>>', "$control/prepare.lock" or die "open preparation lock: $!\n";
flock $lock, LOCK_EX or die "lock integration workspace: $!\n";
checked_path($control, 'workspace/source', 0);
my $roster = "$control/source-files";
checked_path($control, 'source-files', 0);
if (!-e $roster) {
    if (-d $workspace) {
        opendir my $dir, $workspace or die "open $workspace: $!\n";
        die "refusing unowned integration workspace: $workspace\n"
            if grep { $_ ne '.' && $_ ne '..' } readdir $dir;
        closedir $dir;
    } else {
        mkdir $workspace or die "mkdir $workspace: $!\n";
    }
    open my $initial, '>', $roster or die "initialize source roster: $!\n";
    close $initial or die "close source roster: $!\n";
}

open my $git, '-|', 'git', '-C', $root, 'ls-files', '--cached', '--others',
    '--exclude-standard', '-z' or die "list working-tree source: $!\n";
my @files;
{
    local $/ = "\0";
    @files = <$git>;
    chomp @files;
}
close $git or die "git source inventory failed\n";
my %current;
for my $relative (@files) {
    my $source = checked_path($root, $relative, 0);
    next unless -e $source; # A tracked deletion is not an input.
    die "source is not a regular file: $source\n" unless -f $source;
    $current{$relative} = 1;
}
open my $previous, '<', $roster or die "read source roster: $!\n";
{
    local $/ = "\0";
    while (my $relative = <$previous>) {
        chomp $relative;
        next if $current{$relative};
        my $destination = checked_path($workspace, $relative, 0);
        unlink $destination or die "remove stale source $destination: $!\n" if -e $destination;
    }
}
close $previous;
for my $relative (sort keys %current) {
    my $source = "$root/$relative";
    my $destination = checked_path($workspace, $relative, 1);
    if (!-f $destination || compare($source, $destination) != 0) {
        copy($source, $destination) or die "copy $relative: $!\n";
    }
    my $mode = (stat $source)[2] & 0777;
    if (((stat $destination)[2] & 0777) != $mode) {
        chmod($mode, $destination) or die "chmod $relative: $!\n";
    }
}
checked_path($control, 'source-files.next', 0);
open my $next, '>', "$roster.next" or die "write source roster: $!\n";
print {$next} map { "$_\0" } sort keys %current;
close $next or die "close source roster: $!\n";
rename "$roster.next", $roster or die "publish source roster: $!\n";
if ($mode eq '--prepare') {
    print "$workspace\n";
    exit 0;
}

# Keep the snapshot stable until the selected build finishes. Do not exec: the
# parent must retain the preparation lock while its child is running.
$ENV{CANIC_INTEGRATION_WORKSPACE} = $workspace;
my $status = system { $ARGV[0] } @ARGV;
die "run $ARGV[0]: $!\n" if $status == -1;
exit(($status & 127) ? 128 + ($status & 127) : $status >> 8);
