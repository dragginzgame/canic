#!/usr/bin/env perl
# Canic owns scope admission. Shared Tooling owns plan parsing and execution.
use strict;
use warnings;
use JSON::PP;
use Digest::SHA qw(sha256_hex);

my $json = JSON::PP->new->canonical->utf8;
my ($attempt, $force) = @ARGV;
defined($attempt) && (!defined($force) || $force eq '--workspace') or die "usage: validation-plan.pl ATTEMPT [--workspace]\n";
$attempt =~ m{\Atarget/release-validation/[A-Za-z0-9_./-]+\z} && $attempt !~ m{(?:^|/)\.\.(?:/|$)}
    or die "validation attempt must be under target/release-validation\n";
my $group = $ENV{RELEASE_GROUP} // '';
my $source = $ENV{RELEASE_SOURCE} // '';

sub command {
    open my $in, '-|', @_ or die "start command: $!\n";
    my $text = do { local $/; <$in> };
    close $in or die "command failed: @_\n";
    return $text;
}
sub bytes {
    my ($path) = @_;
    open my $in, '<:raw', $path or die "read $path: $!\n";
    my $raw = do { local $/; <$in> };
    close $in or die "close $path: $!\n";
    return $raw;
}
sub write_json {
    my ($path, $value) = @_;
    open my $out, '>:raw', $path or die "write $path: $!\n";
    print {$out} $json->encode($value), "\n" or die "write $path: $!\n";
    close $out or die "close $path: $!\n";
}
chomp(my $head = command('git', 'rev-parse', 'HEAD'));
$source eq $head && $source =~ /\A(?:[a-f0-9]{40}|[a-f0-9]{64})\z/ or die "select the admitted source\n";
length(command('git', 'status', '--porcelain')) == 0 or die "validation selection requires clean source\n";
my $policy = $json->decode(bytes('ci/release-validation.json'));
my $scopes = $json->decode(bytes('ci/release-validation-scopes.json'));
my $groups = $json->decode(bytes('ci/cargo-release-groups.json'));
my @names = map { $_->{name} } @{$groups->{groups}};
$json->encode(\@names) eq $json->encode($policy->{groups})
    && join(',', sort @names) eq join(',', sort keys %{$scopes->{groups}})
    && $scopes->{schema} == 1 or die "validation and Cargo group rosters differ\n";
exists $scopes->{groups}{$group} or die "unknown release group\n";
my @roster = grep length, split /\n/, command('make', '--no-print-directory', '-s', 'release-validation-roster');
$json->encode(\@roster) eq $json->encode($policy->{stages}) or die "validation policy does not cover the full gate\n";
my %stages = map { $_ => 1 } @roster;
my $scope = $scopes->{groups}{$group};
for my $definition (values %{$scopes->{groups}}) {
    $stages{$_} or die "unknown omitted stage: $_\n" for keys %{$definition->{omit}};
}
my @files = sort split /\0/, command('git', 'ls-tree', '-rz', '--name-only', $source);
# The CLI default pin is transaction-derived metadata, like package versions.
# Its exact bytes still belong to the source checkpoint and candidate check.
my %mapping_inputs = map { $_ => sha256_hex(bytes($_)) } grep {
    m{\A(?:scripts/|make/|ci/|\.github/)} || /\A(?:Makefile|\.shared-tooling.snapshot|rust-toolchain.toml|tool-versions.env)\z/
} grep { $_ ne 'scripts/dev/install_dev.sh' } @files;
my $mapping = sha256_hex($json->encode(\%mapping_inputs)."\n");
my %graph_inputs = map { $_ => sha256_hex(bytes($_)) } grep {
    /\ACargo\.(?:toml|lock)\z/ || m{\A(?:crates|canisters)/.+/Cargo\.toml\z}
        || m{\Aintegrations/blob-service/(?:consumer|embedded-consumer)/Cargo\.(?:toml|lock)\z}
} @files;
my $graph = sha256_hex($json->encode(\%graph_inputs)."\n");
my ($definition) = grep { $_->{name} eq $group } @{$groups->{groups}};
my $primary = $definition->{primary};
my $manifest = bytes("crates/$primary/Cargo.toml");
$manifest =~ /^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$/m or die "selected package version missing\n";
my $tag = "$group-v$1";
my ($baseline, $qualification);
# Missing first split tags or an unqualified baseline deliberately select all.
if (command('git', 'tag', '--list', $tag) ne '') {
    chomp(my $type = command('git', 'cat-file', '-t', "refs/tags/$tag"));
    $type eq 'tag' or die "baseline must be an annotated group tag\n";
    chomp(my $commit = command('git', 'rev-list', '-n', '1', $tag));
    system('git', 'merge-base', '--is-ancestor', $commit, $source) == 0 or die "baseline is not an ancestor\n";
    chomp(my $object = command('git', 'rev-parse', "refs/tags/$tag"));
    $baseline = {tag => $tag, commit => $commit, object => $object};
    my $baseline_record = $json->decode(command('git', 'show', "$tag:releases/$group/release-validation.json"));
    my $carry = $baseline_record->{qualification};
    if ($carry && $carry->{schema} == 1 && $carry->{mapping} eq $mapping && $carry->{graph} eq $graph) {
        my $candidate = $carry->{basis};
        $candidate =~ /\A\Q$group\E-v\d+\.\d+\.\d+\z/ or die "invalid qualification basis\n";
        system('git', 'merge-base', '--is-ancestor', $candidate, $commit) == 0 or die "non-ancestral qualification\n";
        chomp(my $candidate_type = command('git', 'cat-file', '-t', "refs/tags/$candidate"));
        $candidate_type eq 'tag' or die "qualification must be annotated\n";
        my $raw = command('git', 'show', "$candidate:releases/$group/release-validation.json");
        my $record = eval { $json->decode($raw) };
        $record && $record->{schema} == 1 && ($record->{gate} // '') eq 'plan'
            && ($record->{group} // '') eq $group or die "invalid qualification receipt\n";
        my $proof = $record->{validation};
        $proof && $proof->{plan}{scope} eq 'workspace'
            && $proof->{context}{candidate_scope} eq 'group'
            && $proof->{context}{mapping} eq $mapping or die "mapping was not fully qualified\n";
        $candidate eq "$group-v$record->{version}" && $record->{source} eq $proof->{plan}{source}
            or die "qualification receipt identity differs\n";
        chomp(my $qualified_commit = command('git', 'rev-list', '-n', '1', $candidate));
        chomp(my $qualified_source = command('git', 'rev-parse', "$qualified_commit^"));
        $qualified_source eq $record->{source} or die "qualification was not the release source\n";
        chomp(my $qualified_policy = command('git', 'rev-parse', "$record->{source}:ci/cargo-release-groups.json"));
        $record->{policy} eq $qualified_policy or die "qualification policy differs\n";
        # Tagged receipts are authority; re-admit their exact source-bound proof,
        # without pretending retained raw logs are available in a fresh checkout.
        write_json("$attempt/qualification.json", $proof);
        system('perl', 'scripts/release/validation-evidence.pl', 'historical', "$attempt/qualification.json") == 0
            or die "invalid historical group qualification\n";
        chomp(my $qualified_object = command('git', 'rev-parse', "refs/tags/$candidate"));
        $qualification = {tag => $candidate, commit => $qualified_commit, object => $qualified_object,
            receipt_sha256 => sha256_hex($raw)};
    }
}
my @changed = $baseline ? sort(split /\0/, command('git', 'diff', '--no-renames', '--name-only', '-z', $baseline->{commit}, $source)) : @files;
my $narrow = @changed && keys(%{$scope->{omit}}) && $baseline;
for my $path (@changed) {
    # Manifests, generators, protocols, compiler and shared inputs have no
    # exception. Only the explicit implementation/test roots below may narrow.
    my $owned = $path eq "releases/$group/CHANGELOG.md" || $path eq 'docs/status/current.md';
    $owned ||= index($path, $_) == 0 for @{$scope->{paths}};
    $owned = 0 if $path =~ m{(?:^|/)(?:Cargo\.(?:toml|lock)|build\.rs)$};
    $narrow &&= $owned;
}
my $selected = $narrow && $qualification && !defined($force);
my $context = {
    schema => 1, source => $source, group => $group, baseline => $baseline,
    changed => \@changed, mapping => $mapping, mapping_inputs => \%mapping_inputs, graph => $graph,
    candidate_scope => $narrow ? 'group' : 'workspace', qualification => $qualification,
    native_hosts => $scopes->{native_hosts},
};
write_json("$attempt/context.json", $context);
my @selection = map { {
    target => $_,
    required => ($selected && exists $scope->{omit}{$_}) ? JSON::PP::false : JSON::PP::true,
    reason => ($selected && exists $scope->{omit}{$_}) ? $scope->{omit}{$_}
        : $selected ? 'Common workspace, native/Wasm, runtime identity, consumer and installed/recovery obligations remain required.'
        : 'Full gate: shared/uncertain inputs, explicit workspace selection or mapping awaiting same-source full qualification.',
} } @roster;
write_json("$attempt/plan.json", {
    schema => 1, source => $source, group => $group, scope => $selected ? 'group' : 'workspace',
    policy => 'ci/release-validation.json', inputs => [@files, "$attempt/context.json"], stages => \@selection,
});
print "$attempt/plan.json\n";
