#!/usr/bin/env perl
# Canic CI setup only. No raw output, caches, environment or payloads are uploaded.
use strict;
use warnings;
use Cwd qw(abs_path);
use File::Basename qw(basename);
use Fcntl qw(O_RDONLY O_NOFOLLOW O_NONBLOCK S_ISREG);

my ($phase, @command) = @ARGV;
die "usage: setup-diagnostics.pl PHASE COMMAND...\n"
    unless defined($phase) && $phase =~ /\A[a-z0-9-]{1,48}\z/ && @command;
my $temp = abs_path($ENV{RUNNER_TEMP} // '') // die "RUNNER_TEMP must exist\n";
my $root = "$temp/canic-setup-tools";
# This ephemeral job owns its tool store; the existing download cache is unchanged.
$ENV{TOKO_TOOLING_STORE} = $root;
my @secrets = sort { length($b) <=> length($a) }
    grep { defined($_) && length($_) >= 4 }
    map { $ENV{$_} } grep { /TOKEN|SECRET|PASSWORD|CREDENTIAL|PRIVATE_KEY|ACCESS_KEY/i } keys %ENV;
my ($tail, $truncated, $key) = ('', 0, 0);
my (@attempts, $attempts_omitted);
$| = 1;
sub sanitize {
    my ($line) = @_;
    $line =~ s/\e\[[0-?]*[ -\/]*[\@-~]//g;
    $key = 1 if $line =~ /-----BEGIN .*PRIVATE KEY-----/;
    if ($key) {
        $key = 0 if $line =~ /-----END .*PRIVATE KEY-----/;
        return "[private key omitted]\n";
    }
    for my $secret (@secrets) { $line =~ s/\Q$secret\E/[redacted]/g; }
    return "[sensitive field or environment assignment omitted]\n"
        if $line =~ /(?:authorization|cookie|password|passwd|secret|credential|token|private[_ -]?key|access[_ -]?key)\s*["']?\s*[:=]/i
        || $line =~ /^\s*[A-Za-z_][A-Za-z0-9_]*=/;
    $line =~ s/\b(?:gh[pousr]_[A-Za-z0-9_]+|github_pat_[A-Za-z0-9_]+|AKIA[A-Z0-9]{16})\b/[redacted]/g;
    $line =~ s/\bBearer\s+\S+/Bearer [redacted]/ig;
    $line =~ s{(https?://)[^\s/\@]+\@}{${1}[redacted]\@}g;
    $line =~ s{(https?://[^\s?#]+)[?#]\S+}{${1}[URL parameters omitted]}g;
    $line =~ s/[\x00-\x08\x0b-\x1f\x7f]/[control]/g;
    return $line;
}
sub retain {
    my ($line) = @_;
    $tail .= sanitize($line);
    if (length($tail) > 65536) {
        $tail = substr($tail, -65536);
        $tail =~ s/^[^\n]*\n//;
        $truncated = 1;
    }
}
sub consume {
    my ($handle, $attempts) = @_;
    my ($pending, $dropping) = ('', 0);
    while (1) {
        my $n = sysread($handle, my $chunk, 4096);
        die "diagnostic stream read failed\n" unless defined $n;
        last unless $n;
        for my $piece (split /(?<=\n)/, $chunk) {
            $pending .= $piece unless $dropping;
            if (length($pending) > 8192) { $pending = ''; $dropping = 1; }
            next unless $piece =~ /\n\z/;
            if ($dropping) { retain("[oversized diagnostic line omitted]\n"); }
            else {
                # Only the installer's explicit failed attempt is selected, never a store scan.
                if ($attempts && $pending =~ /^Cargo tool attempt retained: (.+)\n$/) {
                    if (@attempts < 4) { push @attempts, "$1/install.log"; }
                    else { $attempts_omitted = 1; }
                }
                retain($pending);
            }
            ($pending, $dropping) = ('', 0);
        }
    }
    retain($dropping ? "[oversized diagnostic line omitted]\n" : $pending) if $dropping || length($pending);
}
print "Starting setup phase=$phase\n";
pipe(my $reader, my $writer) or die "cannot create diagnostic pipe\n";
my $pid = fork();
die "cannot start setup command\n" unless defined $pid;
if (!$pid) {
    close $reader;
    open STDOUT, '>&', $writer or exit 125;
    open STDERR, '>&', $writer or exit 125;
    close $writer;
    exec { $command[0] } @command or do { print STDERR "cannot execute setup tool: $!\n"; exit 127; };
}
close $writer;
my $capture_ok = eval { consume($reader, 1); 1 };
close $reader;
waitpid($pid, 0);
my $wait = $?;
my $status = ($wait & 127) ? 128 + ($wait & 127) : $wait >> 8;
print "Setup phase=$phase status=$status\n";
print "| $_\n" for split /\n/, substr($tail, -8192);
print STDERR "Diagnostic capture failed; output may be incomplete\n" unless $capture_ok;
exit($capture_ok ? 0 : 1) unless $status;

my $output = "$temp/canic-setup-diagnostics/$phase";
my $ok = eval {
    die "artifact root is a symlink\n" if -l "$temp/canic-setup-diagnostics";
    mkdir("$temp/canic-setup-diagnostics", 0700) or die "cannot create artifact root\n"
        unless -d "$temp/canic-setup-diagnostics";
    mkdir($output, 0700) or die "cannot create fresh phase evidence\n";
    sub write_file {
        my ($name, $body) = @_;
        open my $file, '>', "$output/$name" or die "cannot write evidence\n";
        print {$file} $body or die "cannot write evidence\n";
        close $file or die "cannot close evidence\n";
    }
    write_file('summary.txt', "phase=$phase\nstatus=$status\ntool=" . sanitize(basename($command[0]))
        . "\ncapture_complete=" . ($capture_ok ? 1 : 0)
        . "\ncommand_tail_truncated=$truncated\nOnly sanitized excerpts retained; arguments/environment/caches/payloads omitted.\n");
    write_file('command.txt', length($tail) ? $tail : "[no setup output available]\n");
    my $index = 0;
    for my $path (@attempts) {
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
        my $n = sysread($in, my $raw, 16384);
        die "attempt log read failed\n" unless defined $n;
        close $in;
        $raw =~ s/[^\n]*\z// if $stat[7] > $n;
        ($tail, $truncated, $key) = ('', 0, 0);
        for my $line (split /(?<=\n)/, $raw) {
            retain(length($line) > 8192 ? "[oversized diagnostic line omitted]\n" : $line);
        }
        if (length($tail) > 16384 || $stat[7] > $n) {
            $tail = substr($tail, 0, 16384);
            $tail .= "\n[attempt excerpt truncated: 16384 source/output bytes]\n";
        }
        write_file('attempt-' . ++$index . '.txt', "selected_attempt=" . sanitize($relative) . "\n$tail");
        print "Selected attempt log $index (sanitized excerpt):\n";
        print "| $_\n" for split /\n/, substr($tail, 0, 1024);
    }
    write_file('collection.txt', "selected_attempt_logs=$index\nMaximum 4 explicit install.log files, 16384 source bytes each.\n"
        . ($attempts_omitted ? "Additional reported attempts omitted at the selection limit.\n" : '')
        . ($index ? '' : "No explicit attempt logs were reported; setup output retained instead.\n"));
    1;
};
if (!$ok) {
    my $error = sanitize($@);
    print STDERR "Setup diagnostic collection failed: $error";
    eval { write_file('collection.txt', "Collection incomplete: $error"); 1 };
}
else { print "Sanitized setup evidence: $output\n"; }
exit $status;
