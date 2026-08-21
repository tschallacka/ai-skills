# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 3. Interactive input channel
# ---------------------------------------------------------------
# fd 3 is the one place prompts read from, so it is opened here, before anything
# can call ask/confirm and before show_splash's `[ -t 3 ]` guard. Order-critical:
# keep this block above every consumer.
#
# `ps -p $$ -o tty=` is not a usable tty probe: Linux prints `?` for no tty but
# macOS prints `??`, so a string compare against `?` passed on macOS, /dev/tty
# always exists as a node, and `exec 3</dev/tty` then failed with ENXIO — killing
# the installer under `set -e` instead of falling through to the closed-fd case.
# Just try the open and let it fail quietly. The probe runs in a subshell so a
# failed `exec` redirection cannot take the installer down with it — bash exits a
# non-interactive shell on a redirection error to a special builtin.
if [ -t 0 ]; then
    exec 3<&0
elif ( exec 3</dev/tty ) 2>/dev/null; then
    exec 3</dev/tty
else
    exec 3<&-
fi

ask() {
    local prompt="$1"
    printf '%s' "$prompt" >&2
    # 2>/dev/null: fd 3 is deliberately closed when there is no tty, and bash
    # prints its own "invalid file descriptor" before this can die, so the real
    # message arrived buried under shell noise.
    IFS= read -r -u 3 REPLY 2>/dev/null || die "Interactive input is required"
}

confirm() {
    local prompt="$1"
    # --yes answers every prompt, not only the replacement ones. It is the flag
    # for a headless run, and a headless run that stops on a question is the thing
    # it exists to prevent -- installing planning under --yes died on the plans
    # directory prompt. YES_ALL is the same answer reached interactively with "a".
    if [ "${YES:-0}" -eq 1 ] || [ "${YES_ALL:-0}" -eq 1 ]; then
        return 0
    fi
    ask "$prompt [y/N/a] "
    case "$REPLY" in
        y|Y|yes|YES) return 0 ;;
        a|A|all|ALL)
            YES_ALL=1
            return 0
            ;;
        *) return 1 ;;
    esac
}

contains() {
    local wanted="$1"
    shift
    local item
    for item in "$@"; do
        [ "$item" = "$wanted" ] && return 0
    done
    return 1
}

