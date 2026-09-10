# tcpdump

Not tested this session -- requires root or CAP_NET_RAW capability for real packet capture, which the sandbox does not permit. Packet capture would also be disruptive. This profile documents standard behavior only; do not attempt real capture even if privilege barriers could be overcome. [unconfirmed] throughout.

### Identity
Packet sniffer and capture tool. `tcpdump` (capture on default interface), `tcpdump -i eth0` (capture on specific interface), `tcpdump -r file.pcap` (read saved capture file). NOT interactive — outputs packet lines to stdout, CTRL-C stops.

### Layout
NOT a screen interface. Output is a stream of text lines, one per packet:
- Each line shows timestamp, source IP, dest IP, protocol, and flags/data
- Output scrolls (no screen refresh, no paging)
- Ends with a summary: "N packets captured, N packets received by filter, N packets dropped by kernel"

### Modes
tcpdump has no interactive modes. It runs until stopped (CTRL-C) or reaches a packet limit (if specified with `-c N`).

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| CTRL-C | Stop capture | Terminates tcpdump and prints summary statistics |

### Workflows
1. Capture on default interface: `tcpdump`, output streams packet lines, CTRL-C to stop.
2. Capture with filter: `tcpdump 'tcp port 80'`, only HTTP traffic, CTRL-C stops.
3. Capture N packets then exit: `tcpdump -c 10`, stops automatically after 10 packets.
4. Read saved file: `tcpdump -r captured.pcap`, reads and replays packets from file, no interactivity.

### Quirks
- tcpdump is NOT a TUI app — it is a command-line streaming tool (like `ls`, `cat`, not like `less` or `vim`).
- Requires elevated privileges (root or CAP_NET_RAW) to capture; will refuse without them.
- Real packet capture requires network interface access; this environment cannot support it.
- Output to stdout is unbuffered by default; packets appear immediately.
- No screen redraw, no prompt, no interactive commands — purely a one-way stream until stopped.

### Unconfirmed
- Exact format and contents of packet lines (varies by packet type, options)
- Whether filter syntax is identical to Berkeley Packet Filter (BPF) standard
- Behavior of options like `-v` (verbose), `-X` (hex dump), `-A` (ASCII)
