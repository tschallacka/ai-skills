# Verification: 04-step-finish-behavior

## Browser verification

Open `button-chain.html` in a browser after implementation. Use real mouse clicks on the current last button to append generated buttons 1, 2, 3, and 4. Click generated button 4 after it is visible.

Pass only if generated buttons 1 through 3 continue the chain, generated button 4 clears all button content when pressed, and the resulting visible completion state contains exactly `finished` in lowercase inside the visible white border.
