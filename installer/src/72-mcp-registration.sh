# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 7b. MCP registration
# ---------------------------------------------------------------
# An mcp-mode install copies an adapter binary, and this is the step that makes
# an agent able to call it -- and the step that takes the registration away
# again when the skill is switched back to `skill` mode, because that switch
# deletes the binary the entry points at (BUGS.json B284, B285).
#
# Each agent's own CLI is preferred over editing its configuration, because the
# CLI owns the format: `claude mcp add` and `codex mcp add` are used wherever
# those binaries exist, so codex's config.toml is never written by hand.
# opencode takes a local command after `--`, which its help does not document
# and its handler reads as yargs' populate-`--`; it writes the global config
# even when a project one sits in the working directory. What it has no
# subcommand for is REMOVAL, so that one case edits opencode.json with rjq --
# the tool the permission step already uses on that file.
#
# Removal only ever touches an entry whose command points inside the skill
# directory this install owns. A hand-made entry of the same name pointing
# somewhere else is left alone: the toggle invalidates what the installer put
# there, not what the user did.

# The adapter binary an mcp-mode install of this skill left in place, or empty
# when this skill is not in mcp mode here.
#
# Read from the INSTALLED directory rather than from what the source tree
# declares: the question is whether an adapter is there to be registered, and
# the mode gate has already decided that by copying it or removing it.
mcp_adapter_path() { # <skill> <installed skill dir>
    local skill="$1" dir="$2" path
    for path in "$dir"/bin/*/*; do
        [ -f "$path" ] || continue
        [ "$(integration_binary_mode "$skill" "${path##*/}")" = mcp ] || continue
        printf '%s\n' "$path"
        return 0
    done
    return 0
}

# Does an agent's registration for this name point inside the directory this
# install owns? Only then is it ours to remove.
mcp_entry_is_ours() { # <kind> <name> <skill dir>
    local kind="$1" name="$2" dir="$3" command=''
    case "$kind" in
        claude)
            [ -f "$HOME/.claude.json" ] && command -v rjq >/dev/null 2>&1 || return 1
            command="$(rjq -r --arg n "$name" '.mcpServers[$n].command // ""' \
                "$HOME/.claude.json" 2>/dev/null)"
            ;;
        codex)
            [ -f "${CODEX_HOME:-$HOME/.codex}/config.toml" ] || return 1
            command="$(awk -v want="[mcp_servers.$name]" '
                $0 == want { inside = 1; next }
                inside && /^\[/ { exit }
                inside && $1 == "command" { sub(/^[^=]*= *"?/, ""); sub(/"$/, ""); print; exit }
            ' "${CODEX_HOME:-$HOME/.codex}/config.toml" 2>/dev/null)"
            ;;
        opencode)
            command -v rjq >/dev/null 2>&1 || return 1
            command="$(rjq -r --arg n "$name" '.mcp[$n].command[0] // ""' \
                "$(opencode_configfile)" 2>/dev/null)"
            ;;
    esac
    case "$command" in "$dir"/*) return 0 ;; *) return 1 ;; esac
}

# `claude mcp add` on a name that already exists reports it and exits 0 WITHOUT
# updating the command, so an upgrade that moved the binary would keep the old
# path. Removing first makes the add the step that decides.
mcp_claude_register() { # <name> <path>
    command -v claude >/dev/null 2>&1 || { mcp_print_manual claude "$1" "$2"; return 0; }
    claude mcp remove -s user "$1" >/dev/null 2>&1 || :
    if claude mcp add -s user -t stdio "$1" "$2" >/dev/null 2>&1; then
        echo "  claude: registered MCP server $1" >&2
    else
        mcp_print_manual claude "$1" "$2"
    fi
}

mcp_codex_register() { # <name> <path>
    command -v codex >/dev/null 2>&1 || { mcp_print_manual codex "$1" "$2"; return 0; }
    if codex mcp add "$1" -- "$2" >/dev/null 2>&1; then
        echo "  codex: registered MCP server $1" >&2
    else
        mcp_print_manual codex "$1" "$2"
    fi
}

# stdin is closed because the same subcommand prompts when it is given no
# command, and an installer must never stop on a prompt it did not intend.
mcp_opencode_register() { # <name> <path>
    if command -v opencode >/dev/null 2>&1 \
        && opencode mcp add "$1" -- "$2" </dev/null >/dev/null 2>&1; then
        echo "  opencode: registered MCP server $1" >&2
        return 0
    fi
    mcp_opencode_write "$1" "$2"
}

# The fallback when opencode is not on PATH: its own shape, as it writes it --
# type "local" and an argv array, with no `enabled` key.
mcp_opencode_write() { # <name> <path>
    local cfg tmpfile
    cfg="$(opencode_configfile)"
    if ! command -v rjq >/dev/null 2>&1 || [ ! -f "$cfg" ]; then
        mcp_print_manual opencode "$1" "$2"
        return 0
    fi
    backup_file "$cfg"
    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || { mcp_print_manual opencode "$1" "$2"; return 0; }
    if rjq --arg n "$1" --arg c "$2" \
        '(if type == "object" then . else {} end)
         | .mcp = ((.mcp // {}) | .[$n] = {"type":"local","command":[$c]})' \
        "$cfg" > "$tmpfile" 2>/dev/null; then
        mv "$tmpfile" "$cfg"
        echo "  opencode: registered MCP server $1" >&2
    else
        rm -f "$tmpfile"
        mcp_print_manual opencode "$1" "$2"
    fi
}

mcp_opencode_unregister() { # <name>
    local cfg tmpfile
    cfg="$(opencode_configfile)"
    command -v rjq >/dev/null 2>&1 && [ -f "$cfg" ] || return 0
    backup_file "$cfg"
    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || return 0
    if rjq --arg n "$1" '(if type == "object" then . else {} end) | .mcp |= (. // {} | del(.[$n]))' \
        "$cfg" > "$tmpfile" 2>/dev/null; then
        mv "$tmpfile" "$cfg"
        echo "  opencode: removed MCP server $1" >&2
    else
        rm -f "$tmpfile"
    fi
}

mcp_print_manual() { # <kind> <name> <path>
    echo "  $1: register the MCP server by hand:" >&2
    case "$1" in
        claude)   echo "    claude mcp add -s user -t stdio $2 $3" >&2 ;;
        codex)    echo "    codex mcp add $2 -- $3" >&2 ;;
        opencode) echo "    opencode mcp add $2 -- $3" >&2 ;;
        *)        echo "    run $3 as a stdio MCP server named $2" >&2 ;;
    esac
}

mcp_register_for_kind() { # <kind> <name> <path>
    case "$1" in
        claude)   mcp_claude_register "$2" "$3" ;;
        codex)    mcp_codex_register "$2" "$3" ;;
        opencode) mcp_opencode_register "$2" "$3" ;;
        *)        mcp_print_manual "$1" "$2" "$3" ;;
    esac
}

mcp_unregister_for_kind() { # <kind> <name> <skill dir>
    mcp_entry_is_ours "$1" "$2" "$3" || return 0
    case "$1" in
        claude)   claude mcp remove -s user "$2" >/dev/null 2>&1 \
                      && echo "  claude: removed MCP server $2" >&2 ;;
        codex)    codex mcp remove "$2" >/dev/null 2>&1 \
                      && echo "  codex: removed MCP server $2" >&2 ;;
        opencode) mcp_opencode_unregister "$2" ;;
    esac
    return 0
}

# One skill, one agent root: register when this install put an adapter there,
# and otherwise take away a registration this installer owns.
mcp_registration_for_root() { # <skill> <root>
    local skill="$1" root="$2" dir kind path
    dir="${root%/}/$skill"
    kind="$(agent_kind_for_root "$root")"
    path="$(mcp_adapter_path "$skill" "$dir")"
    if [ -n "$path" ]; then
        mcp_register_for_kind "$kind" "$skill" "$path"
    else
        mcp_unregister_for_kind "$kind" "$skill" "$dir"
    fi
}

# Runs after the install loop, for every selected skill that offers an mcp
# mode. A skill with no integration.tsv declares no mcp binary, so it never
# reaches an agent config.
mcp_registration_step() {
    local skill root announced=0
    for skill in ${SELECTED_SKILLS[@]+"${SELECTED_SKILLS[@]}"}; do
        [ -n "$(integration_modes "$skill")" ] || continue
        if [ "$announced" -eq 0 ]; then
            echo >&2
            echo "== MCP registration ==" >&2
            announced=1
        fi
        for root in ${SELECTED_TARGET_PATHS[@]+"${SELECTED_TARGET_PATHS[@]}"}; do
            mcp_registration_for_root "$skill" "$root"
        done
    done
    return 0
}
