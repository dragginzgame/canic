use strict;
use warnings;
use JSON::PP qw(decode_json);
# Consumer-owned local package identities come from Cargo's workspace metadata.
my ($metadata_file, $lock_file, $previous, $candidate) = @ARGV;
open my $metadata_input, '<', $metadata_file or die "$metadata_file: $!\n";
my $metadata = decode_json(do { local $/; <$metadata_input> });
my %members = map { $_ => 1 } @{$metadata->{workspace_members}};
my %owned = map { $_->{name} => 1 }
    grep { $members{$_->{id}} && $_->{version} eq $previous }
    @{$metadata->{packages}};
open my $lock_input, '<', $lock_file or die "$lock_file: $!\n";
my $text = do { local $/; <$lock_input> };
my %changed;
$text =~ s{(\[\[package\]\]\n.*?)(?=\n\[\[package\]\]|\z)}{
    my $block = $1; my ($name) = $block =~ /^name = "([^"]+)"$/m;
    if ($owned{$name // ''} && $block !~ /^source = /m) {
        die "local package version changed before release: $name\n"
            unless $block =~ s/^version = "\Q$previous\E"$/version = "$candidate"/m;
        $changed{$name}++;
    }
    for my $package (keys %owned) {
        $block =~ s/"\Q$package $previous\E"/"$package $candidate"/g;
    }
    $block
}gse;
for my $package (keys %owned) {
    die "lockfile must contain one local package: $package\n"
        unless ($changed{$package} // 0) == 1;
}
print $text or die "lockfile output: $!\n";
