#!/usr/bin/env perl
# Fingerprint the immutable Blob source selected by Cargo and both consumer locks.
# No supplier files, Git index or lockfiles are modified.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use File::Find qw(find);
use JSON::PP;
open my $metadata_fh, '-|', 'cargo', 'metadata', '--locked', '--offline', '--format-version', '1', '--manifest-path', 'qualification/Cargo.toml' or die "metadata: $!";
my $metadata;
{ local $/; $metadata = decode_json(<$metadata_fh>); }
close $metadata_fh or die 'locked supplier metadata failed';
my @packages = grep { $_->{name} eq 'ic-blob-storage' } @{$metadata->{packages}};
@packages == 1 or die 'expected one Blob supplier';
my $package = $packages[0];
my $source = $package->{source} // '';
$source =~ m{^git\+https://github\.com/dragginzgame/ic-blob-storage\?rev=([a-f0-9]{40})#\1$}
    or die 'Blob must use an exact immutable Git revision';
my $supplier = $package->{manifest_path};
$supplier =~ s{/crates/ic-blob-storage/Cargo\.toml$}{} or die 'unexpected supplier layout';
my @files = map { "$supplier/$_" } qw(Cargo.toml Cargo.lock qualification/Cargo.toml qualification/Cargo.lock rust-toolchain.toml);
find({ no_chdir => 1, wanted => sub { push @files, $File::Find::name if -f $_ } }, "$supplier/crates/ic-blob-storage");
my %hashes;
for my $path (sort @files) {
    open my $fh, '<:raw', $path or die "$path: $!";
    local $/; my $bytes = <$fh>; close $fh;
    (my $relative = $path) =~ s{^\Q$supplier\E/}{};
    $hashes{$relative} = sha256_hex($bytes);
}
my %consumer;
for my $path (qw(Cargo.lock qualification/Cargo.lock)) {
    open my $fh, '<:raw', $path or die "$path: $!";
    local $/; $consumer{$path} = sha256_hex(<$fh>); close $fh;
}
my $json = JSON::PP->new->canonical;
print $json->pretty->encode({supplier_source => $source, supplier_files => \%hashes,
    supplier_content_sha256 => sha256_hex($json->encode(\%hashes)), consumer_locks => \%consumer});
