#!/usr/bin/perl


use strict;
use warnings;
use feature 'say';
use IPC::Open3;
use File::Spec;
use Config;
use Carp;
use IO::Socket::UNIX;
use IO::Poll;
use File::Temp;


sub print_banner {
    my @banner = (
        "******************************************",
        "*                USBoverSSH              *",
        "*     USB & Remote Hosts Bridge Tool     *",
        "*                  v1.4.1                *",
        "*      ----------------------------      *",
        "*                        by @ImKKingshuk *",
        "* Github- https://github.com/ImKKingshuk *",
        "******************************************"
    );
    my $width = 80; 
    foreach my $line (@banner) {
        printf "%*s\n", (($width + length($line)) / 2), $line;
    }
    say "";
}


my $SSH = 'ssh';
my $PERL = $Config{perlpath};
my $drivers = File::Spec->catdir('sys', 'bus', 'usb', 'drivers');
my $devices = File::Spec->catdir('sys', 'bus', 'usb', 'devices');
my $verbose;


print_banner();


eval { $$1 = "\Q$2\E"; shift } or croak $@ while $ARGV[0] =~ /^(\w+)=(.*)/s;
*DEBUG = sub { say STDERR "@_" } if $verbose;

if ($ARGV[0] eq 'remote') {
    shift;
    remote_script(@ARGV);
} elsif ($ARGV[0] eq 'list') {
    shift;
    list_devices(@ARGV);
} elsif ($ARGV[0] eq 'find') {
    shift;
    say find_dev(@ARGV);
} elsif ($ARGV[0] eq 'unbind') {
    shift;
    unbind(@ARGV);
} elsif ($ARGV[0] eq 'detach') {
    shift;
    local_detach(@ARGV);
} elsif ($ARGV[0] eq 'keep') {
    shift;
    persistent(@ARGV);
} elsif ($ARGV[0] eq 'daemon') {
    shift;
    daemon(@ARGV);
} elsif (length $ARGV[0]) {
    exit xwait(local_script(@ARGV));
} else {
    my $cmd = $0 =~ s{^.*/}{}r;
    croak <<"EOT";
Usage:
    $cmd USER\@HOST DEV_PATTERN
        Attach single device matching DEV_PATTERN from HOST

    $cmd USER\@HOST [list]
        List devices from HOST

    $cmd USER\@HOST find DEV_PATTERN
        Find single device matching DEV_PATTERN on HOST

    $cmd unbind USER\@HOST DEV_PATTERN
        Unbind single device matching DEV_PATTERN from HOST

    $cmd detach USER\@HOST DEV_PATTERN
        Detach single device matching DEV_PATTERN from HOST

    $cmd keep USER\@HOST ...
        Same commands as above, but keep trying to connect to HOST
        and to reconnect to it if the connection is broken

    $cmd daemon USER\@HOST ...
        Same as 'keep' but detached from the tty and using syslog
        for messages and errors

    $cmd list
    $cmd find DEV_PATTERN
        Same commands as above on the local machine

DEV_PATTERN is as returned by the 'list' command: a busid like 3-3.1, a
vid:pid like 03f0:e111, or a pattern matching the vid:pid, the product name
or the serial number.

You can add command line options for ssh before USER\@HOST.
EOT
}


sub readfile {
    open my $h, shift or return "";
    local $/;
    <$h> =~ s/\s+$//r;
}

sub xwritefile {
    my $f = shift;
    say STDERR "WRITE $f < @_";
    open my $h, '>', $f or croak "open> $f: $!\n";
    syswrite $h, "@_" or croak "write> $f @_: $!\n";
}

sub xreadfile {
    my $f = shift;
    open my $h, '<', $f or croak "open< $f: $!\n";
    local $/;
    my $d = <$h> or croak "readline< $f: $!\n";
    $d =~ s/\s+$//;
    die "\nempty file $f" unless length $d;
    say STDERR "READ $f > $d";
    $d;
}

sub xsystem {
    say STDERR "EXEC @_";
    system(@_) == -1 and croak "system $_[0]: $!";
    croak "system @_: status " . ($? >> 8) if $?;
}

sub xtmpdir {
    require File::Temp;
    File::Temp->newdir("USBoverSSH-XXXXXX", TMPDIR => 1);
}

use constant {
    LOW            => 1,
    FULL           => 2,
    HIGH           => 3,
    SUPER          => 5,
    SUPER_PLUS     => 6,
    SDEV_ST_AVAILABLE => 1,
    SDEV_ST_USED      => 2,
    VDEV_ST_NULL      => 4,
};

sub remote_attach {
    my ($sockfd, $busid) = @_;
    my $status = readfile "$drivers/usbip-host/$busid/usbip_status";

    if ($status != SDEV_ST_AVAILABLE) {
        if ($status == SDEV_ST_USED) {
            xwritefile "$devices/$busid/usbip_sockfd", -1;
        } else {
            xsystem qw(/sbin/modprobe -v usbip-host);
        }

        xwritefile "$drivers/usb/unbind", $busid;
        xwritefile "$drivers/usbip-host/match_busid", "add $busid";
        xwritefile "$drivers/usbip-host/bind", $busid;
    }

    xwritefile "$devices/$busid/usbip_sockfd", $sockfd;
}

sub local_attach {
    my ($sockfd, $bus, $dev, $speed) = @_;
    my %hspeed = (1.5 => LOW, 12 => FULL, 480 => HIGH, 5000 => SUPER, 10000 => SUPER_PLUS, 20000 => SUPER_PLUS, 12 => LOW);
    my $hspeed = $hspeed{$speed} // HIGH;
    xsystem qw(/sbin/modprobe -v vhci-hcd);
    my ($vhci, $port) = find_vhci_and_port($hspeed);
    xwritefile "$vhci/attach", sprintf "%d %d %d %d", $port, $sockfd, $bus << 16 | $dev, $hspeed;
}

sub local_detach {
    my $busid = join '|', map { /^all$/i ? '.*' : quotemeta } @_;
    my (@b, $n);

    for my $f (<$devices/platform/vhci_hcd.*/status*>) {
        open my $h, '<', $f or warn "open $f: $!" and next;
        <$h>;   

        while (<$h>) {
            my ($hub, $port, $sta, $spd, $dev, $sock_fd, $lbusid) = split;
            next unless $sta == 6;    
            push @b, $lbusid;

            if ($lbusid =~ $busid) {
                my ($vhci) = $f =~ m{(.*)/};
                xwritefile "$vhci/detach", int($port);
                $n++;
            }
        }
    }

    die "no devices attached\n" unless @b;
    die "'@_' did not match any of: @b\n" unless $n;
}

sub unbind {
    my $busid = find_dev(@_);
    xwritefile "$drivers/usbip-host/unbind", $busid;
    xwritefile "$drivers/usbip-host/match_busid", "del $busid";
    xwritefile "$drivers/usbip-host/rebind", $busid;
}

sub find_vhci_and_port {
    my $speed = shift;
    my $shub = $speed >= SUPER ? 'ss' : 'hs';

    for my $f (<$devices/platform/vhci_hcd.*/status*>) {
        open my $h, '<', $f or warn "open $f: $!" and next;
        <$h>;   

        while (<$h>) {
            my ($hub, $port, $sta) = split;
            return $f =~ m{^(.*)/}, int($port) if $sta == VDEV_ST_NULL and $hub eq $shub;
        }
    }

    croak "no suitable vhci port found for speed = $speed";
}

sub find_dev {
    my ($pat) = @_;

    if ($pat =~ /^\d+-\d+(?:\.\d+)*$/) {
        return $pat if -d "$devices/$pat";
        croak "no such device $pat";
    }

    s/\b0*(?=[0-9a-f]+:)/\\b0*/i, s/:0*([0-9a-f]+)\b/:0*$1\\b/i, $_ = qr/$_/ for my $rpat = $pat;
    say STDERR "looking for $rpat inside $devices";
    my @found;

    for my $p (<$devices/[0-9]*>) {
        next unless my ($busid) = $p =~ m{/(\d+-\d+(?:\.\d+)*)$};
        my ($vid, $pid, $serial, $manufacturer, $product) = map readfile("$p/$_"), qw(idVendor idProduct serial manufacturer product);
        my $spec = sprintf "%-10s %s:%s  %s  %s  %s  %s %s", $busid, $vid, $pid, $serial, $manufacturer, $product;
        push @found, $busid if my $found = $spec =~ $rpat;
        say STDERR sprintf "  <%d> %s", $found, $spec;
    }

    return $found[0] if @found == 1;
    croak @found ? "multiple" : "no", " devices match '$rpat'";
}

sub xwait {
    waitpid shift // -1, 0;
    $? ? ($? >> 8 or $? | 64) : 0;
}

sub ssh_myself {
    my ($stdout, $stderr, $ssh, $uhost, @args) = @_;
    unshift @args, 'verbose=1' if $verbose;
    say STDERR "EXEC @$ssh $uhost $PERL - @args";
    my $pid = open3 my $to, $_[0], $_[1], @$ssh, $uhost, $PERL, '-', @args;
    print $to our $script or croak;
    say STDERR "pid=$pid";
    return $pid;
}

sub ssh_myself_simple {
    xwait ssh_myself '>&STDOUT', '>&STDERR', @_;
}

sub local_script {
    my @ssh = $SSH;

    while ($_[0] =~ /^-/) {
        push @ssh, my $o = shift;
        push @ssh, shift
          if $o =~ /^-[1246afgknqstvxACGKMNPTVXYy]*[bceilmopBDEFIJLOQRSwW]$/;
    }

    my $uhost = shift;

    unless (@_) {
        return ssh_myself_simple [@ssh], $uhost, 'list';
    } elsif ($_[0] =~ /^(?:list|unbind|find)$/) {
        return ssh_myself_simple [@ssh], $uhost, @_;
    }

    my $tmpdir = xtmpdir;
    my @ssh_S = ($ssh[0], -S => "$tmpdir/ctl", $uhost);
    my $pid = ssh_myself my $sfrom, undef, [@ssh, -S => "$tmpdir/ctl", '-M'], $uhost, 'remote', @_;
    my $old_handler = $SIG{__DIE__};
    $SIG{__DIE__} = sub {
        $SIG{__DIE__} = $old_handler;
        use POSIX ':sys_wait_h';

        if (waitpid($pid, WNOHANG) == 0) {
            system @ssh_S, qw(-O exit);
            say STDERR "    $_" while <$sfrom>;
            xwait $pid;
        }

        croak @_;
    };

    my $response;

    while (<$sfrom>) {
        if (/^RESPONSE (.*)/) {
            $response = $1;
            last;
        } else {
            say STDERR "    $_";
        }
    }

    croak "remote failed to attach\n" unless $response;
    my ($bus, $dev, $speed, $rpath) = split ' ', $response;
    my $lpath = "$tmpdir/vhci";
    xsystem @ssh_S, qw(-O forward), -L => "$lpath:$rpath";
    require IO::Socket::UNIX;
    my $sock = IO::Socket::UNIX->new($lpath)
      or croak "connect to $lpath: $!";
    unlink $lpath or warn "unlink $lpath: $!";
    sysread $sock, my $d, 1 or croak "sysread on $lpath: $!\n";
    say STDERR sprintf "DEVSPEC: fd=%d bus/dev=%d/%d (%d) speed=%d", $sock->fileno, $bus, $dev, $bus << 16 | $dev, $speed;
    local_attach($sock->fileno, $bus, $dev, $speed);
    close $sock;
    xsystem @ssh_S, qw(-q -O stop);   
    undef $tmpdir;                   
    say STDERR "   $_" while <$sfrom>;
    return $pid;
}

sub remote_script {
    my ($spec, $opt) = @_;
    my $busid = find_dev($spec);
    my ($bus, $dev, $speed) = map xreadfile("$devices/$busid/$_"), qw(busnum devnum speed);
    my $sock;

    {
        my $tmpdir = xtmpdir;
        my $path = "$tmpdir/host";
        require IO::Socket::UNIX;
        my $srv = IO::Socket::UNIX->new(Listen => 1, Local => $path)
          or croak "bind to $path: $!";
        printf "RESPONSE %d %d %s %s\n", $bus, $dev, $speed, $path;
        open STDOUT, ">/dev/null" or croak "open STDOUT>/dev/null: $!";
        vec(my $rb, $srv->fileno, 1) = 1;
        select($rb, undef, undef, .4)
          or croak "timeout waiting for connection on $path";
        $sock = $srv->accept or croak "accept on $path: $!";
        remote_attach($sock->fileno, $busid);
        syswrite $sock, "T", 1 or croak "syswrite on $path: $!";
        unlink $path;
    }

    exit if $opt eq 'nolinger';

    require IO::Poll;
    sub POLLRDHUP () { 0x2000 }
    my $poll = IO::Poll->new;
    $poll->mask($sock, POLLRDHUP);
    die "fork: $!" unless defined(my $pid = fork);
    exit if $pid;
    use POSIX qw(setsid);
    setsid or croak "setsid: $!";
    open *STDIN, '+</dev/null' or croak "open /dev/null: $!";
    open $_, '>&', *STDIN or croak "open $_ >/dev/null: $!" for *STDOUT, *STDERR;
    my @args = @_;
    unshift @args, '-y';   
    persistent(@args);
}

sub list_devices {
    sub dv { $_[0] =~ s/(\d+)/reverse $1/ge }
    opendir my $d, $devices or croak "opendir $devices: $!";
    my $odev;
    my %c = (
        '01' => 'audio',    '02' => 'COM',
        '03' => 'HID',
        '030001' => 'kbd',
        '030002' => 'mouse',
        '030101' => 'kbd',   
        '030102' => 'mouse',   
        '07' => 'printer',    '08' => 'UMASS',
        '09' => 'hub',        '0a' => 'CDC',
        '0e' => 'video',
        'e0' => 'wireless',
        'e00101' => 'bluetooth',
        'e00103' => 'RNDIS',
    );

    sub read_uevent { split /[=\n]/, readfile "$_[0]/uevent" }
    for (sort { dv($a) cmp dv($b) } readdir $d) {
        next unless my ($vid, $pid, $serial, $manufacturer, $product, $busnum, $devnum, $speed) = read_uevent "$devices/$_";
        my ($i, $l) = ("$_ $manufacturer $product");
        $l =~ s/\s+$//;
        $i .= " $serial" if $serial;
        $i .= " $vid:$pid";
        $i .= " $c{$vid}" if $c{$vid};
        $i .= " $_ $l" if $l ne $i;
        $i .= " [$_]";
        $i .= " $busnum:$devnum";
        $i .= " $speed" if $speed;
        $odev->{$busnum}{$devnum} = $i;
    }

    for my $busnum (sort keys %$odev) {
        for my $devnum (sort { $a <=> $b } keys %{$odev->{$busnum}}) {
            say $odev->{$busnum}{$devnum};
        }
    }
}

sub daemon {
    my $pid = fork;
    croak "fork: $!" unless defined $pid;
    exit if $pid;
    setsid or croak "setsid: $!";
    open *STDIN, '+</dev/null' or croak "open /dev/null: $!";
    open $_, '>&', *STDIN or croak "open $_ >/dev/null: $!" for *STDOUT, *STDERR;
    local_detach(@_);
}

sub persistent {
    local $SIG{CHLD} = 'IGNORE';
    my $status = 0;

    while (1) {
        eval {
            local $SIG{__DIE__} = sub { $status = 1; die @_ };
            $status = local_script(@_);
        };

        last unless $status;
        sleep 1;
    }

    exit 0;
}

__END__
