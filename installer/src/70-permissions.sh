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

# codex reads ~/.codex/config.toml, which is TOML, not JSON -- rjq is
# JSON-only and this installer is not allowed a second runtime dependency
# (B235). So this handles exactly one well-defined shape: a single-line
# `writable_roots = [...]` array, wherever in the file it appears (TOML
# allows either a dotted root key `sandbox_workspace_write.writable_roots =
# [...]` or a `[sandbox_workspace_write]` table with a `writable_roots` key
# inside it; both look identical on the matching line, so one grep covers
# both). A multi-line array, or a file this cannot safely extend, falls back
# to manual instructions -- the same posture opencode_permissions takes for a
# config strict rjq cannot parse.
codex_quoted_csv() { # <path...> -> "path1", "path2"
    local path first=1
    for path in "$@"; do
        [ "$first" -eq 1 ] || printf ', '
        printf '"%s"' "$path"
        first=0
    done
}

codex_writable_roots_line() { # <cfg> -> "<1-based line>:<content>" or nothing
    grep -n 'writable_roots[[:space:]]*=' "$1" 2>/dev/null | head -1
}

# Writes a brand-new dotted-key line naming every path. Prepended (never
# appended) when other content already exists: TOML's dotted-key syntax is
# only guaranteed to define a ROOT-level key while no `[table]` header has
# been opened yet, so appending after an existing file's own sections could
# silently nest this key inside whichever section happens to be last instead
# of at the root the reader expects. Prepending sidesteps that entirely.
codex_write_fresh_roots() { # <cfg> <label> <path...>
    local cfg="$1" label="$2" tmpfile
    shift 2
    if [ ! -f "$cfg" ]; then
        mkdir -p "$(dirname "$cfg")" || { echo "  codex: cannot create $(dirname "$cfg")/" >&2; return 1; }
        printf 'sandbox_workspace_write.writable_roots = [%s]\n' "$(codex_quoted_csv "$@")" > "$cfg" \
            || { echo "  codex: cannot write $cfg" >&2; return 1; }
        echo "  codex: created $cfg" >&2
    else
        backup_file "$cfg"
        tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
        cp -p "$cfg" "$tmpfile"
        { printf 'sandbox_workspace_write.writable_roots = [%s]\n' "$(codex_quoted_csv "$@")"; cat "$cfg"; } \
            > "$tmpfile" || { rm -f "$tmpfile"; die "cannot write next to $cfg"; }
        mv "$tmpfile" "$cfg"
    fi
    printf '  codex: %s\n' "$label"
}

# Appends missing paths into an already-present single-line array, in place,
# preserving everything else on the line (trailing whitespace, a comment).
codex_append_roots() { # <cfg> <label> <line_no> <content> <path...>
    local cfg="$1" label="$2" line_no="$3" content="$4" path tmpfile
    shift 4
    local to_add=()
    for path in "$@"; do
        case "$content" in
            *"\"$path\""*) ;;
            *) to_add+=("$path") ;;
        esac
    done
    [ "${#to_add[@]}" -gt 0 ] || { printf '  codex: %s\n' "$label"; return 0; }

    backup_file "$cfg"
    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    awk -v n="$line_no" -v repl="${content%%]*}, $(codex_quoted_csv "${to_add[@]}")]${content#*]}" \
        'NR==n { print repl; next } { print }' "$cfg" > "$tmpfile" \
        || { rm -f "$tmpfile"; die "cannot update $cfg"; }
    mv "$tmpfile" "$cfg"
    printf '  codex: added to writable_roots:\n'
    printf '%s\n' "${to_add[@]}" | sed 's|^|    - |'
}

codex_merge_writable_roots() { # <cfg> <label> <path...> -- 1 means "fall back"
    local cfg="$1" label="$2" grep_line line_no content
    shift 2
    if [ ! -f "$cfg" ]; then
        codex_write_fresh_roots "$cfg" "$label" "$@"
        return 0
    fi
    grep_line="$(codex_writable_roots_line "$cfg")"
    if [ -z "$grep_line" ]; then
        codex_write_fresh_roots "$cfg" "$label" "$@"
        return 0
    fi
    line_no="${grep_line%%:*}"
    content="${grep_line#*:}"
    case "$content" in
        *'['*']'*) codex_append_roots "$cfg" "$label" "$line_no" "$content" "$@" ;;
        *)
            echo "  codex: $cfg's writable_roots is not a single-line array; add these by hand:" >&2
            return 1
            ;;
    esac
}

codex_permissions() {
    local scripts="$1" plans="$2" tmp="$3"
    plans="$(strip_trailing_slashes "$plans")"
    scripts="$(strip_trailing_slashes "$scripts")"
    tmp="$(strip_trailing_slashes "$tmp")"
    codex_merge_writable_roots "${CODEX_CONFIGFILE:-$HOME/.codex/config.toml}" \
        'writable_roots already present' "$plans" "$scripts" "$tmp" \
        || print_manual_permissions codex "$scripts" "$plans" "$tmp"
}

codex_worktrees_permissions() {
    local worktrees
    worktrees="$(strip_trailing_slashes "$1")"
    codex_merge_writable_roots "${CODEX_CONFIGFILE:-$HOME/.codex/config.toml}" \
        'worktree grant already in place' "$worktrees" \
        || print_manual_worktrees_permissions codex "$worktrees"
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
                codex)    codex_worktrees_permissions "$worktrees" ;;
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
                codex)    codex_permissions "$scripts" "$plans" "$agent_tmp" ;;
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

# Vendor-shipped reference docs (interactive-shell/appprofiles/FORMAT.md and
# one per common TUI program) land in a stable, agent-facing location
# independent of which agent root(s) were selected: SELECTED_TARGET_PATHS[0]
# is only where they are read FROM, since every installed copy is identical.
# Reinstalled (overwritten) every run, so an updated profile always replaces
# an older one already on disk -- unconditional, no confirm, since these are
# read-only reference files with no permission or security implication.
interactive_shell_appprofiles_step() {
    local source="${SELECTED_TARGET_PATHS[0]%/}/interactive-shell/appprofiles"
    local destination="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/appprofiles"
    local file base copied=0
    [ -d "$source" ] || return 0
    mkdir -p "$destination" || { echo "  app profiles: cannot create $destination" >&2; return 0; }
    for file in "$source"/*.md; do
        [ -f "$file" ] || continue
        base="${file##*/}"
        if cp "$file" "$destination/$base"; then
            copied=$((copied + 1))
        else
            echo "  app profiles: cannot write $destination/$base" >&2
        fi
    done
    echo "  app profiles: installed $copied file(s) to $destination" >&2
}

# The files a Claude Code root actually needs from tui-hint-plugin/ -- not
# README.md, opencode/, or tests/, none of which the plugin loader reads.
tui_hint_plugin_claude_files() {
    cat <<'EOF'
.claude-plugin/plugin.json
hooks/hooks.json
hooks/lib.sh
hooks/pre-tool-use.sh
EOF
}

# Installed the same way agent-identity-plugin/ is: whatever this run's
# checkout ships is copied verbatim into the target root, not registered as
# a selectable skill in SKILL_NAMES (nothing chooses it directly; it rides
# with interactive-shell).
install_tui_hint_plugin_claude() {
    local root="$1" relative source destination_file
    local destination="$root/tui-hint-plugin"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$SOURCE_ROOT/tui-hint-plugin/$relative"
        [ -f "$source" ] || continue
        destination_file="$destination/$relative"
        mkdir -p "$(dirname "$destination_file")"
        cp -p "$source" "$destination_file"
    done < <(tui_hint_plugin_claude_files)
    chmod +x "$destination"/hooks/*.sh 2>/dev/null || true
}

# opencode has no per-root plugin directory the way a Claude Code root does
# -- plugins are declared globally in opencode.jsonc's own "plugin" array
# (a bare npm package name, or a local file path -- see
# .agents/knowledge/opencode-plugin-loading-and-advisory-injection.md). The
# .js file is copied once to a stable location under this installer's own
# XDG directory, independent of which root(s) were selected, and that path
# is added to the array if not already present.
opencode_register_plugin_entry() { # <cfg> <destination> <created:0|1>
    local cfg="$1" destination="$2" created="$3"
    local doc added tmpfile
    doc="$(rjq '.' "$cfg" 2>/dev/null || true)"
    [ -n "$doc" ] || doc='{}'
    added='false'
    printf '%s' "$doc" | rjq -e --arg entry "$destination" \
        '(.plugin // []) | index($entry) != null' >/dev/null 2>&1 || added='true'

    tmpfile="$(mktemp "$cfg.tmp.XXXXXX")" || die "cannot write next to $cfg"
    cp -p "$cfg" "$tmpfile"
    if ! printf '%s' "$doc" | rjq --arg entry "$destination" '
        (if type == "object" then . else {} end) as $data
        | ($data.plugin | if type == "array" then . else [] end) as $existing
        | $data | .plugin = ($existing + (if ($existing | index($entry)) then [] else [$entry] end))' \
        > "$tmpfile"; then
        rm -f "$tmpfile"
        die "rjq failed to update $cfg"
    fi
    mv "$tmpfile" "$cfg"

    if [ "$added" = true ]; then
        echo "  opencode: added $destination to the plugin array" >&2
    else
        echo "  opencode: plugin already registered" >&2
    fi
}

install_tui_hint_plugin_opencode() {
    local source="$SOURCE_ROOT/tui-hint-plugin/opencode/tui-hint-plugin.js"
    local destination="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/tui-hint-plugin/tui-hint-plugin.js"
    local cfg created=0
    [ -f "$source" ] || return 0
    mkdir -p "$(dirname "$destination")" || { echo "  opencode: cannot create $(dirname "$destination")/" >&2; return 0; }
    cp -p "$source" "$destination" || { echo "  opencode: cannot write $destination" >&2; return 0; }

    cfg="$(opencode_configfile)"
    if [ ! -f "$cfg" ]; then
        mkdir -p "$(dirname "$cfg")" || { echo "  opencode: cannot create $(dirname "$cfg")/" >&2; return 0; }
        printf '{\n  "$schema": "https://opencode.ai/config.json"\n}\n' > "$cfg" \
            || { echo "  opencode: cannot write $cfg" >&2; return 0; }
        echo "  opencode: created $cfg" >&2
        created=1
    fi
    if ! command -v rjq >/dev/null 2>&1; then
        echo "  opencode: rjq is not installed; add \"$destination\" to $cfg's \"plugin\" array by hand." >&2
        return 0
    fi
    if [ "$created" -eq 0 ] && [ -s "$cfg" ] && ! rjq -e '.' "$cfg" >/dev/null 2>&1; then
        echo "  opencode: cannot safely rewrite $cfg (comments or trailing commas); add \"$destination\" to its \"plugin\" array by hand." >&2
        return 0
    fi
    [ "$created" -eq 1 ] || backup_file "$cfg"

    opencode_register_plugin_entry "$cfg" "$destination" "$created"
}

# Every root a tui-hint-relevant skill was actually selected for. Claude
# Code gets its own copy of the plugin directory per root (the loader reads
# it from there); opencode's registration is global, so it is done at most
# once regardless of how many opencode roots were selected.
tui_hint_plugin_step() {
    local root kind opencode_done=0
    contains interactive-shell "${SELECTED_SKILLS[@]}" || return 0
    echo >&2
    echo "== tui-hint plugin ==" >&2
    for root in "${SELECTED_TARGET_PATHS[@]}"; do
        kind="$(agent_kind_for_root "$root")"
        case "$kind" in
            claude)
                install_tui_hint_plugin_claude "$root"
                echo "  Installed: $root/tui-hint-plugin (reminds an agent of a shipped app profile before it runs a Bash command headlessly)" >&2
                ;;
            opencode)
                [ "$opencode_done" -eq 1 ] && continue
                install_tui_hint_plugin_opencode
                opencode_done=1
                ;;
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

# The files a Claude Code root actually needs from editor-gate-plugin/ --
# not README.md or tests/, neither of which the plugin loader reads.
editor_gate_plugin_files() {
    cat <<'EOF'
.claude-plugin/plugin.json
hooks/hooks.json
hooks/lib.sh
hooks/editor-token
hooks/pre-tool-use-bash.sh
hooks/pre-tool-use-edit-write.sh
EOF
}

# Installed the same way tui-hint-plugin/ and agent-identity-plugin/ are:
# whatever this run's checkout ships is copied verbatim into the target
# root, not registered as a selectable skill in SKILL_NAMES -- nothing
# chooses it directly, it rides with ai-text-editor.
install_editor_gate_plugin() {
    local root="$1" relative source destination_file
    local destination="$root/editor-gate-plugin"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$SOURCE_ROOT/editor-gate-plugin/$relative"
        [ -f "$source" ] || continue
        destination_file="$destination/$relative"
        mkdir -p "$(dirname "$destination_file")"
        cp -p "$source" "$destination_file"
    done < <(editor_gate_plugin_files)
    chmod +x "$destination"/hooks/editor-token "$destination"/hooks/*.sh 2>/dev/null || true
}

# Bash/Edit/Write PreToolUse hooks are Claude Code's own -- offered only to
# a selected Claude Code root, same gate editor_steering_step already uses.
editor_gate_plugin_step() {
    local root kind
    for root in "${SELECTED_TARGET_PATHS[@]}"; do
        kind="$(agent_kind_for_root "$root")"
        [ "$kind" = claude ] || continue
        install_editor_gate_plugin "$root"
        echo "  Installed: $root/editor-gate-plugin (gates sed -i/perl -i/heredoc writes behind a minted token; see editor-gate-plugin/README.md)" >&2
    done
}
