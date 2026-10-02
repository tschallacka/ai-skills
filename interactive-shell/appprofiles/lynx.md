# lynx

Text-mode web browser for viewing HTML documents and navigating links. `lynx https://example.com`, `lynx file.html`, etc.

### Identity

Full interactive TUI web browser with link navigation, history, and search.

### Layout

- Top -> URL bar and status line
- Body -> rendered page content with linked text highlighted
- Bottom -> command hints and status information

### Keys

| Key | Action | Notes |
|-----|--------|-------|
| UP / DOWN | Move between links and page content | Cycles through clickable/navigable elements |
| RIGHT / ENTER | Follow highlighted link | Opens the linked page |
| LEFT / BACKSPACE | Go back to previous page | Returns to last visited page |
| g | Open "Go To" URL prompt | Allows entering a new URL to visit |
| / | Search text on current page | Case-insensitive forward search; `n` finds next match |
| q | Quit lynx | Prompts with y/n confirmation before exit |
| = | Show page information | Displays current URL, title, document info |
| ? | Show help | Displays keybindings and command reference |
| H | Go to help page | Like `?` but in a navigable page |
| p | Print page | Saves or prints current page |
| m | Go to main page | Returns to configured home/main page |
| d | Download current link | Saves the link target to disk |

### Workflows

1. **Browse a page**: ENTER/RIGHT to follow links, LEFT to go back, UP/DOWN to move between links.
2. **Visit a new URL**: Press `g`, type the URL, ENTER.
3. **Search page text**: Press `/`, type search term, ENTER. Press `n` to find next.
4. **Quit**: Press `q`, then `y` to confirm.

### Quirks

- Navigation is link-centric; not all text is traversable (only clickable elements)
- Search is case-insensitive by default
- No visual scrollbar; navigation wraps between top and bottom of page
- History is maintained throughout the session; `LEFT` walks backward through it
- Page rendering is text-only; images are replaced with `[Image]` or similar placeholders
- Some modern web features (JavaScript, CSS styling) are not supported

### Unconfirmed

- Network access and behavior with network errors [unconfirmed]
- Full keybinding details for less common operations [unconfirmed]
