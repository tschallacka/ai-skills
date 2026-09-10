# pg

POSIX pager (older, simpler alternative to `less`/`more`). `pg filename`, `pg < file`, etc.

### Identity

Traditional line-by-line pager with a simple prompt interface (simpler than `less`, less capable than modern pagers).

### Layout

- Display -> one screenful of text at a time (24-30 lines typical)
- Prompt -> `:` (colon) at the bottom, awaiting input

### Keys

| Key | Action | Notes |
|-----|--------|-------|
| ENTER or SPACE | Page forward (next screen) | Shows next full screen of text |
| b | Page backward (previous screen) | Returns to previous screenful |
| / | Search for pattern | Enters search mode; searches forward |
| n | Next search result | Finds next occurrence of last searched pattern |
| q | Quit pager | Returns to shell |
| = | Show current line number | Displays position in file |
| + | Forward (similar to SPACE) | Alias for page-forward |
| Number + ENTER | Jump to line (e.g., `10` ENTER goes to line 10) | |
| g | Go to end of file | Jumps to last lines |
| G | Go to end of file | Alternative spelling |

### Workflows

1. **Page through a file**: SPACE to move forward, `b` to go backward, `q` to quit.
2. **Search**: `/` to start search, type pattern, ENTER. Press `n` to find next match.
3. **Jump to a line**: Type line number, press ENTER (e.g., `42` ENTER goes to line 42).
4. **Quit**: Press `q`.

### Quirks

- Prompt is `:` (colon), distinct from `less`'s status line (which is at the bottom of the screen)
- Search in `pg` does not highlight results; you must scan the displayed text
- Backward paging (`b`) re-displays the previous screen; not random-access like `less`
- No visual scrollbar or position indicator (unlike `less`)
- Simpler and faster than `less` for very large files but less interactive

### Comparison with less

- **Less**: Modern pager with full-screen status line, visual search highlights, rich keybindings, random-access
- **Pg**: Simpler POSIX pager with `:` prompt, basic search, line-by-line paging
- For basic file browsing, both work; `pg` is lighter-weight, `less` is more featureful

### Unconfirmed

- Exact behavior of `G` command [unconfirmed]
- Search pattern syntax and case sensitivity [unconfirmed]
