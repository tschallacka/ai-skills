# Testing companion: 03-step-finish-handler

## Browser verification

After US-01 reaches five visible buttons, click the fourth generated button as the current last button. Observe whether all prior content is removed and whether the remaining visible text is exactly `finished`.

## Pass criteria

Generated buttons one through three do not complete the flow. The fourth generated button clears the document and leaves only the lowercase text `finished`. Do not run this during the planning-only proof.
