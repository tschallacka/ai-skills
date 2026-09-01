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
    echo "    - example (Claude Code settings.json permissions.allow):" >&2
    echo "        Read($plans/**), Edit($plans/**), Bash($scripts/**:*), Bash(bash $scripts/**:*)" >&2
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

