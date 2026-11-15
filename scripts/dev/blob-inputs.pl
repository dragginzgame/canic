#!/usr/bin/env perl
# Fingerprint the explicitly selected local Blob source and both consumer locks.
# No supplier files, Git index or lockfiles are modified.
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use File::Find qw(find);
use JSON::PP;
my $supplier = '../ic-blob-storage';
sub git_output {
    my (@args) = @_;
    open my $fh, '-|', 'git', '-C', $supplier, @args or die "git: $!";
    local $/; my $text = <$fh>; close $fh or die 'supplier Git inspection failed';
    return $text;
}
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
my $head = git_output('rev-parse', 'HEAD'); chomp $head;
print $json->pretty->encode({supplier_head => $head, supplier_status => git_output('status', '--short'), supplier_files => \%hashes,
    supplier_content_sha256 => sha256_hex($json->encode(\%hashes)), consumer_locks => \%consumer});
