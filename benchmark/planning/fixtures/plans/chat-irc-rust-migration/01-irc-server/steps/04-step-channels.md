# Step: 04-step-channels

## Ownership

- Goal: `01-irc-server`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `src/chat-server-rs/src/main.rs`
- Primary symbol or file scope: `channel commands (JOIN/PART/NAMES/PRIVMSG/NOTICE/PING/PONG)`
- Subscope: `N/A`

## Objective

§ 4.1
Implement JOIN/PART/NAMES (reply 353 names + 366 end-of-names, using `:nick!user@host` prefix form for user-visible messages), and emit PRIVMSG/NOTICE to channel members in `:nick!user@host PRIVMSG #chan :text` form; persist to the channel log; honour PING/PONG. Follows RFC 1459 grammar; the log line may retain the MSG <chan> <id> <ts> <nick> :text format internally.

## Instructions

§ 5.1
Implement JOIN/PART/NAMES/PRIVMSG/NOTICE/PING/PONG: JOIN echoes :nick!user@host JOIN :#chan and 353/366; PRIVMSG emits :nick!user@host PRIVMSG #chan :text to members and persists to the log; PING replies PONG.

## Acceptance criteria

§ 6.1
A joined client sees the :nick!user@host PRIVMSG #chan :text form; the channel log records MSG <chan> <id> <ts> <nick> :text; 353 names and 366 end-of-names are emitted; PING returns PONG.

## Handoff

§ 7.1
W05 adds the additive FETCH extension on top of this channel/log model.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
