# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 11. Interactive install, backup, and merge
# ---------------------------------------------------------------
# The interactive counterpart to section 10: compares every managed file against
# the source, then asks once per destination. A .version marker that differs is a
# managed upgrade. Either way a file is backed up unless its recorded digest
# proves we wrote it and nobody has touched it since.
# A change detector for the installed content, recorded per file so the next run
# can tell "we wrote this and nobody touched it" from "the user edited it". The
# whole-skill version marker cannot make that distinction: before this, a version
# transition replaced every file without a backup, including edited ones.
#
# cksum, not sha256: POSIX-mandated so it needs no probe and adds no dependency,
# and it gives CRC-32 plus the byte count. The value never leaves the machine
# that wrote it -- it is compared against the same machine's next install -- so
# cross-platform agreement does not matter, and this is a change detector, not a
# security boundary.
content_digest() {
    cksum "$1" | awk '{print $1 "-" $2}'
}

# Written after a successful copy, so a run that dies part-way leaves the old
# manifest and the next run treats the untouched files as ours (correct) and the
# half-written ones as modified (a needless backup, never a lost edit).
record_digests() {
    local destination="$1" files="$2" relative manifest temporary
    manifest="$(digest_manifest "$destination")"
    temporary="$manifest.tmp.$$"
    : > "$temporary"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        [ -f "$destination/$relative" ] || continue
        printf '%s %s\n' "$(content_digest "$destination/$relative")" "$relative" >> "$temporary"
    done <<RECORD
$files
RECORD
    printf '%s %s\n' "$(content_digest "$destination/.version")" .version >> "$temporary"
    mv -f "$temporary" "$manifest"
}

digest_manifest() {
    printf '%s/.filehashes\n' "$1"
}

# The digest this skill recorded for one relative path, or nothing.
recorded_digest() {
    local destination="$1" relative="$2" manifest
    manifest="$(digest_manifest "$destination")"
    [ -f "$manifest" ] || return 0
    awk -v want="$relative" '$2 == want { print $1; exit }' "$manifest"
}

# True when the file on disk is byte-for-byte what we last installed, so
# replacing it destroys nothing.
unmodified_since_install() {
    local destination="$1" relative="$2" file="$3" recorded
    recorded="$(recorded_digest "$destination" "$relative")"
    [ -n "$recorded" ] || return 1
    [ "$recorded" = "$(content_digest "$file")" ]
}

# A backup only earns its clutter where nothing else can recover the file. Inside
# a git work tree the user already has history, so we say what happened and let
# git be the recovery path; outside one, the .back file IS the only path.
recoverable_from_git() {
    local directory="$1"
    command -v git >/dev/null 2>&1 || return 1
    git -C "$directory" rev-parse --is-inside-work-tree >/dev/null 2>&1
}

# A dotfile beside the original: visible to `ls -a` and to the message below,
# skipped by a plain `grep -r` of the installed tree.
backup_file() {
    local file="$1"
    local directory base stamp backup suffix
    directory="$(dirname "$file")"
    if recoverable_from_git "$directory"; then
        echo "  Replaced (recoverable with git): $file" >&2
        return 0
    fi
    base="$(basename "$file")"
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    backup="$directory/.$base.$stamp.back"
    suffix=1
    while [ -e "$backup" ] || [ -L "$backup" ]; do
        backup="$directory/.$base.$stamp.$suffix.back"
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
    local rjq_notice_printed=0
    local files
    local overview_artifact=''

    if [ "$skill" = planning ]; then
        overview_artifact="$(plan_overview_selected_artifact || true)"
    fi

    # A bundled-binary row whose artifact is absent is skipped with a notice
    # rather than failing the install: CI delivers these per host, and the
    # notice names where the artifact comes from. Returns 0 when the row must
    # be skipped.
    bundled_bin_row_missing() {
        case "$1" in
            bin/*/rjq|bin/*/rjq.exe) ;;
            *) return 1 ;;
        esac
        [ -x "$SOURCE_ROOT/planning/$1" ] && return 1
        if [ "$rjq_notice_printed" -eq 0 ]; then
            bundled_rjq_missing_notice
            rjq_notice_printed=1
        fi
        return 0
    }

    files="$(skill_files "$skill" "$PACKAGE_SELECTION")"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        if [ "$skill" = planning ] && { case "$relative" in bin/*/plan-overview|bin/*/plan-overview.exe) true ;; *) false ;; esac; }; then
            [ "$relative" = "$overview_artifact" ] || continue
        fi
        if [ "$skill" = planning ] && bundled_bin_row_missing "$relative"; then
            continue
        fi
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
                echo "Version transition detected in $destination; replacing, backing up any edited file." >&2
            else
                echo "Changes detected in $destination; replacing after backup." >&2
            fi
        elif [ "$managed_version_transition" -eq 1 ]; then
            if ! confirm "Installed version differs in $destination. Replace it? Any file you have edited is backed up first."; then
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
        if [ "$skill" = planning ] && { case "$relative" in bin/*/plan-overview|bin/*/plan-overview.exe) true ;; *) false ;; esac; }; then
            [ "$relative" = "$overview_artifact" ] || continue
        fi
        if [ "$skill" = planning ] && bundled_bin_row_missing "$relative"; then
            continue
        fi
        source="$(source_file "$skill" "$relative")"
        destination_file="$destination/$relative"
        # Back up unless we can prove the file is ours and untouched. A version
        # transition no longer suppresses this: the marker says the version
        # changed, not that the user's edits are expendable.
        if [ -e "$destination_file" ] && ! cmp -s "$source" "$destination_file" \
            && ! unmodified_since_install "$destination" "$relative" "$destination_file"; then
            backup_file "$destination_file"
        fi
        mkdir -p "$(dirname "$destination_file")"
        cp -p "$source" "$destination_file"
    done <<EOF
$files
EOF
    # The marker is ours by definition, so it is only worth keeping when it does
    # not match any version we wrote.
    if [ -e "$destination/.version" ] && ! cmp -s <(version_marker_content) "$destination/.version" \
        && ! unmodified_since_install "$destination" .version "$destination/.version"; then
        backup_file "$destination/.version"
    fi
    version_marker_content > "$destination/.version"
    record_digests "$destination" "$files"
    echo "Installed: $destination" >&2
    summary_add "Installed: $destination$(summary_soft_note "$skill")"
}
