package ReleaseTiming;
use strict;
use warnings;
use Fcntl qw(:DEFAULT :flock S_ISREG);
use JSON::PP;
use POSIX qw(isfinite);
use Scalar::Util qw(looks_like_number);
use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC time);
my $json=JSON::PP->new->canonical;
my $sequence=0;
sub route {
    my ($path)=@_;
    $path =~ m{\A/} && $path !~ /[\x00-\x1f]/ or die "invalid timing journal path\n";
    my $part='';
    for (split '/', $path) {next unless length; $_ ne '.' && $_ ne '..' or die "noncanonical timing path\n"; $part.="/$_"; !-l $part or die "symlink timing route\n";}
}
sub append {
    my ($record)=@_;
    my $path=$ENV{RELEASE_TIMING_JOURNAL};
    return unless defined($path) && length($path);
    route($path);
    sysopen my $fh,$path,O_WRONLY|O_CREAT|O_APPEND|O_NONBLOCK|O_NOFOLLOW,0600 or die "open timing journal: $!\n";
    S_ISREG((stat $fh)[2]) or die "timing journal is not regular\n";
    flock($fh,LOCK_EX) or die "lock timing journal: $!\n";
    print {$fh} $json->encode($record),"\n" or die "write timing journal: $!\n";
    close $fh or die "close timing journal: $!\n";
}
sub begin {
    my ($phase,$mode,$origin)=@_;
    return unless length($ENV{RELEASE_TIMING_JOURNAL}//'');
    $mode =~ /\A(?:executed|reused|fallback)\z/ or die "invalid timing route\n";
    my $start=clock_gettime(CLOCK_MONOTONIC);
    my $id=join('-', $$, ++$sequence, sprintf('%.9f',$start));
    my $span={schema=>1,event=>'started',id=>$id,phase=>$phase,mode=>$mode,
        attempt=>$ENV{RELEASE_TIMING_ATTEMPT}//$id,parent=>$ENV{RELEASE_TIMING_PARENT}//'',
        group=>$ENV{RELEASE_GROUP}//'',source=>$ENV{RELEASE_SOURCE}//'',
        version=>$ENV{RELEASE_VERSION}//'',origin=>$origin//'',
        started_unix=>time(),started_monotonic=>$start};
    append($span);
    return $span;
}
sub finish {
    my ($span,$status)=@_;
    return unless $span;
    my $elapsed=clock_gettime(CLOCK_MONOTONIC)-$span->{started_monotonic};
    $elapsed>=0 or die "timing clock moved backwards\n";
    append({schema=>1,event=>'finished',id=>$span->{id},status=>0+$status,
        elapsed_seconds=>$elapsed,finished_unix=>time()});
}
sub measure {
    my ($phase,$mode,$origin,$code)=@_;
    my $span=begin($phase,$mode,$origin);
    local $ENV{RELEASE_TIMING_PARENT}=$span ? $span->{id} : ($ENV{RELEASE_TIMING_PARENT}//'');
    my ($value,$ok,$error);
    {local $@; $ok=eval {$value=$code->();1};$error=$@;}
    if (!$ok) {
        eval {finish($span,1)};
        warn "Timing finish also failed: $@" if $@;
        die $error;
    }
    finish($span,0);
    return $value;
}
sub report {
    my ($path)=@_; route($path);
    local $/ = "\n";
    return [] unless -e $path;
    sysopen my $fh,$path,O_RDONLY|O_NONBLOCK|O_NOFOLLOW or die "read timing journal: $!\n";
    S_ISREG((stat $fh)[2]) or die "timing journal is not regular\n";
    flock($fh,LOCK_SH) or die "lock timing journal: $!\n";
    my (@spans,%by_id);
    while (my $line=<$fh>) {
        # An interrupted last write is not a completed observation.
        $line =~ /\n\z/ or die "incomplete timing journal line; retain original bytes\n";
        my $r=$json->decode($line);
        ref($r) eq 'HASH' && !ref($r->{schema}) && ($r->{schema}//'') eq '1'
            && defined($r->{id}) && !ref($r->{id}) && length($r->{id})
            or die "invalid timing event\n";
        if (($r->{event}//'') eq 'started') {
            !exists($by_id{$r->{id}}) or die "duplicate timing start\n";
            my $row={%$r,result=>'incomplete'};delete $row->{event};
            push @spans,$row;$by_id{$r->{id}}=$row;
        } elsif (($r->{event}//'') eq 'finished') {
            defined($r->{status}) && !ref($r->{status})
                && $r->{status} =~ /\A(?:0|[1-9][0-9]*)\z/ && $r->{status} <= 255
                or die "invalid timing finish status\n";
            defined($r->{elapsed_seconds}) && !ref($r->{elapsed_seconds})
                && looks_like_number($r->{elapsed_seconds}) && isfinite(0+$r->{elapsed_seconds})
                && $r->{elapsed_seconds} >= 0
                or die "invalid timing elapsed seconds\n";
            my $row=$by_id{$r->{id}} or die "timing finish without start\n";
            $row->{result} eq 'incomplete' or die "duplicate timing finish\n";
            $row->{status}=$r->{status};$row->{elapsed_seconds}=$r->{elapsed_seconds};
            $row->{result}=$r->{status}==0?'passed':'failed';
        } else {die "unknown timing event\n";}
    }
    close $fh or die "close timing journal: $!\n";
    return \@spans;
}
# Use the outer monotonic observation for a single uninterrupted delivery. A
# resumed batch has separate clocks and may include operator wait between runs.
sub batch_metrics {
    my ($rows)=@_;
    my @attempts=grep { $_->{phase} eq 'batch-attempt' && !length($_->{parent}) } @$rows;
    return unless @attempts && $attempts[-1]{result} eq 'passed';
    my %roots=map { $_->{id}=>1 } @attempts;
    my (%phases,%checks,%packages);
    my ($uploads,$identified,$reuse)=(0,0,0);
    for my $row (@$rows) {
        next unless $roots{$row->{attempt}} && $row->{result} eq 'passed';
        my $phase=$row->{phase};
        $phases{$phase} += $row->{elapsed_seconds} if $roots{$row->{parent}};
        if ($phase =~ /\A(?:validation-stage|publication-admission):[^:]+\z/) {
            if ($row->{mode} eq 'reused') { $reuse++; }
            else { $checks{$phase}++; }
        }
        if ($phase eq 'cargo-publish') {
            $uploads++;
            if (($row->{origin}//'') =~ /\Apackages=(.+)\z/) {
                $identified++;
                $packages{$_}=1 for split /,/, $1;
            }
        }
    }
    return unless keys %phases;
    return {wall=>@attempts==1 ? $attempts[0]{elapsed_seconds} : undef,
        attempts=>scalar @attempts, start=>$attempts[0]{started_unix},
        phases=>\%phases, checks=>\%checks, reuse=>$reuse,
        packages=>$uploads && $identified==$uploads ? scalar(keys %packages) : undef};
}

sub describe_batch {
    my ($current,$groups,$prior,$prior_id)=@_;
    return "Release timing summary unavailable: no completed batch timing.\n" unless $current;
    my $wall=$current->{wall};
    my $out=defined($wall) ? sprintf("Batch wall time %.1fs", $wall)
        : "Resumed batch ($current->{attempts} attempts); uninterrupted wall time unavailable";
    $out.="; groups: ".join(', ',@$groups)."; packages observed by Cargo upload: ".($current->{packages}//'unrecorded')."\n";
    my $times=$current->{phases};
    my @top=sort {$times->{$b}<=>$times->{$a} || $a cmp $b} keys %$times;
    splice @top,3 if @top>3;
    my @labels;
    for my $phase (@top) {
        my $label=sprintf('%s %.1fs',$phase,$times->{$phase});
        if ($prior && exists $prior->{phases}{$phase}) {
            $label.=sprintf(' (%+.1fs)',$times->{$phase}-$prior->{phases}{$phase});
        }
        push @labels,$label;
    }
    $out.="Slowest outer phases (across groups): ".join(', ',@labels)."\n";
    my $counts=$current->{checks};
    my @repeated=sort grep {$counts->{$_}>1} keys %$counts;
    $out.="Repeated journaled checks: ".(@repeated ? join(', ',map {"$_ x$counts->{$_}"} @repeated) : 'none observed')
        ."; reused admissions: $current->{reuse}\n";
    if ($prior && defined($wall) && defined($prior->{wall})) {
        $out.=sprintf("Previous matching batch %s: %.1fs; change %+.1fs\n",$prior_id,$prior->{wall},$wall-$prior->{wall});
        my %all=(%$counts,%{$prior->{checks}});
        my @changed=sort grep {($counts->{$_}//0)!=($prior->{checks}{$_}//0)} keys %all;
        $out.="Changed check execution counts: ".join(', ',map {"$_ ".($prior->{checks}{$_}//0)." -> ".($counts->{$_}//0)} @changed)."\n" if @changed;
        $out.="Previous package count: ".($prior->{packages}//'unrecorded')."; previous reused admissions: $prior->{reuse}\n";
    } else { $out.="No comparable uninterrupted baseline available.\n"; }
    return $out."Cache warmth is unrecorded; compare like inputs before attributing a regression.\n";
}
1;
