# ---------------------------------------------------------------
# 13. Step 2: planning runtime permissions (interactive main path only)
# ---------------------------------------------------------------
# Grants the user-chosen agents read/write on the plans root and execution
# access to the copied planning shell scripts. Every config file that is
# modified is first backed up as <file>.bak.<timestamp>. Additions are
# idempotent: entries already present are never duplicated.
backup_file_timestamp() {
    local file="$1" stamp backup n=1
    stamp="$(date +%Y%m%dT%H%M%S)"
    backup="${file}.bak.${stamp}"
    while [ -e "$backup" ]; do
        backup="${file}.bak.${stamp}.${n}"
        n=$((n + 1))
    done
    cp -p "$file" "$backup"
    echo "  Backup: $backup" >&2
}

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
# jq in runtime_requirements() — so verify_runtime_tools has already refused to
# get this far without jq. jq is therefore guaranteed, and the command -v check
# below only turns a hypothetical `set -e` abort into a clear message plus manual
# instructions. It has to precede backup_file_timestamp, or a failure here leaves
# an orphaned .bak.<timestamp> behind. python3 is deliberately not used anywhere:
# jq is the only runtime dependency this installer is allowed to add.
claude_permissions() {
    local cfg="${CLAUDE_CONFIGFILE:-$HOME/.claude/settings.json}" scripts="$1" plans="$2" tmp="$3"
    local doc added tmpfile program
    [ -f "$cfg" ] || { echo "  claude-code: no $cfg found; skipped" >&2; return 0; }
    if ! command -v jq >/dev/null 2>&1; then
        echo "  claude-code: jq is not installed; cannot edit $cfg safely." >&2
        print_manual_permissions claude "$scripts" "$plans" "$tmp"
        return 0
    fi
    plans="$(strip_trailing_slashes "$plans")"
    scripts="$(strip_trailing_slashes "$scripts")"
    tmp="$(strip_trailing_slashes "$tmp")"
    backup_file_timestamp "$cfg"

    # An unparseable or non-object settings.json is rebuilt from {} rather than
    # edited. `objectify` is the same defensive read at every level.
    doc="$(jq '.' "$cfg" 2>/dev/null || true)"
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
    added="$(printf '%s' "$doc" | jq -r \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'(entries - allowed)[]')"

    # mktemp in the config's own directory so the rename is atomic, and cp -p to
    # inherit the user's mode before jq truncates it.
    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    if ! printf '%s' "$doc" | jq \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'
        objectify
        | (.permissions | objectify) as $perm
        | ($perm.allow | if type == "array" then . else [] end) as $allow
        | .permissions = ($perm | .allow = ($allow + (entries - $allow)))' \
        > "$tmpfile"; then
        rm -f "$tmpfile"
        die "jq failed to update $cfg"
    fi
    mv "$tmpfile" "$cfg"

    if [ -n "$added" ]; then
        printf '  claude-code: added to permissions.allow:\n'
        printf '%s\n' "$added" | sed 's|^|    - |'
    else
        printf '  claude-code: permissions already present\n'
    fi
}

opencode_permissions() {
    local cfg="${OPENCODE_CONFIGFILE:-$HOME/.config/opencode/opencode.json}" scripts="$1" plans="$2" tmp="$3"
    local doc added legacy tmpfile program
    [ -f "$cfg" ] || { echo "  opencode: no $cfg found; skipped" >&2; return 0; }
    if ! command -v jq >/dev/null 2>&1; then
        echo "  opencode: jq is not installed; cannot edit $cfg safely." >&2
        print_manual_permissions opencode "$scripts" "$plans" "$tmp"
        return 0
    fi
    plans="$(strip_trailing_slashes "$plans")"
    scripts="$(strip_trailing_slashes "$scripts")"
    tmp="$(strip_trailing_slashes "$tmp")"
    backup_file_timestamp "$cfg"

    doc="$(jq '.' "$cfg" 2>/dev/null || true)"
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
    legacy="$(printf '%s' "$doc" | jq -r '
        (if type == "object" then . else {} end) | .permission
        | if type == "object" and (.allow | type) == "array" and (.allow | length) > 0
          then "yes" else "no" end')"
    added="$(printf '%s' "$doc" | jq -r \
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
    if ! printf '%s' "$doc" | jq \
        --arg plans "$plans" --arg scripts "$scripts" --arg tmp "$tmp" \
        "$program"'
        (if type == "object" then . else {} end) as $data
        | (reduce wanted[] as $w (base;
              .[$w[0]] = (reduce $w[1][] as $pattern ((.[$w[0]] | rules); .[$pattern] = "allow"))
          )) as $perm
        | $data | .permission = $perm' \
        > "$tmpfile"; then
        rm -f "$tmpfile"
        die "jq failed to update $cfg"
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
first copy it to <file>.bak.<timestamp> before editing, then tell me the exact
path and the entries you changed. Do not change any other permissions and do
not grant broad or all-tools access.
--- END AGENT PROMPT (copy from here) ---
PROMPT
}

planning_permission_step() {
    local plans="$HOME/.plans" agent_tmp="${TMPDIR:-/tmp}/planning-agent" root kind scripts
    echo >&2
    echo "== Step 2: planning runtime permissions ==" >&2
    if confirm "Create $plans as the global plans directory?"; then
        mkdir -p "$plans" && echo "  Created $plans" >&2
    fi
    if confirm "Grant the selected agents read/write on $plans and $agent_tmp, and allow them to execute the planning shell scripts? (Each edited config is backed up as .bak.timestamp)"; then
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

