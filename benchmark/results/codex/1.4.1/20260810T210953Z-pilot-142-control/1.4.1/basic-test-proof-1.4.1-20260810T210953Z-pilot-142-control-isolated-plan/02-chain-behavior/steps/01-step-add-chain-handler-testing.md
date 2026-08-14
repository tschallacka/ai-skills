# Testing companion: 01-step-add-chain-handler

## Static verification

After future implementation, inspect `handleChainClick(event)` in `button-chain.html`. Confirm it accepts only clicks on the current last button, increments generated-button state by one per accepted click, and treats the fourth generated button as the terminal trigger.

## Browser verification

Run UI story `US-01` through normal browser input. The browser run must click visible buttons only and must not use console evaluation, DOM mutation, storage edits, or injected events.

## Pass/fail criteria

Pass when accepted nonterminal clicks append exactly one button below the current last button and the fourth generated button clears the document to bordered lowercase `finished`. Fail on off-by-one terminal behavior, multiple appended buttons, non-last-button appends, retained old document content, or wrong terminal text.
