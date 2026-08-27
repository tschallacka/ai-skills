# MODE: PROD
# lib-config.sh - read the recorded transport. Sourced by the helpers, never run.
#
# The transport is the user's decision, asked once by the server and recorded in
# $AI_CHAT_HOME/config. This is the bash half of reading it, and it is bash and
# not jq on purpose: jq is this repository's ceiling for a runtime dependency, so
# a config the helpers could not read without it would defeat the point of
# having one. The file is therefore key=value, one per line, # comments.
#
# PRECEDENCE, identical to the binary's: an explicit flag beats the config, and
# the config beats the built-in default (127.0.0.1:7717). Each helper applies it
# by calling chat_config_load only when no --host was given.
#
# shellcheck shell=bash

# Built-in default, and the port every helper assumed before there was a config.
CHAT_DEFAULT_PORT=7717
CHAT_DEFAULT_BIND=127.0.0.1

# chat_config_load <home>
#
# Sets CHAT_TRANSPORT to tcp, socket, or none, plus CHAT_CFG_BIND/CHAT_CFG_PORT
# for tcp and CHAT_CFG_SOCKET for a socket. Never fails: an unreadable or absent
# config is 'none', which leaves the caller in local mode — the mode that needs
# no server at all.
chat_config_load() {
    local home="$1" file="$1/config" key value
    CHAT_TRANSPORT=none
    CHAT_CFG_BIND=""
    CHAT_CFG_PORT=""
    CHAT_CFG_SOCKET=""
    [ -f "$file" ] || return 0
    # No `read -r line` splitting on '=': a socket path may contain one, and
    # only the FIRST '=' separates key from value.
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            ''|'#'*) continue ;;
        esac
        key="${line%%=*}"
        value="${line#*=}"
        # Trim the spaces a hand-edit leaves behind. The file invites editing,
        # so `transport = tcp` has to work.
        key="${key#"${key%%[![:space:]]*}"}"
        key="${key%"${key##*[![:space:]]}"}"
        value="${value#"${value%%[![:space:]]*}"}"
        value="${value%"${value##*[![:space:]]}"}"
        case "$key" in
            transport) CHAT_TRANSPORT="$value" ;;
            bind) CHAT_CFG_BIND="$value" ;;
            port) CHAT_CFG_PORT="$value" ;;
            socket) CHAT_CFG_SOCKET="$value" ;;
        esac
    done < "$file"
    case "$CHAT_TRANSPORT" in
        tcp)
            [ -n "$CHAT_CFG_BIND" ] || CHAT_CFG_BIND="$CHAT_DEFAULT_BIND"
            # tcp with no port means the port the helpers already assume.
            case "$CHAT_CFG_PORT" in
                ''|*[!0-9]*) CHAT_CFG_PORT="$CHAT_DEFAULT_PORT" ;;
            esac
            ;;
        socket)
            [ -n "$CHAT_CFG_SOCKET" ] || CHAT_CFG_SOCKET="$home/chat.sock"
            ;;
        *)
            # An unknown transport is not guessed at. Local mode is correct and
            # lossless, so degrade to it rather than dialling something invented.
            CHAT_TRANSPORT=none
            ;;
    esac
    return 0
}

# chat_dial_host <bind>
#
# The address to CONNECT to for a given bind address. A wildcard bind is an
# address to listen on and not a host: 0.0.0.0 means "every interface" to bind(),
# Linux routes a connect to it back to loopback by accident, and macOS is less
# obliging. Mirrors Transport::dial_host in src/chat/src/config.rs exactly; if
# one changes, change both.
chat_dial_host() {
    case "$1" in
        0.0.0.0|'') printf '%s\n' "$CHAT_DEFAULT_BIND" ;;
        ::|'[::]') printf '%s\n' '::1' ;;
        *) printf '%s\n' "$1" ;;
    esac
}

# chat_can_connect <host> <port>
#
# True when something accepts a connection there. A probe rather than a guess,
# because the alternative is what the helpers did before this existed: dial the
# configured port and die with a raw "/dev/tcp/...: Connection refused" from
# bash, exit 1, message unwritten. The whole point of local mode is that a dead
# server never blocks writing history.
chat_can_connect() {
    ( exec 3<> "/dev/tcp/$1/$2" ) 2>/dev/null
}

# chat_config_target <home> <helper-name>
#
# Apply the config to an invocation that named no --host. Sets CHAT_USE_HOST and
# CHAT_USE_PORT when the helper should go over a socket, and leaves them empty
# when it should stay local.
#
# transport=socket leaves them empty on purpose: bash has no unix-domain form of
# /dev/tcp, so a helper cannot speak to a unix socket at all. Local mode is
# complete — send appends under the same channel lock the server uses, read and
# tail read the same log — so the helper degrades rather than failing, and says
# so once. The `chat` binary's client subcommands DO speak the unix socket, which
# is the other half of why they exist.
chat_config_target() {
    local home="$1" me="$2"
    CHAT_USE_HOST=""
    CHAT_USE_PORT=""
    chat_config_load "$home"
    case "$CHAT_TRANSPORT" in
        tcp)
            CHAT_USE_HOST="$(chat_dial_host "$CHAT_CFG_BIND")"
            CHAT_USE_PORT="$CHAT_CFG_PORT"
            # Only on the CONFIG path, never for an explicit --host: the user who
            # named a server wants that server, and quietly writing somewhere
            # else instead would be worse than an error. Here nobody named
            # anything, so the log is the right answer and the note says so.
            if ! chat_can_connect "$CHAT_USE_HOST" "$CHAT_USE_PORT"; then
                printf '%s: nothing answers at %s:%s (the recorded transport); using the log directly\n' \
                    "$me" "$CHAT_USE_HOST" "$CHAT_USE_PORT" >&2
                CHAT_USE_HOST=""
                CHAT_USE_PORT=""
            fi
            ;;
        socket)
            printf '%s: the recorded transport is a unix socket (%s), which bash cannot open; using the log directly\n' \
                "$me" "$CHAT_CFG_SOCKET" >&2
            printf '%s: `chat` (the binary) can speak to it, or record transport=tcp to use the helpers over a socket\n' \
                "$me" >&2
            ;;
    esac
    return 0
}
