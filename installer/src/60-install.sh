# ---------------------------------------------------------------
# 11. Interactive install, backup, and merge
# ---------------------------------------------------------------
# The interactive counterpart to section 10: compares every managed file against
# the source, then asks once per destination. A .version marker that differs is a
# managed upgrade and replaces without backups; anything else backs up first.
backup_file() {
    local file="$1"
    local backup="$file.bak"
    local suffix=1
    while [ -e "$backup" ] || [ -L "$backup" ]; do
        backup="$file.bak.$suffix"
        suffix=$((suffix + 1))
    done
    cp -p "$file" "$backup"
    echo "  Backup: $backup" >&2
}

install_skill() {
    local skill="$1"
    local root="$2"
    local destination="$root/$skill"
    local relative source destination_file
    local changed=0
    local missing=0
    local managed_version_transition=0
    local files

    files="$(skill_files "$skill")"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$skill" "$relative")"
        destination_file="$destination/$relative"
        if [ -L "$destination" ] || [ -L "$destination_file" ]; then
            echo "Skipping $root/$skill: existing symlink requires manual review." >&2
            summary_add "Skipped:   $destination — existing symlink requires manual review"
            return
        fi
        if [ ! -e "$destination_file" ]; then
            missing=1
        elif ! cmp -s "$source" "$destination_file"; then
            changed=1
        fi
    done <<EOF
$files
EOF

    if [ -L "$destination/.version" ]; then
        echo "Skipping $root/$skill: existing .version symlink requires manual review." >&2
        summary_add "Skipped:   $destination — existing .version symlink requires manual review"
        return
    elif [ ! -e "$destination/.version" ]; then
        missing=1
    elif ! cmp -s <(version_marker_content) "$destination/.version"; then
        changed=1
        managed_version_transition=1
    fi

    if [ "$changed" -eq 1 ]; then
        if [ "$YES" -eq 1 ]; then
            if [ "$managed_version_transition" -eq 1 ]; then
                echo "Version transition detected in $destination; replacing without backups." >&2
            else
                echo "Changes detected in $destination; replacing after backup." >&2
            fi
        elif [ "$managed_version_transition" -eq 1 ]; then
            if ! confirm "Installed version differs in $destination. Replace it without backups?"; then
                echo "Skipped $destination" >&2
                summary_add "Skipped:   $destination — replacement declined"
                return
            fi
        elif ! confirm "Changes detected in $destination. Replace them and create .bak backups?"; then
            echo "Skipped $destination" >&2
            summary_add "Skipped:   $destination — replacement declined"
            return
        fi
    elif [ "$missing" -eq 0 ]; then
        echo "Up to date: $destination" >&2
        summary_add "Up to date: $destination$(summary_soft_note "$skill")"
        return
    fi

    mkdir -p "$destination"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$skill" "$relative")"
        destination_file="$destination/$relative"
        if [ -e "$destination_file" ] && ! cmp -s "$source" "$destination_file"; then
            if [ "$managed_version_transition" -eq 0 ]; then
                backup_file "$destination_file"
            fi
        fi
        mkdir -p "$(dirname "$destination_file")"
        cp -p "$source" "$destination_file"
    done <<EOF
$files
EOF
    if [ -e "$destination/.version" ] && ! cmp -s <(version_marker_content) "$destination/.version" && [ "$managed_version_transition" -eq 0 ]; then
        backup_file "$destination/.version"
    fi
    version_marker_content > "$destination/.version"
    echo "Installed: $destination" >&2
    summary_add "Installed: $destination$(summary_soft_note "$skill")"
}

