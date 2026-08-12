# Verification

Start two isolated fixture workers with distinct `.bm-vars` files and invoke the wrapper concurrently.

Pass when each wrapper resolves its own plan, capsule, skill, and helper paths without overwriting the other worker.

