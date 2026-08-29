# MODE: PROD
# lib-config.sh - read the recorded transport. Sourced by the helpers, never run.
#
# The transport is the user's decision, asked once by the server and recorded in
# $AI_CHAT_HOME/config. This is the bash half of reading it, and it is bash and
# not rjq on purpose: rjq is this repository's ceiling for a runtime dependency, so
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

# chat_config_trim <text>
#
# Strip leading and trailing whitespace. The config file invites hand-editing, so
# `transport = tcp` has to work as well as `transport=tcp`.
chat_config_trim() {
    local v="$1"
    v="${v#"${v%%[![:space:]]*}"}"
    v="${v%"${v##*[![:space:]]}"}"
    printf '%s\n' "$v"
}

# chat_config_read_keys <file>
#
# Read the raw keys into CHAT_TRANSPORT/CHAT_CFG_*. Split out from
# chat_config_load only to stay under the repository's function-length cap; the
# two are one operation and chat_config_load is the entry point.
chat_config_read_keys() {
    local file="$1" line key value
    # No `read -r key value` with IFS='=': a socket path may contain '=', and
    # only the FIRST '=' separates key from value.
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            ''|'#'*) continue ;;
        esac
        key="$(chat_config_trim "${line%%=*}")"
        value="$(chat_config_trim "${line#*=}")"
        case "$key" in
            transport) CHAT_TRANSPORT="$value" ;;
            bind) CHAT_CFG_BIND="$value" ;;
            port) CHAT_CFG_PORT="$value" ;;
            socket) CHAT_CFG_SOCKET="$value" ;;
        esac
    done < "$file"
}

# chat_config_load <home>
#
# Sets CHAT_TRANSPORT to tcp, socket, or none, plus CHAT_CFG_BIND/CHAT_CFG_PORT
# for tcp and CHAT_CFG_SOCKET for a socket. Never fails: an unreadable or absent
# config is 'none', which leaves the caller in local mode — the mode that needs
# no server at all.
chat_config_load() {
    local home="$1" file="$1/config"
    CHAT_TRANSPORT=none
    CHAT_CFG_BIND=""
    CHAT_CFG_PORT=""
    CHAT_CFG_SOCKET=""
    [ -f "$file" ] || return 0
    chat_config_read_keys "$file"
    chat_config_defaults "$home"
    return 0
}

# chat_config_defaults <home>
#
# Fill in what a partial entry leaves out, and refuse to guess at a transport we
# do not recognise. Shared by the config and the registry, so a file written by
# either is read the same way.
chat_config_defaults() {
    local home="$1"
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

# chat_registry_dir
#
# Where running servers register. $XDG_RUNTIME_DIR/chat when set - per-user,
# tmpfs, cleared on logout - else <temp>/chat-<uid>.
#
# The per-uid suffix is not cosmetic: /tmp is world-writable, so a fixed shared
# path would let another user plant an entry naming their own socket and every
# client on the machine would talk to them, with no authentication in the
# protocol to notice. This mirrors Registry::open in src/chat/src/registry.rs.
#
# Note also what is NOT claimed here: "cleared at reboot" is a Linux tmpfs
# property. macOS /private/tmp survives reboots and is pruned only after three
# days, so a stale entry must be survivable - which it is, because liveness is
# decided by dialling.
chat_registry_dir() {
    if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
        printf '%s\n' "$XDG_RUNTIME_DIR/chat"
        return 0
    fi
    printf '%s/chat-%s\n' "${TMPDIR:-/tmp}" "$(id -u)"
}

# chat_registry_find <home>
#
# Sets CHAT_TRANSPORT/CHAT_CFG_* from the registered server for <home>, if one is
# registered. Returns 1 when there is none.
#
# An entry is NOT proof of life - a crashed server leaves one behind - so the
# caller dials before trusting it. The helpers do not evict a dead entry the way
# the binary does: removing another process's file from a shared directory is a
# bigger claim than a read-only helper should make, and the binary cleans up on
# its next look.
chat_registry_find() {
    local home="$1" dir entry
    dir="$(chat_registry_dir)"
    [ -d "$dir" ] || return 1
    # Newest last, so the most recent registration wins if two ever coexist.
    for entry in $(ls -1 "$dir"/*.server 2>/dev/null | sort); do
        [ -f "$entry" ] || continue
        case "$(sed -n 's/^home=//p' "$entry" | sed -n '1p')" in
            "$home") ;;
            *) continue ;;
        esac
        chat_config_read_keys "$entry"
        chat_config_defaults "$home"
        [ "$CHAT_TRANSPORT" = none ] || return 0
    done
    return 1
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
    # Discovery before convention: a registered server's endpoint is a fact,
    # whereas the config says how a bus SHOULD be exposed and a recorded endpoint
    # with nothing registered means no server is running. The registry is
    # therefore consulted first, and the config only fills in a transport for a
    # bus that registered a partial entry.
    if ! chat_registry_find "$home"; then
        chat_config_load "$home"
    fi
    case "$CHAT_TRANSPORT" in
        tcp)
            CHAT_USE_HOST="$(chat_dial_host "$CHAT_CFG_BIND")"
            CHAT_USE_PORT="$CHAT_CFG_PORT"
            # An entry is not proof of life, and neither is a config. Only on this
            # path, never for an explicit --host: the user who named a server wants
            # that server, and quietly writing somewhere else would be worse than
            # an error. Here nobody named anything, so the log is right.
            if ! chat_can_connect "$CHAT_USE_HOST" "$CHAT_USE_PORT"; then
                printf '%s: nothing answers at %s:%s; using the log directly\n' \
                    "$me" "$CHAT_USE_HOST" "$CHAT_USE_PORT" >&2
                printf '%s: `chat serve` (or chat-server.sh start) starts one; a helper does not, because it cannot know which runtime you want\n' \
                    "$me" >&2
                CHAT_USE_HOST=""
                CHAT_USE_PORT=""
            fi
            ;;
        socket)
            printf '%s: the server is on a unix socket (%s), which bash cannot open; using the log directly\n' \
                "$me" "$CHAT_CFG_SOCKET" >&2
            printf '%s: `chat` (the binary) can speak to it, or record transport=tcp to use the helpers over a socket\n' \
                "$me" >&2
            ;;
    esac
    return 0
}
