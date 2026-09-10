# emacs

Not tested this session -- nix package fetch is too large/slow for this session. Every fact below is standard, stable documented behavior of emacs terminal mode (`emacs -nw`). [unconfirmed] throughout.

### Identity
Full-featured text editor and environment. `emacs FILE` (GUI mode, not applicable here), `emacs -nw FILE` (no window, terminal/curses mode). Modal in a different way than vi: mostly single-keypress commands, with occasional multi-key sequences (chords like CTRL-X, CTRL-C).

### Layout
- Top -> file name, mode info, line number in modeline
- Middle -> text buffer being edited
- Bottom -> minibuffer (for prompts, search, command input)
- Right -> [unconfirmed] scrollbar or indicator [unconfirmed]

### Modes
- **Normal mode**: Editing text; most keys insert characters, CTRL prefixes commands.
- **Minibuffer mode**: Active when a prompt is shown at the bottom (search, command input, etc.); RETURN submits, ESC or CTRL-G cancels.
- **Help mode**: Triggered by `C-h` (CTRL-H); shows help topics.

### Keys (core navigation and editing)
| Key | Action | Notes |
|-----|--------|-------|
| CTRL-F | Forward char | Move cursor one character right |
| CTRL-B | Backward char | Move cursor one character left |
| CTRL-N | Next line | Move cursor down |
| CTRL-P | Previous line | Move cursor up |
| CTRL-A | Beginning of line | Move to start of line |
| CTRL-E | End of line | Move to end of line |
| CTRL-D | Delete char | Delete character under cursor |
| BACKSPACE | Delete backward | Delete character before cursor |
| CTRL-K | Kill line | Delete from cursor to end of line |
| CTRL-Y | Yank | Paste last deleted text |
| CTRL-SPACE | Set mark | Begin text selection |
| CTRL-X CTRL-S | Save file | Write buffer to file |
| CTRL-X CTRL-C | Exit emacs | Quit (may prompt if unsaved changes) |

### Dialogs
- **Save on exit**: If file has unsaved changes, emacs prompts "Save file ...? (y/n/...)" in minibuffer. Type `y`, `n`, or `!` (save all).
- **Search**: CTRL-S opens search prompt in minibuffer; type search string, RETURN or CTRL-S again to find next.
- **Replace**: CTRL-H opens replace prompt (query-replace); prompts for old string, new string, then for each match: `y` to replace, `n` to skip, `!` to replace all.

### Workflows
1. Edit a file: `emacs -nw file.txt`, buffer loads, edit with arrow/CTRL keys, CTRL-X CTRL-S to save, CTRL-X CTRL-C to exit.
2. Find text: CTRL-S, type search term, RETURN, CTRL-S to find next occurrence.
3. Replace text: CTRL-H (query-replace), type old, RETURN, type new, RETURN, respond to each match (y/n/!).
4. Undo: CTRL-/ or CTRL-X u; redo is typically repeated undo of an undo.

### Quirks
- emacs uses "buffers" (in-memory text), not directly opening files; changes are only saved to disk via explicit save command.
- CTRL-G (keyboard-quit) cancels most operations and returns to normal mode.
- The minibuffer is a small text-editing area itself; CTRL-A/CTRL-E work there too.
- Search and replace use regex by default [unconfirmed].
- No visual mode selection; marking (CTRL-SPACE) is invisible until you perform an operation on the marked region.

### Unconfirmed
- Exact color/highlighting scheme in terminal mode
- Whether mouse support is available in terminal emacs
- Exact undo/redo behavior and whether there is a redo command
- Whether CTRL-L (recenter) works in terminal mode
