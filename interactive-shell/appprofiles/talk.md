# talk

Not tested this session -- requires a talk daemon (talkd) running and another logged-in user, neither of which is available. Modern systems rarely run talkd. Profile from standard documented behavior only. [unconfirmed] throughout.

### Identity
Old-style two-way terminal chat. `talk USER` or `talk USER@hostname` (request chat with another user). Initiates a split-screen bidirectional conversation. Requires talkd daemon running on both systems.

### Layout
- Split screen, top half -> your typing area
- Bottom half -> remote user's typing (live-updated as they type [unconfirmed])
- Each side labeled with username@host [unconfirmed]
- No separate prompt; both halves are input/output areas

### Modes
- **Active mode**: Conversation ongoing, both users can type and see each other's text.
- **Waiting mode**: Initial state after `talk USER`, waiting for remote user to accept and start talking.

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| Printable keys | Type your message | Text appears in top pane, sent to remote user |
| RETURN / ENTER | Send line | Submit current line to remote |
| CTRL-L | Redraw | Refresh screen if display is corrupted [unconfirmed] |
| CTRL-D / CTRL-C | End session | Close talk connection and exit |

### Workflows
1. Initiate chat: `talk bob`, wait for acceptance (screen shows "Waiting for response from bob..."), once accepted, type in top area, see bob's replies in bottom area, CTRL-D to close.

### Quirks
- talk is fundamentally different from write (two-way vs. one-way; split-screen vs. interruption).
- Both users must be logged in; the connection is established via talkd daemon on both systems.
- If talkd is not running or listening, talk will fail with "cannot connect to talkd" or similar.
- Modern systems have largely abandoned talkd for security/stability reasons; even if installed, it is often disabled.
- Network firewalls typically block the talkd UDP port (517-518 [unconfirmed]), preventing remote chat.

### Unconfirmed
- Exact initial prompt/waiting message
- Whether CTRL-L actually redraws or if it refreshes a different aspect
- Whether CTRL-C or CTRL-D is the standard close method
- Exact format of each pane's label or header
- Whether the remote user sees text as you type (character-by-character) or only on line submit
- Whether color/highlighting is used to distinguish your text from theirs
