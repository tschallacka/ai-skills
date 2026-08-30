# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 5. Agent target detection
# ---------------------------------------------------------------
# Which agents are installed on this machine, plus the user's remembered custom
# skill roots. agent_target_available() switches on the TARGET_* index, so the
# registry order in section 1 must not change.
agent_target_available() {
    local index="$1"
    local path="${TARGET_PATHS[$index]}"

    case "$index" in
        0) return 0 ;; # Universal Agent Skills has no owning application.
        1) command -v codex >/dev/null 2>&1 || [ -d "$HOME/.codex" ] ;;
        2) command -v claude >/dev/null 2>&1 || [ -d "$HOME/.claude" ] ;;
        3) command -v opencode >/dev/null 2>&1 || [ -d "$HOME/.config/opencode" ] ;;
        4) command -v openclaw >/dev/null 2>&1 || [ -d "$HOME/.openclaw" ] ;;
        5)
            [ -d "$path" ] || [ -d "$HOME/.vscode/extensions/saoudrizwan.claude-dev" ] \
                || compgen -G "$HOME/.vscode/extensions/saoudrizwan.claude-dev-*" >/dev/null 2>&1 \
                || compgen -G "$HOME/.vscode-server/extensions/saoudrizwan.claude-dev-*" >/dev/null 2>&1 \
                || [ -d "${XDG_CONFIG_HOME:-$HOME/.config}/Code/User/globalStorage/saoudrizwan.claude-dev" ] \
                || compgen -G "${XDG_CONFIG_HOME:-$HOME/.config}/Code/User/globalStorage/saoudrizwan.claude-dev*" >/dev/null 2>&1
            ;;
        *) return 1 ;;
    esac
}

# The shared skill-data directory beside the custom-locations file: user data
# that must survive a skill reinstall (project deviation notes) lives here, so
# the installer guarantees it exists and stays readable, writable and
# creatable by the running user before any skill lands. Agents registered
# under that user inherit the access.
ensure_agent_data_dir() {
    local dir
    dir="$(dirname "$CUSTOM_LOCATIONS_FILE")"
    mkdir -p "$dir"
    chmod u+rwx "$dir" 2>/dev/null || true
}

save_custom_location() {
    local path="$1"
    mkdir -p "$(dirname "$CUSTOM_LOCATIONS_FILE")"
    touch "$CUSTOM_LOCATIONS_FILE"
    if ! grep -Fqx "$path" "$CUSTOM_LOCATIONS_FILE"; then
        printf '%s\n' "$path" >> "$CUSTOM_LOCATIONS_FILE"
    fi
}

load_custom_locations() {
    SAVED_CUSTOM_LOCATIONS=()
    [ -f "$CUSTOM_LOCATIONS_FILE" ] || return 0
    local path
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        [[ "$path" = \#* ]] && continue
        [ -d "$path" ] || continue
        # ${arr[@]+…}: expanding an empty array is an unbound-variable abort
        # under `set -u` on bash < 4.4 (macOS ships 3.2).
        contains "$path" ${SAVED_CUSTOM_LOCATIONS[@]+"${SAVED_CUSTOM_LOCATIONS[@]}"} \
            || SAVED_CUSTOM_LOCATIONS+=("$path")
    done < "$CUSTOM_LOCATIONS_FILE"
}

