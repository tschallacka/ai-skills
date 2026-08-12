# Verification: 03-step-append-behavior

## Browser verification

Open `button-chain.html` in a browser after implementation. Use real mouse clicks only. Click the initial button once and confirm exactly one generated button appears below it. Click the earlier initial button again and confirm no additional button appears. Then click the current last generated button and confirm exactly one new generated button appears below it.

Pass only if each eligible current-last-button click appends exactly one button below it and an earlier non-last button cannot append another button. Do not use console commands, injected events, or DOM mutation as passing evidence.
