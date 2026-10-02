#!/usr/bin/env bash
# MODE: PROD
# PreToolUse hook: when a Bash command's target program has a shipped
# interactive-shell app profile (${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/
# appprofiles/<name>.md) or an agent-written one (appprofiles.d/<name>.md,
# marker-gated -- see lib.sh and FORMAT.md), remind the agent it can be
# driven more effectively through the interactive-shell skill than a plain
# headless call -- the same reason interactive-shell/SKILL.md itself gives
# for preferring a real terminal over a headless probe. Non-blocking: this
# only annotates the call with additionalContext (surfaced to the model, not
# just the user transcript), it never denies or modifies the tool call
# itself.
#
# Which profile a command line invokes is read from each profile's own
# ### Invocation patterns (interactive-shell/appprofiles/FORMAT.md), not
# hardcoded here -- a profile whose filename is not simply its leading word
# (a git subcommand, e.g.) declares its own matching patterns and this hook
# picks them up with no code change.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tui-hint-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

rjq_bin="$(tui_hint_rjq_bin)" || { printf '{}'; exit 0; }
payload="$(cat)"
tool_name="$(printf '%s' "$payload" | "$rjq_bin" -r '.tool_name // empty')"
command_line="$(printf '%s' "$payload" | "$rjq_bin" -r '.tool_input.command // empty')"

if [ "$tool_name" != "Bash" ] || [ -z "$command_line" ]; then
    printf '{}'
    exit 0
fi

stripped="$(tui_hint_stripped_command "$command_line")"
[ -n "$stripped" ] || { printf '{}'; exit 0; }

program='' profile=''
config_root="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills"
vendor_dir="$config_root/appprofiles"
if program="$(tui_hint_match_profile "$vendor_dir" "$stripped" 0)"; then
    profile="$vendor_dir/$program.md"
fi

# appprofiles.d/, the vendor directory's agent-writable sibling
# (interactive-shell/SKILL.md, FORMAT.md): an agent's own note for an app
# with no vendor profile, or an extension to one. Marker-gated
# (tui_hint_profile_has_marker): unlike appprofiles/, this directory is not
# itself a trust boundary -- anything can drop a .md file there -- so a file
# must carry the literal marker line before this hook will name it.
if [ -z "$profile" ]; then
    agent_dir="$config_root/appprofiles.d"
    if program="$(tui_hint_match_profile "$agent_dir" "$stripped" 1)"; then
        profile="$agent_dir/$program.md"
    fi
fi

[ -n "$profile" ] || { printf '{}'; exit 0; }

"$rjq_bin" -n -c --arg program "$program" --arg profile "$profile" '
{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "allow",
    additionalContext: (
      "\($program) has a shipped interactive-shell app profile at \($profile) -- "
      + "screen layout, keybindings, dialogs, and known quirks. A plain Bash call "
      + "cannot observe its screen or send it real keystrokes; consider driving it "
      + "through the interactive-shell skill instead, especially for anything beyond "
      + "a one-shot non-interactive invocation."
    )
  }
}'
