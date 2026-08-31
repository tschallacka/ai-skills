---
name: interactive-shell
description: Drive a generic interactive terminal program through a PTY wrapper and local Unix socket.
---

# Interactive shell

Use this skill when an agent must operate a full-screen or interactive program
such as nano, Midnight Commander, a pager, or a terminal menu. Start
`interactive-shell --socket PATH --cols 80 --rows 24 --idle-timeout 300 -- COMMAND`
and observe its JSONL stdout. Send one request at a time with
`interactive-shell-input --socket PATH text TEXT`, `key KEY`, `combo KEY [CTRL] [ALT] [SHIFT]`, `paste TEXT`,
`mouse X Y BUTTON down|up|move`, `resize COLS ROWS`, `observe`, `raw HEX`, or
`click-id ID BUTTON`, `click-label LABEL BUTTON`, `click-at X Y BUTTON`, or
`shutdown`.

Screen events contain only rows changed since the previous event, a monotonically
increasing `seq`, the preceding `base`, and the cursor. An `observe` request
returns a `snapshot` event containing every current row, followed by an
acknowledgement; use it to resynchronize after missed deltas. Snapshot and screen
events also contain `elements`: OSC 8 hyperlinks appear as elements with an id,
label, URI, row, column, and width. Use `click-id` or `click-label` for those;
use `click-at` when the TUI exposes only a coordinate-based action. Parse those JSON fields;
do not grep terminal output or acknowledgements. An acknowledgement means only
that input was accepted by the wrapper, not that the program completed the
resulting action. Wait for a screen predicate or lifecycle event after every
state-changing request.

Named keys include ENTER, CTRL-A through CTRL-Z, ALT-graphic keys, BACKSPACE,
TAB, ESC, META-RIGHT, UP, DOWN, LEFT, RIGHT, HOME, END, PAGEUP, PAGEDOWN, INSERT,
DELETE, SHIFT/CTRL cursor variants, and F1 through F12. Use `raw` for an
explicit byte sequence, encoded as non-empty even-length hexadecimal. Use
`combo` when the needed modifier combination is not in the named-key list;
for example, `combo h ctrl alt shift` sends Alt followed by Ctrl-H, while
named cursor/function keys use xterm modifier encoding.

`paste` sends bracketed paste delimiters around the text. `mouse` uses 1-based
coordinates and sends an xterm mouse event. `resize` changes both the PTY and
the screen model, and rejects dimensions outside the wrapper bounds.

The wrapper allocates a real PTY, applies the requested dimensions, and owns a
0600 socket inside a private 0700 directory. It emits a final lifecycle event
for child exit, client shutdown, or idle timeout and removes only the socket
inode it created. A child process group is terminated during cleanup.

Screen events expose `styles` as row-based spans for basic ANSI colors, bold,
and reverse video, plus bounded `scrollback`. These observations are useful for
navigation but do not claim complete terminal emulation.

The screen model covers common cursor movement, erase, visibility, OSC,
alternate-screen, charset, line-drawing, scroll-region, and insert/delete-line
controls, including sequences split across PTY reads. Every ncurses extension
remains a possible unsupported control. For mc or another TUI, use predicates
based only on represented controls, or extend and test the model before relying
on a new control.
