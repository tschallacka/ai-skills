# mtr

### Identity
Network diagnostics: combined traceroute and ping with live updating display. `mtr HOSTNAME` or `mtr -c N HOSTNAME` (stop after N pings). Interactive full-screen display showing hop-by-hop statistics.

### Layout
- Header row -> title ("My traceroute"), target host, date/time, key hints
- Separator -> "Keys: H=help D=display R=restart O=order q=quit"
- Column headers -> Host, Loss%, Snt (sent), Last, Avg, Best, Wrst (worst), StDev
- Below -> one row per hop/router in the path, with statistics updated live

### Modes
- **Default mode**: Shows hop statistics (Loss%, Snt, Last, Avg, Best, Wrst, StDev).
- **Other display modes** (toggled via `d` or `o`): Cycle through different stat sets and sort orders [unconfirmed].

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| q | Quit | Exit mtr cleanly |
| h | Help | Display key reference |
| d | Display mode | Cycle through different display modes [unconfirmed] |
| r | Restart | Clear statistics and restart probing [unconfirmed] |
| o | Order fields | Change sort order or column layout [unconfirmed] |
| + / - | Scale | Adjust display width/precision [unconfirmed] |

### Workflows
1. Trace a host: `mtr example.com`, statistics update live, press `q` to quit.
2. Limited probes: `mtr -c 10 example.com`, runs 10 round-trips then exits automatically.

### Quirks
- mtr requires root or CAP_NET_RAW capability to send ICMP packets; without it, may show high loss or refuse to run.
- Live updating means the display continuously refreshes (not a static snapshot).
- Hitting `q` exits immediately; no confirmation needed.

### Unconfirmed
- Exact behavior of display mode (`d`) and order (`o`) toggles
- Whether `r` (restart) clears history entirely or just resets counters
- Whether `+`/`-` keys exist or how display scaling works
