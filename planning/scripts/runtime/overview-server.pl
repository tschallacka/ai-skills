# MODE: PROD
# Minimal localhost HTTP server for the plan overview (perl rung, core
# modules only: IO::Socket::INET). Serves / (rendered HTML), /state.json
# and /sections/<id>; delegates every render to the bash scripts; prints
# the bound port on startup.
use strict; use warnings;
use File::Basename qw(dirname);
use Cwd qw(abs_path);
use IO::Socket::INET;

my ($plan_dir, $want_port) = @ARGV;
$want_port ||= 0;
my $sd = dirname(dirname(abs_path($0)));
my $state_script  = "$sd/overview-state.sh";
my $render_script = "$sd/render-plan-overview.sh";

sub run_bash {
    my ($script, @args) = @_;
    my $cmd = join(' ', 'bash', "\"$script\"", @args) . ' 2>/dev/null';
    my $out = `$cmd`;
    return defined($out) ? $out : "";
}


# The renderer writes a file and prints its path; render to a temp and read
# it back so the response is exactly the artifact.
sub render_to_temp {
    my $tmp = "/tmp/overview-serve-$$.html";
    system('bash', $render_script, '--serve', $plan_dir, '--out', $tmp);
    my $body = do { local $/; open my $fh, '<', $tmp or return undef; <$fh> };
    unlink $tmp;
    return $body;
}

$| = 1;    # the port must reach the invoker before the accept loop blocks
my $srv = IO::Socket::INET->new(
    LocalAddr => '127.0.0.1', LocalPort => $want_port,
    Proto => 'tcp', Listen => 5, ReuseAddr => 1,
) or die "cannot bind: $!\n";
print $srv->sockport, "\n";

while (my $c = $srv->accept) {
    my $req = <$c>;
    last unless defined $req;
    my ($method, $path) = $req =~ m{^(\S+)\s+(\S+)};
    # drain headers
    while (my $h = <$c>) { last if $h =~ /^\r?\n$/; }
    $path =~ s/\?.*//;
    my ($code, $ctype, $body);
    if ($path eq '/state.json' or $path eq '/state') {
        $code = 200; $ctype = 'application/json; charset=utf-8';
        $body = run_bash($state_script, "\"$plan_dir\"");
    } elsif ($path =~ m{^/sections/([a-z-]+)$}) {
        my $id = $1;
        my $html = render_to_temp();
        if (!defined($html) || length($html) == 0) {
            $code = 500; $ctype = 'text/plain'; $body = "render failed\n";
        } elsif ($html =~ m{(<[^>]*id="\Q$id\E"[^>]*>[\s\S]*?)(?=<[^>]*id="(?:identity-panel|step-details|tests-panel|coverage-panel|findings-panel|dep-graph|narr)"|</main>|</body>)}) {
            $code = 200; $ctype = 'text/html; charset=utf-8'; $body = $1;
        } else {
            $code = 404; $ctype = 'text/plain'; $body = "Not found\n";
        }
    } else {
        # B51 class: a failed render is a 500, never a 200 with an empty body.
        $body = render_to_temp();
        if (defined($body) && length($body) > 0) {
            $code = 200; $ctype = 'text/html; charset=utf-8';
        } else {
            $code = 500; $ctype = 'text/plain'; $body = "render failed\n";
        }
    }
    binmode $c, ':raw';
    print $c "HTTP/1.0 $code @{[$code == 200 ? 'OK' : 'Not Found']}\r\n",
             "Content-Type: $ctype\r\n",
             "Content-Length: ", length($body), "\r\n",
             "Connection: close\r\n\r\n",
             $body;
    close $c;
}
