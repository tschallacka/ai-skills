# ---------------------------------------------------------------
# 7. Skill and target selection
# ---------------------------------------------------------------
# Resolves SKILL_SELECTION/TARGET_SELECTION (from the flags) or asks, and leaves
# the answers in SELECTED_SKILLS, SELECTED_TARGET_PATHS and SELECTED_TARGET_NAMES.
# Asking is the picker of sections 6b-6d, with section 6's numbered menu as the
# fallback whenever it declines with 69.
select_skills() {
    SELECTED_SKILLS=()

    if [ -z "$SKILL_SELECTION" ]; then
        # The picker of sections 6b-6d first. 69 is its "fd 3 is not a terminal"
        # answer and the numbered menu below is the fallback, so every non-tty
        # path behaves exactly as it did before the picker existed.
        local ui_rc=0
        iui_select_skills || ui_rc="$?"
        case "$ui_rc" in
            0)
                [ "${#SELECTED_SKILLS[@]}" -gt 0 ] \
                    || { echo "Nothing selected; nothing was installed." >&2; exit 0; }
                return
                ;;
            69) ;;
            *)
                echo "Aborted; nothing was installed." >&2
                exit "$ui_rc"
                ;;
        esac
        show_shop_menu
        printf '\033[%d;1H' "$MENU_PROMPT_ROW"
        ask "Choose 1-6 or enter comma-separated names [6]: "
        SKILL_SELECTION="${REPLY:-6}"
    fi

    if [ "$SKILL_SELECTION" = "all" ] || [ "$SKILL_SELECTION" = "6" ]; then
        SELECTED_SKILLS=("${SKILL_NAMES[@]}")
        return
    fi

    local choice name
    IFS=',' read -r -a choices <<< "$SKILL_SELECTION"
    for choice in "${choices[@]}"; do
        name="$choice"
        # A menu number selects by position in SKILL_NAMES. The patterns exclude
        # a leading zero so `08` cannot reach $(( )) and be read as octal; an
        # out-of-range number falls through to the name check and dies there.
        case "$name" in
            [1-9]|[1-9][0-9])
                if [ "$name" -le "${#SKILL_NAMES[@]}" ]; then
                    name="${SKILL_NAMES[$((name - 1))]}"
                fi
                ;;
        esac
        contains "$name" "${SKILL_NAMES[@]}" || die "Unknown skill: $name"
        contains "$name" ${SELECTED_SKILLS[@]+"${SELECTED_SKILLS[@]}"} \
            || SELECTED_SKILLS+=("$name")
    done
}

select_targets() {
    SELECTED_TARGET_PATHS=()
    SELECTED_TARGET_NAMES=()

    if [ -n "$TARGET_SELECTION" ]; then
        SELECTED_TARGET_PATHS=("$TARGET_SELECTION")
        SELECTED_TARGET_NAMES=("$TARGET_SELECTION")
        if ! contains "$TARGET_SELECTION" "${TARGET_PATHS[@]}"; then
            save_custom_location "$TARGET_SELECTION"
        fi
        return
    fi

    AVAILABLE_TARGET_PATHS=()
    AVAILABLE_TARGET_NAMES=()
    local index path choice selection custom_choice
    load_custom_locations

    for index in "${!TARGET_PATHS[@]}"; do
        if agent_target_available "$index"; then
            AVAILABLE_TARGET_PATHS+=("${TARGET_PATHS[$index]}")
            AVAILABLE_TARGET_NAMES+=("${TARGET_NAMES[$index]}")
        fi
    done
    for path in ${SAVED_CUSTOM_LOCATIONS[@]+"${SAVED_CUSTOM_LOCATIONS[@]}"}; do
        AVAILABLE_TARGET_PATHS+=("$path")
        AVAILABLE_TARGET_NAMES+=("Custom: $path")
    done

    [ "${#AVAILABLE_TARGET_PATHS[@]}" -gt 0 ] || die "No installed agent roots or saved custom locations were found"

    echo >&2
    echo "Install into which skill root?" >&2
    for index in "${!AVAILABLE_TARGET_PATHS[@]}"; do
        path="${AVAILABLE_TARGET_PATHS[$index]}"
        if [ -d "$path" ]; then
            printf '  %d) %s: %s [exists]\n' "$((index + 1))" "${AVAILABLE_TARGET_NAMES[$index]}" "$path" >&2
        else
            printf '  %d) %s: %s [will create]\n' "$((index + 1))" "${AVAILABLE_TARGET_NAMES[$index]}" "$path" >&2
        fi
    done
    custom_choice=$(( ${#AVAILABLE_TARGET_PATHS[@]} + 1 ))
    echo "  $custom_choice) custom directory" >&2
    echo "  a) all listed roots" >&2
    ask "Choose 1-$custom_choice, comma-separated numbers, or a [1]: "
    selection="${REPLY:-1}"

    if [ "$selection" = "a" ] || [ "$selection" = "all" ]; then
        SELECTED_TARGET_PATHS=("${AVAILABLE_TARGET_PATHS[@]}")
        SELECTED_TARGET_NAMES=("${AVAILABLE_TARGET_NAMES[@]}")
        echo "Warning: multiple roots can make the same skill appear more than once." >&2
        return
    fi

    IFS=',' read -r -a choices <<< "$selection"
    for choice in "${choices[@]}"; do
        if [ "$choice" = "$custom_choice" ]; then
            ask "Custom skill root: "
            [ -n "$REPLY" ] || die "A custom directory is required"
            path="${REPLY/#\~/$HOME}"
            case "$path" in
                /*) ;;
                *) die "Custom directory must be an absolute path" ;;
            esac
            if [ ! -d "$path" ]; then
                if ! confirm "$path does not exist. Create it?"; then
                    die "Custom directory does not exist: $path"
                fi
                mkdir -p "$path"
            fi
            save_custom_location "$path"
            SELECTED_TARGET_PATHS+=("$path")
            SELECTED_TARGET_NAMES+=("Custom: $path")
        elif [ "$choice" -ge 1 ] 2>/dev/null && [ "$choice" -le "${#AVAILABLE_TARGET_PATHS[@]}" ]; then
            index=$((choice - 1))
            SELECTED_TARGET_PATHS+=("${AVAILABLE_TARGET_PATHS[$index]}")
            SELECTED_TARGET_NAMES+=("${AVAILABLE_TARGET_NAMES[$index]}")
        else
            die "Unknown target choice: $choice"
        fi
    done
}

