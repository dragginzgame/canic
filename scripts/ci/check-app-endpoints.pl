#!/usr/bin/env perl
# Check declarations before expansion: Canic itself emits Platform attributes.
use strict;
use warnings;
use FindBin;
use lib "$FindBin::Bin/../../vendor/toko-tooling/scripts/ci";
use RustSource qw(code_mask);

my $failed = 0;
open my $files, '-|', 'git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard',
    '--', 'crates', 'apps/examples', 'apps/template' or die $!;
{
    local $/ = "\0";
    while (my $path = <$files>) {
        chomp $path;
        next unless $path =~ /\.rs\z/ && -f $path;
        open my $file, '<', $path or die "$path: $!";
        my $source = do { local $/; <$file> };
        close $file or die "$path: $!";
        my $code = code_mask($source);
        while ($source =~ /\bmethod_id\s*\(\s*"canic\.[\w.-]+\.v(\d+)"/g) {
            my ($version, $offset) = ($1, $-[0]);
            next if $version eq '1' || substr($code, $offset, 9) ne 'method_id';
            my $line = 1 + (substr($code, 0, $offset) =~ tr/\n/\n/);
            warn "$path:$line: use v1 for a versioned pre-1.0 Canic Wire contract\n";
            $failed = 1;
        }
        next unless $path =~ m{^apps/};
        # Balance square brackets so nested arrays cannot end the attribute early.
        while ($code =~ /\#\s* (\[ (?: [^\[\]]++ | (?1) )* \]) /gx) {
            my ($attribute, $offset) = ($1, $-[1]);
            # Platform requires these named options on every query/update, even
            # through renamed imports or cfg_attr. Lifecycle exports have no name.
            next unless $attribute =~ /\bruntime\s*=/ && $attribute =~ /\bmax_bytes\s*=/ && $attribute =~ /\bname\s*=/;
            my $line = 1 + (substr($code, 0, $offset) =~ tr/\n/\n/);
            warn "$path:$line: use #[canic::query] or #[canic::update] for an application endpoint\n";
            $failed = 1;
        }
    }
}
close $files or die "git ls-files failed: $?";
exit $failed;
