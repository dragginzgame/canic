#!/usr/bin/env perl
# Fingerprint the immutable Blob source selected by Cargo and both consumer locks.
# No supplier files, Git index or lockfiles are modified.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use File::Find qw(find);
use JSON::PP;
open my $metadata_fh, '-|', 'cargo', 'metadata', '--locked', '--format-version', '1', '--manifest-path', 'qualification/Cargo.toml' or die "metadata: $!";
my $output = do { local $/; <$metadata_fh> };
unless (close $metadata_fh) {
    my $status = $?;
    my $code = $status < 0 ? 1 : ($status & 127) ? 128 + ($status & 127) : $status >> 8;
    warn "cargo metadata failed for qualification/Cargo.toml (exit $code)\n";
    exit $code;
}
my $metadata = decode_json($output);
my @packages = grep { $_->{name} eq 'ic-blob-storage' } @{$metadata->{packages}};
@packages == 1 or die 'expected one Blob supplier';
my $package = $packages[0];
my $source = $package->{source} // '';
$source eq 'registry+https://github.com/rust-lang/crates.io-index'
    or die 'Blob must use its locked registry archive';
my $supplier = $package->{manifest_path};
$supplier =~ s{/Cargo\.toml$}{} or die 'unexpected supplier layout';
open my $checksum_fh, '<', 'qualification/Cargo.lock' or die "archive checksum: $!";
my $checksum;
{ local $/; my $lock = <$checksum_fh>;
  ($checksum) = $lock =~ /name = "ic-blob-storage"\nversion = "\Q$package->{version}\E"\nsource = "\Q$source\E"\nchecksum = "([a-f0-9]{64})"/;
  defined $checksum or die 'missing registry archive checksum'; }
close $checksum_fh;
my @files = map { "$supplier/$_" } qw(Cargo.toml Cargo.toml.orig .cargo_vcs_info.json);
find({ no_chdir => 1, wanted => sub { push @files, $File::Find::name if -f $_ } }, "$supplier/src");
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
    local $/; my $bytes = <$fh>; close $fh;
    $bytes =~ /name = "ic-blob-storage"\nversion = "\Q$package->{version}\E"\nsource = "\Q$source\E"\nchecksum = "\Q$checksum\E"/
        or die "$path does not select the qualified Blob archive";
    $consumer{$path} = sha256_hex($bytes);
}
my $json = JSON::PP->new->canonical;
print $json->pretty->encode({supplier_source => $source, supplier_version => $package->{version}, supplier_archive_sha256 => $checksum, supplier_files => \%hashes,
    supplier_content_sha256 => sha256_hex($json->encode(\%hashes)), consumer_locks => \%consumer});
