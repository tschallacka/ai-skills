# info

### Identity
GNU Info hypertext documentation browser. `info TOPIC` (e.g., `info coreutils`) opens the manual at the top node.

### Layout
- Top rows -> navigation links (e.g., "Next: SectionName,  Up: ParentNode") and title
- Middle rows -> node content with menu and cross-references
- Menu section (rows with `* ItemName::`) -> navigation links to sub-nodes
- Status line -> "-----Info: (FILENAME)NODENAME, LINE lines --PROGRESS------"
- Last row -> help text "Type H for help, h for tutorial"

### Modes
Node-based navigation (not a linear pager). Each node is a page in a directed graph of documentation.

### Keys
| Key | Action |
|-----|--------|
| TAB | Move to next link (menu item or cross-reference) |
| SHIFT+TAB | Move to previous link [unconfirmed] |
| ENTER | Follow the focused link |
| n | Next node at same level |
| p | Previous node at same level |
| u | Up to parent node |
| l | Back (go to previous node visited) |
| q | Quit |
| SPACE / PAGEDOWN | Scroll down |
| b / PAGEUP | Scroll up |
| h | Tutorial |
| H | Full help |
| `/text` ENTER | Search for text [unconfirmed] |

### Dialogs
None (navigation is direct with arrow keys and TAB).

### Workflows
1. Browse manual: `info coreutils`. Observe Top node with menu. Press TAB to focus a menu item (e.g., "Introduction"). Press ENTER to follow it. Observe new node content. Press u to go back to parent. Press q to quit.
2. Navigate nodes: `info coreutils`, use n/p to move between sibling nodes. Use u to go to parent. Observe status line changes with node name.
3. Search (if supported): `info coreutils`, press `/`, type text, ENTER. Jump to first match [unconfirmed].

### Quirks
- Info is a hypertext browser, not a linear pager; node structure varies per manual
- TAB moves between links (menu items and cross-references); must be used to navigate, not arrow keys alone
- Status line shows "(FILENAME)NODENAME" to indicate current position in the tree
- SPACE scrolls within a node; when at the end of a node, SPACE moves to next node automatically [unconfirmed]
- Visited nodes are tracked; `l` goes back through visited history

### Unconfirmed
- Exact behavior of SPACE when at end of node (scroll to next or stop) [unconfirmed]
- Whether search (`/text`) is available and how it works [unconfirmed]
- Behavior when a cross-reference link is broken or missing [unconfirmed]
- Exact key name for "previous link" (SHIFT+TAB or alt+TAB or other) [unconfirmed]
