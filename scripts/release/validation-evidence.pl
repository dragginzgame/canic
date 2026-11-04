#!/usr/bin/env perl
# Admit executed Canic plans at preparation/recovery and preserve their proof.
use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);
use File::Temp qw(tempdir);
use File::Path qw(make_path);
use Cwd qw(abs_path);

my $json = JSON::PP->new->canonical->utf8;
my ($operation, $path, $candidate_tree) = @ARGV;
defined($path) && ((@ARGV == 2 && $operation =~ /\A(?:seal|verify|historical)\z/)
    || (@ARGV == 3 && $operation eq 'project'))
    or die "usage: validation-evidence.pl seal ATTEMPT | verify PROOF | historical PROOF | project PROOF CANDIDATE-TREE\n";
my $root = abs_path('.');
sub fail { die "Canic validation evidence: @_\n" }
sub regular {
    my ($base, $relative) = @_;
    $relative !~ m{\A/|[\x00-\x1f]|(?:^|/)\.{1,2}(?:/|$)|//|/\z} or fail('invalid input path');
    my $route = $base;
    for (split m{/}, $relative) { $route .= "/$_"; !-l $route or fail("redirected input: $relative") }
    -f $route or fail("missing input: $relative");
    return $route;
}
sub bytes {
    my ($file) = @_;
    open my $in, '<:raw', $file or fail("read $file: $!");
    my $raw = do { local $/; <$in> };
    close $in or fail("close $file: $!");
    return $raw;
}
sub encoded { $json->encode($_[0])."\n" }
sub record { $json->decode(bytes($_[0])) }
sub write_json {
    my ($file, $value) = @_;
    open my $out, '>:raw', $file or fail("write $file: $!");
    print {$out} encoded($value) or fail("write $file: $!");
    close $out or fail("close $file: $!");
}
sub command {
    open my $in, '-|', @_ or fail('start command');
    my $text = do { local $/; <$in> };
    close $in or fail("command failed: @_");
    return $text;
}
sub same { encoded($_[0]) eq encoded($_[1]) or fail($_[2]) }
sub host_configuration {
    my %configuration = map { $_ => $ENV{$_} } grep {
        /\A(?:RUSTC|RUSTDOC|RUSTUP_TOOLCHAIN|RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|CARGO_BUILD_TARGET|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER)\z/
            || /\ACARGO_(?:PROFILE_|TARGET_)/
    } keys %ENV;
    return {system => command('uname', '-sm'), compiler => command($ENV{RUSTC} // 'rustc', '-vV'), configuration => \%configuration};
}
sub source_files { sort split /\0/, command('git', 'ls-tree', '-rz', '--name-only', $_[0]) }
sub graph_identity {
    my ($tree, @files) = @_;
    my %inputs = map { $_ => sha256_hex(bytes(regular($tree, $_))) } grep {
        /\ACargo\.(?:toml|lock)\z/ || m{\A(?:crates|canisters)/.+/Cargo\.toml\z}
            || m{\Aintegrations/blob-service/(?:consumer|embedded-consumer)/Cargo\.(?:toml|lock)\z}
    } @files;
    return sha256_hex(encoded(\%inputs));
}
sub source_tree {
    my ($source) = @_;
    make_path("$root/target/tmp");
    my $scratch = tempdir('canic-validation-source.XXXXXX', DIR => "$root/target/tmp", CLEANUP => 1);
    system('git', 'archive', '--format=tar', "--output=$scratch/source.tar", $source) == 0 or fail('archive source');
    make_path("$scratch/tree");
    system('tar', '-xf', "$scratch/source.tar", '-C', "$scratch/tree") == 0 or fail('extract source');
    return "$scratch/tree";
}
my $proof;
if ($operation eq 'seal') {
    $path =~ m{\Atarget/release-validation/[A-Za-z0-9_./-]+\z} or fail('invalid attempt');
    my @runs = glob "$path/logs/validation.*";
    @runs == 1 && -d $runs[0] && !-l $runs[0] or fail('missing or ambiguous runner evidence');
    my $run = $runs[0];
    my $plan = record(regular($root, "$path/plan.json"));
    same($plan, record(regular($root, "$run/parsed-inputs.json.plan.json")), 'parsed plan differs');
    my $policy = record(regular($root, "$run/parsed-inputs.json.policy.json"));
    same($policy, record(regular($root, $plan->{policy})), 'parsed policy differs');
    my %last;
    for my $line (split /\n/, bytes(regular($root, "$run/plan-events.tsv"))) {
        my ($target, $state) = split /\t/, $line;
        next if $target eq 'target';
        $last{$target} = $state;
    }
    my %timing;
    for my $line (split /\n/, bytes(regular($root, "$run/timings.tsv"))) {
        my ($target, $result, $seconds, $log) = split /\t/, $line;
        next if $target eq 'target';
        !$timing{$target} && $result eq 'PASS' && $seconds =~ /\A\d+\z/ or fail('failed or duplicate execution');
        $log =~ s{\A\Q$root\E/}{};
        index($log, "$run/") == 0 or fail('log outside admitted run');
        $timing{$target} = {target => $target, seconds => 0 + $seconds, log => $log,
            sha256 => sha256_hex(bytes(regular($root, $log))), execution => 'executed'};
    }
    my @executed;
    for my $stage (@{$plan->{stages}}) {
        my $target = $stage->{target};
        if ($stage->{required}) {
            ($last{$target} // '') eq 'PASS' && $timing{$target} or fail("blocked or failed stage: $target");
            push @executed, delete $timing{$target};
        } else {
            ($last{$target} // '') eq 'not-applicable' && !$timing{$target} or fail('excluded stage executed');
        }
        delete $last{$target};
    }
    !keys(%last) && !keys(%timing) or fail('unknown execution evidence');
    $proof = {schema => 1, plan => $plan, policy => $policy,
        context => record(regular($root, "$path/context.json")),
        inputs => record(regular($root, "$run/plan-inputs.json")),
        host => host_configuration(),
        execution => \@executed, retained_dir => $run};
} else {
    $proof = record($path);
}
if ($operation eq 'project') {
    # The existing canonical metadata renderer is the sole caller. Its exact
    # version/lock transaction can advance graph bytes without authorizing a
    # dependency/feature change or editing the original source-bound proof.
    my $plan = $proof->{plan};
    my $context = $proof->{context};
    my $projection;
    if ($context->{candidate_scope} eq 'group') {
        my $version = $ENV{RELEASE_VERSION} // '';
        $version =~ /\A\d+\.\d+\.\d+\z/ or fail('projection requires candidate version');
        my $basis = $plan->{scope} eq 'workspace' ? "$plan->{group}-v$version" : $context->{qualification}{tag};
        $projection = {schema => 1, mapping => $context->{mapping}, basis => $basis,
            graph => graph_identity(abs_path($candidate_tree), source_files($plan->{source}))};
    }
    print encoded($projection);
    exit;
}

# The current receipt stays schema 1. Historical complete/fast receipts retain
# their original shape and meaning; they are never inferred to be group proof.
same([sort keys %$proof], [sort qw(schema plan policy context inputs host execution retained_dir)], 'unknown proof fields');
$proof->{schema} == 1 or fail('unknown schema');
my ($plan, $policy, $context) = @{$proof}{qw(plan policy context)};
same([sort keys %$plan], [sort qw(schema source group scope policy inputs stages)], 'unknown plan fields');
same([sort keys %$policy], [sort qw(schema groups stages)], 'unknown policy fields');
$plan->{schema} == 1 && $policy->{schema} == 1 && $context->{schema} == 1 or fail('unknown plan/context schema');
my $source = $plan->{source};
my $group = $plan->{group};
$source =~ /\A(?:[a-f0-9]{40}|[a-f0-9]{64})\z/ && $group =~ /\A[a-z][a-z0-9-]*\z/ or fail('invalid source/group');
$source eq $context->{source} && $group eq $context->{group} or fail('context identity differs');
if ($operation ne 'historical') {
    $source eq ($ENV{CANIC_RELEASE_VALIDATED_HEAD} // $ENV{RELEASE_SOURCE} // '')
        && $group eq ($ENV{RELEASE_GROUP} // '') or fail('wrong admitted source/group');
    same($proof->{host}, host_configuration(), 'host/compiler/configuration changed; qualify a fresh plan');
}
my $tree = source_tree($source);
my @files = source_files($source);
$plan->{policy} eq 'ci/release-validation.json' or fail('wrong policy');
same($policy, record(regular($tree, $plan->{policy})), 'policy is not source-bound');
my $groups = record(regular($tree, 'ci/cargo-release-groups.json'));
same($policy->{groups}, [map { $_->{name} } @{$groups->{groups}}], 'Cargo group roster differs');
grep($_ eq $group, @{$policy->{groups}}) or fail('unknown group');
same([map { $_->{target} } @{$plan->{stages}}], $policy->{stages}, 'stage roster differs');
my @context_paths = grep { m{\Atarget/release-validation/.+/context\.json\z} } @{$plan->{inputs}};
@context_paths == 1 or fail('selection context missing');
my $context_path = $context_paths[0];
(my $plan_path = $context_path) =~ s{/context\.json\z}{/plan.json};
same([sort @{$plan->{inputs}}], [sort(@files, $context_path)], 'incomplete source/recipe inventory');
my %inputs = map { $_ => sha256_hex(bytes(regular($tree, $_))) } @files;
$inputs{$context_path} = sha256_hex(encoded($context));
$inputs{$plan_path} = sha256_hex(encoded($plan));
same($proof->{inputs}, \%inputs, 'source/configuration checkpoint differs');
my %mapping = map { $_ => $inputs{$_} } grep {
    m{\A(?:scripts/|make/|ci/|\.github/)} || /\A(?:Makefile|\.shared-tooling.snapshot|rust-toolchain.toml|tool-versions.env)\z/
} grep { $_ ne 'scripts/dev/install_dev.sh' } @files;
same($context->{mapping_inputs}, \%mapping, 'recipe inventory differs');
sha256_hex(encoded(\%mapping)) eq $context->{mapping} or fail('mapping identity differs');
graph_identity($tree, @files) eq $context->{graph} or fail('Cargo graph identity differs');
my $scopes = record(regular($tree, 'ci/release-validation-scopes.json'));
same($context->{native_hosts}, $scopes->{native_hosts}, 'native host obligations changed');
my $definition = $scopes->{groups}{$group} or fail('missing group scope');
my $baseline = $context->{baseline};
my @changed = @files;
if ($baseline) {
    $baseline->{tag} =~ /\A\Q$group\E-v\d+\.\d+\.\d+\z/ or fail('wrong baseline group');
    chomp(my $commit = command('git', 'rev-list', '-n', '1', $baseline->{tag}));
    $commit eq $baseline->{commit} or fail('baseline moved');
    chomp(my $object = command('git', 'rev-parse', "refs/tags/$baseline->{tag}"));
    $object eq $baseline->{object} or fail('baseline tag changed');
    system('git', 'merge-base', '--is-ancestor', $commit, $source) == 0 or fail('baseline is not an ancestor');
    @changed = sort split /\0/, command('git', 'diff', '--no-renames', '--name-only', '-z', $commit, $source);
}
same($context->{changed}, \@changed, 'changed inputs differ');
my $narrow = @changed && $baseline && keys %{$definition->{omit}};
for my $changed (@changed) {
    my $owned = $changed eq "releases/$group/CHANGELOG.md" || $changed eq 'docs/status/current.md';
    $owned ||= index($changed, $_) == 0 for @{$definition->{paths}};
    $owned = 0 if $changed =~ m{(?:^|/)(?:Cargo\.(?:toml|lock)|build\.rs)$};
    $narrow &&= $owned;
}
$context->{candidate_scope} eq ($narrow ? 'group' : 'workspace') or fail('invalid proposed scope');
$plan->{scope} =~ /\A(?:workspace|group)\z/ or fail('unknown scope');
if ($plan->{scope} eq 'group') {
    $narrow && $context->{qualification} or fail('unqualified group omission');
    my $qualification = $context->{qualification};
    my $tag = $qualification->{tag};
    my $baseline_record = $json->decode(command('git', 'show', "$baseline->{tag}:releases/$group/release-validation.json"));
    my $carry = $baseline_record->{qualification};
    $carry && $carry->{schema} == 1 && $carry->{graph} eq $context->{graph}
        && $carry->{mapping} eq $context->{mapping} && $carry->{basis} eq $tag
        or fail('baseline does not qualify this Cargo graph');
    $tag =~ /\A\Q$group\E-v\d+\.\d+\.\d+\z/ or fail('wrong qualification group');
    my $raw = command('git', 'show', "$tag:releases/$group/release-validation.json");
    sha256_hex($raw) eq $qualification->{receipt_sha256} or fail('qualification receipt changed');
    chomp(my $object = command('git', 'rev-parse', "refs/tags/$tag"));
    $object eq $qualification->{object} or fail('qualification tag changed');
    chomp(my $commit = command('git', 'rev-list', '-n', '1', $tag));
    $commit eq $qualification->{commit} or fail('qualification commit changed');
    my $previous = $json->decode($raw);
    my $qualified = $previous->{validation};
    $previous->{gate} eq 'plan' && $previous->{group} eq $group
        && $qualified->{plan}{scope} eq 'workspace' && $qualified->{context}{candidate_scope} eq 'group'
        && $qualified->{context}{mapping} eq $context->{mapping} or fail('mapping was not fully qualified');
    system('git', 'merge-base', '--is-ancestor', $tag, $baseline->{commit}) == 0 or fail('qualification is not ancestral');
}
my @required;
for my $stage (@{$plan->{stages}}) {
    JSON::PP::is_bool($stage->{required}) or fail('invalid selection');
    my $omit = $plan->{scope} eq 'group' && exists $definition->{omit}{$stage->{target}};
    ($stage->{required} ? 0 : 1) == ($omit ? 1 : 0) or fail('incorrect stage selection');
    defined($stage->{reason}) && !ref($stage->{reason}) && $stage->{reason} =~ /\S/
        && $stage->{reason} !~ /[\x00\t\r\n]/ or fail('missing single-line selection reason');
    push @required, $stage->{target} if $stage->{required};
}
same([map { $_->{target} } @{$proof->{execution}}], \@required, 'missing, duplicate or unexpected execution');
length($proof->{host}{system} // '') && length($proof->{host}{compiler} // '') or fail('missing host/compiler identity');
for my $executed (@{$proof->{execution}}) {
    $executed->{execution} eq 'executed' && $executed->{sha256} =~ /\A[a-f0-9]{64}\z/
        && $executed->{seconds} =~ /\A\d+\z/ or fail('incomplete execution proof');
    index($executed->{log}, "$proof->{retained_dir}/") == 0 or fail('unbound execution log');
    if ($operation ne 'historical') {
        sha256_hex(bytes(regular($root, $executed->{log}))) eq $executed->{sha256} or fail('missing or changed execution log');
    }
}
if ($operation eq 'seal') {
    chomp(my $head = command('git', 'rev-parse', 'HEAD'));
    $head eq $source && !length(command('git', 'status', '--porcelain')) or fail('source drifted during validation');
    # Runner observations are immutable; retain the proof before publishing its
    # atomic source/group pointer. Failed/interrupted attempts never own one.
    write_json("$path/evidence.json", $proof);
    make_path("target/release-validation/$group");
    my $destination = "target/release-validation/$group/$source.json";
    write_json("$destination.$$", $proof);
    rename "$destination.$$", $destination or fail('publish proof pointer');
    print "$destination\n";
} else {
    print STDERR "Canic $group $plan->{scope} validation evidence admitted\n";
}
