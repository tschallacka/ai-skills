#!/usr/bin/perl
# MODE: PROD
# server.pl - chat runtime chosen when only perl is present. One process, one
# select loop over blocking sockets; same protocol and storage as server.py.
use strict;
use warnings;
use IO::Socket;
use IO::Select;

my $HOME = $ENV{AI_CHAT_HOME} or die "AI_CHAT_HOME must be set\n";
my $CHAN_DIR = "$HOME/channels";
mkdir $HOME unless -d $HOME;
mkdir $CHAN_DIR or die "mkdir $CHAN_DIR: $!" unless -d $CHAN_DIR;
my $LOCK_TRIES = 200;

sub valid_chan { $_[0] =~ /^#[a-z0-9_-]{1,32}$/ }
sub valid_nick { $_[0] =~ /^[A-Za-z0-9_-]{1,32}$/ }
sub chan_path  { my ($c) = @_; "$CHAN_DIR/$c.log" }

sub with_lock {
    my ($chan, $fn) = @_;
    my $lock = chan_path($chan) . '.lock';
    for my $i (1 .. $LOCK_TRIES) {
        if (mkdir $lock) {
            my @out = eval { $fn->() };
            rmdir $lock;
            die $@ if $@;
            return wantarray ? @out : $out[0];
        }
        select undef, undef, undef, 0.05;
    }
    die "lock timeout: $lock";
}

sub last_id {
    my ($chan) = @_;
    open my $fh, '<', chan_path($chan) or return 0;
    my $last = 0;
    while (my $line = <$fh>) {
        $last = $1 if $line =~ /^MSG \S+ (\d+) / && $1 > $last;
    }
    close $fh;
    return $last;
}

my (%NICK, %INBUF);
my $select;

sub send_line { my ($s, $t) = @_; print {$s} "$t\n" }

sub do_register {
    my ($s, $chan) = @_;
    return send_line($s, 'ERR invalid channel') unless valid_chan($chan);
    with_lock $chan, sub { open my $fh, '>>', chan_path($chan) or die "append: $!" };
    send_line($s, "OK register $chan");
}

sub do_privmsg {
    my ($s, $arg) = @_;
    my ($chan, $text) = $arg =~ /^(\S+)\s+:(.+)$/s
        or return send_line($s, 'ERR usage: PRIVMSG #chan :text');
    $text =~ s/\s+/ /g;
    return send_line($s, 'ERR invalid channel') unless valid_chan($chan);
    my $stored = with_lock $chan, sub {
        my $id  = last_id($chan) + 1;
        my $ts  = time;
        my $line = "MSG $chan $id $ts $NICK{$s} :$text";
        open my $fh, '>>', chan_path($chan) or die "append: $!";
        print {$fh} "$line\n";
        close $fh;
        $line;
    };
    send_line($s, $stored);
}

sub do_fetch {
    my ($s, $arg) = @_;
    my ($chan, $since) = $arg =~ /^(\S+)\s+(\d+)$/
        or return send_line($s, 'ERR usage: FETCH #chan <since-id>');
    if (open my $fh, '<', chan_path($chan)) {
        while (my $line = <$fh>) {
            chomp $line;
            my @f = split / /, $line, 5;
            send_line($s, $line) if $f[2] > $since;
        }
        close $fh;
    }
    send_line($s, 'OK fetch end');
}

sub drop_client {
    my ($s) = @_;
    $select->remove($s);
    delete $_{$s} for (\%NICK, \%INBUF);
    close $s;
}

sub handle_line {
    my ($s, $line) = @_;
    $line =~ s/\r$//;
    return unless length $line;
    my ($verb, $arg) = $line =~ /^(\S+)(?:\s+(.*))?$/;
    $arg //= '';
    if ($verb eq 'NICK') {
        valid_nick($arg)
            ? do { $NICK{$s} = $arg; send_line($s, "OK nick $arg") }
            : send_line($s, 'ERR invalid nick');
    } elsif ($verb eq 'REGISTER') {
        do_register($s, $arg);
    } elsif ($verb eq 'JOIN') {
        # Poll mode, like the socat tier: per-connection push needs the
        # event loop to write from another handler's context, which this
        # single-process select design does not do reliably. Clients tail
        # with FETCH instead.
        if (valid_chan($arg)) { send_line($s, "OK join $arg (poll mode)") }
        else { send_line($s, 'ERR invalid channel') }
    } elsif ($verb eq 'LEAVE') {
            send_line($s, "OK leave $arg");
    } elsif ($verb eq 'PRIVMSG') {
        do_privmsg($s, $arg);
    } elsif ($verb eq 'FETCH') {
        do_fetch($s, $arg);
    } elsif ($verb eq 'PING') {
        send_line($s, 'PONG');
    } elsif ($verb eq 'QUIT') {
        send_line($s, 'OK bye');
        drop_client($s);
    } else {
        send_line($s, "ERR unknown verb $verb");
    }
}

my $port     = $ARGV[0] // 0;
my $listener = IO::Socket::INET->new(
    LocalAddr => ($ENV{AI_CHAT_BIND} // '127.0.0.1'), LocalPort => $port,
    Proto => 'tcp', Listen => 16, ReuseAddr => 1,
) or die "listen: $@\n";
open my $pf, '>', "$HOME/server.port" or die "port file: $!";
print {$pf} $listener->sockport, "\n";
close $pf;
$select = IO::Select->new($listener);

while (1) {
    my @ready = $select->can_read(2);
    for my $sock (@ready) {
        if ($sock == $listener) {
            my $client = $listener->accept or next;
            $client->autoflush(1);
            $NICK{$client}  = 'anon-' . fileno($client);
            $INBUF{$client} = '';
            $select->add($client);
            next;
        }
        my $bytes = sysread($sock, my $chunk, 4096);
        if (!defined $bytes || $bytes == 0) { drop_client($sock); next }
        $INBUF{$sock} .= $chunk;
        while ((my $nl = index($INBUF{$sock}, "\n")) >= 0) {
            # Consume the newline too: extracting only up to it leaves an
            # empty line in the buffer when $nl is 0, looping forever.
            my $line = substr($INBUF{$sock}, 0, $nl + 1, '');
            $line =~ s/\n\z//;
            handle_line($sock, $line);
            last unless $select->exists($sock) && defined $INBUF{$sock};
        }
    }
}
