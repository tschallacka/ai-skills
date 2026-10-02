#!/usr/bin/env bash
# MODE: DEV
# test-editor-gate-matching.sh -- the pattern-matching and token mint/consume
# logic shared by pre-tool-use-bash.sh and hooks/editor-token.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
plugin_dir="$(cd "$tests_dir/.." && pwd)"
repo_root="$(cd "$plugin_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

# shellcheck source=editor-gate-plugin/hooks/lib.sh
source "$plugin_dir/hooks/lib.sh"

SED_I="s""e""d -i"

matches() { # <command>
    if editor_gate_matches "$1"; then printf yes; else printf no; fi
}

t_assert_eq 'plain command does not match' "$(matches 'ls -la')" 'no'
t_assert_eq 'sed -i matches' "$(matches "$SED_I s/a/b/ file")" 'yes'
t_assert_eq 'sudo sed -i matches' "$(matches "sudo $SED_I s/a/b/ file")" 'yes'
t_assert_eq 'perl -i matches' "$(matches 'perl -i -pe s/a/b/ file')" 'yes'
t_assert_eq 'a piped grep+sed -i matches' \
    "$(matches "grep -rl foo dir | xargs $SED_I s/foo/bar/")" 'yes'
t_assert_eq 'a heredoc redirected to a file matches' \
    "$(matches $'cat > out.txt <<EOF\nhello\nEOF')" 'yes'
t_assert_eq 'a heredoc redirected to /dev/null does not match' \
    "$(matches $'cat > /dev/null <<EOF\nhello\nEOF')" 'no'
t_assert_eq 'a heredoc with no redirect at all does not match' \
    "$(matches $'cat <<EOF\nhello\nEOF')" 'no'
t_assert_eq 'a python heredoc body opening a file for write matches' \
    "$(matches $'python3 <<PY\nopen("f", "w").write("x")\nPY')" 'yes'

work="$(mktemp -d "${TMPDIR:-/tmp}/editor-gate-test.XXXXXX")"
export XDG_STATE_HOME="$work"
trap 'rm -rf "$work"' EXIT

command="$SED_I s/foo/bar/ file.txt"
mint_out="$(editor_gate_mint --why 'a real reason, not a placeholder' --command "$command")"
token="$(printf '%s\n' "$mint_out" | grep -o 'EDIT_OK=[0-9a-f]\{32\}' | cut -d= -f2)"
t_assert_eq 'mint prints a 32-hex-char token' "${#token}" '32'

if editor_gate_consume "$token" "$command" >/dev/null; then
    t_assert_eq 'a fresh token consumes for its exact command' yes yes
else
    t_assert_eq 'a fresh token consumes for its exact command' no yes
fi

if editor_gate_consume "$token" "$command" >/dev/null 2>&1; then
    t_assert_eq 'the same token cannot be consumed twice' yes no
else
    t_assert_eq 'the same token cannot be consumed twice' no no
fi

mint_out2="$(editor_gate_mint --why 'a second real reason here' --command "$command")"
token2="$(printf '%s\n' "$mint_out2" | grep -o 'EDIT_OK=[0-9a-f]\{32\}' | cut -d= -f2)"
if editor_gate_consume "$token2" "sed -i s/OTHER/bar/ file.txt" >/dev/null 2>&1; then
    t_assert_eq 'a token does not authorise a different command' yes no
else
    t_assert_eq 'a token does not authorise a different command' no no
fi

if editor_gate_mint --why 'short' --command "$command" >/dev/null 2>&1; then
    t_assert_eq 'a placeholder-length --why is refused' yes no
else
    t_assert_eq 'a placeholder-length --why is refused' no no
fi

# B319: editor_gate_rjq_bin resolves the installer's own shared copy first,
# never the user's global PATH -- installed at
# ${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/bin/rjq by install_shared_rjq
# specifically so this hard gate does not depend on an ambient `rjq`
# neither ai-text-editor nor interactive-shell ever promised. Getting this
# wrong risks failing OPEN on exactly the commands the gate exists to catch.
resolver_work="$(mktemp -d "${TMPDIR:-/tmp}/editor-gate-rjq-resolve.XXXXXX")"
trap 'rm -rf "$resolver_work" "$work"' EXIT
export XDG_CONFIG_HOME="$resolver_work/config"
mkdir -p "$XDG_CONFIG_HOME/tsch-ai-skills/bin"
printf '#!/bin/sh\necho stub\n' > "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"
chmod +x "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"

t_assert_eq 'editor_gate_rjq_bin resolves the installed shared copy' \
    "$(editor_gate_rjq_bin)" "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"

rm -f "$XDG_CONFIG_HOME/tsch-ai-skills/bin/rjq"
t_assert_eq 'editor_gate_rjq_bin falls back to an ambient rjq on PATH' \
    "$(editor_gate_rjq_bin)" "$(command -v rjq)"

t_end
