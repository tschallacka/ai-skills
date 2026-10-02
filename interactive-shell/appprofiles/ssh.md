# ssh

### Identity
Remote shell client. `ssh [user@]host`. Once connected, the remote side's
own program (usually a login shell, or whatever command/TUI was requested)
takes over the screen entirely -- this profile covers only the local
connection-time prompts.

### Modes
Not modal in the usual sense: before a session starts, ssh itself owns the
screen for host-key verification and password entry; once connected, every
byte goes to (and the whole screen reflects) the remote side.

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| `~.` (tilde, dot, at start of a line) | Terminate the connection | Local ssh escape sequence; must be the first character after a newline to be recognized, not a raw key at any point |
| `~C` | Open ssh's own command line (add port forwards, etc.) | Same start-of-line requirement as `~.` |
| `~?` | List escape sequences | Same requirement |

### Dialogs
- **Host key verification** (first connection to an unknown host):
  `The authenticity of host '<host> (<ip>)' can't be established.` +
  fingerprint line + `Are you sure you want to continue connecting
  (yes/no/[fingerprint])?`. Typing `yes` + ENTER accepts and proceeds (adds
  to `known_hosts`). Typing `no` + ENTER refuses: prints `Host key
  verification failed.` and exits immediately, no connection made.
- **Password prompt** (`password authentication`): `<user>@<host>'s
  password:`, input not echoed to the screen. A wrong password re-prompts
  (up to a server-configured retry limit) rather than exiting immediately.

### Workflows
1. First connection to a new host: check the host-key prompt appears,
   answer `yes` (trust) or `no` (abort) deliberately -- never send `yes`
   reflexively without a way to verify the fingerprint is expected.
2. Password login: after host-key acceptance (or on a known host), type
   the password at the `password:` prompt, ENTER; check the next screen
   for either a shell prompt (success) or `Permission denied` (retry or
   fail).

### Quirks
- The `~` escape sequences only work as the FIRST character typed after a
  newline on the local side -- sending `~` mid-line is passed through to
  the remote program like any other character.
- Refusing the host-key prompt (`no`) ends the connection immediately with
  no further prompt; the wrapper session ends since ssh itself exits.

### Unconfirmed
- Behavior of `~C`'s own command line beyond opening it
- Keyboard-interactive (as opposed to plain password) authentication
  prompt wording, if a server uses it
