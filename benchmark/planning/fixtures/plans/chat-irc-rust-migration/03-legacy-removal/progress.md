# Progress: 03-legacy-removal

**Progress:** `100%  ####################  100%` ✅

| Goalname | Stepname | Description | Completion status |
|---|---|---|---|
| 03-legacy-removal | 01-step-delete-server-sh | git rm chat/scripts/chat-server.sh (the bash server launcher). The rust server binary is started dir... | ✅ completed |
| 03-legacy-removal | 03-step-delete-send | git rm chat/scripts/chat-send.sh. Replaced by the rust client send command. | ✅ completed |
| 03-legacy-removal | 04-step-delete-read | git rm chat/scripts/chat-read.sh. Replaced by the rust client read-delta command. | ✅ completed |
| 03-legacy-removal | 05-step-delete-tail | git rm chat/scripts/chat-tail.sh. Replaced by the rust client tail command. | ✅ completed |
| 03-legacy-removal | 06-step-delete-register | git rm chat/scripts/chat-register.sh. Channel registration is implicit via JOIN on the rust server. | ✅ completed |
| 03-legacy-removal | 07-step-delete-watch | git rm chat/scripts/chat-watch.sh. Replaced by the rust client tail/poll. | ✅ completed |
| 03-legacy-removal | 08-step-delete-announce | git rm chat/scripts/chat-announce.sh. The rust server now broadcasts the UDP announce beacon itself. | ✅ completed |
| 03-legacy-removal | 09-step-delete-discover | git rm chat/scripts/chat-discover.sh. The rust client now discovers via its own UDP listener. | ✅ completed |
