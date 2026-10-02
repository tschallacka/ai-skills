# write

Not tested this session -- requires a second logged-in user (session), which this environment does not have. Also presents ethical/practical concerns. Profile from standard documented behavior only. [unconfirmed] throughout.

### Identity
Send a message to another logged-in user's terminal. `write USER [TTY]` (send message to USER, optionally specify their terminal if multiple sessions). Sends lines of text from sender's terminal to recipient's, interrupting their screen to display the message.

### Layout
Recipient sees an interruption on their screen:
```
Message from USER@hostname on TERMINAL at TIME:
line of text from sender
another line
```
Sender's terminal has a text-input prompt (similar to a chat interface [unconfirmed]).

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| Printable keys | Type message | Enter lines of text |
| RETURN / ENTER | Send line | Transmit one line to recipient's screen |
| CTRL-D / EOF | End session | Close write and disconnect |
| CTRL-C | Interrupt [unconfirmed] | Cancel current line or session [unconfirmed] |

### Workflows
1. Send message to user: `write alice`, type message lines (each line appears on recipient's screen immediately), CTRL-D to end.
2. Send to specific terminal: `write alice tty2` (if alice is logged in on multiple terminals).

### Quirks
- write interrupts the recipient's current terminal output; not queued or deferred.
- Both users must be logged in (multiple ttys or ssh sessions for the same user may not count [unconfirmed]).
- The recipient does not have to accept or acknowledge; the message is printed regardless.
- Modern systems often block write between users for privacy/security; even if both users exist, write may be disabled.
- write can be used for login-user messaging; using it across accounts requires both to exist and have write permission enabled.

### Unconfirmed
- Whether `write` works between different user accounts or only for the same user's multiple sessions
- Whether write permission can be revoked per-user or globally
- Exact format of the "Message from..." line and timestamp
- Whether CTRL-C cancels the current line or the entire session
- Whether the recipient can reply back (two-way communication) or if it is one-way
