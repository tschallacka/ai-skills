# Verification: 04-step-channels

## Automated tests

§ 2.1
JOIN #ops, assert 353 (NAMES includes the nick) and 366; PRIVMSG to the channel yields ":nick!user@host PRIVMSG #ops :text" to members; PING returns PONG.