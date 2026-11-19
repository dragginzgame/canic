package SetupDiagnostics;
use strict;
use warnings;
use Exporter 'import';
our @EXPORT_OK = qw(run_setup excerpt);

# Scan from the beginning for PEM context and retain a bounded tail.
# Command pipes drain to EOF. Callers can cap reads on an admitted file.
sub excerpt {
    my ($input, $remaining) = @_;
    @_ == 1 || (@_ == 2 && defined($remaining) && $remaining =~ /\A[0-9]+\z/)
        or die "expected a handle and optional nonnegative byte limit\n";
    my @secrets = grep { length($_) >= 4 } map { $ENV{$_} }
        grep { /TOKEN|SECRET|PASSWORD|CREDENTIAL|PRIVATE_KEY|API_KEY|ACCESS_KEY/i } keys %ENV;
    my (@tail, $line);
    my ($pem, $suppress, $omitted, $bytes) = (0, 0, 0, 0);
    $line = '';
    my $retain = sub {
        my ($text) = @_;
        $text =~ s/\e\[[0-?]*[ -\/]*[\@-~]//g;
        $text =~ tr/\t/ /;
        $text =~ s/[^\x20-\x7e]/?/g;
        $pem = 1 if $text =~ /-----BEGIN /;
        my $sensitive = $pem || $text =~ /authorization|cookie|passwd|password|secret|token|credential|private[ _-]?key|(?:api|access)[ _-]?key/i
            || $text =~ /\bBearer\s+\S+/i
            || $text =~ /payload\s*["']?\s*[=:]/i
            || $text =~ /^\s*[A-Za-z_][A-Za-z0-9_]*=/
            || $text =~ m{https?://[^\s/]+\@|https?://\S*[?#]}i
            || $text =~ /(?:gh[pousr]_|github_pat_|AKIA|ASIA)[A-Za-z0-9_]+|eyJ[A-Za-z0-9_-]+\.|[A-Za-z0-9+\/=]{80,}/
            || grep { index($text, $_) >= 0 } @secrets;
        $pem = 0 if $text =~ /-----END /;
        $text = '[sensitive line omitted]' if $sensitive;
        $text =~ s/::/: :/g; # Neutralize hosted workflow commands before display.
        push @tail, substr($text, 0, 500) . "\n";
        $bytes += length($tail[-1]);
        while (@tail > 99 || $bytes > 32000) {
            $bytes -= length(shift @tail);
            $omitted = 1;
        }
    };
    while (!defined($remaining) || $remaining > 0) {
        my $size = defined($remaining) && $remaining < 4096 ? $remaining : 4096;
        my $count = sysread($input, my $chunk, $size);
        defined $count or die "cannot read setup diagnostics\n";
        last unless $count;
        $remaining -= $count if defined $remaining;
        for my $part (split /(?<=\n)/, $chunk) {
            # Track PEM markers across chunks even when dropping an oversized
            # line. Resume on the next line unless a PEM block remains open.
            $line .= $part;
            if (length($line) > 8192 || $suppress) {
                $pem = 1 if $line =~ /-----BEGIN /;
                $suppress = $omitted = 1;
                $line = substr($line, -16);
            }
            if ($part =~ /\n\z/) {
                $line =~ s/\r?\n\z//;
                $retain->($line) unless $suppress;
                $line = '';
                $suppress = 0;
            }
        }
    }
    my $at_limit = defined($remaining) && $remaining == 0;
    # At the ceiling an unfinished line can contain only the prefix of a secret.
    $retain->($line) if !$at_limit && length($line) && !$suppress;
    my @notices;
    push @notices, 'earlier output or oversized lines omitted' if $omitted;
    push @notices, 'input truncated at byte limit' if $at_limit;
    return (@notices ? '[' . join('; ', @notices) . "]\n" : '')
        . (@tail ? join('', @tail) : "[no usable diagnostic output]\n");
}

# The caller chooses its artifact paths and collector. A collector exception
# must never replace the setup command's exit status.
sub run_setup {
    my ($on_failure, @command) = @_;
    ref($on_failure) eq 'CODE' && @command or die "expected collector and setup command\n";
    my $pid = open my $pipe, '-|';
    defined $pid or die "cannot launch setup\n";
    unless ($pid) {
        open STDERR, '>&', STDOUT or exit 126;
        exec { $command[0] } @command or exit 127;
    }
    my $log = eval { excerpt($pipe) };
    close $pipe;
    my $wait = $?;
    my $status = $wait == -1 ? 1 : ($wait & 127 ? 128 + ($wait & 127) : $wait >> 8);
    return 0 if !$status && defined $log;
    $status ||= 1;
    eval { $on_failure->($status, $log // "[setup output collection failed]\n"); 1 }
        or warn "Diagnostic collection failed; original setup status remains $status.\n";
    return $status;
}
1;
