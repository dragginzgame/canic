#!/usr/bin/env perl
use strict;
use warnings;

# Read-only fixture preparation, not a general TOML validator or manifest editor.
# Callers select two adjacent, single-line dependencies in a valid Cargo manifest.
@ARGV == 4 or die "usage: prepare-cargo-formatting-fixture.pl MANIFEST TABLE FIRST SECOND\n";
my ($path, $table, $first, $second) = @ARGV;
$table =~ /\A[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]+)*\z/
    && $first =~ /\A[A-Za-z0-9_-]+\z/ && $second =~ /\A[A-Za-z0-9_-]+\z/
    && $first ne $second or die "invalid table or dependency selection\n";
$path !~ /[\r\n]/ && -f $path && !-l $path or die "expected regular manifest\n";
open my $input, '<:raw', $path or die "cannot read manifest: $!\n";
my $bytes = do { local $/; <$input> };
close $input or die "cannot close manifest: $!\n";
$bytes !~ /\0|"""|'''/ or die "unsupported manifest bytes or multiline string\n";
my @lines = $bytes =~ /[^\n]*\n|[^\n]+\z/g;
my @text = map { my $line = $_; $line =~ s/\r?\n\z//; $line } @lines;
my @tables = grep { $text[$_] =~ /\A[ \t]*\[\Q$table\E\][ \t]*(?:#.*)?\z/ } 0 .. $#text;
@tables == 1 or die "missing or ambiguous dependency table\n";
my $start = $tables[0] + 1;
my $end = scalar @text;
for my $index ($start .. $#text) {
    if ($text[$index] =~ /\A[ \t]*\[/) { $end = $index; last; }
}
my @first = grep { $text[$_] =~ /\A[ \t]*\Q$first\E[ \t]*=/ } $start .. $end - 1;
my @second = grep { $text[$_] =~ /\A[ \t]*\Q$second\E[ \t]*=/ } $start .. $end - 1;
@first == 1 && @second == 1 && $second[0] == $first[0] + 1
    or die "expected one adjacent dependency pair in the selected order\n";
for my $index ($first[0], $second[0]) {
    my ($value) = $text[$index] =~ /\A[^=]+=[ \t]*(.*)\z/;
    $value =~ /\A(?:"(?:[^"\\]|\\.)*"|'[^']*'|\{.*\})[ \t]*(?:#.*)?\z/
        or die "selected dependency must use a single-line string or inline table\n";
}
# Preserve newline bytes at their original positions, including missing final LF.
my @ending = map { /([\r]?\n)\z/ ? $1 : '' } @lines;
($text[$first[0]], $text[$second[0]]) = ($text[$second[0]], $text[$first[0]]);
binmode STDOUT;
for my $index (0 .. $#text) {
    print STDOUT $text[$index], $ending[$index] or die "cannot write fixture: $!\n";
}
close STDOUT or die "cannot close fixture output: $!\n";
