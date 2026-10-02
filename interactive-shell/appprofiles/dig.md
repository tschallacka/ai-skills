# dig

### Identity
DNS lookup tool. NOT interactive — a command-line tool that runs once and exits. `dig HOSTNAME` (A record query), `dig HOSTNAME TYPE` (MX/CNAME/TXT/etc.), `dig -f FILE` (batch mode: reads one query per line from FILE, runs all, prints results, exits).

### Layout
Non-interactive; output is a text dump, not a screen interface. Single run produces:
- Query line (echoed, shows name and record type requested)
- ANSWER SECTION (if records exist)
- AUTHORITY SECTION (if present)
- ADDITIONAL SECTION (if present)
- Query time and server info at the bottom

### Workflows
1. Single lookup: `dig example.com`, query runs, output prints to stdout, process exits.
2. Batch lookup from file: Create a file with one query per line (e.g., `example.com`, `google.com A`), then `dig -f queries.txt`, all queries run sequentially with results interleaved, process exits when done.

### Quirks
- dig is entirely non-interactive; no input prompt, no screen refresh, no pager.
- Batch mode (`-f`) still outputs results for each query in order to stdout; it does not present a menu or prompt.
- Output lines may be long and wrap depending on terminal width.

### Unconfirmed
- Exact formatting of batch output when multiple queries are run
- Whether batch mode stops on error or continues to next query
