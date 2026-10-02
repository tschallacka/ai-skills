# nslookup

### Identity
DNS lookup and query tool. `nslookup HOSTNAME` (one-shot) or bare `nslookup` (interactive REPL).

### Layout
- Body -> query results and menu items
- Last row -> prompt or status line

### Modes
Two modes: one-shot (prints results and exits immediately) and interactive REPL (stays open at `>` prompt).

### Keys
| Key | Action | Mode |
|-----|--------|------|
| ENTER | Execute command | Interactive |
| `exit` text + ENTER | Quit interactive mode | Interactive |
| `server NAMESERVER` + ENTER | Change DNS server | Interactive |
| `set type=TYPE` + ENTER | Set query type (A, MX, NS, etc.) | Interactive |
| HOSTNAME + ENTER | Look up hostname | Interactive |

### Dialogs
None (no modal dialogs).

### Workflows
1. One-shot lookup: `nslookup example.com`. Observe results and automatic exit.
2. Interactive mode: bare `nslookup`, type a hostname, ENTER. Observe results within the same session. Type `exit`, ENTER to quit.
3. Change query type: bare `nslookup`, type `set type=MX`, ENTER, type a hostname, ENTER. Observe MX records.

### Quirks
- One-shot mode prints results to stdout and exits; no prompt is shown
- Interactive mode displays a `>` prompt and accepts DNS commands
- `server` command allows querying a specific nameserver instead of the default

### Unconfirmed
- Complete list of supported query types (A, AAAA, MX, NS, SOA, etc.) [unconfirmed]
- Behavior when a hostname is not found
- Whether results are cached between queries in interactive mode [unconfirmed]
