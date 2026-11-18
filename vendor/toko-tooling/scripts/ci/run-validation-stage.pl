#!/usr/bin/env perl
# Shared companions: scripts/ci/ReleaseTiming.pm
use strict;
use warnings;
use Cwd qw(abs_path getcwd);
use Config;
use Digest::SHA qw(sha256_hex);
use DynaLoader ();
use File::Path qw(make_path);
use File::Temp qw(tempdir);
use File::Spec;
use Fcntl qw(O_RDONLY O_NONBLOCK O_WRONLY O_CREAT O_EXCL S_ISREG);
use Getopt::Long qw(GetOptions);
use JSON::PP;
use POSIX qw(uname);
use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC);
use FindBin qw($RealBin);
use lib $RealBin;
use ReleaseTiming;
my $timing_span;
END {
    my $status=$?;
    if ($timing_span) {
        eval {ReleaseTiming::finish($timing_span,$status)};
        if ($@) {warn $@; $status ||= 1;}
    }
    $?=$status;
}

Getopt::Long::Configure(qw(no_auto_abbrev no_ignore_case require_order));
my ($state, $stage, $contract, $source, $group, $observer, $environment, $limit, $admission_output);
GetOptions('state=s'=>\$state, 'stage=s'=>\$stage, 'contract=s'=>\$contract,
    'source=s'=>\$source, 'group=s'=>\$group, 'observe=s'=>\$observer,
    'admission-output=s'=>\$admission_output,
    'environment=s'=>\$environment, 'max-bytes=s'=>\$limit)
    && @ARGV && !grep { !defined($_) || !length($_) } ($state,$stage,$contract,$source,$group,$observer,$environment,$limit)
    or die "usage: run-validation-stage.pl --state DIR --stage NAME --contract REV --source ID --group NAME --observe EXECUTABLE --environment JSON --max-bytes N -- COMMAND [ARG...]\n";
$stage =~ /\A[A-Za-z0-9][A-Za-z0-9_-]*\z/ && $group =~ /\A[A-Za-z0-9][A-Za-z0-9_-]*\z/
    && $contract =~ /\A[A-Za-z0-9][A-Za-z0-9_.\/-]*\z/ && $source =~ /\A(?:[0-9a-f]{40}|[0-9a-f]{64})\z/
    && $limit =~ /\A[1-9][0-9]{0,8}\z/ or die "invalid stage identity or byte allowance\n";
my $json = JSON::PP->new->canonical->utf8;
my $root = getcwd();
"$root$state$observer$environment" !~ /[\x00\r\n]/ or die "invalid stage path\n";

sub read_input {
    my ($path,$digest) = @_;
    -f $path && -r $path && -s $path <= $limit or die "missing, unreadable or oversized stage input: $path\n";
    sysopen my $in, $path, O_RDONLY|O_NONBLOCK or die "read $path: $!\n";
    binmode $in;
    my @metadata = stat($in);
    @metadata && S_ISREG($metadata[2]) or die "stage input is no longer a regular file: $path\n";
    my $value = '';
    my $size = 0;
    my $hash = $digest ? Digest::SHA->new(256) : undef;
    while (1) {
        my $remaining = $limit - $size;
        my $count = read($in, my $chunk, $remaining < 65536 ? $remaining + 1 : 65536);
        defined($count) or die "read stage input $path: $!\n";
        last unless $count;
        $count <= $remaining or die "stage input grew beyond allowance: $path\n";
        $size += $count;
        if ($digest) { $hash->add($chunk); } else { $value .= $chunk; }
    }
    close $in or die "close $path: $!\n";
    return $digest ? $hash->hexdigest : $value;
}
sub bytes { return read_input($_[0],0); }
sub file_digest { return read_input($_[0],1); }
sub exact_keys {
    my ($object, @keys) = @_;
    ref($object) eq 'HASH' && join(',',sort keys %$object) eq join(',',sort @keys)
        or die "unknown or missing stage schema fields\n";
}
sub canonical_record {
    my ($path,$raw) = @_;
    $raw //= bytes($path);
    my $value = $json->decode($raw);
    $raw eq $json->encode($value)."\n" or die "stage JSON must be canonical (duplicate keys and ambiguous encodings refuse): $path\n";
    return $value;
}
sub safe_route {
    my ($path) = @_;
    $path !~ /[\x00\r\n]|(?:^|\/)\.{1,2}(?:\/|$)|\/\// or die "unsafe stage state/input route\n";
    my $cursor = $path =~ m{^/} ? '' : $root;
    for my $part (split m{/}, $path) {
        next unless length $part;
        $cursor .= "/$part";
        !-l $cursor or die "symlink in stage state/input route: $cursor\n";
    }
}
sub publish {
    my ($path,$value,$raw) = @_;
    $raw //= $json->encode($value)."\n";
    sysopen my $out, $path, O_WRONLY|O_CREAT|O_EXCL, 0600 or die "create immutable evidence $path: $!\n";
    binmode $out;
    print {$out} $raw or die "write stage evidence: $!\n";
    close $out or die "close stage evidence: $!\n";
    return $raw;
}
if (defined $admission_output) {
    safe_route($admission_output);
    !-e $admission_output && !-l $admission_output or die "occupied stage admission output\n";
}
safe_route($state);
safe_route("$state/proofs");
safe_route("$state/attempts");
make_path("$state/proofs", "$state/attempts");
$state = abs_path($state) // die "invalid stage state\n";
my $attempt = tempdir("$stage-$group.XXXXXX", DIR=>"$state/attempts", CLEANUP=>0);
print STDERR "Validation stage evidence retained: $attempt\n";
my $env = canonical_record($environment);
ref($env) eq 'HASH' && defined($env->{PATH}) && !ref($env->{PATH}) && length($env->{PATH})
    && !grep { $_ !~ /\A[A-Za-z_][A-Za-z0-9_]*\z/ || !defined($env->{$_}) || ref($env->{$_}) || $env->{$_} =~ /\x00/ } keys %$env
    or die "stage requires an explicit bounded string environment including PATH\n";
# Preserve invocation-scoped Cargo policy across the bounded child environment.
# It is part of the context hashed for proof lookup and subsequent execution.
if (exists $ENV{CARGO_NET_OFFLINE}) {
    !exists $env->{CARGO_NET_OFFLINE} || $env->{CARGO_NET_OFFLINE} eq $ENV{CARGO_NET_OFFLINE}
        or die "conflicting invocation and stage CARGO_NET_OFFLINE settings\n";
    $env->{CARGO_NET_OFFLINE} = $ENV{CARGO_NET_OFFLINE};
}
safe_route($observer);
$observer = abs_path($observer) // die "missing stage observer\n";
-x $observer or die "stage observer is not executable\n";
my @command = @ARGV;
my ($program,$selected_program);
for my $candidate ($command[0] =~ m{/} ? $command[0] : map { "$_/$command[0]" } split /:/,$env->{PATH}) {
    if (-f $candidate && -x $candidate) { $program = abs_path($candidate); $selected_program = File::Spec->rel2abs($candidate); last; }
}
defined($program) or die "stage executable absent from bounded environment\n";
$command[0] = $selected_program; # Preserve argv[0] for dispatchers such as rustup's cargo proxy.
my $start = clock_gettime(CLOCK_MONOTONIC);
local $ENV{RELEASE_GROUP}=$group;
local $ENV{RELEASE_SOURCE}=$source;
my $runner_digest = file_digest(abs_path($0));

sub run_child {
    my ($out,$err,@argv) = @_;
    my $pid = fork(); defined($pid) or die "fork stage: $!\n";
    if (!$pid) {
        %ENV = %$env;
        open STDOUT, '>:raw', $out or POSIX::_exit(125);
        open STDERR, '>:raw', $err or POSIX::_exit(125);
        exec { $argv[0] } @argv or POSIX::_exit(127);
    }
    local $SIG{TERM} = sub { kill 'TERM',$pid; waitpid($pid,0); die "validation stage interrupted (TERM)\n"; };
    local $SIG{INT} = sub { kill 'INT',$pid; waitpid($pid,0); die "validation stage interrupted (INT)\n"; };
    waitpid($pid,0) == $pid or die "wait stage: $!\n";
    return $? & 127 ? 128 + ($? & 127) : $? >> 8;
}
my ($last_observer_bytes,$last_inventory);
sub observe {
    my ($phase) = @_;
    my $observer_digest = file_digest($observer);
    my $command_span = ReleaseTiming::begin("validation-stage:$stage:$phase-observer-command",'executed',$attempt);
    my $status = run_child("$attempt/$phase-observation.json", "$attempt/$phase-observation.stderr", $observer);
    eval { ReleaseTiming::finish($command_span,$status) };
    if ($@) { $status ? warn($@) : die($@); }
    if ($status) {
        my $diagnostic = eval { bytes("$attempt/$phase-observation.stderr") };
        print STDERR defined($diagnostic) ? $diagnostic : $@;
        print STDERR "stage observation failed ($status); no stage admitted\n";
        exit $status;
    }
    $observer_digest eq file_digest($observer) && $runner_digest eq file_digest(abs_path($0))
        or die "stage observer/runner changed during observation\n";
    -x $selected_program && abs_path($selected_program) eq $program && -x $observer or die "stage executable admission changed during observation\n";
    my $observation = timed("$phase-observation-json",sub {
        my $path = "$attempt/$phase-observation.json";
        my $raw = bytes($path);
        if (!defined($last_observer_bytes) || $raw ne $last_observer_bytes) {
            $last_inventory = canonical_record($path,$raw);
            $last_observer_bytes = $raw;
        }
        return $last_inventory;
    });
    exact_keys($observation,qw(schema files configuration));
    $json->encode($observation->{schema}) eq '1' && ref($observation->{files}) eq 'ARRAY' && @{$observation->{files}}
        && ref($observation->{configuration}) eq 'HASH' or die "unsupported or incomplete stage observation\n";
    my @files;
    timed("$phase-listed-files",sub {
        my %seen;
        for my $path (@{$observation->{files}}) {
            defined($path) && !ref($path) && length($path) && !$seen{$path}++ or die "invalid/duplicate stage input\n";
            safe_route($path) unless $path =~ m{^/}; # Absolute tool routes may use managed selectors.
            $path !~ /[\x00\r\n]/ or die "invalid stage input path\n";
            my $resolved = abs_path($path) // die "missing stage input: $path\n";
            push @files, {path=>$path,resolved=>$resolved,sha256=>file_digest($resolved)};
        }
        return 1;
    });
    return {schema=>1,stage=>$stage,contract=>$contract,cwd=>$root,command=>\@command,
        observer=>{path=>$observer,sha256=>$observer_digest},
        runner=>{sha256=>$runner_digest,perl=>file_digest($^X),
            modules=>{map { my $path=abs_path($_) // die "missing verifier runtime input\n";
                $path=>file_digest($path) }
                (values(%INC), @DynaLoader::dl_shared_objects)},
            json=>$JSON::PP::VERSION,digest=>$Digest::SHA::VERSION},
        program=>file_digest($program),environment=>$env,host=>[uname()],
        observation=>{configuration=>$observation->{configuration},files=>[sort {$a->{path} cmp $b->{path}} @files]}};
}
sub timed {
    my ($phase,$code)=@_;
    return ReleaseTiming::measure("validation-stage:$stage:$phase",'executed',$attempt,$code);
}
my $context = timed('initial-observation',sub { observe('before') });
my $context_raw = timed('context-serialization',sub { $json->encode($context) });
my $key = sha256_hex($context_raw);
publish("$attempt/context.json",$context,$context_raw."\n");
my ($proof_path,$proof,$proof_digest,$mode);
sub check_logs {
    my ($record) = @_;
    exact_keys($record->{logs},qw(stdout stderr));
    for my $stream (qw(stdout stderr)) {
        my $log = $record->{logs}{$stream};
        exact_keys($log,qw(path sha256));
        $log->{path} eq "$record->{origin}{attempt}/command.$stream" or die "invalid stage log provenance\n";
        safe_route($log->{path});
        file_digest($log->{path}) eq $log->{sha256} or die "original stage log drift\n";
    }
}
my @candidates;
timed('proof-lookup',sub {
opendir my $proofs, "$state/proofs" or die "read stage proofs: $!\n";
@candidates = sort map { "$state/proofs/$_" } grep { /^\Q$key\E\.[A-Za-z0-9_-]+\.json\z/ } readdir $proofs;
closedir $proofs or die "close stage proofs: $!\n";
for my $candidate (@candidates) {
    safe_route($candidate);
    my $admitted = eval {
        # The canonical context is the first member of a v1 proof. Match
        # those exact bytes, then validate canonical JSON for the remaining
        # envelope using null in their place. Duplicate keys still refuse.
        my $raw = bytes($candidate);
        my $prefix = '{"context":';
        my $end = length($prefix) + length($context_raw);
        substr($raw,0,length($prefix)) eq $prefix
            && substr($raw,length($prefix),length($context_raw)) eq $context_raw
            && substr($raw,$end,1) eq ',' or die "ineligible original stage context\n";
        my $envelope = $prefix.'null'.substr($raw,$end);
        my $record = $json->decode($envelope);
        $json->encode($record)."\n" eq $envelope or die "noncanonical original stage proof\n";
        exact_keys($record,qw(schema kind context origin result logs elapsed_seconds));
        $json->encode($record->{schema}) eq '1' && $record->{kind} eq 'release-stage-execution'
            && $json->encode($record->{result}) eq '0'
            && $json->encode($record->{elapsed_seconds}) =~ /\A(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:e[+-]?[0-9]+)?\z/i
            or die "ineligible original stage proof\n";
        exact_keys($record->{origin},qw(source group attempt));
        $record->{origin}{source} =~ /\A(?:[0-9a-f]{40}|[0-9a-f]{64})\z/
            && $record->{origin}{group} =~ /\A[A-Za-z0-9][A-Za-z0-9_-]*\z/
            && index($record->{origin}{attempt},"$state/attempts/") == 0
            or die "invalid original stage provenance\n";
        check_logs($record);
        $proof = $record;
        $proof_digest = sha256_hex($raw);
        1;
    };
    if ($admitted) { $proof_path=$candidate; $mode='reused'; last; }
    print STDERR "Prior proof retained; executing ordinarily if no valid match remains: $@";
}
return 1;
});
$timing_span=ReleaseTiming::begin("validation-stage:$stage",
    defined($proof_path)?'reused':@candidates?'fallback':'executed',
    defined($proof_path)?$proof->{origin}{attempt}:$attempt);
if (!defined $proof_path) {
    my $execution_start = clock_gettime(CLOCK_MONOTONIC);
    my $status = run_child("$attempt/command.stdout", "$attempt/command.stderr", @command);
    my $output_error = '';
    for my $stream (qw(stdout stderr)) {
        my $value = eval { bytes("$attempt/command.$stream") };
        if ($@) { $output_error .= $@; next; }
        if ($stream eq 'stdout') { print $value; } else { print STDERR $value; }
    }
    if ($status) {
        eval { publish("$attempt/failure.json",{schema=>1,result=>$status,source=>$source,group=>$group}) };
        print STDERR "Stage failed ($status); evidence publication also failed: $@" if $@;
        print STDERR $output_error if length $output_error;
        exit $status;
    }
    die $output_error if length $output_error;
    my $after = timed('post-execution-observation',sub { observe('after-execution') });
    $context_raw eq $json->encode($after) or die "stage inputs changed during execution; no proof admitted\n";
    $proof = {schema=>1,kind=>'release-stage-execution',context=>$context,
        origin=>{source=>$source,group=>$group,attempt=>$attempt},result=>0,
        logs=>{map { $_=>{path=>"$attempt/command.$_",sha256=>file_digest("$attempt/command.$_")} } qw(stdout stderr)},
        elapsed_seconds=>clock_gettime(CLOCK_MONOTONIC)-$execution_start};
    $proof_path = "$state/proofs/$key.".substr($attempt,-6).".json";
    $proof_digest = sha256_hex(publish($proof_path,$proof));
    $mode='executed';
}
my $final = timed('final-observation',sub { observe('admission') });
timed('custody-checks',sub {
$context_raw eq $json->encode($final) or die "stage inputs drifted before admission\n";
file_digest($proof_path) eq $proof_digest or die "stage execution proof changed before admission\n";
check_logs($proof);
return 1;
});
my $admission = {schema=>1,kind=>'release-stage-admission',mode=>$mode,
    source=>$source,group=>$group,stage=>$stage,contract=>$contract,context_sha256=>$key,
    execution_proof=>$proof_path,execution_proof_sha256=>$proof_digest,
    elapsed_seconds=>clock_gettime(CLOCK_MONOTONIC)-$start};
publish("$attempt/admission.json",$admission);
if (defined $admission_output) { safe_route($admission_output); publish($admission_output,$admission); }
print "Validation stage $stage: $mode; admission $attempt/admission.json\n";
