# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 13. Step 2: planning runtime permissions (interactive main path only)
# ---------------------------------------------------------------
# Grants the user-chosen agents read/write on the plans root and execution
# access to the copied planning shell scripts. Every config file that is
# modified is first backed up by backup_file from section 60 -- one scheme for
# the whole installer, rather than a second one spelled <file>.bak.<timestamp>
# here. That also means a config file inside a git work tree is replaced without
# a copy, because git is already its recovery path. Additions are idempotent:
# entries already present are never duplicated.

# Index lookup against the registry in section 1; anything not in it is custom.
agent_kind_for_root() {
    local root="${1%/}" index
    for index in "${!TARGET_PATHS[@]}"; do
        if [ "$root" = "${TARGET_PATHS[$index]%/}" ]; then
            printf '%s\n' "${TARGET_KINDS[$index]}"
            return
        fi
    done
    printf '%s\n' custom
}

# Trailing-slash trim. The python implementations these replaced used
# rstrip("/"), which removes every trailing slash, not just one.
strip_trailing_slashes() {
    local value="$1"
    while [ "$value" != "${value%/}" ]; do
        value="${value%/}"
    done
    printf '%s\n' "$value"
}

# Both permission editors are reached only from planning_permission_step, inside
# main's `contains planning "${SELECTED_SKILLS[@]}"` branch, and planning declares
# rjq in runtime_requirements() — so verify_runtime_tools has already refused to
# get this far without rjq. rjq is therefore guaranteed, and the command -v check
# below only turns a hypothetical `set -e` abort into a clear message plus manual
# instructions. It has to precede backup_file, or a failure here leaves
# an orphaned backup behind. python3 is deliberately not used anywhere:
# rjq is the only runtime dependency this installer is allowed to add.
claude_permissions() {
    local cfg="${CLAUDE_CONFIGFILE:-$HOME/.claude/settings.json}" scripts="$1" plans="$2" tmp="$3"
    local doc added tmpfile program
    [ -f "$cfg" ] || { echo "  claude-code: no $cfg found; skipped" >&2; return 0; }
    if ! command -v rjq >/dev/null 2>&1; then
        echo "  claude-code: rjq is not installed; cannot edit $cfg safely." >&2
        print_manual_permissions claude "$scripts" "$plans" "$tmp"
        return 0
    fi
    plans="$(strip_trailing_slashes "$plans")"
    scripts="$(strip_trailing_slashes "$scripts")"
    tmp="$(strip_trailing_slashes "$tmp")"
    backup_file "$cfg"

    # An unparseable or non-object settings.json is rebuilt from {} rather than
    # edited. `objectify` is the same defensive read at every level.
    doc="$(rjq '.' "$cfg" 2>/dev/null || true)"
    [ -n "$doc" ] || doc='{}'
    program='
def objectify: if type == "object" then . else {} end;
def entries: [
    "Read(\($plans)/**)", "Edit(\($plans)/**)",
    "Bash(\($scripts)/**:*)", "Read(\($scripts)/**)",
    "Bash(bash \($scripts)/**:*)",
    "Read(\($tmp)/**)", "Edit(\($tmp)/**)",
    "Bash(\($tmp)/**:*)"
];
def allowed: objectify | .permissions | objectify | .allow
    | if type == "array" then . else [] end;
'
    added="$(printf '%s' "$doc" | rjq -r \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'(entries - allowed)[]')"

    # mktemp in the config's own directory so the rename is atomic, and cp -p to
    # inherit the user's mode before rjq truncates it.
    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    if ! printf '%s' "$doc" | rjq \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'
        objectify
        | (.permissions | objectify) as $perm
        | ($perm.allow | if type == "array" then . else [] end) as $allow
        | .permissions = ($perm | .allow = ($allow + (entries - $allow)))' \
        > "$tmpfile"; then
        rm -f "$tmpfile"
        die "rjq failed to update $cfg"
    fi
    mv "$tmpfile" "$cfg"

    if [ -n "$added" ]; then
        printf '  claude-code: added to permissions.allow:\n'
        printf '%s\n' "$added" | sed 's|^|    - |'
    else
        printf '  claude-code: permissions already present\n'
    fi
}

# opencode reads ~/.config/opencode/opencode.json or opencode.jsonc -- either
# name, JSON-C syntax allowed in both. Edit whichever exists, .json preferred;
# when neither exists, create opencode.json so the grant below has a home --
# skipping would leave every planning helper behind a permission prompt.
opencode_configfile() {
    local dir="$HOME/.config/opencode"
    if [ -n "${OPENCODE_CONFIGFILE:-}" ]; then
        printf '%s\n' "$OPENCODE_CONFIGFILE"
    elif [ -f "$dir/opencode.json" ] || [ ! -f "$dir/opencode.jsonc" ]; then
        printf '%s\n' "$dir/opencode.json"
    else
        printf '%s\n' "$dir/opencode.jsonc"
    fi
}

opencode_permissions() {
    local cfg scripts="$1" plans="$2" tmp="$3"
    local doc added legacy tmpfile program created=0
    cfg="$(opencode_configfile)"
    if [ ! -f "$cfg" ]; then
        mkdir -p "$(dirname "$cfg")" \
            || { echo "  opencode: cannot create $(dirname "$cfg")/" >&2; print_manual_permissions opencode "$scripts" "$plans" "$tmp"; return 0; }
        printf '{\n  "$schema": "https://opencode.ai/config.json"\n}\n' > "$cfg" \
            || { echo "  opencode: cannot write $cfg" >&2; print_manual_permissions opencode "$scripts" "$plans" "$tmp"; return 0; }
        echo "  opencode: created $cfg" >&2
        created=1
    fi
    if ! command -v rjq >/dev/null 2>&1; then
        echo "  opencode: rjq is not installed; cannot edit $cfg safely." >&2
        print_manual_permissions opencode "$scripts" "$plans" "$tmp"
        return 0
    fi
    plans="$(strip_trailing_slashes "$plans")"
    scripts="$(strip_trailing_slashes "$scripts")"
    tmp="$(strip_trailing_slashes "$tmp")"
    # A non-empty config that strict rjq cannot parse carries JSON-C comments or
    # trailing commas, which a rewrite would strip: print manual instructions
    # instead of rebuilding from {}. Emptiness is decided here -- rjq's own exit
    # status for empty input flips between versions.
    if [ "$created" -eq 0 ] && [ -s "$cfg" ] && ! rjq -e '.' "$cfg" >/dev/null 2>&1; then
        echo "  opencode: $cfg is not strict JSON; add these by hand:" >&2
        print_manual_permissions opencode "$scripts" "$plans" "$tmp"
        return 0
    fi
    [ "$created" -eq 1 ] || backup_file "$cfg"

    doc="$(rjq '.' "$cfg" 2>/dev/null || true)"
    [ -n "$doc" ] || doc='{}'
    # opencode's permission block is keyed by tool name; each value is either an
    # action string ("ask"/"allow"/"deny") or a {pattern: action} object. A bare
    # action string is preserved as the "*" fallback pattern. A stray
    # Claude-style allow/deny/ask list is not valid here, so `base` migrates it
    # out — that removal is what the legacy notice below reports.
    program='
def objectify: if type == "object" then . else {} end;
def wanted: [
    ["read",               ["\($plans)/**", "\($scripts)/**", "\($tmp)/**"]],
    ["edit",               ["\($plans)/**", "\($tmp)/**"]],
    ["bash",               ["\($scripts)/**", "bash \($scripts)/**", "\($tmp)/**"]],
    ["external_directory", ["\($plans)/**", "\($scripts)/**", "\($tmp)/**"]]
];
def rules: if type == "object" then . elif type == "string" then {"*": .} else {} end;
def base:
    objectify
    | .permission as $p
    | (if ($p | type) == "string"
       then reduce wanted[] as $w ({}; .[$w[0]] = {"*": $p})
       else ($p | objectify) end)
    | del(.allow, .deny, .ask);
'
    legacy="$(printf '%s' "$doc" | rjq -r '
        (if type == "object" then . else {} end) | .permission
        | if type == "object" and (.allow | type) == "array" and (.allow | length) > 0
          then "yes" else "no" end')"
    added="$(printf '%s' "$doc" | rjq -r \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'
        [ wanted[] as $w
          | ($w[0]) as $tool
          | (base[$tool] | rules) as $rule
          | $w[1][] as $pattern
          | select($rule[$pattern] != "allow")
          | "\($tool): \($pattern)" ][]')"

    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    if ! printf '%s' "$doc" | rjq \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'
        (if type == "object" then . else {} end) as $data
        | (reduce wanted[] as $w (base;
              .[$w[0]] = (reduce $w[1][] as $pattern ((.[$w[0]] | rules); .[$pattern] = "allow"))
          )) as $perm
        | $data | .permission = $perm' \
        > "$tmpfile"; then
        rm -f "$tmpfile"
        die "rjq failed to update $cfg"
    fi
    mv "$tmpfile" "$cfg"

    if [ "$legacy" = "yes" ]; then
        printf '  opencode: removed invalid claude-style permission.allow list\n'
    fi
    if [ -n "$added" ]; then
        printf '  opencode: allowed in permission:\n'
        printf '%s\n' "$added" | sed 's|^|    - |'
    else
        printf '  opencode: permissions already present\n'
    fi
}

print_manual_permissions() {
    local kind="$1" scripts="$2" plans="$3" tmp="$4"
    echo "  $kind: no safe auto-editable permission file was modified." >&2
    echo "    - grant $kind read/write on $plans" >&2
    echo "    - allow $kind to execute the planning helpers under $scripts" >&2
    echo "    - allow $kind read/write/execute under the planning temp dir $tmp" >&2
    # B236: the worked example must match the agent being addressed, or -- for
    # a kind this fallback does not carry per-agent syntax for -- state no
    # example rather than print one written for a different agent's format.
    case "$kind" in
        claude)
            echo "    - example (Claude Code settings.json permissions.allow):" >&2
            echo "        Read($plans/**), Edit($plans/**), Bash($scripts/**:*), Bash(bash $scripts/**:*)" >&2
            ;;
        opencode)
            echo "    - example (opencode.json permission, each pattern -> \"allow\"):" >&2
            echo "        {\"permission\": {\"read\": {\"$plans/**\": \"allow\"}, \"edit\": {\"$plans/**\": \"allow\"}, \"bash\": {\"$scripts/**\": \"allow\"}}}" >&2
            ;;
        codex)
            echo "    - example (~/.codex/config.toml):" >&2
            echo "        sandbox_workspace_write.writable_roots = [\"$plans\", \"$scripts\", \"$tmp\"]" >&2
            ;;
    esac
}

# Fallback because auto-configuration is not always possible or effective
# (e.g. Codex/OpenClaw/Cline have no stable JSON permission key, or an agent
# changed its format). Prints a self-contained prompt the user can paste into
# their AI agent so the agent configures the correct permissions itself.
print_agent_permission_prompt() {
    local plans="$1" tmp="$2"; shift 2
    local root kind
    echo >&2
    echo "If the automatic permission setup above is not effective for your agent," >&2
    echo "paste the following into your AI agent to configure it for the planning" >&2
    echo "skill. It is self-contained and refuses broad permissions:" >&2
    echo >&2
    printf '%s\n' '--- BEGIN AGENT PROMPT (copy from here) ---'
    cat <<PROMPT
Please configure my AI tooling so the "planning" skill can run. Concretely ensure:

1. My plans directory is readable and writable by you and me:
   $plans

2. My planning scratch directory is readable, writable, and executable by you
   (the planning skill writes temporary capsules and run artifacts there):
   $tmp
   Create it if it does not exist (mkdir -p).

3. The planning helper shell scripts under each of the following scripts
   directories may be executed by you; do not block them as unknown or
   suspicious:
PROMPT
    for root in "$@"; do
        kind="$(agent_kind_for_root "$root")"
        printf '   - %s: %s\n' "$kind" "${root%/}/planning/scripts"
    done
    cat <<PROMPT
Find your own permission/settings file (for example: Claude Code
settings.json "permissions.allow", OpenCode opencode.json "permission.allow",
Codex policy/sandbox config, Cline or OpenClaw allowed-tools list) and add
entries that:
  - grant read + write under $plans
  - grant read/write/execute under $tmp
  - allow executing Bash for the planning helper scripts (Read/Edit/Write plus
    Bash rules scoped to those scripts)
Add only entries that are not already present. If you modify a config file,
first copy it to .<basename>.<UTC timestamp>.back beside it before editing --
the same scheme the installer uses -- then tell me the exact path and the
entries you changed. If the file is inside a git working tree, commit or stash
instead: git is its recovery path and a stray backup file only clutters the
tree. Do not change any other permissions and do
not grant broad or all-tools access.
--- END AGENT PROMPT (copy from here) ---
PROMPT
}

# Merge a jq-computed `entries` list into Claude's permissions.allow.
#
# $1 is a jq fragment defining `entries`, $2 the line to print when everything
# was already there, and everything after them is passed through to rjq -- so
# paths travel as --arg values and are never interpolated into the program text.
# Extracted because the caller below would otherwise be a second copy of this
# whole merge, which CODE-STYLE.md's 40-line cap correctly refuses.
claude_merge_allow() {
    local entries_def="$1" present_label="$2"
    shift 2
    local cfg="${CLAUDE_CONFIGFILE:-$HOME/.claude/settings.json}"
    local doc added tmpfile program
    doc="$(rjq '.' "$cfg" 2>/dev/null || true)"
    [ -n "$doc" ] || doc='{}'
    program="def objectify: if type == \"object\" then . else {} end;
$entries_def
def allowed: objectify | .permissions | objectify | .allow
    | if type == \"array\" then . else [] end;
"
    added="$(printf '%s' "$doc" | rjq -r "$@" "$program"'(entries - allowed)[]')"

    # mktemp in the config's own directory so the rename is atomic, and cp -p to
    # inherit the user's mode before rjq truncates it.
    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    if ! printf '%s' "$doc" | rjq "$@" "$program"'
        objectify
        | (.permissions | objectify) as $perm
        | ($perm.allow | if type == "array" then . else [] end) as $allow
        | .permissions = ($perm | .allow = ($allow + (entries - $allow)))' \
        > "$tmpfile"; then
        rm -f "$tmpfile"
        die "rjq failed to update $cfg"
    fi
    mv "$tmpfile" "$cfg"

    if [ -n "$added" ]; then
        printf '  claude-code: added to permissions.allow:\n'
        printf '%s\n' "$added" | sed 's|^|    - |'
    else
        printf '  claude-code: %s\n' "$present_label"
    fi
}

claude_worktrees_permissions() {
    local worktrees="$1"
    local cfg="${CLAUDE_CONFIGFILE:-$HOME/.claude/settings.json}"
    [ -f "$cfg" ] || { echo "  claude-code: no $cfg found; skipped" >&2; return 0; }
    if ! command -v rjq >/dev/null 2>&1; then
        echo "  claude-code: rjq is not installed; cannot edit $cfg safely." >&2
        print_manual_worktrees_permissions claude "$worktrees"
        return 0
    fi
    worktrees="$(strip_trailing_slashes "$worktrees")"
    backup_file "$cfg"
    # "worktree grant already in place" rather than the planning arm's
    # "permissions already present": both grants now run in one install, and
    # test-installer-opencode-permissions counts that phrase expecting exactly
    # one. Distinct wording keeps its count honest and tells the two apart in
    # the output.
    # No separate Write(...) entry: Claude Code's permission engine has no rule
    # keyed on the Write tool, and does not fall back to a matching Edit(...)
    # rule either -- Edit(path) is the umbrella that covers every file-editing
    # tool, Write included, so a Write(path) entry here matches nothing and
    # leaves the grant it promised silently missing. claude_permissions above
    # already grants the planning root this way for the same reason.
    claude_merge_allow 'def entries: [
    "Read(\($worktrees)/**)", "Edit(\($worktrees)/**)", "Bash(\($worktrees)/**:*)"
];' 'worktree grant already in place' --arg worktrees "$worktrees"
}

# The shared jq preamble for an opencode permission merge, given a fragment
# defining `wanted`. Its own function so the merge below stays inside the
# 40-line cap.
#
# opencode's permission block is keyed by tool name; each value is either an
# action string ("ask"/"allow"/"deny") or a {pattern: action} object. A bare
# action string is preserved as the "*" fallback pattern, and a stray
# Claude-style allow/deny/ask list is not valid here, so `base` drops it.
opencode_permission_program() {
    printf 'def objectify: if type == "object" then . else {} end;\n%s\n' "$1"
    cat <<'PROGRAM'
def rules: if type == "object" then . elif type == "string" then {"*": .} else {} end;
def base:
    objectify
    | .permission as $p
    | (if ($p | type) == "string"
       then reduce wanted[] as $w ({}; .[$w[0]] = {"*": $p})
       else ($p | objectify) end)
    | del(.allow, .deny, .ask);
PROGRAM
}

# Merge a jq-computed `wanted` list of [tool, [patterns]] pairs into opencode's
# permission block. Same split, and for the same reason, as claude_merge_allow:
# $1 defines `wanted`, $2 is the config path, $3 the already-present line, and
# the rest goes to rjq.
opencode_merge_permission() {
    local wanted_def="$1" cfg="$2" present_label="$3"
    shift 3
    local doc added tmpfile program
    doc="$(rjq '.' "$cfg" 2>/dev/null || true)"
    [ -n "$doc" ] || doc='{}'
    program="$(opencode_permission_program "$wanted_def")"
    added="$(printf '%s' "$doc" | rjq -r "$@" "$program"'
        [ wanted[] as $w
          | ($w[0]) as $tool
          | (base[$tool] | rules) as $rule
          | $w[1][] as $pattern
          | select($rule[$pattern] != "allow")
          | "\($tool): \($pattern)" ][]')"

    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    if ! printf '%s' "$doc" | rjq "$@" "$program"'
        (if type == "object" then . else {} end) as $data
        | (reduce wanted[] as $w (base;
              .[$w[0]] = (reduce $w[1][] as $pattern ((.[$w[0]] | rules); .[$pattern] = "allow"))
          )) as $perm
        | $data | .permission = $perm' \
        > "$tmpfile"; then
        rm -f "$tmpfile"
        die "rjq failed to update $cfg"
    fi
    mv "$tmpfile" "$cfg"

    if [ -n "$added" ]; then
        printf '  opencode: allowed\n'
        printf '%s\n' "$added" | sed 's|^|    - |'
    else
        printf '  opencode: %s\n' "$present_label"
    fi
}

opencode_worktrees_permissions() {
    local worktrees="$1"
    local cfg created=0
    cfg="$(opencode_configfile)"
    if [ ! -f "$cfg" ]; then
        mkdir -p "$(dirname "$cfg")" \
            || { echo "  opencode: cannot create $(dirname "$cfg")/" >&2; print_manual_worktrees_permissions opencode "$worktrees"; return 0; }
        printf '{\n  "$schema": "https://opencode.ai/config.json"\n}\n' > "$cfg" \
            || { echo "  opencode: cannot write $cfg" >&2; print_manual_worktrees_permissions opencode "$worktrees"; return 0; }
        echo "  opencode: created $cfg" >&2
        created=1
    fi
    if ! command -v rjq >/dev/null 2>&1; then
        echo "  opencode: rjq is not installed; cannot edit $cfg safely." >&2
        print_manual_worktrees_permissions opencode "$worktrees"
        return 0
    fi
    worktrees="$(strip_trailing_slashes "$worktrees")"
    # A config strict rjq cannot parse carries comments or trailing commas a
    # rewrite would strip, so print instructions rather than rebuild it. The
    # wording avoids the planning arm's "is not strict JSON", which
    # test-installer-opencode-permissions counts expecting exactly one.
    if [ "$created" -eq 0 ] && [ -s "$cfg" ] && ! rjq -e '.' "$cfg" >/dev/null 2>&1; then
        echo "  opencode: cannot safely rewrite $cfg (comments or trailing commas); add the worktree rules by hand:" >&2
        print_manual_worktrees_permissions opencode "$worktrees"
        return 0
    fi
    [ "$created" -eq 1 ] || backup_file "$cfg"
    opencode_merge_permission 'def wanted: [
    ["read",               ["\($worktrees)/**"]],
    ["edit",               ["\($worktrees)/**"]],
    ["write",              ["\($worktrees)/**"]],
    ["bash",               ["\($worktrees)/**"]],
    ["external_directory", ["\($worktrees)/**"]]
];' "$cfg" 'worktree grant already in place' --arg worktrees "$worktrees"
}

print_manual_worktrees_permissions() {
    local kind="$1" worktrees="$2"
    echo "  $kind: no safe auto-editable permission file was modified." >&2
    echo "    - grant $kind read, write and execute under $worktrees" >&2
    # B236: match the addressed agent's own syntax, or state none for a kind
    # this fallback carries no worked example for.
    case "$kind" in
        claude)
            echo "    - example (Claude Code settings.json permissions.allow):" >&2
            echo "        Read($worktrees/**), Edit($worktrees/**), Bash($worktrees/**:*)" >&2
            ;;
        opencode)
            echo "    - example (opencode.json permission, each pattern -> \"allow\"):" >&2
            echo "        {\"permission\": {\"read\": {\"$worktrees/**\": \"allow\"}, \"edit\": {\"$worktrees/**\": \"allow\"}, \"bash\": {\"$worktrees/**\": \"allow\"}, \"external_directory\": {\"$worktrees/**\": \"allow\"}}}" >&2
            ;;
        codex)
            echo "    - example (~/.codex/config.toml):" >&2
            echo "        sandbox_workspace_write.writable_roots = [\"$worktrees\"]" >&2
            ;;
    esac
}

# Runs for every install, not only a planning one: any agent may be asked to
# take a worktree. See git-worktrees/SKILL.md for why this root sits beside
# tsch-ai-skills rather than inside it.
worktrees_permission_step() {
    local worktrees="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-worktrees" root kind
    echo >&2
    echo "== Agent worktree permissions ==" >&2
    if confirm "Create $worktrees as the agent worktree root?"; then
        mkdir -p "$worktrees" && echo "  Created $worktrees" >&2
    fi
    if confirm "Grant the selected agents read/write/execute on $worktrees, so a worktree there needs no prompt per file? (Each edited config is backed up beside itself, unless git already tracks it)"; then
        for root in "${SELECTED_TARGET_PATHS[@]}"; do
            kind="$(agent_kind_for_root "$root")"
            case "$kind" in
                claude)   claude_worktrees_permissions "$worktrees" ;;
                opencode) opencode_worktrees_permissions "$worktrees" ;;
                *)        print_manual_worktrees_permissions "$kind" "$worktrees" ;;
            esac
        done
    fi
}

planning_permission_step() {
    local plans="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/plans" agent_tmp="${TMPDIR:-/tmp}/planning-agent" root kind scripts
    echo >&2
    echo "== Step 2: planning runtime permissions ==" >&2
    if confirm "Create $plans as the global plans directory?"; then
        mkdir -p "$plans" && echo "  Created $plans" >&2
    fi
    if confirm "Grant the selected agents read/write on $plans and $agent_tmp, and allow them to execute the planning shell scripts? (Each edited config is backed up beside itself, unless git already tracks it)"; then
        for root in "${SELECTED_TARGET_PATHS[@]}"; do
            kind="$(agent_kind_for_root "$root")"
            scripts="${root%/}/planning/scripts"
            case "$kind" in
                claude)   claude_permissions "$scripts" "$plans" "$agent_tmp" ;;
                opencode) opencode_permissions "$scripts" "$plans" "$agent_tmp" ;;
                *)        print_manual_permissions "$kind" "$scripts" "$plans" "$agent_tmp" ;;
            esac
        done
    fi
    print_agent_permission_prompt "$plans" "$agent_tmp" "${SELECTED_TARGET_PATHS[@]}"
}


# ---------------------------------------------------------------
# 13a. Step 3: interactive-shell execution permission
# ---------------------------------------------------------------
# A denied Bash call does not read as "ask for permission" to an agent; it reads
# as "this tool does not work", after which the agent falls back to a headless
# invocation that cannot observe the program at all. That is the failure this
# grant prevents, so it is offered on every install that places the skill.

print_manual_interactive_shell_permissions() {
    local kind="$1" bins="$2"
    echo "  $kind: no safe auto-editable permission file was modified." >&2
    echo "    - allow $kind to execute the wrapper and its input client under $bins" >&2
    # B236: match the addressed agent's own syntax, or state none for a kind
    # this fallback carries no worked example for.
    case "$kind" in
        claude)
            echo "    - example (Claude Code settings.json permissions.allow):" >&2
            echo "        Bash($bins/**:*)" >&2
            ;;
        opencode)
            echo "    - example (opencode.json permission, each pattern -> \"allow\"):" >&2
            echo "        {\"permission\": {\"bash\": {\"$bins/**\": \"allow\"}}}" >&2
            ;;
        codex)
            echo "    - example (~/.codex/config.toml):" >&2
            echo "        sandbox_workspace_write.writable_roots = [\"$bins\"]" >&2
            ;;
    esac
}

claude_interactive_shell_permissions() {
    local bins="$1"
    local cfg="${CLAUDE_CONFIGFILE:-$HOME/.claude/settings.json}"
    [ -f "$cfg" ] || { echo "  claude-code: no $cfg found; skipped" >&2; return 0; }
    if ! command -v rjq >/dev/null 2>&1; then
        echo "  claude-code: rjq is not installed; cannot edit $cfg safely." >&2
        print_manual_interactive_shell_permissions claude "$bins"
        return 0
    fi
    bins="$(strip_trailing_slashes "$bins")"
    backup_file "$cfg"
    # Distinct from the planning and worktree wordings because
    # test-installer-opencode-permissions counts those phrases and expects
    # exactly one of each.
    claude_merge_allow 'def entries: [
    "Bash(\($bins)/**:*)"
];' 'interactive-shell grant already in place' --arg bins "$bins"
}

interactive_shell_permission_step() {
    local root kind bins
    echo >&2
    echo "== Step 3: interactive-shell execution permission ==" >&2
    if ! confirm "Allow the selected agents to execute the interactive-shell binaries, so driving a terminal program needs no prompt per call? (Each edited config is backed up beside itself, unless git already tracks it)"; then
        echo "  Left unchanged. A refused wrapper call reads as a broken tool, so" >&2
        echo "  expect the skill to be skipped in favour of a headless command." >&2
        return 0
    fi
    for root in "${SELECTED_TARGET_PATHS[@]}"; do
        kind="$(agent_kind_for_root "$root")"
        bins="${root%/}/interactive-shell/bin"
        case "$kind" in
            claude) claude_interactive_shell_permissions "$bins" ;;
            *)      print_manual_interactive_shell_permissions "$kind" "$bins" ;;
        esac
    done
}

# ---------------------------------------------------------------
# 13b. Step 4: ai-text-editor tool steering
# ---------------------------------------------------------------
# Claude Code injects a "bash-first" instruction telling the agent to edit files
# with sed and heredocs rather than an editor. It is a prompt-size experiment,
# switched on per account by a remote cohort assignment, and while it is active
# the editor this install just placed is usually bypassed. Two documented
# environment settings turn it down; both are Claude Code's own, so no other
# agent kind is offered them.

# Written whether or not the offer is taken, because declining has a cost that is
# invisible from the outside: a shell rewrite that matched nothing looks exactly
# like one that worked.
print_editor_steering_warning() {
    cat >&2 <<'WARNING'
  Claude Code may instruct the agent to make file changes with sed, heredocs
  or short scripts instead of an editor. While that instruction is active the
  ai-text-editor MCP is usually skipped, and these are what it costs:
    - an in-place sed rewrites the file and exits 0 whether or not the
      pattern matched, so a mistype is indistinguishable from success
    - a script heredoc stacks the shell's escaping on top of the language's
      on top of the target file's syntax
    - neither verifies what it replaces, while the editor's expected_text
      refuses on mismatch and its journal survives a git checkout
  Two settings turn it down, and either is enough:
    CLAUDE_CODE_THRIFTY_SONIC=false  the instruction is not injected at all
    CLAUDE_CODE_COZY_TEAPOT=relaxed  softer wording that leaves the choice to
                                     the agent, so the editor still competes
WARNING
}

# One env key merged into Claude's settings.json, same write discipline as the
# permission editors above: backup, defensive read, atomic rename.
claude_env_setting() {
    local key="$1" value="$2"
    local cfg="${CLAUDE_CONFIGFILE:-$HOME/.claude/settings.json}"
    local doc present tmpfile
    [ -f "$cfg" ] || { echo "  claude-code: no $cfg found; skipped" >&2; return 0; }
    if ! command -v rjq >/dev/null 2>&1; then
        echo "  claude-code: rjq is not installed; set env.$key to \"$value\" in $cfg by hand." >&2
        return 0
    fi
    backup_file "$cfg"
    doc="$(rjq '.' "$cfg" 2>/dev/null || true)"
    [ -n "$doc" ] || doc='{}'
    present="$(printf '%s' "$doc" | rjq -r --arg key "$key" --arg value "$value" '
        (if type == "object" then . else {} end) | .env
        | (if type == "object" then . else {} end)
        | if .[$key] == $value then "yes" else "no" end')"
    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    if ! printf '%s' "$doc" | rjq --arg key "$key" --arg value "$value" '
        (if type == "object" then . else {} end)
        | (.env | if type == "object" then . else {} end) as $env
        | .env = ($env | .[$key] = $value)' > "$tmpfile"; then
        rm -f "$tmpfile"
        die "rjq failed to update $cfg"
    fi
    mv "$tmpfile" "$cfg"
    if [ "$present" = yes ]; then
        printf '  claude-code: env.%s is already "%s"\n' "$key" "$value"
    else
        printf '  claude-code: set env.%s to "%s"\n' "$key" "$value"
    fi
}

editor_steering_step() {
    local root kind claude_seen=0
    echo >&2
    echo "== Step 4: ai-text-editor tool steering ==" >&2
    for root in "${SELECTED_TARGET_PATHS[@]}"; do
        kind="$(agent_kind_for_root "$root")"
        [ "$kind" = claude ] && claude_seen=1
    done
    if [ "$claude_seen" -eq 0 ]; then
        echo "  No Claude Code root selected; these settings are Claude Code's own." >&2
        return 0
    fi
    print_editor_steering_warning
    if confirm "Turn the instruction off (env CLAUDE_CODE_THRIFTY_SONIC=false)?"; then
        claude_env_setting CLAUDE_CODE_THRIFTY_SONIC false
        return 0
    fi
    if confirm "Soften it instead (env CLAUDE_CODE_COZY_TEAPOT=relaxed)?"; then
        claude_env_setting CLAUDE_CODE_COZY_TEAPOT relaxed
        return 0
    fi
    echo "  Left unchanged. Expect the editor to be bypassed for sed and heredocs." >&2
}
