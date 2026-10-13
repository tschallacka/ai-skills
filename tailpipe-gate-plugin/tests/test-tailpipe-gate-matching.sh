#!/usr/bin/env bash
# MODE: DEV
# test-tailpipe-gate-matching.sh -- the pattern-matching logic
# pre-tool-use-bash.sh shares via hooks/lib.sh.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
plugin_dir="$(cd "$tests_dir/.." && pwd)"
repo_root="$(cd "$plugin_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

# shellcheck source=tailpipe-gate-plugin/hooks/lib.sh
source "$plugin_dir/hooks/lib.sh"

matches() { # <command>
    if tailpipe_gate_matches "$1"; then printf yes; else printf no; fi
}

t_assert_eq 'plain command does not match' "$(matches 'ls -la')" 'no'
t_assert_eq 'an ordinary pipe to less does not match (not an ongoing process)' \
    "$(matches 'ls -la | less')" 'no'
t_assert_eq 'tail -f matches' "$(matches 'tail -f app.log')" 'yes'
t_assert_eq 'tail -F matches' "$(matches 'tail -F app.log')" 'yes'
t_assert_eq 'tail --follow matches' "$(matches 'tail --follow app.log')" 'yes'
t_assert_eq 'tail -f combined with another flag still matches' \
    "$(matches 'tail -f -n 50 app.log')" 'yes'
t_assert_eq 'tail with no follow flag does not match' "$(matches 'tail -n 20 app.log')" 'no'
t_assert_eq 'a piped tail -f matches' "$(matches 'ssh host tail -f /var/log/app.log')" 'yes'
t_assert_eq 'a backgrounded command redirecting output matches' \
    "$(matches 'npm run dev > dev.log 2>&1 &')" 'yes'
t_assert_eq 'a backgrounded command piping its output matches' \
    "$(matches 'long_build.sh | tee build.log &')" 'yes'
t_assert_eq 'a backgrounded command with no redirect at all does not match' \
    "$(matches 'sleep 300 &')" 'no'
t_assert_eq 'a redirect with no backgrounding does not match (it already blocks and finishes)' \
    "$(matches 'long_build.sh > build.log 2>&1')" 'no'

# B319-equivalent: tailpipe_gate_rjq_bin resolves the installer's own shared
# copy first, never the user's global PATH.
resolver_work="$(mktemp -d "${TMPDIR:-/tmp}/tailpipe-gate-rjq-resolve.XXXXXX")"
trap 'rm -rf "$resolver_work"' EXIT
export XDG_CONFIG_HOME="$resolver_work/config"
mkdir -p "$XDG_CONFIG_HOME/tsch-ai-skills/bin"
printf '#!/bin/sh\necho stub\n' > "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"
chmod +x "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"

t_assert_eq 'tailpipe_gate_rjq_bin resolves the installed shared copy' \
    "$(tailpipe_gate_rjq_bin)" "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"

rm -f "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"
t_assert_eq 'tailpipe_gate_rjq_bin does not fall back to an ambient rjq on PATH' \
    "$(tailpipe_gate_rjq_bin || true)" ''

t_end 'test-tailpipe-gate-matching'
