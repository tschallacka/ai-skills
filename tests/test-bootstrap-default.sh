#!/usr/bin/env bash
# MODE: DEV
# test-bootstrap-default.sh — installer/bootstrap.sh's own argv default.
#
# install.sh's bare `curl ... | bash` launched the interactive skill picker.
# The compiled installer's argv parsing does not special-case an empty argv
# the same way (main.rs prints --help and exits 0), so bootstrap.sh supplies
# the default itself: no arguments means `interactive`, anything given is
# passed through untouched. This is the one behavior a doc fix cannot cover,
# so it gets a real test instead of a comment.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/bootstrap-default.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# A stub release tarball: its only "installer" is a script that records the
# argv it was handed, so this test asserts on what bootstrap.sh execs into,
# not on the real binary or a network fetch.
payload_dir="$work/payload"
mkdir -p "$payload_dir"
argv_log="$work/argv.log"
cat > "$payload_dir/installer" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$@" > "$argv_log"
EOF
chmod +x "$payload_dir/installer"
tar -czf "$work/release.tar.gz" -C "$payload_dir" installer

run_bootstrap() {
    rm -f "$argv_log"
    AI_SKILLS_NO_SPLASH=1 AI_SKILLS_RELEASE_URL="file://$work/release.tar.gz" \
        "$repo_root/installer/bootstrap.sh" "$@" </dev/null >/dev/null 2>&1
}

run_bootstrap
t_assert_eq 'no arguments default to interactive' "$(cat "$argv_log")" 'interactive'

run_bootstrap --skill planning --target "$work/target"
t_assert_eq 'given arguments pass through untouched' \
    "$(cat "$argv_log")" "$(printf '%s\n' --skill planning --target "$work/target")"

t_end
