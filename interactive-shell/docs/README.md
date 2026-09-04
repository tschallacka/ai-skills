<!-- MODE: PROD -->
# Interactive shell skill and tools

This directory contains the agent instructions for operating interactive
terminal programs. The Rust crate in `src/interactive-shell` provides the PTY
wrapper and Unix-socket input client.

The tools are generic: they do not know whether the child is `bash`, `nano`,
`mc`, `mcedit`, `lynx`, `less`, or another TUI. The agent observes the current
screen, discovers controls from the screen, built-in help, or a manpage, sends
one action, and verifies the resulting screen before choosing the next action.

## Build

Use the repository's Nix development environment for the current stable Rust
toolchain and dependencies:

```sh
nix develop .#default --command cargo build \
  --manifest-path src/interactive-shell/Cargo.toml
```

The resulting binaries are under `src/interactive-shell/target/debug/`:

- `interactive-shell` starts a child in a real PTY and exposes its terminal
  state through a Unix socket.
- `interactive-shell-input` observes that state and sends input requests.
- `interactive-shell-fixture` is a deterministic protocol-test child.

## Start and connect

Use a session ID so later input requests do not repeat the socket and command
configuration:

```sh
src/interactive-shell/target/debug/interactive-shell \
  --session agent-session --cols 80 --rows 24 --idle-timeout 300 -- \
  bash --noprofile --norc

src/interactive-shell/target/debug/interactive-shell-input \
  --session agent-session view
```

`--agent ID` is an alias for an agent-keyed session. Without an explicit ID,
the wrapper checks `INTERACTIVE_SHELL_AGENT`, `CODEX_AGENT_ID`, and `AGENT_ID`.
Explicit socket mode is also available:

```sh
interactive-shell --socket /private/session-dir/term.sock -- bash
interactive-shell-input --socket /private/session-dir/term.sock observe
```

The socket parent must be a private directory owned by the current user. A
wrapper-managed session directory is mode 0700 and its socket is mode 0600.

## Observe the screen

Start with the smallest useful response and request detail only when needed:

```text
view                 # compact text, including terminal dimensions
view 10-15           # only rows 10 through 15
view-delta           # rows changed since the previous view request
rgbview              # compact text with terminal colors preserved
rgbview-delta        # colored changed rows only
locate 'label'       # visible text coordinates, not a click guarantee
elements             # verified actionable elements and labels
observe              # complete JSON snapshot and metadata
wait 'expected text' 30000
```

Rows and mouse coordinates are one-based. `elements` reports semantic
actionable elements such as OSC 8 hyperlinks. Ordinary visible text is exposed
as a non-actionable coordinate hint. Use `click-id` or `click-label` only for
actionable elements; use `click-at` for coordinate-driven TUIs and verify the
result on the next screen.

View output includes the terminal size and rendered column spans. Prefer a
small terminal to preserve context, increasing it only when labels or paths
are clipped. Use `observe` when cursor position, styles, scrollback, or
structured element metadata matters.

## Send input

```text
text 'literal text'
paste 'text sent as a bracketed paste'
key ENTER
key TAB
key PAGEUP
key PAGEDOWN
key F4
combo h CTRL ALT SHIFT
raw 1b5b32315f7e
mouse 20 8 0 down
resize 100 30
shutdown
```

The named-key list is a transport vocabulary, not an application keymap. For
a combination not covered by a named key, use `combo`; it constructs the
terminal sequence at request time. Use `raw` only when the exact byte sequence
has been discovered or documented. Do not assume example shortcuts such as
Ctrl-X, F4, F10, or Q.

## Agent workflow

1. Start the requested command inside one interactive shell when subprocess
   behavior matters.
2. Request `view` and identify the application, focus, selection, prompts, and
   visible controls.
3. Discover unknown commands with visible help, application help, or a
   manpage. Prefer visible UI elements and keyboard navigation, including TAB,
   arrows, and page navigation.
4. Send one request at a time. Wait for a screen predicate or lifecycle event
   after each state-changing request; an acknowledgement only means the
   wrapper accepted the request.
5. Use a delta or targeted row range after each action. Use `elements`,
   `locate`, `rgbview`, or `observe` when compact text is insufficient.
6. Confirm writes, overwrite prompts, exits, and subprocess transitions from
   the screen. Branch on the screen actually returned rather than replaying a
   known sequence.
7. Shut down the session and report unsupported controls, ambiguous focus,
   clipped labels, or other obstacles that would make future decisions harder.

## Protocol and limits

The wrapper owns a real PTY, forwards child output, maintains a terminal screen
model, and emits JSONL lifecycle and screen events. Events contain sequence
numbers, the preceding base sequence, changed rows, and cursor state. `observe`
returns a full snapshot for resynchronization after missed deltas. The idle
timeout measures PTY output inactivity, not agent input or a pending wait.

The model covers common cursor movement, erase, alternate-screen, OSC,
charset, line-drawing, scroll-region, and insert/delete-line controls, but is
not a complete terminal emulator. Unicode cell widths may be approximate, and
an application's ncurses extension may be unsupported. Extend and test the
model before relying on a control it does not represent.

Run the focused tests with:

```sh
nix develop .#default --command cargo test \
  --manifest-path src/interactive-shell/Cargo.toml
```
