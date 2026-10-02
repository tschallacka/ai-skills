#!/usr/bin/env bash
# MODE: DEV
# test-tui-hint-matching.sh -- the profile-matching logic shared by
# hooks/pre-tool-use.sh (Claude Code) and, ported line-for-line into
# JavaScript, opencode/tui-hint-plugin.js. Only the bash side is exercised
# here; the JS port has no test runner wired into this repo's suites, so it
# is verified by hand against these same cases when it changes.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
plugin_dir="$(cd "$tests_dir/.." && pwd)"
repo_root="$(cd "$plugin_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

# shellcheck source=tui-hint-plugin/hooks/lib.sh
source "$plugin_dir/hooks/lib.sh"

vendor_dir="$repo_root/interactive-shell/appprofiles"

match() { # <dir> <command> <require_marker>
    local stripped
    stripped="$(tui_hint_stripped_command "$2")"
    tui_hint_match_profile "$1" "$stripped" "$3" 2>/dev/null || printf ''
}

t_assert_eq 'bare mc matches by filename' \
    "$(match "$vendor_dir" 'mc' 0)" 'mc'
t_assert_eq 'a sudo prefix is stripped before matching' \
    "$(match "$vendor_dir" 'sudo mc /root' 0)" 'mc'
t_assert_eq 'a leading VAR=value assignment is stripped before matching' \
    "$(match "$vendor_dir" 'FOO=bar mc' 0)" 'mc'
t_assert_eq 'an unrelated command has no match' \
    "$(match "$vendor_dir" 'ls -la' 0)" ''
t_assert_eq 'git status has no profile' \
    "$(match "$vendor_dir" 'git status' 0)" ''
t_assert_eq 'git add without -p/-i has no profile' \
    "$(match "$vendor_dir" 'git add .' 0)" ''
t_assert_eq 'git add -p matches its declared Invocation pattern' \
    "$(match "$vendor_dir" 'git add -p' 0)" 'git-add-patch'
t_assert_eq 'git add --patch matches its declared Invocation pattern' \
    "$(match "$vendor_dir" 'git add --patch' 0)" 'git-add-patch'
t_assert_eq 'git bisect matches its declared Invocation pattern' \
    "$(match "$vendor_dir" 'git bisect start' 0)" 'git-bisect'
t_assert_eq 'git subtree matches its declared Invocation pattern' \
    "$(match "$vendor_dir" 'git subtree add --prefix=x y z' 0)" 'git-subtree'
t_assert_eq 'a sudo prefix is stripped before an Invocation pattern is tested' \
    "$(match "$vendor_dir" 'sudo git bisect start' 0)" 'git-bisect'

work="$(mktemp -d "${TMPDIR:-/tmp}/tui-hint-matching.XXXXXX")"
trap 'rm -rf "$work"' EXIT
printf '<!-- tui-app-profile: v1 -->\n# marked-tool\n' >"$work/marked-tool.md"
printf '# unmarked-tool\nno marker on the first line\n' >"$work/unmarked-tool.md"

t_assert_eq 'a marked file in an agent-writable dir is trusted' \
    "$(match "$work" 'marked-tool --x' 1)" 'marked-tool'
t_assert_eq 'an unmarked file in an agent-writable dir is never trusted' \
    "$(match "$work" 'unmarked-tool --x' 1)" ''
t_assert_eq 'require_marker=0 trusts the directory itself, marker or not' \
    "$(match "$work" 'unmarked-tool --x' 0)" 'unmarked-tool'

# B319: tui_hint_rjq_bin resolves the installer's own shared copy first,
# never the user's global PATH -- installed at
# ${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/bin/rjq by install_shared_rjq
# specifically so a hook running independently of install.sh, potentially
# long after, does not depend on an ambient `rjq` neither ai-text-editor
# nor interactive-shell (the skills this plugin rides with) ever promised.
resolver_work="$(mktemp -d "${TMPDIR:-/tmp}/tui-hint-rjq-resolve.XXXXXX")"
trap 'rm -rf "$resolver_work"' EXIT
export XDG_CONFIG_HOME="$resolver_work/config"
mkdir -p "$XDG_CONFIG_HOME/tsch-ai-skills/bin"
printf '#!/bin/sh\necho stub\n' > "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"
chmod +x "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"

t_assert_eq 'tui_hint_rjq_bin resolves the installed shared copy' \
    "$(tui_hint_rjq_bin)" "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"

rm -f "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"
t_assert_eq 'tui_hint_rjq_bin falls back to an ambient rjq on PATH' \
    "$(tui_hint_rjq_bin)" "$(command -v rjq)"

t_end
