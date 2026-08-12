# Testing companion: 02-step-append-handler

## Browser verification

After implementation, execute the US-01 cache with direct clicks on the current last button. Observe the button count after every click: two buttons after click one, three after click two, four after click three, and five after click four.

## Pass criteria

Each click appends exactly one button below the previous last button. Clicking an older non-last button must not append a button. Do not run this during the planning-only proof.
