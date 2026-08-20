# MODE: DEV
# PACKAGE: DEV
# ---------------------------------------------------------------
# 8. Source acquisition
# ---------------------------------------------------------------
# Local checkout or downloaded tarball, decided solely by whether
# ${BASH_SOURCE[0]} is a readable file next to planning/SKILL.md — that is what
# makes the curl|bash form work, so do not replace the probe with $0 or a marker.
download_source() {
    local script_dir ref_prefix archive source_commit source_branch
    if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
        script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        if [ -f "$script_dir/planning/SKILL.md" ]; then
            SOURCE_ROOT="$script_dir"
            source_commit="$(git -C "$SOURCE_ROOT" rev-parse --short=12 HEAD 2>/dev/null || true)"
            source_branch="$(git -C "$SOURCE_ROOT" symbolic-ref --short -q HEAD 2>/dev/null || true)"
            SOURCE_VERSION="$(git -C "$SOURCE_ROOT" describe --tags --exact-match HEAD 2>/dev/null || true)"
            if [ -n "$SOURCE_VERSION" ]; then
                SOURCE_VERSION="tag:$SOURCE_VERSION commit:${source_commit:-unknown}"
            else
                SOURCE_VERSION="branch:${source_branch:-detached} commit:${source_commit:-unknown}"
            fi
            return
        fi
    fi

    command -v curl >/dev/null 2>&1 || die "curl is required"
    command -v tar >/dev/null 2>&1 || die "tar is required"
    TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills.XXXXXX")"
    local archive="$TEMP_ROOT/source.tar.gz"
    if [[ "$REPO_REF" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        ref_prefix="refs/tags"
        SOURCE_VERSION="tag:$REPO_REF"
        local url="${REPO_URL%/}/archive/${ref_prefix}/${REPO_REF}.tar.gz"
    elif [[ "$REPO_REF" =~ ^[0-9a-fA-F]{7,40}$ ]]; then
        SOURCE_VERSION="commit:$REPO_REF"
        local url="${REPO_URL%/}/archive/${REPO_REF}.tar.gz"
    else
        ref_prefix="refs/heads"
        source_commit=""
        if command -v git >/dev/null 2>&1; then
            source_commit="$(git ls-remote "$REPO_URL" "refs/heads/$REPO_REF" 2>/dev/null | awk 'NR == 1 {print $1}')"
        fi
        SOURCE_VERSION="branch:$REPO_REF commit:${source_commit:-unknown}"
        local url="${REPO_URL%/}/archive/${ref_prefix}/${REPO_REF}.tar.gz"
    fi
    echo "Downloading skills from $url" >&2
    curl -fsSL "$url" -o "$archive"
    tar -xzf "$archive" -C "$TEMP_ROOT"
    SOURCE_ROOT="$(find "$TEMP_ROOT" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
    [ -n "$SOURCE_ROOT" ] && [ -f "$SOURCE_ROOT/planning/SKILL.md" ] \
        || die "Downloaded archive does not contain the expected skills"
}

