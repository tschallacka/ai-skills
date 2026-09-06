# Step: 03-step-registration

## Ownership

- Goal: `01-irc-server`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `src/chat-server-rs/src/main.rs`
- Primary symbol or file scope: `registration lifecycle (NICK/USER backend)`
- Subscope: `N/A`

## Objective

§ 4.1
Implement RFC registration: on NICK+USER set the nick/user, reply 001/002/003/004 + 005 ISUPPORT; 433 on nick-in-use; deliver MOTD via 375/372/376; reject a duplicate NICK with ERR. ALSO add an explicit CAP arm: on CAP LS 302 answer 410 (or otherwise not reject it), and on CAP END proceed — so a real client that negotiates caps before NICK/USER registers cleanly and still gets a valid 005. Standard prefix `:server 001 nick :Welcome...`.

## Instructions

§ 5.1
Implement NICK+USER registration: reply 001-004 + 005 ISUPPORT, MOTD 375/372/376, 433 on nick-in-use, and a CAP LS/END arm answering 410/not rejecting, so a client negotiating caps before NICK/USER registers cleanly. Standard :server 001 nick :Welcome prefix.

## Acceptance criteria

§ 6.1
A TLS client sending NICK+USER receives 001-005 and MOTD 375/372/376; a CAP LS 302 / CAP END exchange does not block registration; duplicate NICK gets 433.

## Handoff

§ 7.1
W04 channel commands follow the same registration/numeric conventions.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
