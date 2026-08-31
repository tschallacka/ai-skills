---
name: interactive-shell
description: Drive a generic interactive terminal program through a PTY wrapper and local Unix socket.
---

# Interactive shell

Use this skill when an agent must operate a full-screen or interactive program
such as nano, Midnight Commander, a pager, or a terminal menu. Start
`interactive-shell --socket PATH --cols 80 --rows 24 --idle-timeout 300 -- COMMAND`
and observe its JSONL stdout. Send one request at a time with
`interactive-shell-input --socket PATH text TEXT`, `key KEY`, `raw HEX`, or
`shutdown`.

Screen events contain only rows changed since the previous event, a monotonically
increasing `seq`, the preceding `base`, and the cursor. Parse those JSON fields;
do not grep terminal output or acknowledgements. An acknowledgement means only
that input was accepted by the wrapper, not that the program completed the
resulting action. Wait for a screen predicate or lifecycle event after every
state-changing request.

Named keys include ENTER, CTRL-X, CTRL-O, CTRL-C, CTRL-D, CTRL-Z, CTRL-E,
BACKSPACE, TAB, ESC, META-RIGHT, UP, DOWN, LEFT, RIGHT, HOME, END, PAGEUP,
PAGEDOWN, INSERT, DELETE, and F1 through F4. Use `raw` for an explicit byte
sequence, encoded as non-empty even-length hexadecimal.

The wrapper allocates a real PTY, applies the requested dimensions, and owns a
0600 socket inside a private 0700 directory. It emits a final lifecycle event
for child exit, client shutdown, or idle timeout and removes only the socket
inode it created. A child process group is terminated during cleanup.

The screen model covers common cursor movement, erase, visibility, OSC,
alternate-screen, charset, and line-drawing controls, including sequences split
across PTY reads. It is intentionally not a complete terminal emulator: scroll
regions, insert/delete line operations, colors, and every ncurses extension are
not represented. For mc or another TUI, use predicates based only on represented
controls, or extend and test the model before relying on a new control.
