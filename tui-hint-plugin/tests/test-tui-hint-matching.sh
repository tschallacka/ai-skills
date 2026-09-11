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

# B319: the hook's own JSON in/out has no rjq dependency -- these run with
# rjq deliberately hidden from PATH, so a regression back to calling it
# would fail the same way it did for a real end user with neither
# ai-text-editor nor interactive-shell (the skills this plugin rides with)
# ever declaring rjq as a runtime requirement.
PATH="/usr/bin:/bin"

t_assert_eq 'tui_hint_json_field extracts a plain string value' \
    "$(printf '{"tool_name":"Bash","tool_input":{"command":"mc"}}' | tui_hint_json_field tool_name)" \
    'Bash'
t_assert_eq 'tui_hint_json_field extracts a nested string value by its own key' \
    "$(printf '{"tool_name":"Bash","tool_input":{"command":"mc /root"}}' | tui_hint_json_field command)" \
    'mc /root'
t_assert_eq 'tui_hint_json_field unescapes a quote in the value' \
    "$(printf '{"command":"echo \\"hi\\""}' | tui_hint_json_field command)" \
    'echo "hi"'
t_assert_eq 'tui_hint_json_escape escapes a quote and a backslash' \
    "$(tui_hint_json_escape 'a "quoted" \path')" \
    'a \"quoted\" \\path'
t_assert_eq 'a value round-trips through escape then field-extraction unchanged' \
    "$(printf '{"x":"%s"}' "$(tui_hint_json_escape 'weird "value" with \backslash')" | tui_hint_json_field x)" \
    'weird "value" with \backslash'

t_end
